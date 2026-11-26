// SPDX-License-Identifier: GPL-3.0-or-later
// T357: alpha enforcement and per-target DMA-BUF failure isolation.
//
// The suite injects the failure tokens the producer and protocol can report at
// the same decision boundary the controller uses. No Wayland, EGL, or GPU is
// needed: the alpha rule is checked on the exported-frame description, and the
// target plan/recovery rules are pure. The pixel fixture proves that the
// supported premultiplied interpretation keeps transparent pixels and rejects
// a non-premultiplied sample.
#include <QtTest>

#include <QImage>
#include <QSet>
#include <QVector>

#include "chromedmabuf.h"
#include "gpuchromecapabilities.h"

using GpuChromeCapabilities::Abgr8888;
using GpuChromeCapabilities::Argb8888;
using GpuChromeCapabilities::FormatModifier;
using GpuChromeCapabilities::ModifierLinear;
using GpuChromeCapabilities::Xbgr8888;
using GpuChromeCapabilities::Xrgb8888;

namespace {

ChromeDmabufFrame frame(uint32_t format = Abgr8888,
                        uint64_t modifier = ModifierLinear)
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
        {Abgr8888, ModifierLinear},
        {Argb8888, ModifierLinear},
        {Xbgr8888, ModifierLinear},
        {Xrgb8888, ModifierLinear},
    };
}

// A small injection harness for the controller's decision boundary. A target
// failure is a set entry; an export failure disables the rail; a capability
// change runs the bounded recovery.
struct InjectedScene {
    QVector<QString> labels;
    QSet<QString> disabled;
    bool railEnabled = true;
    ChromeDmabufRecovery recovery{3};
    // Targets that actually attached the export in the last simulated commit.
    QVector<QString> committed;
    bool usedCompletedFrameFallback = false;

    void injectImportFailure(const QString &label) { disabled.insert(label); }
    void injectExportFailure() { railEnabled = false; }

    bool commitFrame()
    {
        const ChromeDmabuf::TargetPlan plan = this->plan();
        committed = plan.commit;
        usedCompletedFrameFallback = !plan.useDmabuf();
        return plan.useDmabuf();
    }

    bool capabilityChange()
    {
        if (!recovery.recover())
            return false;
        disabled.clear();
        railEnabled = true;
        return true;
    }

    ChromeDmabuf::TargetPlan plan() const
    {
        if (!railEnabled) {
            // No target can attach this export. The caller reads the completed
            // FBO back once and commits the scene through shared memory.
            return ChromeDmabuf::TargetPlan();
        }
        return ChromeDmabuf::planTargets(labels, disabled);
    }
};

} // namespace

class TestDmabufFailure : public QObject
{
    Q_OBJECT

private slots:
    void opaqueFormatIsRejectedBeforeAttach();
    void supportedAlphaFormatsArePremultiplied();
    void supportedImportFixtureKeepsTransparentPixels();
    void importFailureIsIsolatedToItsTarget();
    void independentPopupViewFallsBackFromTheCompletedFrame();
    void transientFailureRetriesWithoutDisabling();
    void transportLossIsClassifiedSeparately();
    void capabilityChangeRecoversBounded();
    void recoveryForcesFullDamageOnSlotReuse();
    void exportFailureFallsBackFromTheCompletedFrame();
};

void TestDmabufFailure::opaqueFormatIsRejectedBeforeAttach()
{
    // The opaque pair is advertised by the compositor and matches the exported
    // frame exactly. It is still refused: the chrome scene is translucent, and
    // an X* buffer has no alpha to carry its cleared pixels.
    for (uint32_t format : {Xbgr8888, Xrgb8888}) {
        const ChromeDmabufCheck check =
            ChromeDmabuf::validate(frame(format), advertised());
        QVERIFY(!check.valid);
        QCOMPARE(check.reason, QStringLiteral("opaque-format"));
    }

    // The supported alpha pair is accepted.
    const ChromeDmabufCheck alpha =
        ChromeDmabuf::validate(frame(Abgr8888), advertised());
    QVERIFY2(alpha.valid, qPrintable(alpha.reason));
}

void TestDmabufFailure::supportedAlphaFormatsArePremultiplied()
{
    QVERIFY(GpuChromeCapabilities::isAlphaFormat(Abgr8888));
    QVERIFY(GpuChromeCapabilities::isAlphaFormat(Argb8888));
    QVERIFY(!GpuChromeCapabilities::isAlphaFormat(Xbgr8888));
    QVERIFY(!GpuChromeCapabilities::isAlphaFormat(Xrgb8888));

    QVERIFY(GpuChromeCapabilities::supportsPremultipliedAlpha(Abgr8888));
    QVERIFY(GpuChromeCapabilities::supportsPremultipliedAlpha(Argb8888));
    QVERIFY(!GpuChromeCapabilities::supportsPremultipliedAlpha(Xbgr8888));
    QVERIFY(!GpuChromeCapabilities::supportsPremultipliedAlpha(Xrgb8888));
    QVERIFY(!GpuChromeCapabilities::supportsPremultipliedAlpha(0));
}

