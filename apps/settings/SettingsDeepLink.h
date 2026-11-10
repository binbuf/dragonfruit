// SPDX-License-Identifier: MIT
// T232: the Settings app's small session-bus deep-link endpoint. The shell
// owns the menu-bar extras and raises `settings://pane/<id>` links; when the
// app is already running this object receives the link and the QML shell
// navigates without a second window. A cold launch carries the same link on
// the command line (`--deep-link <url>`).
#pragma once

#include <QObject>
#include <QString>

class SettingsDeepLink : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.SettingsApp1")

public:
    explicit SettingsDeepLink(QObject *parent = nullptr);

    // Claim the session-bus name and object. Returns false when another
    // instance already owns it (the caller then forwards the link and exits).
    bool registerService();

signals:
    void requested(const QString &url);

public slots:
    // D-Bus entry point: `org.dragonfruit.SettingsApp1.Open(url)`.
    void Open(const QString &url);
};
