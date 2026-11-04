// SPDX-License-Identifier: MIT
// T-21.1d dmabuf format/commit rules. Pure: no Wayland, EGL, or compositor.
#include <QtTest>

#include "chromedmabuf.h"
#include "gpuchromecapabilities.h"

using GpuChromeCapabilities::FormatModifier;

class TestDmabufCommit : public QObject
{
    Q_OBJECT

private slots:
    void validateAcceptsAdvertisedPair();
    void validateRejectsUnknownFormat();
    void validateRejectsUnknownModifier();
    void validateRejectsBadFrame();
    void modifierSplit();
    void slotReuseDefersWhileBusy();
    void slotFailureIsStickyUntilReset();
    void releaseTrackerWaitsForEverySurface();
    void releaseTrackerIgnoresStaleSerial();
    void ringReusesReleasedSlot();
    void ringDefersWhileAllSlotsBusy();
    void ringResizeRetiresUntilRelease();
    void ringFormatChangeRetires();
    void ringFirstFormatIsAdopted();
    void ringResetDropsEverything();
};

namespace {

ChromeDmabufFrame frame(uint32_t format = GpuChromeCapabilities::Abgr8888,
                        uint64_t modifier = GpuChromeCapabilities::ModifierLinear)
{
    ChromeDmabufFrame value;
    value.fd = 7;
    value.size = QSize(320, 180);
    value.format = format;
    value.modifier = modifier;
    value.stride = 320 * 4;
    value.offset = 0;
    return value;
}

QVector<FormatModifier> advertised()
{
    return {
        {GpuChromeCapabilities::Abgr8888, GpuChromeCapabilities::ModifierLinear},
        {GpuChromeCapabilities::Argb8888, GpuChromeCapabilities::ModifierLinear},
        {GpuChromeCapabilities::Abgr8888, 0x0100000000000002ULL},
    };
}

} // namespace

void TestDmabufCommit::validateAcceptsAdvertisedPair()
{
    const ChromeDmabufCheck check = ChromeDmabuf::validate(frame(), advertised());
    QVERIFY2(check.valid, qPrintable(check.reason));
    QVERIFY(check.reason.isEmpty());
}

void TestDmabufCommit::validateRejectsUnknownFormat()
{
    const ChromeDmabufCheck check = ChromeDmabuf::validate(
        frame(GpuChromeCapabilities::Xrgb8888), advertised());
    QVERIFY(!check.valid);
    QCOMPARE(check.reason, QStringLiteral("no-format"));
}

void TestDmabufCommit::validateRejectsUnknownModifier()
{
    const ChromeDmabufCheck check = ChromeDmabuf::validate(
        frame(GpuChromeCapabilities::Abgr8888, 0x0200000000000004ULL),
        advertised());
    QVERIFY(!check.valid);
    QCOMPARE(check.reason, QStringLiteral("no-modifier"));
}

void TestDmabufCommit::validateRejectsBadFrame()
{
    ChromeDmabufFrame bad = frame();
    bad.fd = -1;
    QCOMPARE(ChromeDmabuf::validate(bad, advertised()).reason,
             QStringLiteral("bad-export"));

    bad = frame();
    bad.stride = 0;
    QCOMPARE(ChromeDmabuf::validate(bad, advertised()).reason,
             QStringLiteral("bad-export"));

    bad = frame();
    bad.size = QSize(16385, 180);
    QCOMPARE(ChromeDmabuf::validate(bad, advertised()).reason,
             QStringLiteral("oversized"));
}

void TestDmabufCommit::modifierSplit()
{
    constexpr uint64_t modifier = 0x0102030405060708ULL;
    QCOMPARE(ChromeDmabuf::modifierHigh(modifier), 0x01020304u);
    QCOMPARE(ChromeDmabuf::modifierLow(modifier), 0x05060708u);
    QCOMPARE(ChromeDmabuf::modifierHigh(GpuChromeCapabilities::ModifierLinear), 0u);
    QCOMPARE(ChromeDmabuf::modifierLow(GpuChromeCapabilities::ModifierLinear), 0u);
}

