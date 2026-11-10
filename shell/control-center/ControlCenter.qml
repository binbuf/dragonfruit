// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Control Center panel (T-11.3a/T-11.3b, measured grid T235): one
// quick-settings surface opened from the menu-bar Control Center item (or its
// keyboard shortcut). It is rendered into its own top-right `overlay` layer
// surface by `ShellController`; this item is the panel's whole scene.
//
// The panel is a pure view with one test seam: `tiles` is the normalized
// model a headless test can assert without touching the scene, and every
// gesture is raised as a signal the shell forwards to the owning adapter or
// owner (`services/system-status`, the notification service, settingsd). The
// QML never talks to a daemon itself.
//
// T235 replaces the curated full-width tile list with the measured 4-column
// grid: 64 pt cells, 12 pt gaps, and 1×1 / 2×1 / 2×2 / 4×1 module spans. The
// same registry drives the Edit Controls catalogue, the three-page model, the
// span overrides and the menu-bar-only path. Expanded modules grow the outer
// panel instead of detaching a popover.
Item {
    id: root

    // ── Service views (unchanged public properties) ─────────────────────
    property var wifi: ({})
    property var audio: ({})
    property var bluetooth: ({})
    property var storage: ({})
    property var input: ({})
    property var missionControl: ({})
    property var lockPolicy: ({})
    property var menuBar: ({})
    property var battery: ({})
    property var updates: ({})
    property var accounts: ({})
    property var printers: ({})
    property var privacy: ({})
    property var accessibility: ({})
    property var vpn: ({})
    property real brightness: 1.0
    property var focusPolicy: ({})
    property bool dark: false
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0

    // Service write seams set by ShellController (unchanged T-15/T-225).
    property bool wifiWritable: false
    property bool bluetoothWritable: false
    property bool storageWritable: false

    // ── Customization (settingsd keys, T227/T235) ───────────────────────
    property var quickSettingsModules: []
    property var quickSettingsMenuBarOnly: []
    property var quickSettingsPage2: []
    property var quickSettingsPage3: []
    property int quickSettingsPageCount: 1
    property int quickSettingsActivePage: 0
    property bool quickSettingsPerDisplay: false
    property var quickSettingsSpans: []

    // The shell flips this while the 736×900 Edit Controls surface is open.
    property bool editMode: false

    readonly property var registry: ControlCenterRegistry
    readonly property var defaultModuleOrder: ControlCenterRegistry.defaultModules
    readonly property var legacyModuleOrder: ControlCenterRegistry.legacyModules
    readonly property var spanMap: ControlCenterRegistry.parseSpans(root.quickSettingsSpans)
    readonly property int pageCount: Math.max(1, Math.min(3,
        root.quickSettingsPageCount > 0 ? root.quickSettingsPageCount : 1))
    readonly property int activePage: Math.max(0, Math.min(root.pageCount - 1,
        root.quickSettingsActivePage))
    readonly property bool reduceMotion: Theme.reducedMotion

    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    // ── Page/module model ────────────────────────────────────────────────
    function pageModuleList(page) {
        if (page === 1)
            return root.quickSettingsPage2 ? Array.from(root.quickSettingsPage2) : [];
        if (page === 2)
            return root.quickSettingsPage3 ? Array.from(root.quickSettingsPage3) : [];
        if (root.quickSettingsModules && root.quickSettingsModules.length > 0)
            return Array.from(root.quickSettingsModules);
        return root.defaultModuleOrder.slice();
    }

    readonly property var moduleOrder: root.pageModuleList(root.activePage)
    readonly property var allPanelModules: {
        var out = [];
        for (var page = 0; page < root.pageCount; ++page) {
            var list = root.pageModuleList(page);
            for (var i = 0; i < list.length; ++i)
                out.push(list[i]);
        }
        return out;
    }

    function moduleVisible(id) {
        if (root.quickSettingsMenuBarOnly.indexOf(id) >= 0)
            return false;
        return root.moduleOrder.indexOf(id) >= 0;
    }

    function moduleRemovedEverywhere(id) {
        return root.allPanelModules.indexOf(id) < 0;
    }

    // ── Live state projections (the old tile labels) ─────────────────────
    readonly property bool wifiAvailable: root.wifi.state === "available"
    readonly property bool wifiOn: root.wifi.radioEnabled === true
    readonly property var wifiNetworks: root.wifi.networks !== undefined
        ? root.wifi.networks : []
    readonly property string wifiLabel: {
        if (!root.wifiAvailable)
            return qsTr("Unavailable");
        if (root.wifi.label !== undefined && root.wifi.label !== "")
            return root.wifi.label;
        return root.wifiOn ? qsTr("On") : qsTr("Off");
    }

    readonly property bool audioAvailable: root.audio.state === "available"
    readonly property bool muted: root.audio.muted === true
    property real volume: root.audio.volume !== undefined ? root.audio.volume : 0.0
    readonly property var audioOutputs: root.audio.sinks !== undefined
        ? root.audio.sinks : []
    readonly property string audioOutputName: {
        for (var i = 0; i < root.audioOutputs.length; ++i) {
            if (root.audioOutputs[i].default === true)
                return root.audioOutputs[i].description !== undefined
                    ? root.audioOutputs[i].description : "";
        }
        return "";
    }

    readonly property bool bluetoothAvailable: root.bluetooth.state === "available"
    readonly property bool bluetoothPresent: root.bluetooth.present === true
    readonly property bool bluetoothVisible: root.bluetoothAvailable
        && root.bluetoothPresent
    readonly property bool bluetoothOn: root.bluetooth.powered === true
    readonly property string bluetoothLabel: {
        if (!root.bluetoothVisible)
            return qsTr("Unavailable");
        if (root.bluetooth.discovering === true)
            return qsTr("Discovering\u2026");
        if (root.bluetooth.label !== undefined && root.bluetooth.label !== "")
            return root.bluetooth.label;
        return root.bluetoothOn ? qsTr("On") : qsTr("Off");
    }
    readonly property var bluetoothDevices: root.bluetooth.knownDevices !== undefined
        ? root.bluetooth.knownDevices : []

    readonly property bool storageAvailable: root.storage.state === "available"
    readonly property bool storagePresent: root.storage.present === true
    readonly property bool storageVisible: root.storageAvailable
        && root.storagePresent
    readonly property var storageVolumes: root.storage.volumes !== undefined
        ? root.storage.volumes : []
    readonly property int storageMountedCount:
        root.storage.mountedCount !== undefined ? root.storage.mountedCount : 0
    readonly property string storageLabel: {
        if (!root.storageVisible)
            return qsTr("Unavailable");
        if (root.storage.label !== undefined && root.storage.label !== "")
            return root.storage.label;
        return root.storageMountedCount > 0
            ? qsTr("%1 mounted").arg(root.storageMountedCount) : qsTr("No volumes");
    }

    readonly property bool inputAvailable: root.input.state === "available"
    readonly property bool inputPresent: root.input.present === true
    readonly property bool inputVisible: root.inputAvailable && root.inputPresent
    readonly property string inputLabel: {
        if (!root.inputVisible)
            return qsTr("Unavailable");
        if (root.input.label !== undefined && root.input.label !== "")
            return root.input.label;
        return qsTr("No input devices");
    }

    readonly property bool missionControlVisible:
        root.missionControl.state === "available"
    readonly property string missionControlLabel: {
        if (!root.missionControlVisible)
            return qsTr("Unavailable");
        if (root.missionControl.label !== undefined
                && root.missionControl.label !== "")
            return root.missionControl.label;
        return qsTr("No trigger");
    }

    readonly property bool lockScreenVisible:
        root.lockPolicy.state === "available"
    readonly property string lockScreenLabel: {
        if (!root.lockScreenVisible)
            return qsTr("Unavailable");
        if (root.lockPolicy.label !== undefined
                && root.lockPolicy.label !== "")
            return root.lockPolicy.label;
        return qsTr("No password required");
    }

    readonly property bool menuBarVisible:
        root.menuBar.state === "available"
    readonly property string menuBarLabel: {
        if (!root.menuBarVisible)
            return qsTr("Unavailable");
        if (root.menuBar.label !== undefined
                && root.menuBar.label !== "")
            return root.menuBar.label;
        return qsTr("In Full Screen Only");
    }

    readonly property bool batteryAvailable: root.battery.state === "available"
    readonly property bool batteryPresent: root.battery.present === true
    readonly property bool batteryProfiles: root.battery.profilesAvailable === true
    readonly property bool batteryVisible: root.batteryAvailable
        && (root.batteryPresent || root.batteryProfiles)
    readonly property int batteryPercent:
        root.battery.percent !== undefined ? Number(root.battery.percent) : 0
    readonly property string batteryProfileLabel:
        root.battery.profileLabel !== undefined
            ? String(root.battery.profileLabel) : ""
    readonly property var batteryProfilesList: root.battery.profiles !== undefined
        ? root.battery.profiles : []
    readonly property string batteryGlyph: {
        for (var i = 0; i < root.batteryProfilesList.length; ++i) {
            if (root.batteryProfilesList[i].active === true
                    && root.batteryProfilesList[i].glyph !== undefined)
                return root.batteryProfilesList[i].glyph;
        }
        return "battery";
    }
    readonly property string batteryLabel: {
        if (!root.batteryVisible)
            return qsTr("Unavailable");
        var parts = [];
        if (root.batteryPresent)
            parts.push(qsTr("%1%").arg(root.batteryPercent));
        if (root.batteryProfiles && root.batteryProfileLabel.length > 0)
            parts.push(root.batteryProfileLabel);
        return parts.length > 0 ? parts.join(" \u00b7 ") : qsTr("Battery");
    }

    readonly property bool updatesAvailable: root.updates.state === "available"
    readonly property bool updatesProvider: root.updates.updatesAvailable === true
    readonly property bool updatesBusy: root.updates.busy === true
    readonly property bool updatesRebootRequired: root.updates.rebootRequired === true
    readonly property int updatesCount: root.updates.updateCount !== undefined
        ? Number(root.updates.updateCount) : 0
    readonly property string updatesGlyph:
        root.updates.glyph !== undefined ? String(root.updates.glyph)
                                         : "software-update"
    readonly property string updatesLabel: {
        if (!root.updatesAvailable)
            return qsTr("Unavailable");
        if (root.updates.label !== undefined && root.updates.label !== "")
            return root.updates.label;
        return root.updatesProvider ? qsTr("Up to Date")
                                    : qsTr("Software Update Unavailable");
    }
    readonly property string updatesAction: {
        if (!root.updatesProvider)
            return qsTr("Unavailable");
        if (root.updatesRebootRequired)
            return qsTr("Restart");
        if (root.updatesCount > 0)
            return qsTr("Install");
        return qsTr("Check for Updates");
    }

    readonly property bool accountsAvailable: root.accounts.state === "available"
    readonly property int accountsHumanCount:
        root.accounts.humanCount !== undefined ? Number(root.accounts.humanCount) : 0
    readonly property int accountsAdminCount:
        root.accounts.adminCount !== undefined ? Number(root.accounts.adminCount) : 0
    readonly property string accountsGlyph:
        root.accounts.glyph !== undefined ? String(root.accounts.glyph) : "users"
    readonly property string accountsLabel: {
        if (!root.accountsAvailable)
            return qsTr("Unavailable");
        if (root.accounts.label !== undefined && root.accounts.label !== "")
            return root.accounts.label;
        return root.accountsHumanCount > 0
            ? qsTr("%1 Users").arg(root.accountsHumanCount) : qsTr("No Users");
    }

    readonly property bool printersAvailable: root.printers.state === "available"
    readonly property bool printersVisible: root.printers.visible !== undefined
        ? root.printers.visible === true : root.printersAvailable
    readonly property int printersPrinterCount:
        root.printers.printerCount !== undefined ? Number(root.printers.printerCount) : 0
    readonly property int printersScannerCount:
        root.printers.scannerCount !== undefined ? Number(root.printers.scannerCount) : 0
    readonly property string printersGlyph:
        root.printers.glyph !== undefined ? String(root.printers.glyph) : "printer"
    readonly property string printersLabel: {
        if (!root.printersAvailable)
            return qsTr("Unavailable");
        if (root.printers.label !== undefined && root.printers.label !== "")
            return root.printers.label;
        if (root.printersPrinterCount === 0 && root.printersScannerCount === 0)
            return qsTr("No Printers or Scanners");
        return qsTr("%1 Printers, %2 Scanners")
            .arg(root.printersPrinterCount).arg(root.printersScannerCount);
    }

    readonly property bool privacyAvailable: root.privacy.state === "available"
    readonly property bool privacyVisible: root.privacy.visible !== undefined
        ? root.privacy.visible === true : root.privacyAvailable
    readonly property int privacyAppCount:
        root.privacy.appCount !== undefined ? Number(root.privacy.appCount) : 0
    readonly property string privacyGlyph:
        root.privacy.glyph !== undefined ? String(root.privacy.glyph) : "privacy"
    readonly property string privacyLabel: {
        if (!root.privacyAvailable)
            return qsTr("Unavailable");
        if (root.privacy.label !== undefined && root.privacy.label !== "")
            return root.privacy.label;
        return root.privacyAppCount > 0
            ? qsTr("%1 Apps").arg(root.privacyAppCount)
            : qsTr("No App Permissions");
    }

    readonly property bool accessibilityAvailable: root.accessibility.state === "available"
    readonly property bool accessibilityVisible: root.accessibility.visible !== undefined
        ? root.accessibility.visible === true : root.accessibilityAvailable
    readonly property bool accessibilityScreenReader:
        root.accessibility.screenReader === true
    readonly property string accessibilityGlyph:
        root.accessibility.glyph !== undefined
            ? String(root.accessibility.glyph) : "accessibility"
    readonly property string accessibilityLabel: {
        if (!root.accessibilityAvailable)
            return qsTr("Unavailable");
        if (root.accessibility.label !== undefined && root.accessibility.label !== "")
            return root.accessibility.label;
        return root.accessibilityScreenReader ? qsTr("Screen Reader On") : qsTr("Off");
    }

    readonly property bool vpnAvailable: root.vpn.state === "available"
    readonly property bool vpnVisible: root.vpn.visible !== undefined
        ? root.vpn.visible === true : root.vpnAvailable
    readonly property bool vpnConnected: root.vpn.connectedCount !== undefined
        ? Number(root.vpn.connectedCount) > 0 : false
    readonly property string vpnGlyph: root.vpn.glyph !== undefined
        ? String(root.vpn.glyph) : "vpn-off"
    readonly property string vpnLabel: {
        if (!root.vpnAvailable)
            return qsTr("Unavailable");
        if (root.vpn.label !== undefined && root.vpn.label !== "")
            return root.vpn.label;
        return root.vpnConnected ? qsTr("Connected") : qsTr("Not Connected");
    }

    readonly property string focusMode: root.focusPolicy.mode !== undefined
        ? root.focusPolicy.mode : "off"
    readonly property bool focusOn: root.focusMode === "focus"
        || root.focusMode === "dnd"
    readonly property string focusProfileLabel:
        root.focusPolicy.profileLabel !== undefined
            && root.focusPolicy.profileLabel !== null
            ? String(root.focusPolicy.profileLabel) : ""
    readonly property string focusLabel: {
        if (root.focusOn && root.focusProfileLabel.length > 0)
            return root.focusProfileLabel;
        if (root.focusMode === "dnd")
            return qsTr("Do Not Disturb");
        if (root.focusMode === "focus")
            return qsTr("Focus");
        return qsTr("Off");
    }
    readonly property int focusBatched: root.focusPolicy.batchedCount !== undefined
        ? root.focusPolicy.batchedCount : 0
    readonly property string focusSubtitle: {
        if (root.focusOn && root.focusBatched > 0)
            return qsTr("%1 \u00b7 %2 silenced").arg(root.focusLabel)
                .arg(root.focusBatched);
        return root.focusLabel;
    }

    // ── Clipboard history (T-13.5b) ──────────────────────────────────────
    property var clipboardEntries: []
    readonly property int clipboardShown: Math.min(root.clipboardEntries.length, 5)

    signal clipboardCopyRequested(int index)
    signal clipboardPinToggled(int index, bool pinned)
    signal clipboardClearRequested()

    // ── Module state map ─────────────────────────────────────────────────
    // Every catalogue id resolves to the live presentation the capsule and
    // the expanded sheet use. A module whose owner is absent is `available:
    // false` with an explicit reason, never invented data.
    readonly property var moduleStates: ({
        "wifi": { title: qsTr("Wi-Fi"), subtitle: root.wifiLabel,
                  glyph: "wifi", kind: "toggle", checked: root.wifiOn,
                  enabled: root.wifiAvailable && root.wifiWritable,
                  available: root.wifiAvailable,
                  objectName: "wifiTile" },
        "bluetooth": { title: qsTr("Bluetooth"), subtitle: root.bluetoothLabel,
                       glyph: "bluetooth", kind: "toggle",
                       checked: root.bluetoothOn,
                       enabled: root.bluetoothVisible && root.bluetoothWritable,
                       available: root.bluetoothVisible,
                       visible: root.bluetoothVisible,
                       objectName: "bluetoothTile" },
        "airdrop": { title: qsTr("Nearby Transfer"), subtitle: qsTr("Unavailable"),
                     glyph: "airdrop", kind: "action", checked: false,
                     enabled: false, available: false,
                     reason: qsTr("Nearby Transfer is not available on this system."),
                     objectName: "airdropTile" },
        "vpn": { title: qsTr("VPN"), subtitle: root.vpnLabel,
                 glyph: root.vpnGlyph, kind: "info", checked: root.vpnConnected,
                 enabled: root.vpnVisible, available: root.vpnAvailable,
                 visible: root.vpnVisible, objectName: "vpnTile" },
        "now-playing": { title: qsTr("Now Playing"),
                         subtitle: qsTr("No media playing"),
                         glyph: "now-playing", kind: "media", checked: false,
                         enabled: false, available: false,
                         reason: qsTr("No media player is publishing Now Playing."),
                         objectName: "nowPlayingTile" },
        "stage-manager": { title: qsTr("Stage"),
                           subtitle: qsTr("Unavailable"),
                           glyph: "stage-manager", kind: "action",
                           checked: false, enabled: false, available: false,
                           reason: qsTr("Stage is not part of this build."),
                           objectName: "stageManagerTile" },
        "screen-mirroring": { title: qsTr("Screen Casting"),
                              subtitle: qsTr("Not mirroring"),
                              glyph: "screen-mirroring", kind: "action",
                              checked: false, enabled: true, available: true,
                              objectName: "screenMirroringTile" },
        "edit-controls": { title: qsTr("Edit Controls"), subtitle: "",
                           glyph: "edit-controls", kind: "edit",
                           checked: false, enabled: true, available: true,
                           objectName: "editControlsTile" },
        "brightness": { title: qsTr("Display"),
                        subtitle: qsTr("%1%").arg(Math.round(root.brightness * 100)),
                        glyph: "brightness", kind: "slider", checked: false,
                        enabled: true, available: true, value: root.brightness,
                        objectName: "brightnessTile" },
        "dark": { title: qsTr("Dark Mode"),
                  subtitle: root.dark ? qsTr("On") : qsTr("Off"),
                  glyph: "appearance", kind: "toggle", checked: root.dark,
                  enabled: true, available: true, objectName: "darkTile" },
        "keyboard-brightness": { title: qsTr("Keyboard Brightness"),
                                 subtitle: qsTr("Unavailable"),
                                 glyph: "keyboard-brightness", kind: "info",
                                 checked: false, enabled: false, available: false,
                                 reason: qsTr("No keyboard backlight is exposed."),
                                 objectName: "keyboardBrightnessTile" },
        "focus": { title: qsTr("Focus"), subtitle: root.focusSubtitle,
                   glyph: "focus", kind: "toggle", checked: root.focusOn,
                   enabled: true, available: true, objectName: "focusTile" },
        "volume": { title: qsTr("Sound"), subtitle: root.muted
                        ? qsTr("Muted")
                        : (root.audioOutputName.length > 0
                           ? root.audioOutputName : qsTr("Output")),
                    glyph: "volume", kind: "slider", checked: false,
                    enabled: root.audioAvailable, available: root.audioAvailable,
                    value: root.volume, objectName: "volumeTile" },
        "airplay-audio": { title: qsTr("Audio Casting"),
                           subtitle: qsTr("Unavailable"),
                           glyph: "airplay-audio", kind: "info", checked: false,
                           enabled: false, available: false,
                           reason: qsTr("Audio casting is not available on this system."),
                           objectName: "airplayAudioTile" },
        "battery": { title: qsTr("Battery"), subtitle: root.batteryLabel,
                     glyph: root.batteryGlyph, kind: "info",
                     checked: false, enabled: root.batteryVisible,
                     available: root.batteryVisible,
                     visible: root.batteryVisible, objectName: "batteryTile" },
        "clock": { title: qsTr("Clock"), subtitle: qsTr("Menu bar clock"),
                   glyph: "clock", kind: "info", checked: false, enabled: true,
                   available: true, objectName: "clockTile" },
        "lock-screen": { title: qsTr("Lock Screen"),
                         subtitle: root.lockScreenLabel, glyph: "lock",
                         kind: "info", checked: false,
                         enabled: root.lockScreenVisible,
                         available: root.lockScreenVisible,
                         visible: root.lockScreenVisible,
                         objectName: "lockScreenTile" },
        "keyboard": { title: qsTr("Keyboard"), subtitle: root.inputLabel,
                      glyph: "keyboard", kind: "info", checked: false,
                      enabled: root.inputVisible, available: root.inputVisible,
                      visible: root.inputVisible, objectName: "keyboardTile" },
        "shortcuts": { title: qsTr("Shortcuts"), subtitle: qsTr("Unavailable"),
                       glyph: "shortcuts", kind: "info", checked: false,
                       enabled: false, available: false,
                       reason: qsTr("Shortcuts has no provider in this build."),
                       objectName: "shortcutsTile" },
        "capture-screen": { title: qsTr("Capture Screen"),
                            subtitle: qsTr("Screenshot"),
                            glyph: "capture-screen", kind: "action",
                            checked: false, enabled: true, available: true,
                            objectName: "captureScreenTile" },
        "screenshot": { title: qsTr("Screenshot"), subtitle: qsTr("Capture"),
                        glyph: "screenshot", kind: "action", checked: false,
                        enabled: true, available: true,
                        objectName: "screenshotTile" },
        "storage": { title: qsTr("Storage"), subtitle: root.storageLabel,
                     glyph: "storage", kind: "storage", checked: false,
                     enabled: root.storageVisible, available: root.storageVisible,
                     visible: root.storageVisible, objectName: "storageTile" },
        "mission-control": { title: qsTr("Overview"),
                             subtitle: root.missionControlLabel,
                             glyph: "overview", kind: "info", checked: false,
                             enabled: root.missionControlVisible,
                             available: root.missionControlVisible,
                             visible: root.missionControlVisible,
                             objectName: "missionControlTile" },
        "menu-bar": { title: qsTr("Menu Bar"), subtitle: root.menuBarLabel,
                      glyph: "menu-bar", kind: "info", checked: false,
                      enabled: root.menuBarVisible, available: root.menuBarVisible,
                      visible: root.menuBarVisible, objectName: "menuBarTile" },
        "software-update": { title: qsTr("Software Update"),
                             subtitle: root.updatesLabel,
                             glyph: root.updatesGlyph, kind: "info",
                             checked: false, enabled: root.updatesAvailable,
                             available: root.updatesAvailable,
                             visible: root.updatesAvailable,
                             objectName: "updatesTile" },
        "users": { title: qsTr("Users"), subtitle: root.accountsLabel,
                   glyph: root.accountsGlyph, kind: "info", checked: false,
                   enabled: root.accountsAvailable,
                   available: root.accountsAvailable,
                   visible: root.accountsAvailable,
                   objectName: "usersTile" },
        "printers": { title: qsTr("Printers"), subtitle: root.printersLabel,
                      glyph: root.printersGlyph, kind: "info", checked: false,
                      enabled: root.printersVisible,
                      available: root.printersVisible,
                      visible: root.printersVisible,
                      objectName: "printersTile" },
        "privacy": { title: qsTr("Privacy"), subtitle: root.privacyLabel,
                     glyph: root.privacyGlyph, kind: "info", checked: false,
                     enabled: root.privacyVisible, available: root.privacyVisible,
                     visible: root.privacyVisible, objectName: "privacyTile" },
        "utilities": { title: qsTr("Utilities"), subtitle: qsTr("Unavailable"),
                       glyph: "utilities", kind: "info", checked: false,
                       enabled: false, available: false,
                       reason: qsTr("No utility is available in this build."),
                       objectName: "utilitiesTile" },
        "home-control": { title: qsTr("Home"), subtitle: qsTr("Unavailable"),
                          glyph: "home-control", kind: "info", checked: false,
                          enabled: false, available: false,
                          reason: qsTr("Home has no provider in this build."),
                          objectName: "homeControlTile" },
        "notes": { title: qsTr("Notes"), subtitle: qsTr("Unavailable"),
                   glyph: "notes", kind: "info", checked: false,
                   enabled: false, available: false,
                   reason: qsTr("Notes has no provider in this build."),
                   objectName: "notesTile" },
        "voice-memos": { title: qsTr("Voice Memos"),
                         subtitle: qsTr("Unavailable"),
                         glyph: "voice-memos", kind: "info", checked: false,
                         enabled: false, available: false,
                         reason: qsTr("Voice Memos has no provider in this build."),
                         objectName: "voiceMemosTile" },
        "fast-user-switching": { title: qsTr("Fast User Switching"),
                                 subtitle: qsTr("Unavailable"),
                                 glyph: "fast-user-switching", kind: "info",
                                 checked: false, enabled: false, available: false,
                                 reason: qsTr("Fast User Switching is not available."),
                                 objectName: "fastUserSwitchingTile" },
        "accessibility": { title: qsTr("Accessibility"),
                           subtitle: root.accessibilityLabel,
                           glyph: root.accessibilityGlyph, kind: "info",
                           checked: false, enabled: root.accessibilityVisible,
                           available: root.accessibilityVisible,
                           visible: root.accessibilityVisible,
                           objectName: "accessibilityTile" },
        "hearing-accessibility": { title: qsTr("Hearing"),
                                   subtitle: qsTr("Unavailable"),
                                   glyph: "hearing", kind: "info",
                                   checked: false, enabled: false, available: false,
                                   reason: qsTr("No hearing accessibility provider."),
                                   objectName: "hearingAccessibilityTile" },
        "motor-accessibility": { title: qsTr("Motor"),
                                 subtitle: qsTr("Unavailable"),
                                 glyph: "motor", kind: "info", checked: false,
                                 enabled: false, available: false,
                                 reason: qsTr("No motor accessibility provider."),
                                 objectName: "motorAccessibilityTile" },
        "vision-accessibility": { title: qsTr("Vision"),
                                  subtitle: qsTr("Unavailable"),
                                  glyph: "vision", kind: "info", checked: false,
                                  enabled: false, available: false,
                                  reason: qsTr("No vision accessibility provider."),
                                  objectName: "visionAccessibilityTile" }
    })

    function moduleMeta(id) {
        var entry = root.registry.module(id);
        var state = root.moduleStates[id];
        if (entry === null)
            return null;
        var out = {
            id: entry.id,
            title: entry.label,
            glyph: entry.glyph,
            kind: entry.kind,
            span: root.registry.spanFor(id, root.spanMap),
            category: entry.category,
            owner: entry.owner,
            expandable: entry.expandable
        };
        if (state !== undefined) {
            for (var key in state)
                out[key] = state[key];
        }
        return out;
    }

    // The ordered, laid-out module list for the active page. `footer` modules
    // (Edit Controls) keep their page position but are not grid cells.
    readonly property var tiles: {
        var ids = [];
        for (var k = 0; k < root.moduleOrder.length; ++k) {
            var candidate = root.moduleOrder[k];
            if (root.quickSettingsMenuBarOnly.indexOf(candidate) < 0)
                ids.push(candidate);
        }
        var layout = root.registry.layout(ids, root.spanMap);
        var out = [];
        var seen = {};
        for (var i = 0; i < layout.length; ++i) {
            var entry = layout[i];
            if (seen[entry.id] === true)
                continue;
            var meta = root.moduleMeta(entry.id);
            if (meta === null)
                continue;
            for (var key in entry)
                meta[key] = entry[key];
            meta.footer = false;
            out.push(meta);
            seen[entry.id] = true;
        }
        // Footer controls keep the page order but render below the grid.
        var gridHeight = root.registry.gridHeight(layout);
        for (var j = 0; j < ids.length; ++j) {
            var id = ids[j];
            if (seen[id] === true)
                continue;
            var meta = root.moduleMeta(id);
            if (meta === null || meta.span !== "footer")
                continue;
            meta.x = 0;
            meta.y = gridHeight;
            meta.w = root.registry.moduleFieldWidth;
            meta.h = 40;
            meta.footer = true;
            out.push(meta);
            seen[id] = true;
        }
        return out;
    }

    function tileById(id) {
        for (var i = 0; i < root.tiles.length; ++i) {
            if (root.tiles[i].id === id)
                return root.tiles[i];
        }
        return null;
    }

    readonly property var gridTiles: {
        var out = [];
        for (var i = 0; i < root.tiles.length; ++i) {
            if (!root.tiles[i].footer && root.tiles[i].visible !== false)
                out.push(root.tiles[i]);
        }
        return out;
    }

    readonly property int gridHeight: {
        var height = 0;
        for (var i = 0; i < root.gridTiles.length; ++i)
            height = Math.max(height, root.gridTiles[i].y + root.gridTiles[i].h);
        return height;
    }

    readonly property var footerTiles: {
        var out = [];
        for (var i = 0; i < root.tiles.length; ++i) {
            if (root.tiles[i].footer)
                out.push(root.tiles[i]);
        }
        // Edit Controls is the management entry point; it is always present
        // even if a customization list omits it.
        if (out.length === 0) {
            var meta = root.moduleMeta("edit-controls");
            if (meta !== null) {
                meta.footer = true;
                out.push(meta);
            }
        }
        return out;
    }

    // ── Expanded sheet state ─────────────────────────────────────────────
    property string expandedId: ""
    readonly property bool wifiExpanded: root.expandedId === "wifi"
    readonly property bool bluetoothExpanded: root.expandedId === "bluetooth"

    function openModule(id) {
        if (id === "edit-controls") {
            root.openEditControls();
            return;
        }
        var meta = root.moduleMeta(id);
        if (meta === null)
            return;
        root.expandedId = (root.expandedId === id) ? "" : id;
    }

    function closeSheet() {
        root.expandedId = "";
    }

    function toggleWifiExpanded() {
        root.expandedId = root.expandedId === "wifi" ? "" : "wifi";
    }

    function toggleBluetoothExpanded() {
        root.expandedId = root.expandedId === "bluetooth" ? "" : "bluetooth";
    }

    // ── Edit Controls ────────────────────────────────────────────────────
    function openEditControls() {
        root.editMode = true;
        root.expandedId = "";
        root.editControlsRequested();
    }

    function closeEditControls() {
        root.editMode = false;
        root.editControlsClosed();
    }

    // ── Material presets ─────────────────────────────────────────────────
    readonly property var outerGlass: Glass.resolve(
        root.expandedId.length > 0 ? "controlCenterExpandedOuter"
                                   : "controlCenterOuter", {
        scheme: root.dark ? "dark" : "light",
        rollout: Glass.rollout,
        reduceTransparency: Glass.reduceTransparency,
        increaseContrast: Glass.increaseContrast,
        backend: materialFallback === 2 ? "software"
               : materialBlurActive ? "gpu-sampled" : "gpu-no-sampled-blur",
        tier: materialTier === 2 ? "minimal" : materialTier === 1 ? "reduced" : "full"
    })
    readonly property var moduleGlass: Glass.resolve("controlCenterModule", {
        scheme: root.dark ? "dark" : "light",
        rollout: Glass.rollout,
        reduceTransparency: Glass.reduceTransparency,
        increaseContrast: Glass.increaseContrast,
        backend: materialFallback === 2 ? "software"
               : materialBlurActive ? "gpu-sampled" : "gpu-no-sampled-blur",
        tier: materialTier === 2 ? "minimal" : materialTier === 1 ? "reduced" : "full"
    })
    readonly property var expandedGlass: Glass.resolve("controlCenterExpanded", {
        scheme: root.dark ? "dark" : "light",
        rollout: Glass.rollout,
        reduceTransparency: Glass.reduceTransparency,
        increaseContrast: Glass.increaseContrast,
        backend: materialFallback === 2 ? "software"
               : materialBlurActive ? "gpu-sampled" : "gpu-no-sampled-blur",
        tier: materialTier === 2 ? "minimal" : materialTier === 1 ? "reduced" : "full"
    })
    readonly property bool replacementLook: Glass.rollout === "replacement"
    readonly property bool sampledGlass: replacementLook && materialBlurActive
        && outerGlass.renderMode === "sampled-glass"
    readonly property real outerRadius: replacementLook
        ? Theme.primitive.radius.xxl : Theme.controls.popover.radius
    readonly property color outerFill: sampledGlass ? "transparent"
        : replacementLook
          ? Qt.rgba(outerGlass.tint, outerGlass.tint, outerGlass.tint, outerGlass.alpha)
          : Material.popupFill(materialBlurActive)
    readonly property real outerBorderWidth: sampledGlass ? 0
        : replacementLook ? Math.max(1, outerGlass.edgeWidth)
        : Theme.controls.window.borderWidth
    readonly property color outerBorderColor: replacementLook
        ? (outerGlass.tint > 0.5 ? Qt.rgba(0, 0, 0, 0.18)
                                 : Qt.rgba(1, 1, 1, outerGlass.edge))
        : Theme.color.border
    readonly property color moduleFill: sampledGlass ? "transparent"
        : replacementLook
          ? Qt.rgba(moduleGlass.tint, moduleGlass.tint, moduleGlass.tint, moduleGlass.alpha)
          : Theme.color.surfaceSunken
    readonly property color moduleHoverFill: sampledGlass
        ? Qt.rgba(1, 1, 1, 0.10) : Theme.color.surfaceElevated
    readonly property real moduleBorderWidth: replacementLook && !sampledGlass
        ? Math.max(1, moduleGlass.edgeWidth) : 0
    readonly property color moduleBorderColor: moduleBorderWidth > 0
        ? (moduleGlass.tint > 0.5 ? Qt.rgba(0, 0, 0, 0.18)
                                  : Qt.rgba(1, 1, 1, Math.min(1, moduleGlass.edge)))
        : "transparent"
    readonly property color expandedFill: sampledGlass ? "transparent"
        : replacementLook
          ? Qt.rgba(expandedGlass.tint, expandedGlass.tint, expandedGlass.tint,
                    expandedGlass.alpha)
          : Theme.color.surfaceSunken
    readonly property real expandedBorderWidth: replacementLook && !sampledGlass
        ? Math.max(1, expandedGlass.edgeWidth) : 0
    readonly property color expandedBorderColor: expandedBorderWidth > 0
        ? (expandedGlass.tint > 0.5 ? Qt.rgba(0, 0, 0, 0.18)
                                    : Qt.rgba(1, 1, 1, Math.min(1, expandedGlass.edge)))
        : "transparent"
    readonly property color moduleTextPrimary:
        sampledGlass && !root.dark ? "#f8f6fa" : Theme.color.textPrimary
    readonly property color moduleTextSecondary:
        sampledGlass && !root.dark ? "#d8d0e0" : Theme.color.textSecondary
    readonly property color moduleAccent:
        sampledGlass && !root.dark ? "#f0a6c8" : Theme.color.accent
    readonly property color moduleAccentHover:
        sampledGlass && !root.dark ? "#ffd0e2" : Theme.color.accentHover
    readonly property color moduleAccentMuted:
        sampledGlass && !root.dark ? Qt.rgba(1, 1, 1, 0.16) : Theme.color.accentMuted
    readonly property color moduleControlFill:
        sampledGlass && !root.dark ? Qt.rgba(1, 1, 1, 0.16) : Theme.color.controlFill
    readonly property color expandedTextPrimary: Theme.color.textPrimary
    readonly property color expandedTextSecondary: Theme.color.textSecondary
    readonly property color expandedAccent: Theme.color.accent

    // ── Signals (the shell forwards every gesture) ───────────────────────
    signal closed()
    signal volumeSetRequested(double volume)
    signal muteToggleRequested()
    signal brightnessSetRequested(double level)
    signal wifiToggleRequested(bool enabled)
    signal wifiSettingsRequested()
    signal soundSettingsRequested()
    signal bluetoothToggleRequested(bool enabled)
    signal bluetoothDeviceToggled(string address, bool connected)
    signal bluetoothSettingsRequested()
    signal storageMountRequested(string volumePath)
    signal storageUnmountRequested(string volumePath)
    signal storageEjectRequested(string drivePath)
    signal storageSettingsRequested()
    signal focusToggleRequested(bool enabled)
    signal focusModeSetRequested(string mode)
    signal focusSettingsRequested()
    signal darkModeToggleRequested(bool dark)
    signal appearanceSettingsRequested()
    signal keyboardSettingsRequested()
    signal missionControlSettingsRequested()
    signal batterySettingsRequested()
    signal lockScreenSettingsRequested()
    signal menuBarSettingsRequested()
    signal updatesCheckRequested()
    signal updatesInstallRequested()
    signal updatesRebootRequested()
    signal updatesSettingsRequested()
    signal usersSettingsRequested()
    signal printersSettingsRequested()
    signal privacySettingsRequested()
    signal accessibilitySettingsRequested()
    signal vpnSettingsRequested()
    signal quickSettingsSettingsRequested()
    signal screenshotRequested()
    signal screencastRequested()
    signal editControlsRequested()
    signal editControlsClosed()
    // One typed write path for the customization keys; the shell validates
    // and forwards it through settingsd (the single writer).
    signal quickSettingsWriteRequested(string key, var value)

    // ── Gestures ─────────────────────────────────────────────────────────
    function setVolume(fraction) {
        root.volume = Math.max(0.0, Math.min(1.0, fraction));
        root.volumeSetRequested(root.volume);
    }

    function setBrightness(level) {
        root.brightness = Math.max(0.0, Math.min(1.0, level));
        root.brightnessSetRequested(root.brightness);
    }

    function toggleWifi() {
        if (!root.wifiAvailable || !root.wifiWritable)
            return;
        root.wifiToggleRequested(!root.wifiOn);
    }

    function toggleBluetooth() {
        if (!root.bluetoothVisible || !root.bluetoothWritable)
            return;
        root.bluetoothToggleRequested(!root.bluetoothOn);
    }

    function toggleBluetoothDevice(address) {
        if (!root.bluetoothVisible || !root.bluetoothWritable || !address)
            return;
        for (var i = 0; i < root.bluetoothDevices.length; ++i) {
            if (root.bluetoothDevices[i].address === address) {
                root.bluetoothDeviceToggled(address,
                                            !root.bluetoothDevices[i].connected);
                return;
            }
        }
    }

    function toggleStorageVolume(path) {
        if (!root.storageVisible || !root.storageWritable || !path)
            return;
        for (var i = 0; i < root.storageVolumes.length; ++i) {
            if (root.storageVolumes[i].path === path) {
                if (root.storageVolumes[i].mounted)
                    root.storageUnmountRequested(path);
                else
                    root.storageMountRequested(path);
                return;
            }
        }
    }

    function ejectStorageVolume(path) {
        if (!root.storageVisible || !root.storageWritable || !path)
            return;
        for (var i = 0; i < root.storageVolumes.length; ++i) {
            if (root.storageVolumes[i].path === path
                    && root.storageVolumes[i].drivePath)
                root.storageEjectRequested(root.storageVolumes[i].drivePath);
        }
    }

    function toggleFocus() {
        root.focusToggleRequested(!root.focusOn);
    }

    function toggleDarkMode() {
        root.darkModeToggleRequested(!root.dark);
    }

    function performUpdatesAction() {
        if (!root.updatesAvailable || !root.updatesProvider || root.updatesBusy)
            return;
        if (root.updatesRebootRequired)
            root.updatesRebootRequested();
        else if (root.updatesCount > 0)
            root.updatesInstallRequested();
        else
            root.updatesCheckRequested();
    }

    function requestModuleAction(id) {
        switch (id) {
        case "capture-screen":
        case "screenshot":
            root.screenshotRequested();
            break;
        case "screen-mirroring":
            root.screencastRequested();
            break;
        case "wifi": root.wifiSettingsRequested(); break;
        case "bluetooth": root.bluetoothSettingsRequested(); break;
        case "volume": root.soundSettingsRequested(); break;
        case "brightness":
        case "dark": root.appearanceSettingsRequested(); break;
        case "focus": root.focusSettingsRequested(); break;
        case "battery": root.batterySettingsRequested(); break;
        case "storage": root.storageSettingsRequested(); break;
        case "keyboard":
        case "keyboard-brightness": root.keyboardSettingsRequested(); break;
        case "mission-control": root.missionControlSettingsRequested(); break;
        case "lock-screen": root.lockScreenSettingsRequested(); break;
        case "menu-bar": root.menuBarSettingsRequested(); break;
        case "software-update": root.updatesSettingsRequested(); break;
        case "users": root.usersSettingsRequested(); break;
        case "printers": root.printersSettingsRequested(); break;
        case "privacy": root.privacySettingsRequested(); break;
        case "accessibility":
        case "hearing-accessibility":
        case "motor-accessibility":
        case "vision-accessibility": root.accessibilitySettingsRequested(); break;
        case "vpn": root.vpnSettingsRequested(); break;
        default: break;
        }
    }

    function settingsLinkObjectName(id) {
        switch (id) {
        case "keyboard": return "keyboardSettingsLink";
        case "keyboard-brightness": return "keyboardBrightnessSettingsLink";
        case "mission-control": return "missionControlSettingsLink";
        case "lock-screen": return "lockScreenSettingsLink";
        case "menu-bar": return "menuBarSettingsLink";
        case "software-update": return "updatesSettingsLink";
        case "users": return "usersSettingsLink";
        case "printers": return "printersSettingsLink";
        case "privacy": return "privacySettingsLink";
        case "accessibility": return "accessibilitySettingsLink";
        case "vpn": return "vpnSettingsLink";
        default: return id + "SettingsLink";
        }
    }

    function settingsLinkAccessibleName(id) {
        switch (id) {
        case "wifi": return qsTr("Open Wi-Fi Settings");
        case "bluetooth": return qsTr("Open Bluetooth Settings");
        case "volume": return qsTr("Open Sound Settings");
        case "brightness": return qsTr("Open Display Settings");
        case "dark": return qsTr("Open Appearance Settings");
        case "focus": return qsTr("Open Focus Settings");
        case "battery": return qsTr("Open Battery Settings");
        case "storage": return qsTr("Open Storage Settings");
        case "keyboard":
        case "keyboard-brightness": return qsTr("Open Keyboard Settings");
        case "mission-control": return qsTr("Open Overview Settings");
        case "lock-screen": return qsTr("Open Lock Screen Settings");
        case "menu-bar": return qsTr("Open Menu Bar Settings");
        case "software-update": return qsTr("Open General Settings");
        case "users": return qsTr("Open Users & Groups Settings");
        case "printers": return qsTr("Open Printers & Scanners Settings");
        case "privacy": return qsTr("Open Privacy & Security Settings");
        case "accessibility":
        case "hearing-accessibility":
        case "motor-accessibility":
        case "vision-accessibility": return qsTr("Open Accessibility Settings");
        case "vpn": return qsTr("Open Network Settings");
        default: return qsTr("Open %1 Settings")
            .arg(ControlCenterRegistry.moduleLabel(id));
        }
    }

    function settingsLinkLabel(id) {
        switch (id) {
        case "wifi": return qsTr("Wi-Fi Settings\u2026");
        case "bluetooth": return qsTr("Bluetooth Settings\u2026");
        case "volume": return qsTr("Sound Settings\u2026");
        case "brightness": return qsTr("Display Settings\u2026");
        case "dark": return qsTr("Appearance Settings\u2026");
        case "focus": return qsTr("Focus Settings\u2026");
        case "battery": return qsTr("Battery Settings\u2026");
        case "storage": return qsTr("Storage Settings\u2026");
        case "keyboard":
        case "keyboard-brightness": return qsTr("Keyboard Settings\u2026");
        case "mission-control": return qsTr("Overview Settings\u2026");
        case "lock-screen": return qsTr("Lock Screen Settings\u2026");
        case "menu-bar": return qsTr("Menu Bar Settings\u2026");
        case "software-update": return qsTr("General Settings\u2026");
        case "users": return qsTr("Users & Groups Settings\u2026");
        case "printers": return qsTr("Printers & Scanners Settings\u2026");
        case "privacy": return qsTr("Privacy & Security Settings\u2026");
        case "accessibility":
        case "hearing-accessibility":
        case "motor-accessibility":
        case "vision-accessibility": return qsTr("Accessibility Settings\u2026");
        case "vpn": return qsTr("Network Settings\u2026");
        default: return "";
        }
    }

    function performModuleToggle(id, checked) {
        switch (id) {
        case "wifi": root.wifiToggleRequested(checked); break;
        case "bluetooth": root.bluetoothToggleRequested(checked); break;
        case "focus": root.focusToggleRequested(checked); break;
        case "dark": root.darkModeToggleRequested(checked); break;
        default: break;
        }
    }

    // ── Layout metrics ───────────────────────────────────────────────────
    readonly property int contentMargin: Theme.primitive.spacing.md
    readonly property int fieldWidth: root.registry.moduleFieldWidth
    readonly property int sheetWidth: root.registry.expandedSheetWidth
    readonly property int panelWidth: root.width
    readonly property bool sheetOpen: root.expandedId.length > 0
    // The panel grows behind the expanded module; the fixed shell surface is
    // only the hosting limit. `panelHeight` is the visible outer plate.
    readonly property real panelHeight: Math.min(root.height,
        contentColumn.implicitHeight + 2 * root.contentMargin)

    // ── Material regions ─────────────────────────────────────────────────
    function _clipMaterialRegion(item, preset, key, corners) {
        if (!item || !item.visible)
            return null
        var itemX = item.x
        var itemY = item.y
        var itemW = item.width
        var itemH = item.height
        if (itemW <= 0 || itemH <= 0)
            return null
        var viewportX = contentScroll.x
        var viewportY = contentScroll.y
        var viewportW = contentScroll.width
        var viewportH = contentScroll.height
        var p = item.mapToItem(root, 0, 0)
        var x1 = Math.max(p.x, viewportX)
        var y1 = Math.max(p.y, viewportY)
        var x2 = Math.min(p.x + itemW, viewportX + viewportW)
        var y2 = Math.min(p.y + itemH, viewportY + viewportH)
        if (x2 <= x1 || y2 <= y1)
            return null
        return {
            key: key, preset: preset,
            x: x1, y: y1, w: x2 - x1, h: y2 - y1,
            tl: corners, tr: corners, br: corners, bl: corners,
            overscan: 6
        }
    }

    readonly property var materialRegions: {
        if (root.editMode)
            return []
        var scrollY = contentScroll.contentY
        var contentHeight = contentColumn.implicitHeight
        var out = [{
            key: "outer",
            preset: root.sheetOpen ? "controlCenterExpandedOuter"
                                   : "controlCenterOuter",
            x: 0, y: 0, w: background.width, h: background.height,
            tl: root.outerRadius, tr: root.outerRadius,
            br: root.outerRadius, bl: root.outerRadius,
            overscan: 8
        }]
        for (var i = 0; i < gridRepeater.count; ++i) {
            var item = gridRepeater.itemAt(i)
            if (!item)
                continue
            var tile = root.gridTiles[i]
            if (!tile)
                continue
            var region = root._clipMaterialRegion(item, "controlCenterModule",
                             "module-" + tile.id, Theme.primitive.radius.md)
            if (region)
                out.push(region)
        }
        if (root.sheetOpen && sheetColumn.visible) {
            var sheet = root._clipMaterialRegion(sheetColumn,
                             "controlCenterExpanded",
                             "expanded-" + root.expandedId,
                             Theme.primitive.radius.sm)
            if (sheet)
                out.push(sheet)
        }
        return out
    }
    onVolumeChanged: volumeSlider.value = root.volume

    focus: true
    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Quick Settings")

    Keys.onEscapePressed: (event) => {
        if (root.expandedId.length > 0) {
            root.closeSheet();
        } else {
            root.closed();
        }
        event.accepted = true;
    }

    // ── Normal panel scene ───────────────────────────────────────────────
    Rectangle {
        id: background
        objectName: "controlCenterSurface"
        visible: !root.editMode
        width: root.width
        height: root.panelHeight
        color: root.outerFill
        radius: root.outerRadius
        border.width: root.outerBorderWidth
        border.color: root.outerBorderColor

        Flickable {
            id: contentScroll
            objectName: "controlCenterScroll"
            anchors.fill: parent
            anchors.margins: root.contentMargin
            contentWidth: width
            contentHeight: contentColumn.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            Column {
                id: contentColumn
                objectName: "controlCenterContent"
                width: contentScroll.width
                spacing: Theme.primitive.spacing.sm

                // ── Measured 4-column grid ───────────────────────────────
                Item {
                    id: gridField
                    objectName: "controlCenterGrid"
                    width: root.fieldWidth
                    height: root.gridHeight
                    x: (parent.width - width) / 2

                    Repeater {
                        id: gridRepeater
                        model: root.gridTiles

                        delegate: ControlCenterModule {
                            required property var modelData
                            tile: modelData
                            x: modelData.x
                            y: modelData.y
                            sampled: root.sampledGlass
                            reduceMotion: root.reduceMotion
                            fill: root.moduleFill
                            hoverFill: root.moduleHoverFill
                            textPrimary: root.moduleTextPrimary
                            textSecondary: root.moduleTextSecondary
                            accent: root.moduleAccent
                            accentMuted: root.moduleAccentMuted
                            controlFill: root.moduleControlFill
                            rimWidth: root.moduleBorderWidth
                            rimColor: root.moduleBorderColor
                            onActivated: root.openModule(modelData.id)
                            onToggleRequested: (checked) =>
                                root.performModuleToggle(modelData.id, checked)
                            onSliderMoved: (value) => {
                                if (modelData.id === "volume")
                                    root.setVolume(value)
                                else if (modelData.id === "brightness")
                                    root.setBrightness(value)
                            }
                            onMediaControlRequested: (action) => {
                                // No media provider: the explicit absence in
                                // the sheet is the only truthful answer.
                            }
                        }
                    }
                }

                // ── Page control (multiple pages) ────────────────────────
                Row {
                    id: pageRow
                    objectName: "controlCenterPages"
                    visible: root.pageCount > 1
                    width: root.fieldWidth
                    height: 22
                    x: (parent.width - width) / 2
                    spacing: Theme.primitive.spacing.xs

                    Text {
                        text: "\u2039"
                        color: root.moduleTextSecondary
                        font.pixelSize: Theme.primitive.font.sizeLg
                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Previous Quick Settings page")
                        TapHandler {
                            onTapped: root.setActivePage(root.activePage - 1)
                        }
                    }

                    Repeater {
                        model: root.pageCount
                        delegate: Rectangle {
                            required property int index
                            width: 8
                            height: 8
                            radius: 4
                            y: (parent.height - height) / 2
                            color: root.activePage === index
                                ? root.moduleAccent : root.moduleTextSecondary
                            Accessible.role: Accessible.PageTab
                            Accessible.name: qsTr("Page %1").arg(index + 1)
                            Accessible.selected: root.activePage === index
                            TapHandler {
                                onTapped: root.setActivePage(index)
                            }
                        }
                    }

                    Text {
                        text: "\u203a"
                        color: root.moduleTextSecondary
                        font.pixelSize: Theme.primitive.font.sizeLg
                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Next Quick Settings page")
                        TapHandler {
                            onTapped: root.setActivePage(root.activePage + 1)
                        }
                    }
                }

                // ── Expanded sheet (grows the outer panel) ───────────────
                Column {
                    id: sheetColumn
                    objectName: "controlCenterExpandedSheet"
                    visible: root.sheetOpen
                    width: root.sheetWidth
                    height: visible ? implicitHeight : 0
                    x: (parent.width - width) / 2
                    spacing: Theme.primitive.spacing.xxs

                    // Wi-Fi: 308×87 pt measured sheet with a 280×32 row.
                    Rectangle {
                        id: wifiSheet
                        objectName: "wifiSheet"
                        visible: root.expandedId === "wifi"
                        width: root.sheetWidth
                        implicitHeight: wifiSheetColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: wifiSheetColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                objectName: "wifiNetworksEmpty"
                                visible: root.wifiNetworks.length === 0
                                width: parent.width
                                text: root.wifiAvailable
                                    ? qsTr("No networks") : qsTr("Wi-Fi unavailable")
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }

                            Repeater {
                                model: root.wifiNetworks
                                delegate: Item {
                                    id: wifiNetworkRow
                                    required property var modelData
                                    width: wifiSheetColumn.width
                                    height: 32

                                    Text {
                                        objectName: "wifiNetworkName"
                                        anchors.left: parent.left
                                        anchors.right: wifiNetworkStatus.left
                                        anchors.rightMargin: Theme.primitive.spacing.sm
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: wifiNetworkRow.modelData.ssid
                                        color: root.expandedTextPrimary
                                        font.pixelSize: Theme.primitive.font.sizeSm
                                        elide: Text.ElideRight
                                    }

                                    Text {
                                        id: wifiNetworkStatus
                                        objectName: "wifiNetworkStatus"
                                        anchors.right: parent.right
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: wifiNetworkRow.modelData.active === true
                                            ? qsTr("Connected")
                                            : (wifiNetworkRow.modelData.security !== undefined
                                               ? wifiNetworkRow.modelData.security : "")
                                        color: wifiNetworkRow.modelData.active === true
                                            ? root.expandedAccent : root.expandedTextSecondary
                                        font.pixelSize: Theme.primitive.font.sizeSm
                                    }
                                }
                            }

                            TextLink {
                                objectName: "wifiDisclosure"
                                visible: root.wifiAvailable
                                text: qsTr("Other Networks\u2026")
                                accessibleName: qsTr("Show other Wi-Fi networks")
                                onActivated: root.wifiSettingsRequested()
                            }

                            TextLink {
                                objectName: "wifiSettingsLink"
                                text: qsTr("Wi-Fi Settings\u2026")
                                accessibleName: qsTr("Open Wi-Fi Settings")
                                onActivated: root.wifiSettingsRequested()
                            }
                        }
                    }

                    // Bluetooth: 308×64 pt measured sheet, two 280×32 rows.
                    Rectangle {
                        id: bluetoothSheet
                        objectName: "bluetoothSheet"
                        visible: root.expandedId === "bluetooth"
                        width: root.sheetWidth
                        implicitHeight: bluetoothSheetColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: bluetoothSheetColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                objectName: "bluetoothDevicesEmpty"
                                visible: root.bluetoothDevices.length === 0
                                width: parent.width
                                text: root.bluetoothVisible
                                    ? qsTr("No devices") : qsTr("Bluetooth unavailable")
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }

                            Column {
                                id: bluetoothDeviceList
                                objectName: "bluetoothDeviceList"
                                width: parent.width
                                spacing: 0
                                readonly property int count: root.bluetoothDevices.length

                                Repeater {
                                    model: root.bluetoothDevices
                                    delegate: Item {
                                        id: bluetoothRow
                                        required property var modelData
                                        width: bluetoothDeviceList.width
                                        height: 32

                                        Text {
                                            objectName: "bluetoothDeviceName"
                                            anchors.left: parent.left
                                            anchors.right: bluetoothDeviceAction.left
                                            anchors.rightMargin: Theme.primitive.spacing.sm
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: bluetoothRow.modelData.name
                                            color: root.expandedTextPrimary
                                            font.pixelSize: Theme.primitive.font.sizeSm
                                            elide: Text.ElideRight
                                        }

                                        Text {
                                            id: bluetoothDeviceAction
                                            objectName: "bluetoothDeviceAction"
                                            anchors.verticalCenter: parent.verticalCenter
                                            anchors.right: parent.right
                                            text: bluetoothRow.modelData.connected
                                                ? qsTr("Connected") : qsTr("Not Connected")
                                            color: bluetoothRow.modelData.connected
                                                ? root.expandedAccent : root.expandedTextSecondary
                                            font.pixelSize: Theme.primitive.font.sizeSm
                                            Accessible.role: Accessible.Button
                                            Accessible.name: qsTr("%1, %2")
                                                .arg(bluetoothRow.modelData.name)
                                                .arg(text)
                                            TapHandler {
                                                onTapped: root.toggleBluetoothDevice(
                                                    bluetoothRow.modelData.address)
                                            }
                                        }
                                    }
                                }
                            }

                            TextLink {
                                objectName: "bluetoothDisclosure"
                                visible: root.bluetoothVisible
                                text: root.bluetoothExpanded
                                    ? qsTr("Hide Devices") : qsTr("Show Devices")
                                accessibleName: root.bluetoothExpanded
                                    ? qsTr("Hide Bluetooth devices")
                                    : qsTr("Show Bluetooth devices")
                                onActivated: root.toggleBluetoothExpanded()
                            }

                            TextLink {
                                objectName: "bluetoothSettingsLink"
                                text: qsTr("Bluetooth Settings\u2026")
                                accessibleName: qsTr("Open Bluetooth Settings")
                                onActivated: root.bluetoothSettingsRequested()
                            }
                        }
                    }

                    // Storage: volume rows plus the pane link.
                    Rectangle {
                        id: storageSheet
                        objectName: "storageSheet"
                        visible: root.expandedId === "storage"
                        width: root.sheetWidth
                        implicitHeight: storageSheetColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: storageSheetColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                width: parent.width
                                text: root.storageLabel
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                wrapMode: Text.WordWrap
                            }

                            Column {
                                id: storageVolumeList
                                objectName: "storageVolumeList"
                                width: parent.width
                                spacing: Theme.primitive.spacing.xxs
                                readonly property int count: root.storageVolumes.length
                                visible: count > 0

                                Repeater {
                                    model: root.storageVolumes
                                    delegate: Item {
                                        id: storageRow
                                        required property var modelData
                                        width: storageVolumeList.width
                                        height: 26

                                        Text {
                                            objectName: "storageVolumeName"
                                            anchors.left: parent.left
                                            anchors.right: storageRowAction.left
                                            anchors.rightMargin: Theme.primitive.spacing.sm
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: storageRow.modelData.name
                                            color: root.expandedTextPrimary
                                            font.pixelSize: Theme.primitive.font.sizeSm
                                            elide: Text.ElideRight
                                        }

                                        Text {
                                            id: storageRowEject
                                            objectName: "storageVolumeEject"
                                            visible: storageRow.modelData.removable === true
                                                     && storageRow.modelData.drivePath !== undefined
                                            anchors.right: parent.right
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: qsTr("Eject")
                                            color: root.expandedAccent
                                            font.pixelSize: Theme.primitive.font.sizeSm
                                            Accessible.role: Accessible.Button
                                            Accessible.name: qsTr("Eject %1")
                                                .arg(storageRow.modelData.name)
                                            TapHandler {
                                                onTapped: root.ejectStorageVolume(
                                                    storageRow.modelData.path)
                                            }
                                        }

                                        Text {
                                            id: storageRowAction
                                            objectName: "storageVolumeAction"
                                            anchors.right: storageRowEject.visible
                                                ? storageRowEject.left : parent.right
                                            anchors.rightMargin: storageRowEject.visible
                                                ? Theme.primitive.spacing.md : 0
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: storageRow.modelData.mounted
                                                ? qsTr("Unmount") : qsTr("Mount")
                                            color: root.expandedAccent
                                            font.pixelSize: Theme.primitive.font.sizeSm
                                            Accessible.role: Accessible.Button
                                            Accessible.name: qsTr("%1 %2")
                                                .arg(text).arg(storageRow.modelData.name)
                                            TapHandler {
                                                onTapped: root.toggleStorageVolume(
                                                    storageRow.modelData.path)
                                            }
                                        }
                                    }
                                }
                            }

                            TextLink {
                                objectName: "storageSettingsLink"
                                text: qsTr("Storage Settings\u2026")
                                accessibleName: qsTr("Open Storage Settings")
                                onActivated: root.storageSettingsRequested()
                            }
                        }
                    }

                    // Focus: two rows (Do Not Disturb, Reduce Interruptions).
                    Rectangle {
                        id: focusSheet
                        objectName: "focusSheet"
                        visible: root.expandedId === "focus"
                        width: root.sheetWidth
                        implicitHeight: focusSheetColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: focusSheetColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                width: parent.width
                                text: root.focusSubtitle
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }

                            Item {
                                width: parent.width
                                height: 30
                                Text {
                                    anchors.left: parent.left
                                    anchors.right: focusDndState.left
                                    anchors.rightMargin: Theme.primitive.spacing.sm
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: qsTr("Do Not Disturb")
                                    color: root.expandedTextPrimary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                }
                                Text {
                                    id: focusDndState
                                    anchors.right: parent.right
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: root.focusMode === "dnd"
                                        ? qsTr("On") : qsTr("Off")
                                    color: root.focusMode === "dnd"
                                        ? root.expandedAccent : root.expandedTextSecondary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    Accessible.role: Accessible.Button
                                    Accessible.name: qsTr("Do Not Disturb, %1").arg(text)
                                    TapHandler {
                                        onTapped: root.focusModeSetRequested(
                                            root.focusMode === "dnd" ? "off" : "dnd")
                                    }
                                }
                            }

                            Item {
                                width: parent.width
                                height: 30
                                Text {
                                    anchors.left: parent.left
                                    anchors.right: focusReduceState.left
                                    anchors.rightMargin: Theme.primitive.spacing.sm
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: qsTr("Reduce Interruptions")
                                    color: root.expandedTextPrimary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                }
                                Text {
                                    id: focusReduceState
                                    anchors.right: parent.right
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: root.focusMode === "focus"
                                        ? qsTr("On") : qsTr("Off")
                                    color: root.focusMode === "focus"
                                        ? root.expandedAccent : root.expandedTextSecondary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    Accessible.role: Accessible.Button
                                    Accessible.name: qsTr("Reduce Interruptions, %1").arg(text)
                                    TapHandler {
                                        onTapped: root.focusModeSetRequested(
                                            root.focusMode === "focus" ? "off" : "focus")
                                    }
                                }
                            }

                            TextLink {
                                objectName: "focusSettingsLink"
                                text: qsTr("Focus Settings\u2026")
                                accessibleName: qsTr("Open Focus Settings")
                                onActivated: root.focusSettingsRequested()
                            }
                        }
                    }

                    // Screen Mirroring: display settings plus an empty list.
                    Rectangle {
                        id: screenMirroringSheet
                        objectName: "screenMirroringSheet"
                        visible: root.expandedId === "screen-mirroring"
                        width: root.sheetWidth
                        implicitHeight: screenMirroringColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: screenMirroringColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                objectName: "screenMirroringEmpty"
                                width: parent.width
                                text: qsTr("No displays available")
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }

                            TextLink {
                                objectName: "screenMirroringSettingsLink"
                                text: qsTr("Display Settings\u2026")
                                accessibleName: qsTr("Open Display Settings")
                                onActivated: root.requestModuleAction("brightness")
                            }
                        }
                    }

                    // Now Playing: explicit absence; controls live in the
                    // 2×2 capsule when a provider exists.
                    Rectangle {
                        id: nowPlayingSheet
                        objectName: "nowPlayingSheet"
                        visible: root.expandedId === "now-playing"
                        width: root.sheetWidth
                        implicitHeight: nowPlayingColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: nowPlayingColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                width: parent.width
                                text: qsTr("No media playing")
                                color: root.expandedTextPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                            Text {
                                width: parent.width
                                text: qsTr("No media player is publishing Now Playing.")
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeXs
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    // Sound: volume, mute, output device, settings link.
                    Rectangle {
                        id: soundSheet
                        objectName: "soundSheet"
                        visible: root.expandedId === "volume"
                        width: root.sheetWidth
                        implicitHeight: soundSheetColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: soundSheetColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                objectName: "volumeSubtitle"
                                width: parent.width
                                text: root.muted ? qsTr("Muted")
                                    : (root.audioOutputName.length > 0
                                       ? root.audioOutputName : qsTr("Output"))
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }

                            Slider {
                                id: volumeSlider
                                objectName: "volumeSlider"
                                width: parent.width
                                from: 0.0
                                to: 1.0
                                value: root.volume
                                enabled: root.audioAvailable
                                accessibleName: qsTr("Volume")
                                onMoved: (value) => root.setVolume(value)
                            }

                            TextLink {
                                objectName: "muteButton"
                                text: root.muted ? qsTr("Unmute") : qsTr("Mute")
                                accessibleName: root.muted ? qsTr("Unmute") : qsTr("Mute")
                                visible: root.audioAvailable
                                onActivated: root.muteToggleRequested()
                            }

                            TextLink {
                                objectName: "soundSettingsLink"
                                text: qsTr("Sound Settings\u2026")
                                accessibleName: qsTr("Open Sound Settings")
                                onActivated: root.soundSettingsRequested()
                            }
                        }
                    }

                    // Display: brightness slider.
                    Rectangle {
                        id: displaySheet
                        objectName: "displaySheet"
                        visible: root.expandedId === "brightness"
                        width: root.sheetWidth
                        implicitHeight: displaySheetColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: displaySheetColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                width: parent.width
                                text: qsTr("Brightness %1%").arg(Math.round(root.brightness * 100))
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }

                            Slider {
                                id: brightnessSlider
                                objectName: "brightnessSlider"
                                width: parent.width
                                from: 0.0
                                to: 1.0
                                value: root.brightness
                                accessibleName: qsTr("Brightness")
                                onMoved: (value) => root.setBrightness(value)
                            }

                            TextLink {
                                objectName: "appearanceSettingsLink"
                                text: qsTr("Display Settings\u2026")
                                accessibleName: qsTr("Open Display Settings")
                                onActivated: root.appearanceSettingsRequested()
                            }
                        }
                    }

                    // Battery: charge, profile and settings.
                    Rectangle {
                        id: batterySheet
                        objectName: "batterySheet"
                        visible: root.expandedId === "battery"
                        width: root.sheetWidth
                        implicitHeight: batterySheetColumn.implicitHeight
                                        + 2 * Theme.primitive.spacing.xs
                        radius: Theme.primitive.radius.sm
                        color: root.expandedFill
                        border.width: root.expandedBorderWidth
                        border.color: root.expandedBorderColor

                        Column {
                            id: batterySheetColumn
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.xs
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                width: parent.width
                                text: root.batteryLabel
                                color: root.expandedTextPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                            Text {
                                width: parent.width
                                visible: root.batteryProfiles
                                text: qsTr("Power mode: %1").arg(root.batteryProfileLabel)
                                color: root.expandedTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeXs
                            }
                            TextLink {
                                objectName: "batterySettingsLink"
                                text: qsTr("Battery Settings\u2026")
                                accessibleName: qsTr("Open Battery Settings")
                                onActivated: root.batterySettingsRequested()
                            }
                        }
                    }

                    // The generic catalogue sheet: state, absence reason and
                    // the owner's one action. Every remaining module gets one.
                    Repeater {
                        model: root.genericSheetIds

                        delegate: Rectangle {
                            required property string modelData
                            width: root.sheetWidth
                            implicitHeight: genericColumn.implicitHeight
                                            + 2 * Theme.primitive.spacing.xs
                            visible: root.expandedId === modelData
                            radius: Theme.primitive.radius.sm
                            color: root.expandedFill
                            border.width: root.expandedBorderWidth
                            border.color: root.expandedBorderColor

                            readonly property var meta: root.moduleMeta(modelData)

                            Column {
                                id: genericColumn
                                anchors.fill: parent
                                anchors.margins: Theme.primitive.spacing.xs
                                spacing: Theme.primitive.spacing.xxs

                                Text {
                                    width: parent.width
                                    text: meta !== null ? meta.title : modelData
                                    color: root.expandedTextPrimary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    font.weight: Theme.primitive.font.weightMedium
                                }

                                Text {
                                    width: parent.width
                                    text: {
                                        if (meta === null)
                                            return "";
                                        if (meta.reason !== undefined)
                                            return String(meta.reason);
                                        return meta.subtitle !== undefined
                                            ? String(meta.subtitle) : "";
                                    }
                                    color: root.expandedTextSecondary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    wrapMode: Text.WordWrap
                                }

                                TextLink {
                                    objectName: root.settingsLinkObjectName(modelData)
                                    visible: meta !== null
                                        && root.settingsLinkLabel(modelData).length > 0
                                    text: root.settingsLinkLabel(modelData)
                                    accessibleName: root.settingsLinkAccessibleName(modelData)
                                    onActivated: root.requestModuleAction(modelData)
                                }

                                TextLink {
                                    visible: modelData === "capture-screen"
                                        || modelData === "screenshot"
                                        || modelData === "screen-mirroring"
                                    text: modelData === "screen-mirroring"
                                        ? qsTr("Start Screen Casting\u2026")
                                        : qsTr("Capture Screen")
                                    accessibleName: text
                                    onActivated: root.requestModuleAction(modelData)
                                }

                                TextLink {
                                    visible: modelData === "software-update"
                                        && root.updatesProvider
                                    text: root.updatesAction
                                    accessibleName: qsTr("Software Update: %1")
                                        .arg(root.updatesAction)
                                    onActivated: root.performUpdatesAction()
                                }
                            }
                        }
                    }
                }

                // ── Clipboard history ───────────────────────────────────
                Column {
                    id: clipboardColumn
                    objectName: "clipboardSection"
                    width: root.fieldWidth
                    x: (parent.width - width) / 2
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        objectName: "clipboardTitle"
                        text: qsTr("Clipboard")
                        color: root.moduleTextPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        font.weight: Theme.primitive.font.weightMedium
                    }

                    Text {
                        objectName: "clipboardEmpty"
                        width: parent.width
                        visible: root.clipboardEntries.length === 0
                        text: qsTr("No recent items")
                        color: root.moduleTextSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Repeater {
                        model: root.clipboardShown
                        delegate: Rectangle {
                            id: clipboardRow
                            required property int index
                            readonly property var entry: root.clipboardEntries[index]
                            width: clipboardColumn.width
                            height: 26
                            radius: Theme.primitive.radius.sm
                            color: rowHover.hovered ? root.moduleHoverFill
                                                    : "transparent"

                            HoverHandler { id: rowHover }

                            Text {
                                objectName: "clipboardRowPreview"
                                anchors.left: parent.left
                                anchors.leftMargin: Theme.primitive.spacing.xs
                                anchors.right: pinAction.left
                                anchors.verticalCenter: parent.verticalCenter
                                text: clipboardRow.entry.preview
                                color: root.moduleTextPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                                Accessible.role: Accessible.Button
                                Accessible.name: qsTr("Copy %1").arg(clipboardRow.entry.preview)
                                TapHandler {
                                    onTapped: root.clipboardCopyRequested(clipboardRow.entry.index)
                                }
                            }

                            Text {
                                id: pinAction
                                objectName: "clipboardRowPin"
                                anchors.right: parent.right
                                anchors.rightMargin: Theme.primitive.spacing.xs
                                anchors.verticalCenter: parent.verticalCenter
                                text: clipboardRow.entry.pinned ? qsTr("Unpin") : qsTr("Pin")
                                color: clipboardRow.entry.pinned ? root.moduleAccent
                                                                 : root.moduleTextSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                Accessible.role: Accessible.Button
                                Accessible.name: clipboardRow.entry.pinned
                                    ? qsTr("Unpin %1").arg(clipboardRow.entry.preview)
                                    : qsTr("Pin %1").arg(clipboardRow.entry.preview)
                                TapHandler {
                                    onTapped: root.clipboardPinToggled(
                                        clipboardRow.entry.index, !clipboardRow.entry.pinned)
                                }
                            }
                        }
                    }

                    TextLink {
                        objectName: "clipboardClearLink"
                        text: qsTr("Clear Clipboard")
                        accessibleName: qsTr("Clear Clipboard")
                        visible: root.clipboardEntries.length > 0
                        onActivated: root.clipboardClearRequested()
                    }
                }

                // ── Footer: Edit Controls + the Settings pane link ──────
                Item {
                    id: footerRow
                    objectName: "controlCenterFooter"
                    width: root.fieldWidth
                    height: 32
                    x: (parent.width - width) / 2

                    Row {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.md

                        Repeater {
                            model: root.footerTiles
                            delegate: TextLink {
                                required property var modelData
                                objectName: "editControlsLink"
                                text: modelData.title
                                accessibleName: qsTr("Open Edit Controls")
                                onActivated: root.openEditControls()
                            }
                        }
                    }

                    TextLink {
                        objectName: "quickSettingsSettingsLink"
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Quick Settings\u2026")
                        accessibleName: qsTr("Open Quick Settings Settings")
                        onActivated: root.quickSettingsSettingsRequested()
                    }
                }
            }
        }
    }

    // ── Edit Controls surface (736×900) ──────────────────────────────────
    EditControls {
        id: editControls
        objectName: "editControlsSurface"
        visible: root.editMode
        width: root.registry.editWindowWidth
        height: root.registry.editWindowHeight
        panelModules: root.quickSettingsModules.length > 0
            ? root.quickSettingsModules : root.defaultModuleOrder
        page2: root.quickSettingsPage2
        page3: root.quickSettingsPage3
        pageCount: root.pageCount
        activePage: root.activePage
        menuBarOnly: root.quickSettingsMenuBarOnly
        spans: root.spanMap
        perDisplay: root.quickSettingsPerDisplay
        states: root.moduleStates
        onClosed: root.closeEditControls()
        onQuickSettingsWriteRequested: (key, value) =>
            root.quickSettingsWriteRequested(key, value)
    }

    // The generic sheet list excludes the hand-built sheets.
    readonly property var genericSheetIds: {
        var special = ["wifi", "bluetooth", "storage", "focus",
                       "screen-mirroring", "now-playing", "volume",
                       "brightness", "battery"];
        var out = [];
        for (var i = 0; i < root.registry.catalogueIds.length; ++i) {
            var id = root.registry.catalogueIds[i];
            if (special.indexOf(id) < 0 && id !== "edit-controls")
                out.push(id);
        }
        return out;
    }

    function setActivePage(page) {
        if (page < 0 || page >= root.pageCount)
            return;
        root.quickSettingsActivePage = page;
        root.quickSettingsWriteRequested("quickSettings.activePage", page);
    }

    // ── Components ───────────────────────────────────────────────────────
    component TextLink: Text {
        property string accessibleName: text
        signal activated()

        activeFocusOnTab: true
        Accessible.role: Accessible.Button
        Accessible.name: accessibleName
        Accessible.focusable: true
        Accessible.onPressAction: activated()
        color: linkHover.hovered ? root.moduleAccentHover : root.moduleAccent
        font.pixelSize: Theme.primitive.font.sizeSm

        HoverHandler { id: linkHover }

        TapHandler {
            onTapped: activated()
        }

        FocusRing {
            target: parent
            shown: parent.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                activated();
                event.accepted = true;
            }
        }
    }
}
