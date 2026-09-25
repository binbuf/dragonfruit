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
    // Only the two theme keys matter here; the Dock handles the rest.
    if (key == QLatin1String("appearance.colorScheme")
        || key == QLatin1String("accessibility.reduceMotion")) {
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

    if (m_theme->property("dark").toBool() != dark)
        m_theme->setProperty("dark", dark);
    if (m_theme->property("reducedMotion").toBool() != reduced)
        m_theme->setProperty("reducedMotion", reduced);
}