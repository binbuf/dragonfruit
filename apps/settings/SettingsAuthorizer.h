// SPDX-License-Identifier: MIT
// The Settings app's one privileged-action authorizer (T224).
//
// Settings never collects an administrator password. A privileged action calls
// the host polkit authority with `CheckAuthorization` and
// `AllowUserInteraction`; polkit routes the request to the shell's registered
// authentication agent, which renders the system prompt and owns the PAM
// conversation. The result is a plain boolean: this class never sees a
// password and never escalates privilege itself.
//
// The `DF_SETTINGS_AUTH_FIXTURE` seam (`allow`, `deny`, `cancel`,
// `wrong-password`, `unavailable`) makes the path deterministic for headless
// tests and captures; `DF_SETTINGS_POLKIT_BUS` points the live path at a
// private test bus. With no authority the check fails closed and the pane
// shows an inline message.
#pragma once

#include <QObject>
#include <QString>

class SettingsAuthorizer : public QObject
{
    Q_OBJECT

public:
    explicit SettingsAuthorizer(QObject *parent = nullptr);

    // Test/capture seam. Empty selects the live authority; one of `allow`,
    // `deny`, `cancel`, `wrong-password`, `unavailable` resolves locally.
    void setFixture(const QString &mode);
    QString fixture() const { return m_fixture; }

    // Ask the authority for `actionId`. The result arrives through
    // `finished(requestId, authorized, error)`; `error` is empty on success.
    void check(const QString &requestId, const QString &actionId,
               const QString &message);

signals:
    void finished(const QString &requestId, bool authorized, const QString &error);

private:
    QString m_fixture;
    QString m_busAddress;
    QString m_service;
};
