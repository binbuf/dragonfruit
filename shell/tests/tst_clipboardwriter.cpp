// SPDX-License-Identifier: GPL-3.0-or-later
// Async clipboard source writer tests (T310): the shell serves a stored
// clipboard entry from the event loop, never from the Wayland dispatch
// callback. A pipe that is never drained must time out instead of hanging the
// process, and every finish path must close the fd exactly once.
//
// Pure Qt + POSIX pipes: no Wayland, D-Bus, QML, or compositor.
#include <QtTest>

#include <QElapsedTimer>
#include <QSignalSpy>

#include <cerrno>
#include <csignal>
#include <fcntl.h>
#include <unistd.h>

#include "clipboardhistory.h"
#include "clipboardwriter.h"

namespace {

bool fdIsClosed(int fd)
{
    return ::fcntl(fd, F_GETFD) == -1 && errno == EBADF;
}

void setNonBlocking(int fd)
{
    const int flags = ::fcntl(fd, F_GETFL, 0);
    if (flags >= 0)
        ::fcntl(fd, F_SETFL, flags | O_NONBLOCK);
}

// Fill the pipe until it reports EAGAIN, so the writer's very first write
// cannot make progress regardless of the host's pipe capacity.
qint64 fillPipe(int fd)
{
    setNonBlocking(fd);
    const QByteArray chunk(65536, 'f');
    qint64 total = 0;
    while (total < (4 << 20)) {
        const ssize_t n = ::write(fd, chunk.constData(), chunk.size());
        if (n <= 0)
            break;
        total += n;
    }
    return total;
}

QByteArray drainPipe(int fd)
{
    QByteArray received;
    char buffer[8192];
    ssize_t n = 0;
    while ((n = ::read(fd, buffer, sizeof(buffer))) > 0)
        received.append(buffer, n);
    return received;
}

} // namespace

class TestClipboardWriter : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        // The writer blocks SIGPIPE around its own write; ignore it here too so
        // a regression in that guard fails the test instead of killing it.
        ::signal(SIGPIPE, SIG_IGN);
    }

    void writesPayloadAndCloses()
    {
        int fds[2] = { -1, -1 };
        QVERIFY(::pipe(fds) == 0);
        const QByteArray payload("the quick brown fox");

        ClipboardSourceWriter writer(fds[1], payload);
        QSignalSpy spy(&writer, &ClipboardSourceWriter::finished);
        QVERIFY(spy.wait(2000));

        QCOMPARE(writer.result(), ClipboardSourceWriter::Result::Completed);
        QCOMPARE(writer.bytesWritten(), payload.size());
        QVERIFY(fdIsClosed(fds[1]));
        QCOMPARE(drainPipe(fds[0]), payload);
        ::close(fds[0]);
    }

    void resumesAfterPartialWrites()
    {
        int fds[2] = { -1, -1 };
        QVERIFY(::pipe(fds) == 0);
        setNonBlocking(fds[0]);
        // Larger than the default pipe buffer, so the first write is partial
        // and the notifier has to resume the rest.
        const QByteArray payload(256 * 1024, 'p');

        ClipboardSourceWriter writer(fds[1], payload);
        QSignalSpy spy(&writer, &ClipboardSourceWriter::finished);

        QByteArray received;
        QElapsedTimer timer;
        timer.start();
        while (spy.isEmpty() && timer.elapsed() < 5000) {
            char buffer[4096];
            const ssize_t n = ::read(fds[0], buffer, sizeof(buffer));
            if (n > 0)
                received.append(buffer, n);
            QCoreApplication::processEvents(QEventLoop::AllEvents, 5);
        }

        QVERIFY2(!spy.isEmpty(), "the writer stalled on a partial pipe");
        QCOMPARE(writer.result(), ClipboardSourceWriter::Result::Completed);
        QCOMPARE(writer.bytesWritten(), payload.size());
        received.append(drainPipe(fds[0]));
        QCOMPARE(received, payload);
        QVERIFY(fdIsClosed(fds[1]));
        ::close(fds[0]);
    }

    void fullPipeTimesOutWithoutBlocking()
    {
        int fds[2] = { -1, -1 };
        QVERIFY(::pipe(fds) == 0);
        QVERIFY(fillPipe(fds[1]) > 0);

        const QByteArray payload(ClipboardHistory::kMaxEntryBytes, 'x');
        QElapsedTimer construction;
        construction.start();
        ClipboardSourceWriter writer(fds[1], payload, 200);
        QVERIFY2(construction.elapsed() < 500,
                 "construction waited on the full pipe");

        QSignalSpy spy(&writer, &ClipboardSourceWriter::finished);
        QVERIFY2(spy.wait(5000),
                 "a pipe with no reader hung the writer instead of timing out");
        QCOMPARE(writer.result(), ClipboardSourceWriter::Result::TimedOut);
        QCOMPARE(writer.bytesWritten(), qsizetype(0));
        QVERIFY(fdIsClosed(fds[1]));
        ::close(fds[0]);
    }

    void cancellationClosesFd()
    {
        int fds[2] = { -1, -1 };
        QVERIFY(::pipe(fds) == 0);
        const QByteArray payload(ClipboardHistory::kMaxEntryBytes, 'c');

        ClipboardSourceWriter writer(fds[1], payload);
        QSignalSpy spy(&writer, &ClipboardSourceWriter::finished);
        writer.cancel();

        QCOMPARE(spy.count(), 1);
        QCOMPARE(writer.result(), ClipboardSourceWriter::Result::Cancelled);
        QVERIFY(fdIsClosed(fds[1]));
        // Cancelling twice is harmless.
        writer.cancel();
        QCOMPARE(spy.count(), 1);
        ::close(fds[0]);
    }

    void epipeFinishesWithError()
    {
        int fds[2] = { -1, -1 };
        QVERIFY(::pipe(fds) == 0);
        // The client went away before the first write.
        ::close(fds[0]);

        ClipboardSourceWriter writer(fds[1], QByteArray("payload"));
        QSignalSpy spy(&writer, &ClipboardSourceWriter::finished);
        QVERIFY(spy.wait(2000));

        QCOMPARE(writer.result(), ClipboardSourceWriter::Result::Error);
        QVERIFY(fdIsClosed(fds[1]));
    }

    void emptyPayloadCompletes()
    {
        int fds[2] = { -1, -1 };
        QVERIFY(::pipe(fds) == 0);

        ClipboardSourceWriter writer(fds[1], QByteArray());
        QSignalSpy spy(&writer, &ClipboardSourceWriter::finished);
        QVERIFY(spy.wait(2000));

        QCOMPARE(writer.result(), ClipboardSourceWriter::Result::Completed);
        QVERIFY(fdIsClosed(fds[1]));
        ::close(fds[0]);
    }
};

QTEST_GUILESS_MAIN(TestClipboardWriter)
#include "tst_clipboardwriter.moc"
