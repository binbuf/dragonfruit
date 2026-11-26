// SPDX-License-Identifier: GPL-3.0-only
// T338: the menu-bar luma decode runs off the GUI thread from a scaled
// thumbnail, and the wallpaper provider's property read is asynchronous,
// bounded, and coalesced. The provider half uses a slow mock on a private
// session bus (dbus-run-session); the luma half runs on any host.
#include "wallpaperluma.h"
#include "wallpaperpolicy.h"
#include "wallpaperproviderclient.h"

#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusVirtualObject>
#include <QDateTime>
#include <QElapsedTimer>
#include <QFile>
#include <QImage>
#include <QPainter>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>
#include <QThread>
#include <QTimer>
#include <QVariantMap>

#include <optional>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.Wallpaper1.test");
const QString kPath = QStringLiteral("/org/dragonfruit/Wallpaper1/test");
const QString kInterface = QStringLiteral("org.dragonfruit.Wallpaper1.test");
const QString kProviderConnection = QStringLiteral("dragonfruit_test_wallpaperd");

QImage makeWallpaper(const QColor &top, const QColor &bottom)
{
    QImage image(3840, 2160, QImage::Format_RGB32);
    image.fill(bottom);
    QPainter painter(&image);
    painter.fillRect(QRect(0, 0, image.width(), qMax(1, image.height() / 10)),
                     top);
    return image;
}

// A stand-in for wallpaperd's standard `org.freedesktop.DBus.Properties`
// surface. A slow mode delays the reply without blocking the test's GUI
// thread, which is exactly the hung-provider shape T338 guards against.
class FakeWallpaperProvider : public QDBusVirtualObject
{
    Q_OBJECT

public:
    explicit FakeWallpaperProvider(QObject *parent = nullptr)
        : QDBusVirtualObject(parent)
    {
    }

    int getAllCalls = 0;
    bool slow = false;
    bool answer = true;
    int delayMs = 300;
    QString defaultSource = QStringLiteral("/cache/default.jpg");
    QString builtinDefaultSource =
        QStringLiteral("/usr/share/dragonfruit/wallpapers/Default.jpg");

    QString introspect(const QString &path) const override
    {
        Q_UNUSED(path);
        return QString();
    }

    bool handleMessage(const QDBusMessage &message,
                       const QDBusConnection &connection) override
    {
        if (message.interface() != QLatin1String("org.freedesktop.DBus.Properties")
            || message.member() != QLatin1String("GetAll")) {
            return false;
        }
        ++getAllCalls;
        if (!slow) {
            QDBusMessage response = message.createReply();
            response << properties();
            connection.send(response);
            return true;
        }
        const QDBusMessage reply = message;
        QTimer::singleShot(delayMs, this, [this, reply, connection]() mutable {
            if (!answer)
                return;
            QDBusMessage response = reply.createReply();
            response << properties();
            connection.send(response);
        });
        return true;
    }

    void publishDefaultSource(const QString &source,
                              const QDBusConnection &connection)
    {
        defaultSource = source;
        QDBusMessage signal = QDBusMessage::createSignal(
            kPath, QStringLiteral("org.freedesktop.DBus.Properties"),
            QStringLiteral("PropertiesChanged"));
        QVariantMap values;
        values.insert(QStringLiteral("DefaultSource"), source);
        signal << kInterface << values << QStringList();
        connection.send(signal);
    }

private:
    QVariantMap properties() const
    {
        QVariantMap map;
        map.insert(QStringLiteral("DefaultSource"), defaultSource);
        map.insert(QStringLiteral("BuiltinDefaultSource"), builtinDefaultSource);
        return map;
    }
};

} // namespace

class TestWallpaperLuma : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanupTestCase();
    void cleanup();

    void thumbnailIsScaledAndKeepsTheTopBand();
    void lumaDecodeRunsOffTheGuiThread();
    void cacheKeyTracksMtimeAndReDecodes();
    void missingSourceUsesTheFallback();

    void providerReadIsAsyncAndCoalesced();
    void providerSlowReplyKeepsTheLastValue();
    void providerTimeoutKeepsTheLastValue();
    void providerPropertiesChangedAppliesWithoutAReread();

