// SPDX-License-Identifier: GPL-3.0-or-later
// The Settings app's live settings client, exposed to QML as the `Settings`
// singleton (T-09.1b).
//
// Panes never touch D-Bus: `Settings.values` is a reactive per-key property
// map of every `org.dragonfruit.Settings1` key, and `Settings.set(key, value)`
// writes one through settingsd (optimistically local first, then mirrored). A
// control binds to one key and writes on change, so the key and the consumer
// both update without restart:
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

#include <QHash>
#include <QObject>
#include <QQmlPropertyMap>
#include <QString>
#include <functional>
#include <QStringList>
#include <QUrl>
#include <QVariant>
#include <QVariantList>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

class SettingsClient;
class SettingsAuthorizer;
class BluetoothClient;
class StorageClient;
class SoundClient;
class InputClient;
class BatteryClient;
class NotificationsClient;
class UpdatesClient;
class GeneralClient;
class AccountsClient;
class PrintersClient;
class PrivacyClient;
class AccessibilityClient;
class VpnClient;
class WifiClient;
class NetworkClient;
class QDBusServiceWatcher;
class QProcess;
class QScreen;

class SettingsBridge : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Settings)
    QML_SINGLETON
    // Every key's current value. This is a `QQmlPropertyMap`, so a binding
    // such as `Settings.values["dock.size"]` tracks exactly that key: one
    // settingsd change no longer re-evaluates every pane binding in the app.
    // The map is the compatibility facade for the T-09 bracket syntax; C++
    // consumers keep `SettingsClient::values()`.
    Q_PROPERTY(QQmlPropertyMap *values READ valuesMap CONSTANT)
    // Whether the daemon currently owns the bus name. False means defaults +
    // in-memory writes (the absent-provider state panes must degrade under).
    Q_PROPERTY(bool available READ available NOTIFY availableChanged)
    // T224: the common apply path's state. `writeErrors` maps a key to the
    // last failure message (empty when it succeeded); `writeStates` maps a key
    // to `idle`/`pending`/`deferred`/`error`. Both are per-key property maps
    // (T268), so a row only re-evaluates for its own key.
    Q_PROPERTY(QQmlPropertyMap *writeErrors READ writeErrorsMap CONSTANT)
    Q_PROPERTY(QQmlPropertyMap *writeStates READ writeStatesMap CONSTANT)
    // T224: a denied/cancelled privileged account action. The pane shows this
    // next to the control; there is no password field in the app.
    Q_PROPERTY(QString accountsError READ accountsError NOTIFY accountsErrorChanged)
    // The Wallpaper pane's built-in artwork (T-09.3): one entry per original
    // gradient preset, each `{ id, name, collection, source, url }` where
    // `source` is an absolute image path the compositor can decode and `url` a
    // `file://` URL for the QML preview. Stable for the process lifetime.
    Q_PROPERTY(QVariantList wallpaperPresets READ wallpaperPresets CONSTANT)
    // Whether the xdg-desktop-portal FileChooser is on the session bus; the
    // "Add Photo…" button disables when it is not (absent provider).
    Q_PROPERTY(bool wallpaperChooserAvailable READ wallpaperChooserAvailable NOTIFY
                   wallpaperChooserAvailableChanged)
    // The online provider catalogue (T-18.1b / ADR 0240): one entry per cached
    // picture, mirroring `wallpaperPresets`. Empty when the provider is absent
    // or its cache is cold; never an error. Each entry carries the attribution
    // (`provider`, `artist`, `licenseShortName`, `licenseUrl`, `pageUrl`,
    // `description`) plus `source` (the full-resolution local path, empty for a
    // deferred item), `thumbUrl` (the grid image), `previewUrl` (the full image
    // when local, else the thumb), `pageid`, and `category`.
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
    // The other bundled built-in wallpapers, as `{ name, source }` entries,
    // resolved next to the shipped default. Empty when the package predates
    // them; the Built-in row then shows only Default.
    Q_PROPERTY(QVariantList wallpaperBuiltinAlternatives READ wallpaperBuiltinAlternatives NOTIFY providerChanged)
    // How many eager `Preload` requests the bridge has sent (T-18.2). The
    // Wallpaper pane calls `preloadWallpapers()` when it opens; a test asserts
    // this counter advanced. With `DF_WALLPAPER_FIXTURE` the request is served
    // in-process, so the count is observable without a bus.
    Q_PROPERTY(int providerPreloadCount READ providerPreloadCount NOTIFY providerChanged)
    // T276: the Desktop & Dock pane's default-browser row. The choices come
    // from app-index's installed `WebBrowser` records; the current value is
    // `defaults.browser` (empty follows the host default). The write path is
    // the freedesktop default-apps delegate (`xdg-settings`), which the
    // Settings portal exposes read-only through `org.dragonfruit.desktop`.
    Q_PROPERTY(QVariantList defaultBrowserChoices READ defaultBrowserChoices NOTIFY
                   defaultBrowserChanged)
    Q_PROPERTY(QString defaultBrowser READ defaultBrowser NOTIFY defaultBrowserChanged)
    Q_PROPERTY(QString defaultBrowserError READ defaultBrowserError NOTIFY
                   defaultBrowserChanged)
    // T337: true while the app-index `Enumerate` or the host default-apps
    // delegate is in flight. The row shows a loading state and keeps the last
    // known choices/value; an absent source is an absence, not a spinner.
    Q_PROPERTY(bool defaultBrowserLoading READ defaultBrowserLoading NOTIFY
                   defaultBrowserChanged)
    // T277: the live StatusNotifier/AppIndicator items from app-index, for the
    // Menu Bar pane's "Allow in the Menu Bar" list. Each entry is
    // `{ id, name, label, icon, iconPath, tooltip, state }`. Empty when
    // app-index is absent; that is a normal absence, never an error.
    Q_PROPERTY(QVariantList thirdPartyMenuBarItems READ thirdPartyMenuBarItems
                   NOTIFY thirdPartyMenuBarItemsChanged)
    Q_PROPERTY(bool thirdPartyMenuBarAvailable READ thirdPartyMenuBarAvailable
                   NOTIFY thirdPartyMenuBarItemsChanged)
    // The Bluetooth view from the bridge host (T-15.1b): `{ state, present,
    // powered, discovering, label, adapterName, knownDevices, nearbyDevices }`.
    // Empty when the host (or BlueZ) is absent; the pane renders the absence
    // state and disables its controls rather than erroring.
    Q_PROPERTY(QVariantMap bluetooth READ bluetooth NOTIFY bluetoothChanged)
    // Whether the bridge host is on the session bus. False means no Bluetooth
    // surface at all; the pane shows the absence note.
    Q_PROPERTY(bool bluetoothAvailable READ bluetoothAvailable NOTIFY bluetoothChanged)
    // T271: the last Bluetooth host write's operation (`power`, `discovery`,
    // `pair`, `connect`, `forget`), state, and message.
    Q_PROPERTY(QString bluetoothWriteOperation READ bluetoothWriteOperation
                   NOTIFY bluetoothWriteChanged)
    Q_PROPERTY(QString bluetoothWriteState READ bluetoothWriteState NOTIFY
                   bluetoothWriteChanged)
    Q_PROPERTY(QString bluetoothWriteError READ bluetoothWriteError NOTIFY
                   bluetoothWriteChanged)
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
    // T229: the last controller test report (`{outcome, path, detail, bytes}`),
    // pushed by `testController`. Empty until a test runs.
    Q_PROPERTY(QVariantMap controllerTest READ controllerTest NOTIFY controllerTestChanged)
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
    // The General sub-panes view from the bridge host (T226): `{ kind, state,
    // time, locale, login, sharing, backup, storage, startup, errors }`, each
    // sub-view carrying its own `state`. Empty when the host is absent; the
    // pane renders the per-sub-pane absence notes.
    Q_PROPERTY(QVariantMap general READ general NOTIFY generalChanged)
    // Whether the General bridge host is on the session bus.
    Q_PROPERTY(bool generalAvailable READ generalAvailable NOTIFY generalChanged)
    // T270: the last General host write's operation, state (`idle`, `busy`,
    // `applied`, `absent`, `denied`, `failed`), and message. A pane that owns
    // a host write shows the busy/auth/error result next to its control instead
    // of leaving the control in the state the user attempted.
    Q_PROPERTY(QString generalWriteOperation READ generalWriteOperation NOTIFY
                   generalWriteChanged)
    Q_PROPERTY(QString generalWriteState READ generalWriteState NOTIFY
                   generalWriteChanged)
    Q_PROPERTY(QString generalWriteError READ generalWriteError NOTIFY
                   generalWriteChanged)
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
    // T229: the Add Printer discovery report (`{outcome, devices}`) and the
    // options sheet's report (`{outcome, options}`). Empty until requested.
    Q_PROPERTY(QVariantMap printerDiscovery READ printerDiscovery NOTIFY printerDiscoveryChanged)
    Q_PROPERTY(QVariantMap printerOptions READ printerOptions NOTIFY printerOptionsChanged)
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
    // The Security view from the bridge host (T228): `{ state, glyph, label,
    // softwareSources, encryption, hardening, extensions, advanced }`. Empty
    // when the security owners are absent; the pane keeps the privacy list
    // live and marks only the security rows absent.
    Q_PROPERTY(QVariantMap security READ security NOTIFY securityChanged)
    Q_PROPERTY(bool securityAvailable READ securityAvailable NOTIFY securityChanged)
    // The Biometrics and Password view from the bridge host (T228).
    Q_PROPERTY(QVariantMap biometrics READ biometrics NOTIFY biometricsChanged)
    Q_PROPERTY(bool biometricsAvailable READ biometricsAvailable NOTIFY biometricsChanged)
    // The Online Accounts view from the bridge host (T228). The pane is hidden
    // when this is false; GOA is optional.
    Q_PROPERTY(QVariantMap onlineAccounts READ onlineAccounts NOTIFY onlineAccountsChanged)
    Q_PROPERTY(bool onlineAccountsAvailable READ onlineAccountsAvailable NOTIFY
                   onlineAccountsChanged)
    // T279: the last biometrics/account write report. The password dialog
    // stays open until `biometricsWriteState` is `applied`; a denial or an
    // unsupported reader is shown at the row. `idle` until the first write.
    Q_PROPERTY(QString biometricsWriteOperation READ biometricsWriteOperation NOTIFY
                   biometricsWriteChanged)
    Q_PROPERTY(QString biometricsWriteState READ biometricsWriteState NOTIFY
                   biometricsWriteChanged)
    Q_PROPERTY(QString biometricsWriteError READ biometricsWriteError NOTIFY
                   biometricsWriteChanged)
    // The last account write (create user / set password) report, so the
    // Add User and password dialogs wait for the host too.
    Q_PROPERTY(QString accountsWriteOperation READ accountsWriteOperation NOTIFY
                   accountsWriteChanged)
    Q_PROPERTY(QString accountsWriteState READ accountsWriteState NOTIFY
                   accountsWriteChanged)
    Q_PROPERTY(QString accountsWriteError READ accountsWriteError NOTIFY
                   accountsWriteChanged)
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
    // T275: the three external delegate programs. A false value disables the
    // matching action with a reason; the pane never fakes a launch.
    Q_PROPERTY(bool accessibilityScreenReaderAvailable READ accessibilityScreenReaderAvailable
                   NOTIFY accessibilityChanged)
    Q_PROPERTY(bool accessibilitySpeechAvailable READ accessibilitySpeechAvailable NOTIFY
                   accessibilityChanged)
    Q_PROPERTY(bool accessibilityOnScreenKeyboardAvailable READ
                   accessibilityOnScreenKeyboardAvailable NOTIFY accessibilityChanged)
    // T275: the last delegate the pane launched, for tests and diagnostics.
    Q_PROPERTY(QString accessibilityLastDelegate READ accessibilityLastDelegate NOTIFY
                   accessibilityDelegateChanged)
    // The Network advanced (VPN) view from the bridge host (T-15.15b):
    // `{ state, glyph, label, present, connectionCount, connectedCount,
    // activeUuid, activeName, readOnly, note, connections }`. Empty when the
    // host (or NetworkManager) is absent; the pane renders the absence state
    // rather than erroring. NetworkManager owns the connection state, so the
    // pane has no durable settingsd preference of its own; the two writes are
    // the adapter's (`connectVpn`/`disconnectVpn`).
    Q_PROPERTY(QVariantMap vpn READ vpn NOTIFY vpnChanged)
    // Whether the bridge host is on the session bus. False means no Network
    // advanced (VPN) surface at all; the pane shows the absence note.
    Q_PROPERTY(bool vpnAvailable READ vpnAvailable NOTIFY vpnChanged)
    // The Wi-Fi view from the bridge host (T225): `{ state, glyph, label,
    // radioEnabled, wifiState, connectivity, activeSsid, activeAddress,
    // readOnly, note, networks, knownNetworks, services }`. The Wi-Fi pane and
    // the shell's quick sheet read the same shape; empty when the host (or
    // NetworkManager) is absent.
    Q_PROPERTY(QVariantMap wifi READ wifi NOTIFY wifiChanged)
    // Whether the bridge host is on the session bus. False means no Wi-Fi
    // surface at all; the pane shows the absence note.
    Q_PROPERTY(bool wifiAvailable READ wifiAvailable NOTIFY wifiChanged)
    // T271: the last Wi-Fi host write's operation (`power`, `join`,
    // `joinHidden`, `forget`, `autoconnect`, `scan`, `disconnect`, `hotspot`),
    // state (`idle`, `busy`, `applied`, `absent`, `denied`, `failed`), and
    // message. A failed write leaves the view untouched, so a bound control
    // reverts and the pane shows the reason.
    Q_PROPERTY(QString wifiWriteOperation READ wifiWriteOperation NOTIFY
                   wifiWriteChanged)
    Q_PROPERTY(QString wifiWriteState READ wifiWriteState NOTIFY wifiWriteChanged)
    Q_PROPERTY(QString wifiWriteError READ wifiWriteError NOTIFY wifiWriteChanged)
    // The Network pane's combined view from the bridge host (T225): the Wi-Fi
    // snapshot's services plus the `firewall` sub-view (`{ state, detected,
    // backend, backendLabel, statusKnown, enabled, ports, services, note }`).
    // Empty when the host is absent.
    Q_PROPERTY(QVariantMap network READ network NOTIFY networkChanged)
    // Whether the bridge host is on the session bus. False means no Network
    // surface at all; the pane shows the absence note.
    Q_PROPERTY(bool networkAvailable READ networkAvailable NOTIFY networkChanged)
    // T272: the last Network host write's operation (`activate`, `deactivate`,
    // `move`, `add`, `forget`, `autoconnect`, `firewall`, `port`, `service`),
    // state (`idle`, `busy`, `applied`, `absent`, `denied`, `failed`), and
    // message. A failed write leaves the view untouched, so a bound control
    // reverts and the pane shows the reason.
    Q_PROPERTY(QString networkWriteOperation READ networkWriteOperation NOTIFY
                   networkWriteChanged)
    Q_PROPERTY(QString networkWriteState READ networkWriteState NOTIFY
                   networkWriteChanged)
    Q_PROPERTY(QString networkWriteError READ networkWriteError NOTIFY
                   networkWriteChanged)
    // T272: the last VPN host write's operation (`connect`, `disconnect`,
    // `add`, `autoconnect`), state, and message.
    Q_PROPERTY(QString vpnWriteOperation READ vpnWriteOperation NOTIFY
                   vpnWriteChanged)
    Q_PROPERTY(QString vpnWriteState READ vpnWriteState NOTIFY vpnWriteChanged)
    Q_PROPERTY(QString vpnWriteError READ vpnWriteError NOTIFY vpnWriteChanged)
    // T268: diagnostic — how many pane adapter clients have been constructed
    // so far. Zero after the bridge is created; each pane's first `refresh*()`
    // adds exactly one. The startup budget test asserts no constructor fan-out.
    Q_PROPERTY(int adapterClientCount READ adapterClientCount NOTIFY
                   adapterClientCountChanged)
    // The pane the shell opens on startup. Empty uses the first shipped pane;
    // `DF_SETTINGS_START_PANE=wallpaper` selects one for captures and tests.
    Q_PROPERTY(QString startPane READ startPane CONSTANT)
    // T232: a `settings://pane/<id>` link carried on the cold-launch command
    // line. The shell consumes it once after the first pane is mounted.
    Q_PROPERTY(QString startDeepLink READ startDeepLink CONSTANT)
    // T263: whether the shell restores/persists the last pane and the window
    // restores/persists its geometry. Production always does. Headless tests
    // that force `DF_SETTINGS_FIXTURE` keep the old clean-process behavior
    // unless `DF_SETTINGS_WINDOW_STATE=1` opts in, so the pane suites do not
    // leak a previous case's pane.
    Q_PROPERTY(bool windowStatePersistence READ windowStatePersistence CONSTANT)
    // T222: the human business-logic map. `mapDocPath` is the repo-relative
    // path (or `DF_SETTINGS_MAP_DOC`); `mapDocUrl` is a local file URL when the
    // document resolves, so the Help menu/debug view can open it.
    Q_PROPERTY(QString mapDocPath READ mapDocPath CONSTANT)
    Q_PROPERTY(QUrl mapDocUrl READ mapDocUrl CONSTANT)
    // T222: `DF_SETTINGS_DEBUG=1` opens the pane/row owner debug view on
    // launch. The Help menu opens the same view interactively.
    Q_PROPERTY(bool debugView READ debugView CONSTANT)
    // The explicit glass rollout override (`legacy`/`replacement`) read from
    // the same `DRAGONFRUIT_GLASS_ROLLOUT` switch the shell and compositor
    // use. Empty means "use the shipped token default"; the QML shell then
    // resolves with that look, so it matches the rendered menus.
    Q_PROPERTY(QString glassRollout READ glassRollout CONSTANT)
    // T206: whether the app window has a client-alpha composition path. A
    // capture/diagnostic can force the opaque fallback with
    // `DF_APP_CHROME_SOLID=1`; the normal Wayland process is supported.
    Q_PROPERTY(bool appChromeSupported READ appChromeSupported CONSTANT)
    // Temporary diagnostic: whether `DF_SETTINGS_TRACE` is set, so QML can
    // emit `DFTRACE` lines for the pane-switch timing trace.
    Q_PROPERTY(bool trace READ trace CONSTANT)

