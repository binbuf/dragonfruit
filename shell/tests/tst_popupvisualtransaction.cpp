// SPDX-License-Identifier: GPL-3.0-or-later
// T354: popup visual transaction and lifetime owner.
//
// Pure tests for the shell-side owner: accepted content joins a generation and
// output, stale frames and transport switches are fenced, a closing snapshot
// outlives the input owner, and every retained resource is released inside the
// documented lifetime.
#include "popupvisualtransaction.h"

#include <QTest>

namespace {
const QRect kCrop(40, 28, 220, 340);
const QRect kPaint(36, 24, 228, 348);
} // namespace

class TestPopupVisualTransaction : public QObject
{
    Q_OBJECT

private slots:
    void openAndRetargetJoinTheAcceptedContent();
    void staleGenerationAndSerialAreRejected();
    void closingReleasesInputButRetainsTheSnapshot();
    void completeHoldsUntilTheDeadlineThenReleases();
    void closingFramesDoNotExtendTheLifetime();
    void lateRetargetCannotMoveTheClosingSnapshot();
    void cancelReleasesImmediately();
    void outputRemovalReleasesTheOwningSnapshot();
    void supersededReopenReleasesThePreviousSnapshot();
    void bothTransportsRetainAndReleaseTheSameWay();
    void theRetainedLifetimeIsClamped();
};

void TestPopupVisualTransaction::openAndRetargetJoinTheAcceptedContent()
{
    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 100);
    QCOMPARE(transaction.phase(), PopupVisualTransaction::Phase::Opening);

    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint,
                                 QVariantList{QVariantMap{{QStringLiteral("key"),
                                                           QStringLiteral("card")}}},
                                 QList<QRect>{QRect(4, 4, 200, 300)}, true,
                                 110));
    QVERIFY(transaction.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    QCOMPARE(transaction.phase(), PopupVisualTransaction::Phase::Open);
    QVERIFY(transaction.inputActive());
    QCOMPARE(transaction.crop(), kCrop);
    QCOMPARE(transaction.paint(), kPaint);
    QCOMPARE(transaction.materialRegions().size(), 1);
    QCOMPARE(transaction.inputRects().size(), 1);

    QImage frame(220, 340, QImage::Format_ARGB32_Premultiplied);
    frame.fill(Qt::transparent);
    transaction.storeSnapshot(frame, QSize(220, 340), kCrop, 120);
    QVERIFY(transaction.hasRetainedSnapshot());
    QCOMPARE(transaction.snapshot().generation, 1u);
    QCOMPARE(transaction.snapshot().contentSerial, 1u);
    QCOMPARE(transaction.retainedBytes(), frame.sizeInBytes());
}

