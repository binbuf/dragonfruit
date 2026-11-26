// SPDX-License-Identifier: GPL-3.0-only
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "clipboardwriter.h"

#include <QSocketNotifier>
#include <QTimer>

#include <cerrno>
#include <csignal>
#include <ctime>

#include <fcntl.h>
#include <pthread.h>
#include <unistd.h>

namespace {

// Write without letting a client that closed its read end kill the shell with
// SIGPIPE. The signal is blocked for the duration of this call and, when this
// call generated it, consumed before the caller's mask is restored. A mask
// the caller already had is left alone.
ssize_t writeWithoutSigpipe(int fd, const char *data, size_t size)
{
    sigset_t pipeSignal;
    sigemptyset(&pipeSignal);
    sigaddset(&pipeSignal, SIGPIPE);

    sigset_t previous;
    const bool blocked =
        ::pthread_sigmask(SIG_BLOCK, &pipeSignal, &previous) == 0;
    const bool alreadyBlocked =
        blocked && ::sigismember(&previous, SIGPIPE) == 1;

    const ssize_t written = ::write(fd, data, size);
    const int savedErrno = errno;

    if (blocked) {
        if (!alreadyBlocked && written < 0 && savedErrno == EPIPE) {
            // Consume the synchronous SIGPIPE this write generated.
            struct timespec zero = {};
            ::sigtimedwait(&pipeSignal, nullptr, &zero);
        }
        ::pthread_sigmask(SIG_SETMASK, &previous, nullptr);
    }
    errno = savedErrno;
    return written;
}

} // namespace

ClipboardSourceWriter::ClipboardSourceWriter(int fd, QByteArray payload,
                                             int timeoutMs, QObject *parent)
    : QObject(parent)
    , m_fd(fd)
    , m_payload(std::move(payload))
{
    if (m_fd < 0) {
        m_done = true;
        m_result = Result::Error;
        return;
    }

    const int flags = ::fcntl(m_fd, F_GETFL, 0);
    if (flags < 0 || ::fcntl(m_fd, F_SETFL, flags | O_NONBLOCK) < 0) {
        // Keep the fd owned and close it from the event loop, so a connection
        // made right after construction still sees `finished`.
        m_setupFailed = true;
    } else {
        m_notifier = new QSocketNotifier(m_fd, QSocketNotifier::Write, this);
        QObject::connect(m_notifier, &QSocketNotifier::activated, this,
                         [this]() { writeOnce(); });
    }

    m_deadline = new QTimer(this);
    m_deadline->setSingleShot(true);
    m_deadline->setInterval(timeoutMs > 0 ? timeoutMs : kDefaultTimeoutMs);
    QObject::connect(m_deadline, &QTimer::timeout, this,
                     [this]() { finish(Result::TimedOut); });

    // Defer the first write: construction happens inside the Wayland dispatch
    // callback, and a completed transfer must still emit `finished` to a
    // caller that connects after construction.
    QMetaObject::invokeMethod(this, [this]() { writeOnce(); },
                              Qt::QueuedConnection);
}

ClipboardSourceWriter::~ClipboardSourceWriter()
{
    if (m_notifier)
        m_notifier->setEnabled(false);
    if (m_fd >= 0)
        ::close(m_fd);
}

void ClipboardSourceWriter::cancel()
{
    finish(Result::Cancelled);
}

void ClipboardSourceWriter::writeOnce()
{
    if (m_done)
        return;

    if (!m_started) {
        m_started = true;
        if (m_setupFailed) {
            finish(Result::Error);
            return;
        }
        m_deadline->start();
    }

    const qsizetype remaining = m_payload.size() - m_offset;
    if (remaining <= 0) {
        finish(Result::Completed);
        return;
    }

    const ssize_t written = writeWithoutSigpipe(
        m_fd, m_payload.constData() + m_offset, static_cast<size_t>(remaining));
    if (written > 0) {
        m_offset += written;
        if (m_offset >= m_payload.size())
            finish(Result::Completed);
        return;
    }
    if (written < 0) {
        // A partial pipe waits for the writable callback; a hard error (EPIPE,
        // EBADF, ...) ends the transfer now.
        if (errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK)
            return;
        finish(Result::Error);
        return;
    }

    // `write()` returned zero with bytes still queued; never spin on it.
    finish(Result::Error);
}

void ClipboardSourceWriter::finish(Result result)
{
    if (m_done)
        return;
    m_done = true;
    m_result = result;

    if (m_deadline) {
        m_deadline->stop();
        m_deadline->deleteLater();
        m_deadline = nullptr;
    }
    if (m_notifier) {
        m_notifier->setEnabled(false);
        m_notifier->deleteLater();
        m_notifier = nullptr;
    }
    if (m_fd >= 0) {
        ::close(m_fd);
        m_fd = -1;
    }

    // Last statement: a direct-connected slot may delete this writer.
    emit finished(result);
}
