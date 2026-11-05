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
    ChromeStats &stats = ChromeStats::instance();
    QVERIFY(stats.enabled());

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

    const QStringList lines = stats.summaryLines(QStringLiteral("SIGUSR1-1"));
    QCOMPARE(lines.size(), 2);

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

    const QString surface = lines.at(1);
    QVERIFY(surface.startsWith(QStringLiteral("chrome surface (SIGUSR1-1):")));
    QCOMPARE(field(surface, QStringLiteral("surface")), QStringLiteral("dock"));
    QCOMPARE(field(surface, QStringLiteral("path")), QStringLiteral("dmabuf"));
    QCOMPARE(field(surface, QStringLiteral("copies")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("dmabuf_buffers")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("dmabuf_reuses")), QStringLiteral("1"));
    QCOMPARE(field(surface, QStringLiteral("dmabuf_fallbacks")), QStringLiteral("1"));
}

QTEST_MAIN(TestChromeStats)
#include "tst_chromestats.moc"
