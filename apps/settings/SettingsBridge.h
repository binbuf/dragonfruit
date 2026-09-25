// SPDX-License-Identifier: MIT
// The Settings app's live settings client, exposed to QML as the `Settings`
// singleton (T-09.1b).
//
// Panes never touch D-Bus: `Settings.values` is a reactive map of every
// `org.dragonfruit.Settings1` key, and `Settings.set(key, value)` writes one
// through settingsd (optimistically local first, then mirrored). A control
// binds to the map and writes on change, so the key and the consumer both
// update without restart:
//
//     Toggle {
//         onToggled: (checked) => Settings.set("accessibility.reduceMotion", checked)
//         Binding {
//             target: reduceMotion
//             property: "checked"
//             value: Settings.values["accessibility.reduceMotion"] === true
//         }
//     }
//
// With no daemon on the bus the underlying `DbusSettingsClient` serves the
// schema defaults and keeps writes in memory, so panes still work. Tests set
// `DF_SETTINGS_FIXTURE` to force the deterministic `MockSettingsClient`.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariant>
#include <QVariantList>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

class SettingsClient;

class SettingsBridge : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Settings)
    QML_SINGLETON
    // Every key's current value. QML bindings such as
    // `Settings.values["dock.size"]` re-evaluate when this changes; it is the
    // one reactive surface, so no pane polls.
    Q_PROPERTY(QVariantMap values READ values NOTIFY valuesChanged)
    // Whether the daemon currently owns the bus name. False means defaults +
    // in-memory writes (the absent-provider state panes must degrade under).
    Q_PROPERTY(bool available READ available NOTIFY availableChanged)
    // The Wallpaper pane's built-in artwork (T-09.3): one entry per original
    // gradient preset, each `{ id, name, collection, source, url }` where
    // `source` is an absolute image path the compositor can decode and `url` a
    // `file://` URL for the QML preview. Stable for the process lifetime.
    Q_PROPERTY(QVariantList wallpaperPresets READ wallpaperPresets CONSTANT)
    // Whether the xdg-desktop-portal FileChooser is on the session bus; the
    // "Add Photo…" button disables when it is not (absent provider).
    Q_PROPERTY(bool wallpaperChooserAvailable READ wallpaperChooserAvailable NOTIFY
                   wallpaperChooserAvailableChanged)
    // The pane the shell opens on startup. Empty uses the first shipped pane;
    // `DF_SETTINGS_START_PANE=wallpaper` selects one for captures and tests.
    Q_PROPERTY(QString startPane READ startPane CONSTANT)
    // Temporary diagnostic: whether `DF_SETTINGS_TRACE` is set, so QML can
    // emit `DFTRACE` lines for the pane-switch timing trace.
    Q_PROPERTY(bool trace READ trace CONSTANT)

public:
    explicit SettingsBridge(QObject *parent = nullptr);
    ~SettingsBridge() override;

    QVariantMap values() const;
    bool available() const;
    QVariantList wallpaperPresets() const;
    bool wallpaperChooserAvailable() const;
    QString startPane() const;
    bool trace() const;

    // Read one key (with an optional fallback for an unknown key).
    Q_INVOKABLE QVariant value(const QString &key, const QVariant &fallback = {}) const;
    // Write one key through settingsd. The local store updates on the same
    // event-loop turn; the daemon's `Changed` echo is de-duplicated.
    Q_INVOKABLE void set(const QString &key, const QVariant &value);
    // Re-read the daemon snapshot (`GetAll`); a no-op when no daemon is up.
    Q_INVOKABLE void refresh();
    // Temporary diagnostics: emit one `DFTRACE <message> t=<epoch-ms>` line to
    // stderr when `DF_SETTINGS_TRACE` is set. `fprintf` (not `console.log`) so
    // it survives the app's Qt logging rules and lands in the demo log.
    Q_INVOKABLE void traceLog(const QString &message) const;
    // Every key the schema (and the defaults table) knows.
    Q_INVOKABLE QStringList keys() const;

    // Open the portal file chooser at the user's Pictures directory. A chosen
    // image is announced through `wallpaperPhotoChosen`; a cancelled or
    // unavailable chooser emits nothing (the pane stays on the current image).
    Q_INVOKABLE void chooseWallpaperPhoto();

    // `file:///path/to/a.png` / `file://host/path` -> a local path. Pure, so
    // the URI contract is unit-testable.
    static QString localPathFromUri(const QString &uri);

signals:
    void valuesChanged();
    void availableChanged(bool available);
    // One key really changed (local write or daemon signal). `values` also
    // changes; listen to this only when a key's identity matters.
    void changed(const QString &key, const QVariant &value);
    void wallpaperChooserAvailableChanged();
    // The chosen photo's local path, ready for `wallpaper.source`.
    void wallpaperPhotoChosen(const QString &path);

private:
    void buildWallpaperPresets();
    void connectPortalWatcher();
    void setChooserAvailable(bool available);

private slots:
    void onPortalResponse(uint response, const QVariantMap &results);

private:
    SettingsClient *m_client = nullptr;
    QVariantList m_presets;
    bool m_chooserAvailable = false;
    QString m_requestPath;
};