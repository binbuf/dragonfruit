// SPDX-License-Identifier: MIT
#include "dockprocess.h"

#include <QFile>
#include <QThread>

#include <cerrno>
#include <csignal>

#if defined(Q_OS_LINUX)
#include <poll.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <unistd.h>
#endif

quint64 dockProcessStartTime(qint64 pid)
{
    if (pid <= 0)
        return 0;
    QFile stat(QStringLiteral("/proc/%1/stat").arg(pid));
    if (!stat.open(QIODevice::ReadOnly))
        return 0;
    const QByteArray contents = stat.readAll();
    // The comm field may contain spaces and parentheses, so parse from the
    // last ')' rather than splitting the whole line. The remainder starts at
    // field 3 (state); `starttime` is field 22, i.e. index 19 from there.
    const int close = contents.lastIndexOf(')');
    if (close < 0)
        return 0;
    const QList<QByteArray> fields =
        contents.mid(close + 1).simplified().split(' ');
    constexpr int kStartTimeIndex = 19;
    if (fields.size() <= kStartTimeIndex)
        return 0;
    bool ok = false;
    const quint64 startTime = fields.at(kStartTimeIndex).toULongLong(&ok);
    return ok ? startTime : 0;
}

DockProcessHandle DockProcessHandle::open(qint64 pid)
{
    if (pid <= 0)
        return {};
    // Record the start time first: it is both the fallback identity and the
    // check that the PID names a live process at acquisition time.
    const quint64 startTime = dockProcessStartTime(pid);
    if (startTime == 0)
        return {};

    DockProcessHandle handle;
    handle.m_pid = pid;
    handle.m_startTime = startTime;

#if defined(Q_OS_LINUX) && defined(SYS_pidfd_open)
    const int fd = static_cast<int>(
        ::syscall(SYS_pidfd_open, static_cast<pid_t>(pid), 0u));
    if (fd >= 0) {
        // Close the acquisition race: if the PID was reused between reading
        // its start time and opening the pidfd, the descriptor names a
        // different instance. Verify both observations agree before trusting
        // the handle.
        if (dockProcessStartTime(pid) != startTime) {
            ::close(fd);
            return {};
        }
        handle.m_pidfd = std::shared_ptr<int>(new int(fd), [](int *value) {
            if (*value >= 0)
                ::close(*value);
            delete value;
        });
        return handle;
    }
    // A kernel that has pidfd but refused this process gives no safe handle.
    // Only an absent syscall falls back to the validated /proc identity.
    if (errno != ENOSYS)
        return {};
#endif
    return handle;
}

void DockProcessHandle::close()
{
    m_pidfd.reset();
    m_pid = 0;
    m_startTime = 0;
}

bool DockProcessHandle::isAlive() const
{
    if (!isValid())
        return false;
#if defined(Q_OS_LINUX)
    if (m_pidfd) {
        pollfd descriptor{*m_pidfd, POLLIN, 0};
        int result = 0;
        do {
            result = ::poll(&descriptor, 1, 0);
        } while (result < 0 && errno == EINTR);
        if (result < 0)
            return false;
        // A pidfd becomes readable when the exact process instance exits.
        return (descriptor.revents & (POLLIN | POLLHUP | POLLERR)) == 0;
    }
#endif
    // Fallback: a recycled PID has a different start time, so require both.
    if (dockProcessStartTime(m_pid) != m_startTime)
        return false;
    const int result = ::kill(static_cast<pid_t>(m_pid), 0);
    return result == 0 || errno == EPERM;
}

