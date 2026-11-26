// SPDX-License-Identifier: GPL-3.0-only
// Lock-before-sleep tests (T318): a fake `org.freedesktop.login1` emits
// `PrepareForSleep` on a private bus. The gate holds one sleep delay
// inhibitor, requests the lock once, waits for the compositor's confirmation
// before the handler returns, ignores duplicate hooks, and on wake only
// refreshes the scene and DPMS. The host logind is never contacted
// (`DBUS_SYSTEM_BUS_ADDRESS` points at a dead path).
#include "suspendlock.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusContext>
#include <QDBusUnixFileDescriptor>
#include <QElapsedTimer>
#include <QSignalSpy>
#include <QTimer>
#include <QtTest>

#include <unistd.h>

namespace {

const QString kLogindService = QStringLiteral("org.freedesktop.login1");
const QString kLogindPath = QStringLiteral("/org/freedesktop/login1");

// A fake logind manager. `Inhibit` hands back a pipe read end like the real
// service (the client keeps it open); the write end stays here so the pipe
// stays valid for the client's copy. `PrepareForSleep` is the real wire
// signal, so the gate's subscription path is exercised end to end.
class FakeLogind : public QObject, protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.freedesktop.login1.Manager")

public:
    ~FakeLogind() override
    {
        for (int fd : m_writeEnds)
            ::close(fd);
    }

    int inhibitCalls() const { return m_inhibitCalls; }
    QString lastWhat() const { return m_lastWhat; }
    QString lastMode() const { return m_lastMode; }

public slots:
    QDBusUnixFileDescriptor Inhibit(const QString &what, const QString &who,
                                    const QString &why, const QString &mode)
    {
        Q_UNUSED(who);
        Q_UNUSED(why);
        ++m_inhibitCalls;
        m_lastWhat = what;
        m_lastMode = mode;
        int fds[2];
        if (::pipe(fds) != 0)
            return QDBusUnixFileDescriptor();
        m_writeEnds.append(fds[1]);
        return QDBusUnixFileDescriptor(fds[0]);
    }

signals:
    void PrepareForSleep(bool entering);

private:
    int m_inhibitCalls = 0;
    QString m_lastWhat;
    QString m_lastMode;
    QList<int> m_writeEnds;
};

bool registerLogind(QDBusConnection &bus, FakeLogind &service)
{
    if (!bus.registerService(kLogindService))
        return false;
    return bus.registerObject(kLogindPath, &service,
                              QDBusConnection::ExportAllSlots
                                  | QDBusConnection::ExportAllSignals);
}

QString busAddress()
{
    return QString::fromLocal8Bit(qgetenv("DBUS_SESSION_BUS_ADDRESS"));
}

// The shell-side effects. `lock` is the one `lockScreen()`/`lockSession()`
// path with a deliberately asynchronous confirmation:
// the compositor answers after the request, not inside it, so the gate must
// pump the GUI loop before it can report a confirmed lock.
struct MockShell
{
    QObject context;
    int lockSessionCalls = 0;
    bool locked = false;
    bool lockReturns = true;
    bool confirm = true;
    int refreshCalls = 0;
    int dpmsCalls = 0;
    bool dpmsOn = false;
    int confirmDelayMs = 40;

    SuspendLockEffects effects()
    {
        SuspendLockEffects e;
        e.lock = [this]() {
            ++lockSessionCalls;
            if (lockReturns && confirm) {
                QTimer::singleShot(confirmDelayMs, &context,
                                   [this]() { locked = true; });
            }
            return lockReturns;
        };
        e.isLocked = [this]() { return locked; };
        e.refresh = [this]() { ++refreshCalls; };
        e.wakeDpms = [this]() {
            ++dpmsCalls;
            dpmsOn = true;
        };
        return e;
    }
};

} // namespace

