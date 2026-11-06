// SPDX-License-Identifier: MIT
// Per-surface chrome buffer pool (T-21.1c).
//
// One pool belongs to one wl_surface. It tracks which slots are free, busy
// (attached and not yet released by the compositor), or retired by a surface
// resize. The Wayland resources are owned by ShellProtocol; this class is the
// pure state machine so the reuse/busy/release rules can be unit-tested
// without a compositor.
#pragma once

#include <QRect>
#include <QSize>
#include <QVector>
#include <QtGlobal>

class ChromeBufferPool
{
public:
    explicit ChromeBufferPool(int capacity = 3);

    // Change the buffer size. Every live slot becomes retired: its buffer is
    // kept until the compositor releases it, but it is never reused for the
    // new size. Returns true when the size actually changed.
    bool setSize(const QSize &size);
    QSize size() const { return m_size; }

    // Acquire a free slot at the current size. Returns -1 when every slot is
    // busy or retired; the caller must wait for a release. A new slot is
    // allocated only while the pool is below `capacity`.
    int acquire();

    // Record damage from the new frame and return the area that must be
    // copied to the acquired slot, including damage it missed while busy.
    QRect damageForCommit(int slot, const QRect &damage);

    // Release a busy slot. Returns true when it was busy.
    bool release(int slot);

    // Drop a released retired slot after the caller destroyed its resources.
    // Returns false when the slot is busy or still current.
    bool remove(int slot);

    // Retired slots that are already free, dropped from the pool and returned
    // so the caller can destroy their Wayland buffers. Busy retired slots are
    // dropped by `remove()` when they are released.
    QVector<int> takeRetired();

    bool isBusy(int slot) const;
    bool isRetired(int slot) const;
    bool contains(int slot) const;

    int capacity() const { return m_capacity; }
    // Every live slot, including retired-but-not-yet-released ones. Never
    // exceeds `capacity`.
    int liveCount() const;
    int busyCount() const;
    int freeCount() const;

    quint64 allocations() const { return m_allocations; }
    quint64 reuses() const { return m_reuses; }
    void reset();

private:
    struct Slot {
        bool used = false;
        bool busy = false;
        int generation = -1;
        QRect pendingDamage;
    };

    int allocateSlot();
    int findFreeCurrent() const;

    int m_capacity;
    QSize m_size;
    int m_generation = 0;
    QVector<Slot> m_slots;
    quint64 m_allocations = 0;
    quint64 m_reuses = 0;
};