public:
    explicit SettingsBridge(QObject *parent = nullptr);
    ~SettingsBridge() override;

    QQmlPropertyMap *valuesMap() const { return m_valuesMap; }
    bool available() const;
    QQmlPropertyMap *writeErrorsMap() const { return m_writeErrorsMap; }
    QQmlPropertyMap *writeStatesMap() const { return m_writeStatesMap; }
    QVariantMap values() const;
    QVariantMap writeErrors() const;
    QVariantMap writeStates() const;
    QString accountsError() const { return m_accountsError; }
    QString biometricsWriteOperation() const { return m_biometricsWriteOperation; }
    QString biometricsWriteState() const { return m_biometricsWriteState; }
    QString biometricsWriteError() const { return m_biometricsWriteError; }
    QString accountsWriteOperation() const { return m_accountsWriteOperation; }
    QString accountsWriteState() const { return m_accountsWriteState; }
    QString accountsWriteError() const { return m_accountsWriteError; }
    QVariantList wallpaperPresets() const;
    QVariantList providerItems() const;
    QVariantList defaultBrowserChoices() const;
    QString defaultBrowser() const;
    QString defaultBrowserError() const;
    bool defaultBrowserLoading() const;
    QVariantList thirdPartyMenuBarItems() const { return m_thirdPartyMenuBarItems; }
    bool thirdPartyMenuBarAvailable() const { return m_thirdPartyMenuBarAvailable; }
    QVariantMap bluetooth() const;
    bool bluetoothAvailable() const;
    QString bluetoothWriteOperation() const { return m_bluetoothWriteOperation; }
    QString bluetoothWriteState() const { return m_bluetoothWriteState; }
    QString bluetoothWriteError() const { return m_bluetoothWriteError; }
    QVariantMap storage() const;
    bool storageAvailable() const;
    QVariantMap sound() const;
    bool soundAvailable() const;
    QVariantMap input() const;
    bool inputAvailable() const;
    QVariantMap controllerTest() const;
    QVariantMap battery() const;
    bool batteryAvailable() const;
    QVariantMap notifications() const;
    bool notificationsAvailable() const;
    QVariantMap updates() const;
    bool updatesAvailable() const;
    QVariantMap general() const;
    bool generalAvailable() const;
    QString generalWriteOperation() const { return m_generalWriteOperation; }
    QString generalWriteState() const { return m_generalWriteState; }
    QString generalWriteError() const { return m_generalWriteError; }
    QVariantMap accounts() const;
    bool accountsAvailable() const;
    QVariantMap printers() const;
    bool printersAvailable() const;
    QVariantMap printerDiscovery() const;
    QVariantMap printerOptions() const;
    QVariantMap privacy() const;
    bool privacyAvailable() const;
    QVariantMap security() const;
    bool securityAvailable() const;
    QVariantMap biometrics() const;
    bool biometricsAvailable() const;
    QVariantMap onlineAccounts() const;
    bool onlineAccountsAvailable() const;
    QVariantMap accessibility() const;
    bool accessibilityAvailable() const;
    bool accessibilityScreenReaderAvailable() const;
    bool accessibilitySpeechAvailable() const;
    bool accessibilityOnScreenKeyboardAvailable() const;
    QString accessibilityLastDelegate() const;
    QVariantMap vpn() const;
    bool vpnAvailable() const;
    QVariantMap wifi() const;
    bool wifiAvailable() const;
    QString wifiWriteOperation() const { return m_wifiWriteOperation; }
    QString wifiWriteState() const { return m_wifiWriteState; }
    QString wifiWriteError() const { return m_wifiWriteError; }
    QVariantMap network() const;
    bool networkAvailable() const;
    QString networkWriteOperation() const { return m_networkWriteOperation; }
    QString networkWriteState() const { return m_networkWriteState; }
    QString networkWriteError() const { return m_networkWriteError; }
    QString vpnWriteOperation() const { return m_vpnWriteOperation; }
    QString vpnWriteState() const { return m_vpnWriteState; }
    QString vpnWriteError() const { return m_vpnWriteError; }
    int adapterClientCount() const;
    QString providerStatus() const;
    QString providerDefault() const;
    QString wallpaperBuiltinDefault() const;
    QVariantList wallpaperBuiltinAlternatives() const;
    int providerPreloadCount() const;
    bool wallpaperChooserAvailable() const;
    QString startPane() const;
    QString startDeepLink() const;
    bool windowStatePersistence() const;
    QString mapDocPath() const;
    QUrl mapDocUrl() const;
    bool debugView() const;
    QString glassRollout() const;
    bool appChromeSupported() const;
    bool trace() const;

    // Read one key (with an optional fallback for an unknown key).
    Q_INVOKABLE QVariant value(const QString &key, const QVariant &fallback = {}) const;
    // T224: the one write path. Validate first, apply optimistically, then
    // confirm or revert with a message. Returns false when the value was
    // rejected before any change. `set` is the compatibility alias panes have
    // used since T-09; it now routes through the same path.
    Q_INVOKABLE bool apply(const QString &key, const QVariant &value);
    Q_INVOKABLE bool set(const QString &key, const QVariant &value);
    // QML passes a JS array as a string list through this typed slot; the
    // generic `set` sees a single string for a list key (T227).
    Q_INVOKABLE bool setList(const QString &key, const QStringList &value);
    // The schema validation message for a proposed value (empty when valid).
    Q_INVOKABLE QString validationError(const QString &key,
                                        const QVariant &value) const;
    // The last failure/state for one key, and the schema metadata the row can
    // use to configure a bounded control.
    Q_INVOKABLE QString errorFor(const QString &key) const;
    Q_INVOKABLE QString stateFor(const QString &key) const;
    Q_INVOKABLE QVariantMap schemaFor(const QString &key) const;
    // T227: the Displays advanced capability table. Supported rows write the
    // real `display.*` keys; unsupported rows carry the OPEN reason.
    Q_INVOKABLE QVariantList displayAdvancedRows() const;
    // T319: the host exposes a panel backlight. The Displays pane shows its
    // absence note when false. Live reads `/sys/class/backlight`;
    // `DF_BACKLIGHT_FIXTURE=present|absent` makes tests deterministic.
    Q_INVOKABLE bool displayBacklightAvailable() const;
    // T273: the real outputs the Displays pane lists. Live reads
    // `QGuiApplication::screens()`; `DF_DISPLAY_FIXTURE` selects a
    // deterministic one/two-output (or absent) list for tests and captures.
    // Each entry is `{id, name, width, height, refreshRate, primary,
    // manufacturer, model, projector}`; `width`/`height` are logical pixels.
    // `projector` is the host's TV/projector signal: the live path reads the
    // screen name/model/manufacturer (the only EDID-derived text Qt exposes),
    // never a hardcoded output list.
    Q_PROPERTY(int displayOutputsRevision READ displayOutputsRevision
                   NOTIFY displayOutputsChanged)
    Q_INVOKABLE QVariantList displayOutputs() const;
    int displayOutputsRevision() const { return m_displayOutputsRevision; }
    // Test/capture seam (`DF_DISPLAY_FIXTURE` only): `single`, `dual`,
    // `projector`, or `absent`. A no-op without the fixture environment.
    Q_INVOKABLE void setDisplayFixture(const QString &mode);
    // T274: the icon/widget-style provider. The list is the host's installed
    // icon themes (a directory with an `index.theme`), with `Default` first.
    // `DF_ICON_THEME_FIXTURE=present|none` makes it deterministic for tests
    // and captures. The pane disables the row when the provider is absent.
    Q_PROPERTY(int iconThemesRevision READ iconThemesRevision
                   NOTIFY iconThemesChanged)
    Q_INVOKABLE QVariantList iconThemes() const;
    // The four macOS style names, each mapped to an installed theme when the
    // provider has a match. `enabled` is false with a `reason` otherwise.
    Q_INVOKABLE QVariantList iconStyleOptions() const;
    // The host's cursor themes (a directory with a `cursors/` subdirectory).
    Q_INVOKABLE QVariantList cursorThemes() const;
    // The theme the QIcon provider currently resolves to (explicit
    // `appearance.iconTheme` wins over the `appearance.iconStyle` mapping).
    Q_INVOKABLE QString effectiveIconTheme() const;
    int iconThemesRevision() const { return m_iconThemesRevision; }
    // Test/capture seams: `present` or `none`, only when the matching
    // environment variable is set.
    Q_INVOKABLE void setIconThemeFixture(const QString &mode);
    Q_INVOKABLE void setCursorThemeFixture(const QString &mode);
    // Test/capture seam (`DF_SETTINGS_FIXTURE` only): make the next write of
    // `key` apply optimistically and then fail. A no-op on the live client.
    Q_INVOKABLE void failNextWrite(const QString &key, const QString &message);
    // Test/capture seam (`DF_SETTINGS_FIXTURE` only): pretend settingsd
    // disappeared/returned while a pane is open. A no-op on the live client.
    Q_INVOKABLE void setAvailableFixture(bool available);
    // T224 test/capture seam for the privileged-action path: `allow`, `deny`,
    // `cancel`, `wrong-password`, `unavailable`, or empty for the live
    // authority.
    Q_INVOKABLE void setAuthFixture(const QString &mode);
    // T224: ask the system authority for one privileged action. The shell's
    // polkit agent renders the prompt; the app never collects a password.
    Q_INVOKABLE QString requestAuthorization(const QString &actionId,
                                             const QString &message);
    // Re-read the daemon snapshot (`GetAll`); a no-op when no daemon is up.
    Q_INVOKABLE void refresh();
    // Reset every settings key to its schema default (T226). The daemon emits
    // one `Changed` per key; a pane sees the new values through `values`.
    Q_INVOKABLE void resetSettings();
    // Temporary diagnostics: emit one `DFTRACE <message> t=<epoch-ms>` line to
    // stderr when `DF_SETTINGS_TRACE` is set. `fprintf` (not `console.log`) so
    // it survives the app's Qt logging rules and lands in the demo log.
    Q_INVOKABLE void traceLog(const QString &message) const;
    // Every key the schema (and the defaults table) knows.
    Q_INVOKABLE QStringList keys() const;

    // T223: the `?` help target. Returns the business-logic map URL with a
    // pane anchor (`#pane-<id>`), or an empty URL when the document is not
    // installed. The Settings shell disables its help control in that case
    // rather than opening a dead link.
    Q_INVOKABLE QUrl helpUrl(const QString &topic) const;

    // Open the portal file chooser at the user's Pictures directory. A chosen
    // image is announced through `wallpaperPhotoChosen`; a cancelled or
    // unavailable chooser emits nothing (the pane stays on the current image).
    Q_INVOKABLE void chooseWallpaperPhoto();
    // T270: choose a backup destination folder. The chosen local path is
    // announced through `backupDestinationChosen`; a cancelled or unavailable
    // chooser emits nothing and the field keeps its current value.
    Q_INVOKABLE void chooseBackupDestination();

    // T-18.1b: ask the wallpaper provider for an eager catalogue load (the
    // Wallpapers pane calls this on open). A no-op when the provider is absent;
    // the catalogue arrives through `providerChanged`.
    Q_INVOKABLE void preloadWallpapers();

    // ADR 0240: force a catalogue retry (the pane's "Try Again"). The provider
    // refreshes even when its cache is fresh; a no-op when it is absent.
    Q_INVOKABLE void refreshWallpapers();

    // T276: re-read the installed browser choices and the host default. The
    // Desktop & Dock pane calls this on open; a fixture makes it deterministic.
    Q_INVOKABLE void refreshDefaultBrowsers();
    // T277: re-read the live third-party menu-bar items from app-index. The
    // Menu Bar pane calls this on open; a fixture makes it deterministic.
    Q_INVOKABLE void refreshThirdPartyMenuBarItems();
    // T276: choose a default browser. Persists `defaults.browser` and asks the
    // freedesktop default-apps delegate to apply it; a missing delegate keeps
    // the persisted choice and reports the error on the row.
    Q_INVOKABLE void setDefaultBrowser(const QString &desktopId);
    // T337 test/capture seam (`DF_SETTINGS_FIXTURE` only): `ready`,
    // `slow-index`, `slow-subprocess`, `timeout-subprocess`, or `absent`.
    // A no-op on the live app.
    Q_INVOKABLE void setDefaultBrowserFixture(const QString &mode);

    // ADR 0240: download one deferred item's full-resolution image. The reply
    // is announced through `wallpaperFetchFinished`; a successful fetch also
    // updates `providerItems` through `providerChanged`.
    Q_INVOKABLE void fetchWallpaperItem(int pageid);

    // T-15.1b: the Bluetooth pane's one seam. `refreshBluetooth` re-reads the
    // bridge host on pane open; the four writes each call the adapter once and
    // the host pushes the new view back through `bluetoothChanged`. A no-op
    // when the host is absent.
    Q_INVOKABLE void refreshBluetooth();
    Q_INVOKABLE void setBluetoothPowered(bool powered);
    Q_INVOKABLE void setBluetoothDiscovering(bool discovering);
    Q_INVOKABLE void pairBluetooth(const QString &address);
    Q_INVOKABLE void setBluetoothConnected(const QString &address, bool connected);
    Q_INVOKABLE void forgetBluetooth(const QString &address);
    Q_INVOKABLE void resetBluetoothFixture();
    Q_INVOKABLE void setBluetoothFixture(const QString &mode);
    Q_INVOKABLE void failNextBluetoothWrite(const QString &operation,
                                            const QString &message);

    // T-15.2b: the Storage pane's one seam. `refreshStorage` re-reads the
    // bridge host on pane open; the three writes each call the adapter once
    // and the host pushes the new view back through `storageChanged`. A no-op
    // when the host is absent.
    Q_INVOKABLE void refreshStorage();
    Q_INVOKABLE void mountStorage(const QString &volumePath);
    Q_INVOKABLE void unmountStorage(const QString &volumePath);
    Q_INVOKABLE void ejectStorage(const QString &drivePath);
    // T273 test/capture seam (`DF_STORAGE_FIXTURE` only): `live`, `empty`
    // (UDisks2 present, no mountable volume), or `absent`.
    Q_INVOKABLE void setStorageFixture(const QString &mode);
    Q_INVOKABLE void resetStorageFixture();

    // T-15.3b: the Sound pane's one seam. `refreshSound` re-reads the bridge
    // host on pane open; the writes each call the audio adapter once and the
    // host pushes the new view back through `soundChanged`. Device routing is
    // by PipeWire node id. A no-op when the host is absent.
    Q_INVOKABLE void refreshSound();
    Q_INVOKABLE void setSoundVolume(double volume);
    Q_INVOKABLE void setSoundMute(bool muted);
    Q_INVOKABLE void setSoundSourceVolume(double volume);
    Q_INVOKABLE void setSoundSourceMute(bool muted);
    Q_INVOKABLE void setSoundDefaultSink(int id);
    Q_INVOKABLE void setSoundDefaultSource(int id);
    Q_INVOKABLE void playTestSound();
    Q_INVOKABLE void playAlertSound();

    // T-15.4b: the Keyboard/Mouse/Trackpad pane's one inventory seam.
    // `refreshInput` re-reads the bridge host on pane open; the view is
    // read-only (libinput has no setter), so `input` carries no write. A no-op
    // when the host is absent.
    Q_INVOKABLE void refreshInput();
    Q_INVOKABLE void testController(const QString &path);
    // T280: clear the last controller test report when a dialog opens, so a
    // fresh test never shows the previous controller's outcome.
    Q_INVOKABLE void clearControllerTest();

    // T-15.6b: the Battery pane's one seam. `refreshBattery` re-reads the
    // bridge host on pane open; `setPowerProfile` selects the active profile by
    // its stable id (`power-saver`/`balanced`/`performance`) and the host
    // pushes the new view back through `batteryChanged`. A no-op when the host
    // is absent.
    Q_INVOKABLE void refreshBattery();
    Q_INVOKABLE void setPowerProfile(const QString &profile);
    // T273 test/capture seam (`DF_BATTERY_FIXTURE` only): `present`, `empty`,
    // or `absent`; a no-op on the live client.
    Q_INVOKABLE void setBatteryFixture(const QString &mode);
    Q_INVOKABLE void resetBatteryFixture();

    // T-15.7b: the Notifications/Focus panes' one seam. `refreshNotifications`
    // re-reads the bridge host on pane open; `setFocusMode` selects the policy
    // (`off`/`focus`/`dnd`) and `setFocusApp` adds or removes one app from the
    // per-app allow list, reading the current list first so a toggle changes
    // exactly one entry. A no-op when the host is absent.
    Q_INVOKABLE void refreshNotifications();
    Q_INVOKABLE void setFocusMode(const QString &mode);
    Q_INVOKABLE void setFocusApp(const QString &app, bool allowed);
    Q_INVOKABLE void setFocusProfile(const QString &id);
    Q_INVOKABLE void setFocusPerson(const QString &person, bool allowed);
    // T229a: one per-app preference write (allow/style/sound/badge). The host
    // re-reads and pushes the new view back through `notificationsChanged`.
    Q_INVOKABLE void setNotificationAppPreference(const QString &app,
                                                  const QString &style,
                                                  bool sound, bool badge,
                                                  bool allowed);

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
    // T269 test seam (`DF_UPDATES_FIXTURE` only): pretend the bridge host is
    // absent/returned, or keep the host identity and drop only the update
    // provider, while the General list is open.
    Q_INVOKABLE void setUpdatesAbsentFixture(bool absent);
    Q_INVOKABLE void setUpdatesProviderFixture(bool available);

    // T226: the General sub-panes seam. `refreshGeneral` re-reads every
    // sub-pane on open; the writes each call the host once and the host pushes
    // the new view back through `generalChanged`.
    Q_INVOKABLE void refreshGeneral();
    Q_INVOKABLE void setNtp(bool enabled);
    Q_INVOKABLE void setTimezone(const QString &timezone);
    Q_INVOKABLE void setTime(const QString &value);
    Q_INVOKABLE void setLocale(const QString &locale);
    Q_INVOKABLE void setAutostartEnabled(const QString &id, bool enabled);
    Q_INVOKABLE void addAutostart(const QString &appId);
    Q_INVOKABLE void removeAutostart(const QString &id);
    Q_INVOKABLE void setHostname(const QString &hostname);
    Q_INVOKABLE void restartSystem();
    Q_INVOKABLE void setUserUnitEnabled(const QString &unit, bool enabled);
    Q_INVOKABLE void setSharingEnabled(const QString &id, bool enabled);
    Q_INVOKABLE void runBackup(const QString &destination);
    Q_INVOKABLE void openBackupRestore();
    // T269: open an installed password provider's application. A delegate
    // request only; the Settings app never reads or stores a secret.
    Q_INVOKABLE void openPasswordManager(const QString &id);
    // The update policy keys are settingsd's; the pane forwards them to the
    // running host so automatic checks/installs act immediately.
    Q_INVOKABLE void applyUpdatePolicy(bool autoCheck, bool autoInstall);
    // Test seam (`DF_GENERAL_FIXTURE` only): restore the fixture.
    Q_INVOKABLE void resetGeneralFixture();
    // Test seam (`DF_GENERAL_FIXTURE` only): pretend the bridge host is
    // absent/returned while a pane is open.
    Q_INVOKABLE void setGeneralAvailableFixture(bool available);
    // Test seam (`DF_GENERAL_FIXTURE` only): make the next host write of one
    // operation fail with `message`, so the pane's error path is testable.
    Q_INVOKABLE void failNextGeneralWrite(const QString &operation,
                                          const QString &message);

    // T-15.11b: the Users & Groups pane's one seam. `refreshAccounts` re-reads
    // the bridge host on pane open; each write calls the host stack adapter
    // once and the host pushes the new view back through `accountsChanged`. A
    // no-op when the host is absent. Account types are stable ids (`standard` /
    // `administrator`); automatic login takes a uid (`0` clears it).
    Q_INVOKABLE void refreshAccounts();
    Q_INVOKABLE void createAccount(const QString &userName, const QString &realName,
                                   const QString &accountType, const QString &password);
    Q_INVOKABLE void setAccountPassword(int uid, const QString &password);
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
    Q_INVOKABLE void discoverPrinters();
    Q_INVOKABLE void addPrinter(const QString &name, const QString &uri,
                                const QString &driver);
    Q_INVOKABLE void loadPrinterOptions(const QString &name);
    Q_INVOKABLE void setPrinterOption(const QString &name, const QString &key,
                                      const QString &value);
    Q_INVOKABLE void removePrinter(const QString &name);
    Q_INVOKABLE void printTestPage(const QString &name);
    Q_INVOKABLE void setPrinterShared(const QString &name, bool shared);
    Q_INVOKABLE void openScanner(const QString &device);
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

    // T228: the Security half of the Privacy & Security pane.
    Q_INVOKABLE void refreshSecurity();
    Q_INVOKABLE void setSecuritySourceEnabled(const QString &id, bool enabled);
    // Test seam (`DF_PRIVACY_FIXTURE` only): hide the portal store while the
    // security host stays live.
    Q_INVOKABLE void setPrivacyAvailableFixture(bool available);
    // T279: the USBGuard accessory policy state in the privacy fixture.
    Q_INVOKABLE void setAccessoriesAvailableFixture(bool available);

    // T228: the Biometrics & Password pane. The password is handed straight to
    // the host; it is never stored or logged by the app.
    Q_INVOKABLE void refreshBiometrics();
    Q_INVOKABLE void startFingerprintEnrollment(const QString &finger);
    Q_INVOKABLE void cancelFingerprintEnrollment();
    Q_INVOKABLE void deleteFingerprint(const QString &finger);
    Q_INVOKABLE void changePassword(const QString &password);
    Q_INVOKABLE void resetBiometricsFixture();
    // T279 fixture controls: a denied polkit write, an unsupported reader, the
    // password outcome, and the staged fprintd enrollment. No-ops live.
    Q_INVOKABLE void setBiometricsDeniedFixture(bool denied);
    Q_INVOKABLE void setBiometricsReaderFixture(bool present);
    Q_INVOKABLE void setPasswordOutcomeFixture(const QString &outcome, const QString &note);
    Q_INVOKABLE void advanceFingerprintEnrollmentFixture();
    Q_INVOKABLE void setFingerprintStageFixture(const QString &stage, bool done);

    // T228: the Online Accounts pane.
    Q_INVOKABLE void refreshOnlineAccounts();
    Q_INVOKABLE void addOnlineAccount(const QString &provider, const QString &identity);
    Q_INVOKABLE void removeOnlineAccount(const QString &id);
    Q_INVOKABLE void setOnlineAccountSync(const QString &id, const QString &capability,
                                          bool enabled);
    Q_INVOKABLE void ensureOnlineAccountCredentials(const QString &id);
    Q_INVOKABLE void resetOnlineAccountsFixture();
    // T279 fixture control: mark the fixture account as needing a sign-in.
    Q_INVOKABLE void setOnlineAttentionFixture(bool needed);

    // T-15.14b: the Accessibility pane's one seam. `refreshAccessibility`
    // re-reads the bridge host on pane open; the adapter is read-only, so there
    // is no write here (the pane's `accessibility.reduceMotion` toggle goes
    // through `set`). A no-op when the host is absent.
    Q_INVOKABLE void refreshAccessibility();
    // Test seam (`DF_ACCESSIBILITY_FIXTURE` only): restore the in-process
    // fixture. A no-op on the live client.
    Q_INVOKABLE void resetAccessibilityFixture();
    // T275: the three external delegates. Each returns false when the program
    // is absent, so the pane can show a disabled action and a reason instead
    // of a fake toggle. `DF_ACCESSIBILITY_DELEGATE_FIXTURE=present|absent`
    // makes headless tests deterministic without spawning a process.
    Q_INVOKABLE bool openAccessibilityScreenReader();
    Q_INVOKABLE bool speakAccessibilityText(const QString &text);
    Q_INVOKABLE bool openAccessibilityKeyboard();

    // T-15.15b: the Network advanced (VPN) pane's one seam. `refreshVpn`
    // re-reads the bridge host on pane open; `connectVpn`/`disconnectVpn` each
    // call the adapter once, by UUID, and the host pushes the new view back
    // through `vpnChanged`. There is no settingsd key: NetworkManager owns the
    // state. A no-op when the host is absent.
    Q_INVOKABLE void refreshVpn();
    Q_INVOKABLE void connectVpn(const QString &uuid);
    Q_INVOKABLE void disconnectVpn(const QString &uuid);
    Q_INVOKABLE void addVpn(const QString &plugin, const QString &name,
                            const QString &gateway);
    Q_INVOKABLE void addVpnWireguard(const QString &name,
                                     const QString &privateKey,
                                     const QString &address,
                                     const QString &peerPublicKey,
                                     const QString &endpoint,
                                     const QString &dns, int mtu);
    Q_INVOKABLE void editVpn(const QString &uuid);
    Q_INVOKABLE void setVpnAutoconnect(const QString &uuid, bool enabled);
    // Test seam (`DF_VPN_FIXTURE` only): restore the in-process fixture. A
    // no-op on the live client.
    Q_INVOKABLE void resetVpnFixture();
    Q_INVOKABLE void setVpnFixture(const QString &mode);
    Q_INVOKABLE void failNextVpnWrite(const QString &operation,
                                      const QString &message);

    // T225: the Wi-Fi pane's seam. `refreshWifi` re-reads the bridge host on
    // open; each write calls the adapter once and the host pushes the new view
    // back through `wifiChanged`. A no-op when the host is absent.
    Q_INVOKABLE void refreshWifi();
    Q_INVOKABLE void setWifiEnabled(bool enabled);
    Q_INVOKABLE void joinWifi(const QString &ssid, const QString &secret);
    Q_INVOKABLE void joinHiddenWifi(const QString &ssid, const QString &secret);
    Q_INVOKABLE void forgetWifi(const QString &uuid);
    Q_INVOKABLE void setWifiAutoconnect(const QString &uuid, bool enabled);
    Q_INVOKABLE void scanWifi();
    Q_INVOKABLE void disconnectWifi(const QString &uuid);
    Q_INVOKABLE void startWifiHotspot(const QString &ssid, const QString &secret);
    Q_INVOKABLE void stopWifiHotspot(const QString &uuid);
    Q_INVOKABLE void resetWifiFixture();
    Q_INVOKABLE void setWifiFixture(const QString &mode);
    Q_INVOKABLE void failNextWifiWrite(const QString &operation,
                                       const QString &message);

    // T225: the Network pane's seam. Service order is NetworkManager's
    // autoconnect priority; `moveNetworkService` moves one position. Firewall
    // writes go to the same bridge host's Firewall interface.
    Q_INVOKABLE void refreshNetwork();
    Q_INVOKABLE void activateNetworkService(const QString &uuid);
    Q_INVOKABLE void deactivateNetworkService(const QString &uuid);
    Q_INVOKABLE void moveNetworkService(const QString &uuid, int delta);
    Q_INVOKABLE void addNetworkService(const QString &name, const QString &kind,
                                       const QString &plugin,
                                       const QString &gateway,
                                       const QString &interfaceName);
    Q_INVOKABLE void forgetNetworkService(const QString &uuid);
    Q_INVOKABLE void setNetworkAutoconnect(const QString &uuid, bool enabled);
    Q_INVOKABLE void setFirewallEnabled(bool enabled);
    Q_INVOKABLE void openFirewallPort(int port, const QString &protocol);
    Q_INVOKABLE void closeFirewallPort(int port, const QString &protocol);
    Q_INVOKABLE void openFirewallService(const QString &service);
    Q_INVOKABLE void closeFirewallService(const QString &service);
    Q_INVOKABLE void resetNetworkFixture();
    Q_INVOKABLE void setNetworkFixture(const QString &mode);
    Q_INVOKABLE void failNextNetworkWrite(const QString &operation,
                                          const QString &message);

    // T-18.2 / ADR 0240 test seam: with `DF_WALLPAPER_FIXTURE` set, seed the
    // provider lifecycle. `ready`/`categories`/`many`/`deferred`/
    // `deferred-fail` load deterministic catalogues with generated local
    // images; `fetching`/`offline`/`error` leave the catalogue empty. Without
    // the environment variable this is a no-op, so production behavior is
    // unchanged.
    Q_INVOKABLE void setWallpaperFixture(const QString &status);

    // `file:///path/to/a.png` / `file://host/path` -> a local path. Pure, so
    // the URI contract is unit-testable.
    static QString localPathFromUri(const QString &uri);
    // A local path -> a properly escaped `file://` URL (T278). The Wallpaper
    // pane used to concatenate `"file://" + path`, which broke on spaces and
    // `#`; every pane URL now goes through QUrl.
    Q_INVOKABLE QString fileUrl(const QString &path) const;
    // Strip tags and decode the handful of HTML entities Wikimedia's
    // `extmetadata` uses, so an `Artist` value is never rendered as markup.
    static QString sanitizeHtmlText(const QString &value);