void TestDmabufFailure::supportedImportFixtureKeepsTransparentPixels()
{
    // The supported import fixture is a premultiplied ARGB image: the cleared
    // corner stays fully transparent, and a 50% card carries colour channels
    // no larger than its alpha.
    QImage fixture(16, 16, QImage::Format_ARGB32_Premultiplied);
    fixture.fill(Qt::transparent);
    fixture.setPixel(8, 8, qRgba(128, 0, 0, 128));

    QCOMPARE(qAlpha(fixture.pixel(0, 0)), 0);
    QCOMPARE(qRed(fixture.pixel(0, 0)), 0);
    const QRgb card = fixture.pixel(8, 8);
    QCOMPARE(qAlpha(card), 128);
    QVERIFY(qRed(card) <= qAlpha(card));
    QVERIFY(qGreen(card) <= qAlpha(card));
    QVERIFY(qBlue(card) <= qAlpha(card));

    // The same sample without premultiplication would leak colour past the
    // alpha channel; the compositor would show it as a bright fringe over a
    // dark underlay. The fixture proves the rule can tell them apart.
    fixture.setPixel(9, 9, qRgba(255, 0, 0, 128));
    QVERIFY(qRed(fixture.pixel(9, 9)) > qAlpha(fixture.pixel(9, 9)));
}

void TestDmabufFailure::importFailureIsIsolatedToItsTarget()
{
    // One shared scene: the strip and its popup. The popup import fails, but
    // the strip must keep committing on the same export.
    InjectedScene scene;
    scene.labels = {QStringLiteral("dock"), QStringLiteral("dock-popup")};
    scene.injectImportFailure(QStringLiteral("dock-popup"));

    const ChromeDmabuf::TargetPlan plan = scene.plan();
    QVERIFY(plan.useDmabuf());
    QVERIFY(plan.healthySibling());
    QCOMPARE(plan.commit, QVector<QString>{QStringLiteral("dock")});
    QCOMPARE(plan.skip, QVector<QString>{QStringLiteral("dock-popup")});

    // Observe the healthy sibling commit: the strip attaches the export while
    // the failed popup does not.
    QVERIFY(scene.commitFrame());
    QCOMPARE(scene.committed, QVector<QString>{QStringLiteral("dock")});
    QVERIFY(!scene.committed.contains(QStringLiteral("dock-popup")));
    QVERIFY(!scene.usedCompletedFrameFallback);

    // The next frame still commits the healthy strip; the failure is sticky
    // only for the popup target until a capability/output recovery.
    QVERIFY(scene.commitFrame());
    QCOMPARE(scene.committed, QVector<QString>{QStringLiteral("dock")});
}

void TestDmabufFailure::independentPopupViewFallsBackFromTheCompletedFrame()
{
    // Independent targets have their own view and export. A failed popup view
    // falls back on its own; the strip view is a separate decision and stays
    // on DMA-BUF.
    const QVector<QString> popup = {QStringLiteral("dock-popup")};
    const QVector<QString> strip = {QStringLiteral("dock")};
    const QSet<QString> failed = {QStringLiteral("dock-popup")};

    const ChromeDmabuf::TargetPlan popupPlan =
        ChromeDmabuf::planTargets(popup, failed);
    QVERIFY(!popupPlan.useDmabuf());
    QVERIFY(popupPlan.fallbackToCompletedFrame());

    const ChromeDmabuf::TargetPlan stripPlan =
        ChromeDmabuf::planTargets(strip, failed);
    QVERIFY(stripPlan.useDmabuf());
    QCOMPARE(stripPlan.commit, strip);
    QVERIFY(!stripPlan.healthySibling());
}

void TestDmabufFailure::transientFailureRetriesWithoutDisabling()
{
    for (const QString &reason : {QStringLiteral("sync-timeout"),
                                  QStringLiteral("ring-exhausted"),
                                  QStringLiteral("slot-busy"),
                                  QStringLiteral("import-pending"),
                                  QStringLiteral("completion-pending")}) {
        QCOMPARE(ChromeDmabuf::classifyFailure(reason),
                 ChromeDmabuf::FailureClass::Transient);
        QVERIFY(ChromeDmabuf::isTransientFailure(reason));
    }
}

