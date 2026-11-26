// SPDX-License-Identifier: GPL-3.0-or-later
// Asynchronous clipboard source writer (T310).
//
// The shell serves a stored clipboard entry by handing the compositor a pipe
// whose write end is granted to a client. A client is free to never read it.
// Writing that payload from the Wayland dispatch callback would block the
// whole shell once the pipe buffer fills, so this class owns the fd and
// drains the payload from the event loop instead:
//
// - the fd is set non-blocking;
// - one `write()` runs per writable callback, resuming at the last offset;
// - `EAGAIN` waits for the notifier, `EPIPE`/other errors finish at once;
// - a deadline closes the fd when the client never drains it.
//
// The payload is held by value (implicitly shared with the caller's map), so
// no per-write copy is made. It names no Wayland or D-Bus and is unit-testable
// with a plain pipe.
#pragma once

#include <QByteArray>
#include <QObject>

class QSocketNotifier;
class QTimer;

class ClipboardSourceWriter : public QObject
{
    Q_OBJECT

public:
    // Why the writer stopped serving the fd.
    enum class Result {
        Completed, // every byte was written
        TimedOut,  // the deadline expired with bytes still queued
        Cancelled, // the selection changed or the writer was replaced
        Error,     // EPIPE or another write/setup error
    };
    Q_ENUM(Result)

    // The safety-net deadline, not a user-visible delay.
    static constexpr int kDefaultTimeoutMs = 2000;

    // Takes ownership of `fd`. The first write is deferred to the event loop,
    // so construction never blocks and a `finished` connection made right
    // after construction cannot miss the signal.
    ClipboardSourceWriter(int fd, QByteArray payload,
                          int timeoutMs = kDefaultTimeoutMs,
                          QObject *parent = nullptr);
    ~ClipboardSourceWriter() override;

    bool isActive() const { return !m_done; }
    Result result() const { return m_result; }
    qsizetype bytesWritten() const { return m_offset; }
    qsizetype payloadSize() const { return m_payload.size(); }

public slots:
    // Stop serving and close the fd. Idempotent.
    void cancel();

signals:
    void finished(ClipboardSourceWriter::Result result);

private:
    void writeOnce();
    void finish(Result result);

    int m_fd = -1;
    QByteArray m_payload;
    qsizetype m_offset = 0;
    QSocketNotifier *m_notifier = nullptr;
    QTimer *m_deadline = nullptr;
    Result m_result = Result::Cancelled;
    bool m_done = false;
    bool m_started = false;
    bool m_setupFailed = false;
};
