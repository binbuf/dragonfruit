// SPDX-License-Identifier: MIT
#include "notificationcenter.h"

#include <QSet>

namespace {

QString entryKey(const QVariantMap &entry, int index)
{
    const quint32 id = entry.value(QStringLiteral("id")).toUInt();
    return id != 0 ? QString::number(id)
                   : QStringLiteral("entry-%1").arg(index);
}

bool entryIsActive(const QVariantMap &entry)
{
    if (entry.value(QStringLiteral("suppressed")).toBool())
        return false;
    // The service sets both fields when a notification leaves the queue. A
    // row with a reason or close time is history even if a malformed payload
    // also carries actions.
    if (!entry.value(QStringLiteral("reason")).toString().isEmpty())
        return false;
    if (entry.value(QStringLiteral("closedAt")).toULongLong() != 0)
        return false;
    return true;
}

QString entryStatus(const QVariantMap &entry, bool active)
{
    if (entry.value(QStringLiteral("suppressed")).toBool())
        return QStringLiteral("suppressed");
    const QString reason = entry.value(QStringLiteral("reason")).toString();
    if (!reason.isEmpty())
        return reason;
    return active ? QStringLiteral("active") : QStringLiteral("closed");
}

QVariantList entryActions(const QVariantMap &entry, bool active)
{
    if (!active)
        return {};
    return entry.value(QStringLiteral("actions")).toList();
}

bool hasAction(const QVariantList &actions, const QString &key)
{
    for (const QVariant &action : actions) {
        if (action.toMap().value(QStringLiteral("key")).toString() == key)
            return true;
    }
    return false;
}

} // namespace

NotificationCenter::NotificationCenter(QObject *parent)
    : QObject(parent)
{
}

QString NotificationCenter::selectedKey() const
{
    return m_selectedKey;
}

int NotificationCenter::activeCount() const
{
    int count = 0;
    for (const QVariant &value : m_history) {
        if (value.toMap().value(QStringLiteral("active")).toBool())
            ++count;
    }
    return count;
}

QVariantList NotificationCenter::normalizeHistory(const QVariantList &raw)
{
    QVariantList out;
    out.reserve(raw.size());
    QSet<quint32> seen;
    for (int i = 0; i < raw.size(); ++i) {
        const QVariantMap source = raw.at(i).toMap();
        const quint32 id = source.value(QStringLiteral("id")).toUInt();
        // The service's history is newest first and ids are unique; a
        // duplicate id (a replacement or a malformed replay) keeps the first,
        // most recent row so the stable key stays one row.
        if (id != 0) {
            if (seen.contains(id))
                continue;
            seen.insert(id);
        }
        const bool active = entryIsActive(source);
        const QVariantList actions = entryActions(source, active);
        QVariantMap entry = source;
        entry.insert(QStringLiteral("key"), entryKey(source, i));
        entry.insert(QStringLiteral("active"), active);
        entry.insert(QStringLiteral("canAct"), active && !actions.isEmpty());
        entry.insert(QStringLiteral("status"), entryStatus(source, active));
        entry.insert(QStringLiteral("actions"), actions);
        out.append(entry);
    }
    return out;
}

void NotificationCenter::setAvailable(bool available)
{
    if (m_available == available)
        return;
    m_available = available;
    if (!available) {
        // Never leave a stale action row behind when the service goes away.
        m_history.clear();
        m_selectedIndex = -1;
        m_selectedKey.clear();
    }
    emit changed();
}

void NotificationCenter::setHistory(const QVariantList &raw)
{
    if (!m_available) {
        // A late reply from a service that already unregistered must not
        // resurrect stale rows; the reconnect refresh is the repopulation
        // path.
        if (!m_history.isEmpty()) {
            m_history.clear();
            m_selectedIndex = -1;
            m_selectedKey.clear();
            emit changed();
        }
        return;
    }
    const QString preferred = m_selectedKey;
    m_history = normalizeHistory(raw);
    rebuildSelection(preferred);
    emit changed();
}

void NotificationCenter::clear()
{
    m_history.clear();
    m_selectedIndex = -1;
    m_selectedKey.clear();
    emit changed();
}