signals:
    void valuesChanged();
    void availableChanged(bool available);
    // T224: any key's write state/error changed.
    void writeStatesChanged();
    void writeFinished(const QString &key, const QVariant &value, bool ok,
                       const QString &message);
    // T224: one privileged action resolved. `error` is empty on success.
    void authorizationFinished(const QString &requestId, bool authorized,
                               const QString &error);
    void accountsErrorChanged();
    // One key really changed (local write or daemon signal). `values` also
    // changes; listen to this only when a key's identity matters.
    void changed(const QString &key, const QVariant &value);
    void wallpaperChooserAvailableChanged();
    // T276: the default-browser choices/value/error changed.
    void defaultBrowserChanged();
    void thirdPartyMenuBarItemsChanged();
    // T270: the chosen backup destination's local path.
    void backupDestinationChosen(const QString &path);
    // T270: the last General host write's operation/state/message changed.
    void generalWriteChanged();
    // The chosen photo's local path, ready for `wallpaper.source`.
    void wallpaperPhotoChosen(const QString &path);
    // Any provider property (catalogue, status, defaults) changed. Panes bind
    // the `provider*` properties and re-read on this.
    void providerChanged();
    // ADR 0240: one deferred full-resolution fetch finished. `ok` false carries
    // the provider/bus error; the pane keeps the previous wallpaper.
    void wallpaperFetchFinished(int pageid, bool ok, const QString &message);
    // The Bluetooth view or availability changed (T-15.1b).
    void bluetoothChanged();
    // The storage view or availability changed (T-15.2b).
    void storageChanged();
    // The audio view or availability changed (T-15.3b).
    void soundChanged();
    // The input inventory or availability changed (T-15.4b).
    void inputChanged();
    void controllerTestChanged();
    // The battery / power-profiles view or availability changed (T-15.6b).
    void batteryChanged();
    // The real display-output list changed (T273).
    void displayOutputsChanged();
    // The icon/cursor theme provider inventory changed (T274).
    void iconThemesChanged();
    // The Notifications/Focus view or availability changed (T-15.7b).
    void notificationsChanged();
    // The General/About/Updates view or availability changed (T-15.10b).
    void updatesChanged();
    // The General sub-panes view or availability changed (T226).
    void generalChanged();
    // The Users and Groups view or availability changed (T-15.11b).
    void accountsChanged();
    // The Printers and Scanners view or availability changed (T-15.12b).
    void printersChanged();
    void printerDiscoveryChanged();
    void printerOptionsChanged();
    // The Privacy and Security view or availability changed (T-15.13b).
    void privacyChanged();
    void securityChanged();
    void biometricsChanged();
    void onlineAccountsChanged();
    // T279: the host's write report for the account/security panes.
    void biometricsWriteChanged();
    void accountsWriteChanged();
    // The Accessibility view or availability changed (T-15.14b).
    void accessibilityChanged();
    // T275: a delegate launch attempt completed (`ok` false when the program
    // is absent or the launch failed).
    void accessibilityDelegateChanged();
    // The Network advanced (VPN) view or availability changed (T-15.15b).
    void vpnChanged();
    // The Wi-Fi view or availability changed (T225).
    void wifiChanged();
    // T271: the last Wi-Fi host write's operation/state/message changed.
    void wifiWriteChanged();
    // T271: the last Bluetooth host write's operation/state/message changed.
    void bluetoothWriteChanged();
    // The Network/firewall view or availability changed (T225).
    void networkChanged();
    // T272: the last Network/VPN host write's operation/state/message changed.
    void networkWriteChanged();
    void vpnWriteChanged();
    // T268: one more adapter client was created on first use.
    void adapterClientCountChanged();

