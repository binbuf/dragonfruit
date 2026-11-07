// SPDX-License-Identifier: MIT
import QtQuick

// The executable Settings business-logic map (T222).
//
// `docs/design/settings-business-logic-map.md` is the human contract. This
// singleton is its runtime projection: every pane in the catalog, the owner
// allowed to change its state, the absence path when that owner is missing,
// and the row-level owner/key/absence metadata the Settings debug view and the
// T223 search index consume.
//
// The `SettingsPanes.catalog` is the one ordered sidebar catalog; this file is
// the one place its owner, deep link, and row metadata come from. A pane is
// listed in the sidebar only when `shipped` is true, and a shipped pane must
// have a body (the no-half-panes rule). The map test asserts the two agree and
// fails on a visible row with no owner.
//
// Row `state`:
//   * `shipped` — the row exists in the current pane body and is visible.
//   * `planned` — the row is in the map but lands with T225–T229.
//   * `omitted` — the reference row is deliberately not shipped.
// A row may be visible but disabled with an explanation; it may never be a
// fake toggle. `absence` uses the five design-doc kinds plus `omitted`.
pragma Singleton

QtObject {
    id: root

    // The one human document this projection implements.
    readonly property string docPath:
        "docs/design/settings-business-logic-map.md"

    readonly property var absenceKinds: [
        "live-on-defaults", "disabled-with-reason", "hidden",
        "read-only", "delegated", "omitted"
    ]

    readonly property var rowStates: ["shipped", "planned", "omitted"]

    // The top-level pane catalog in map order. `storage`, `mission-control`,
    // and `menu-bar` are existing shipped panes that the reference groups
    // inside General/Desktop & Dock/Quick Settings; they keep their ids and
    // top-level placement (T222 design note). New panes use the map names;
    // user-facing titles follow the T220 generic naming map.
    readonly property var panes: [
        { id: "wifi", title: qsTr("Wi-Fi"), icon: "wifi", shipped: false,
          owner: "NetworkManager", disposition: "ship",
          deepLink: "settings://pane/wifi",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Wi-Fi is unavailable because NetworkManager is not running.") },
        { id: "bluetooth", title: qsTr("Bluetooth"), icon: "bluetooth", shipped: true,
          owner: "BlueZ", disposition: "ship",
          deepLink: "settings://pane/bluetooth",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Bluetooth is unavailable because its host service is not running.") },
        { id: "network", title: qsTr("Network"), icon: "network", shipped: true,
          owner: "NetworkManager + firewall adapter", disposition: "ship/adapt",
          deepLink: "settings://pane/network",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Network services are unavailable because NetworkManager is not running.") },
        { id: "battery", title: qsTr("Battery"), icon: "battery", shipped: true,
          owner: "UPower + power-profiles-daemon", disposition: "ship",
          deepLink: "settings://pane/battery",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Battery and power profiles are unavailable because their host services are not running.") },
        { id: "general", title: qsTr("General"), icon: "general", shipped: true,
          owner: "settingsd + host/distro adapters", disposition: "ship",
          deepLink: "settings://pane/general",
          absence: "live-on-defaults",
          absenceNote: qsTr("Durable preferences stay live; the host-stack rows show their own absence note.") },
        { id: "storage", title: qsTr("Storage"), icon: "storage", shipped: true,
          owner: "UDisks2 + filesystem usage", disposition: "ship",
          deepLink: "settings://pane/storage",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Storage is unavailable because UDisks2 is not running.") },
        { id: "appearance", title: qsTr("Appearance"), icon: "appearance", shipped: true,
          owner: "settingsd + compositor", disposition: "ship",
          deepLink: "settings://pane/appearance",
          absence: "live-on-defaults",
          absenceNote: qsTr("Appearance stays live on the schema defaults.") },
        { id: "accessibility", title: qsTr("Accessibility"), icon: "accessibility", shipped: true,
          owner: "settingsd + compositor + AT-SPI", disposition: "ship/delegate",
          deepLink: "settings://pane/accessibility",
          absence: "live-on-defaults",
          absenceNote: qsTr("Durable accessibility preferences stay live; the live status rows show their own absence note.") },
        { id: "control-center", title: qsTr("Quick Settings"), icon: "control-center", shipped: false,
          owner: "settingsd + shell + adapters", disposition: "ship",
          deepLink: "settings://pane/control-center",
          absence: "live-on-defaults",
          absenceNote: qsTr("Quick Settings customization stays live on the schema defaults.") },
        { id: "menu-bar", title: qsTr("Menu Bar"), icon: "menu-bar", shipped: true,
          owner: "settingsd -> shell", disposition: "ship",
          deepLink: "settings://pane/menu-bar",
          absence: "live-on-defaults",
          absenceNote: qsTr("Menu-bar preferences stay live on the schema defaults.") },
        { id: "search", title: qsTr("Search"), icon: "search", shipped: false,
          owner: "search service + app-index + settingsd", disposition: "adapt",
          deepLink: "settings://pane/search",
          absence: "live-on-defaults",
          absenceNote: qsTr("Search preferences stay live; the indexing rows show the service state.") },
        { id: "privacy", title: qsTr("Privacy & Security"), icon: "privacy", shipped: true,
          owner: "portals + polkit + disk encryption + firewall adapter", disposition: "ship/adapt",
          deepLink: "settings://pane/privacy",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Permissions are unavailable because the permission store is not running.") },
        { id: "desktop-dock", title: qsTr("Desktop & Dock"), icon: "dock", shipped: true,
          owner: "settingsd -> compositor/shell", disposition: "ship",
          deepLink: "settings://pane/desktop-dock",
          absence: "live-on-defaults",
          absenceNote: qsTr("Desktop and Dock preferences stay live on the schema defaults.") },
        { id: "mission-control", title: qsTr("Overview"), icon: "overview", shipped: true,
          owner: "settingsd + compositor", disposition: "ship",
          deepLink: "settings://pane/mission-control",
          absence: "live-on-defaults",
          absenceNote: qsTr("Overview preferences stay live on the schema defaults.") },
        { id: "displays", title: qsTr("Displays"), icon: "displays", shipped: true,
          owner: "compositor + colord/display adapter", disposition: "ship/adapt",
          deepLink: "settings://pane/displays",
          absence: "live-on-defaults",
          absenceNote: qsTr("Display preferences stay live; output rows show No display when none is present.") },
        { id: "wallpaper", title: qsTr("Wallpaper"), icon: "wallpaper", shipped: true,
          owner: "settingsd + wallpaperd + compositor", disposition: "ship",
          deepLink: "settings://pane/wallpaper",
          absence: "live-on-defaults",
          absenceNote: qsTr("The shipped default wallpaper is always available; Featured is empty, never an error.") },
        { id: "screen-saver", title: qsTr("Screen Saver"), icon: "displays", shipped: false,
          owner: "shell/compositor + settingsd", disposition: "ship",
          deepLink: "settings://pane/screen-saver",
          absence: "live-on-defaults",
          absenceNote: qsTr("Screen-saver preferences stay live on the schema defaults.") },
        { id: "notifications", title: qsTr("Notifications"), icon: "bell", shipped: true,
          owner: "notification service + settingsd", disposition: "ship",
          deepLink: "settings://pane/notifications",
          absence: "live-on-defaults",
          absenceNote: qsTr("Presentation preferences stay live; the per-app list shows its own absence note.") },
        { id: "sound", title: qsTr("Sound"), icon: "volume", shipped: true,
          owner: "PipeWire/WirePlumber + settingsd", disposition: "ship",
          deepLink: "settings://pane/sound",
          absence: "live-on-defaults",
          absenceNote: qsTr("Sound Effects and Balance stay live; device routing shows its own absence note.") },
        { id: "focus", title: qsTr("Focus"), icon: "focus", shipped: true,
          owner: "notification service + settingsd", disposition: "ship",
          deepLink: "settings://pane/focus",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Focus modes are unavailable because the notification service is not running.") },
        { id: "lock-screen", title: qsTr("Lock Screen"), icon: "lock", shipped: true,
          owner: "settingsd + compositor/lock + logind", disposition: "ship",
          deepLink: "settings://pane/lock-screen",
          absence: "live-on-defaults",
          absenceNote: qsTr("Lock-screen preferences stay live on the schema defaults.") },
        { id: "biometrics", title: qsTr("Biometrics & Password"), icon: "general", shipped: false,
          owner: "fprintd + PAM + distro adapter", disposition: "adapt",
          deepLink: "settings://pane/biometrics",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Biometrics are unavailable because no fingerprint service is running.") },
        { id: "users-groups", title: qsTr("Users & Groups"), icon: "users", shipped: true,
          owner: "accountsservice + distro adapter + polkit", disposition: "ship/adapt",
          deepLink: "settings://pane/users-groups",
          absence: "read-only",
          absenceNote: qsTr("The user list is read-only while the account service is unavailable.") },
        { id: "online-accounts", title: qsTr("Online Accounts"), icon: "network", shipped: false,
          owner: "Online Accounts service", disposition: "adapt/delegate",
          deepLink: "settings://pane/online-accounts",
          absence: "delegated",
          absenceNote: qsTr("Online accounts are not installed.") },
        { id: "printers", title: qsTr("Printers & Scanners"), icon: "printer", shipped: true,
          owner: "CUPS + SANE", disposition: "ship",
          deepLink: "settings://pane/printers",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Printers and scanners are unavailable because their host services are not running.") },
        { id: "keyboard", title: qsTr("Keyboard"), icon: "keyboard", shipped: true,
          owner: "settingsd + compositor input + ibus/fcitx5", disposition: "ship/adapt",
          deepLink: "settings://pane/keyboard",
          absence: "live-on-defaults",
          absenceNote: qsTr("Keyboard preferences stay live; the device inventory shows its own absence note.") },
        { id: "mouse", title: qsTr("Mouse"), icon: "mouse", shipped: true,
          owner: "settingsd + compositor input", disposition: "ship",
          deepLink: "settings://pane/mouse",
          absence: "live-on-defaults",
          absenceNote: qsTr("Pointer preferences stay live; the device inventory shows its own absence note.") },
        { id: "trackpad", title: qsTr("Trackpad"), icon: "trackpad", shipped: true,
          owner: "settingsd + compositor input (libinput)", disposition: "ship/adapt",
          deepLink: "settings://pane/trackpad",
          absence: "live-on-defaults",
          absenceNote: qsTr("Trackpad preferences stay live; the device inventory shows its own absence note.") },
        { id: "game-controllers", title: qsTr("Game Controllers"), icon: "input", shipped: false,
          owner: "udev/SDL", disposition: "adapt/delegate",
          deepLink: "settings://pane/game-controllers",
          absence: "disabled-with-reason",
          absenceNote: qsTr("No game controller service is available.") }
    ]

    // Reference panes that are deliberately not shipped. Their absence from
    // the catalog is the executable half; the reason is the documented half.
    readonly property var omittedPanes: [
        { id: "vendor-account", label: qsTr("Vendor account"),
          reason: qsTr("Vendor account service; no Linux equivalent.") },
        { id: "nearby-transfer", label: qsTr("Nearby Transfer"),
          reason: qsTr("No permissive drop-in; an optional third-party transfer service may follow.") },
        { id: "managed-devices", label: qsTr("Managed devices"),
          reason: qsTr("Mobile-device management is vendor-specific.") },
        { id: "usage-limits", label: qsTr("Usage limits"),
          reason: qsTr("Vendor parental controls; not shipped.") },
        { id: "game-service", label: qsTr("Game service"),
          reason: qsTr("Vendor game service; no equivalent.") },
        { id: "wallet", label: qsTr("Wallet"),
          reason: qsTr("Vendor payment service; not shipped.") },
        { id: "device-location", label: qsTr("Device location"),
          reason: qsTr("Vendor device-location service; not shipped.") }
    ]

    // The row constructor keeps the table compact and the field set uniform.
    function row(id, section, label, control, owner, key, absence, state) {
        return {
            id: id, section: section, label: label, control: control,
            owner: owner, key: key, absence: absence,
            state: state !== undefined ? state : "planned"
        };
    }

    readonly property var rows: root.buildRows()

    function buildRows() {
        return ({
            "wifi": [
                root.row("power", qsTr("Wi-Fi"), qsTr("Wi-Fi"), "toggle",
                         "NetworkManager", "WirelessEnabled", "disabled-with-reason"),
                root.row("current", qsTr("Wi-Fi"), qsTr("Current network"), "info+details",
                         "NetworkManager", "ActiveConnection", "disabled-with-reason"),
                root.row("saved", qsTr("Wi-Fi"), qsTr("Saved networks"), "list",
                         "NetworkManager", "Settings.Connection", "disabled-with-reason"),
                root.row("available", qsTr("Wi-Fi"), qsTr("Available networks"), "list",
                         "NetworkManager", "AccessPoint", "disabled-with-reason"),
                root.row("hidden", qsTr("Wi-Fi"), qsTr("Join hidden network..."), "dialog",
                         "NetworkManager", "AddAndActivateConnection", "disabled-with-reason"),
                root.row("ask", qsTr("Wi-Fi"), qsTr("Ask to join networks"), "popup",
                         "settingsd", "network.wifi.askToJoin", "live-on-defaults"),
                root.row("askHotspots", qsTr("Wi-Fi"), qsTr("Ask to join hotspots"), "popup",
                         "settingsd", "network.wifi.askToJoinHotspots", "live-on-defaults"),
                root.row("advanced", qsTr("Wi-Fi"), qsTr("Advanced..."), "sheet",
                         "NetworkManager + settingsd", "nm.connection", "disabled-with-reason"),
                root.row("quick", qsTr("Wi-Fi"), qsTr("Quick sheet"), "panel",
                         "shell", "quickSettings.wifi", "live-on-defaults")
            ],
            "bluetooth": [
                root.row("power", qsTr("Bluetooth"), qsTr("Bluetooth"), "toggle",
                         "BlueZ", "Adapter1.Powered", "disabled-with-reason", "shipped"),
                root.row("discoverable", qsTr("Bluetooth"), qsTr("Discoverable as..."), "info",
                         "BlueZ", "Adapter1.Discoverable", "disabled-with-reason", "shipped"),
                root.row("paired", qsTr("Bluetooth"), qsTr("Paired devices"), "list",
                         "BlueZ", "Device1", "disabled-with-reason", "shipped"),
                root.row("nearby", qsTr("Bluetooth"), qsTr("Nearby devices"), "list",
                         "BlueZ", "discovery", "disabled-with-reason", "shipped"),
                root.row("advanced", qsTr("Bluetooth"), qsTr("Advanced..."), "sheet",
                         "BlueZ + settingsd", "bluetooth.*", "disabled-with-reason"),
                root.row("quick", qsTr("Bluetooth"), qsTr("Quick sheet"), "panel",
                         "shell", "quickSettings.bluetooth", "live-on-defaults")
            ],
            "network": [
                root.row("wifi", qsTr("Network"), qsTr("Wi-Fi"), "disclosure",
                         "NetworkManager", "WirelessEnabled", "disabled-with-reason"),
                root.row("firewall", qsTr("Network"), qsTr("Firewall"), "disclosure",
                         "firewall adapter", "firewall.status", "disabled-with-reason"),
                root.row("services", qsTr("Network"), qsTr("Other services"), "list",
                         "NetworkManager", "Devices", "disabled-with-reason"),
                root.row("vpn", qsTr("Network"), qsTr("VPN"), "disclosure",
                         "NetworkManager", "Settings.Connection", "disabled-with-reason", "shipped"),
                root.row("actions", qsTr("Network"), qsTr("Service actions"), "menu",
                         "NetworkManager", "ActivateConnection", "disabled-with-reason"),
                root.row("advanced", qsTr("Network"), qsTr("Advanced..."), "sheet",
                         "NetworkManager", "nm.connection", "disabled-with-reason"),
                root.row("profiles", qsTr("Network"), qsTr("Profiles"), "popup",
                         "NetworkManager", "Settings.Connection", "disabled-with-reason"),
                root.row("firewallOptions", qsTr("Network"), qsTr("Firewall options"), "sheet",
                         "firewall adapter", "firewall.rules", "disabled-with-reason"),
                root.row("addService", qsTr("Network"), qsTr("Add service..."), "sheet",
                         "NetworkManager", "AddConnection", "disabled-with-reason")
            ],
            "battery": [
                root.row("profile", qsTr("Battery"), qsTr("Power mode"), "popup",
                         "power-profiles-daemon", "power.profile", "disabled-with-reason", "shipped"),
                root.row("health", qsTr("Battery"), qsTr("Battery health"), "info",
                         "UPower", "Battery.Health", "read-only", "shipped"),
                root.row("charging", qsTr("Battery"), qsTr("Charging"), "info",
                         "UPower", "Battery.State", "read-only", "shipped"),
                root.row("usage", qsTr("Battery"), qsTr("Usage range"), "segmented",
                         "settingsd + UPower", "power.historyRange", "live-on-defaults", "shipped"),
                root.row("lastCharged", qsTr("Battery"), qsTr("Last charged"), "info",
                         "UPower", "Battery.EnergyFull", "read-only", "shipped"),
                root.row("level", qsTr("Battery"), qsTr("Battery level"), "chart",
                         "UPower", "Battery.Percentage", "read-only", "shipped"),
                root.row("screenOn", qsTr("Battery"), qsTr("Screen-on usage"), "chart",
                         "settingsd/session", "power.screenOn", "live-on-defaults"),
                root.row("options", qsTr("Battery"), qsTr("Battery options"), "sheet",
                         "settingsd", "power.*", "live-on-defaults"),
                root.row("powerSettings", qsTr("Battery"), qsTr("Power settings"), "rows",
                         "settingsd", "idle.*", "live-on-defaults")
            ],
            "general": [
                root.row("about.deviceName", qsTr("About"), qsTr("Device name"), "text",
                         "hostname/distro adapter", "host.hostname", "read-only", "shipped"),
                root.row("about.hardware", qsTr("About"), qsTr("Processor, memory, serial, OS"), "info",
                         "host info", "host.info", "read-only", "shipped"),
                root.row("about.moreInfo", qsTr("About"), qsTr("System Info..."), "button",
                         "host info", "host.info", "read-only"),
                root.row("about.regulatory", qsTr("About"), qsTr("Regulatory info"), "link",
                         "distro/legal", "distro.regulatory", "delegated"),
                root.row("updates.auto", qsTr("Software Update"), qsTr("Automatic updates"), "toggle",
                         "PackageKit/distro adapter", "updates.auto", "disabled-with-reason"),
                root.row("updates.check", qsTr("Software Update"), qsTr("Check for updates"), "button",
                         "PackageKit", "updates.check", "disabled-with-reason", "shipped"),
                root.row("updates.install", qsTr("Software Update"), qsTr("Install updates"), "button",
                         "PackageKit", "updates.install", "disabled-with-reason", "shipped"),
                root.row("updates.options", qsTr("Software Update"), qsTr("Options..."), "sheet",
                         "distro adapter", "updates.options", "disabled-with-reason"),
                root.row("storage.categories", qsTr("Storage"), qsTr("Categories"), "chart/list",
                         "UDisks2 + filesystem usage", "storage.usage", "disabled-with-reason"),
                root.row("storage.manage", qsTr("Storage"), qsTr("Manage storage..."), "button",
                         "Files/Storage", "files.open", "delegated"),
                root.row("login.start", qsTr("Login Items"), qsTr("Start at login"), "list",
                         "XDG autostart + settingsd", "login.startup", "disabled-with-reason"),
                root.row("login.background", qsTr("Login Items"), qsTr("Background services"), "list",
                         "systemd user units + portals", "login.background", "disabled-with-reason"),
                root.row("login.extensions", qsTr("Login Items"), qsTr("Extensions"), "list",
                         "portal extensions", "portal.extensions", "delegated"),
                root.row("language.languages", qsTr("Language & Region"), qsTr("Languages"), "list",
                         "localectl/locale adapter", "locale.languages", "disabled-with-reason"),
                root.row("language.region", qsTr("Language & Region"), qsTr("Region"), "popup",
                         "locale adapter", "locale.region", "disabled-with-reason"),
                root.row("language.calendar", qsTr("Language & Region"), qsTr("Calendar"), "popup",
                         "settingsd", "locale.calendar", "live-on-defaults"),
                root.row("language.temperature", qsTr("Language & Region"), qsTr("Temperature"), "popup",
                         "settingsd", "locale.temperature", "live-on-defaults"),
                root.row("language.measurement", qsTr("Language & Region"), qsTr("Measurement"), "popup",
                         "settingsd", "locale.measurement", "live-on-defaults"),
                root.row("language.dateFormat", qsTr("Language & Region"), qsTr("Date format"), "popup",
                         "settingsd", "locale.dateFormat", "live-on-defaults"),
                root.row("language.numberFormat", qsTr("Language & Region"), qsTr("Number format"), "popup",
                         "settingsd", "locale.numberFormat", "live-on-defaults"),
                root.row("language.firstDay", qsTr("Language & Region"), qsTr("First day of week"), "popup",
                         "settingsd", "locale.firstDay", "live-on-defaults"),
                root.row("time.auto", qsTr("Date & Time"), qsTr("Set automatically"), "toggle",
                         "timedatectl/chrony adapter", "time.ntp", "disabled-with-reason"),
                root.row("time.zone", qsTr("Date & Time"), qsTr("Time zone"), "popup/map",
                         "timedatectl", "time.zone", "disabled-with-reason"),
                root.row("time.manual", qsTr("Date & Time"), qsTr("Date and time"), "fields",
                         "timedatectl", "time.set", "delegated"),
                root.row("time.hour24", qsTr("Date & Time"), qsTr("24-hour time"), "toggle",
                         "settingsd", "clock.use24Hour", "live-on-defaults"),
                root.row("time.seconds", qsTr("Date & Time"), qsTr("Show seconds"), "toggle",
                         "settingsd", "menu.clock.showSeconds", "live-on-defaults"),
                root.row("time.showDate", qsTr("Date & Time"), qsTr("Show date"), "toggle",
                         "settingsd", "menu.clock.showDate", "live-on-defaults"),
                root.row("time.options", qsTr("Date & Time"), qsTr("Clock options..."), "sheet",
                         "settingsd", "menu.clock.*", "live-on-defaults"),
                root.row("sharing.file", qsTr("Sharing"), qsTr("File sharing"), "toggle",
                         "Samba/NFS adapter", "sharing.file", "disabled-with-reason"),
                root.row("sharing.screen", qsTr("Sharing"), qsTr("Screen sharing"), "toggle",
                         "portal ScreenCast/RDP", "sharing.screen", "delegated"),
                root.row("sharing.media", qsTr("Sharing"), qsTr("Media sharing"), "toggle",
                         "DLNA adapter", "sharing.media", "hidden"),
                root.row("sharing.printer", qsTr("Sharing"), qsTr("Printer sharing"), "toggle",
                         "CUPS", "sharing.printer", "hidden"),
                root.row("sharing.remoteLogin", qsTr("Sharing"), qsTr("Remote login"), "toggle",
                         "OpenSSH adapter", "sharing.ssh", "disabled-with-reason"),
                root.row("sharing.remoteManagement", qsTr("Sharing"), qsTr("Remote management"), "toggle",
                         "distro adapter", "sharing.remote", "hidden"),
                root.row("sharing.internet", qsTr("Sharing"), qsTr("Internet sharing"), "toggle",
                         "NetworkManager", "sharing.internet", "hidden"),
                root.row("backup.auto", qsTr("Backup"), qsTr("Back up automatically"), "toggle",
                         "backup adapter", "backup.auto", "disabled-with-reason"),
                root.row("backup.now", qsTr("Backup"), qsTr("Back up now"), "button",
                         "backup adapter", "backup.now", "disabled-with-reason"),
                root.row("backup.destination", qsTr("Backup"), qsTr("Choose destination..."), "button",
                         "FileChooser portal", "backup.destination", "delegated"),
                root.row("backup.options", qsTr("Backup"), qsTr("Options..."), "sheet",
                         "backup adapter", "backup.options", "disabled-with-reason"),
                root.row("backup.restore", qsTr("Backup"), qsTr("Restore..."), "button",
                         "backup adapter", "backup.restore", "delegated"),
                root.row("reset.startupDisk", qsTr("Startup Disk"), qsTr("Startup disk"), "info",
                         "UDisks2/bootloader adapter", "storage.boot", "read-only"),
                root.row("reset.reset", qsTr("Reset"), qsTr("Reset"), "button",
                         "settingsd", "reset.settings", "live-on-defaults"),
                root.row("passwords.disclosure", qsTr("Passwords & Autofill"), qsTr("Passwords & autofill"), "disclosure",
                         "Secret Service/portal", "passwords.provider", "delegated"),
                root.row("passwords.open", qsTr("Passwords & Autofill"), qsTr("Passwords..."), "button",
                         "Secret Service", "passwords.open", "delegated"),
                root.row("passwords.source", qsTr("Passwords & Autofill"), qsTr("Autofill source"), "popup",
                         "settingsd", "defaults.passwordManager", "hidden"),
                root.row("passwords.keyring", qsTr("Passwords & Autofill"), qsTr("Locked keyring"), "info",
                         "Secret Service", "passwords.keyring", "delegated")
            ],
            "storage": [
                root.row("categories", qsTr("Storage"), qsTr("Categories"), "chart/list",
                         "UDisks2 + filesystem usage", "storage.usage", "disabled-with-reason"),
                root.row("manage", qsTr("Storage"), qsTr("Manage storage..."), "button",
                         "Files/Storage", "files.open", "delegated"),
                root.row("volumes", qsTr("Storage"), qsTr("Volumes"), "list",
                         "UDisks2", "storage.volumes", "disabled-with-reason", "shipped"),
                root.row("removable", qsTr("Storage"), qsTr("Removable media"), "list",
                         "UDisks2", "storage.drives", "disabled-with-reason", "shipped")
            ],
            "appearance": [
                root.row("scheme", qsTr("Appearance"), qsTr("Appearance"), "segmented",
                         "settingsd + compositor", "appearance.colorScheme", "live-on-defaults", "shipped"),
                root.row("accent", qsTr("Appearance"), qsTr("Accent color"), "swatches",
                         "settingsd", "appearance.accent", "live-on-defaults", "shipped"),
                root.row("highlight", qsTr("Appearance"), qsTr("Highlight color"), "popup",
                         "settingsd", "appearance.highlight", "live-on-defaults"),
                root.row("sidebarIconSize", qsTr("Appearance"), qsTr("Sidebar icon size"), "popup",
                         "settingsd", "appearance.sidebarIconSize", "live-on-defaults"),
                root.row("wallpaperTint", qsTr("Appearance"), qsTr("Wallpaper tinting"), "toggle",
                         "settingsd", "appearance.wallpaperTint", "live-on-defaults"),
                root.row("transparency", qsTr("Appearance"), qsTr("Transparency"), "slider",
                         "settingsd", "appearance.transparency", "live-on-defaults", "shipped"),
                root.row("scrollBars", qsTr("Appearance"), qsTr("Scroll bars"), "popup",
                         "settingsd", "appearance.scrollBars", "live-on-defaults"),
                root.row("motion", qsTr("Appearance"), qsTr("Motion"), "link",
                         "settingsd", "accessibility.reduceMotion", "live-on-defaults")
            ],
            "accessibility": [
                root.row("vision.screenReader", qsTr("Vision"), qsTr("Screen reader"), "toggle+open",
                         "Orca (AT-SPI)", "a11y.screenReader", "delegated", "shipped"),
                root.row("vision.magnifier", qsTr("Vision"), qsTr("Magnifier"), "toggle+options",
                         "compositor", "a11y.magnifier", "live-on-defaults"),
                root.row("vision.hoverText", qsTr("Vision"), qsTr("Hover text"), "toggle+options",
                         "shell overlay + settingsd", "a11y.hoverText", "live-on-defaults"),
                root.row("vision.display", qsTr("Vision"), qsTr("Display"), "disclosure",
                         "settingsd", "a11y.display", "live-on-defaults", "shipped"),
                root.row("vision.motion", qsTr("Vision"), qsTr("Motion"), "toggle",
                         "settingsd", "accessibility.reduceMotion", "live-on-defaults", "shipped"),
                root.row("vision.readSpeak", qsTr("Vision"), qsTr("Read & speak"), "disclosure",
                         "speech-dispatcher/espeak-ng/Piper", "a11y.readSpeak", "delegated"),
                root.row("vision.audioDescriptions", qsTr("Vision"), qsTr("Audio descriptions"), "omit",
                         "omit", "omit", "omitted", "omitted"),
                root.row("hearing.devices", qsTr("Hearing"), qsTr("Hearing devices"), "disclosure",
                         "BlueZ LE Audio/PipeWire", "a11y.hearingDevices", "delegated"),
                root.row("hearing.audio", qsTr("Hearing"), qsTr("Audio"), "disclosure",
                         "PipeWire + settingsd", "a11y.audio", "delegated"),
                root.row("hearing.captions", qsTr("Hearing"), qsTr("Captions"), "disclosure",
                         "live-caption delegate", "a11y.captions", "delegated"),
                root.row("hearing.liveCaptions", qsTr("Hearing"), qsTr("Live captions"), "toggle",
                         "speech-to-text delegate", "a11y.liveCaptions", "delegated"),
                root.row("hearing.nameRecognition", qsTr("Hearing"), qsTr("Name recognition"), "omit",
                         "omit", "omit", "omitted", "omitted"),
                root.row("motor.voiceControl", qsTr("Motor"), qsTr("Voice control"), "toggle",
                         "speech-to-text delegate", "a11y.voiceControl", "delegated"),
                root.row("motor.keyboard", qsTr("Motor"), qsTr("Keyboard"), "disclosure",
                         "settingsd + compositor", "a11y.keyboard", "live-on-defaults"),
                root.row("motor.pointer", qsTr("Motor"), qsTr("Pointer control"), "disclosure",
                         "compositor input", "a11y.pointer", "live-on-defaults"),
                root.row("motor.switchControl", qsTr("Motor"), qsTr("Switch control"), "disclosure",
                         "delegate", "a11y.switchControl", "delegated"),
                root.row("motor.shortcuts", qsTr("Motor"), qsTr("Accessibility shortcuts"), "sheet",
                         "settingsd", "a11y.shortcuts", "live-on-defaults")
            ],
            "control-center": [
                root.row("modules", qsTr("Quick Settings"), qsTr("Modules"), "list",
                         "settingsd", "menu.control.*", "live-on-defaults"),
                root.row("wifi", qsTr("Quick Settings"), qsTr("Wi-Fi"), "checkbox",
                         "settingsd", "menu.control.wifi", "live-on-defaults"),
                root.row("bluetooth", qsTr("Quick Settings"), qsTr("Bluetooth"), "checkbox",
                         "settingsd", "menu.control.bluetooth", "live-on-defaults"),
                root.row("focus", qsTr("Quick Settings"), qsTr("Focus"), "checkbox",
                         "settingsd", "menu.control.focus", "live-on-defaults"),
                root.row("screenMirroring", qsTr("Quick Settings"), qsTr("Screen casting"), "checkbox",
                         "settingsd", "menu.control.screenMirroring", "live-on-defaults"),
                root.row("nowPlaying", qsTr("Quick Settings"), qsTr("Now Playing"), "checkbox",
                         "settingsd", "menu.control.nowPlaying", "live-on-defaults"),
                root.row("sound", qsTr("Quick Settings"), qsTr("Sound"), "checkbox",
                         "settingsd", "menu.control.volume", "live-on-defaults"),
                root.row("display", qsTr("Quick Settings"), qsTr("Display"), "checkbox",
                         "settingsd", "menu.control.display", "live-on-defaults"),
                root.row("battery", qsTr("Quick Settings"), qsTr("Battery"), "checkbox",
                         "settingsd", "menu.control.battery", "live-on-defaults"),
                root.row("darkMode", qsTr("Quick Settings"), qsTr("Dark mode"), "checkbox",
                         "settingsd", "menu.control.darkMode", "live-on-defaults"),
                root.row("stage", qsTr("Quick Settings"), qsTr("Stage"), "checkbox",
                         "settingsd", "menu.control.stageManager", "live-on-defaults"),
                root.row("screenRecording", qsTr("Quick Settings"), qsTr("Screen recording"), "checkbox",
                         "settingsd", "menu.control.screenRecording", "live-on-defaults"),
                root.row("keyboardBrightness", qsTr("Quick Settings"), qsTr("Keyboard brightness"), "checkbox",
                         "settingsd", "menu.control.keyboardBrightness", "live-on-defaults"),
                root.row("hearing", qsTr("Quick Settings"), qsTr("Hearing"), "checkbox",
                         "settingsd", "menu.control.hearing", "live-on-defaults"),
                root.row("menuBarItems", qsTr("Quick Settings"), qsTr("Menu bar items"), "list",
                         "settingsd", "menu.control.*", "live-on-defaults"),
                root.row("otherModules", qsTr("Quick Settings"), qsTr("Other modules"), "list",
                         "settingsd", "menu.control.*", "live-on-defaults"),
                root.row("addControls", qsTr("Quick Settings"), qsTr("Add controls..."), "sheet",
                         "settingsd", "menu.control.*", "live-on-defaults")
            ],
            "menu-bar": [
                root.row("autoHide", qsTr("Menu Bar"), qsTr("Auto-hide menu bar"), "popup",
                         "settingsd", "menu.autoHide", "live-on-defaults", "shipped"),
                root.row("background", qsTr("Menu Bar"), qsTr("Show menu bar background"), "toggle",
                         "settingsd", "menu.showBackground", "live-on-defaults", "shipped"),
                root.row("recentItems", qsTr("Menu Bar"), qsTr("Recent items"), "stepper",
                         "settingsd", "menu.recentItems", "live-on-defaults", "shipped"),
                root.row("addControls", qsTr("Menu Bar"), qsTr("Add menu-bar controls..."), "sheet",
                         "settingsd + registry", "menu.control.*", "live-on-defaults"),
                root.row("clock", qsTr("Menu Bar"), qsTr("Clock"), "checkbox+options",
                         "settingsd", "menu.clock.showDate/showSeconds", "live-on-defaults", "shipped"),
                root.row("search", qsTr("Menu Bar"), qsTr("Search"), "checkbox",
                         "settingsd", "menu.control.search", "live-on-defaults"),
                root.row("wifi", qsTr("Menu Bar"), qsTr("Wi-Fi"), "checkbox",
                         "settingsd", "menu.control.wifi", "live-on-defaults", "shipped"),
                root.row("bluetooth", qsTr("Menu Bar"), qsTr("Bluetooth"), "checkbox",
                         "settingsd", "menu.control.bluetooth", "live-on-defaults", "shipped"),
                root.row("battery", qsTr("Menu Bar"), qsTr("Battery"), "checkbox+options",
                         "settingsd", "menu.control.battery", "live-on-defaults", "shipped"),
                root.row("focus", qsTr("Menu Bar"), qsTr("Focus"), "checkbox+popup",
                         "settingsd", "menu.control.focus", "live-on-defaults", "shipped"),
                root.row("screenMirroring", qsTr("Menu Bar"), qsTr("Screen casting"), "checkbox+popup",
                         "settingsd", "menu.control.screenMirroring", "live-on-defaults"),
                root.row("display", qsTr("Menu Bar"), qsTr("Display"), "checkbox+popup",
                         "settingsd", "menu.control.display", "live-on-defaults"),
                root.row("sound", qsTr("Menu Bar"), qsTr("Sound"), "checkbox+popup",
                         "settingsd", "menu.control.volume", "live-on-defaults", "shipped"),
                root.row("thirdParty", qsTr("Menu Bar"), qsTr("Third-party items"), "list",
                         "StatusNotifier registry", "tray.items", "delegated")
            ],
            "search": [
                root.row("results", qsTr("Search"), qsTr("Search results"), "checkboxes",
                         "search service", "search.categories", "live-on-defaults"),
                root.row("menuBar", qsTr("Search"), qsTr("Show Search in menu bar"), "toggle",
                         "settingsd", "menu.control.search", "live-on-defaults"),
                root.row("privacy", qsTr("Search"), qsTr("Search privacy"), "disclosure",
                         "search service", "search.indexLocations", "live-on-defaults"),
                root.row("shortcut", qsTr("Search"), qsTr("Shortcut"), "shortcut-recorder",
                         "settingsd", "search.shortcut", "live-on-defaults"),
                root.row("web", qsTr("Search"), qsTr("Web search"), "toggle",
                         "settingsd", "search.web", "live-on-defaults"),
                root.row("status", qsTr("Search"), qsTr("Indexing status"), "info",
                         "search service", "search.status", "disabled-with-reason"),
                root.row("rebuild", qsTr("Search"), qsTr("Rebuild index"), "button",
                         "search service", "search.rebuild", "disabled-with-reason")
            ],
            "privacy": [
                root.row("location", qsTr("Privacy"), qsTr("Location"), "disclosure",
                         "portal Location/Geoclue", "privacy.location", "delegated", "shipped"),
                root.row("contacts", qsTr("Privacy"), qsTr("Contacts"), "disclosure",
                         "portal/EDS", "privacy.contacts", "delegated", "shipped"),
                root.row("calendars", qsTr("Privacy"), qsTr("Calendars"), "disclosure",
                         "portal/EDS", "privacy.calendars", "delegated", "shipped"),
                root.row("reminders", qsTr("Privacy"), qsTr("Reminders"), "disclosure",
                         "portal/EDS", "privacy.reminders", "delegated", "shipped"),
                root.row("photos", qsTr("Privacy"), qsTr("Photos"), "disclosure",
                         "portal/files", "privacy.photos", "delegated", "shipped"),
                root.row("camera", qsTr("Privacy"), qsTr("Camera"), "disclosure",
                         "portal", "privacy.camera", "delegated", "shipped"),
                root.row("microphone", qsTr("Privacy"), qsTr("Microphone"), "disclosure",
                         "portal", "privacy.microphone", "delegated", "shipped"),
                root.row("speech", qsTr("Privacy"), qsTr("Speech recognition"), "disclosure",
                         "portal", "privacy.speech", "delegated", "shipped"),
                root.row("accessibility", qsTr("Privacy"), qsTr("Accessibility"), "disclosure",
                         "AT-SPI/portal", "privacy.accessibility", "delegated", "shipped"),
                root.row("fullDisk", qsTr("Privacy"), qsTr("Full disk access"), "disclosure",
                         "portal/polkit", "privacy.fullDisk", "delegated", "shipped"),
                root.row("files", qsTr("Privacy"), qsTr("Files and folders"), "disclosure",
                         "portal FileChooser/document portal", "privacy.files", "delegated", "shipped"),
                root.row("screenRecording", qsTr("Privacy"), qsTr("Screen recording"), "disclosure",
                         "portal ScreenCast", "privacy.screenRecording", "delegated", "shipped"),
                root.row("automation", qsTr("Privacy"), qsTr("Automation"), "disclosure",
                         "portal/D-Bus policy", "privacy.automation", "delegated", "shipped"),
                root.row("appManagement", qsTr("Privacy"), qsTr("App management"), "disclosure",
                         "package manager/portal", "privacy.appManagement", "delegated", "shipped"),
                root.row("developerTools", qsTr("Privacy"), qsTr("Developer tools"), "disclosure",
                         "portal", "privacy.developerTools", "delegated", "shipped"),
                root.row("appIntents", qsTr("Privacy"), qsTr("App intents"), "omit",
                         "omit", "omit", "omitted", "omitted"),
                root.row("sources", qsTr("Security"), qsTr("Software sources"), "popup",
                         "distro adapter", "security.sources", "disabled-with-reason"),
                root.row("encryption", qsTr("Security"), qsTr("Disk encryption"), "info",
                         "LUKS adapter", "security.luks", "read-only"),
                root.row("hardening", qsTr("Security"), qsTr("Hardening mode"), "toggle",
                         "settingsd/distro adapter", "security.hardening", "live-on-defaults"),
                root.row("extensions", qsTr("Security"), qsTr("Extensions"), "list",
                         "portal extensions", "security.extensions", "delegated"),
                root.row("advanced", qsTr("Security"), qsTr("Advanced..."), "sheet",
                         "adapters", "security.advanced", "disabled-with-reason")
            ],
            "desktop-dock": [
                root.row("size", qsTr("Dock"), qsTr("Size"), "slider",
                         "settingsd", "dock.size", "live-on-defaults", "shipped"),
                root.row("magnification", qsTr("Dock"), qsTr("Magnification"), "slider",
                         "settingsd", "dock.magnification", "live-on-defaults", "shipped"),
                root.row("position", qsTr("Dock"), qsTr("Dock position on screen"), "popup",
                         "settingsd", "dock.position", "live-on-defaults", "shipped"),
                root.row("minimizeAnimation", qsTr("Dock"), qsTr("Minimized window animation"), "popup",
                         "settingsd", "dock.minimizedAnimation", "live-on-defaults", "shipped"),
                root.row("titlebarDoubleClick", qsTr("Dock"), qsTr("Window title bar double-click action"), "popup",
                         "settingsd", "dock.titlebarDoubleClick", "live-on-defaults", "shipped"),
                root.row("minimizeIntoTileIcon", qsTr("Dock"), qsTr("Minimize windows into application icon"), "toggle",
                         "settingsd", "dock.minimizeIntoTileIcon", "live-on-defaults", "shipped"),
                root.row("autohide", qsTr("Dock"), qsTr("Automatically hide and show Dock"), "toggle",
                         "settingsd", "dock.autohide", "live-on-defaults", "shipped"),
                root.row("animateOpening", qsTr("Dock"), qsTr("Animate opening applications"), "toggle",
                         "settingsd", "dock.animateOpening", "live-on-defaults", "shipped"),
                root.row("showIndicators", qsTr("Dock"), qsTr("Show indicators for open applications"), "toggle",
                         "settingsd", "dock.showIndicators", "live-on-defaults", "shipped"),
                root.row("showRecentApps", qsTr("Dock"), qsTr("Show suggested and recent apps in Dock"), "toggle",
                         "settingsd", "dock.showRecentApps", "live-on-defaults", "shipped"),
                root.row("chooserOnHover", qsTr("Dock"), qsTr("Open a window chooser by hovering"), "toggle",
                         "settingsd", "dock.chooserOnHover", "live-on-defaults", "shipped"),
                root.row("minimizeReaction", qsTr("Dock"), qsTr("Bounce app icons when windows minimize"), "toggle",
                         "settingsd", "dock.minimizeReaction", "live-on-defaults", "shipped"),
                root.row("menuBarToggle", qsTr("Menu Bar"), qsTr("Show application menus in menu bar"), "toggle",
                         "settingsd", "menu.global", "live-on-defaults", "shipped"),
                root.row("desktopItems", qsTr("Desktop"), qsTr("Desktop items"), "segmented",
                         "settingsd", "desktop.showItems", "live-on-defaults"),
                root.row("clickWallpaper", qsTr("Desktop"), qsTr("Click wallpaper to reveal desktop"), "popup",
                         "settingsd", "desktop.clickWallpaper", "live-on-defaults"),
                root.row("stage", qsTr("Stage"), qsTr("Stage"), "toggle",
                         "settingsd", "stageManager.enabled", "live-on-defaults"),
                root.row("stageRecent", qsTr("Stage"), qsTr("Recent apps in Stage"), "toggle",
                         "settingsd", "stageManager.showRecentApps", "live-on-defaults"),
                root.row("widgets", qsTr("Desktop"), qsTr("Widgets"), "disclosure",
                         "settingsd", "widgets.*", "live-on-defaults"),
                root.row("overview", qsTr("Desktop"), qsTr("Overview"), "disclosure",
                         "settingsd", "overview.*", "live-on-defaults"),
                root.row("hotCorners", qsTr("Desktop"), qsTr("Hot corners..."), "sheet",
                         "settingsd", "overview.hotCorner*", "live-on-defaults"),
                root.row("shortcuts", qsTr("Desktop"), qsTr("Shortcuts..."), "sheet",
                         "settingsd", "shortcuts.*", "live-on-defaults"),
                root.row("windows", qsTr("Desktop"), qsTr("Windows"), "disclosure",
                         "settingsd", "tiling.*", "live-on-defaults"),
                root.row("defaultBrowser", qsTr("Desktop"), qsTr("Default browser"), "popup",
                         "settingsd + portal", "defaults.browser", "delegated")
            ],
            "mission-control": [
                root.row("swipeUp", qsTr("Overview"), qsTr("Swipe up to open"), "toggle",
                         "settingsd", "gestures.missionControl", "live-on-defaults", "shipped"),
                root.row("spaceSwitch", qsTr("Overview"), qsTr("Swipe between Workspaces"), "toggle",
                         "settingsd", "gestures.spaceSwitch", "live-on-defaults", "shipped"),
                root.row("topLeft", qsTr("Hot Corners"), qsTr("Top Left"), "popup",
                         "settingsd", "overview.hotCornerTopLeft", "live-on-defaults", "shipped"),
                root.row("topRight", qsTr("Hot Corners"), qsTr("Top Right"), "popup",
                         "settingsd", "overview.hotCornerTopRight", "live-on-defaults", "shipped"),
                root.row("bottomLeft", qsTr("Hot Corners"), qsTr("Bottom Left"), "popup",
                         "settingsd", "overview.hotCornerBottomLeft", "live-on-defaults", "shipped"),
                root.row("bottomRight", qsTr("Hot Corners"), qsTr("Bottom Right"), "popup",
                         "settingsd", "overview.hotCornerBottomRight", "live-on-defaults", "shipped")
            ],
            "displays": [
                root.row("preview", qsTr("Displays"), qsTr("Display preview"), "art",
                         "compositor", "display.preview", "live-on-defaults"),
                root.row("scale", qsTr("Displays"), qsTr("Scale/resolution"), "segmented",
                         "settingsd + compositor", "display.scale", "live-on-defaults", "shipped"),
                root.row("brightness", qsTr("Displays"), qsTr("Brightness"), "slider",
                         "settingsd", "display.brightness", "live-on-defaults"),
                root.row("autoBrightness", qsTr("Displays"), qsTr("Auto-brightness"), "toggle",
                         "settingsd", "display.autoBrightness", "hidden"),
                root.row("adaptiveColor", qsTr("Displays"), qsTr("Adaptive color"), "toggle",
                         "settingsd", "display.adaptiveColor", "hidden"),
                root.row("colorPreset", qsTr("Displays"), qsTr("Color preset"), "popup",
                         "colord", "display.colorPreset", "disabled-with-reason"),
                root.row("refreshRate", qsTr("Displays"), qsTr("Refresh rate"), "popup",
                         "compositor", "display.refreshRate", "live-on-defaults"),
                root.row("tvMode", qsTr("Displays"), qsTr("TV mode"), "popup",
                         "compositor", "display.tvMode", "live-on-defaults"),
                root.row("nightLight", qsTr("Displays"), qsTr("Night light"), "disclosure",
                         "settingsd", "display.nightLight", "live-on-defaults"),
                root.row("colorProfile", qsTr("Displays"), qsTr("Color profile"), "popup",
                         "colord", "display.colorProfile", "disabled-with-reason"),
                root.row("advanced", qsTr("Displays"), qsTr("Advanced..."), "sheet",
                         "compositor/colord", "display.advanced", "disabled-with-reason"),
                root.row("arrangement", qsTr("Displays"), qsTr("Arrangement"), "canvas",
                         "compositor", "display.arrangement", "live-on-defaults"),
                root.row("rotation", qsTr("Displays"), qsTr("Rotation"), "popup",
                         "settingsd", "display.rotation", "live-on-defaults", "shipped")
            ],
            "wallpaper": [
                root.row("current", qsTr("Wallpaper"), qsTr("Current wallpaper"), "preview",
                         "settingsd", "wallpaper.source", "live-on-defaults", "shipped"),
                root.row("showAll", qsTr("Wallpaper"), qsTr("Show on all workspaces"), "toggle",
                         "settingsd", "wallpaper.showOnAllSpaces", "live-on-defaults", "shipped"),
                root.row("fit", qsTr("Wallpaper"), qsTr("Fit"), "popup",
                         "settingsd", "wallpaper.fit", "live-on-defaults", "shipped"),
                root.row("screenSaver", qsTr("Wallpaper"), qsTr("Screen saver..."), "button",
                         "settingsd", "screenSaver.style", "live-on-defaults"),
                root.row("clock", qsTr("Wallpaper"), qsTr("Clock appearance..."), "button",
                         "settingsd", "lock.clock", "live-on-defaults"),
                root.row("dynamic", qsTr("Wallpaper"), qsTr("Dynamic"), "grid",
                         "wallpaperd", "wallpaper.dynamic", "disabled-with-reason"),
                root.row("photos", qsTr("Wallpaper"), qsTr("Your photos"), "button",
                         "FileChooser portal", "wallpaper.photo", "delegated", "shipped"),
                root.row("landscapes", qsTr("Wallpaper"), qsTr("Landscapes"), "grid",
                         "wallpaperd", "wallpaper.provider", "disabled-with-reason"),
                root.row("cityscapes", qsTr("Wallpaper"), qsTr("Cityscapes"), "grid",
                         "wallpaperd", "wallpaper.provider", "disabled-with-reason"),
                root.row("builtin", qsTr("Wallpaper"), qsTr("Built-in"), "grid",
                         "shipped assets", "wallpaper.builtinDefault", "live-on-defaults", "shipped")
            ],
            "screen-saver": [
                root.row("style", qsTr("Screen Saver"), qsTr("Screen saver"), "grid",
                         "settingsd/shell", "screenSaver.style", "live-on-defaults"),
                root.row("startAfter", qsTr("Screen Saver"), qsTr("Start after"), "popup",
                         "settingsd", "idle.blank", "live-on-defaults"),
                root.row("clock", qsTr("Screen Saver"), qsTr("Show with clock"), "toggle",
                         "settingsd", "screenSaver.clock", "live-on-defaults"),
                root.row("options", qsTr("Screen Saver"), qsTr("Options..."), "sheet",
                         "settingsd", "screenSaver.options", "live-on-defaults"),
                root.row("hotCorners", qsTr("Screen Saver"), qsTr("Hot corners..."), "sheet",
                         "settingsd", "overview.hotCorner*", "live-on-defaults")
            ],
            "notifications": [
                root.row("previews", qsTr("Notifications"), qsTr("Show previews"), "popup",
                         "settingsd", "notifications.showPreviews", "live-on-defaults", "shipped"),
                root.row("sleeping", qsTr("Notifications"), qsTr("When display is asleep"), "toggle",
                         "settingsd", "notifications.showWhenSleeping", "live-on-defaults", "shipped"),
                root.row("locked", qsTr("Notifications"), qsTr("When locked"), "toggle",
                         "settingsd", "notifications.showWhenLocked", "live-on-defaults", "shipped"),
                root.row("sharing", qsTr("Notifications"), qsTr("When sharing"), "popup",
                         "settingsd", "notifications.showWhenMirroring", "live-on-defaults", "shipped"),
                root.row("apps", qsTr("Notifications"), qsTr("App notifications"), "list",
                         "notification service", "notifications.apps", "disabled-with-reason", "shipped"),
                root.row("allow", qsTr("Notifications"), qsTr("Allow"), "toggle",
                         "notification service", "notifications.allow", "disabled-with-reason", "shipped"),
                root.row("style", qsTr("Notifications"), qsTr("Style"), "popup",
                         "notification service", "notifications.style", "disabled-with-reason", "shipped"),
                root.row("sound", qsTr("Notifications"), qsTr("Sound"), "toggle",
                         "notification service", "notifications.sound", "disabled-with-reason", "shipped"),
                root.row("badge", qsTr("Notifications"), qsTr("Badge"), "toggle",
                         "notification service", "notifications.badge", "disabled-with-reason", "shipped"),
                root.row("focus", qsTr("Notifications"), qsTr("Focus"), "link",
                         "settingsd", "notifications.focus", "live-on-defaults")
            ],
            "sound": [
                root.row("alertSound", qsTr("Sound Effects"), qsTr("Alert sound"), "popup+play",
                         "settingsd", "sound.alertSound", "live-on-defaults", "shipped"),
                root.row("effectsThrough", qsTr("Sound Effects"), qsTr("Play effects through"), "popup",
                         "settingsd", "sound.playEffectsThrough", "live-on-defaults", "shipped"),
                root.row("alertVolume", qsTr("Sound Effects"), qsTr("Alert volume"), "slider",
                         "settingsd", "sound.alertVolume", "live-on-defaults", "shipped"),
                root.row("startup", qsTr("Sound Effects"), qsTr("Startup sound"), "toggle",
                         "settingsd", "sound.playOnStartup", "live-on-defaults", "shipped"),
                root.row("uiEffects", qsTr("Sound Effects"), qsTr("UI sounds"), "toggle",
                         "settingsd", "sound.uiEffects", "live-on-defaults", "shipped"),
                root.row("volumeFeedback", qsTr("Sound Effects"), qsTr("Volume feedback"), "toggle",
                         "settingsd", "sound.volumeFeedback", "live-on-defaults", "shipped"),
                root.row("output", qsTr("Output & Input"), qsTr("Output"), "tab",
                         "PipeWire", "audio.sinks", "disabled-with-reason", "shipped"),
                root.row("input", qsTr("Output & Input"), qsTr("Input"), "tab",
                         "PipeWire", "audio.sources", "disabled-with-reason", "shipped"),
                root.row("test", qsTr("Output & Input"), qsTr("Test"), "button",
                         "PipeWire", "audio.test", "disabled-with-reason")
            ],
            "focus": [
                root.row("dnd", qsTr("Focus"), qsTr("Do Not Disturb"), "toggle",
                         "notification service", "focus.dnd", "disabled-with-reason", "shipped"),
                root.row("modes", qsTr("Focus"), qsTr("Modes"), "list",
                         "notification service", "focus.modes", "disabled-with-reason", "shipped"),
                root.row("people", qsTr("Focus"), qsTr("Allowed people"), "list",
                         "notification service", "focus.people", "disabled-with-reason"),
                root.row("apps", qsTr("Focus"), qsTr("Allowed apps"), "list",
                         "notification service", "focus.apps", "disabled-with-reason", "shipped"),
                root.row("schedule", qsTr("Focus"), qsTr("Schedule"), "sheet",
                         "notification service", "focus.schedule", "disabled-with-reason"),
                root.row("shortcut", qsTr("Focus"), qsTr("Shortcut"), "recorder",
                         "settingsd", "focus.shortcut", "live-on-defaults")
            ],
            "lock-screen": [
                root.row("displayBattery", qsTr("Lock Screen"), qsTr("Display off on battery"), "popup",
                         "settingsd", "idle.blank", "live-on-defaults", "shipped"),
                root.row("displayAC", qsTr("Lock Screen"), qsTr("Display off on AC"), "popup",
                         "settingsd", "idle.blank", "live-on-defaults"),
                root.row("requirePassword", qsTr("Lock Screen"), qsTr("Require password"), "popup",
                         "settingsd", "idle.lock", "live-on-defaults", "shipped"),
                root.row("userNamePhoto", qsTr("Lock Screen"), qsTr("Show user name/photo"), "toggle",
                         "settingsd", "lock.showUserNameAndPhoto", "live-on-defaults", "shipped"),
                root.row("passwordHints", qsTr("Lock Screen"), qsTr("Show password hints"), "toggle",
                         "settingsd", "lock.showPasswordHints", "live-on-defaults", "shipped"),
                root.row("message", qsTr("Lock Screen"), qsTr("Lock message"), "toggle+set",
                         "settingsd", "lock.showMessageWhenLocked/lock.message", "live-on-defaults", "shipped"),
                root.row("loginWindow", qsTr("Lock Screen"), qsTr("Login window"), "radio",
                         "settingsd", "login.windowStyle", "live-on-defaults"),
                root.row("powerButtons", qsTr("Lock Screen"), qsTr("Power buttons"), "toggle",
                         "settingsd", "lock.showPowerButtons", "live-on-defaults", "shipped"),
                root.row("accessibility", qsTr("Lock Screen"), qsTr("Accessibility options..."), "button",
                         "settingsd", "a11y.lockOptions", "live-on-defaults")
            ],
            "biometrics": [
                root.row("password", qsTr("Password"), qsTr("Change password..."), "button",
                         "PAM/distro adapter", "auth.changePassword", "delegated"),
                root.row("fingerprints", qsTr("Fingerprints"), qsTr("Fingerprints"), "list+add",
                         "fprintd", "biometrics.fingerprints", "disabled-with-reason"),
                root.row("unlock", qsTr("Fingerprints"), qsTr("Use fingerprint to unlock"), "toggle",
                         "PAM + settingsd", "biometrics.unlock", "disabled-with-reason"),
                root.row("auth", qsTr("Fingerprints"), qsTr("Use fingerprint for authentication"), "toggle",
                         "PAM", "biometrics.auth", "disabled-with-reason"),
                root.row("userSwitching", qsTr("Fingerprints"), qsTr("Fingerprint for user switching"), "toggle",
                         "PAM/display manager", "biometrics.userSwitching", "disabled-with-reason"),
                root.row("purchases", qsTr("Fingerprints"), qsTr("Fingerprint purchases"), "omit",
                         "omit", "omit", "omitted", "omitted")
            ],
            "users-groups": [
                root.row("users", qsTr("Users & Groups"), qsTr("Users"), "list",
                         "accountsservice", "users.list", "read-only", "shipped"),
                root.row("guest", qsTr("Users & Groups"), qsTr("Guest user"), "toggle",
                         "distro adapter", "users.guest", "hidden"),
                root.row("addUser", qsTr("Users & Groups"), qsTr("Add user..."), "sheet",
                         "distro adapter", "users.add", "disabled-with-reason", "shipped"),
                root.row("addGroup", qsTr("Users & Groups"), qsTr("Add group..."), "sheet",
                         "distro adapter", "users.addGroup", "disabled-with-reason", "shipped"),
                root.row("autoLogin", qsTr("Users & Groups"), qsTr("Automatic login"), "popup",
                         "display-manager adapter", "users.autologin", "disabled-with-reason", "shipped"),
                root.row("directory", qsTr("Users & Groups"), qsTr("Directory account"), "button",
                         "SSSD/FreeIPA adapter", "users.directory", "delegated"),
                root.row("adminGroup", qsTr("Users & Groups"), qsTr("Admin group"), "info",
                         "distro adapter", "users.adminGroup", "read-only", "shipped")
            ],
            "online-accounts": [
                root.row("list", qsTr("Online Accounts"), qsTr("Online accounts"), "list",
                         "Online Accounts service", "onlineAccounts.list", "delegated"),
                root.row("add", qsTr("Online Accounts"), qsTr("Add account..."), "sheet",
                         "Online Accounts service", "onlineAccounts.add", "delegated"),
                root.row("sync", qsTr("Online Accounts"), qsTr("Sync toggles"), "toggles",
                         "Online Accounts service", "onlineAccounts.sync", "delegated"),
                root.row("remove", qsTr("Online Accounts"), qsTr("Remove account"), "button",
                         "Online Accounts service", "onlineAccounts.remove", "delegated")
            ],
            "printers": [
                root.row("defaultPrinter", qsTr("Printers & Scanners"), qsTr("Default printer"), "popup",
                         "CUPS", "printers.default", "disabled-with-reason", "shipped"),
                root.row("paperSize", qsTr("Printers & Scanners"), qsTr("Default paper size"), "popup",
                         "settingsd", "printers.defaultPaperSize", "live-on-defaults", "shipped"),
                root.row("list", qsTr("Printers & Scanners"), qsTr("Printers"), "list",
                         "CUPS", "printers.list", "disabled-with-reason", "shipped"),
                root.row("add", qsTr("Printers & Scanners"), qsTr("Add printer or scanner..."), "button",
                         "CUPS/SANE", "printers.add", "disabled-with-reason", "shipped"),
                root.row("options", qsTr("Printers & Scanners"), qsTr("Options & supplies..."), "sheet",
                         "CUPS", "printers.options", "disabled-with-reason"),
                root.row("scanners", qsTr("Printers & Scanners"), qsTr("Scanners"), "list",
                         "SANE", "printers.scanners", "disabled-with-reason", "shipped")
            ],
            "keyboard": [
                root.row("repeatRate", qsTr("Keyboard"), qsTr("Key repeat rate"), "slider",
                         "settingsd", "input.repeatRate", "live-on-defaults", "shipped"),
                root.row("repeatDelay", qsTr("Keyboard"), qsTr("Repeat delay"), "slider",
                         "settingsd", "input.repeatDelay", "live-on-defaults", "shipped"),
                root.row("autoBrightness", qsTr("Keyboard"), qsTr("Auto keyboard brightness"), "toggle",
                         "settingsd", "input.adjustBrightnessLowLight", "live-on-defaults", "shipped"),
                root.row("brightness", qsTr("Keyboard"), qsTr("Keyboard brightness"), "slider",
                         "settingsd", "input.keyboardBrightness", "live-on-defaults", "shipped"),
                root.row("backlightOff", qsTr("Keyboard"), qsTr("Backlight off after"), "popup",
                         "settingsd", "input.backlightOffAfter", "live-on-defaults", "shipped"),
                root.row("superKey", qsTr("Keyboard"), qsTr("Press Super key to"), "popup",
                         "settingsd", "input.emojiKeyAction", "live-on-defaults", "shipped"),
                root.row("navigation", qsTr("Keyboard"), qsTr("Keyboard navigation"), "toggle",
                         "settingsd", "input.keyboardNavigation", "live-on-defaults", "shipped"),
                root.row("shortcuts", qsTr("Keyboard"), qsTr("Keyboard shortcuts..."), "sheet",
                         "settingsd", "input.shortcuts", "live-on-defaults"),
                root.row("sources", qsTr("Keyboard"), qsTr("Input sources"), "popup+edit",
                         "ibus/fcitx5 + localectl", "input.sources", "delegated"),
                root.row("replacements", qsTr("Keyboard"), qsTr("Text replacements..."), "sheet",
                         "settingsd/IME", "input.replacements", "live-on-defaults"),
                root.row("dictation", qsTr("Keyboard"), qsTr("Dictation"), "toggle+options",
                         "speech-to-text delegate", "input.dictation", "delegated")
            ],
            "mouse": [
                root.row("speed", qsTr("Mouse"), qsTr("Pointer speed"), "slider",
                         "settingsd", "input.pointerSpeed", "live-on-defaults", "shipped"),
                root.row("natural", qsTr("Mouse"), qsTr("Natural scrolling"), "toggle",
                         "settingsd", "input.naturalScroll", "live-on-defaults", "shipped"),
                root.row("secondary", qsTr("Mouse"), qsTr("Secondary click"), "popup",
                         "settingsd", "input.secondaryClick", "live-on-defaults"),
                root.row("scrollSpeed", qsTr("Mouse"), qsTr("Scroll speed"), "slider",
                         "settingsd", "input.scrollSpeed", "live-on-defaults"),
                root.row("doubleClick", qsTr("Mouse"), qsTr("Double-click speed"), "slider",
                         "settingsd", "input.doubleClickSpeed", "live-on-defaults"),
                root.row("leftHanded", qsTr("Mouse"), qsTr("Left-handed"), "toggle",
                         "settingsd", "input.leftHanded", "live-on-defaults", "shipped"),
                root.row("scrollMethod", qsTr("Mouse"), qsTr("Scroll method"), "popup",
                         "settingsd", "input.scrollMethod", "live-on-defaults", "shipped")
            ],
            "trackpad": [
                root.row("speed", qsTr("Trackpad"), qsTr("Pointer speed"), "slider",
                         "settingsd", "input.pointerSpeed", "live-on-defaults", "shipped"),
                root.row("click", qsTr("Trackpad"), qsTr("Click pressure"), "slider",
                         "settingsd", "input.tapToClick", "live-on-defaults"),
                root.row("haptics", qsTr("Trackpad"), qsTr("Haptic feedback"), "toggle",
                         "settingsd", "input.haptics", "hidden"),
                root.row("lookUp", qsTr("Trackpad"), qsTr("Look up"), "popup",
                         "settingsd", "input.lookUp", "live-on-defaults"),
                root.row("secondary", qsTr("Trackpad"), qsTr("Secondary click"), "popup",
                         "settingsd", "input.secondaryClick", "live-on-defaults"),
                root.row("tap", qsTr("Trackpad"), qsTr("Tap to click"), "toggle",
                         "settingsd", "input.tapToClick", "live-on-defaults", "shipped"),
                root.row("scrollZoom", qsTr("Trackpad"), qsTr("Scroll & zoom"), "tab",
                         "settingsd", "input.naturalScroll/input.pinchZoom", "live-on-defaults", "shipped"),
                root.row("gestures", qsTr("Trackpad"), qsTr("More gestures"), "tab",
                         "settingsd", "gestures.*", "live-on-defaults", "shipped"),
                root.row("pair", qsTr("Trackpad"), qsTr("Pair trackpad..."), "button",
                         "BlueZ", "input.pairTrackpad", "disabled-with-reason")
            ],
            "game-controllers": [
                root.row("list", qsTr("Game Controllers"), qsTr("Controllers"), "list",
                         "udev/SDL", "controllers.list", "disabled-with-reason"),
                root.row("mapping", qsTr("Game Controllers"), qsTr("Mapping"), "sheet",
                         "SDL mapping", "controllers.mapping", "delegated"),
                root.row("test", qsTr("Game Controllers"), qsTr("Test"), "button",
                         "SDL", "controllers.test", "disabled-with-reason")
            ]
        });
    }

    function pane(id) {
        for (var i = 0; i < root.panes.length; ++i) {
            if (root.panes[i].id === id)
                return root.panes[i];
        }
        return null;
    }

    function paneIndex(id) {
        for (var i = 0; i < root.panes.length; ++i) {
            if (root.panes[i].id === id)
                return i;
        }
        return -1;
    }

    function rowsFor(paneId) {
        return root.rows[paneId] !== undefined ? root.rows[paneId] : [];
    }

    function rowFor(paneId, rowId) {
        var list = root.rowsFor(paneId);
        for (var i = 0; i < list.length; ++i) {
            if (list[i].id === rowId)
                return list[i];
        }
        return null;
    }

    function visibleRowsFor(paneId) {
        return root.rowsFor(paneId).filter(function(entry) {
            return entry.state === "shipped";
        });
    }

    function rowDeepLink(paneId, rowId) {
        return "settings://pane/" + paneId + "/row/" + rowId;
    }

    // A row is valid when it names an owner and an absence path. An omitted
    // row is valid only with the explicit `omitted` state/absence pair.
    function validateRow(entry) {
        var errors = [];
        if (entry === null || entry === undefined) {
            errors.push("row is missing");
            return errors;
        }
        if (!entry.id || String(entry.id).length === 0)
            errors.push("row id is missing");
        if (!entry.label || String(entry.label).length === 0)
            errors.push("row " + (entry.id || "?") + " has no label");
        if (root.rowStates.indexOf(entry.state) < 0)
            errors.push("row " + entry.id + " has unknown state " + entry.state);
        if (root.absenceKinds.indexOf(entry.absence) < 0)
            errors.push("row " + entry.id + " has unknown absence " + entry.absence);
        if (entry.state !== "omitted") {
            if (!entry.owner || String(entry.owner).length === 0
                    || entry.owner === "omit")
                errors.push("row " + entry.id + " has no owner");
            if (!entry.key || String(entry.key).length === 0
                    || entry.key === "omit")
                errors.push("row " + entry.id + " has no key/API");
        }
        return errors;
    }

    // Validate the whole executable map. The test calls this and also feeds it
    // an intentionally unowned row to prove the gate fails.
    function validate() {
        var errors = [];
        var seen = {};
        for (var i = 0; i < root.panes.length; ++i) {
            var pane = root.panes[i];
            if (!pane.id || !pane.title || !pane.icon)
                errors.push("pane " + i + " is missing id/title/icon");
            if (seen[pane.id] === true)
                errors.push("duplicate pane id " + pane.id);
            seen[pane.id] = true;
            if (!pane.owner || String(pane.owner).length === 0)
                errors.push("pane " + pane.id + " has no owner");
            if (!pane.deepLink || String(pane.deepLink).length === 0)
                errors.push("pane " + pane.id + " has no deep link");
            if (root.absenceKinds.indexOf(pane.absence) < 0)
                errors.push("pane " + pane.id + " has unknown absence " + pane.absence);
            var list = root.rowsFor(pane.id);
            if (pane.shipped && list.filter(function(entry) {
                    return entry.state === "shipped";
                }).length === 0)
                errors.push("shipped pane " + pane.id + " has no shipped rows");
            for (var r = 0; r < list.length; ++r)
                errors = errors.concat(root.validateRow(list[r]));
        }
        for (var o = 0; o < root.omittedPanes.length; ++o) {
            var omitted = root.omittedPanes[o];
            if (!omitted.id || !omitted.label || !omitted.reason)
                errors.push("omitted pane " + o + " is not documented");
            if (seen[omitted.id] === true)
                errors.push("omitted pane " + omitted.id + " is in the catalog");
        }
        return errors;
    }
}