private:
    QString writeImage(const QString &name, const QColor &top,
                       const QColor &bottom);

    QTemporaryDir m_dir;
    std::optional<QDBusConnection> m_providerBus;
};

void TestWallpaperLuma::initTestCase()
{
    QVERIFY(m_dir.isValid());
    // A second connection makes the mock provider remote from the client, so
    // a delayed reply really crosses the bus (Qt does not allow a delayed
    // reply to a local in-process method).
    if (QDBusConnection::sessionBus().isConnected()) {
        m_providerBus.emplace(QDBusConnection::connectToBus(
            QDBusConnection::SessionBus, kProviderConnection));
    }
}

void TestWallpaperLuma::cleanupTestCase()
{
    if (m_providerBus && m_providerBus->isConnected())
        QDBusConnection::disconnectFromBus(kProviderConnection);
}

void TestWallpaperLuma::cleanup()
{
    if (!m_providerBus || !m_providerBus->isConnected())
        return;
    m_providerBus->unregisterObject(kPath);
    m_providerBus->unregisterService(kService);
}

QString TestWallpaperLuma::writeImage(const QString &name, const QColor &top,
                                      const QColor &bottom)
{
    const QString path = m_dir.filePath(name);
    if (!makeWallpaper(top, bottom).save(path, "PNG"))
        return QString();
    return path;
}

void TestWallpaperLuma::thumbnailIsScaledAndKeepsTheTopBand()
{
    const QString path = writeImage(QStringLiteral("large.png"), Qt::white,
                                    Qt::black);
    QVERIFY(!path.isEmpty());
    const QImage thumbnail = wallpaperLumaThumbnail(path, QSize(320, 320));
    QVERIFY(!thumbnail.isNull());
    // A plugin that ignores setScaledSize is scaled down afterwards, so a 4K
    // source never reaches the luma pass at full size.
    QVERIFY(thumbnail.width() <= 320);
    QVERIFY(thumbnail.height() <= 320);
    QVERIFY(qAbs(wallpaperTopBandLuminance(thumbnail, Qt::black) - 1.0) < 0.05);
}

void TestWallpaperLuma::lumaDecodeRunsOffTheGuiThread()
{
    const QString path = writeImage(QStringLiteral("async.png"), Qt::white,
                                    Qt::black);
    QVERIFY(!path.isEmpty());
    // A deliberately slow decode makes the GUI-thread assertion meaningful on
    // a fast host: the request must return at once and the event loop must
    // keep ticking while the worker runs.
    WallpaperLuma luma(nullptr, [](const QString &source) {
        QThread::msleep(250);
        return wallpaperLumaThumbnail(source);
    });
    QSignalSpy ready(&luma, &WallpaperLuma::lumaReady);
    int ticks = 0;
    int ticksAtReady = -1;
    QTimer timer;
    timer.setInterval(10);
    connect(&timer, &QTimer::timeout, [&ticks]() { ++ticks; });
    connect(&luma, &WallpaperLuma::lumaReady, [&ticks, &ticksAtReady]() {
        ticksAtReady = ticks;
    });
    timer.start();

    const QString key = WallpaperLuma::keyFor(path, Qt::black);
    QElapsedTimer elapsed;
    elapsed.start();
    const std::optional<double> immediate = luma.request(key, path, Qt::black);
    const qint64 requestMs = elapsed.elapsed();
    QVERIFY(!immediate.has_value());
    QVERIFY2(requestMs < 50,
             qPrintable(QStringLiteral("luma request blocked %1 ms")
                            .arg(requestMs)));

    QTRY_COMPARE_WITH_TIMEOUT(ready.count(), 1, 5000);
    QVERIFY2(ticksAtReady >= 5,
             qPrintable(QStringLiteral("only %1 timer ticks before the luma "
                                       "arrived")
                            .arg(ticksAtReady)));
    QCOMPARE(ready.at(0).at(0).toString(), key);
    QVERIFY(ready.at(0).at(1).toDouble() > 0.9);
    double cached = -1.0;
    QVERIFY(luma.cached(key, &cached));
    QVERIFY(cached > 0.9);
}

