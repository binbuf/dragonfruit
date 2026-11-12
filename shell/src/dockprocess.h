// SPDX-License-Identifier: MIT
// Stable process identity for Dock Force Quit (T236 follow-up).
//
// A numeric PID is not a process identity: after a process exits the kernel is
// free to reuse the number for an unrelated process. Force Quit must never
// signal a recycled PID, so a launch records a handle while the shell still
// owns the launch. On Linux the handle is a pidfd, which names the exact
// process instance and is immune to PID reuse. When pidfd_open is unavailable
// the handle keeps the PID plus the process start time from /proc and
// revalidates both immediately before signalling.
//
// The managed-launch path has no child PID (systemd-run owns it), so it keeps
// the validated transient-unit name instead and stops the unit through an
// asynchronous operation whose result can be observed. Both kinds of identity
// live in one `DockLaunchRecords` value per desktop id.
#pragma once

#include <QList>
#include <QString>
#include <QStringList>

#include <memory>

// The kernel start time of `pid` (the `starttime` field from
// /proc/<pid>/stat), or 0 when the process does not exist or the file cannot
// be parsed. Two observations of the same PID with different start times are
// different process instances. Pure OS seam, unit-tested (tst_dockcore).
quint64 dockProcessStartTime(qint64 pid);

// A handle to one process instance. Copyable: copies share the underlying
// pidfd, so passing a handle around cannot close it out from under another
// copy. A default-constructed handle is invalid.
class DockProcessHandle
{
public:
    DockProcessHandle() = default;

    // Open a handle for `pid` while the caller still owns the launch. Returns
    // an invalid handle when `pid` is not a live process or no stable identity
    // can be acquired.
    static DockProcessHandle open(qint64 pid);

    bool isValid() const { return m_pid > 0; }
    qint64 pid() const { return m_pid; }
    // True when this handle is backed by a pidfd (the strong identity). A
    // false value means the validated /proc start-time fallback is in use.
    bool isPidfd() const { return m_pidfd != nullptr; }
    // The recorded kernel start time (0 for an invalid handle).
    quint64 startTime() const { return m_startTime; }

    // True while the exact process instance this handle names is still
    // running. Never matches a recycled PID.
    bool isAlive() const;
    // Send `signo` to the exact recorded instance. Returns false, without
    // signalling anything, when the instance is gone or its identity changed.
    bool signal(int signo) const;
    // Wait up to `timeoutMs` for the process to exit; true when it has exited.
    bool waitForExit(int timeoutMs) const;

    // Release the handle. Idempotent; a copy keeps the shared pidfd alive.
    void close();

private:
    qint64 m_pid = 0;
    quint64 m_startTime = 0;
    // A shared fd with a close(2) deleter, so the handle stays copyable.
    std::shared_ptr<int> m_pidfd;
};

// The identities the shell holds for one desktop id. Process handles are
// acquired at launch and pruned when the process exits; managed-unit names are
// retired when a stop succeeds.
class DockLaunchRecords
{
public:
    static constexpr int kMaxProcesses = 8;
    static constexpr int kMaxUnits = 8;

    // Open and append a stable handle for a just-launched process. Returns
    // false when no handle could be acquired (Force Quit must not claim it can
    // terminate that instance).
    bool addProcess(qint64 pid);
    // Record a validated transient user unit. Empty, non-managed, or duplicate
    // names are ignored.
    void addUnit(const QString &unit);

    // Drop process handles whose instance has exited. Returns the number
    // closed.
    int prune();

    bool isEmpty() const { return m_processes.isEmpty() && m_units.isEmpty(); }
    bool hasProcess() const { return !m_processes.isEmpty(); }
    bool hasUnit() const { return !m_units.isEmpty(); }
    // True when at least one identity can still be force-terminated.
    bool canForceQuit() const { return hasProcess() || hasUnit(); }

    // Signal every live process instance; returns the number signalled.
    int signalProcesses(int signo);
    // Hand the managed units to the caller's asynchronous stop operation.
    QStringList takeUnits();
    // Re-add a unit whose stop failed so a later Force Quit can retry it.
    void retainUnit(const QString &unit);
    void clear();

    int processCount() const { return m_processes.size(); }
    const QList<DockProcessHandle> &processes() const { return m_processes; }
    const QStringList &units() const { return m_units; }

private:
    QList<DockProcessHandle> m_processes;
    QStringList m_units;
};

// True for the transient units this shell creates (`dragonfruit-app-…`). A
// unit name outside that namespace is never handed to systemctl.
bool isDockManagedUnitName(const QString &unit);

// The `systemctl` argv that stops one validated transient unit, or an empty
// list for a non-managed name. No `--no-block`: the helper waits for the job,
// so its exit status is the operation's result. Pure and unit-tested
// (tst_dockcore).
QStringList dockUnitStopArguments(const QString &unit);

// How a finished `systemctl stop` helper maps onto the operation result.
enum class DockUnitStopOutcome {
    Succeeded,
    Failed,
    NotStarted,
};

DockUnitStopOutcome dockUnitStopOutcome(int exitCode, bool normalExit,
                                        bool failedToStart);

// Whether an app menu should offer Force Quit for the identities held for one
// desktop id. A close request is not a forceful termination, so an app the
// shell did not launch (no handle, no unit) must not advertise one. Pure and
// unit-tested (tst_dockcore).
bool dockForceQuitAvailable(bool hasProcessHandle, bool hasManagedUnit);
