// SPDX-License-Identifier: GPL-3.0-or-later
// GUI-thread watchdog heartbeat (T314).
//
// The shell unit is `Type=notify` with `WatchdogSec=30`. systemd kills and
// restarts the process when it stops receiving `WATCHDOG=1`. The heartbeat is
// deliberately driven by a `QTimer` on the main (GUI) thread: a worker thread
// would keep beating through a GUI-thread deadlock, which is exactly the
// failure this watchdog must catch.
//
// `sd_notify` is resolved at runtime with `QLibrary`, so the shell builds and
// runs on hosts without the systemd development package. Outside systemd
// (`NOTIFY_SOCKET` absent) the timer stays off and the shell behaves as before.
#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>

#include <functional>

class QLibrary;
class QTimer;

class ShellWatchdog : public QObject
{
    Q_OBJECT

public:
    // The shipped unit's WatchdogSec and the heartbeat interval: three beats
    // per systemd window, so one missed timer tick is not a kill.
    static constexpr int kWatchdogSec = 30;
    static constexpr int kHeartbeatIntervalMs = kWatchdogSec * 1000 / 3;

    // Test seam: replace libsystemd with a plain callback. Returning false
    // means the notification could not be delivered.
    using Notifier = std::function<bool(const QByteArray &)>;

    explicit ShellWatchdog(QObject *parent = nullptr);
    ~ShellWatchdog() override;

    // Resolve `sd_notify` from libsystemd. Returns false when the library or
    // symbol is absent; the shell then has no watchdog (a non-systemd run).
    bool load();

    // Test seam. An injected notifier takes precedence over libsystemd.
    void setNotifier(Notifier notifier);

    // Send `READY=1` once the shell's event loop is running.
    bool notifyReady();

    // Send one `WATCHDOG=1` beat. Also the timer callback.
    bool heartbeat();

    // Start the GUI-thread heartbeat timer. `intervalMs <= 0` uses the
    // default. A no-op when libsystemd is unavailable and no notifier was
    // injected, or when the process is not running under systemd.
    void start(int intervalMs = kHeartbeatIntervalMs);
    void stop();
    bool isRunning() const;

    // The compositor routes this exact input-action name for Ctrl+Alt+Esc.
    // Shared with `ShellController` so the action mapping is unit-testable.
    static const char *restartActionName();
    static bool isRestartAction(const QString &action);

signals:
    void heartbeatSent();
    void heartbeatFailed();

private:
    bool send(const QByteArray &state);

    QTimer *m_timer = nullptr;
    QLibrary *m_library = nullptr;
    int (*m_sdNotify)(int, const char *) = nullptr;
    Notifier m_notifier;
};