private:
    // T273: follow one platform screen's geometry/refresh changes so the
    // Displays pane's real-output list stays current.
    void watchDisplayScreen(QScreen *screen);
    // T268: adapter clients are created on first use (the pane that owns them),
    // not all fifteen in the constructor. Each ensure* wires the client's
    // signals exactly once; absence stays a normal empty view.
    BluetoothClient *ensureBluetooth();
    // T271: publish the busy state before one Wi-Fi/Bluetooth write.
    void beginWifiWrite(const QString &operation);
    void beginBluetoothWrite(const QString &operation);
    // T272: publish the busy state before one Network/VPN write.
    void beginNetworkWrite(const QString &operation);
    void beginVpnWrite(const QString &operation);
    StorageClient *ensureStorage();
    SoundClient *ensureSound();
    InputClient *ensureInput();
    BatteryClient *ensureBattery();
    NotificationsClient *ensureNotifications();
    UpdatesClient *ensureUpdates();
    GeneralClient *ensureGeneral();
    AccountsClient *ensureAccounts();
    PrintersClient *ensurePrinters();
    PrivacyClient *ensurePrivacy();
    AccessibilityClient *ensureAccessibility();
    VpnClient *ensureVpn();
    WifiClient *ensureWifi();
    NetworkClient *ensureNetwork();
    // T224: authorize one privileged account write, then run it. A denial
    // leaves the view untouched and publishes `accountsError`.
    void authorizeAccountAction(const QString &actionId, const QString &message,
                                const std::function<void()> &action);
    void setAccountsError(const QString &message);
    void publishValue(const QString &key, const QVariant &value);
    void refreshWriteState(const QString &key);
    void noteAdapterCreated();
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
    // ADR 0240: the deterministic fixture catalogue and its generated local
    // images, so the pane's rows and apply/download states are testable with
    // no bus and no network.
    void buildWallpaperFixtureImages();
    QString wallpaperFixtureJson(const QString &mode) const;
    QString wallpaperFixturePath(int pageid, bool thumb) const;

    // T337: the default-browser read is two asynchronous sources. app-index is
    // queried with a `QDBusPendingCallWatcher`; the host default-apps delegate
    // runs through `QProcess` signals with a timeout. Neither path blocks the
    // GUI thread, and a second refresh coalesces into one follow-up read.
    void queryAppIndexBrowsers(quint64 generation);
    void queryDefaultBrowserHost();
    void startDefaultBrowserProcess(const QString &program,
                                    const QStringList &arguments, int timeoutMs,
                                    const QString &kind);
    void finishDefaultBrowserProcess(QProcess *process, bool ok,
                                     const QString &output);
    void discardDefaultBrowserProcess(QProcess *process);
    void finishDefaultBrowserRefresh(quint64 generation);
    void resolveDefaultBrowser();
    void applyDefaultBrowserFixture(const QString &mode);
    void startDefaultBrowserFixtureRefresh(quint64 generation);

