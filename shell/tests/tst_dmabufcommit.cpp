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

QTEST_MAIN(TestDmabufCommit)
#include "tst_dmabufcommit.moc"
