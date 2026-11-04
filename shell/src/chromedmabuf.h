// SPDX-License-Identifier: MIT
// Pure dmabuf frame validation and single-buffer commit state (T-21.1d).
//
// This is the dependency-free half of the GPU chrome transport: it validates
// an exported dmabuf frame against the compositor's linux-dmabuf feedback and
// owns the busy/reuse rules for one reusable buffer. No Wayland, EGL, or Qt
// GUI state is involved, so the format/commit rules are unit-tested headlessly.
#pragma once

#include <QHash>
#include <QSize>
#include <QString>
#include <QVector>

#include <cstdint>

#include "gpuchromecapabilities.h"

// One exported dmabuf plane. `fd` is owned by the caller; a commit path dups
// it before handing it to `zwp_linux_buffer_params_v1`.
struct ChromeDmabufFrame {
    int fd = -1;
    QSize size;
    uint32_t format = 0;
    uint64_t modifier = 0;
    uint32_t stride = 0;
    uint32_t offset = 0;
    // EGL exports a GL texture with its origin at the bottom-left, so the
    // dmabuf rows are y-inverted relative to the top-left compositor
    // convention. The linux-dmabuf `y_invert` flag tells the compositor to
    // flip it when sampling.
    bool yInverted = false;
    // Monotonic identity of the producer-side export. Every committed frame
    // gets a new serial; the shell uses it to match a `wl_buffer.release` to
    // the slot that produced it.
    uint64_t serial = 0;
    // Producer-side ring slot that owns this export. The shell keeps one
    // reusable wl_buffer per (surface, slot), so a reused slot does not
    // allocate a new buffer.
    int slot = -1;

    bool valid() const
    {
        return fd >= 0 && !size.isEmpty() && stride > 0;
    }
};

struct ChromeDmabufCheck {
    bool valid = false;
    // Empty when valid; otherwise a short fallback token (`no-format`,
    // `no-modifier`, `bad-export`, ...).
    QString reason;
};

namespace ChromeDmabuf {

// Validate an exported frame against the advertised format/modifier table.
// The compositor's `dmabuf_imported` accepts only an exact `(format, modifier)`
// pair from the feedback table, so an implicit or unlisted modifier is a
// fallback, not something to paper over.
ChromeDmabufCheck validate(
    const ChromeDmabufFrame &frame,
    const QVector<GpuChromeCapabilities::FormatModifier> &advertised);

// `zwp_linux_buffer_params_v1.add` takes the 64-bit modifier as two uint32s.
uint32_t modifierHigh(uint64_t modifier);
uint32_t modifierLow(uint64_t modifier);

} // namespace ChromeDmabuf

// The reuse state of one dmabuf-backed surface buffer. Kept as the small
// single-buffer state machine used by the export smoke test; the production
// transport uses `ChromeDmabufRing` below.
// Tracks how many committed surfaces still hold one exported frame (T186d).
//
// A Dock/menu scene is exported once but can be committed to two surfaces: the
// bar itself and its popover sub-rect. Each surface gets its own wl_buffer, so
// each gets its own `wl_buffer.release`; the producer slot must stay busy until
// the last user is done. The tracker turns the per-surface releases back into
// one producer release and is pure so the rule is unit-tested headlessly.
class ChromeDmabufReleaseTracker
{
public:
    void acquire(quint64 serial);
    // True when this was the last user of `serial`; the entry is then removed.
    bool release(quint64 serial);
    int users(quint64 serial) const;
    int size() const { return m_users.size(); }
    void clear() { m_users.clear(); }

private:
    QHash<quint64, int> m_users;
};

class ChromeDmabufSlot
{
public:
    enum class State {
        Empty,   // no buffer has been created yet
        Ready,   // a buffer exists and is free to render into
        Busy,    // attached and not yet released by the compositor
        Failed,  // creation/import failed; the surface must fall back
    };

    State state() const { return m_state; }
    bool canRender() const { return m_state == State::Empty || m_state == State::Ready; }
    bool busy() const { return m_state == State::Busy; }
    bool failed() const { return m_state == State::Failed; }
    QString failureReason() const { return m_reason; }

    // A render/commit begins. False when the buffer is busy or failed.
    bool beginCommit();
    // The compositor returned the buffer through `wl_buffer.release`.
    void release();
    // Creation or import failed; the surface falls back to T185 shm.
    void fail(const QString &reason);
    void reset();

private:
    State m_state = State::Empty;
    QString m_reason;
};

// A bounded, per-surface dmabuf ring (T186c).
//
// The producer owns one FBO/export per slot and the shell owns one reusable
// `wl_buffer` per slot. A slot is only acquired for rendering while it is
// free; the compositor holds it from commit until `wl_buffer.release`, which
// returns the slot through `release(serial)`. A size or format change retires
// the current generation: free retired slots can be dropped immediately,
// busy ones stay alive until their release. Allocation is bounded by
// `capacity` (2-3; the transport uses 3).
class ChromeDmabufRing
{
public:
    explicit ChromeDmabufRing(int capacity = 3);

    // Change the geometry. Returns true when live slots were retired.
    bool setSize(const QSize &size);
    // Change the exported format/modifier. The first format is adopted
    // without retiring (all slots share the FBO's internal format); a later
    // change retires the old generation. Returns true when slots were retired.
    bool setFormat(uint32_t format, uint64_t modifier);
    // Convenience for tests: setSize() followed by setFormat().
    bool configure(const QSize &size, uint32_t format, uint64_t modifier);
    // Retire every live slot without changing the geometry.
    bool retireAll();

    QSize size() const { return m_size; }
    uint32_t format() const { return m_format; }
    uint64_t modifier() const { return m_modifier; }
    int capacity() const { return m_capacity; }

    // Acquire a free current-generation slot and mark it in use. Returns -1
    // when every slot is busy/retired; the caller must wait for a release.
    int acquire();
    // Attach the frame serial to an acquired slot. False when it is not busy.
    bool setSerial(int slot, uint64_t serial);
    // `wl_buffer.release` returns the busy slot with this serial.
    bool release(uint64_t serial);
    bool releaseSlot(int slot);
    // Return an acquired slot that was not committed (deferred/failure).
    bool abandon(int slot);

    // Retired slots that are already free, dropped from the ring so the caller
    // can destroy their FBO/export/wl_buffer. Busy retired slots are returned
    // by a later call after their release.
    QVector<int> takeRetired();

    bool isBusy(int slot) const;
    bool isRetired(int slot) const;
    bool isFree(int slot) const;
    bool contains(int slot) const;

    int liveCount() const;
    int busyCount() const;
    int freeCount() const;
    quint64 allocations() const { return m_allocations; }
    quint64 reuses() const { return m_reuses; }
    quint64 resets() const { return m_resets; }
    void reset();

private:
    struct Slot {
        bool used = false;
        bool busy = false;
        bool retired = false;
        uint64_t serial = 0;
    };

    int allocateSlot();
    int findFreeCurrent() const;
    void retireLiveSlots();

    int m_capacity;
    QSize m_size;
    uint32_t m_format = 0;
    uint64_t m_modifier = 0;
    QVector<Slot> m_slots;
    quint64 m_allocations = 0;
    quint64 m_reuses = 0;
    quint64 m_resets = 0;
};
