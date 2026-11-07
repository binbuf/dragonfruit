// SPDX-License-Identifier: MIT
// Notification Center state (T204).
//
// The notification service owns the queue, the history, expiry, Focus/DND and
// the banner. The shell owns one decoded history view and this small,
// Wayland-free controller: it normalizes the service's `History()` payload
// into stable-keyed rows, tracks the keyboard selection, and turns an
// activation/dismissal into the existing `NotificationClient` contract.
//
// A row is *active* only while the service still has the notification open
// (no `reason`, no `closedAt`) and Focus/DND did not suppress it. Every other
// row is history: it keeps its identity and display fields, but it carries no
// actions, so a stale action can never be invoked after expiry, dismissal,
// close or DND suppression.
#pragma once

#include <QObject>
#include <QVariantList>
#include <QVariantMap>

class NotificationCenter : public QObject
{
    Q_OBJECT

public:
    explicit NotificationCenter(QObject *parent = nullptr);

    // The normalized history, most recent first. Each map carries the service
    // fields plus `key` (stable id string), `active`, `canAct`, `status` and
    // the filtered `actions` list.
    QVariantList history() const { return m_history; }
    bool available() const { return m_available; }
    bool isOpen() const { return m_open; }
    int selectedIndex() const { return m_selectedIndex; }
    QString selectedKey() const;
    bool isEmpty() const { return m_history.isEmpty(); }
    int activeCount() const;

    // Normalize a decoded service `History()` list (public for tests).
    static QVariantList normalizeHistory(const QVariantList &raw);

public slots:
    // The service appeared or went away. Absence drops the stale rows; the
    // Center stays open and shows the unavailable empty state, and a later
    // reconnect repopulates it through the normal `History()` refresh.
    void setAvailable(bool available);
    // Apply a decoded `History()` list. Selection follows the entry's stable
    // key across updates, not its row index.
    void setHistory(const QVariantList &raw);
    // Drop the view and selection (lock/output teardown).
    void clear();

    void open();
    void close(const QString &reason = QString());
    void toggle();

    // Keyboard traversal. `moveSelection` clamps at the ends.
    void moveSelection(int delta);
    void selectIndex(int index);
    void selectFirst();
    void selectLast();

    // Return activates the selected active notification: its `default`
    // action when present, otherwise dismissal. Delete/Backspace call
    // `dismissSelected` directly. Both are no-ops for history-only rows.
    bool activateSelected();
    bool dismissSelected();

    // A pointer action on one row. `id` must still be active and (for
    // `invoke`) the action key must still be advertised.
    bool invoke(quint32 id, const QString &actionKey);
    bool dismiss(quint32 id);

signals:
    void changed();
    void openChanged(bool open);
    void invokeRequested(quint32 id, const QString &actionKey);
    void dismissRequested(quint32 id);
    // Emitted once when an open Center closes, with the reason used by the
    // shell (`escape`, `outside`, `lock`, `output`, `competing`, `closed`).
    void closed(const QString &reason);

private:
    QVariantMap entryAt(int index) const;
    QVariantMap entryById(quint32 id) const;
    void rebuildSelection(const QString &preferredKey);
    void setSelectedIndex(int index);

    QVariantList m_history;
    bool m_available = true;
    bool m_open = false;
    int m_selectedIndex = -1;
    QString m_selectedKey;
};