void TestDmabufCommit::slotReuseDefersWhileBusy()
{
    ChromeDmabufSlot slot;
    QCOMPARE(slot.state(), ChromeDmabufSlot::State::Empty);
    QVERIFY(slot.canRender());
    QVERIFY(slot.beginCommit());
    QCOMPARE(slot.state(), ChromeDmabufSlot::State::Busy);
    QVERIFY(slot.busy());
    QVERIFY(!slot.canRender());
    // A second render/commit before release must be deferred, not raced.
    QVERIFY(!slot.beginCommit());
    slot.release();
    QCOMPARE(slot.state(), ChromeDmabufSlot::State::Ready);
    QVERIFY(slot.canRender());
    QVERIFY(slot.beginCommit());
}

void TestDmabufCommit::slotFailureIsStickyUntilReset()
{
    ChromeDmabufSlot slot;
    slot.fail(QStringLiteral("import-failed"));
    QVERIFY(slot.failed());
    QCOMPARE(slot.failureReason(), QStringLiteral("import-failed"));
    QVERIFY(!slot.canRender());
    QVERIFY(!slot.beginCommit());
    slot.reset();
    QCOMPARE(slot.state(), ChromeDmabufSlot::State::Empty);
    QVERIFY(slot.failureReason().isEmpty());
    QVERIFY(slot.beginCommit());
}

void TestDmabufCommit::releaseTrackerWaitsForEverySurface()
{
    ChromeDmabufReleaseTracker tracker;
    tracker.acquire(41);
    tracker.acquire(41);
    QCOMPARE(tracker.users(41), 2);
    // One surface released (the popover), the producer slot is still busy.
    QVERIFY(!tracker.release(41));
    QCOMPARE(tracker.users(41), 1);
    // The last user returns it; a later release is stale and ignored.
    QVERIFY(tracker.release(41));
    QCOMPARE(tracker.users(41), 0);
    QVERIFY(!tracker.release(41));
    QCOMPARE(tracker.size(), 0);
}

void TestDmabufCommit::releaseTrackerIgnoresStaleSerial()
{
    ChromeDmabufReleaseTracker tracker;
    tracker.acquire(7);
    tracker.acquire(8);
    QVERIFY(tracker.release(7));
    QCOMPARE(tracker.users(7), 0);
    QCOMPARE(tracker.users(8), 1);
    QVERIFY(!tracker.release(99));
    QCOMPARE(tracker.size(), 1);
    tracker.clear();
    QCOMPARE(tracker.size(), 0);
}

void TestDmabufCommit::ringReusesReleasedSlot()
{
    ChromeDmabufRing ring(3);
    QVERIFY(!ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                            GpuChromeCapabilities::ModifierLinear));
    QCOMPARE(ring.capacity(), 3);
    QCOMPARE(ring.liveCount(), 0);

    const int first = ring.acquire();
    QCOMPARE(first, 0);
    QVERIFY(ring.setSerial(first, 11));
    QVERIFY(ring.isBusy(first));

    const int second = ring.acquire();
    QCOMPARE(second, 1);
    QVERIFY(ring.setSerial(second, 12));
    const int third = ring.acquire();
    QCOMPARE(third, 2);
    QVERIFY(ring.setSerial(third, 13));

    // Bounded: a fourth acquisition is refused, never allocated per frame.
    QCOMPARE(ring.acquire(), -1);
    QCOMPARE(ring.liveCount(), 3);
    QCOMPARE(ring.allocations(), quint64(3));

    QVERIFY(ring.release(11));
    QVERIFY(!ring.isBusy(first));
    QCOMPARE(ring.acquire(), first);
    QCOMPARE(ring.reuses(), quint64(1));
    QCOMPARE(ring.allocations(), quint64(3));
}

