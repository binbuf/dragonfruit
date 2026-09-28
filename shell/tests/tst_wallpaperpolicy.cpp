// SPDX-License-Identifier: MIT
// T-09.3 / T-18.1b: the pure mapping from the settingsd wallpaper keys and
// the provider's resolved defaults onto what the shell forwards over
// `df_workspace.set_wallpaper`. No bus, Wayland, or QML.
#include <QtTest>

#include "settingsclient.h"
#include "wallpaperpolicy.h"

class TestWallpaperPolicy : public QObject
{
    Q_OBJECT

private slots:
    void defaultsMatchTheSchema();
    void everyKeyMapsThrough();
    void providerKeysMapThrough();
    void userChoiceWins();
    void bundledDefaultBeatsTheProvider();
    void providerIsTheFetchedFallback();
    void missingEverythingIsTheSolidColor();
    void shippedDefaultResolvesTheOverride();
    void fitNamesMapToTheWireEnum();
    void unknownFitFallsBackToFill();
    void liveFlipChangesTheSettings();
};

void TestWallpaperPolicy::defaultsMatchTheSchema()
{
    const WallpaperSettings settings = wallpaperSettingsFromValues({});
    QCOMPARE(settings.source, QString());
    QCOMPARE(settings.builtinDefault, QString());
    QCOMPARE(settings.providerSource, QString());
    QCOMPARE(settings.fit, QStringLiteral("fill"));
    QCOMPARE(settings.showOnAllSpaces, true);
    // With nothing supplied, the effective source is the solid color.
    QCOMPARE(settings.effectiveSource(), QString());
}

void TestWallpaperPolicy::everyKeyMapsThrough()
{
    QVariantMap values;
    values.insert(QStringLiteral("wallpaper.source"), QStringLiteral("/tmp/w.png"));
    values.insert(QStringLiteral("wallpaper.fit"), QStringLiteral("fit"));
    values.insert(QStringLiteral("wallpaper.showOnAllSpaces"), false);

    const WallpaperSettings settings = wallpaperSettingsFromValues(values);
    QCOMPARE(settings.source, QStringLiteral("/tmp/w.png"));
    QCOMPARE(settings.fit, QStringLiteral("fit"));
    QCOMPARE(settings.showOnAllSpaces, false);
}

// T-18.1b: the additive provider keys feed the precedence fields.
void TestWallpaperPolicy::providerKeysMapThrough()
{
    QVariantMap values;
    values.insert(QStringLiteral("wallpaper.builtinDefault"),
                  QStringLiteral("/usr/share/dragonfruit/wallpapers/Default.jpg"));
    values.insert(QStringLiteral("wallpaper.providerSource"),
                  QStringLiteral("/home/u/.cache/dragonfruit/wallpapers/nature/1.jpg"));

    const WallpaperSettings settings = wallpaperSettingsFromValues(values);
    QCOMPARE(settings.builtinDefault,
             QStringLiteral("/usr/share/dragonfruit/wallpapers/Default.jpg"));
    QCOMPARE(settings.providerSource,
             QStringLiteral("/home/u/.cache/dragonfruit/wallpapers/nature/1.jpg"));
    // No user choice: the shipped default is effective, not the fetch.
    QCOMPARE(settings.effectiveSource(),
             QStringLiteral("/usr/share/dragonfruit/wallpapers/Default.jpg"));
}

// A user choice is sticky and beats both defaults.
void TestWallpaperPolicy::userChoiceWins()
{
    QCOMPARE(effectiveWallpaperSource(QStringLiteral("/u/photo.jpg"),
                                      QStringLiteral("/ship/Default.jpg"),
                                      QStringLiteral("/cache/nature.jpg")),
             QStringLiteral("/u/photo.jpg"));
}

// The shipped default beats the fetched Featured fallback (ADR 0094).
void TestWallpaperPolicy::bundledDefaultBeatsTheProvider()
{
    QCOMPARE(effectiveWallpaperSource(QString(), QStringLiteral("/ship/Default.jpg"),
                                      QStringLiteral("/cache/nature.jpg")),
             QStringLiteral("/ship/Default.jpg"));
}

// With no shipped default available, the fetched entry is the fallback.
void TestWallpaperPolicy::providerIsTheFetchedFallback()
{
    QCOMPARE(effectiveWallpaperSource(QString(), QString(),
                                      QStringLiteral("/cache/nature.jpg")),
             QStringLiteral("/cache/nature.jpg"));
}

// Nothing supplied is the Space's solid color (empty), not an error.
void TestWallpaperPolicy::missingEverythingIsTheSolidColor()
{
    QCOMPARE(effectiveWallpaperSource(QString(), QString(), QString()), QString());
}

// The shipped-asset resolver honours `DF_DEFAULT_WALLPAPER` first, matching
// wallpaperd's resolution order.
void TestWallpaperPolicy::shippedDefaultResolvesTheOverride()
{
    QTemporaryFile file;
    QVERIFY(file.open());
    const QString previous = qEnvironmentVariable("DF_DEFAULT_WALLPAPER");
    qputenv("DF_DEFAULT_WALLPAPER", file.fileName().toUtf8());
    QCOMPARE(shippedDefaultWallpaperPath(), file.fileName());
    if (previous.isNull())
        qunsetenv("DF_DEFAULT_WALLPAPER");
    else
        qputenv("DF_DEFAULT_WALLPAPER", previous.toUtf8());
}

void TestWallpaperPolicy::fitNamesMapToTheWireEnum()
{
    // df_workspace.wallpaper_fit: fill=0, fit=1, stretch=2, center=3.
    QCOMPARE(wallpaperFitFromName(QStringLiteral("fill")), 0u);
    QCOMPARE(wallpaperFitFromName(QStringLiteral("fit")), 1u);
    QCOMPARE(wallpaperFitFromName(QStringLiteral("stretch")), 2u);
    QCOMPARE(wallpaperFitFromName(QStringLiteral("center")), 3u);
}

void TestWallpaperPolicy::unknownFitFallsBackToFill()
{
    QCOMPARE(wallpaperFitFromName(QStringLiteral("cover")), 0u);
    QCOMPARE(wallpaperFitFromName(QString()), 0u);
}

void TestWallpaperPolicy::liveFlipChangesTheSettings()
{
    // The same path a settingsd `Changed` takes: re-derive from the client's
    // values and the forward only fires when the struct differs.
    QVariantMap values;
    values.insert(QStringLiteral("wallpaper.source"), QStringLiteral("/tmp/a.png"));
    const WallpaperSettings before = wallpaperSettingsFromValues(values);

    values.insert(QStringLiteral("wallpaper.source"), QStringLiteral("/tmp/b.png"));
    const WallpaperSettings after = wallpaperSettingsFromValues(values);
    QVERIFY(before != after);

    // Re-deriving an unchanged map is a no-op (`operator==`), so the shell
    // does not re-send the protocol request.
    QVERIFY(after == wallpaperSettingsFromValues(values));

    // A provider default arriving (settingsd- or bus-driven) also changes the
    // struct, so the effective source is re-evaluated and forwarded.
    values.insert(QStringLiteral("wallpaper.providerSource"),
                  QStringLiteral("/tmp/featured.jpg"));
    const WallpaperSettings withProvider = wallpaperSettingsFromValues(values);
    QVERIFY(after != withProvider);
    QCOMPARE(withProvider.effectiveSource(), QStringLiteral("/tmp/b.png"));
}

QTEST_MAIN(TestWallpaperPolicy)
#include "tst_wallpaperpolicy.moc"