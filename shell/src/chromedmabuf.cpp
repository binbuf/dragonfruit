// SPDX-License-Identifier: GPL-3.0-or-later
#include "chromedmabuf.h"

#include "chromescale.h"

namespace ChromeDmabuf {

ChromeDmabufCheck validate(
    const ChromeDmabufFrame &frame,
    const QVector<GpuChromeCapabilities::FormatModifier> &advertised)
{
    ChromeDmabufCheck check;
    if (!frame.valid()) {
        check.reason = QStringLiteral("bad-export");
        return check;
    }
    constexpr int kMaxDimension = 16384;
    if (frame.size.width() > kMaxDimension || frame.size.height() > kMaxDimension) {
        check.reason = QStringLiteral("oversized");
        return check;
    }

    bool formatSeen = false;
    for (const GpuChromeCapabilities::FormatModifier &pair : advertised) {
        if (pair.format != frame.format)
            continue;
        formatSeen = true;
        if (pair.modifier != frame.modifier)
            continue;
        // The pair is importable, but an opaque `X*` export cannot represent
        // the transparent pixels a chrome scene clears to. Reject it here so
        // the caller takes the shm fallback instead of attaching a buffer
        // that would paint black behind every rounded corner and glass edge.
        if (!GpuChromeCapabilities::supportsPremultipliedAlpha(pair.format)) {
            check.reason = QStringLiteral("opaque-format");
            return check;
        }
        check.valid = true;
        return check;
    }
    check.reason = formatSeen ? QStringLiteral("no-modifier")
                              : QStringLiteral("no-format");
    return check;
}

FailureClass classifyFailure(const QString &reason)
{
    // Saturation and timing: the frame is valid, the producer just needs
    // another turn. These never disable a target or the rail.
    static const QSet<QString> transient = {
        QStringLiteral("ring-exhausted"),
        QStringLiteral("sync-timeout"),
        QStringLiteral("completion-pending"),
        QStringLiteral("completion-ready"),
        QStringLiteral("not-ready"),
        QStringLiteral("slot-busy"),
        QStringLiteral("import-pending"),
        QStringLiteral("deferred"),
    };
    // The exported frame cannot be attached as-is. At a producer boundary
    // this is rail-wide (every view shares the exporter); at a target commit
    // it is local to the target.
    static const QSet<QString> format = {
        QStringLiteral("opaque-format"),
        QStringLiteral("no-format"),
        QStringLiteral("no-modifier"),
        QStringLiteral("bad-export"),
        QStringLiteral("oversized"),
        QStringLiteral("bad-texture"),
        QStringLiteral("egl-image-failed"),
        QStringLiteral("export-query-failed"),
        QStringLiteral("multi-plane"),
        QStringLiteral("export-failed"),
    };
    // A target's own import/geometry rejected this commit. The other targets
    // of the same export remain eligible.
    static const QSet<QString> target = {
        QStringLiteral("import-failed"),
        QStringLiteral("no-viewporter"),
        QStringLiteral("bad-source"),
        QStringLiteral("no-slot"),
        QStringLiteral("dup-failed"),
        QStringLiteral("no-params"),
        QStringLiteral("bad-frame"),
        QStringLiteral("bad-size"),
    };
    // The connection/device/context/exporter is gone. No target can recover
    // by itself; the rail is disabled until a bounded capability/output
    // change retries it.
    static const QSet<QString> transport = {
        QStringLiteral("no-dmabuf"),
        QStringLiteral("no-egl-context"),
        QStringLiteral("no-egl-export"),
        QStringLiteral("no-gl-context"),
        QStringLiteral("no-offscreen-surface"),
        QStringLiteral("no-current-context"),
        QStringLiteral("no-render-context"),
        QStringLiteral("render-control-init-failed"),
        QStringLiteral("fbo-create-failed"),
        QStringLiteral("software-renderer"),
        QStringLiteral("render-failed"),
        QStringLiteral("ring-phase"),
        QStringLiteral("ring-serial"),
        QStringLiteral("context-lost"),
        QStringLiteral("device-lost"),
        QStringLiteral("connection-lost"),
    };
    // The surface closed or was replaced while its asynchronous frame was
    // still completing. This is a stale scene, not a transport failure.
    static const QSet<QString> stale = {
        QStringLiteral("unknown-surface"),
        QStringLiteral("unknown-lock-surface"),
        QStringLiteral("no-surface"),
        QStringLiteral("no-buffer-state"),
    };

    if (reason.isEmpty())
        return FailureClass::None;
    if (transient.contains(reason))
        return FailureClass::Transient;
    if (stale.contains(reason))
        return FailureClass::Stale;
    if (transport.contains(reason))
        return FailureClass::Transport;
    if (format.contains(reason))
        return FailureClass::Format;
    if (target.contains(reason))
        return FailureClass::Target;
    // An unlisted reason is treated as target-local: it disables one surface
    // rather than the whole rail. A producer boundary widens that to the rail
    // because a shared exporter failure affects every view.
    return FailureClass::Target;
}

bool isTransientFailure(const QString &reason)
{
    return classifyFailure(reason) == FailureClass::Transient;
}

QString failureClassName(FailureClass failure)
{
    switch (failure) {
    case FailureClass::None:
        return QStringLiteral("none");
    case FailureClass::Transient:
        return QStringLiteral("transient");
    case FailureClass::Format:
        return QStringLiteral("format");
    case FailureClass::Target:
        return QStringLiteral("target");
    case FailureClass::Transport:
        return QStringLiteral("transport");
    case FailureClass::Stale:
        return QStringLiteral("stale");
    }
    return QStringLiteral("target");
}

TargetPlan planTargets(const QVector<QString> &labels,
                       const QSet<QString> &disabled)
{
    TargetPlan plan;
    for (const QString &label : labels) {
        if (disabled.contains(label))
            plan.skip.append(label);
        else
            plan.commit.append(label);
    }
    return plan;
}

uint32_t modifierHigh(uint64_t modifier)
{
    return static_cast<uint32_t>(modifier >> 32);
}

uint32_t modifierLow(uint64_t modifier)
{
    return static_cast<uint32_t>(modifier & 0xffffffffu);
}

ChromeFrameDamage frameDamage(const QRect &logicalRegion, qreal scale,
                              const QSize &frameSize)
{
    if (logicalRegion.isEmpty() || frameSize.isEmpty())
        return ChromeFrameDamage::full();
    // The same rounding the source crop uses, so a damage box can never fall
    // inside the pixels the target actually samples. A region that maps
    // entirely outside the frame is an empty (unchanged) region, not an
    // unknown one.
    return ChromeFrameDamage::region(
        ChromeScale::bufferRect(logicalRegion, scale)
            .intersected(QRect(QPoint(0, 0), frameSize)));
}

QRect targetDamage(const ChromeFrameDamage &damage, const QRect &sourceRect)
{
    if (!damage.known())
        return sourceRect;
    return damage.rect.intersected(sourceRect);
}

} // namespace ChromeDmabuf

