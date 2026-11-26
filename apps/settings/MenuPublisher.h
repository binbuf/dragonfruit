// SPDX-License-Identifier: GPL-3.0-or-later
// The Settings app's native global-menu publisher (T230, T-14.2b).
//
// The menu-broker is the single owner of the resolved menu. A first-party app
// publishes its design-system model once with `Publish(appId, modelJson)` and
// withdraws it on exit; the app id is the canonical desktop id so the shell's
// focus resolution (raw Wayland app_id → app-index desktop id) and the broker
// key agree. The model itself stays in `SettingsMenu.qml`; this class is only
// the D-Bus transport, never a second model.
//
// The same connection also serves the publisher-side activation endpoint the
// broker calls after validating the app's current publication generation. An
// activation is delivered synchronously to QML through
// `activationRequested`; QML calls `completeActivation` with the local
// dispatcher's result, which becomes the D-Bus reply. A command the app does
// not handle is therefore rejected instead of silently reported as accepted.
#pragma once

#include <QDBusConnection>
#include <QObject>
#include <QString>

class QDBusServiceWatcher;

class MenuPublisher : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.MenuPublisher1")

public:
    explicit MenuPublisher(QObject *parent = nullptr);
    ~MenuPublisher() override;

    // Publish or replace `appId`'s model. Returns whether the broker accepted
    // the call; an absent broker is a normal miss, not an app failure. The
    // model is retained and re-published when the broker (re)appears.
    Q_INVOKABLE bool publish(const QString &appId, const QString &modelJson);

    // Remove `appId`'s model (best effort).
    Q_INVOKABLE bool withdraw(const QString &appId);

    // QML calls this with the local dispatcher's result while
    // `activationRequested` is being emitted.
    Q_INVOKABLE void completeActivation(bool handled);

signals:
    // A validated application action arrived from the broker. `itemJson` is
    // the published row (identity and enabled state); `generation` is the
    // publication the shell rendered; `windowId` is the focused compositor
    // window the action targets (0 when the request is app-wide).
    void activationRequested(const QString &action, const QString &itemJson,
                             qulonglong generation, uint windowId);

public slots:
    // The publisher-side D-Bus method. The broker addresses this on the
    // publisher's unique bus name, so another instance cannot receive it.
    bool Activate(const QString &appId, const QString &action, const QString &itemJson,
                  qulonglong generation, uint windowId);

private:
    void ensureRegistered();
    void republish();

    QString m_appId;
    QString m_modelJson;
    bool m_registered = false;
    bool m_published = false;
    bool m_activationPending = false;
    bool m_activationHandled = false;
    QDBusServiceWatcher *m_brokerWatcher = nullptr;
};