void TestDmabufCommit::ringDefersWhileAllSlotsBusy()
{
    ChromeDmabufRing ring(2);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    QCOMPARE(ring.acquire(), 0);
    QVERIFY(ring.setSerial(0, 21));
    QCOMPARE(ring.acquire(), 1);
    QVERIFY(ring.setSerial(1, 22));
    QCOMPARE(ring.acquire(), -1);

    // A stale serial never frees a slot.
    QVERIFY(!ring.release(999));
    QVERIFY(ring.isBusy(0));
    QVERIFY(ring.release(21));
    QVERIFY(ring.isFree(0));
    QCOMPARE(ring.acquire(), 0);
    QVERIFY(ring.isBusy(0));
}

void TestDmabufCommit::ringResizeRetiresUntilRelease()
{
    ChromeDmabufRing ring(3);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    for (int i = 0; i < 3; ++i) {
        QCOMPARE(ring.acquire(), i);
        QVERIFY(ring.setSerial(i, 100 + i));
    }

    QVERIFY(ring.setSize(QSize(640, 360)));
    for (int i = 0; i < 3; ++i) {
        QVERIFY(ring.isRetired(i));
        QVERIFY(ring.isBusy(i));
    }
    // The bounded ring cannot allocate a new generation while all three old
    // slots are still held by the compositor.
    QCOMPARE(ring.acquire(), -1);
    QVERIFY(ring.takeRetired().isEmpty());

    QVERIFY(ring.release(100));
    QVERIFY(!ring.isBusy(0));
    const QVector<int> retired = ring.takeRetired();
    QCOMPARE(retired, QVector<int>{0});

    // The released index is reused for the new size, not leaked.
    QCOMPARE(ring.acquire(), 0);
    QVERIFY(ring.isBusy(0));
    QVERIFY(!ring.isRetired(0));
    QCOMPARE(ring.allocations(), quint64(4));
}

void TestDmabufCommit::ringFormatChangeRetires()
{
    ChromeDmabufRing ring(3);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    QCOMPARE(ring.acquire(), 0);
    QVERIFY(ring.setSerial(0, 31));

    QVERIFY(ring.setFormat(GpuChromeCapabilities::Argb8888,
                           GpuChromeCapabilities::ModifierLinear));
    QVERIFY(ring.isRetired(0));
    QVERIFY(ring.isBusy(0));
    // A new generation is still bounded and may allocate a second slot.
    QCOMPARE(ring.acquire(), 1);
    QVERIFY(ring.isBusy(1));
    QVERIFY(!ring.isRetired(1));
}

void TestDmabufCommit::ringFirstFormatIsAdopted()
{
    ChromeDmabufRing ring(3);
    ring.setSize(QSize(320, 180));
    QCOMPARE(ring.acquire(), 0);
    QVERIFY(ring.setSerial(0, 41));
    // The first export establishes the format without retiring the slot that
    // produced it.
    QVERIFY(!ring.setFormat(GpuChromeCapabilities::Abgr8888,
                            GpuChromeCapabilities::ModifierLinear));
    QVERIFY(!ring.isRetired(0));
    QCOMPARE(ring.format(), GpuChromeCapabilities::Abgr8888);
    QCOMPARE(ring.modifier(), GpuChromeCapabilities::ModifierLinear);
}

void TestDmabufCommit::ringResetDropsEverything()
{
    ChromeDmabufRing ring(3);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    QCOMPARE(ring.acquire(), 0);
    ring.reset();
    QCOMPARE(ring.liveCount(), 0);
    QCOMPARE(ring.busyCount(), 0);
    QVERIFY(ring.size().isEmpty());
    QCOMPARE(ring.allocations(), quint64(0));
    QCOMPARE(ring.reuses(), quint64(0));
}

QTEST_MAIN(TestDmabufCommit)
#include "tst_dmabufcommit.moc"
