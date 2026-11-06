// SPDX-License-Identifier: MIT
#include "themebinding.h"

#include "settingsclient.h"

#include <QGuiApplication>
#include <QQmlEngine>
#include <QStyleHints>

ThemeBinding::ThemeBinding(SettingsClient *settings, QQmlEngine *engine, QObject *parent)
    : QObject(parent)
    , m_settings(settings)
    , m_engine(engine)
{
    if (m_settings) {
        connect(m_settings, &SettingsClient::changed, this,
                &ThemeBinding::onSettingsChanged);
        connect(m_settings, &SettingsClient::refreshed, this, &ThemeBinding::apply);
    }
    // "auto" follows the host scheme; keep following it live. `styleHints()` is
    // null without a QGuiApplication (headless unit tests), which is fine.
    if (QStyleHints *hints = QGuiApplication::styleHints()) {
        connect(hints, &QStyleHints::colorSchemeChanged, this, [this]() {
            if (m_settings) {
                const QString scheme =
                    m_settings->string(QStringLiteral("appearance.colorScheme"),
                                       QStringLiteral("auto"));
                if (scheme != QLatin1String("light") && scheme != QLatin1String("dark"))
                    apply();
            }
        });
    }
}

ThemeBinding::~ThemeBinding() = default;

bool ThemeBinding::darkForScheme(const QString &scheme, bool hostDark)
{
    if (scheme == QLatin1String("dark"))
        return true;
    if (scheme == QLatin1String("light"))
        return false;
    return hostDark;
}

bool ThemeBinding::hostDark()
{
    const QStyleHints *hints = QGuiApplication::styleHints();
    return hints && hints->colorScheme() == Qt::ColorScheme::Dark;
}

void ThemeBinding::onSettingsChanged(const QString &key, const QVariant &)
{
    // Only the theme/glass keys matter here; the Dock handles the rest.
    if (key == QLatin1String("appearance.colorScheme")
        || key == QLatin1String("appearance.accent")
        || key == QLatin1String("appearance.transparency")
        || key == QLatin1String("accessibility.reduceMotion")
        || key == QLatin1String("accessibility.reduceTransparency")
        || key == QLatin1String("accessibility.increaseContrast")) {
        apply();
    }
}

void ThemeBinding::apply()
{
    if (!m_settings || !m_engine)
        return;
    if (!m_theme)
        m_theme = m_engine->singletonInstance<QObject *>(QStringLiteral("Dragonfruit"),
                                                         QStringLiteral("Theme"));
    if (!m_theme)
        return;

    const QString scheme = m_settings->string(QStringLiteral("appearance.colorScheme"),
                                              QStringLiteral("auto"));
    const bool dark = darkForScheme(scheme, hostDark());
    const bool reduced = m_settings->boolean(QStringLiteral("accessibility.reduceMotion"), false);
    const QString accent = m_settings->string(QStringLiteral("appearance.accent"), QString());

    if (m_theme->property("dark").toBool() != dark)
        m_theme->setProperty("dark", dark);
    if (m_theme->property("reducedMotion").toBool() != reduced)
        m_theme->setProperty("reducedMotion", reduced);
    if (m_theme->property("accentOverride").toString() != accent)
        m_theme->setProperty("accentOverride", accent);

    // T196: the optical preferences are one resolver input, not a second
    // slider curve. The Glass singleton is optional in a QML-less test engine.
    if (!m_glass)
        m_glass = m_engine->singletonInstance<QObject *>(QStringLiteral("Dragonfruit"),
                                                         QStringLiteral("Glass"));
    if (!m_glass)
        return;
    const double transparency =
        m_settings->real(QStringLiteral("appearance.transparency"), 0.5255);
    const double bounded = transparency >= 0.0 && transparency <= 1.0 ? transparency : 0.5255;
    const bool reduceTransparency =
        m_settings->boolean(QStringLiteral("accessibility.reduceTransparency"), false);
    const bool increaseContrast =
        m_settings->boolean(QStringLiteral("accessibility.increaseContrast"), false);
    if (m_glass->property("slider").toDouble() != bounded)
        m_glass->setProperty("slider", bounded);
    if (m_glass->property("reduceTransparency").toBool() != reduceTransparency)
        m_glass->setProperty("reduceTransparency", reduceTransparency);
    if (m_glass->property("increaseContrast").toBool() != increaseContrast)
        m_glass->setProperty("increaseContrast", increaseContrast);
}