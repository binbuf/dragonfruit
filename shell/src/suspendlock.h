// SPDX-License-Identifier: GPL-3.0-or-later
// The shell's lock-before-sleep gate (T318).
//
// logind emits `PrepareForSleep(true)` before a suspend, whether the request
// came from the system menu, the idle chain, or a lid close. The gate holds a
// logind "sleep" delay inhibitor while awake so logind waits for it, requests
// the existing fail-secure lock, and releases the inhibitor only once the
// compositor confirms the lock. A wake refreshes the lock scene and DPMS. The
// gate has no unlock effect, so a wake can never clear the lock; if the shell
// is gone while locked, the compositor's T306 policy ends the session.
//
// The class is Wayland-free and the bus is injectable, so `tst_locksuspend`
// runs a fake `org.freedesktop.login1` on a private bus.
#pragma once

#include <QDBusConnection>
#include <QDBusUnixFileDescriptor>
#include <QObject>
#include <QString>

#include <functional>

// The shell-owned effects of one sleep cycle. Tests bind recording mocks.
struct SuspendLockEffects {
    // Request the one fail-secure lock (`lockScreen()`). False when there is
    // no lock path at all (no protocol yet, or the compositor refused).
    std::function<bool()> lock;
    // The compositor's lock confirmation
    // (`ShellProtocol::isSessionLocked`).
    std::function<bool()> isLocked;
    // Repaint the lock scene after a wake (the clock and every lock surface).
    std::function<void()> refresh;
    // Turn the panel back on after a wake (`df_output.set_dpms(true)`).
    std::function<void()> wakeDpms;
};

class SuspendLock : public QObject
{
    Q_OBJECT

public:
    explicit SuspendLock(SuspendLockEffects effects, QObject *parent = nullptr);

    // Test seams. `setBusAddress` must run before `start()`; an empty address
    // means the system bus. `setConfirmTimeoutMs` shortens the bounded wait
    // for the compositor's lock confirmation.
    void setBusAddress(const QString &address);
    void setConfirmTimeoutMs(int milliseconds);

    // Subscribe to logind's `PrepareForSleep` and take the sleep delay
    // inhibitor. Returns false when the bus is unavailable; the shell still
    // runs and simply cannot lock on sleep.
    bool start();

    // The one handler. Public so a synthetic trigger can drive the same path
    // the signal uses.
    void handlePrepareForSleep(bool entering);

    bool asleep() const { return m_asleep; }
    bool lockedForSleep() const { return m_lockedForSleep; }
    bool holdsInhibitor() const { return m_inhibitor.isValid(); }
    int confirmTimeoutMs() const { return m_confirmTimeoutMs; }

signals:
    // Emitted once an entering signal is handled. `confirmed` is whether the
    // compositor reported the lock before the bounded wait expired.
    void sleepLocked(bool confirmed);
    // Emitted once a wake refreshed the scene and DPMS.
    void resumed();

private slots:
    void onPrepareForSleep(bool entering);

private:
    QDBusConnection connection();
    void acquireInhibitor();
    void releaseInhibitor();
    bool waitForLock();

    SuspendLockEffects m_effects;
    QString m_busAddress;
    // The dedicated connection used when a test injects a bus address. One
    // name per gate instance, so two gates in one process cannot steal each
    // other's subscription.
    QString m_testConnectionName;
    int m_confirmTimeoutMs = 2000;
    bool m_started = false;
    bool m_subscribed = false;
    bool m_asleep = false;
    bool m_lockedForSleep = false;
    // One wake refresh per sleep cycle; a duplicate `false` is a no-op.
    bool m_resumeHandled = false;
    // The inhibitor is held while awake and released once the lock is
    // confirmed so logind may proceed.
    bool m_wantInhibitor = true;
    bool m_inhibitPending = false;
    bool m_inhibitUnavailable = false;
    QDBusUnixFileDescriptor m_inhibitor;
};
