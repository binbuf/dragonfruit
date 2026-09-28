// SPDX-License-Identifier: MIT
// The Settings app's Notifications and Focus seam (T-15.7b).
//
// The Notifications and Focus panes never touch D-Bus: they bind the
// `Settings` singleton, and the bridge forwards reads and the two Focus writes
// here. The live `DbusNotificationsClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Notifications` interface (the same adapter
// view the Focus tile and the notification service share), and
// `MockNotificationsClient` serves a deterministic fixture for the headless
// pane tests (`DF_NOTIFICATIONS_FIXTURE`).
//
// The four global presentation preferences (`Show previews`, the sleeping /
// locked / mirroring toggles) are settingsd keys, not this view: the panes
// write them through `Settings.set(...)` like every other desktop preference.
//
// Absence is normal: the host itself may be absent (`available()` is false) or
// a foreign notification daemon that does not serve the shell interface makes
// the view `unavailable`; the panes render the absence state without error.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantMap>

class QDBusServiceWatcher;

class NotificationsClient : public QObject
{
    Q_OBJECT

public:
    explicit NotificationsClient(QObject *parent = nullptr) : QObject(parent) {}
    ~NotificationsClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `notifications` view (`{state, glyph, label, mode, modeLabel,
    // suppressing, dnd, batched, allowList, activeCount, historyCount,
    // appCount, apps}`); empty when the host is absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the panes call this on open).
    virtual void refresh() = 0;
    // Set the Focus/DND mode by its stable id (`off`/`focus`/`dnd`). One
    // explicit write; the host re-reads and pushes the new view.
    virtual void setFocusMode(const QString &mode) = 0;
    // Replace the per-app Focus allow list. One explicit write.
    virtual void setFocusAllowList(const QStringList &apps) = 0;

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusNotificationsClient : public NotificationsClient
{
    Q_OBJECT

public:
    explicit DbusNotificationsClient(QObject *parent = nullptr);
    ~DbusNotificationsClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setFocusMode(const QString &mode) override;
    void setFocusAllowList(const QStringList &apps) override;

private:
    void call(const QString &method, const QVariantList &arguments);
    void applyReply(const QByteArray &json);
    QString m_service;
    QString m_path;
    QString m_interface;
    QVariantMap m_view;
    bool m_available = false;
    QDBusServiceWatcher *m_watcher = nullptr;
};

// The fixture client used by `DF_NOTIFICATIONS_FIXTURE`: a Focus policy with
// one allowed app and three observed apps. Both Focus writes mutate the
// simulated policy and re-emit, so a pane control's round-trip is observable
// with no bus.
class MockNotificationsClient : public NotificationsClient
{
    Q_OBJECT

public:
    explicit MockNotificationsClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setFocusMode(const QString &mode) override;
    void setFocusAllowList(const QStringList &apps) override;

private:
    void rebuild();
    QString m_mode = QStringLiteral("focus");
    QStringList m_allowList = QStringList{ QStringLiteral("Pager") };
    QVariantMap m_view;
};