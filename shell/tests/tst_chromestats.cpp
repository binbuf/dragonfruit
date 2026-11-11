// SPDX-License-Identifier: MIT
// T-21.4b chrome transport counters. Pure: no Wayland, QML, or compositor.
#include <QtTest>

#include <QStringList>

#include "chromestats.h"

namespace {

QString field(const QString &line, const QString &key)
{
    const QString prefix = key + QLatin1Char('=');
    for (const QString &token : line.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
        if (token.startsWith(prefix))
            return token.mid(prefix.size());
    }
    return QString();
}

} // namespace

class TestChromeStats : public QObject
{
    Q_OBJECT

private slots:
    void summaryCarriesTransportCounters();
};

void TestChromeStats::summaryCarriesTransportCounters()
{
    qputenv("DRAGONFRUIT_CHROME_STATS", "1");
    qputenv("DRAGONFRUIT_CHROME_TRACE", "1");
    ChromeStats &stats = ChromeStats::instance();
    QVERIFY(stats.enabled());
    QVERIFY(stats.traceEnabled());

    stats.setTransportPath(QStringLiteral("render-control+dmabuf"));
    stats.setGpuPath(QStringLiteral("dmabuf"));
    stats.setSurfacePath(QStringLiteral("dock"), QStringLiteral("dmabuf"));
    stats.recordSceneFrame();
    stats.recordRender(QStringLiteral("dock"));
    stats.recordRenderComplete();
    stats.recordCommit(QStringLiteral("dock"), QSize(100, 50), QSize(200, 100), 2,
                       2.0, true, 42, 4096, 100);
    stats.recordDmabufCommit(QStringLiteral("dock"), 8000);
    stats.recordDmabufBufferCreated(QStringLiteral("dock"));
    stats.recordDmabufReuse(QStringLiteral("dock"));
    stats.setDmabufSyncMode(QStringLiteral("gl-fence"));
    stats.recordDmabufSyncWait(100, false);
    stats.recordDmabufSyncWait(200, true);
    stats.recordDmabufFallback(QStringLiteral("dock"), QStringLiteral("ring-exhausted"));
    stats.recordFrameCallback(QStringLiteral("dock"), 1200);
    stats.recordSceneResize(QStringLiteral("dock"), QSize(100, 50), QSize(200, 100));
    stats.recordFboAlloc(QStringLiteral("dock"), QSize(200, 100), 160000, 300);
    stats.recordFboFree(QStringLiteral("dock"), 40000);
    stats.recordDmabufImport(QStringLiteral("dock"), 1500, 1700, true);
    stats.recordDmabufHold(QStringLiteral("dock"), 4000, 9000, true);
    stats.recordRingExhausted(QStringLiteral("dock"));
    stats.recordCommitFlush(25);
    stats.recordPopoverOpen(QStringLiteral("dock"), 12000);

    const QStringList lines = stats.summaryLines(QStringLiteral("SIGUSR1-1"));
    QCOMPARE(lines.size(), 3);

    const QString global = lines.at(0);
    QVERIFY(global.startsWith(QStringLiteral("chrome stats (SIGUSR1-1):")));
    QCOMPARE(field(global, QStringLiteral("path")),
             QStringLiteral("render-control+dmabuf"));
    QCOMPARE(field(global, QStringLiteral("copies")), QStringLiteral("1"));
    QCOMPARE(field(global, QStringLiteral("copy_bytes")), QStringLiteral("4096"));
    QCOMPARE(field(global, QStringLiteral("commit_us_samples")), QStringLiteral("1"));
    QCOMPARE(field(global, QStringLiteral("commit_us_p50")), QStringLiteral("42"));
    QCOMPARE(field(global, QStringLiteral("frame_callback_us_p50")),
             QStringLiteral("1200"));
    QCOMPARE(field(global, QStringLiteral("dmabuf_buffers")), QStringLiteral("1"));
    QCOMPARE(field(global, QStringLiteral("dmabuf_fallbacks")), QStringLiteral("1"));
    QCOMPARE(field(global, QStringLiteral("dmabuf_sync_timeouts")), QStringLiteral("1"));
    QCOMPARE(field(global, QStringLiteral("shm_bytes")), QStringLiteral("0"));

    const QString stage = lines.at(1);
    QVERIFY(stage.startsWith(QStringLiteral("chrome stage stats (SIGUSR1-1):")));
    QCOMPARE(field(stage, QStringLiteral("scene_resizes")), QStringLiteral("1"));
    QCOMPARE(field(stage, QStringLiteral("fbo_allocs")), QStringLiteral("1"));
    QCOMPARE(field(stage, QStringLiteral("fbo_bytes_live")), QStringLiteral("120000"));
    QCOMPARE(field(stage, QStringLiteral("fbo_bytes_peak")), QStringLiteral("160000"));
    QCOMPARE(field(stage, QStringLiteral("dmabuf_imports")), QStringLiteral("1"));
    QCOMPARE(field(stage, QStringLiteral("ring_exhausted")), QStringLiteral("1"));
    QCOMPARE(field(stage, QStringLiteral("popover_opens")), QStringLiteral("1"));
    QCOMPARE(field(stage, QStringLiteral("fbo_create_us_p50")), QStringLiteral("300"));
    QCOMPARE(field(stage, QStringLiteral("dmabuf_import_us_p50")), QStringLiteral("1700"));
    QCOMPARE(field(stage, QStringLiteral("dmabuf_roundtrip_us_p50")),
             QStringLiteral("1500"));
    QCOMPARE(field(stage, QStringLiteral("dmabuf_hold_us_p50")), QStringLiteral("4000"));
    QCOMPARE(field(stage, QStringLiteral("dmabuf_shared_hold_us_p50")),
             QStringLiteral("9000"));
    QCOMPARE(field(stage, QStringLiteral("commit_flush_us_p50")), QStringLiteral("25"));
    QCOMPARE(field(stage, QStringLiteral("popover_open_us_p50")),
             QStringLiteral("12000"));

    const QString surface = lines.at(2);
    QVERIFY(surface.startsWith(QStringLiteral("chrome surface (SIGUSR1-1):")));
    QCOMPARE(field(surface, QStringLiteral("surface")), QStringLiteral("dock"));
    QCOMPARE(field(surface, QStringLiteral("path")), QStringLiteral("dmabuf"));
    QCOMPARE(field(surface, QStringLiteral("copies")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("dmabuf_buffers")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("dmabuf_reuses")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("dmabuf_fallbacks")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("scene_resizes")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("fbo_allocs")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("fbo_bytes_live")), QStringLiteral("120000"));
    QCOMPARE(field(surface, QStringLiteral("dmabuf_imports")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("ring_exhausted")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("popover_opens")), QStringLiteral("1"));

    // The correlation trace is a separate opt-in; ids are monotonic and an
    // ended interaction cannot be ended twice.
    const quint64 first = stats.beginTrace(QStringLiteral("test"),
                                           QStringLiteral("detail=1"));
    QVERIFY(first > 0);
    stats.traceEvent(first, QStringLiteral("stage"));
    QVERIFY(stats.endTrace(first, QStringLiteral("done")) >= 0);
    QCOMPARE(stats.endTrace(first), qint64(-1));
    const quint64 second = stats.beginTrace(QStringLiteral("test"));
    QVERIFY(second > first);
    QVERIFY(stats.endTrace(second) >= 0);
}

QTEST_MAIN(TestChromeStats)
#include "tst_chromestats.moc"