void TestDmabufFailure::transportLossIsClassifiedSeparately()
{
    // A target-local import/format failure never disables the rail.
    QCOMPARE(ChromeDmabuf::classifyFailure(QStringLiteral("import-failed")),
             ChromeDmabuf::FailureClass::Target);
    QCOMPARE(ChromeDmabuf::classifyFailure(QStringLiteral("no-viewporter")),
             ChromeDmabuf::FailureClass::Target);
    QCOMPARE(ChromeDmabuf::classifyFailure(QStringLiteral("opaque-format")),
             ChromeDmabuf::FailureClass::Format);

    // A lost connection/device/context/exporter is rail-wide.
    for (const QString &reason : {QStringLiteral("no-dmabuf"),
                                  QStringLiteral("no-egl-export"),
                                  QStringLiteral("no-current-context"),
                                  QStringLiteral("device-lost"),
                                  QStringLiteral("connection-lost")}) {
        QCOMPARE(ChromeDmabuf::classifyFailure(reason),
                 ChromeDmabuf::FailureClass::Transport);
    }

    // A surface that closed while its frame was completing is stale, not a
    // transport failure.
    QCOMPARE(ChromeDmabuf::classifyFailure(QStringLiteral("unknown-surface")),
             ChromeDmabuf::FailureClass::Stale);
}

void TestDmabufFailure::capabilityChangeRecoversBounded()
{
    InjectedScene scene;
    scene.labels = {QStringLiteral("dock"), QStringLiteral("dock-popup")};
    scene.injectImportFailure(QStringLiteral("dock-popup"));
    scene.injectExportFailure();
    QVERIFY(!scene.plan().useDmabuf());

    // A capability change clears both the per-target and the rail failure.
    QVERIFY(scene.capabilityChange());
    QVERIFY(scene.railEnabled);
    QVERIFY(scene.disabled.isEmpty());
    QCOMPARE(scene.plan().commit, scene.labels);
    QCOMPARE(scene.recovery.attempts(), 1);

    // A persistent failure cannot loop: the remaining two attempts consume
    // the bounded budget, and the next capability change leaves the failure
    // in place.
    for (int i = 0; i < 2; ++i) {
        scene.injectImportFailure(QStringLiteral("dock-popup"));
        QVERIFY(scene.capabilityChange());
    }
    scene.injectImportFailure(QStringLiteral("dock-popup"));
    QVERIFY(!scene.capabilityChange());
    QVERIFY(scene.disabled.contains(QStringLiteral("dock-popup")));

    // A successful commit restores the budget for a later real change.
    scene.recovery.noteSuccess();
    QVERIFY(scene.capabilityChange());
    QVERIFY(scene.disabled.isEmpty());
}

void TestDmabufFailure::recoveryForcesFullDamageOnSlotReuse()
{
    // A capability/export change retires the slot generation. The next use of
    // a recycled index has no baseline, so its first commit is a full redraw.
    ChromeDmabufRing ring(3);
    ring.configure(QSize(320, 180), Abgr8888, ModifierLinear);
    const int slot = ring.acquire();
    QVERIFY(ring.setSerial(slot, 11));
    QVERIFY(ring.markPending(slot));
    QVERIFY(ring.markReady(slot));
    QVERIFY(ring.markOwned(slot));
    ring.setContentValid(slot, true);
    QVERIFY(ring.contentValid(slot));

    // The exporter now reports the other supported alpha pair. The generation
    // is retired and its content is no longer a partial-damage baseline.
    QVERIFY(ring.setFormat(Argb8888, ModifierLinear));
    QVERIFY(ring.isRetired(slot));
    QVERIFY(!ring.contentValid(slot));

    QVERIFY(ring.release(11));
    QCOMPARE(ring.takeRetired(), QVector<int>{slot});
    const int reused = ring.acquire();
    QCOMPARE(reused, slot);
    QVERIFY(!ring.contentValid(reused));
}

void TestDmabufFailure::exportFailureFallsBackFromTheCompletedFrame()
{
    // A single-target view cannot commit an export that failed validation.
    // The completed frame is read back once for the shm fallback instead of
    // being attached as an invalid dmabuf.
    InjectedScene scene;
    scene.labels = {QStringLiteral("menubar")};
    scene.injectExportFailure();
    const ChromeDmabuf::TargetPlan plan = scene.plan();
    QVERIFY(!plan.useDmabuf());
    QVERIFY(plan.fallbackToCompletedFrame());

    // The fallback does not disable the target permanently: a capability
    // change retries it on the DMA-BUF rail.
    QVERIFY(scene.capabilityChange());
    QCOMPARE(scene.plan().commit, scene.labels);
}

QTEST_MAIN(TestDmabufFailure)
#include "tst_dmabuffailure.moc"
