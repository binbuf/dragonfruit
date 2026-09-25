// SPDX-License-Identifier: MIT
// T-08.2b theme-binding tests: a settingsd key change (delivered through the
// same `SettingsClient` the Dock uses) flips the design-system `Theme`
// singleton, and the gallery variant renders the new scheme. Headless:
// offscreen platform + software scene graph (see CMakeLists.txt), real pixels
// grabbed from a QQuickWindow.
#include "settingsclient.h"
#include "themebinding.h"

#include <QGuiApplication>
#include <QImage>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QScopedPointer>
#include <QSignalSpy>
#include <QTest>

class TestThemeBinding : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        m_theme = m_engine.singletonInstance<QObject *>(QStringLiteral("Dragonfruit"),
                                                        QStringLiteral("Theme"));
        QVERIFY2(m_theme, "the Dragonfruit.Theme singleton must be importable");
    }

    // The pure resolver: explicit light/dark win; "auto" and anything unknown
    // follow the host.
    void schemeResolver()
    {
        QCOMPARE(ThemeBinding::darkForScheme(QStringLiteral("dark"), false), true);
        QCOMPARE(ThemeBinding::darkForScheme(QStringLiteral("light"), true), false);
        QCOMPARE(ThemeBinding::darkForScheme(QStringLiteral("auto"), true), true);
        QCOMPARE(ThemeBinding::darkForScheme(QStringLiteral("auto"), false), false);
        QCOMPARE(ThemeBinding::darkForScheme(QString(), true), true);
    }

    // The acceptance path, headless: a key flipped through the client (as
    // settingsd would emit it) flips Theme.
    void settingsChangeFlipsTheme()
    {
        MockSettingsClient client;
        ThemeBinding binding(&client, &m_engine);
        binding.apply();

        QSignalSpy spy(&client, &SettingsClient::changed);
        client.set(QStringLiteral("appearance.colorScheme"), QStringLiteral("dark"));
        QCOMPARE(spy.count(), 1);
        QCOMPARE(m_theme->property("dark").toBool(), true);
        QCOMPARE(colorSurface(), QStringLiteral("#1d1723"));

        client.set(QStringLiteral("appearance.colorScheme"), QStringLiteral("light"));
        QCOMPARE(m_theme->property("dark").toBool(), false);
        QCOMPARE(colorSurface(), QStringLiteral("#ffffff"));
    }

    void reduceMotionFlipsTheme()
    {
        MockSettingsClient client;
        ThemeBinding binding(&client, &m_engine);
        binding.apply();
        QCOMPARE(m_theme->property("reducedMotion").toBool(), false);

        client.set(QStringLiteral("accessibility.reduceMotion"), true);
        QCOMPARE(m_theme->property("reducedMotion").toBool(), true);
        // The motion tokens collapse to the reduced variant.
        QObject *motion = m_theme->property("motion").value<QObject *>();
        QVERIFY(motion);
        QObject *popupOpen = motion->property("popupOpen").value<QObject *>();
        QVERIFY(popupOpen);
        QCOMPARE(popupOpen->property("duration").toInt(), 0);

        client.set(QStringLiteral("accessibility.reduceMotion"), false);
        QCOMPARE(m_theme->property("reducedMotion").toBool(), false);
        QCOMPARE(popupOpen->property("duration").toInt(), 160);
    }

    // "The gallery variant": the component gallery renders whichever scheme the
    // binding set, not the scheme it was constructed with.
    void galleryVariantFollowsTheme()
    {
        MockSettingsClient client;
        ThemeBinding binding(&client, &m_engine);

        QQmlComponent component(&m_engine);
        component.loadFromModule(QStringLiteral("Dragonfruit.Gallery"),
                                 QStringLiteral("GalleryContent"));
        QVERIFY2(!component.isError(), qPrintable(component.errorString()));
        QScopedPointer<QObject> object(component.create());
        QVERIFY(object);
        QQuickItem *gallery = qobject_cast<QQuickItem *>(object.data());
        QVERIFY(gallery);
        gallery->setProperty("pageIndex", 0);
        gallery->setWidth(480);
        gallery->setHeight(360);

        QQuickWindow window;
        window.setColor(Qt::transparent);
        gallery->setParentItem(window.contentItem());
        window.resize(480, 360);
        window.show();
        QTest::qWait(100);

        client.set(QStringLiteral("appearance.colorScheme"), QStringLiteral("dark"));
        QTest::qWait(100);
        const QImage dark = window.grabWindow();

        client.set(QStringLiteral("appearance.colorScheme"), QStringLiteral("light"));
        QTest::qWait(100);
        const QImage light = window.grabWindow();

        QVERIFY(!dark.isNull());
        QVERIFY(!light.isNull());
        QVERIFY2(dark != light, "the gallery must re-render when the scheme flips");

        // The gallery's sunken background is the resolved scheme surface: dark
        // in the dark grab, light in the light grab.
        const QColor darkSurface(0x13, 0x0f, 0x17);
        const QColor lightSurface(0xef, 0xea, 0xf3);
        QCOMPARE(dark.pixelColor(3, 3), darkSurface);
        QCOMPARE(light.pixelColor(3, 3), lightSurface);
    }

private:
    // The resolved `Theme.color.surface` as a string, read through the nested
    // QML QtObject.
    QString colorSurface() const
    {
        QObject *colors = m_theme->property("color").value<QObject *>();
        return colors ? colors->property("surface").value<QColor>().name() : QString();
    }

    QQmlEngine m_engine;
    QObject *m_theme = nullptr;
};

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    TestThemeBinding testCase;
    return QTest::qExec(&testCase, argc, argv);
}

#include "tst_themebinding.moc"