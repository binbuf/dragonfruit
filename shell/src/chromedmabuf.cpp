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
        if (pair.modifier == frame.modifier) {
            check.valid = true;
            return check;
        }
    }
    check.reason = formatSeen ? QStringLiteral("no-modifier")
                              : QStringLiteral("no-format");
    return check;
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
