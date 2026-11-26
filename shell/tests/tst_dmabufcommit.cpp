// SPDX-License-Identifier: GPL-3.0-or-later
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
    void ringTracksCompletionPhases();
    void ringRefusesPhaseSkips();
    void ringAbandonFromPendingFreesSlot();
    void ringRetireWhilePendingWaitsForRelease();
    void frameDamageMapsThroughScale();
    void frameDamageClipsToTheFrame();
    void targetDamageIntersectsTheCrop();
    void ringTracksContentValidity();
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

void TestDmabufCommit::ringTracksCompletionPhases()
{
    ChromeDmabufRing ring(3);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    const int slot = ring.acquire();
    QCOMPARE(slot, 0);
    // Acquired work is submitted, not yet pending or ready.
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Submitted);
    QCOMPARE(ring.pendingCount(), 0);
    QVERIFY(ring.setSerial(slot, 71));
    QVERIFY(ring.markPending(slot));
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Pending);
    QCOMPARE(ring.pendingCount(), 1);
    QVERIFY(ring.isBusy(slot));
    // A pending slot is not reusable: the fence still guards its pixels.
    QVERIFY(!ring.isFree(slot));
    QVERIFY(ring.markReady(slot));
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Ready);
    QCOMPARE(ring.readyCount(), 1);
    QCOMPARE(ring.pendingCount(), 0);
    QVERIFY(ring.markOwned(slot));
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Owned);
    QCOMPARE(ring.ownedCount(), 1);
    QCOMPARE(ring.readyCount(), 0);
    // The compositor release is the only transition back to reuse.
    QVERIFY(ring.release(71));
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Free);
    QCOMPARE(ring.ownedCount(), 0);
    QVERIFY(ring.isFree(slot));
}

void TestDmabufCommit::ringRefusesPhaseSkips()
{
    ChromeDmabufRing ring(2);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    const int slot = ring.acquire();
    // A frame cannot become ready before its fence was retained, and cannot
    // be owned before it is ready.
    QVERIFY(!ring.markReady(slot));
    QVERIFY(!ring.markOwned(slot));
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Submitted);
    QVERIFY(ring.markPending(slot));
    QVERIFY(!ring.markOwned(slot));
    QVERIFY(!ring.markPending(slot));
    // A free or unknown slot has no phase transition.
    QVERIFY(!ring.markPending(slot + 5));
    QCOMPARE(ring.phase(slot + 5), ChromeDmabufRing::Phase::Free);
}

void TestDmabufCommit::ringAbandonFromPendingFreesSlot()
{
    ChromeDmabufRing ring(2);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    const int slot = ring.acquire();
    QVERIFY(ring.setSerial(slot, 81));
    QVERIFY(ring.markPending(slot));
    // A view that closes while its frame is in flight returns the slot
    // without waiting for a compositor release.
    QVERIFY(ring.abandon(slot));
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Free);
    QCOMPARE(ring.pendingCount(), 0);
    QVERIFY(ring.isFree(slot));
    // The stale serial cannot release the reused slot.
    QVERIFY(!ring.release(81));
}

void TestDmabufCommit::ringRetireWhilePendingWaitsForRelease()
{
    ChromeDmabufRing ring(3);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    const int slot = ring.acquire();
    QVERIFY(ring.setSerial(slot, 91));
    QVERIFY(ring.markPending(slot));

    // A resize retires the generation but cannot destroy a slot the GPU is
    // still writing. The completion is what makes it collectible.
    QVERIFY(ring.setSize(QSize(640, 360)));
    QVERIFY(ring.isRetired(slot));
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Pending);
    QVERIFY(ring.takeRetired().isEmpty());

    QVERIFY(ring.markReady(slot));
    QVERIFY(ring.markOwned(slot));
    QVERIFY(ring.release(91));
    const QVector<int> retired = ring.takeRetired();
    QCOMPARE(retired, QVector<int>{slot});
    QCOMPARE(ring.phase(slot), ChromeDmabufRing::Phase::Free);
}