ChromeDmabufRecovery::ChromeDmabufRecovery(int maxAttempts)
    : m_maxAttempts(qMax(1, maxAttempts))
{
}

bool ChromeDmabufRecovery::recover()
{
    if (!canRecover())
        return false;
    ++m_attempts;
    return true;
}

void ChromeDmabufReleaseTracker::acquire(quint64 serial)
{
    ++m_users[serial];
}

bool ChromeDmabufReleaseTracker::release(quint64 serial)
{
    auto it = m_users.find(serial);
    if (it == m_users.end())
        return false;
    if (--it.value() > 0)
        return false;
    m_users.erase(it);
    return true;
}

int ChromeDmabufReleaseTracker::users(quint64 serial) const
{
    return m_users.value(serial, 0);
}

bool ChromeDmabufSlot::beginCommit()
{
    if (m_state == State::Empty || m_state == State::Ready) {
        m_state = State::Busy;
        return true;
    }
    return false;
}

void ChromeDmabufSlot::release()
{
    if (m_state == State::Busy)
        m_state = State::Ready;
}

void ChromeDmabufSlot::fail(const QString &reason)
{
    m_state = State::Failed;
    m_reason = reason;
}

void ChromeDmabufSlot::reset()
{
    m_state = State::Empty;
    m_reason.clear();
}

