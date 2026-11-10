// SPDX-License-Identifier: MIT
// The Files app's read-only view of the desktop settings (T206).
//
// Files is a separate process from the shell, so the shell's ThemeBinding
// cannot reach its QML `Theme`/`Glass` singletons. This small bridge links the
// same settings client the shell and Settings app use and exposes the values
// the window chrome needs. It is deliberately not a second settings model:
// `values` is the one reactive map and `set` is not exposed because Files does
// not own appearance policy.
#pragma once

#include <QObject>
#include <QVariant>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

class SettingsClient;

class FilesSettings : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Settings)
    QML_SINGLETON

    // Every desktop settings key. The Files shell binds its app-local Theme
    // and Glass to the appearance/accessibility entries; the window content
    // never reads this map directly.
    Q_PROPERTY(QVariantMap values READ values NOTIFY valuesChanged)
    // Whether settingsd currently owns the bus name. False is a normal state:
    // the client serves the schema defaults and the window still styles
    // itself.
    Q_PROPERTY(bool available READ available NOTIFY availableChanged)
    // The explicit rollout override, read from the same environment variable
    // as the shell and Settings app. Empty means the shipped token default.
    Q_PROPERTY(QString glassRollout READ glassRollout CONSTANT)
    // T206: whether the app window has a client-alpha composition path.
    // `DF_APP_CHROME_SOLID=1` forces the opaque fallback for captures.
    Q_PROPERTY(bool appChromeSupported READ appChromeSupported CONSTANT)
    // The effective wallpaper's average sRGB luma (0..1), resolved from the
    // same `wallpaper.source` → `builtinDefault` → `providerSource` precedence
    // the shell forwards to the compositor. The desktop uses it to pick the
    // menu bar's near-white/black label pair; 0.5 is the no-wallpaper fallback.
    Q_PROPERTY(qreal wallpaperLuminance READ wallpaperLuminance
               NOTIFY wallpaperLuminanceChanged)

public:
    explicit FilesSettings(QObject *parent = nullptr);

    QVariantMap values() const;
    bool available() const;
    QString glassRollout() const;
    bool appChromeSupported() const;
    qreal wallpaperLuminance() const;

    // The shared client's write path, exposed only so the headless QML tests
    // can drive an external settings change and prove the app-local bindings
    // converge. Files itself has no appearance UI and never calls this.
    Q_INVOKABLE void set(const QString &key, const QVariant &value);

signals:
    void valuesChanged();
    void availableChanged();
    void wallpaperLuminanceChanged();

private:
    QString effectiveWallpaperSource() const;
    SettingsClient *m_client = nullptr;
    mutable QString m_wallpaperLumaKey;
    mutable qreal m_wallpaperLuma = 0.5;
};