void TestDmabufCommit::frameDamageMapsThroughScale()
{
    // A logical region becomes a framebuffer region with the same outward
    // rounding the source crop uses, so a fractional scale never leaves a
    // changed pixel outside the damage box.
    const ChromeFrameDamage atOne =
        ChromeDmabuf::frameDamage(QRect(10, 20, 30, 40), 1.0, QSize(320, 180));
    QVERIFY(atOne.known());
    QCOMPARE(atOne.rect, QRect(10, 20, 30, 40));

    const ChromeFrameDamage atHalf =
        ChromeDmabuf::frameDamage(QRect(10, 20, 30, 40), 1.5, QSize(480, 270));
    QVERIFY(atHalf.known());
    QCOMPARE(atHalf.rect, QRect(15, 30, 45, 60));

    // A missing report is not "no change": it is an unknown (full) damage.
    const ChromeFrameDamage unknown =
        ChromeDmabuf::frameDamage(QRect(), 1.0, QSize(320, 180));
    QVERIFY(!unknown.known());
}

void TestDmabufCommit::frameDamageClipsToTheFrame()
{
    const ChromeFrameDamage clipped =
        ChromeDmabuf::frameDamage(QRect(300, 170, 60, 40), 1.0, QSize(320, 180));
    QVERIFY(clipped.known());
    QCOMPARE(clipped.rect, QRect(300, 170, 20, 10));

    // A change entirely outside the render target leaves the target's pixels
    // untouched, so the report is a known empty region rather than unknown.
    const ChromeFrameDamage outside =
        ChromeDmabuf::frameDamage(QRect(400, 400, 10, 10), 1.0, QSize(320, 180));
    QVERIFY(outside.known());
    QVERIFY(outside.rect.isEmpty());
}

void TestDmabufCommit::targetDamageIntersectsTheCrop()
{
    // A shared scene exports once and commits to two surfaces; each target
    // only gets the part of the report that falls inside its crop.
    const QRect strip(0, 0, 320, 40);
    const QRect popup(120, 40, 200, 140);
    const ChromeFrameDamage report =
        ChromeFrameDamage::region(QRect(140, 60, 20, 20));
    QCOMPARE(ChromeDmabuf::targetDamage(report, strip), QRect());
    QCOMPARE(ChromeDmabuf::targetDamage(report, popup), QRect(140, 60, 20, 20));

    // An unknown report always damages the target's whole crop.
    QCOMPARE(ChromeDmabuf::targetDamage(ChromeFrameDamage::full(), strip), strip);
    QCOMPARE(ChromeDmabuf::targetDamage(ChromeFrameDamage::full(), popup), popup);

    // A known empty report is "this target already shows the frame".
    QVERIFY(ChromeDmabuf::targetDamage(ChromeFrameDamage::region(QRect()), popup)
                .isEmpty());
}

void TestDmabufCommit::ringTracksContentValidity()
{
    ChromeDmabufRing ring(3);
    ring.configure(QSize(320, 180), GpuChromeCapabilities::Abgr8888,
                   GpuChromeCapabilities::ModifierLinear);
    const int slot = ring.acquire();
    // A freshly allocated slot has no complete frame to use as a baseline.
    QVERIFY(!ring.contentValid(slot));
    ring.setContentValid(slot, true);
    QVERIFY(ring.contentValid(slot));

    // Reuse keeps the frame the slot already holds.
    QVERIFY(ring.setSerial(slot, 101));
    QVERIFY(ring.release(101));
    QCOMPARE(ring.acquire(), slot);
    QVERIFY(ring.contentValid(slot));

    // A resize retires the generation: the old pixels are no longer a
    // baseline for the new mapping, and a retired slot never becomes one.
    QVERIFY(ring.setSize(QSize(640, 360)));
    QVERIFY(ring.isRetired(slot));
    QVERIFY(!ring.contentValid(slot));
    ring.setContentValid(slot, true);
    QVERIFY(!ring.contentValid(slot));

    // The retired slot's contents are dropped with the slot.
    QVERIFY(ring.setSerial(slot, 102));
    QVERIFY(ring.release(102));
    QCOMPARE(ring.takeRetired(), QVector<int>{slot});
    QVERIFY(!ring.contentValid(slot));
}

QTEST_MAIN(TestDmabufCommit)
#include "tst_dmabufcommit.moc"