bool DockProcessHandle::signal(int signo) const
{
    if (!isValid() || !isAlive())
        return false;
#if defined(Q_OS_LINUX)
    if (m_pidfd) {
#if defined(SYS_pidfd_send_signal)
        const long result = ::syscall(SYS_pidfd_send_signal, *m_pidfd, signo,
                                      nullptr, 0u);
        return result == 0;
#else
        // A pidfd exists but this build has no way to signal through it.
        // Refuse rather than fall back to a numeric kill that could hit a
        // recycled PID.
        return false;
#endif
    }
#endif
    // Revalidate immediately before the numeric kill. The pidfd path has no
    // such window; this fallback only runs on kernels without pidfd.
    if (dockProcessStartTime(m_pid) != m_startTime)
        return false;
    return ::kill(static_cast<pid_t>(m_pid), signo) == 0;
}

bool DockProcessHandle::waitForExit(int timeoutMs) const
{
    if (!isValid() || !isAlive())
        return true;
#if defined(Q_OS_LINUX)
    if (m_pidfd) {
        pollfd descriptor{*m_pidfd, POLLIN, 0};
        int result = 0;
        do {
            result = ::poll(&descriptor, 1, timeoutMs);
        } while (result < 0 && errno == EINTR);
        return result > 0 && (descriptor.revents & POLLIN);
    }
#endif
    constexpr int kStepMs = 10;
    for (int waited = 0; waited <= timeoutMs; waited += kStepMs) {
        if (!isAlive())
            return true;
        QThread::msleep(kStepMs);
    }
    return !isAlive();
}

bool DockLaunchRecords::addProcess(qint64 pid)
{
    DockProcessHandle handle = DockProcessHandle::open(pid);
    if (!handle.isValid())
        return false;
    while (m_processes.size() >= kMaxProcesses)
        m_processes.removeFirst();
    m_processes.append(std::move(handle));
    return true;
}

void DockLaunchRecords::addUnit(const QString &unit)
{
    const QString trimmed = unit.trimmed();
    if (!isDockManagedUnitName(trimmed) || m_units.contains(trimmed))
        return;
    while (m_units.size() >= kMaxUnits)
        m_units.removeFirst();
    m_units.append(trimmed);
}

int DockLaunchRecords::prune()
{
    int closed = 0;
    for (auto it = m_processes.begin(); it != m_processes.end();) {
        if (it->isAlive()) {
            ++it;
        } else {
            it = m_processes.erase(it);
            ++closed;
        }
    }
    return closed;
}

int DockLaunchRecords::signalProcesses(int signo)
{
    int signalled = 0;
    for (const DockProcessHandle &handle : std::as_const(m_processes)) {
        if (handle.isAlive() && handle.signal(signo))
            ++signalled;
    }
    return signalled;
}

QStringList DockLaunchRecords::takeUnits()
{
    const QStringList units = m_units;
    m_units.clear();
    return units;
}

void DockLaunchRecords::retainUnit(const QString &unit)
{
    addUnit(unit);
}

void DockLaunchRecords::clear()
{
    m_processes.clear();
    m_units.clear();
}

bool isDockManagedUnitName(const QString &unit)
{
    static const QString prefix = QStringLiteral("dragonfruit-app-");
    if (!unit.startsWith(prefix) || unit.size() == prefix.size())
        return false;
    for (const QChar character : unit) {
        if (!(character.isLetterOrNumber() || character == u'-'
              || character == u'_' || character == u'.' || character == u':')) {
            return false;
        }
    }
    return true;
}

QStringList dockUnitStopArguments(const QString &unit)
{
    if (!isDockManagedUnitName(unit))
        return {};
    // No `--no-block`: systemctl waits for the stop job, so the helper's exit
    // status is the operation's result rather than just "the helper started".
    return {QStringLiteral("--user"), QStringLiteral("stop"), unit};
}

DockUnitStopOutcome dockUnitStopOutcome(int exitCode, bool normalExit,
                                        bool failedToStart)
{
    if (failedToStart)
        return DockUnitStopOutcome::NotStarted;
    if (normalExit && exitCode == 0)
        return DockUnitStopOutcome::Succeeded;
    return DockUnitStopOutcome::Failed;
}

bool dockForceQuitAvailable(bool hasProcessHandle, bool hasManagedUnit)
{
    return hasProcessHandle || hasManagedUnit;
}