void TestWallpaperLuma::cacheKeyTracksMtimeAndReDecodes()
{
    const QString path = writeImage(QStringLiteral("cache.png"), Qt::white,
                                    Qt::black);
    QVERIFY(!path.isEmpty());
    const QString firstKey = WallpaperLuma::keyFor(path, Qt::black);

    WallpaperLuma luma;
    QSignalSpy ready(&luma, &WallpaperLuma::lumaReady);
    QVERIFY(!luma.request(firstKey, path, Qt::black).has_value());
    QTRY_COMPARE_WITH_TIMEOUT(ready.count(), 1, 5000);
    QVERIFY(ready.at(0).at(1).toDouble() > 0.9);

    // Rewrite the image and move its mtime: the key changes, so the cache
    // misses and the new top band is decoded.
    QVERIFY(makeWallpaper(Qt::black, Qt::white).save(path, "PNG"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::ReadWrite));
    QVERIFY(file.setFileTime(QDateTime::currentDateTime().addSecs(10),
                             QFileDevice::FileModificationTime));
    file.close();
    const QString secondKey = WallpaperLuma::keyFor(path, Qt::black);
    QVERIFY(firstKey != secondKey);
    // A different fallback is a different result, so it is also a key.
    QVERIFY(WallpaperLuma::keyFor(path, Qt::white) != secondKey);

    double cached = -1.0;
    QVERIFY(luma.cached(firstKey, &cached));
    QVERIFY(!luma.cached(secondKey, &cached));
    QVERIFY(!luma.request(secondKey, path, Qt::black).has_value());
    QTRY_COMPARE_WITH_TIMEOUT(ready.count(), 2, 5000);
    QCOMPARE(ready.at(1).at(0).toString(), secondKey);
    QVERIFY(ready.at(1).at(1).toDouble() < 0.1);
}

void TestWallpaperLuma::missingSourceUsesTheFallback()
{
    WallpaperLuma luma;
    QSignalSpy ready(&luma, &WallpaperLuma::lumaReady);

    // An empty source is the Space's solid colour and needs no worker.
    const QString solidKey = WallpaperLuma::keyFor(QString(), Qt::white);
    const std::optional<double> solid =
        luma.request(solidKey, QString(), Qt::white);
    QVERIFY(solid.has_value());
    QCOMPARE(qRound(*solid * 1000), 1000);

    // A missing file decodes asynchronously to the fallback colour.
    const QString missing = m_dir.filePath(QStringLiteral("absent.png"));
    const QString key = WallpaperLuma::keyFor(missing, Qt::black);
    QVERIFY(!luma.request(key, missing, Qt::black).has_value());
    QTRY_COMPARE_WITH_TIMEOUT(ready.count(), 1, 5000);
    QCOMPARE(ready.at(0).at(0).toString(), key);
    QCOMPARE(qRound(ready.at(0).at(1).toDouble() * 1000), 0);
}

void TestWallpaperLuma::providerReadIsAsyncAndCoalesced()
{
    if (!m_providerBus || !m_providerBus->isConnected())
        QSKIP("no session bus available (run under dbus-run-session)");

    FakeWallpaperProvider provider;
    provider.slow = true;
    provider.delayMs = 300;
    QVERIFY(m_providerBus->registerService(kService));
    QVERIFY(m_providerBus->registerVirtualObject(kPath, &provider));

    WallpaperProviderClient client(kService, kPath, kInterface, 2000);
    QSignalSpy changed(&client, &WallpaperProviderClient::changed);
    QElapsedTimer elapsed;
    elapsed.start();
    client.start();
    const qint64 startMs = elapsed.elapsed();
    QVERIFY2(startMs < 100,
             qPrintable(QStringLiteral("provider start blocked %1 ms")
                            .arg(startMs)));
    // The slow reply has not arrived, and the event loop is alive.
    int ticks = 0;
    QTimer timer;
    timer.setInterval(10);
    connect(&timer, &QTimer::timeout, [&ticks]() { ++ticks; });
    timer.start();
    QTest::qWait(80);
    QVERIFY(ticks >= 4);
    QVERIFY(changed.isEmpty());
    QVERIFY(client.defaultSource().isEmpty());

    // A burst of refreshes while the read is in flight becomes one follow-up.
    client.refresh();
    client.refresh();
    QTRY_COMPARE_WITH_TIMEOUT(provider.getAllCalls, 2, 3000);
    QTRY_VERIFY_WITH_TIMEOUT(changed.count() >= 1, 3000);
    QCOMPARE(client.defaultSource(), provider.defaultSource);
    QCOMPARE(client.builtinDefaultSource(), provider.builtinDefaultSource);
    QTest::qWait(100);
    QCOMPARE(provider.getAllCalls, 2);
}