private slots:
    void onPortalResponse(uint response, const QVariantMap &results);
    // T-18.1b: the provider's contract signal and its standard property
    // notification. Both keep the pane's catalogue and defaults in step.
    void onProviderItemsChanged();
    void onProviderPropertiesChanged(const QString &interface, const QVariantMap &changed,
                                     const QStringList &invalidated);

private:
    void beginGeneralWrite(const QString &operation);
    SettingsClient *m_client = nullptr;
    // T268: per-key reactive maps. `values` replaces the full-map property;
    // the write-state maps replace the full-map error/state copies.
    QQmlPropertyMap *m_valuesMap = nullptr;
    QQmlPropertyMap *m_writeErrorsMap = nullptr;
    QQmlPropertyMap *m_writeStatesMap = nullptr;
    SettingsAuthorizer *m_authorizer = nullptr;
    QHash<QString, std::function<void()>> m_pendingAuthActions;
    int m_authRequestCounter = 0;
    QString m_accountsError;
    QString m_biometricsWriteOperation;
    QString m_biometricsWriteState = QStringLiteral("idle");
    QString m_biometricsWriteError;
    QString m_accountsWriteOperation;
    QString m_accountsWriteState = QStringLiteral("idle");
    QString m_accountsWriteError;
    QVariantList m_presets;
    bool m_chooserAvailable = false;
    QString m_requestPath;
    // T270: which file chooser opened (`wallpaper` or `backup`), so the one
    // portal response handler routes the chosen path to the right signal.
    QString m_chooserPurpose;
    QString m_generalWriteOperation;
    QString m_generalWriteState = QStringLiteral("idle");
    QString m_generalWriteError;
    // T271: the last Wi-Fi and Bluetooth write state.
    QString m_wifiWriteOperation;
    QString m_wifiWriteState = QStringLiteral("idle");
    QString m_wifiWriteError;
    QString m_networkWriteOperation;
    QString m_networkWriteState = QStringLiteral("idle");
    QString m_networkWriteError;
    QString m_vpnWriteOperation;
    QString m_vpnWriteState = QStringLiteral("idle");
    QString m_vpnWriteError;
    QString m_bluetoothWriteOperation;
    QString m_bluetoothWriteState = QStringLiteral("idle");
    QString m_bluetoothWriteError;
    // T-18.1b: the provider catalogue and its resolved defaults. `m_providerBuiltin`
    // is the provider's own `BuiltinDefaultSource`; the property falls back to
    // the shared shipped-asset resolver so it is non-empty even when absent.
    // T276: the default-browser choices/value/error.
    QVariantList m_defaultBrowserChoices;
    QString m_defaultBrowser;
    QString m_defaultBrowserError;
    bool m_defaultBrowserLoaded = false;
    // T337: the async read state. `m_defaultBrowserFixtureMode` is empty on the
    // live app; `DF_SETTINGS_FIXTURE` selects `ready` for deterministic tests.
    QString m_defaultBrowserFixtureMode;
    bool m_defaultBrowserLoading = false;
    bool m_defaultBrowserEnumeratePending = false;
    bool m_defaultBrowserProcessPending = false;
    bool m_defaultBrowserRefreshQueued = false;
    quint64 m_defaultBrowserGeneration = 0;
    QString m_defaultBrowserHostDefault;
    QProcess *m_defaultBrowserProcess = nullptr;
    QString m_defaultBrowserProcessKind;
    // T277: the third-party StatusNotifier items the Menu Bar pane lists.
    QVariantList m_thirdPartyMenuBarItems;
    bool m_thirdPartyMenuBarAvailable = false;
    QVariantList m_providerItems;
    QString m_providerStatus;
    QString m_providerDefault;
    QString m_providerBuiltin;
    int m_providerPreloads = 0;
    // T268: the provider read is asynchronous and coalesced. A second request
    // while a `GetAll` is in flight is folded into one follow-up read.
    bool m_providerRefreshPending = false;
    bool m_providerRefreshQueued = false;
    // T-18.2: true when `DF_WALLPAPER_FIXTURE` selected the in-process
    // catalogue, so `Preload` and fixture seeding stay deterministic.
    bool m_wallpaperFixture = false;
    QString m_wallpaperFixtureMode;
    QString m_wallpaperFixtureDir;
    QDBusServiceWatcher *m_providerWatcher = nullptr;
    // T-15.1b: the Bluetooth seam (`DF_BLUETOOTH_FIXTURE` selects the mock).
    BluetoothClient *m_bluetooth = nullptr;
    // T-15.2b: the storage seam (`DF_STORAGE_FIXTURE` selects the mock).
    StorageClient *m_storage = nullptr;
    // T-15.3b: the Sound seam (`DF_SOUND_FIXTURE` selects the mock).
    SoundClient *m_sound = nullptr;
    // T-15.4b: the input inventory seam (`DF_INPUT_FIXTURE` selects the mock).
    InputClient *m_input = nullptr;
    QVariantMap m_controllerTest;
    // T-15.6b: the battery / power-profiles seam (`DF_BATTERY_FIXTURE` selects
    // the mock).
    BatteryClient *m_battery = nullptr;
    // T273: the Displays output fixture mode (`DF_DISPLAY_FIXTURE` only).
    QString m_displayFixtureMode;
    int m_displayOutputsRevision = 0;
    // T274: the icon/cursor theme provider fixture modes and revision.
    QString m_iconThemeFixtureMode;
    QString m_cursorThemeFixtureMode;
    int m_iconThemesRevision = 0;
    QStringList installedIconThemes() const;
    QStringList installedCursorThemes() const;
    QString iconThemeForStyle(const QString &style,
                              const QStringList &themes) const;
    void applyIconTheme();
    // T-15.7b: the Notifications/Focus seam (`DF_NOTIFICATIONS_FIXTURE`
    // selects the mock).
    NotificationsClient *m_notifications = nullptr;
    // T-15.10b: the General/About/Updates seam (`DF_UPDATES_FIXTURE` selects
    // the mock).
    UpdatesClient *m_updates = nullptr;
    // T226: the General sub-panes seam (`DF_GENERAL_FIXTURE` selects the mock).
    GeneralClient *m_general = nullptr;
    // T-15.11b: the Users & Groups seam (`DF_ACCOUNTS_FIXTURE` selects the
    // mock).
    AccountsClient *m_accounts = nullptr;
    // T-15.12b: the Printers & Scanners seam (`DF_PRINTERS_FIXTURE` selects the
    // mock).
    PrintersClient *m_printers = nullptr;
    QVariantMap m_printerDiscovery;
    QVariantMap m_printerOptions;
    // T-15.13b: the Privacy & Security seam (`DF_PRIVACY_FIXTURE` selects the
    // mock).
    PrivacyClient *m_privacy = nullptr;
    // T-15.14b: the Accessibility seam (`DF_ACCESSIBILITY_FIXTURE` selects the
    // mock).
    AccessibilityClient *m_accessibility = nullptr;
    // T-15.15b: the Network advanced (VPN) seam (`DF_VPN_FIXTURE` selects the
    // mock).
    VpnClient *m_vpn = nullptr;
    WifiClient *m_wifi = nullptr;
    NetworkClient *m_network = nullptr;
};
