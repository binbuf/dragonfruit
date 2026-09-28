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
class BluetoothClient;
class StorageClient;
class SoundClient;
class InputClient;
class BatteryClient;
class NotificationsClient;
class UpdatesClient;
class AccountsClient;
class PrintersClient;
class PrivacyClient;
class AccessibilityClient;
class QDBusServiceWatcher;

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
    // The online provider catalogue (T-18.1b): one entry per cached Featured
    // picture, mirroring `wallpaperPresets`. Empty when the provider is absent
    // or its cache is cold; never an error. Each entry carries the attribution
    // (`artist`, `licenseShortName`, `licenseUrl`, `pageUrl`, `description`)
    // plus `source` (an absolute local path) and `url` (a `file://` preview).
    Q_PROPERTY(QVariantList providerItems READ providerItems NOTIFY providerChanged)
    // The provider lifecycle (`idle`/`fetching`/`ready`/`offline`), empty when
    // the provider is absent. Panes must treat absence as a normal state.
    Q_PROPERTY(QString providerStatus READ providerStatus NOTIFY providerChanged)
    // The fetched Featured default/fallback local path; empty until a
    // catalogue exists.
    Q_PROPERTY(QString providerDefault READ providerDefault NOTIFY providerChanged)
    // The resolved shipped `Default.jpg`; non-empty even with the provider
    // absent, so the Built-in row always has its out-of-box entry (ADR 0094).
    Q_PROPERTY(QString wallpaperBuiltinDefault READ wallpaperBuiltinDefault NOTIFY providerChanged)
    // How many eager `Preload` requests the bridge has sent (T-18.2). The
    // Wallpaper pane calls `preloadWallpapers()` when it opens; a test asserts
    // this counter advanced. With `DF_WALLPAPER_FIXTURE` the request is served
    // in-process, so the count is observable without a bus.
    Q_PROPERTY(int providerPreloadCount READ providerPreloadCount NOTIFY providerChanged)
    // The Bluetooth view from the bridge host (T-15.1b): `{ state, present,
    // powered, discovering, label, adapterName, knownDevices, nearbyDevices }`.
    // Empty when the host (or BlueZ) is absent; the pane renders the absence
    // state and disables its controls rather than erroring.
    Q_PROPERTY(QVariantMap bluetooth READ bluetooth NOTIFY bluetoothChanged)
    // Whether the bridge host is on the session bus. False means no Bluetooth
    // surface at all; the pane shows the absence note.
    Q_PROPERTY(bool bluetoothAvailable READ bluetoothAvailable NOTIFY bluetoothChanged)
    // The storage view from the bridge host (T-15.2b): `{ state, present,
    // label, mountedCount, volumes, drives }`. Empty when the host (or
    // UDisks2) is absent; the pane renders the absence state and disables its
    // controls rather than erroring.
    Q_PROPERTY(QVariantMap storage READ storage NOTIFY storageChanged)
    // Whether the bridge host is on the session bus. False means no storage
    // surface at all; the pane shows the absence note.
    Q_PROPERTY(bool storageAvailable READ storageAvailable NOTIFY storageChanged)
    // The audio view from the bridge host (T-15.3b): `{ state, volume, muted,
    // defaultSink, defaultSource, sinks, sources }`. Empty when the host (or
    // WirePlumber) is absent; the pane renders the absence state and disables
    // its controls rather than erroring.
    Q_PROPERTY(QVariantMap sound READ sound NOTIFY soundChanged)
    // Whether the bridge host is on the session bus. False means no Sound
    // surface at all; the pane shows the absence note.
    Q_PROPERTY(bool soundAvailable READ soundAvailable NOTIFY soundChanged)
    // The read-only input inventory from the bridge host (T-15.4b):
    // `{ state, present, label, glyph, deviceCount, keyboardCount,
    // pointerCount, mouseCount, touchpadCount, devices }`. Empty when the host
    // (or libinput) is absent; the pane renders the absence state rather than
    // erroring. The pane's preferences are settingsd keys, not this view.
    Q_PROPERTY(QVariantMap input READ input NOTIFY inputChanged)
    // Whether the bridge host is on the session bus. False means no input
    // inventory at all; the pane shows the absence note.
    Q_PROPERTY(bool inputAvailable READ inputAvailable NOTIFY inputChanged)
    // The battery and power-profiles view from the bridge host (T-15.6b):
    // `{ state, present, percent, level, charging, plugged, onBattery,
    // chargeState, health, healthLabel, capacity, chargeCycles,
    // profilesAvailable, activeProfile, profileLabel, profiles }`. Empty when
    // the host (or both daemons) is absent; the pane renders the absence state
    // rather than erroring.
    Q_PROPERTY(QVariantMap battery READ battery NOTIFY batteryChanged)
    // Whether the bridge host is on the session bus. False means no battery or
    // power-profile surface at all; the pane shows the absence note.
    Q_PROPERTY(bool batteryAvailable READ batteryAvailable NOTIFY batteryChanged)
    // The Notifications/Focus view from the bridge host (T-15.7b):
    // `{ state, glyph, label, mode, modeLabel, suppressing, dnd, batched,
    // allowList, activeCount, historyCount, appCount, apps }`. Empty when the
    // host (or the notification service) is absent; the panes render the
    // absence state rather than erroring. The four global presentation
    // preferences are settingsd keys, not this view.
    Q_PROPERTY(QVariantMap notifications READ notifications NOTIFY notificationsChanged)
    // Whether the bridge host is on the session bus. False means no
    // Notifications/Focus surface at all; the panes show the absence note.
    Q_PROPERTY(bool notificationsAvailable READ notificationsAvailable NOTIFY notificationsChanged)
    // The General/About/Updates view from the bridge host (T-15.10b):
    // `{ state, hostName, deviceName, osLabel, kernel, architecture, processor,
    // memoryLabel, serial, hasSerial, updatesAvailable, glyph, label, phase,
    // busy, rebootRequired, updateCount, securityCount, lastCheckedMs,
    // message, updates }`. Empty when the host is absent; the pane renders the
    // absence state and disables its controls rather than erroring.
    Q_PROPERTY(QVariantMap updates READ updates NOTIFY updatesChanged)
    // Whether the bridge host is on the session bus. False means no General /
    // About / Updates surface at all; the pane shows the absence note.
    Q_PROPERTY(bool updatesAvailable READ updatesAvailable NOTIFY updatesChanged)
    // The Users and Groups view from the bridge host (T-15.11b):
    // `{ state, glyph, label, present, humanCount, adminCount, lockedCount,
    // groupsAvailable, groupCount, automaticLogin, automaticLoginUser,
    // automaticLoginUid, users, groups }`. Empty when the host is absent; the
    // pane renders the absence state rather than erroring. A host with no group
    // provider is `available` with `groupsAvailable: false`, so only the group
    // controls disable.
    Q_PROPERTY(QVariantMap accounts READ accounts NOTIFY accountsChanged)
    // Whether the bridge host is on the session bus. False means no Users &
    // Groups surface at all; the pane shows the absence note.
    Q_PROPERTY(bool accountsAvailable READ accountsAvailable NOTIFY accountsChanged)
    // The Printers and Scanners view from the bridge host (T-15.12b):
    // `{ state, glyph, label, present, printersAvailable, scannersAvailable,
    // printerCount, scannerCount, queuedJobCount, defaultPrinter, printers,
    // scanners }`. Empty when the host is absent; the pane renders the absence
    // state rather than erroring. A host with no SANE is `available` with
    // `scannersAvailable: false`, so only the scanner half is marked absent.
    Q_PROPERTY(QVariantMap printers READ printers NOTIFY printersChanged)
    // Whether the bridge host is on the session bus. False means no Printers &
    // Scanners surface at all; the pane shows the absence note.
    Q_PROPERTY(bool printersAvailable READ printersAvailable NOTIFY printersChanged)
    // The Privacy and Security view from the bridge host (T-15.13b):
    // `{ state, glyph, label, present, appCount, grantedCount, deniedCount,
    // categoryCount, categories }`. Empty when the host is absent; the pane
    // renders the absence state rather than erroring. Every portal category is
    // a row (empty ones say `None`); the tile hides when `present` is false
    // (the store records no application permission).
    Q_PROPERTY(QVariantMap privacy READ privacy NOTIFY privacyChanged)
    // Whether the bridge host is on the session bus. False means no Privacy &
    // Security surface at all; the pane shows the absence note.
    Q_PROPERTY(bool privacyAvailable READ privacyAvailable NOTIFY privacyChanged)
    // The Accessibility view from the bridge host (T-15.14b): `{ state, glyph,
    // label, present, enabled, enabledLabel, screenReader, screenReaderLabel }`.
    // Empty when the host (or the AT-SPI bus) is absent; the pane renders the
    // absence state rather than erroring. The pane's one durable preference
    // (`accessibility.reduceMotion`) is a settingsd key in `values`, not this
    // live read.
    Q_PROPERTY(QVariantMap accessibility READ accessibility NOTIFY accessibilityChanged)
    // Whether the bridge host is on the session bus. False means no
    // Accessibility surface at all; the pane shows the absence note.
    Q_PROPERTY(bool accessibilityAvailable READ accessibilityAvailable NOTIFY
                   accessibilityChanged)
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
    QVariantList providerItems() const;
    QVariantMap bluetooth() const;
    bool bluetoothAvailable() const;
    QVariantMap storage() const;
    bool storageAvailable() const;
    QVariantMap sound() const;
    bool soundAvailable() const;
    QVariantMap input() const;
    bool inputAvailable() const;
    QVariantMap battery() const;
    bool batteryAvailable() const;
    QVariantMap notifications() const;
    bool notificationsAvailable() const;
    QVariantMap updates() const;
    bool updatesAvailable() const;
    QVariantMap accounts() const;
    bool accountsAvailable() const;
    QVariantMap printers() const;
    bool printersAvailable() const;
    QVariantMap privacy() const;
    bool privacyAvailable() const;
    QVariantMap accessibility() const;
    bool accessibilityAvailable() const;
    QString providerStatus() const;
    QString providerDefault() const;
    QString wallpaperBuiltinDefault() const;
    int providerPreloadCount() const;
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

    // T-18.1b: ask the wallpaper provider for an eager catalogue load (the
    // Wallpapers pane calls this on open). A no-op when the provider is absent;
    // the catalogue arrives through `providerChanged`.
    Q_INVOKABLE void preloadWallpapers();

    // T-15.1b: the Bluetooth pane's one seam. `refreshBluetooth` re-reads the
    // bridge host on pane open; the four writes each call the adapter once and
    // the host pushes the new view back through `bluetoothChanged`. A no-op
    // when the host is absent.
    Q_INVOKABLE void refreshBluetooth();
    Q_INVOKABLE void setBluetoothPowered(bool powered);
    Q_INVOKABLE void setBluetoothDiscovering(bool discovering);
    Q_INVOKABLE void pairBluetooth(const QString &address);
    Q_INVOKABLE void setBluetoothConnected(const QString &address, bool connected);

    // T-15.2b: the Storage pane's one seam. `refreshStorage` re-reads the
    // bridge host on pane open; the three writes each call the adapter once
    // and the host pushes the new view back through `storageChanged`. A no-op
    // when the host is absent.
    Q_INVOKABLE void refreshStorage();
    Q_INVOKABLE void mountStorage(const QString &volumePath);
    Q_INVOKABLE void unmountStorage(const QString &volumePath);
    Q_INVOKABLE void ejectStorage(const QString &drivePath);

    // T-15.3b: the Sound pane's one seam. `refreshSound` re-reads the bridge
    // host on pane open; the writes each call the audio adapter once and the
    // host pushes the new view back through `soundChanged`. Device routing is
    // by PipeWire node id. A no-op when the host is absent.
    Q_INVOKABLE void refreshSound();
    Q_INVOKABLE void setSoundVolume(double volume);
    Q_INVOKABLE void setSoundMute(bool muted);
    Q_INVOKABLE void setSoundDefaultSink(int id);
    Q_INVOKABLE void setSoundDefaultSource(int id);

    // T-15.4b: the Keyboard/Mouse/Trackpad pane's one inventory seam.
    // `refreshInput` re-reads the bridge host on pane open; the view is
    // read-only (libinput has no setter), so `input` carries no write. A no-op
    // when the host is absent.
    Q_INVOKABLE void refreshInput();

    // T-15.6b: the Battery pane's one seam. `refreshBattery` re-reads the
    // bridge host on pane open; `setPowerProfile` selects the active profile by
    // its stable id (`power-saver`/`balanced`/`performance`) and the host
    // pushes the new view back through `batteryChanged`. A no-op when the host
    // is absent.
    Q_INVOKABLE void refreshBattery();
    Q_INVOKABLE void setPowerProfile(const QString &profile);

    // T-15.7b: the Notifications/Focus panes' one seam. `refreshNotifications`
    // re-reads the bridge host on pane open; `setFocusMode` selects the policy
    // (`off`/`focus`/`dnd`) and `setFocusApp` adds or removes one app from the
    // per-app allow list, reading the current list first so a toggle changes
    // exactly one entry. A no-op when the host is absent.
    Q_INVOKABLE void refreshNotifications();
    Q_INVOKABLE void setFocusMode(const QString &mode);
    Q_INVOKABLE void setFocusApp(const QString &app, bool allowed);

    // T-15.10b: the General/About/Updates pane's one seam. `refreshUpdates`
    // re-reads the bridge host on pane open; the three writes each call the
    // host stack adapter once and the host pushes the new view back through
    // `updatesChanged`. A no-op when the host is absent.
    Q_INVOKABLE void refreshUpdates();
    Q_INVOKABLE void checkUpdates();
    Q_INVOKABLE void installUpdates();
    Q_INVOKABLE void rebootUpdates();
    // Test seam (`DF_UPDATES_FIXTURE` only): restore the in-process fixture's
    // initial state so a test starts on a known update phase. A no-op on the
    // live client, which has no fixture to reset.
    Q_INVOKABLE void resetUpdatesFixture();

    // T-15.11b: the Users & Groups pane's one seam. `refreshAccounts` re-reads
    // the bridge host on pane open; each write calls the host stack adapter
    // once and the host pushes the new view back through `accountsChanged`. A
    // no-op when the host is absent. Account types are stable ids (`standard` /
    // `administrator`); automatic login takes a uid (`0` clears it).
    Q_INVOKABLE void refreshAccounts();
    Q_INVOKABLE void createAccount(const QString &userName, const QString &realName,
                                   const QString &accountType);
    Q_INVOKABLE void deleteAccount(int uid);
    Q_INVOKABLE void setAccountType(int uid, const QString &accountType);
    Q_INVOKABLE void setAccountLocked(int uid, bool locked);
    Q_INVOKABLE void setAccountAutomaticLogin(int uid, bool automaticLogin);
    Q_INVOKABLE void createAccountGroup(const QString &name);
    Q_INVOKABLE void deleteAccountGroup(const QString &name);
    Q_INVOKABLE void setAccountGroupMembers(const QString &name,
                                            const QStringList &members);
    // Test seam (`DF_ACCOUNTS_FIXTURE` only): restore the in-process fixture's
    // initial stack. A no-op on the live client.
    Q_INVOKABLE void resetAccountsFixture();

    // T-15.12b: the Printers & Scanners pane's one seam. `refreshPrinters`
    // re-reads the bridge host on pane open; the three queue writes each call
    // the host stack adapter once and the host pushes the new view back through
    // `printersChanged`. A no-op when the host is absent.
    Q_INVOKABLE void refreshPrinters();
    Q_INVOKABLE void setDefaultPrinter(const QString &name);
    Q_INVOKABLE void setPrinterAcceptingJobs(const QString &name, bool accepting);
    Q_INVOKABLE void cancelPrinterJob(int jobId);
    // Test seam (`DF_PRINTERS_FIXTURE` only): restore the in-process fixture's
    // initial queues. A no-op on the live client.
    Q_INVOKABLE void resetPrintersFixture();

    // T-15.13b: the Privacy & Security pane's one seam. `refreshPrivacy`
    // re-reads the bridge host on pane open; the two permission writes each
    // call the host stack adapter once and the host pushes the new view back
    // through `privacyChanged`. `permission` is the stable tristate id
    // (`allowed`/`denied`/`ask`); an empty `permission` removes the record. A
    // no-op when the host is absent.
    Q_INVOKABLE void refreshPrivacy();
    Q_INVOKABLE void setPrivacyPermission(const QString &table, const QString &resourceId,
                                          const QString &app, const QString &permission);
    Q_INVOKABLE void deletePrivacyPermission(const QString &table, const QString &resourceId,
                                             const QString &app);
    // Test seam (`DF_PRIVACY_FIXTURE` only): restore the in-process fixture's
    // initial store. A no-op on the live client.
    Q_INVOKABLE void resetPrivacyFixture();

    // T-15.14b: the Accessibility pane's one seam. `refreshAccessibility`
    // re-reads the bridge host on pane open; the adapter is read-only, so there
    // is no write here (the pane's `accessibility.reduceMotion` toggle goes
    // through `set`). A no-op when the host is absent.
    Q_INVOKABLE void refreshAccessibility();
    // Test seam (`DF_ACCESSIBILITY_FIXTURE` only): restore the in-process
    // fixture. A no-op on the live client.
    Q_INVOKABLE void resetAccessibilityFixture();

    // T-18.2 test seam: with `DF_WALLPAPER_FIXTURE` set, seed the provider
    // lifecycle to `status` (`ready` loads the deterministic fixture
    // catalogue; any other status leaves it empty) so the pane's fetching /
    // ready / offline / error states are assertable with no bus. Without the
    // environment variable this is a no-op, so production behavior is
    // unchanged.
    Q_INVOKABLE void setWallpaperFixture(const QString &status);

    // `file:///path/to/a.png` / `file://host/path` -> a local path. Pure, so
    // the URI contract is unit-testable.
    static QString localPathFromUri(const QString &uri);
    // Strip tags and decode the handful of HTML entities Wikimedia's
    // `extmetadata` uses, so an `Artist` value is never rendered as markup.
    static QString sanitizeHtmlText(const QString &value);

