// SPDX-License-Identifier: MIT
// The Settings app's native global-menu publisher (T230).
//
// The menu-broker is the single owner of the resolved menu. A first-party app
// publishes its design-system model once with `Publish(appId, modelJson)` and
// withdraws it on exit; the app id is the canonical desktop id so the shell's
// focus resolution (raw Wayland app_id → app-index desktop id) and the broker
// key agree. The model itself stays in `SettingsMenu.qml`; this class is only
// the D-Bus transport, never a second model.
#pragma once

#include <QObject>
#include <QString>

class MenuPublisher : public QObject
{
    Q_OBJECT

public:
    explicit MenuPublisher(QObject *parent = nullptr);

    // Publish or replace `appId`'s model. Returns whether the broker accepted
    // the call; an absent broker is a normal miss, not an app failure.
    Q_INVOKABLE bool publish(const QString &appId, const QString &modelJson);

    // Remove `appId`'s model (best effort).
    Q_INVOKABLE bool withdraw(const QString &appId);
};