ChromeDmabufRing::ChromeDmabufRing(int capacity)
    : m_capacity(qMax(1, capacity))
{
}

void ChromeDmabufRing::retireLiveSlots()
{
    bool retired = false;
    for (Slot &slot : m_slots) {
        if (!slot.used || slot.retired)
            continue;
        slot.retired = true;
        // A retired slot holds pixels of the previous generation; they are no
        // longer a baseline for a partial commit.
        slot.contentValid = false;
        retired = true;
    }
    if (retired)
        ++m_resets;
}

bool ChromeDmabufRing::setSize(const QSize &size)
{
    if (size == m_size)
        return false;
    const bool hadLive = liveCount() > 0;
    m_size = size;
    if (hadLive)
        retireLiveSlots();
    return hadLive;
}

bool ChromeDmabufRing::setFormat(uint32_t format, uint64_t modifier)
{
    if (format == m_format && modifier == m_modifier)
        return false;
    // The first successful export establishes the ring's format. All slots
    // share the FBO internal format, so this is not a generation change.
    if (m_format == 0 && m_modifier == 0) {
        m_format = format;
        m_modifier = modifier;
        return false;
    }
    const bool hadLive = liveCount() > 0;
    m_format = format;
    m_modifier = modifier;
    if (hadLive)
        retireLiveSlots();
    return hadLive;
}

bool ChromeDmabufRing::configure(const QSize &size, uint32_t format,
                                 uint64_t modifier)
{
    const bool sizeRetired = setSize(size);
    const bool formatRetired = setFormat(format, modifier);
    return sizeRetired || formatRetired;
}

bool ChromeDmabufRing::retireAll()
{
    if (liveCount() == 0)
        return false;
    retireLiveSlots();
    return true;
}

int ChromeDmabufRing::findFreeCurrent() const
{
    for (int i = 0; i < m_slots.size(); ++i) {
        const Slot &slot = m_slots.at(i);
        if (slot.used && !slot.busy && !slot.retired)
            return i;
    }
    return -1;
}

int ChromeDmabufRing::allocateSlot()
{
    for (int i = 0; i < m_slots.size(); ++i) {
        if (m_slots.at(i).used)
            continue;
        Slot &slot = m_slots[i];
        slot = Slot();
        slot.used = true;
        slot.busy = true;
        slot.phase = Phase::Submitted;
        ++m_allocations;
        return i;
    }
    if (m_slots.size() >= m_capacity)
        return -1;
    Slot slot;
    slot.used = true;
    slot.busy = true;
    slot.phase = Phase::Submitted;
    m_slots.append(slot);
    ++m_allocations;
    return m_slots.size() - 1;
}

int ChromeDmabufRing::acquire()
{
    if (m_size.isEmpty())
        return -1;
    const int free = findFreeCurrent();
    if (free >= 0) {
        m_slots[free].busy = true;
        m_slots[free].phase = Phase::Submitted;
        ++m_reuses;
        return free;
    }
    return allocateSlot();
}

bool ChromeDmabufRing::setSerial(int slot, uint64_t serial)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || !entry.busy)
        return false;
    entry.serial = serial;
    return true;
}

bool ChromeDmabufRing::release(uint64_t serial)
{
    if (serial == 0)
        return false;
    for (Slot &slot : m_slots) {
        if (slot.used && slot.busy && slot.serial == serial) {
            slot.busy = false;
            slot.phase = Phase::Free;
            return true;
        }
    }
    return false;
}

