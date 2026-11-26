// SPDX-License-Identifier: GPL-3.0-or-later
// Shell watchdog tests (T314): the heartbeat is an event-loop timer on the
// GUI thread, so a GUI-thread block stops the beats and systemd's
// `WatchdogSec=30` restarts the shell. The same class owns the stable
// `restart-shell` action spelling the compositor routes for Ctrl+Alt+Esc.
//
// Pure Qt: no Wayland, D-Bus, QML, or systemd. The `T314_WATCHDOG_PROBE`
// environment mode is a scripted systemd check (see
// `scripts/t314-watchdog-check.sh`); it is never selected by ctest.
#include <QtTest>

#include <QCoreApplication>
#include <QFile>
#include <QSignalSpy>
#include <QThread>
#include <QTimer>

#include "shellwatchdog.h"

class TestShellWatchdog : public QObject
{
    Q_OBJECT

private slots:
    void restartActionMapping();
    void readyUsesTheNotifyPayload();
    void heartbeatUsesTheWatchdogPayload();
    void heartbeatTimerRunsOnTheGuiThread();
    void blockingTheGuiThreadStopsTheHeartbeat();
    void stoppingTheTimerStopsTheBeats();
};

void TestShellWatchdog::restartActionMapping()
{
    QVERIFY(ShellWatchdog::isRestartAction(QStringLiteral("restart-shell")));
    QVERIFY(!ShellWatchdog::isRestartAction(QStringLiteral("lock-screen")));
    QVERIFY(!ShellWatchdog::isRestartAction(QStringLiteral("log-out")));
    QVERIFY(!ShellWatchdog::isRestartAction(QString()));
    QCOMPARE(QString::fromLatin1(ShellWatchdog::restartActionName()),
             QStringLiteral("restart-shell"));
}

void TestShellWatchdog::readyUsesTheNotifyPayload()
{
    ShellWatchdog watchdog;
    QByteArray payload;
    watchdog.setNotifier([&payload](const QByteArray &state) {
        payload = state;
        return true;
    });
    QVERIFY(watchdog.notifyReady());
    QCOMPARE(payload, QByteArrayLiteral("READY=1"));
}

void TestShellWatchdog::heartbeatUsesTheWatchdogPayload()
{
    ShellWatchdog watchdog;
    QByteArray payload;
    watchdog.setNotifier([&payload](const QByteArray &state) {
        payload = state;
        return true;
    });
    QSignalSpy sent(&watchdog, &ShellWatchdog::heartbeatSent);
    QVERIFY(watchdog.heartbeat());
    QCOMPARE(payload, QByteArrayLiteral("WATCHDOG=1"));
    QCOMPARE(sent.count(), 1);
}

void TestShellWatchdog::heartbeatTimerRunsOnTheGuiThread()
{
    ShellWatchdog watchdog;
    QThread *callbackThread = nullptr;
    int beats = 0;
    watchdog.setNotifier([&](const QByteArray &state) {
        if (state == QByteArrayLiteral("WATCHDOG=1")) {
            ++beats;
            callbackThread = QThread::currentThread();
        }
        return true;
    });

    watchdog.start(20);
    QVERIFY(watchdog.isRunning());
    QTRY_VERIFY_WITH_TIMEOUT(beats >= 2, 3000);
    watchdog.stop();
    QVERIFY(!watchdog.isRunning());
    QCOMPARE(callbackThread, QThread::currentThread());
}

void TestShellWatchdog::blockingTheGuiThreadStopsTheHeartbeat()
{
    ShellWatchdog watchdog;
    int beats = 0;
    watchdog.setNotifier([&beats](const QByteArray &state) {
        if (state == QByteArrayLiteral("WATCHDOG=1"))
            ++beats;
        return true;
    });

    watchdog.start(20);
    QTRY_VERIFY_WITH_TIMEOUT(beats >= 1, 3000);
    const int before = beats;
    // The timer runs on this thread; a GUI-thread block cannot deliver a
    // beat. A worker heartbeat would keep incrementing here.
    QThread::msleep(120);
    QCOMPARE(beats, before);
    watchdog.stop();
}

void TestShellWatchdog::stoppingTheTimerStopsTheBeats()
{
    ShellWatchdog watchdog;
    int beats = 0;
    watchdog.setNotifier([&beats](const QByteArray &state) {
        if (state == QByteArrayLiteral("WATCHDOG=1"))
            ++beats;
        return true;
    });

    watchdog.start(20);
    QTRY_VERIFY_WITH_TIMEOUT(beats >= 1, 3000);
    watchdog.stop();
    const int stopped = beats;
    QTest::qWait(100);
    QCOMPARE(beats, stopped);
}

namespace {

// Scripted systemd check only. The probe sends READY, heartbeats, and then
// blocks the GUI thread on its first run. systemd's watchdog kills it and
// Restart=on-failure starts run 2, which exits cleanly after heartbeating.
int runWatchdogProbe(int argc, char **argv)
{
    QCoreApplication app(argc, argv);

    int run = 0;
    const QString counterPath = qEnvironmentVariable("T314_PROBE_COUNTER");
    if (!counterPath.isEmpty()) {
        QFile counter(counterPath);
        if (counter.open(QIODevice::ReadOnly))
            run = QString::fromUtf8(counter.readAll()).trimmed().toInt();
        counter.close();
        ++run;
        if (counter.open(QIODevice::WriteOnly | QIODevice::Truncate))
            counter.write(QByteArray::number(run));
        counter.close();
    }

    ShellWatchdog watchdog;
    if (!watchdog.load()) {
        fprintf(stderr, "t314-watchdog-probe: libsystemd/sd_notify unavailable\n");
        return 2;
    }
    if (!watchdog.notifyReady()) {
        fprintf(stderr, "t314-watchdog-probe: READY failed (NOTIFY_SOCKET unset?)\n");
        return 3;
    }
    // Faster than the shipped 10 s beat so the probe finishes promptly; the
    // timer is still the same GUI-thread path.
    watchdog.start(1000);

    if (run >= 2) {
        // The restart happened. Prove the loop can still beat, then exit 0 so
        // Restart=on-failure does not start a third run.
        QTimer::singleShot(2500, &app, []() { QCoreApplication::exit(0); });
    } else {
        // A simulated GUI-thread deadlock. The watchdog timer cannot fire, so
        // systemd kills the unit after WatchdogSec.
        QTimer::singleShot(2500, &app, []() { QThread::sleep(60); });
    }
    return app.exec();
}

} // namespace

int main(int argc, char **argv)
{
    if (qEnvironmentVariableIsSet("T314_WATCHDOG_PROBE"))
        return runWatchdogProbe(argc, argv);
    QCoreApplication app(argc, argv);
    TestShellWatchdog test;
    return QTest::qExec(&test, argc, argv);
}

#include "tst_shellwatchdog.moc"
