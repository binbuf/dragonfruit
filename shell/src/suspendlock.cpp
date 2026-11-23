// SPDX-License-Identifier: MIT
#include "suspendlock.h"

#include <QCoreApplication>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDebug>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QVariant>

#include <utility>

namespace {

constexpr const char *kLogindService = "org.freedesktop.login1";
constexpr const char *kLogindPath = "/org/freedesktop/login1";
constexpr const char *kLogindManager = "org.freedesktop.login1.Manager";

// The logind delay window defaults to 5 s; this is the shell's own bound on
// waiting for the compositor's lock confirmation. It must be shorter than the
// logind window so a hung compositor does not hold the machine awake for the
// full delay allowance.
constexpr int kDefaultConfirmTimeoutMs = 2000;

} // namespace

SuspendLock::SuspendLock(SuspendLockEffects effects, QObject *parent)
    : QObject(parent)
    , m_effects(std::move(effects))
    , m_confirmTimeoutMs(kDefaultConfirmTimeoutMs)
{
}

void SuspendLock::setBusAddress(const QString &address)
{
    if (m_busAddress == address)
        return;
    if (!m_testConnectionName.isEmpty()) {
        QDBusConnection::disconnectFromBus(m_testConnectionName);
        m_testConnectionName.clear();
    }
    m_busAddress = address;
}

void SuspendLock::setConfirmTimeoutMs(int milliseconds)
{
    m_confirmTimeoutMs = qBound(0, milliseconds, 30'000);
}

QDBusConnection SuspendLock::connection()
{
    if (m_busAddress.isEmpty())
        return QDBusConnection::systemBus();
    if (m_testConnectionName.isEmpty()) {
        m_testConnectionName =
            QStringLiteral("dragonfruit-suspendlock-%1")
                .arg(reinterpret_cast<quintptr>(this), 0, 16);
    }
    return QDBusConnection::connectToBus(m_busAddress, m_testConnectionName);
}

bool SuspendLock::start()
{
    if (m_started)
        return m_subscribed;
    m_started = true;

    QDBusConnection bus = connection();
    if (!bus.isConnected()) {
        qWarning() << "shell: logind is not reachable; the session will not"
                   << "lock on sleep";
        return false;
    }
    m_subscribed = bus.connect(
        QString::fromLatin1(kLogindService), QString::fromLatin1(kLogindPath),
        QString::fromLatin1(kLogindManager), QStringLiteral("PrepareForSleep"),
        this, SLOT(onPrepareForSleep(bool)));
    if (!m_subscribed) {
        qWarning() << "shell: could not subscribe to logind PrepareForSleep";
        return false;
    }
    // Take the delay inhibitor now, before any suspend request, because logind
    // emits PrepareForSleep before it checks for delay inhibitors. The
    // inhibitor is what makes the machine wait for the lock to confirm.
    acquireInhibitor();
    return true;
}

void SuspendLock::onPrepareForSleep(bool entering)
{
    handlePrepareForSleep(entering);
}

void SuspendLock::handlePrepareForSleep(bool entering)
{
    if (entering) {
        if (m_asleep) {
            // A duplicate hook (or a re-entrant signal during the wait) is
            // idempotent: one lock request per sleep cycle.
            qInfo() << "shell: duplicate PrepareForSleep(true) ignored";
            return;
        }
        m_asleep = true;
        m_resumeHandled = false;
        // Stop wanting the inhibitor while asleep. If an acquire reply is
        // still in flight it is dropped when it arrives.
        m_wantInhibitor = false;

        const bool requested = m_effects.lock && m_effects.lock();
        m_lockedForSleep = requested && waitForLock();
        // The lock is confirmed (or the bound expired); let logind proceed.
        // The inhibitor is never what keeps the session locked.
        releaseInhibitor();

        if (!requested) {
            qWarning() << "shell: no lock path before sleep; continuing"
                       << "without a pre-sleep lock";
        } else if (!m_lockedForSleep) {
            qWarning() << "shell: the session lock did not confirm before"
                       << "sleep; continuing";
        } else {
            qInfo() << "shell: session locked for sleep";
        }
        emit sleepLocked(m_lockedForSleep);
        return;
    }

    if (m_resumeHandled) {
        qInfo() << "shell: duplicate PrepareForSleep(false) ignored";
        return;
    }
    m_resumeHandled = true;
    m_asleep = false;
    m_lockedForSleep = false;

    // A wake is not an unlock. Refresh the lock scene (the clock moved while
    // asleep) and wake every output's DPMS; the lock state is untouched.
    if (m_effects.refresh)
        m_effects.refresh();
    if (m_effects.wakeDpms)
        m_effects.wakeDpms();

    m_wantInhibitor = true;
    m_inhibitUnavailable = false;
    acquireInhibitor();
    qInfo() << "shell: resumed from sleep; the lock stays";
    emit resumed();
}

bool SuspendLock::waitForLock()
{
    if (!m_effects.isLocked)
        return false;
    if (m_effects.isLocked())
        return true;

    // The compositor confirms asynchronously over Wayland. Pump the GUI loop
    // so the confirmation and the lock surface's first render can happen
    // while the machine is still awake and the compositor can still render.
    QElapsedTimer timer;
    timer.start();
    while (timer.elapsed() < m_confirmTimeoutMs) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        if (m_effects.isLocked())
            return true;
    }
    return m_effects.isLocked();
}

void SuspendLock::acquireInhibitor()
{
    if (!m_wantInhibitor || m_inhibitor.isValid() || m_inhibitPending
        || m_inhibitUnavailable) {
        return;
    }
    const QDBusConnection bus = connection();
    if (!bus.isConnected())
        return;

    QDBusMessage message = QDBusMessage::createMethodCall(
        QString::fromLatin1(kLogindService), QString::fromLatin1(kLogindPath),
        QString::fromLatin1(kLogindManager), QStringLiteral("Inhibit"));
    message << QStringLiteral("sleep") << QStringLiteral("dragonfruit-shell")
            << QStringLiteral("Lock the session before sleep")
            << QStringLiteral("delay");

    auto *watcher = new QDBusPendingCallWatcher(bus.asyncCall(message), this);
    m_inhibitPending = true;
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this](QDBusPendingCallWatcher *finished) {
                m_inhibitPending = false;
                const QDBusPendingReply<QDBusUnixFileDescriptor> reply =
                    *finished;
                finished->deleteLater();
                if (reply.isError()) {
                    m_inhibitUnavailable = true;
                    qWarning() << "shell: logind sleep delay inhibitor"
                               << "unavailable:" << reply.error().message()
                               << "- the lock still runs, without a guaranteed"
                               << "pre-sleep wait";
                    return;
                }
                const QDBusUnixFileDescriptor fd = reply.value();
                if (!fd.isValid()) {
                    m_inhibitUnavailable = true;
                    qWarning() << "shell: logind returned no sleep delay"
                               << "inhibitor";
                    return;
                }
                if (!m_wantInhibitor) {
                    // The cycle moved on while the reply was in flight (the
                    // lock is already confirmed); drop it immediately.
                    return;
                }
                m_inhibitor = fd;
                qInfo() << "shell: logind sleep delay inhibitor acquired";
            });
}

void SuspendLock::releaseInhibitor()
{
    if (!m_inhibitor.isValid())
        return;
    m_inhibitor = QDBusUnixFileDescriptor();
    qInfo() << "shell: logind sleep delay inhibitor released";
}