bool ChromeDmabufRing::releaseSlot(int slot)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || !entry.busy)
        return false;
    entry.busy = false;
    entry.phase = Phase::Free;
    return true;
}

bool ChromeDmabufRing::abandon(int slot)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || !entry.busy)
        return false;
    entry.busy = false;
    entry.serial = 0;
    entry.phase = Phase::Free;
    return true;
}

bool ChromeDmabufRing::markPending(int slot)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || !entry.busy || entry.phase != Phase::Submitted)
        return false;
    entry.phase = Phase::Pending;
    return true;
}

bool ChromeDmabufRing::markReady(int slot)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || !entry.busy || entry.phase != Phase::Pending)
        return false;
    entry.phase = Phase::Ready;
    return true;
}

bool ChromeDmabufRing::markOwned(int slot)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || !entry.busy || entry.phase != Phase::Ready)
        return false;
    entry.phase = Phase::Owned;
    return true;
}

ChromeDmabufRing::Phase ChromeDmabufRing::phase(int slot) const
{
    if (slot < 0 || slot >= m_slots.size() || !m_slots.at(slot).used)
        return Phase::Free;
    return m_slots.at(slot).phase;
}

int ChromeDmabufRing::pendingCount() const
{
    int count = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used && slot.phase == Phase::Pending)
            ++count;
    }
    return count;
}

int ChromeDmabufRing::readyCount() const
{
    int count = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used && slot.phase == Phase::Ready)
            ++count;
    }
    return count;
}

int ChromeDmabufRing::ownedCount() const
{
    int count = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used && slot.phase == Phase::Owned)
            ++count;
    }
    return count;
}

QVector<int> ChromeDmabufRing::takeRetired()
{
    QVector<int> retired;
    for (int i = 0; i < m_slots.size(); ++i) {
        Slot &slot = m_slots[i];
        if (slot.used && !slot.busy && slot.retired) {
            slot = Slot();
            retired.append(i);
        }
    }
    return retired;
}

bool ChromeDmabufRing::isBusy(int slot) const
{
    return slot >= 0 && slot < m_slots.size() && m_slots.at(slot).used
        && m_slots.at(slot).busy;
}

bool ChromeDmabufRing::isRetired(int slot) const
{
    return slot >= 0 && slot < m_slots.size() && m_slots.at(slot).used
        && m_slots.at(slot).retired;
}

bool ChromeDmabufRing::isFree(int slot) const
{
    return slot >= 0 && slot < m_slots.size() && m_slots.at(slot).used
        && !m_slots.at(slot).busy && !m_slots.at(slot).retired;
}

bool ChromeDmabufRing::contains(int slot) const
{
    return slot >= 0 && slot < m_slots.size() && m_slots.at(slot).used;
}

bool ChromeDmabufRing::contentValid(int slot) const
{
    return contains(slot) && m_slots.at(slot).contentValid;
}

void ChromeDmabufRing::setContentValid(int slot, bool valid)
{
    if (!contains(slot))
        return;
    // A retired slot's pixels belong to a previous generation and are never a
    // valid baseline again.
    if (valid && m_slots.at(slot).retired)
        return;
    m_slots[slot].contentValid = valid;
}

int ChromeDmabufRing::liveCount() const
{
    int live = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used)
            ++live;
    }
    return live;
}

int ChromeDmabufRing::busyCount() const
{
    int busy = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used && slot.busy)
            ++busy;
    }
    return busy;
}

int ChromeDmabufRing::freeCount() const
{
    int free = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used && !slot.busy && !slot.retired)
            ++free;
    }
    return free;
}

void ChromeDmabufRing::reset()
{
    m_slots.clear();
    m_size = QSize();
    m_format = 0;
    m_modifier = 0;
    m_allocations = 0;
    m_reuses = 0;
    m_resets = 0;
}