void TestWallpaperLuma::providerSlowReplyKeepsTheLastValue()
{
    if (!m_providerBus || !m_providerBus->isConnected())
        QSKIP("no session bus available (run under dbus-run-session)");

    FakeWallpaperProvider provider;
    QVERIFY(m_providerBus->registerService(kService));
    QVERIFY(m_providerBus->registerVirtualObject(kPath, &provider));

    WallpaperProviderClient client(kService, kPath, kInterface, 2000);
    QSignalSpy changed(&client, &WallpaperProviderClient::changed);
    client.start();
    QTRY_VERIFY_WITH_TIMEOUT(changed.count() >= 1, 3000);
    const QString firstDefault = client.defaultSource();
    QCOMPARE(firstDefault, provider.defaultSource);

    // A hung provider keeps the last resolved values; a later reply is not
    // required for the desktop to stay usable.
    provider.slow = true;
    provider.answer = false;
    client.refresh();
    QTest::qWait(150);
    QCOMPARE(client.defaultSource(), firstDefault);
    QCOMPARE(client.builtinDefaultSource(), provider.builtinDefaultSource);
}

void TestWallpaperLuma::providerTimeoutKeepsTheLastValue()
{
    if (!m_providerBus || !m_providerBus->isConnected())
        QSKIP("no session bus available (run under dbus-run-session)");

    FakeWallpaperProvider provider;
    QVERIFY(m_providerBus->registerService(kService));
    QVERIFY(m_providerBus->registerVirtualObject(kPath, &provider));

    WallpaperProviderClient client(kService, kPath, kInterface, 120);
    QSignalSpy changed(&client, &WallpaperProviderClient::changed);
    client.start();
    QTRY_VERIFY_WITH_TIMEOUT(changed.count() >= 1, 3000);
    const QString firstDefault = client.defaultSource();

    provider.slow = true;
    provider.answer = false;
    client.refresh();
    // The 120 ms bound expires; the cached values survive.
    QTest::qWait(300);
    QCOMPARE(client.defaultSource(), firstDefault);
    QCOMPARE(client.builtinDefaultSource(), provider.builtinDefaultSource);
    QCOMPARE(changed.count(), 1);
}

void TestWallpaperLuma::providerPropertiesChangedAppliesWithoutAReread()
{
    if (!m_providerBus || !m_providerBus->isConnected())
        QSKIP("no session bus available (run under dbus-run-session)");

    FakeWallpaperProvider provider;
    QVERIFY(m_providerBus->registerService(kService));
    QVERIFY(m_providerBus->registerVirtualObject(kPath, &provider));

    WallpaperProviderClient client(kService, kPath, kInterface, 2000);
    QSignalSpy changed(&client, &WallpaperProviderClient::changed);
    client.start();
    QTRY_VERIFY_WITH_TIMEOUT(changed.count() >= 1, 3000);
    const int reads = provider.getAllCalls;

    provider.defaultSource = QStringLiteral("/cache/new.jpg");
    provider.publishDefaultSource(provider.defaultSource, *m_providerBus);
    QTRY_COMPARE_WITH_TIMEOUT(client.defaultSource(), provider.defaultSource,
                              2000);
    // The signal carried the value; no second property read was needed.
    QCOMPARE(provider.getAllCalls, reads);
}

QTEST_GUILESS_MAIN(TestWallpaperLuma)
#include "tst_wallpaperluma.moc"