void TestPopupVisualTransaction::staleGenerationAndSerialAreRejected()
{
    PopupVisualTransaction transaction;
    transaction.open(4, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(4, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(transaction.beginFrame(
        4, 10, PopupVisualTransaction::Transport::SharedMemory));

    QVERIFY(!transaction.beginFrame(
        3, 11, PopupVisualTransaction::Transport::SharedMemory));
    QVERIFY(!transaction.beginFrame(
        4, 9, PopupVisualTransaction::Transport::SharedMemory));
    QCOMPARE(transaction.contentSerial(), 10u);

    QVERIFY(transaction.beginFrame(
        4, 11, PopupVisualTransaction::Transport::SharedMemory));
    QCOMPARE(transaction.contentSerial(), 11u);
}

void TestPopupVisualTransaction::closingReleasesInputButRetainsTheSnapshot()
{
    PopupVisualTransaction transaction;
    transaction.open(7, QStringLiteral("eDP-1"), 1000);
    QVERIFY(transaction.retarget(7, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 1000));
    QVERIFY(transaction.beginFrame(
        7, 20, PopupVisualTransaction::Transport::SharedMemory));
    QImage frame(10, 10, QImage::Format_ARGB32_Premultiplied);
    frame.fill(Qt::red);
    transaction.storeSnapshot(frame, QSize(10, 10), kCrop, 1000);
    QVERIFY(transaction.inputActive());

    transaction.dismiss(8, 400, 1100);
    QCOMPARE(transaction.phase(), PopupVisualTransaction::Phase::Closing);
    QVERIFY(!transaction.inputActive());
    QVERIFY(transaction.hasRetainedSnapshot());
    QVERIFY(transaction.retained(1100));
    QVERIFY(transaction.retained(1499));
    QVERIFY(!transaction.retained(1500));
    QCOMPARE(transaction.retainedUntilMs(), 1500);
}

void TestPopupVisualTransaction::completeHoldsUntilTheDeadlineThenReleases()
{
    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(transaction.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    QImage frame(8, 8, QImage::Format_ARGB32_Premultiplied);
    frame.fill(Qt::blue);
    transaction.storeSnapshot(frame, QSize(8, 8), kCrop, 0);
    transaction.dismiss(2, 300, 100);
    QVERIFY(transaction.complete(2, 200));
    QCOMPARE(transaction.phase(), PopupVisualTransaction::Phase::Complete);
    QVERIFY(transaction.retained(399));
    QVERIFY(!transaction.releaseExpired(399));
    QVERIFY(transaction.releaseExpired(400));
    QVERIFY(!transaction.hasRetainedSnapshot());
    QCOMPARE(transaction.retainedBytes(), 0);
    QCOMPARE(transaction.lastReleaseReason(), QStringLiteral("expired"));
    QCOMPARE(transaction.lastReleasedBytes(), frame.sizeInBytes());
}

void TestPopupVisualTransaction::closingFramesDoNotExtendTheLifetime()
{
    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(transaction.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    transaction.dismiss(2, 300, 100);
    QVERIFY(transaction.beginFrame(
        2, 2, PopupVisualTransaction::Transport::SharedMemory));
    QVERIFY(transaction.beginFrame(
        2, 3, PopupVisualTransaction::Transport::SharedMemory));
    QCOMPARE(transaction.retainedUntilMs(), 400);
    QCOMPARE(transaction.contentSerial(), 3u);
}

void TestPopupVisualTransaction::lateRetargetCannotMoveTheClosingSnapshot()
{
    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(transaction.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    transaction.dismiss(2, 300, 100);
    QVERIFY(!transaction.retarget(2, QStringLiteral("eDP-1"), QRect(0, 0, 10, 10),
                                  QRect(0, 0, 10, 10), {}, {}, true, 200));
    QCOMPARE(transaction.crop(), kCrop);
    QVERIFY(!transaction.inputActive());
}

void TestPopupVisualTransaction::cancelReleasesImmediately()
{
    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(transaction.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    QImage frame(4, 4, QImage::Format_ARGB32_Premultiplied);
    frame.fill(Qt::green);
    transaction.storeSnapshot(frame, QSize(4, 4), kCrop, 0);
    QVERIFY(transaction.cancel(1, QStringLiteral("output"), 10));
    QVERIFY(!transaction.active());
    QVERIFY(!transaction.hasRetainedSnapshot());
    QCOMPARE(transaction.lastReleaseReason(), QStringLiteral("output"));
}

void TestPopupVisualTransaction::outputRemovalReleasesTheOwningSnapshot()
{
    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(transaction.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    QVERIFY(!transaction.outputRemoved(QStringLiteral("HDMI-A-1"), 10));
    QVERIFY(transaction.active());
    QVERIFY(transaction.outputRemoved(QStringLiteral("eDP-1"), 20));
    QVERIFY(!transaction.active());
    QCOMPARE(transaction.lastReleaseReason(), QStringLiteral("output"));
}

void TestPopupVisualTransaction::supersededReopenReleasesThePreviousSnapshot()
{
    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(transaction.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    QImage frame(4, 4, QImage::Format_ARGB32_Premultiplied);
    transaction.storeSnapshot(frame, QSize(4, 4), kCrop, 0);
    transaction.open(2, QStringLiteral("eDP-1"), 10);
    QCOMPARE(transaction.lastReleaseReason(), QStringLiteral("superseded"));
    QVERIFY(!transaction.hasRetainedSnapshot());
    QCOMPARE(transaction.generation(), 2u);
}

void TestPopupVisualTransaction::bothTransportsRetainAndReleaseTheSameWay()
{
    PopupVisualTransaction shm;
    shm.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(shm.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(shm.beginFrame(
        1, 1, PopupVisualTransaction::Transport::SharedMemory));
    QImage frame(6, 6, QImage::Format_ARGB32_Premultiplied);
    frame.fill(Qt::yellow);
    shm.storeSnapshot(frame, QSize(6, 6), kCrop, 0);
    shm.dismiss(2, 200, 10);
    QCOMPARE(shm.retainedBytes(), frame.sizeInBytes());

    PopupVisualTransaction dmabuf;
    dmabuf.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(dmabuf.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    QVERIFY(dmabuf.beginFrame(
        1, 1, PopupVisualTransaction::Transport::Dmabuf));
    dmabuf.storeSnapshot(QImage(), QSize(6, 6), kCrop, 0);
    dmabuf.dismiss(2, 200, 10);
    QCOMPARE(dmabuf.retainedBytes(), 0);
    QVERIFY(dmabuf.hasRetainedSnapshot());
    QVERIFY(dmabuf.retained(209));
    QVERIFY(dmabuf.releaseExpired(210));
    QVERIFY(!dmabuf.hasRetainedSnapshot());
}

void TestPopupVisualTransaction::theRetainedLifetimeIsClamped()
{
    QCOMPARE(PopupVisualTransaction::clampRetainMs(-5), qint64(0));
    QCOMPARE(PopupVisualTransaction::clampRetainMs(200),
             qint64(200));
    QCOMPARE(PopupVisualTransaction::clampRetainMs(999999),
             PopupVisualTransaction::kMaxRetainMs);

    PopupVisualTransaction transaction;
    transaction.open(1, QStringLiteral("eDP-1"), 0);
    QVERIFY(transaction.retarget(1, QStringLiteral("eDP-1"), kCrop, kPaint, {}, {}, true, 0));
    transaction.dismiss(2, 999999, 0);
    QCOMPARE(transaction.retainedUntilMs(), PopupVisualTransaction::kMaxRetainMs);
}

QTEST_GUILESS_MAIN(TestPopupVisualTransaction)

#include "tst_popupvisualtransaction.moc"