void NotificationCenter::open()
{
    if (m_open)
        return;
    m_open = true;
    // Start on the first actionable row when there is one; otherwise the
    // first history row, so Up/Down/Home/End still walk the visible list.
    int first = 0;
    for (int i = 0; i < m_history.size(); ++i) {
        if (m_history.at(i).toMap().value(QStringLiteral("active")).toBool()) {
            first = i;
            break;
        }
    }
    setSelectedIndex(m_history.isEmpty() ? -1 : first);
    emit openChanged(true);
    emit changed();
}

void NotificationCenter::close(const QString &reason)
{
    if (!m_open)
        return;
    m_open = false;
    emit closed(reason);
    emit openChanged(false);
    emit changed();
}

void NotificationCenter::toggle()
{
    if (m_open)
        close(QStringLiteral("toggle"));
    else
        open();
}

void NotificationCenter::moveSelection(int delta)
{
    if (m_history.isEmpty())
        return;
    setSelectedIndex(m_selectedIndex + delta);
}

void NotificationCenter::selectIndex(int index)
{
    if (m_history.isEmpty())
        return;
    setSelectedIndex(index);
}

void NotificationCenter::selectFirst()
{
    if (!m_history.isEmpty())
        setSelectedIndex(0);
}

void NotificationCenter::selectLast()
{
    if (!m_history.isEmpty())
        setSelectedIndex(m_history.size() - 1);
}

bool NotificationCenter::activateSelected()
{
    const QVariantMap entry = entryAt(m_selectedIndex);
    if (entry.isEmpty() || !entry.value(QStringLiteral("active")).toBool())
        return false;
    const quint32 id = entry.value(QStringLiteral("id")).toUInt();
    const QVariantList actions = entry.value(QStringLiteral("actions")).toList();
    if (hasAction(actions, QStringLiteral("default")))
        return invoke(id, QStringLiteral("default"));
    return dismiss(id);
}

bool NotificationCenter::dismissSelected()
{
    const QVariantMap entry = entryAt(m_selectedIndex);
    if (entry.isEmpty() || !entry.value(QStringLiteral("active")).toBool())
        return false;
    return dismiss(entry.value(QStringLiteral("id")).toUInt());
}

bool NotificationCenter::invoke(quint32 id, const QString &actionKey)
{
    const QVariantMap entry = entryById(id);
    if (entry.isEmpty() || !entry.value(QStringLiteral("active")).toBool()
        || actionKey.isEmpty()) {
        return false;
    }
    const QVariantList actions = entry.value(QStringLiteral("actions")).toList();
    if (!hasAction(actions, actionKey))
        return false;
    emit invokeRequested(id, actionKey);
    return true;
}

bool NotificationCenter::dismiss(quint32 id)
{
    const QVariantMap entry = entryById(id);
    if (entry.isEmpty() || !entry.value(QStringLiteral("active")).toBool())
        return false;
    emit dismissRequested(id);
    return true;
}

QVariantMap NotificationCenter::entryAt(int index) const
{
    if (index < 0 || index >= m_history.size())
        return {};
    return m_history.at(index).toMap();
}

QVariantMap NotificationCenter::entryById(quint32 id) const
{
    for (const QVariant &value : m_history) {
        const QVariantMap entry = value.toMap();
        if (entry.value(QStringLiteral("id")).toUInt() == id)
            return entry;
    }
    return {};
}

void NotificationCenter::rebuildSelection(const QString &preferredKey)
{
    if (m_history.isEmpty()) {
        m_selectedIndex = -1;
        m_selectedKey.clear();
        return;
    }
    int index = -1;
    if (!preferredKey.isEmpty()) {
        for (int i = 0; i < m_history.size(); ++i) {
            if (m_history.at(i).toMap().value(QStringLiteral("key")).toString()
                == preferredKey) {
                index = i;
                break;
            }
        }
    }
    if (index < 0)
        index = m_selectedIndex >= 0 ? qMin(m_selectedIndex, m_history.size() - 1) : 0;
    setSelectedIndex(index);
}

void NotificationCenter::setSelectedIndex(int index)
{
    if (m_history.isEmpty()) {
        m_selectedIndex = -1;
        m_selectedKey.clear();
        return;
    }
    const int clamped = qBound(0, index, m_history.size() - 1);
    const QString key =
        m_history.at(clamped).toMap().value(QStringLiteral("key")).toString();
    if (clamped == m_selectedIndex && key == m_selectedKey)
        return;
    m_selectedIndex = clamped;
    m_selectedKey = key;
    emit changed();
}
