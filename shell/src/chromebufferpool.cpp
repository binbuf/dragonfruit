// SPDX-License-Identifier: MIT
#include "chromebufferpool.h"

ChromeBufferPool::ChromeBufferPool(int capacity)
    : m_capacity(qMax(1, capacity))
{
}

bool ChromeBufferPool::setSize(const QSize &size)
{
    if (size == m_size)
        return false;
    m_size = size;
    ++m_generation;
    // Existing slots keep their generation and are retired. They stay alive
    // until the compositor releases them; `takeRetired()` drops the ones that
    // are already free.
    return true;
}

int ChromeBufferPool::findFreeCurrent() const
{
    for (int i = 0; i < m_slots.size(); ++i) {
        const Slot &slot = m_slots.at(i);
        if (slot.used && !slot.busy && slot.generation == m_generation)
            return i;
    }
    return -1;
}

int ChromeBufferPool::allocateSlot()
{
    for (int i = 0; i < m_slots.size(); ++i) {
        if (!m_slots.at(i).used) {
            Slot &slot = m_slots[i];
            slot.used = true;
            slot.busy = true;
            slot.generation = m_generation;
            ++m_allocations;
            return i;
        }
    }
    if (m_slots.size() >= m_capacity)
        return -1;
    Slot slot;
    slot.used = true;
    slot.busy = true;
    slot.generation = m_generation;
    m_slots.append(slot);
    ++m_allocations;
    return m_slots.size() - 1;
}

int ChromeBufferPool::acquire()
{
    if (m_size.isEmpty())
        return -1;
    const int free = findFreeCurrent();
    if (free >= 0) {
        m_slots[free].busy = true;
        ++m_reuses;
        return free;
    }
    return allocateSlot();
}

bool ChromeBufferPool::release(int slot)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || !entry.busy)
        return false;
    entry.busy = false;
    return true;
}

bool ChromeBufferPool::remove(int slot)
{
    if (slot < 0 || slot >= m_slots.size())
        return false;
    Slot &entry = m_slots[slot];
    if (!entry.used || entry.busy || entry.generation == m_generation)
        return false;
    entry.used = false;
    entry.generation = -1;
    return true;
}

QVector<int> ChromeBufferPool::takeRetired()
{
    QVector<int> retired;
    for (int i = 0; i < m_slots.size(); ++i) {
        Slot &slot = m_slots[i];
        if (slot.used && !slot.busy && slot.generation != m_generation) {
            slot.used = false;
            slot.generation = -1;
            retired.append(i);
        }
    }
    return retired;
}

bool ChromeBufferPool::isBusy(int slot) const
{
    return slot >= 0 && slot < m_slots.size() && m_slots.at(slot).used
        && m_slots.at(slot).busy;
}

bool ChromeBufferPool::isRetired(int slot) const
{
    return slot >= 0 && slot < m_slots.size() && m_slots.at(slot).used
        && m_slots.at(slot).generation != m_generation;
}

bool ChromeBufferPool::contains(int slot) const
{
    return slot >= 0 && slot < m_slots.size() && m_slots.at(slot).used;
}

int ChromeBufferPool::liveCount() const
{
    int live = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used)
            ++live;
    }
    return live;
}

int ChromeBufferPool::busyCount() const
{
    int busy = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used && slot.busy)
            ++busy;
    }
    return busy;
}

int ChromeBufferPool::freeCount() const
{
    int free = 0;
    for (const Slot &slot : m_slots) {
        if (slot.used && !slot.busy && slot.generation == m_generation)
            ++free;
    }
    return free;
}

void ChromeBufferPool::reset()
{
    m_slots.clear();
    m_size = QSize();
    m_generation = 0;
    m_allocations = 0;
    m_reuses = 0;
}
