// SPDX-License-Identifier: GPL-3.0-only
// The shell's one session-ending path (T313).
//
// Log Out is a session action, not an application action: the shell asks
// logind to terminate its own session, the compositor (the session anchor)
// exits, and the T-12.2 session entry stops the target and returns to the
// greeter. The shell never signals itself or the compositor directly.
//
// The class is Wayland-free and the bus is injectable, so `tst_sessionactions`
// can run a fake `org.freedesktop.login1` on a private bus and assert the
// method and session id without a real logind. `DRAGONFRUIT_NO_LOGOUT` is the
// fixture/capture no-op: it records the intended call and never reaches D-Bus
// or systemctl, so a test can never end the developer's session.
#pragma once

#include <QDBusConnection>
#include <QObject>
#include <QString>

class SessionActions : public QObject
{
    Q_OBJECT

public:
    explicit SessionActions(QObject *parent = nullptr);

    // Test seams. `setBusAddress` connects to a private bus instead of the
    // system bus; `setSessionId` overrides XDG_SESSION_ID; `setSystemctlPath`
    // overrides the fallback binary (a non-executable path makes the fallback
    // unavailable).
    void setBusAddress(const QString &address);
    void setSessionId(const QString &sessionId);
    void setSystemctlPath(const QString &path);

    // True when logind owns `org.freedesktop.login1` on the active bus.
    bool logindAvailable();

    // Empty when the session can be ended (logind, or the systemctl fallback);
    // otherwise the user-facing reason the Log Out row is disabled.
    QString unavailableReason();

    // `DRAGONFRUIT_NO_LOGOUT` is the fixture/capture seam: the request is
    // recorded but no D-Bus call or systemctl process is started.
    static bool noLogoutFixture();

    // Ask logind to terminate this session. Returns true when the request was
    // handed to D-Bus (or recorded in fixture mode). When logind is absent it
    // falls back to stopping `dragonfruit-session.target`; false means there
    // is no session id or neither path could be started.
    bool requestLogout();

    // The last request, for the fixture and for diagnostics.
    QString lastMethod() const { return m_lastMethod; }
    QString lastSessionId() const { return m_lastSessionId; }
    bool lastWasFixture() const { return m_lastWasFixture; }

signals:
    // Emitted once a request was handed off (or recorded in fixture mode).
    void dispatched(const QString &method, const QString &sessionId);
    // Emitted when no path could be used; the shell logs it and carries on.
    void failed(const QString &reason);

private:
    QDBusConnection bus() const;
    QString sessionId() const;
    QString systemctlPath() const;

    QString m_busAddress;
    QString m_sessionId;
    QString m_systemctlPath;
    QString m_lastMethod;
    QString m_lastSessionId;
    bool m_lastWasFixture = false;
};