signals:
    void valuesChanged();
    void availableChanged(bool available);
    // One key really changed (local write or daemon signal). `values` also
    // changes; listen to this only when a key's identity matters.
    void changed(const QString &key, const QVariant &value);
    void wallpaperChooserAvailableChanged();
    // The chosen photo's local path, ready for `wallpaper.source`.
    void wallpaperPhotoChosen(const QString &path);
    // Any provider property (catalogue, status, defaults) changed. Panes bind
    // the `provider*` properties and re-read on this.
    void providerChanged();
    // The Bluetooth view or availability changed (T-15.1b).
    void bluetoothChanged();
    // The storage view or availability changed (T-15.2b).
    void storageChanged();
    // The audio view or availability changed (T-15.3b).
    void soundChanged();
    // The input inventory or availability changed (T-15.4b).
    void inputChanged();
    // The battery / power-profiles view or availability changed (T-15.6b).
    void batteryChanged();
    // The Notifications/Focus view or availability changed (T-15.7b).
    void notificationsChanged();
    // The General/About/Updates view or availability changed (T-15.10b).
    void updatesChanged();
    // The Users and Groups view or availability changed (T-15.11b).
    void accountsChanged();
    // The Printers and Scanners view or availability changed (T-15.12b).
    void printersChanged();
    // The Privacy and Security view or availability changed (T-15.13b).
    void privacyChanged();
    // The Accessibility view or availability changed (T-15.14b).
    void accessibilityChanged();

