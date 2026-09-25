// SPDX-License-Identifier: MIT
// T-09.3: the pure mapping from the settingsd wallpaper keys onto what the
// shell forwards over `df_workspace.set_wallpaper`. No bus, Wayland, or QML.
#include <QtTest>

#include "wallpaperpolicy.h"

class TestWallpaperPolicy : public QObject
{
    Q_OBJECT

private slots:
    void defaultsMatchTheSchema();
    void everyKeyMapsThrough();
    void fitNamesMapToTheWireEnum();
    void unknownFitFallsBackToFill();
    void liveFlipChangesTheSettings();
};

void TestWallpaperPolicy::defaultsMatchTheSchema()
{
    const WallpaperSettings settings = wallpaperSettingsFromValues({});
    QCOMPARE(settings.source, QString());
    QCOMPARE(settings.fit, QStringLiteral("fill"));
    QCOMPARE(settings.showOnAllSpaces, true);
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
}

QTEST_MAIN(TestWallpaperPolicy)
#include "tst_wallpaperpolicy.moc"