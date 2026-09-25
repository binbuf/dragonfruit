// SPDX-License-Identifier: MIT
// T-08.2b: the one writer of the design-system Theme's settings-driven
// appearance.
//
// The shell owns the single `org.dragonfruit.Settings1` connection
// (`SettingsClient`, ADR 0032); this binding consumes that same client and
// maps the appearance/accessibility keys onto the `Dragonfruit.Theme` QML
// singleton:
//
//   appearance.colorScheme     "light" | "dark" | "auto"  -> Theme.dark
//   appearance.accent          "#rrggbb" | ""             -> Theme.accentOverride
//   accessibility.reduceMotion boolean                    -> Theme.reducedMotion
//
// It reacts to `SettingsClient::changed` and `refreshed` only (no timer, no
// second bus connection). "auto" follows the host's `QStyleHints` color scheme
// and stays live through `colorSchemeChanged`, so the Theme singleton's own
// `Application.styleHints` default is replaced by one owner rather than a
// binding that a first settings change would silently break.
#pragma once

#include <QObject>
#include <QPointer>
#include <QString>
#include <QVariant>
#include <QVariantMap>

class QQmlEngine;
class SettingsClient;

class ThemeBinding : public QObject
{
    Q_OBJECT

public:
    explicit ThemeBinding(SettingsClient *settings, QQmlEngine *engine,
                          QObject *parent = nullptr);
    ~ThemeBinding() override;

    // Re-evaluate every theme key against the client's current values.
    void apply();

    // Pure: resolve `appearance.colorScheme` against the host scheme. Only
    // "light"/"dark" are absolute; anything else ("auto", unknown) follows the
    // host.
    static bool darkForScheme(const QString &scheme, bool hostDark);
    // The host's current light/dark preference (Unknown counts as light).
    static bool hostDark();

private:
    void onSettingsChanged(const QString &key, const QVariant &value);

    SettingsClient *m_settings = nullptr;
    QQmlEngine *m_engine = nullptr;
    QPointer<QObject> m_theme;
};