class TestLockSuspend : public QObject
{
    Q_OBJECT

private slots:
    void cleanup()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (bus.isConnected()) {
            bus.unregisterObject(kLogindPath);
            bus.unregisterService(kLogindService);
        }
    }

    void enteringLocksOnceAndConfirmsBeforeTheHandlerReturns()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        QVERIFY(registerLogind(bus, service));

        MockShell shell;
        SuspendLock gate(shell.effects());
        gate.setBusAddress(busAddress());
        QVERIFY(gate.start());
        QTRY_COMPARE(service.inhibitCalls(), 1);
        QTRY_VERIFY(gate.holdsInhibitor());
        QCOMPARE(service.lastWhat(), QStringLiteral("sleep"));
        QCOMPARE(service.lastMode(), QStringLiteral("delay"));

        QSignalSpy locked(&gate, &SuspendLock::sleepLocked);
        QSignalSpy resumed(&gate, &SuspendLock::resumed);
        // The signal is emitted synchronously by the handler, so this records
        // the state exactly when the handler is about to return.
        bool confirmedWhenHandlerReturned = false;
        connect(&gate, &SuspendLock::sleepLocked, this,
                [&]() { confirmedWhenHandlerReturned = shell.locked; },
                Qt::DirectConnection);

        emit service.PrepareForSleep(true);
        QTRY_COMPARE(locked.count(), 1);
        QCOMPARE(locked.at(0).at(0).toBool(), true);
        QCOMPARE(shell.lockSessionCalls, 1);
        QVERIFY(shell.locked);
        QVERIFY(confirmedWhenHandlerReturned);
        QVERIFY(gate.asleep());
        QVERIFY(gate.lockedForSleep());
        // The inhibitor is released so logind may sleep now that the lock is
        // confirmed; it is not what keeps the session locked.
        QVERIFY(!gate.holdsInhibitor());

        // A duplicate entering hook does not request the lock again.
        emit service.PrepareForSleep(true);
        QTest::qWait(150);
        QCOMPARE(shell.lockSessionCalls, 1);
        QCOMPARE(locked.count(), 1);

        // A wake refreshes the scene and DPMS; it never unlocks.
        emit service.PrepareForSleep(false);
        QTRY_COMPARE(resumed.count(), 1);
        QCOMPARE(shell.refreshCalls, 1);
        QCOMPARE(shell.dpmsCalls, 1);
        QVERIFY(shell.dpmsOn);
        QCOMPARE(shell.lockSessionCalls, 1);
        QVERIFY(shell.locked);
        QVERIFY(!gate.asleep());
        // A fresh delay inhibitor is held for the next sleep.
        QTRY_COMPARE(service.inhibitCalls(), 2);
        QTRY_VERIFY(gate.holdsInhibitor());

        // A duplicate wake is idempotent too.
        emit service.PrepareForSleep(false);
        QTest::qWait(100);
        QCOMPARE(resumed.count(), 1);
        QCOMPARE(shell.refreshCalls, 1);

        bus.unregisterObject(kLogindPath);
        bus.unregisterService(kLogindService);
    }

    void aLockThatNeverConfirmsReleasesTheInhibitorAfterTheBound()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        QVERIFY(registerLogind(bus, service));

        MockShell shell;
        shell.confirm = false;
        SuspendLock gate(shell.effects());
        gate.setBusAddress(busAddress());
        gate.setConfirmTimeoutMs(120);
        QVERIFY(gate.start());
        QTRY_VERIFY(gate.holdsInhibitor());

        QSignalSpy locked(&gate, &SuspendLock::sleepLocked);
        QElapsedTimer timer;
        timer.start();
        gate.handlePrepareForSleep(true);
        QCOMPARE(locked.count(), 1);
        QCOMPARE(locked.at(0).at(0).toBool(), false);
        QVERIFY(timer.elapsed() >= 100);
        QCOMPARE(shell.lockSessionCalls, 1);
        QVERIFY(!gate.holdsInhibitor());
        QVERIFY(gate.asleep());
    }

    void aMissingLockPathDoesNotWaitOrHoldTheMachine()
    {
        MockShell shell;
        shell.lockReturns = false;
        SuspendLock gate(shell.effects());
        gate.setConfirmTimeoutMs(5000);

        QSignalSpy locked(&gate, &SuspendLock::sleepLocked);
        QElapsedTimer timer;
        timer.start();
        gate.handlePrepareForSleep(true);
        QCOMPARE(locked.count(), 1);
        QCOMPARE(locked.at(0).at(0).toBool(), false);
        QVERIFY(timer.elapsed() < 1000);
        QCOMPARE(shell.lockSessionCalls, 1);
        QVERIFY(!gate.holdsInhibitor());
    }

    void absentLogindLeavesTheShellRunning()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        if (bus.interface()->isServiceRegistered(kLogindService))
            QSKIP("the private bus unexpectedly owns the logind name");

        MockShell shell;
        SuspendLock gate(shell.effects());
        gate.setBusAddress(busAddress());
        // Subscribing to a well-known name succeeds before its owner exists;
        // the inhibitor request fails and is logged, but the gate still locks.
        QVERIFY(gate.start());
        QTest::qWait(100);
        QVERIFY(!gate.holdsInhibitor());

        gate.handlePrepareForSleep(true);
        QCOMPARE(shell.lockSessionCalls, 1);
        QVERIFY(gate.lockedForSleep());
        QVERIFY(!gate.holdsInhibitor());
    }

    void anUnreachableBusIsNotFatal()
    {
        MockShell shell;
        SuspendLock gate(shell.effects());
        gate.setBusAddress(
            QStringLiteral("unix:path=/nonexistent-dragonfruit-suspendlock-bus"));
        QVERIFY(!gate.start());
        // A synthetic trigger still follows the same path.
        gate.handlePrepareForSleep(true);
        QCOMPARE(shell.lockSessionCalls, 1);
        QVERIFY(gate.lockedForSleep());
    }
};

QTEST_MAIN(TestLockSuspend)
#include "tst_locksuspend.moc"