private:
    void buildWallpaperPresets();
    void connectPortalWatcher();
    void setChooserAvailable(bool available);
    // T-18.1b: subscribe to `org.dragonfruit.Wallpaper1` and read its
    // properties. Absence is normal: the catalogue stays empty and
    // `wallpaperBuiltinDefault` still resolves the shipped asset.
    void connectWallpaperProvider();
    void refreshWallpaperProvider();
    void applyProviderItems(const QString &json);
    void applyProviderStatus(const QString &status);
    void applyProviderDefault(const QString &path);
    void applyProviderBuiltin(const QString &path);

private slots:
    void onPortalResponse(uint response, const QVariantMap &results);
    // T-18.1b: the provider's contract signal and its standard property
    // notification. Both keep the pane's catalogue and defaults in step.
    void onProviderItemsChanged();
    void onProviderPropertiesChanged(const QString &interface, const QVariantMap &changed,
                                     const QStringList &invalidated);

private:
    SettingsClient *m_client = nullptr;
    QVariantList m_presets;
    bool m_chooserAvailable = false;
    QString m_requestPath;
    // T-18.1b: the provider catalogue and its resolved defaults. `m_providerBuiltin`
    // is the provider's own `BuiltinDefaultSource`; the property falls back to
    // the shared shipped-asset resolver so it is non-empty even when absent.
    QVariantList m_providerItems;
    QString m_providerStatus;
    QString m_providerDefault;
    QString m_providerBuiltin;
    int m_providerPreloads = 0;
    // T-18.2: true when `DF_WALLPAPER_FIXTURE` selected the in-process
    // catalogue, so `Preload` and fixture seeding stay deterministic.
    bool m_wallpaperFixture = false;
    QDBusServiceWatcher *m_providerWatcher = nullptr;
    // T-15.1b: the Bluetooth seam (`DF_BLUETOOTH_FIXTURE` selects the mock).
    BluetoothClient *m_bluetooth = nullptr;
    // T-15.2b: the storage seam (`DF_STORAGE_FIXTURE` selects the mock).
    StorageClient *m_storage = nullptr;
    // T-15.3b: the Sound seam (`DF_SOUND_FIXTURE` selects the mock).
    SoundClient *m_sound = nullptr;
    // T-15.4b: the input inventory seam (`DF_INPUT_FIXTURE` selects the mock).
    InputClient *m_input = nullptr;
    // T-15.6b: the battery / power-profiles seam (`DF_BATTERY_FIXTURE` selects
    // the mock).
    BatteryClient *m_battery = nullptr;
    // T-15.7b: the Notifications/Focus seam (`DF_NOTIFICATIONS_FIXTURE`
    // selects the mock).
    NotificationsClient *m_notifications = nullptr;
    // T-15.10b: the General/About/Updates seam (`DF_UPDATES_FIXTURE` selects
    // the mock).
    UpdatesClient *m_updates = nullptr;
    // T-15.11b: the Users & Groups seam (`DF_ACCOUNTS_FIXTURE` selects the
    // mock).
    AccountsClient *m_accounts = nullptr;
    // T-15.12b: the Printers & Scanners seam (`DF_PRINTERS_FIXTURE` selects the
    // mock).
    PrintersClient *m_printers = nullptr;
    // T-15.13b: the Privacy & Security seam (`DF_PRIVACY_FIXTURE` selects the
    // mock).
    PrivacyClient *m_privacy = nullptr;
    // T-15.14b: the Accessibility seam (`DF_ACCESSIBILITY_FIXTURE` selects the
    // mock).
    AccessibilityClient *m_accessibility = nullptr;
};