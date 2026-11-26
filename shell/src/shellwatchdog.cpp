// SPDX-License-Identifier: GPL-3.0-only
#include "shellwatchdog.h"

#include <QLibrary>
#include <QTimer>

namespace {

using SdNotifyFn = int (*)(int, const char *);

} // namespace

ShellWatchdog::ShellWatchdog(QObject *parent)
    : QObject(parent)
    , m_timer(new QTimer(this))
{
    m_timer->setTimerType(Qt::CoarseTimer);
    connect(m_timer, &QTimer::timeout, this, [this]() { heartbeat(); });
}

ShellWatchdog::~ShellWatchdog() = default;

bool ShellWatchdog::load()
{
    if (m_sdNotify || m_notifier)
        return true;
    if (!m_library)
        m_library = new QLibrary(this);

    // Prefer the runtime soname; the unversioned name is the development
    // package's linker alias and is the fallback on hosts that only ship it.
    for (const char *name : {"libsystemd.so.0", "libsystemd.so"}) {
        m_library->setFileName(QString::fromLatin1(name));
        if (!m_library->load())
            continue;
        m_sdNotify = reinterpret_cast<SdNotifyFn>(m_library->resolve("sd_notify"));
        if (m_sdNotify)
            return true;
    }
    return false;
}

void ShellWatchdog::setNotifier(Notifier notifier)
{
    m_notifier = std::move(notifier);
}

bool ShellWatchdog::send(const QByteArray &state)
{
    if (m_notifier)
        return m_notifier(state);
    if (!m_sdNotify)
        return false;
    // sd_notify returns > 0 on success and 0 when NOTIFY_SOCKET is unset.
    return m_sdNotify(0, state.constData()) > 0;
}

bool ShellWatchdog::notifyReady()
{
    return send(QByteArrayLiteral("READY=1"));
}

bool ShellWatchdog::heartbeat()
{
    if (!send(QByteArrayLiteral("WATCHDOG=1"))) {
        emit heartbeatFailed();
        return false;
    }
    emit heartbeatSent();
    return true;
}

void ShellWatchdog::start(int intervalMs)
{
    if (m_timer->isActive())
        return;
    if (!m_notifier && (!m_sdNotify || !qEnvironmentVariableIsSet("NOTIFY_SOCKET")))
        return;
    m_timer->start(intervalMs > 0 ? intervalMs : kHeartbeatIntervalMs);
}

void ShellWatchdog::stop()
{
    m_timer->stop();
}

bool ShellWatchdog::isRunning() const
{
    return m_timer->isActive();
}

const char *ShellWatchdog::restartActionName()
{
    return "restart-shell";
}

bool ShellWatchdog::isRestartAction(const QString &action)
{
    return action == QLatin1String(restartActionName());
}
