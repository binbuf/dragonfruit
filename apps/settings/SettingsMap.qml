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

    // The top-level pane catalog in map order (T285 reduced IA). The order is
    // the mockup sidebar order: Connectivity, Desktop, Input. `storage` stays
    // a shipped pane body but is hidden from the sidebar and aliased to
    // General -> Storage. `mission-control` and `control-center` are no longer
    // top-level panes; their ids are documented in `aliases`/`removed`.
    readonly property var panes: [
        { id: "wifi", title: qsTr("Wi-Fi"), icon: "wifi", group: "connectivity",
          shipped: true,
          owner: "NetworkManager", disposition: "ship",
          deepLink: "settings://pane/wifi",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Wi-Fi is unavailable because NetworkManager is not running.") },
        { id: "bluetooth", title: qsTr("Bluetooth"), icon: "bluetooth", group: "connectivity",
          shipped: true,
          owner: "BlueZ", disposition: "ship",
          deepLink: "settings://pane/bluetooth",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Bluetooth is unavailable because its host service is not running.") },
        { id: "network", title: qsTr("Network"), icon: "network", group: "connectivity",
          shipped: true,
          owner: "NetworkManager + firewall adapter", disposition: "ship/adapt",
          deepLink: "settings://pane/network",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Network services are unavailable because NetworkManager is not running.") },
        { id: "vpn", title: qsTr("VPN"), icon: "vpn", group: "connectivity",
          shipped: true,
          owner: "NetworkManager VPN", disposition: "ship/adapt",
          deepLink: "settings://pane/vpn",
          absence: "disabled-with-reason",
          absenceNote: qsTr("VPN is unavailable because NetworkManager is not running.") },
        // The visible label is Energy; the id stays `battery` (ADR 0300).
        { id: "battery", title: qsTr("Energy"), icon: "battery", group: "connectivity",
          shipped: true,
          owner: "UPower + power-profiles-daemon", disposition: "ship",
          deepLink: "settings://pane/battery",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Energy and power profiles are unavailable because their host services are not running.") },
        { id: "general", title: qsTr("General"), icon: "general", group: "desktop",
          shipped: true,
          owner: "settingsd + host/distro adapters", disposition: "ship",
          deepLink: "settings://pane/general",
          absence: "live-on-defaults",
          absenceNote: qsTr("Durable preferences stay live; the host-stack rows show their own absence note.") },
        { id: "storage", title: qsTr("Storage"), icon: "storage", group: "desktop",
          shipped: true, hidden: true,
          owner: "UDisks2 + filesystem usage", disposition: "ship",
          deepLink: "settings://pane/storage",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Storage is unavailable because UDisks2 is not running.") },
        { id: "accessibility", title: qsTr("Accessibility"), icon: "accessibility", group: "desktop",
          shipped: true,
          owner: "settingsd + compositor + AT-SPI", disposition: "ship/delegate",
          deepLink: "settings://pane/accessibility",
          absence: "live-on-defaults",
          absenceNote: qsTr("Durable accessibility preferences stay live; the live status rows show their own absence note.") },
        { id: "appearance", title: qsTr("Appearance"), icon: "appearance", group: "desktop",
          shipped: true,
          owner: "settingsd + compositor", disposition: "ship",
          deepLink: "settings://pane/appearance",
          absence: "live-on-defaults",
          absenceNote: qsTr("Appearance stays live on the schema defaults.") },
        { id: "desktop-dock", title: qsTr("Desktop & Dock"), icon: "dock", group: "desktop",
          shipped: true,
          owner: "settingsd -> compositor/shell", disposition: "ship",
          deepLink: "settings://pane/desktop-dock",
          absence: "live-on-defaults",
          absenceNote: qsTr("Desktop and Dock preferences stay live on the schema defaults.") },
        { id: "displays", title: qsTr("Displays"), icon: "displays", group: "desktop",
          shipped: true,
          owner: "compositor + colord/display adapter", disposition: "ship/adapt",
          deepLink: "settings://pane/displays",
          absence: "live-on-defaults",
          absenceNote: qsTr("Display preferences stay live; output rows show No display when none is present.") },
        { id: "menu-bar", title: qsTr("Menu Bar"), icon: "menu-bar", group: "desktop",
          shipped: true,
          owner: "settingsd -> shell", disposition: "ship",
          deepLink: "settings://pane/menu-bar",
          absence: "live-on-defaults",
          absenceNote: qsTr("Menu-bar preferences stay live on the schema defaults.") },
        { id: "search", title: qsTr("Search"), icon: "search", group: "desktop",
          shipped: true,
          owner: "search service + app-index + settingsd", disposition: "adapt",
          deepLink: "settings://pane/search",
          absence: "live-on-defaults",
          absenceNote: qsTr("Search preferences stay live; the indexing rows show the service state.") },
        { id: "wallpaper", title: qsTr("Wallpaper"), icon: "wallpaper", group: "desktop",
          shipped: true,
          owner: "settingsd + wallpaperd + compositor", disposition: "ship",
          deepLink: "settings://pane/wallpaper",
          absence: "live-on-defaults",
          absenceNote: qsTr("The shipped default wallpaper is always available; Featured is empty, never an error.") },
        { id: "screen-saver", title: qsTr("Screen Saver"), icon: "screen-saver", group: "desktop",
          shipped: true,
          owner: "shell/compositor + settingsd", disposition: "ship",
          deepLink: "settings://pane/screen-saver",
          absence: "live-on-defaults",
          absenceNote: qsTr("Screen-saver preferences stay live on the schema defaults.") },
        { id: "notifications", title: qsTr("Notifications"), icon: "bell", group: "input",
          shipped: true,
          owner: "notification service + settingsd", disposition: "ship",
          deepLink: "settings://pane/notifications",
          absence: "live-on-defaults",
          absenceNote: qsTr("Presentation preferences stay live; the per-app list shows its own absence note.") },
        { id: "sound", title: qsTr("Sound"), icon: "volume", group: "input",
          shipped: true,
          owner: "PipeWire/WirePlumber + settingsd", disposition: "ship",
          deepLink: "settings://pane/sound",
          absence: "live-on-defaults",
          absenceNote: qsTr("Sound Effects and Balance stay live; device routing shows its own absence note.") },
        { id: "focus", title: qsTr("Focus"), icon: "focus", group: "input",
          shipped: true,
          owner: "notification service + settingsd", disposition: "ship",
          deepLink: "settings://pane/focus",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Focus modes are unavailable because the notification service is not running.") },
        { id: "lock-screen", title: qsTr("Lock Screen"), icon: "lock", group: "input",
          shipped: true,
          owner: "settingsd + compositor/lock + logind", disposition: "ship",
          deepLink: "settings://pane/lock-screen",
          absence: "live-on-defaults",
          absenceNote: qsTr("Lock-screen preferences stay live on the schema defaults.") },
        { id: "privacy", title: qsTr("Privacy & Security"), icon: "privacy", group: "input",
          shipped: true,
          owner: "portals + polkit + disk encryption + firewall adapter", disposition: "ship/adapt",
          deepLink: "settings://pane/privacy",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Permissions are unavailable because the permission store is not running.") },
        { id: "users-groups", title: qsTr("Users & Groups"), icon: "users", group: "input",
          shipped: true,
          owner: "accountsservice + distro adapter + polkit", disposition: "ship/adapt",
          deepLink: "settings://pane/users-groups",
          absence: "read-only",
          absenceNote: qsTr("The user list is read-only while the account service is unavailable.") },
        { id: "online-accounts", title: qsTr("Online Accounts"), icon: "online-accounts", group: "input",
          shipped: true,
          owner: "Online Accounts service", disposition: "adapt/delegate",
          deepLink: "settings://pane/online-accounts",
          absence: "delegated",
          absenceNote: qsTr("Online accounts are not installed.") },
        { id: "keyboard", title: qsTr("Keyboard"), icon: "keyboard", group: "input",
          shipped: true,
          owner: "settingsd + compositor input + ibus/fcitx5", disposition: "ship/adapt",
          deepLink: "settings://pane/keyboard",
          absence: "live-on-defaults",
          absenceNote: qsTr("Keyboard preferences stay live; the device inventory shows its own absence note.") },
        { id: "mouse", title: qsTr("Mouse"), icon: "mouse", group: "input",
          shipped: true,
          owner: "settingsd + compositor input", disposition: "ship",
          deepLink: "settings://pane/mouse",
          absence: "live-on-defaults",
          absenceNote: qsTr("Pointer preferences stay live; the device inventory shows its own absence note.") },
        { id: "trackpad", title: qsTr("Trackpad"), icon: "trackpad", group: "input",
          shipped: true,
          owner: "settingsd + compositor input (libinput)", disposition: "ship/adapt",
          deepLink: "settings://pane/trackpad",
          absence: "live-on-defaults",
          absenceNote: qsTr("Trackpad preferences stay live; the device inventory shows its own absence note.") },
        { id: "game-controllers", title: qsTr("Game Controllers"), icon: "input", group: "input",
          shipped: true,
          owner: "udev/SDL", disposition: "adapt/delegate",
          deepLink: "settings://pane/game-controllers",
          absence: "disabled-with-reason",
          absenceNote: qsTr("No game controller service is available.") },
        { id: "printers", title: qsTr("Printers & Scanners"), icon: "printer", group: "input",
          shipped: true,
          owner: "CUPS + SANE", disposition: "ship",
          deepLink: "settings://pane/printers",
          absence: "disabled-with-reason",
          absenceNote: qsTr("Printers and scanners are unavailable because their host services are not running.") }
    ]

    // T285: the deep-link aliases and removed panes, mirrored one-for-one from
    // `SettingsPanes.aliases`/`SettingsPanes.removed`. This is the only place
    // the removal reason is explained; the debug view reads it. `energy` is
    // the label spelling of the stable `battery` id (ADR 0300).
    readonly property var aliases: ({
        "storage": { paneId: "general", subPaneId: "storage", anchor: "" },
        "mission-control": { paneId: "desktop-dock", subPaneId: "",
                             anchor: "Workspace Overview" },
        "overview": { paneId: "desktop-dock", subPaneId: "",
                      anchor: "Workspace Overview" },
        "control-center": { paneId: "menu-bar", subPaneId: "controls",
                            anchor: "" },
        "quick-settings": { paneId: "menu-bar", subPaneId: "controls",
                            anchor: "" },
        "internet-accounts": { paneId: "online-accounts", subPaneId: "",
                               anchor: "" },
        // The visible label is Energy; the canonical id stays `battery`.
        "energy": { paneId: "battery", subPaneId: "", anchor: "" }
    })

    readonly property var removed: [
        { id: "biometrics", label: qsTr("Biometrics & Password"),
          refused: true,
          reason: qsTr("Biometrics is not offered; fingerprint and password "
                       + "settings live in Users & Groups and Passwords & Keys.") },
        { id: "mission-control", label: qsTr("Overview"), refused: false,
          target: "desktop-dock",
          reason: qsTr("Overview moved to Desktop & Dock -> Workspace Overview.") },
        { id: "control-center", label: qsTr("Quick Settings"), refused: false,
          target: "menu-bar",
          reason: qsTr("Quick Settings moved to Menu Bar -> Menu Bar Controls.") }
    ]

    function aliasFor(id) {
        return root.aliases[id] !== undefined ? root.aliases[id] : null;
    }

    function removedFor(id) {
        for (var i = 0; i < root.removed.length; ++i) {
            if (root.removed[i].id === id)
                return root.removed[i];
        }
        return null;
    }

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
    // T281: `consumer` records the component that reacts to the row's key for
    // the Linux-specific audit; empty means the row predates that audit.
    function row(id, section, label, control, owner, key, absence, state,
                 storedOnly, consumer, reason) {
        return {
            id: id, section: section, label: label, control: control,
            owner: owner, key: key, absence: absence,
            state: state !== undefined ? state : "planned",
            // T271: a durable preference with no live consumer is explicitly
            // marked, never presented as live (ADR 0262).
            storedOnly: storedOnly === true,
            consumer: consumer !== undefined ? consumer : "",
            // T287: an omitted row carries the reason the mockup row has no
            // Linux owner. Empty for every row that ships.
            reason: reason !== undefined ? reason : ""
        };
    }

    readonly property var rows: root.buildRows()

    function buildRows() {
        return ({
            "wifi": [
                root.row("power", qsTr("Wi-Fi"), qsTr("Wi-Fi"), "toggle",
                         "NetworkManager", "WirelessEnabled", "disabled-with-reason", "shipped"),
                root.row("current", qsTr("Wi-Fi"), qsTr("Current network"), "info+details",
                         "NetworkManager", "ActiveConnection", "disabled-with-reason", "shipped"),
                root.row("hotspot", qsTr("Wi-Fi"), qsTr("Personal Hotspot"), "toggle+sheet",
                         "NetworkManager", "802-11-wireless.mode=ap", "disabled-with-reason", "shipped"),
                root.row("saved", qsTr("Wi-Fi"), qsTr("Known networks"), "list",
                         "NetworkManager", "Settings.Connection", "disabled-with-reason", "shipped"),
                root.row("available", qsTr("Wi-Fi"), qsTr("Other networks"), "list",
                         "NetworkManager", "AccessPoint", "disabled-with-reason", "shipped"),
                root.row("scan", qsTr("Wi-Fi"), qsTr("Scan for networks"), "button",
                         "NetworkManager", "AccessPoint.RequestScan", "disabled-with-reason", "shipped"),
                // T288: the mockup's Other… footer opens the hidden-network
                // dialog; the map keeps the descriptive label so search still
                // finds "hidden network".
                root.row("hidden", qsTr("Wi-Fi"), qsTr("Join Other Network..."), "dialog",
                         "NetworkManager", "AddAndActivateConnection", "disabled-with-reason", "shipped"),
                // T271: the shell has no Wi-Fi prompt consumer yet, so these
                // durable preferences are stored-only and say so in the pane.
                root.row("ask", qsTr("Wi-Fi"), qsTr("Ask to join networks"), "popup",
                         "settingsd", "network.wifi.askToJoin", "live-on-defaults", "shipped", true),
                root.row("askHotspots", qsTr("Wi-Fi"), qsTr("Ask to join hotspots"), "popup",
                         "settingsd", "network.wifi.askToJoinHotspots", "live-on-defaults", "shipped", true),
                root.row("advanced", qsTr("Wi-Fi"), qsTr("Advanced..."), "sheet",
                         "NetworkManager + settingsd", "nm.connection", "disabled-with-reason", "shipped"),
                root.row("quick", qsTr("Wi-Fi"), qsTr("Quick sheet"), "panel",
                         "shell", "quickSettings.wifi", "live-on-defaults", "shipped")
            ],
            "bluetooth": [
                root.row("power", qsTr("Bluetooth"), qsTr("Bluetooth"), "toggle",
                         "BlueZ", "Adapter1.Powered", "disabled-with-reason", "shipped"),
                root.row("discoverable", qsTr("Bluetooth"), qsTr("Discoverable as..."), "info",
                         "BlueZ", "Adapter1.Discoverable", "disabled-with-reason", "shipped"),
                root.row("paired", qsTr("Bluetooth"), qsTr("My devices"), "list+info",
                         "BlueZ", "Device1", "disabled-with-reason", "shipped"),
                root.row("nearby", qsTr("Bluetooth"), qsTr("Nearby devices"), "list",
                         "BlueZ", "discovery", "disabled-with-reason", "shipped"),
                root.row("info", qsTr("Bluetooth"), qsTr("Device info"), "sheet",
                         "BlueZ", "Device1.Address", "disabled-with-reason", "shipped"),
                root.row("advanced", qsTr("Bluetooth"), qsTr("Advanced..."), "sheet",
                         "BlueZ + settingsd", "bluetooth.*", "disabled-with-reason", "shipped"),
                root.row("quick", qsTr("Bluetooth"), qsTr("Quick sheet"), "panel",
                         "shell", "quickSettings.bluetooth", "live-on-defaults")
            ],
            "network": [
                root.row("wifi", qsTr("Network"), qsTr("Wi-Fi"), "disclosure",
                         "NetworkManager", "WirelessEnabled", "disabled-with-reason", "shipped"),
                root.row("firewall", qsTr("Network"), qsTr("Firewall"), "disclosure",
                         "firewall adapter", "firewall.status", "disabled-with-reason", "shipped"),
                root.row("services", qsTr("Network"), qsTr("Other services"), "list",
                         "NetworkManager", "Devices", "disabled-with-reason", "shipped"),
                root.row("vpn", qsTr("Network"), qsTr("VPN"), "disclosure",
                         "NetworkManager", "Settings.Connection", "disabled-with-reason", "shipped"),
                root.row("actions", qsTr("Network"), qsTr("Service actions"), "menu",
                         "NetworkManager", "ActivateConnection", "disabled-with-reason", "shipped"),
                root.row("details", qsTr("Network"), qsTr("Details..."), "sheet",
                         "NetworkManager", "nm.connection", "disabled-with-reason", "shipped"),
                root.row("metered", qsTr("Network"), qsTr("Metered"), "info",
                         "NetworkManager", "connection.metered", "read-only", "shipped"),
                root.row("mac", qsTr("Network"), qsTr("MAC randomization"), "info",
                         "NetworkManager", "cloned-mac-address", "read-only", "shipped"),
                root.row("ipv6", qsTr("Network"), qsTr("IPv6"), "info",
                         "NetworkManager", "ipv6.method", "read-only", "shipped"),
                root.row("8021x", qsTr("Network"), qsTr("802.1X"), "info",
                         "NetworkManager", "802-11-wireless-security.key-mgmt", "read-only", "shipped"),
                root.row("profiles", qsTr("Network"), qsTr("Profiles"), "popup",
                         "NetworkManager", "Settings.Connection", "disabled-with-reason"),
                root.row("firewallOptions", qsTr("Network"), qsTr("Firewall options"), "sheet",
                         "firewall adapter", "firewall.rules", "disabled-with-reason", "shipped",
                         false, "firewall adapter (ufw/nftables read; writes prompt with pkexec)"),
                root.row("addService", qsTr("Network"), qsTr("Add service..."), "sheet",
                         "NetworkManager", "AddConnection", "disabled-with-reason", "shipped",
                         false, "NetworkManager (Ethernet, Wi-Fi, bridge, bond, VLAN)"),
                // T288: the mockup's More popup and its real service-order
                // sheet (NetworkManager autoconnect priority).
                root.row("more", qsTr("Network"), qsTr("More"), "menu",
                         "NetworkManager", "autoconnect-priority", "disabled-with-reason", "shipped"),
                root.row("serviceOrder", qsTr("Network"), qsTr("Set service order..."), "sheet",
                         "NetworkManager", "autoconnect-priority", "disabled-with-reason", "shipped")
            ],
            "vpn": [
                root.row("vpn", qsTr("VPN"), qsTr("Connections"), "list",
                         "NetworkManager VPN", "Settings.Connection", "disabled-with-reason", "shipped"),
                root.row("vpn.wireguard", qsTr("VPN"), qsTr("WireGuard"), "info",
                         "NetworkManager VPN", "wireguard", "read-only", "shipped",
                         false, "NetworkManager (native WireGuard connection type)"),
                root.row("vpn.add", qsTr("VPN"), qsTr("Add VPN configuration..."), "sheet",
                         "NetworkManager VPN", "AddConnection", "disabled-with-reason", "shipped")
            ],
            "battery": [
                root.row("profile", qsTr("Battery"), qsTr("Low Power Mode"), "popup",
                         "power-profiles-daemon", "power.profile", "disabled-with-reason", "shipped"),
                root.row("health", qsTr("Battery"), qsTr("Battery health"), "info+detail",
                         "UPower", "Battery.Health", "read-only", "shipped"),
                root.row("status", qsTr("Battery"), qsTr("Battery status"), "pill",
                         "UPower", "Battery.State", "read-only", "shipped"),
                root.row("usage", qsTr("Battery"), qsTr("History range"), "segmented",
                         "settingsd + UPower", "power.historyRange", "live-on-defaults", "shipped"),
                root.row("lastCharged", qsTr("Battery"), qsTr("Last charged"), "info",
                         "UPower", "Battery.EnergyFull", "read-only", "shipped"),
                root.row("level", qsTr("Battery"), qsTr("Battery level"), "chart",
                         "UPower", "Battery.Percentage", "read-only", "shipped"),
                root.row("screenOn", qsTr("Battery"), qsTr("Screen-on usage"), "info",
                         "session sampler", "power.screenOn", "disabled-with-reason", "shipped"),
                root.row("options", qsTr("Battery"), qsTr("Options..."), "sheet",
                         "settingsd", "power.*", "live-on-defaults", "shipped"),
                root.row("options.showPercentage", qsTr("Battery"), qsTr("Show battery percentage"), "toggle",
                         "settingsd -> shell/MenuBar", "power.showPercentage", "live-on-defaults", "shipped"),
                root.row("options.dimOnBattery", qsTr("Battery"), qsTr("Dim on battery"), "toggle",
                         "settingsd -> shell/display", "power.dimOnBattery", "live-on-defaults", "shipped"),
                root.row("options.preventSleep", qsTr("Battery"), qsTr("Prevent sleep"), "toggle",
                         "settingsd -> session/idle", "power.preventSleep", "live-on-defaults", "shipped"),
                root.row("options.wakeForNetwork", qsTr("Battery"), qsTr("Wake for network access"), "toggle",
                         "host firmware", "power.wakeForNetwork", "disabled-with-reason", "shipped"),
                root.row("options.optimizeVideo", qsTr("Battery"), qsTr("Optimize video streaming"), "toggle",
                         "media pipeline", "power.optimizeVideo", "disabled-with-reason", "shipped"),
                root.row("adapter.state", qsTr("Battery"), qsTr("Power adapter"), "info",
                         "UPower", "Battery.State", "read-only", "shipped"),
                root.row("adapter.chargeLimit", qsTr("Battery"), qsTr("Charge limit"), "info",
                         "UPower", "Battery.ChargeThreshold", "read-only", "shipped",
                         false, "UPower (read-only; no charge-threshold write)"),
                root.row("powerSettings", qsTr("Battery"), qsTr("Power settings"), "rows",
                         "settingsd", "idle.*", "live-on-defaults", "shipped")
            ],
            "general": [
                root.row("about.deviceName", qsTr("About"), qsTr("Name"), "text",
                         "hostname/distro adapter", "host.hostname", "read-only", "shipped"),
                root.row("about.hardware", qsTr("About"), qsTr("Processor, memory, GPU, disk, serial, OS"), "info",
                         "host info", "host.info", "read-only", "shipped"),
                root.row("about.diskEncryption", qsTr("About"), qsTr("Disk encryption"), "info",
                         "host info", "host.diskEncryption", "read-only", "shipped"),
                root.row("about.securityUpdates", qsTr("About"), qsTr("Security updates"), "info",
                         "update provider", "updates.security", "read-only", "shipped"),
                root.row("about.session", qsTr("About"), qsTr("Session"), "info",
                         "compositor", "session.type", "read-only", "shipped"),
                root.row("about.moreInfo", qsTr("About"), qsTr("System Report..."), "button",
                         "host info", "host.info", "read-only", "shipped"),
                root.row("about.regulatory", qsTr("About"), qsTr("Regulatory info"), "link",
                         "distro/legal", "distro.regulatory", "delegated", "omitted",
                         false, "", qsTr("No regulatory document ships with this build.")),
                root.row("about.energy", qsTr("About"), qsTr("Energy Compliance"), "link",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No host energy-compliance document exists.")),
                root.row("updates.auto", qsTr("Software Update"), qsTr("Automatic updates"), "toggle",
                         "settingsd -> update provider", "updates.auto", "disabled-with-reason", "shipped"),
                root.row("updates.check", qsTr("Software Update"), qsTr("Check for updates"), "button",
                         "PackageKit", "updates.check", "disabled-with-reason", "shipped"),
                root.row("updates.install", qsTr("Software Update"), qsTr("Install updates"), "button",
                         "PackageKit", "updates.install", "disabled-with-reason", "shipped"),
                root.row("updates.options", qsTr("Software Update"), qsTr("Automatic update policy"), "sheet",
                         "settingsd -> update provider", "updates.autoInstall", "disabled-with-reason", "shipped"),
                root.row("updates.alsoAvailable", qsTr("Software Update"), qsTr("Also Available"), "list",
                         "PackageKit", "updates.list", "disabled-with-reason", "shipped"),
                root.row("updates.installed", qsTr("Software Update"), qsTr("Installed"), "info",
                         "host info", "host.osLabel", "read-only", "shipped"),
                root.row("updates.beta", qsTr("Software Update"), qsTr("Beta Updates"), "toggle",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("The update provider advertises no beta channel.")),
                root.row("storage.categories", qsTr("Storage"), qsTr("Categories"), "chart/list",
                         "UDisks2 + filesystem usage", "storage.usage", "disabled-with-reason", "shipped"),
                root.row("storage.manage", qsTr("Storage"), qsTr("Show Detail"), "button",
                         "Files/Storage", "files.open", "delegated", "shipped"),
                root.row("login.start", qsTr("Login Items & Extensions"), qsTr("Open at Login"), "list",
                         "XDG autostart + settingsd", "login.startup", "disabled-with-reason", "shipped"),
                root.row("login.background", qsTr("Login Items & Extensions"), qsTr("Background App Activity"), "list",
                         "systemd user units + portals", "login.background", "disabled-with-reason", "shipped"),
                root.row("login.extensions", qsTr("Login Items & Extensions"), qsTr("Extensions"), "list",
                         "portal extensions", "portal.extensions", "delegated", "shipped"),
                root.row("login.extensionsGroup", qsTr("Login Items & Extensions"), qsTr("Group extensions"), "segmented",
                         "shell", "portal.extensions", "live-on-defaults", "shipped"),
                root.row("login.add", qsTr("Login Items & Extensions"), qsTr("Add login item..."), "button",
                         "XDG autostart adapter", "login.startup", "disabled-with-reason", "shipped"),
                root.row("language.languages", qsTr("Language & Region"), qsTr("Preferred Languages"), "list",
                         "localectl/locale adapter", "locale.languages", "disabled-with-reason", "shipped"),
                root.row("language.region", qsTr("Language & Region"), qsTr("Region"), "popup",
                         "locale adapter", "locale.region", "disabled-with-reason", "shipped"),
                root.row("language.calendar", qsTr("Language & Region"), qsTr("Calendar"), "popup",
                         "settingsd", "locale.calendar", "live-on-defaults", "shipped"),
                root.row("language.temperature", qsTr("Language & Region"), qsTr("Temperature"), "popup",
                         "settingsd", "locale.temperature", "live-on-defaults", "shipped"),
                root.row("language.measurement", qsTr("Language & Region"), qsTr("Measurement"), "popup",
                         "settingsd", "locale.measurement", "live-on-defaults", "shipped"),
                root.row("language.dateFormat", qsTr("Language & Region"), qsTr("Date format"), "popup",
                         "settingsd", "locale.dateFormat", "live-on-defaults", "shipped"),
                root.row("language.numberFormat", qsTr("Language & Region"), qsTr("Number format"), "popup",
                         "settingsd", "locale.numberFormat", "live-on-defaults", "shipped"),
                root.row("language.firstDay", qsTr("Language & Region"), qsTr("First day of week"), "popup",
                         "settingsd", "locale.firstDay", "live-on-defaults", "shipped"),
                root.row("language.textInImages", qsTr("Language & Region"), qsTr("Text in Images"), "toggle",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No desktop text-in-images owner exists.")),
                root.row("language.applications", qsTr("Language & Region"), qsTr("Applications"), "list",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No per-app language store exists.")),
                root.row("language.translation", qsTr("Language & Region"), qsTr("Translation Languages"), "button",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No translation consumer exists.")),
                root.row("time.auto", qsTr("Date & Time"), qsTr("Set automatically"), "toggle",
                         "timedatectl/chrony adapter", "time.ntp", "disabled-with-reason", "shipped"),
                root.row("time.source", qsTr("Date & Time"), qsTr("Source"), "info",
                         "timedatectl/chrony adapter", "timedatectl.timesync", "disabled-with-reason", "shipped"),
                root.row("time.zone", qsTr("Date & Time"), qsTr("Time zone"), "popup/map",
                         "timedatectl", "time.zone", "disabled-with-reason", "shipped"),
                root.row("time.city", qsTr("Date & Time"), qsTr("Time zone city"), "popup",
                         "timedatectl", "time.zone", "disabled-with-reason", "shipped"),
                root.row("time.manual", qsTr("Date & Time"), qsTr("Date and time"), "fields",
                         "timedatectl", "time.set", "delegated", "shipped"),
                root.row("time.hour24", qsTr("Date & Time"), qsTr("24-hour time"), "toggle",
                         "settingsd", "clock.use24Hour", "live-on-defaults", "shipped"),
                root.row("time.seconds", qsTr("Date & Time"), qsTr("Show seconds"), "toggle",
                         "settingsd", "menu.clock.showSeconds", "live-on-defaults", "shipped"),
                root.row("time.showDate", qsTr("Date & Time"), qsTr("Show date"), "toggle",
                         "settingsd", "menu.clock.showDate", "live-on-defaults", "shipped"),
                root.row("time.autoZone", qsTr("Date & Time"), qsTr("Set time zone automatically"), "toggle",
                         "location service adapter", "time.autoZone", "disabled-with-reason", "shipped"),
                root.row("time.options", qsTr("Date & Time"), qsTr("Clock options..."), "sheet",
                         "settingsd", "menu.clock.*", "live-on-defaults", "shipped"),
                root.row("sharing.hostname", qsTr("Sharing"), qsTr("Local hostname"), "text",
                         "hostname adapter", "sharing.hostname", "disabled-with-reason", "shipped"),
                root.row("sharing.file", qsTr("Sharing"), qsTr("File sharing"), "toggle",
                         "Samba/NFS adapter", "sharing.file", "disabled-with-reason", "shipped"),
                root.row("sharing.screen", qsTr("Sharing"), qsTr("Screen sharing"), "toggle",
                         "portal ScreenCast/RDP", "sharing.screen", "delegated", "shipped"),
                root.row("sharing.media", qsTr("Sharing"), qsTr("Media sharing"), "toggle",
                         "DLNA adapter", "sharing.media", "hidden", "shipped"),
                root.row("sharing.printer", qsTr("Sharing"), qsTr("Printer sharing"), "toggle",
                         "CUPS", "sharing.printer", "hidden", "shipped"),
                root.row("sharing.remoteLogin", qsTr("Sharing"), qsTr("Remote login"), "toggle",
                         "OpenSSH adapter", "sharing.ssh", "disabled-with-reason", "shipped"),
                root.row("sharing.remoteManagement", qsTr("Sharing"), qsTr("Remote management"), "toggle",
                         "distro adapter", "sharing.remote", "hidden", "shipped"),
                root.row("sharing.internet", qsTr("Sharing"), qsTr("Internet sharing"), "toggle",
                         "NetworkManager", "sharing.internet", "hidden", "shipped"),
                root.row("sharing.contentCaching", qsTr("Sharing"), qsTr("Content Caching"), "toggle",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No Linux content-caching service exists.")),
                root.row("sharing.bluetooth", qsTr("Sharing"), qsTr("Bluetooth Sharing"), "toggle",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No Bluetooth sharing service exists.")),
                root.row("sharing.remoteScripting", qsTr("Sharing"), qsTr("Remote Application Scripting"), "toggle",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No remote application scripting service exists.")),
                root.row("backup.backend", qsTr("Backups"), qsTr("Backup backend"), "popup",
                         "backup adapter", "backup.backend", "disabled-with-reason", "shipped",
                         false, "backup adapter (restic/Pika/Timeshift detection)"),
                root.row("backup.add", qsTr("Backups"), qsTr("Add Backup Disk..."), "button",
                         "backup adapter", "backup.destination", "delegated", "shipped"),
                root.row("backup.auto", qsTr("Backups"), qsTr("Back up automatically"), "toggle",
                         "settingsd -> backup backend", "backup.auto", "disabled-with-reason", "shipped"),
                root.row("backup.now", qsTr("Backups"), qsTr("Back up now"), "button",
                         "backup adapter", "backup.now", "disabled-with-reason", "shipped"),
                root.row("backup.destination", qsTr("Backups"), qsTr("Destination"), "text",
                         "settingsd -> backup backend", "backup.destination", "delegated", "shipped"),
                root.row("backup.options", qsTr("Backups"), qsTr("Schedule"), "popup",
                         "settingsd -> backup backend", "backup.schedule", "disabled-with-reason", "shipped"),
                root.row("backup.restore", qsTr("Backups"), qsTr("Restore..."), "button",
                         "backup adapter", "backup.restore", "delegated", "shipped"),
                // T287: the mockup's device management row has no Linux
                // MDM/profile owner. The generic label keeps the naming gate
                // and the debug view honest; the reason is the omission half.
                root.row("device.management", qsTr("General"), qsTr("Managed Devices"), "disclosure",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No Linux MDM/profile owner is in scope.")),
                root.row("reset.startupDisk", qsTr("Startup Disk"), qsTr("Startup disk"), "info",
                         "UDisks2/bootloader adapter", "storage.boot", "read-only", "shipped"),
                root.row("startup.disks", qsTr("Startup Disk"), qsTr("Disks"), "list",
                         "UDisks2/bootloader adapter", "storage.boot", "read-only", "shipped"),
                root.row("startup.advanced", qsTr("Startup Disk"), qsTr("Firmware and Secure Boot"), "info",
                         "firmware adapter", "storage.boot", "read-only", "shipped"),
                root.row("startup.restart", qsTr("Startup Disk"), qsTr("Restart"), "button",
                         "logind", "system.reboot", "disabled-with-reason", "shipped"),
                root.row("reset.reset", qsTr("Reset"), qsTr("Reset"), "button",
                         "settingsd", "reset.settings", "live-on-defaults", "shipped"),
                root.row("reset.transfer", qsTr("Transfer or Reset"), qsTr("Transfer or Reset"), "disclosure",
                         "settingsd reset + delegates", "reset.settings", "live-on-defaults", "shipped"),
                root.row("reset.erase", qsTr("Transfer or Reset"), qsTr("Erase All Content and Settings"), "info",
                         "operating-system reinstall", "reset.erase", "hidden", "omitted",
                         false, "", qsTr("Erasing content is an operating-system reinstall, not a settings action.")),
                root.row("passwords.disclosure", qsTr("Passwords & Keys"), qsTr("Passwords & Keys"), "disclosure",
                         "Secret Service/portal", "passwords.provider", "delegated", "shipped"),
                root.row("passwords.open", qsTr("Passwords & Keys"), qsTr("Open Passwords..."), "button",
                         "Secret Service", "passwords.open", "delegated", "shipped"),
                root.row("passwords.source", qsTr("Passwords & Keys"), qsTr("Password provider"), "popup",
                         "settingsd", "defaults.passwordManager", "live-on-defaults", "shipped"),
                root.row("passwords.keyring", qsTr("Passwords & Keys"), qsTr("System Keyring"), "info",
                         "Secret Service", "passwords.keyring", "read-only", "shipped"),
                root.row("passwords.autofill", qsTr("Passwords & Keys"), qsTr("AutoFill Passwords and Keys"), "toggle",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No autofill consumer exists.")),
                root.row("passwords.deleteAfterUse", qsTr("Passwords & Keys"), qsTr("Delete After Use"), "toggle",
                         "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No verification-code consumer exists."))
            ],
            "storage": [
                root.row("categories", qsTr("Storage"), qsTr("Categories"), "chart/list",
                         "UDisks2 + filesystem usage", "storage.usage", "disabled-with-reason", "shipped"),
                root.row("manage", qsTr("Storage"), qsTr("Manage storage..."), "button",
                         "Files/Storage", "files.open", "delegated", "shipped"),
                root.row("volumes", qsTr("Storage"), qsTr("Volumes"), "list",
                         "UDisks2", "storage.volumes", "disabled-with-reason", "shipped"),
                root.row("removable", qsTr("Storage"), qsTr("Removable media"), "list",
                         "UDisks2", "storage.drives", "disabled-with-reason", "shipped")
            ],
            "appearance": [
                root.row("scheme", qsTr("Appearance"), qsTr("Appearance"), "tiles",
                         "settingsd + compositor", "appearance.colorScheme", "live-on-defaults", "shipped"),
                root.row("accent", qsTr("Theme"), qsTr("Accent color"), "swatches",
                         "settingsd", "appearance.accent", "live-on-defaults", "shipped"),
                root.row("highlight", qsTr("Theme"), qsTr("Text highlight color"), "popup",
                         "settingsd", "appearance.highlight", "live-on-defaults", "shipped"),
                root.row("iconStyle", qsTr("Theme"), qsTr("Icon & widget style"), "tiles",
                         "settingsd + icon-theme provider", "appearance.iconStyle",
                         "disabled-with-reason", "shipped"),
                root.row("folderColor", qsTr("Theme"), qsTr("Folder color"), "popup",
                         "settingsd + Files", "appearance.folderColor", "live-on-defaults", "shipped"),
                root.row("sidebarIconSize", qsTr("Windows"), qsTr("Sidebar icon size"), "popup",
                         "settingsd", "appearance.sidebarIconSize", "live-on-defaults", "shipped"),
                root.row("wallpaperTint", qsTr("Windows"),
                         qsTr("Tint window background with wallpaper color"), "toggle",
                         "settingsd", "appearance.wallpaperTint", "live-on-defaults", "shipped"),
                root.row("scrollBars", qsTr("Show scroll bars"), qsTr("Show scroll bars"), "radio",
                         "settingsd", "appearance.scrollBars", "live-on-defaults", "shipped"),
                root.row("scrollBarClick", qsTr("Click in the scroll bar to"),
                         qsTr("Click in the scroll bar to"), "radio",
                         "settingsd", "appearance.scrollBarClick", "live-on-defaults", "shipped"),
                root.row("transparency", qsTr("Glass"), qsTr("Glass tint"), "slider",
                         "settingsd", "appearance.transparency", "live-on-defaults", "shipped"),
                root.row("iconTheme", qsTr("Advanced Appearance"), qsTr("Icon theme"), "popup",
                         "settingsd + icon-theme provider", "appearance.iconTheme",
                         "disabled-with-reason", "shipped",
                         false, "shell/app-index icon lookup, apps/settings icon provider"),
                root.row("cursorTheme", qsTr("Advanced Appearance"), qsTr("Cursor theme"), "popup",
                         "settingsd + compositor", "appearance.cursorTheme",
                         "disabled-with-reason", "shipped",
                         false, "compositor/cursor theme (next session)"),
                root.row("cursorSize", qsTr("Advanced Appearance"), qsTr("Cursor size"), "slider",
                         "settingsd + compositor", "appearance.cursorSize",
                         "disabled-with-reason", "shipped",
                         false, "compositor/cursor theme (next session)"),
                root.row("windowControls", qsTr("Advanced Appearance"), qsTr("Window control layout"), "popup",
                         "compositor/window decoration", "appearance.windowControls",
                         "disabled-with-reason", "shipped",
                         false, "compositor/window decoration (traffic lights stay left)"),
                root.row("material", qsTr("Advanced Appearance"), qsTr("Material effects"), "popup",
                         "shell/design-system Glass", "appearance.material",
                         "disabled-with-reason", "shipped",
                         false, "shell/design-system Glass (GPU budget owns the tier)"),
                root.row("accentFromWallpaper", qsTr("Advanced Appearance"), qsTr("Accent from wallpaper"), "toggle",
                         "shell/design-system Theme", "appearance.accentFromWallpaper",
                         "disabled-with-reason", "shipped")
            ],
            "accessibility": [
                // T291a rebuilds Accessibility as the mockup's big list of
                // in-place sub-panes. Vision is the first shipped section; the
                // later children (T291b-T291e) flip their own planned rows to
                // shipped as each sub-pane body lands. A planned row is not
                // rendered and not search-indexed.
                //
                // Vision. The screen reader is a live status plus a delegate
                // action, not a toggle: org.a11y.Status is read-only (ADR 0144).
                root.row("vision.screenReader", qsTr("Screen Reader"),
                         qsTr("Screen reader"), "status+open", "Orca (AT-SPI)",
                         "org.a11y.Status.ScreenReaderEnabled", "delegated", "shipped"),
                root.row("vision.bridge", qsTr("Screen Reader"),
                         qsTr("Accessibility bridge"), "status", "AT-SPI",
                         "org.a11y.Status.IsEnabled", "read-only", "shipped"),
                // reference-only Vision rows with no Linux owner. They are omitted
                // from the sub-pane and carry the reason here.
                root.row("vision.hoverText", qsTr("Vision"), qsTr("Hover text"),
                         "omit", "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("No hover-text overlay reads the text under the pointer.")),
                root.row("vision.display.invertColors", qsTr("Display"),
                         qsTr("Invert colors"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor has no whole-scene colour-inversion pass.")),
                root.row("vision.display.increaseContrast", qsTr("Display"),
                         qsTr("Increase contrast"), "toggle",
                         "shell/design-system Glass", "accessibility.increaseContrast",
                         "live-on-defaults", "shipped"),
                root.row("vision.display.reduceTransparency", qsTr("Display"),
                         qsTr("Reduce transparency"), "toggle",
                         "shell/design-system Glass", "accessibility.reduceTransparency",
                         "live-on-defaults", "shipped"),
                root.row("vision.display.differentiateWithoutColor", qsTr("Display"),
                         qsTr("Differentiate without color"), "toggle",
                         "shell/design-system Theme/Toggle",
                         "accessibility.differentiateWithoutColor",
                         "live-on-defaults", "shipped"),
                root.row("vision.display.titleIcons", qsTr("Display"),
                         qsTr("Show window title icons"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("Window decoration has no title-icon slot.")),
                root.row("vision.display.toolbarShapes", qsTr("Display"),
                         qsTr("Show toolbar button shapes"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The design system has no toolbar shape-cue pass.")),
                root.row("vision.display.displayContrast", qsTr("Display"),
                         qsTr("Display contrast"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor has no display-contrast adjustment pass.")),
                root.row("vision.display.textSize", qsTr("Display"),
                         qsTr("Text size"), "popup", "shell/design-system Theme",
                         "accessibility.textSize", "live-on-defaults", "shipped"),
                root.row("vision.display.menuBarSize", qsTr("Display"),
                         qsTr("Menu bar size"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor allocates a fixed menu-bar height at session start.")),
                // The cursor key is shared with Appearance and is applied by
                // the compositor at the next session start, so the row is
                // stored-only rather than pretending to be live.
                root.row("vision.display.pointerSize", qsTr("Display"),
                         qsTr("Pointer size"), "slider",
                         "settingsd + compositor cursor", "appearance.cursorSize",
                         "live-on-defaults", "shipped", true),
                root.row("vision.display.pointerColors", qsTr("Display"),
                         qsTr("Pointer colors"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The cursor theme has no per-colour override.")),
                root.row("vision.display.colorFilters", qsTr("Display"),
                         qsTr("Color filters"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor has no colour-filter pass.")),
                root.row("vision.magnifier", qsTr("Zoom"),
                         qsTr("Use keyboard shortcuts to zoom"), "toggle",
                         "compositor/magnifier", "accessibility.magnifier",
                         "live-on-defaults", "shipped"),
                root.row("vision.magnifier.zoom", qsTr("Zoom"), qsTr("Zoom level"),
                         "slider", "compositor/magnifier",
                         "accessibility.magnifierZoom", "live-on-defaults", "shipped"),
                root.row("vision.magnifier.mode", qsTr("Zoom"), qsTr("Follow"),
                         "popup", "compositor/magnifier",
                         "accessibility.magnifierMode", "live-on-defaults", "shipped"),
                root.row("vision.motion.reduceMotion", qsTr("Motion"),
                         qsTr("Reduce motion"), "toggle",
                         "shell/design-system Theme, compositor/window motion",
                         "accessibility.reduceMotion", "live-on-defaults", "shipped"),
                root.row("vision.motion.dimFlashingLights", qsTr("Motion"),
                         qsTr("Dim flashing lights"), "toggle",
                         "shell/Dock attention bounce", "accessibility.dimFlashingLights",
                         "live-on-defaults", "shipped"),
                root.row("vision.motion.autoPlayImages", qsTr("Motion"),
                         qsTr("Auto-play animated images"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The shell has no animated-image surface to gate.")),
                root.row("vision.motion.nonBlinkingCursor", qsTr("Motion"),
                         qsTr("Prefer non-blinking cursor"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The text cursor blink is toolkit-owned; no per-app override exists.")),
                root.row("vision.readSpeak", qsTr("Read & Speak"), qsTr("Read & speak"),
                         "action", "speech-dispatcher/espeak-ng", "spd-say",
                         "delegated", "shipped"),
                root.row("vision.audioDescriptions", qsTr("Read & Speak"),
                         qsTr("Audio descriptions"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("Audio descriptions are content-level, not a shell preference.")),
                // Physical and Motor (T291b). Keyboard and Pointer Control are
                // the two shipped groups; Voice Control and Switch Control have
                // no Linux owner and are omitted with a reason.
                root.row("motor.voiceControl", qsTr("Voice Control"),
                         qsTr("Voice control"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No voice-control engine or speech-to-pointer "
                              + "delegate exists in this build.")),
                root.row("motor.keyboard.fullKeyboardAccess", qsTr("Keyboard"),
                         qsTr("Full keyboard access"), "toggle", "settingsd",
                         "input.keyboardNavigation", "live-on-defaults",
                         "shipped", true),
                root.row("motor.keyboard.stickyKeys", qsTr("Keyboard"),
                         qsTr("Sticky keys"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor has no sticky-key modifier latch.")),
                root.row("motor.keyboard.slowKeys", qsTr("Keyboard"),
                         qsTr("Slow keys"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor has no slow-key acceptance delay.")),
                root.row("motor.keyboard.onScreenKeyboard", qsTr("Keyboard"),
                         qsTr("On-screen keyboard"), "button",
                         "on-screen keyboard (onboard/wvkbd)", "onboard",
                         "delegated", "shipped"),
                root.row("motor.pointer.doubleClickSpeed",
                         qsTr("Pointer Control"), qsTr("Double-click speed"),
                         "slider", "compositor/input pointer",
                         "input.doubleClickSpeed", "live-on-defaults", "shipped"),
                root.row("motor.pointer.mouseKeys", qsTr("Pointer Control"),
                         qsTr("Mouse keys"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor has no keyboard pointer mode.")),
                root.row("motor.pointer.ignoreTrackpad",
                         qsTr("Pointer Control"), qsTr("Ignore built-in trackpad"),
                         "omit", "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("The compositor cannot ignore one pointer device.")),
                root.row("motor.switchControl", qsTr("Switch Control"),
                         qsTr("Switch control"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No switch-control delegate or scanning input "
                              + "source exists in this build.")),
                // Hearing (T291c). Audio is the one group with a real owner.
                // Hearing Devices, Subtitles and Captioning, Live Captions, and
                // Spoken Name have no Linux owner; their rows are omissions with
                // a reason and are never rendered or search-indexed.
                root.row("hearing.devices", qsTr("Hearing Devices"),
                         qsTr("Hearing devices"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No hearing-aid or LE Audio hearing-device service "
                              + "is integrated in this build.")),
                root.row("hearing.flashScreen", qsTr("Audio"),
                         qsTr("Flash the screen on alert"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("The compositor has no full-output alert flash "
                              + "surface.")),
                root.row("hearing.monoAudio", qsTr("Audio"), qsTr("Mono audio"),
                         "omit", "omit", "omit", "omitted", "omitted", false, "",
                         qsTr("PipeWire has no per-session stereo-to-mono downmix "
                              + "in the control path this build uses.")),
                root.row("hearing.balance", qsTr("Audio"), qsTr("Balance"), "slider",
                         "apps/settings", "sound.balance", "live-on-defaults",
                         "shipped", true,
                         "apps/settings (balance write deferred)"),
                root.row("hearing.backgroundSounds", qsTr("Audio"),
                         qsTr("Background sounds"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No background-sound player or audio delegate is "
                              + "shipped.")),
                root.row("hearing.captions", qsTr("Subtitles and Captioning"),
                         qsTr("Captions"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No live-caption or subtitle delegate exists in "
                              + "this build.")),
                root.row("hearing.rtt", qsTr("Subtitles and Captioning"),
                         qsTr("RTT"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No real-time text service exists in this build.")),
                root.row("hearing.nameRecognition", qsTr("Live Captions"),
                         qsTr("Name recognition"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No on-device name-recognition model exists.")),
                root.row("hearing.spokenName", qsTr("Spoken Name"),
                         qsTr("Spoken name"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No on-device spoken-name model exists.")),
                // Speech (T291d). There is no first-party speech service and
                // no voice model, so the section keeps one group with one real
                // owner: the speech-dispatcher/espeak-ng delegate the Vision
                // Read & Speak row also uses. Custom Voice and Vocal Shortcuts
                // have no Linux owner; their rows are omissions with a reason
                // and are never rendered or search-indexed.
                root.row("speech.liveSpeech", qsTr("Live Speech"),
                         qsTr("Speak sample"), "action",
                         "speech-dispatcher/espeak-ng", "spd-say",
                         "delegated", "shipped"),
                root.row("speech.customVoice", qsTr("Custom Voice"),
                         qsTr("Custom voice"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No on-device voice model or voice-cloning "
                              + "service exists in this build; the engine's "
                              + "own voices are chosen inside speech-dispatcher.")),
                root.row("speech.vocalShortcuts", qsTr("Vocal Shortcuts"),
                         qsTr("Vocal shortcuts"), "omit", "omit", "omit",
                         "omitted", "omitted", false, "",
                         qsTr("No vocal-shortcut trigger engine (voice sound "
                              + "recognition mapped to shell actions) exists in "
                              + "this build.")),
                // General -> Accessibility Shortcut (T291e). The last section of
                // the list. The row reuses the one `ShortcutRecorder`; the
                // compositor's shortcut engine installs the recorded chord and
                // the `accessibility-shortcut` action opens this pane, so the
                // row is live on the durable key, not stored-only.
                root.row("shortcut.recorder", qsTr("Accessibility Shortcut"),
                         qsTr("Accessibility shortcut"), "recorder",
                         "shell/compositor shortcut engine",
                         "accessibility.shortcut", "live-on-defaults", "shipped")
            ],
            "control-center": [
                root.row("modules", qsTr("Quick Settings"), qsTr("Modules"), "list",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("wifi", qsTr("Quick Settings"), qsTr("Wi-Fi"), "checkbox",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("bluetooth", qsTr("Quick Settings"), qsTr("Bluetooth"), "checkbox",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("focus", qsTr("Quick Settings"), qsTr("Focus"), "checkbox",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("screenMirroring", qsTr("Quick Settings"), qsTr("Screen casting"), "checkbox",
                         "settingsd", "menu.control.screenMirroring", "live-on-defaults"),
                root.row("nowPlaying", qsTr("Quick Settings"), qsTr("Now Playing"), "checkbox",
                         "settingsd", "menu.control.nowPlaying", "live-on-defaults"),
                root.row("sound", qsTr("Quick Settings"), qsTr("Sound"), "checkbox",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("display", qsTr("Quick Settings"), qsTr("Display"), "checkbox",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("battery", qsTr("Quick Settings"), qsTr("Battery"), "checkbox",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("darkMode", qsTr("Quick Settings"), qsTr("Dark mode"), "checkbox",
                         "settingsd", "quickSettings.modules", "live-on-defaults", "shipped"),
                root.row("stage", qsTr("Quick Settings"), qsTr("Stage"), "checkbox",
                         "settingsd", "menu.control.stageManager", "live-on-defaults"),
                root.row("screenRecording", qsTr("Quick Settings"), qsTr("Screen recording"), "checkbox",
                         "settingsd", "menu.control.screenRecording", "live-on-defaults"),
                root.row("keyboardBrightness", qsTr("Quick Settings"), qsTr("Keyboard brightness"), "checkbox",
                         "settingsd", "menu.control.keyboardBrightness", "live-on-defaults"),
                root.row("hearing", qsTr("Quick Settings"), qsTr("Hearing"), "checkbox",
                         "settingsd", "menu.control.hearing", "live-on-defaults"),
                root.row("menuBarItems", qsTr("Quick Settings"), qsTr("Menu bar items"), "list",
                         "settingsd", "quickSettings.menuBarOnly", "live-on-defaults", "shipped"),
                root.row("pages", qsTr("Quick Settings"), qsTr("Pages"), "list",
                         "settingsd", "quickSettings.pageCount", "live-on-defaults", "shipped"),
                root.row("spans", qsTr("Quick Settings"), qsTr("Control sizes"), "list",
                         "settingsd", "quickSettings.spans", "live-on-defaults", "shipped"),
                root.row("pageNames", qsTr("Quick Settings"), qsTr("Page names"), "list",
                         "settingsd", "quickSettings.pageNames", "live-on-defaults", "shipped"),
                root.row("preview", qsTr("Quick Settings"), qsTr("Preview"), "preview",
                         "settingsd + shell/control-center", "quickSettings.modules",
                         "live-on-defaults", "shipped"),
                root.row("perDisplay", qsTr("Quick Settings"), qsTr("Show on all displays"), "toggle",
                         "settingsd -> shell", "quickSettings.perDisplay",
                         "disabled-with-reason", "shipped"),
                root.row("otherModules", qsTr("Quick Settings"), qsTr("Other modules"), "list",
                         "settingsd", "quickSettings.modules", "live-on-defaults"),
                root.row("addControls", qsTr("Quick Settings"), qsTr("Add controls..."), "sheet",
                         "settingsd + shell/control-center", "quickSettings.modules",
                         "live-on-defaults", "shipped"),
                root.row("reset", qsTr("Quick Settings"), qsTr("Reset Quick Settings"), "button",
                         "settingsd", "quickSettings.modules + menu.control.*",
                         "live-on-defaults", "shipped")
            ],
            "menu-bar": [
                root.row("autoHide", qsTr("Menu Bar"), qsTr("Auto-hide menu bar"), "popup",
                         "settingsd", "menu.autoHide", "live-on-defaults", "shipped"),
                root.row("background", qsTr("Menu Bar"), qsTr("Show menu bar background"), "toggle",
                         "settingsd", "menu.showBackground", "live-on-defaults", "shipped"),
                root.row("osGlyph", qsTr("Menu Bar"), qsTr("OS menu glyph"), "radio",
                         "settingsd -> shell", "menu.osGlyph", "live-on-defaults", "shipped"),
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
                         "StatusNotifier registry", "menu.tray.hidden", "delegated"),
                root.row("overflow", qsTr("Menu Bar"), qsTr("Overflow policy"), "popup",
                         "settingsd -> shell/MenuBar", "menu.overflow",
                         "live-on-defaults", "shipped", false, "shell/MenuBar"),
                root.row("workspaces", qsTr("Menu Bar"), qsTr("Workspace indicator"), "toggle",
                         "settingsd -> shell/MenuBar", "menu.control.workspaces",
                         "live-on-defaults", "shipped", false, "shell/MenuBar"),
                root.row("globalMenu", qsTr("Menu Bar"), qsTr("Show application menus in the menu bar"), "toggle",
                         "settingsd -> shell/MenuBar", "menu.global",
                         "live-on-defaults", "shipped", false, "shell/MenuBar, services/menu-broker"),
                root.row("perDisplayPanel", qsTr("Menu Bar"), qsTr("Show on every display"), "toggle",
                         "settingsd -> shell", "quickSettings.perDisplay",
                         "disabled-with-reason", "shipped"),
                root.row("reset", qsTr("Menu Bar"), qsTr("Reset Quick Settings"), "button",
                         "settingsd", "menu.control.*", "live-on-defaults", "shipped")
            ],
            "search": [
                root.row("results", qsTr("Search"), qsTr("Search results"), "checkboxes",
                         "settingsd -> search service", "search.categories", "live-on-defaults", "shipped"),
                root.row("menuBar", qsTr("Search"), qsTr("Show Search in menu bar"), "toggle",
                         "settingsd", "menu.control.search", "live-on-defaults", "shipped"),
                root.row("privacy", qsTr("Search"), qsTr("Search privacy"), "disclosure",
                         "settingsd -> search service", "search.indexLocations", "live-on-defaults", "shipped"),
                root.row("shortcut", qsTr("Search"), qsTr("Shortcut"), "shortcut-recorder",
                         "settingsd", "search.shortcut", "live-on-defaults", "shipped"),
                root.row("web", qsTr("Search"), qsTr("Web search"), "toggle",
                         "settingsd", "search.web", "live-on-defaults", "shipped"),
                root.row("clipboard", qsTr("Clipboard History"), qsTr("Keep clipboard history"), "toggle",
                         "settingsd", "search.clipboardHistory", "live-on-defaults", "shipped",
                         true, "stored-only: shell clipboard history always observes; no preference consumer yet"),
                root.row("status", qsTr("Search"), qsTr("Indexing status"), "info",
                         "search service", "search.status", "disabled-with-reason", "shipped"),
                root.row("rebuild", qsTr("Search"), qsTr("Rebuild index"), "button",
                         "search service", "search.rebuild", "disabled-with-reason", "shipped")
            ],
            "privacy": [
                root.row("locationMaster", qsTr("Location Services"),
                         qsTr("Location Services"), "toggle",
                         "settingsd", "privacy.locationServices",
                         "live-on-defaults", "shipped"),
                root.row("location", qsTr("Location Services"), qsTr("Location"), "disclosure",
                         "portal Location/Geoclue", "privacy.location", "delegated", "shipped",
                         false, "portal PermissionStore, Geoclue"),
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
                root.row("analytics", qsTr("Analytics & Improvements"),
                         qsTr("Share analytics with the distribution"), "toggle",
                         "settingsd", "privacy.analytics", "live-on-defaults",
                         "shipped", true),
                root.row("sources", qsTr("Security"), qsTr("Software sources"), "popup",
                         "distro adapter + Flatpak", "security.sources", "disabled-with-reason", "shipped",
                         false, "distro package manager / Flatpak remotes (owner absent)"),
                root.row("encryption", qsTr("Security"), qsTr("Disk encryption"), "info+open",
                         "LUKS adapter", "security.luks", "read-only", "shipped"),
                root.row("accessories", qsTr("Security"), qsTr("USB accessories"), "popup",
                         "USBGuard", "security.accessories", "disabled-with-reason", "shipped",
                         false, "USBGuard device policy (owner absent)"),
                root.row("hardening", qsTr("Security"), qsTr("Hardening mode"), "info",
                         "SELinux/AppArmor + Secure Boot", "security.hardening", "read-only", "shipped"),
                root.row("extensions", qsTr("Security"), qsTr("Extensions"), "list",
                         "portal extensions", "security.extensions", "read-only", "shipped"),
                root.row("advanced", qsTr("Security"), qsTr("Advanced..."), "sheet",
                         "adapters", "security.advanced", "read-only", "shipped")
            ],
            // T292a: the mockup's Dock / Desktop & Workspaces / Widgets /
            // Windows order. Section titles equal the rendered group titles.
            // The Linux-only Dock rows (`chooserOnHover`, `minimizeReaction`)
            // and the window-rules row keep their owners and keys but live in
            // the Advanced Dock / Advanced Windows sheets ("dialog" surface).
            // The former Overview group is the temporary **Workspace Overview**
            // entry; T292b merges the Mission Control rows into it.
            "desktop-dock": [
                root.row("size", qsTr("Dock"), qsTr("Dock Size"), "slider",
                         "settingsd", "dock.size", "live-on-defaults", "shipped"),
                root.row("magnification", qsTr("Dock"), qsTr("Dock Magnification Size"), "slider",
                         "settingsd", "dock.magnification", "live-on-defaults", "shipped"),
                root.row("position", qsTr("Dock"), qsTr("Dock position on screen"), "popup",
                         "settingsd", "dock.position", "live-on-defaults", "shipped"),
                root.row("minimizeAnimation", qsTr("Dock"), qsTr("Minimized window animation"), "popup",
                         "settingsd", "dock.minimizedAnimation", "live-on-defaults", "shipped"),
                root.row("titlebarDoubleClick", qsTr("Dock"), qsTr("Window title bar double-click action"), "popup",
                         "settingsd", "dock.titlebarDoubleClick", "live-on-defaults", "shipped"),
                root.row("minimizeIntoTileIcon", qsTr("Dock"), qsTr("Minimize windows into application icon"), "toggle",
                         "settingsd", "dock.minimizeIntoTileIcon", "live-on-defaults", "shipped"),
                root.row("autohide", qsTr("Dock"), qsTr("Automatically hide and show the Dock"), "toggle",
                         "settingsd", "dock.autohide", "live-on-defaults", "shipped"),
                root.row("animateOpening", qsTr("Dock"), qsTr("Animate opening applications"), "toggle",
                         "settingsd", "dock.animateOpening", "live-on-defaults", "shipped"),
                root.row("showIndicators", qsTr("Dock"), qsTr("Show indicators for open applications"), "toggle",
                         "settingsd", "dock.showIndicators", "live-on-defaults", "shipped"),
                root.row("showRecentApps", qsTr("Dock"), qsTr("Show suggested and recent apps in Dock"), "toggle",
                         "settingsd", "dock.showRecentApps", "live-on-defaults", "shipped"),
                root.row("desktopItems", qsTr("Desktop & Workspaces"), qsTr("Show items"), "segmented",
                         "settingsd -> apps/files desktop", "desktop.showItems", "live-on-defaults", "shipped"),
                root.row("clickWallpaper", qsTr("Desktop & Workspaces"), qsTr("Click wallpaper to show desktop"), "popup",
                         "settingsd", "desktop.clickWallpaper", "disabled-with-reason", "shipped"),
                root.row("stage", qsTr("Desktop & Workspaces"), qsTr("Workspace Focus"), "toggle",
                         "settingsd", "stageManager.enabled", "disabled-with-reason", "shipped"),
                root.row("stageRecent", qsTr("Desktop & Workspaces"), qsTr("Show recent apps in Workspace Focus"), "toggle",
                         "settingsd", "stageManager.showRecentApps", "disabled-with-reason", "shipped"),
                root.row("stageWindowMode", qsTr("Desktop & Workspaces"), qsTr("Show windows from an application"), "popup",
                         "compositor (T242)", "stageManager.windowMode", "disabled-with-reason", "shipped"),
                root.row("showWidgets", qsTr("Widgets"), qsTr("Show Widgets"), "segmented",
                         "shell + files desktop (T245b)", "widgets.show", "disabled-with-reason", "shipped"),
                root.row("dimWidgets", qsTr("Widgets"), qsTr("Dim widgets on desktop"), "popup",
                         "shell + files desktop (T245b)", "widgets.dim", "disabled-with-reason", "shipped"),
                root.row("defaultBrowser", qsTr("Widgets"), qsTr("Default web browser"), "popup",
                         "settingsd + default-apps delegate", "defaults.browser", "delegated", "shipped"),
                root.row("preferTabs", qsTr("Windows"), qsTr("Prefer tabs when opening documents"), "popup",
                         "first-party apps", "windows.preferTabs", "disabled-with-reason", "shipped"),
                root.row("askToKeepChanges", qsTr("Windows"), qsTr("Ask to keep changes when closing documents"), "toggle",
                         "first-party apps", "windows.askToKeepChanges", "disabled-with-reason", "shipped"),
                root.row("closeOnQuit", qsTr("Windows"), qsTr("Close windows when quitting an application"), "toggle",
                         "first-party app lifecycle (T243)", "windows.closeOnQuit", "disabled-with-reason", "shipped"),
                root.row("dragToTile", qsTr("Windows"), qsTr("Drag windows to left or right edge of screen to tile"), "toggle",
                         "compositor (T242)", "tiling.dragToTile", "disabled-with-reason", "shipped",
                         false, "compositor/window tiling (not implemented)"),
                root.row("dragToFill", qsTr("Windows"), qsTr("Drag windows to menu bar to fill screen"), "toggle",
                         "compositor (T242)", "tiling.dragToFill", "disabled-with-reason", "shipped",
                         false, "compositor/window tiling (not implemented)"),
                root.row("tileModifier", qsTr("Windows"), qsTr("Hold Super key while dragging windows to tile"), "toggle",
                         "compositor (T242)", "tiling.modifier", "disabled-with-reason", "shipped",
                         false, "compositor/window tiling (not implemented)"),
                root.row("tileMargins", qsTr("Windows"), qsTr("Tiled windows have margins"), "toggle",
                         "compositor (T242)", "tiling.margins", "disabled-with-reason", "shipped",
                         false, "compositor/window tiling (not implemented)"),
                root.row("overview", qsTr("Workspace Overview"), qsTr("Workspace Overview"), "disclosure",
                         "settingsd + compositor", "overview.*", "live-on-defaults", "shipped"),
                // T292a: the Shortcuts… and Hot Corners… actions are the mockup
                // footer, not pane rows. They keep their map ids under the
                // Window Overview section so a deep link lands on the live
                // Workspace Overview entry; the shortcut editor is still
                // disabled.
                root.row("hotCorners", qsTr("Workspace Overview"), qsTr("Hot Corners..."), "sheet",
                         "settingsd + compositor", "overview.hotCorner*", "live-on-defaults", "shipped"),
                root.row("shortcuts", qsTr("Workspace Overview"), qsTr("Shortcuts..."), "sheet",
                         "compositor", "shortcuts.*", "disabled-with-reason", "shipped"),
                // Advanced sheets.
                root.row("chooserOnHover", qsTr("Advanced Dock"), qsTr("Open a window chooser by hovering"), "toggle",
                         "settingsd", "dock.chooserOnHover", "live-on-defaults", "shipped"),
                root.row("minimizeReaction", qsTr("Advanced Dock"), qsTr("Bounce app icons when windows minimize"), "toggle",
                         "settingsd", "dock.minimizeReaction", "live-on-defaults", "shipped"),
                root.row("windowRules", qsTr("Advanced Windows"), qsTr("Application window rules..."), "button",
                         "compositor", "windows.rules", "disabled-with-reason", "shipped",
                         false, "compositor/window-rule engine (not implemented)")
            ],
            "mission-control": [
                root.row("rearrange", qsTr("Overview"), qsTr("Automatically rearrange workspaces"), "toggle",
                         "compositor (T240)", "overview.autoRearrange", "disabled-with-reason", "shipped"),
                root.row("switchToAppSpace", qsTr("Overview"), qsTr("Switch to an application's workspace"), "toggle",
                         "settingsd -> compositor", "overview.switchToAppSpace", "live-on-defaults", "shipped"),
                root.row("groupByApp", qsTr("Overview"), qsTr("Group windows by application"), "toggle",
                         "compositor (T240)", "overview.groupByApp", "disabled-with-reason", "shipped"),
                root.row("perDisplay", qsTr("Overview"), qsTr("Displays have separate workspaces"), "info",
                         "compositor", "overview.perDisplay", "read-only", "shipped",
                         false, "compositor/workspace model"),
                root.row("dragToOpen", qsTr("Overview"), qsTr("Drag windows to the top to open Overview"), "toggle",
                         "compositor (T240)", "overview.dragToOpen", "disabled-with-reason", "shipped"),
                root.row("workspaceCount", qsTr("Workspaces"), qsTr("Workspace count"), "popup",
                         "settingsd -> compositor", "workspaces.count", "live-on-defaults", "shipped",
                         false, "compositor/workspace model, shell"),
                root.row("swipeUp", qsTr("Workspaces"), qsTr("Swipe up to open"), "toggle",
                         "settingsd", "gestures.missionControl", "live-on-defaults", "shipped",
                         false, "compositor/input"),
                root.row("spaceSwitch", qsTr("Workspaces"), qsTr("Swipe between Workspaces"), "toggle",
                         "settingsd", "gestures.spaceSwitch", "live-on-defaults", "shipped",
                         false, "compositor/input"),
                root.row("topLeft", qsTr("Hot Corners"), qsTr("Top Left"), "popup",
                         "settingsd", "overview.hotCornerTopLeft", "live-on-defaults", "shipped"),
                root.row("topRight", qsTr("Hot Corners"), qsTr("Top Right"), "popup",
                         "settingsd", "overview.hotCornerTopRight", "live-on-defaults", "shipped"),
                root.row("bottomLeft", qsTr("Hot Corners"), qsTr("Bottom Left"), "popup",
                         "settingsd", "overview.hotCornerBottomLeft", "live-on-defaults", "shipped"),
                root.row("bottomRight", qsTr("Hot Corners"), qsTr("Bottom Right"), "popup",
                         "settingsd", "overview.hotCornerBottomRight", "live-on-defaults", "shipped"),
                root.row("shortcuts", qsTr("Shortcuts"), qsTr("Shortcuts..."), "sheet",
                         "compositor", "shortcuts.*", "disabled-with-reason", "shipped")
            ],
            "displays": [
                root.row("display", qsTr("Displays"), qsTr("Display"), "row+sheet",
                         "platform outputs", "display.perOutput", "live-on-defaults", "shipped",
                         false, "compositor/output (per-output sheet)"),
                root.row("perOutput", qsTr("Displays"), qsTr("Displays"), "list",
                         "platform outputs", "display.perOutput", "live-on-defaults", "shipped",
                         false, "compositor/output"),
                root.row("useAs", qsTr("Displays"), qsTr("Use as"), "popup",
                         "compositor", "display.useAs", "disabled-with-reason", "shipped"),
                root.row("scale", qsTr("Displays"), qsTr("Resolution"), "radios",
                         "settingsd + compositor", "display.scale", "live-on-defaults", "shipped"),
                root.row("showAllResolutions", qsTr("Displays"), qsTr("Show all resolutions"), "toggle",
                         "settingsd + compositor", "display.scale", "live-on-defaults", "shipped",
                         false, "the Resolution radio list (view filter, not persisted)"),
                root.row("resolutionList", qsTr("Displays"), qsTr("Resolution list"), "popup",
                         "compositor", "display.resolution", "disabled-with-reason", "shipped",
                         false, "compositor/output (pixel-mode list not advertised)"),
                root.row("brightness", qsTr("Displays"), qsTr("Brightness"), "slider",
                         "settingsd -> compositor", "display.brightness", "live-on-defaults", "shipped"),
                root.row("autoBrightness", qsTr("Displays"), qsTr("Auto-brightness"), "toggle",
                         "compositor sensor", "display.autoBrightness", "disabled-with-reason", "shipped"),
                root.row("adaptiveColor", qsTr("Displays"), qsTr("Ambient color"), "toggle",
                         "compositor sensor", "display.adaptiveColor", "disabled-with-reason", "shipped"),
                root.row("colorProfile", qsTr("Displays"), qsTr("Preset"), "popup",
                         "colord", "display.colorProfile", "disabled-with-reason", "shipped"),
                root.row("refreshRate", qsTr("Displays"), qsTr("Refresh rate"), "popup",
                         "compositor", "display.refreshRate", "disabled-with-reason", "shipped",
                         false, "compositor/output (refresh choice not advertised)"),
                root.row("tvMode", qsTr("Displays"), qsTr("Projector mode"), "popup",
                         "compositor", "display.tvMode", "disabled-with-reason", "shipped",
                         false, "compositor/output (projector mode not advertised)"),
                root.row("nightLightSheet", qsTr("Displays"), qsTr("Night Light..."), "sheet",
                         "compositor", "display.nightLight", "disabled-with-reason", "shipped",
                         false, "compositor/output (night-light pipeline not present)"),
                root.row("nightLight", qsTr("Displays"), qsTr("Night light"), "disclosure",
                         "compositor", "display.nightLight", "disabled-with-reason", "shipped",
                         false, "compositor/output (night-light pipeline not present)"),
                root.row("nightLightSchedule", qsTr("Displays"), qsTr("Night light schedule"), "popup",
                         "compositor", "display.nightLight.schedule", "disabled-with-reason", "shipped",
                         false, "compositor/output (night-light pipeline not present)"),
                root.row("nightLightTemperature", qsTr("Displays"), qsTr("Night light temperature"), "slider",
                         "compositor", "display.nightLight.temperature", "disabled-with-reason", "shipped",
                         false, "compositor/output (night-light pipeline not present)"),
                root.row("hdr", qsTr("Displays"), qsTr("High dynamic range"), "toggle",
                         "compositor", "display.hdr", "disabled-with-reason", "shipped",
                         false, "compositor/output (HDR not advertised)"),
                root.row("vrr", qsTr("Displays"), qsTr("Variable refresh rate"), "toggle",
                         "compositor", "display.vrr", "disabled-with-reason", "shipped",
                         false, "compositor/output (VRR not advertised)"),
                root.row("overscan", qsTr("Displays"), qsTr("Overscan"), "slider",
                         "compositor", "display.overscan", "disabled-with-reason", "shipped"),
                root.row("advanced", qsTr("Displays"), qsTr("Advanced..."), "sheet",
                         "compositor/colord", "display.advanced", "disabled-with-reason", "shipped"),
                root.row("arrangement", qsTr("Displays"), qsTr("Arrangement"), "canvas",
                         "compositor", "display.arrangement", "disabled-with-reason", "shipped"),
                root.row("rotation", qsTr("Displays"), qsTr("Rotation"), "popup",
                         "settingsd", "display.rotation", "live-on-defaults", "shipped")
            ],
            "wallpaper": [
                root.row("current", qsTr("Wallpaper"), qsTr("Current wallpaper"), "preview",
                         "settingsd", "wallpaper.source", "live-on-defaults", "shipped"),
                root.row("showAll", qsTr("Wallpaper"), qsTr("Show on all workspaces"), "toggle",
                         "settingsd", "wallpaper.showOnAllSpaces", "live-on-defaults", "shipped",
                         false, "shell/wallpaper forwarder, compositor/workspace model"),
                root.row("fit", qsTr("Wallpaper"), qsTr("Fit"), "popup",
                         "settingsd", "wallpaper.fit", "live-on-defaults", "shipped"),
                root.row("screenSaver", qsTr("Wallpaper"), qsTr("Screen saver..."), "button",
                         "settingsd", "screenSaver.style", "live-on-defaults", "shipped"),
                root.row("clock", qsTr("Wallpaper"), qsTr("Clock appearance..."), "button",
                         "settingsd", "screenSaver.clock", "live-on-defaults", "shipped"),
                root.row("abstract", qsTr("Online wallpapers"), qsTr("Dynamic"), "grid",
                         "wallpaperd (Wikimedia Commons)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("nature", qsTr("Online wallpapers"), qsTr("Landscape"), "grid",
                         "wallpaperd (Wikimedia Commons)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("water", qsTr("Online wallpapers"), qsTr("Water"), "grid",
                         "wallpaperd (Wikimedia Commons)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("cityscapes", qsTr("Online wallpapers"), qsTr("Cityscapes"), "grid",
                         "wallpaperd (Wikimedia Commons)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("earth", qsTr("Online wallpapers"), qsTr("Earth"), "grid",
                         "wallpaperd (Wikimedia Commons)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("space", qsTr("Online wallpapers"), qsTr("Space"), "grid",
                         "wallpaperd (NASA)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("scenery", qsTr("Online wallpapers"), qsTr("Scenery"), "grid",
                         "wallpaperd (Wikimedia Commons)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("underwater", qsTr("Online wallpapers"), qsTr("Underwater"), "grid",
                         "wallpaperd (Wikimedia Commons)", "wallpaper.provider",
                         "live-on-defaults", "shipped"),
                root.row("colors", qsTr("Colors"), qsTr("Colors"), "grid",
                         "shipped assets", "wallpaper.source", "live-on-defaults", "shipped"),
                root.row("shuffle", qsTr("Shuffle"), qsTr("Shuffle"), "grid",
                         "wallpaperd", "wallpaper.provider", "live-on-defaults", "shipped",
                         false, "wallpaperd (applies the chosen Featured image)"),
                root.row("slideshow", qsTr("Slideshow"), qsTr("Change wallpaper every"), "popup",
                         "settingsd", "wallpaper.slideshow", "live-on-defaults", "shipped",
                         true, "stored-only: the shell has no wallpaper-rotation timer yet"),
                root.row("photos", qsTr("Your Photos"), qsTr("Your photos"), "grid",
                         "FileChooser portal", "wallpaper.photo", "delegated", "shipped"),
                root.row("builtin", qsTr("Built-in"), qsTr("Built-in"), "grid",
                         "shipped assets", "wallpaper.builtinDefault", "live-on-defaults", "shipped")
            ],
            "screen-saver": [
                root.row("style", qsTr("Screen Saver"), qsTr("Screen saver"), "grid",
                         "settingsd/shell", "screenSaver.style", "live-on-defaults", "shipped"),
                root.row("startAfter", qsTr("Screen Saver"), qsTr("Start after"), "popup",
                         "settingsd", "idle.blank", "live-on-defaults", "shipped"),
                root.row("clock", qsTr("Screen Saver"), qsTr("Show with clock"), "toggle",
                         "settingsd", "screenSaver.clock", "live-on-defaults", "shipped"),
                root.row("showDate", qsTr("Screen Saver"), qsTr("Show date"), "toggle",
                         "settingsd", "screenSaver.showDate", "live-on-defaults", "shipped"),
                root.row("showSeconds", qsTr("Screen Saver"), qsTr("Show seconds"), "toggle",
                         "settingsd", "screenSaver.showSeconds", "live-on-defaults", "shipped"),
                root.row("options", qsTr("Screen Saver"), qsTr("Options..."), "sheet",
                         "settingsd", "screenSaver.clock/showDate/showSeconds", "live-on-defaults", "shipped"),
                root.row("hotCorners", qsTr("Screen Saver"), qsTr("Hot corners..."), "sheet",
                         "settingsd", "overview.hotCorner*", "live-on-defaults", "shipped")
            ],
            "notifications": [
                root.row("previews", qsTr("Notifications"), qsTr("Show previews"), "popup",
                         "settingsd", "notifications.showPreviews", "live-on-defaults", "shipped", true),
                root.row("sleeping", qsTr("Notifications"), qsTr("When display is asleep"), "toggle",
                         "settingsd", "notifications.showWhenSleeping", "live-on-defaults", "shipped", true),
                root.row("locked", qsTr("Notifications"), qsTr("When locked"), "toggle",
                         "settingsd", "notifications.showWhenLocked", "live-on-defaults", "shipped", true),
                root.row("sharing", qsTr("Notifications"), qsTr("When sharing"), "popup",
                         "settingsd", "notifications.showWhenMirroring", "live-on-defaults", "shipped", true),
                root.row("apps", qsTr("Notifications"), qsTr("App notifications"), "list",
                         "notification service", "notifications.apps", "disabled-with-reason", "shipped"),
                root.row("allow", qsTr("Notifications"), qsTr("Allow"), "toggle",
                         "notification service", "notifications.allow", "disabled-with-reason", "shipped"),
                root.row("style", qsTr("Notifications"), qsTr("Style"), "popup",
                         "notification service", "notifications.style", "disabled-with-reason", "shipped"),
                root.row("lockScreen", qsTr("Notifications"), qsTr("Show on lock screen"), "toggle",
                         "settingsd", "notifications.appOptions", "live-on-defaults", "shipped", true),
                root.row("centerCount", qsTr("Notifications"), qsTr("Notifications count"), "popup",
                         "settingsd", "notifications.appOptions", "live-on-defaults", "shipped", true),
                root.row("appPreviews", qsTr("Notifications"), qsTr("App previews"), "popup",
                         "settingsd", "notifications.appOptions", "live-on-defaults", "shipped", true),
                root.row("grouping", qsTr("Notifications"), qsTr("Notification grouping"), "popup",
                         "settingsd", "notifications.appOptions", "live-on-defaults", "shipped", true),
                root.row("sound", qsTr("Notifications"), qsTr("Sound"), "toggle",
                         "notification service", "notifications.sound", "disabled-with-reason", "shipped"),
                root.row("badge", qsTr("Notifications"), qsTr("Badge"), "toggle",
                         "notification service", "notifications.badge", "disabled-with-reason", "shipped"),
                root.row("focus", qsTr("Notifications"), qsTr("Focus"), "link",
                         "settingsd", "notifications.focus", "live-on-defaults", "shipped")
            ],
            "sound": [
                root.row("alertSound", qsTr("Sound Effects"), qsTr("Alert sound"), "popup+play",
                         "settingsd + audio preview", "sound.alertSound", "live-on-defaults", "shipped"),
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
                root.row("outputVolume", qsTr("Output & Input"), qsTr("Output volume"), "slider",
                         "PipeWire", "audio.volume", "disabled-with-reason", "shipped"),
                root.row("outputMute", qsTr("Output & Input"), qsTr("Mute"), "toggle",
                         "PipeWire", "audio.muted", "disabled-with-reason", "shipped"),
                root.row("inputVolume", qsTr("Output & Input"), qsTr("Input volume"), "slider",
                         "PipeWire", "audio.sourceVolume", "disabled-with-reason", "shipped"),
                root.row("inputMute", qsTr("Output & Input"), qsTr("Input mute"), "toggle",
                         "PipeWire", "audio.sourceMuted", "disabled-with-reason", "shipped"),
                root.row("test", qsTr("Output & Input"), qsTr("Test"), "button",
                         "PipeWire", "audio.test", "disabled-with-reason", "shipped"),
                root.row("balance", qsTr("Output & Input"), qsTr("Balance"), "slider",
                         "settingsd", "sound.balance", "live-on-defaults", "shipped", true),
                root.row("perApp", qsTr("Output & Input"), qsTr("Per-app volume"), "list",
                         "PipeWire", "audio.streams", "disabled-with-reason", "shipped",
                         false, "PipeWire (control CLI exposes no stream levels)")
            ],
            "focus": [
                root.row("list", qsTr("Focus"), qsTr("Focus list"), "list",
                         "notification service", "focus.modes", "disabled-with-reason", "shipped"),
                root.row("add", qsTr("Focus"), qsTr("Add Focus..."), "sheet",
                         "settingsd", "focus.customProfiles", "live-on-defaults", "shipped"),
                root.row("delete", qsTr("Focus"), qsTr("Delete Focus..."), "button",
                         "settingsd", "focus.customProfiles", "live-on-defaults", "shipped"),
                root.row("people", qsTr("Focus"), qsTr("Allowed people"), "list",
                         "notification service", "focus.people", "disabled-with-reason", "shipped"),
                root.row("apps", qsTr("Focus"), qsTr("Allowed apps"), "list",
                         "notification service", "focus.apps", "disabled-with-reason", "shipped"),
                root.row("filters", qsTr("Focus"), qsTr("Filters"), "popup",
                         "apps/settings", "focus.apps/focus.people", "live-on-defaults", "shipped"),
                root.row("schedule", qsTr("Focus"), qsTr("Schedule"), "sheet",
                         "settingsd", "focus.schedule.*", "live-on-defaults", "shipped"),
                root.row("shortcut", qsTr("Focus"), qsTr("Shortcut"), "recorder",
                         "settingsd", "focus.shortcut", "live-on-defaults", "shipped", true)
            ],
            "lock-screen": [
                root.row("idleDim", qsTr("Lock Screen"), qsTr("Dim screen when inactive"), "popup",
                         "settingsd -> session/idle", "idle.dim", "live-on-defaults", "shipped",
                         false, "session/idle engine"),
                root.row("displayBattery", qsTr("Lock Screen"), qsTr("Display off on battery"), "popup",
                         "settingsd", "idle.blank", "live-on-defaults", "shipped",
                         false, "session/idle engine"),
                root.row("displayAC", qsTr("Lock Screen"), qsTr("Display off on AC"), "popup",
                         "settingsd", "idle.blank", "live-on-defaults"),
                root.row("powerStage", qsTr("Lock Screen"), qsTr("One display-off stage"), "info",
                         "apps/settings", "idle.blank", "live-on-defaults", "shipped"),
                root.row("idleSuspend", qsTr("Lock Screen"), qsTr("Suspend when inactive"), "popup",
                         "settingsd -> session/idle", "idle.suspend", "live-on-defaults", "shipped",
                         false, "session/idle engine, session/suspend"),
                root.row("requirePassword", qsTr("Lock Screen"), qsTr("Require password"), "popup",
                         "settingsd", "lock.requirePasswordAfter", "live-on-defaults", "shipped",
                         false, "session/idle engine (derives idle.lock)"),
                root.row("preview", qsTr("Lock Screen"), qsTr("Lock screen preview"), "preview",
                         "shell", "lock.*", "live-on-defaults", "shipped"),
                root.row("userNamePhoto", qsTr("Lock Screen"), qsTr("Show user name/photo"), "toggle",
                         "settingsd", "lock.showUserNameAndPhoto", "live-on-defaults", "shipped"),
                root.row("passwordHints", qsTr("Lock Screen"), qsTr("Show password hints"), "toggle+set",
                         "settingsd", "lock.showPasswordHints/lock.passwordHint", "live-on-defaults", "shipped"),
                root.row("message", qsTr("Lock Screen"), qsTr("Lock message"), "toggle+set",
                         "settingsd", "lock.showMessageWhenLocked/lock.message", "live-on-defaults", "shipped"),
                root.row("loginWindow", qsTr("Lock Screen"), qsTr("Login window"), "radio",
                         "settingsd", "login.windowStyle", "disabled-with-reason", "shipped"),
                root.row("powerButtons", qsTr("Lock Screen"), qsTr("Power buttons"), "toggle",
                         "settingsd", "lock.showPowerButtons", "live-on-defaults", "shipped"),
                root.row("accessibility", qsTr("Lock Screen"), qsTr("Accessibility options..."), "button",
                         "settingsd", "a11y.lockOptions", "live-on-defaults", "shipped")
            ],
            "biometrics": [
                root.row("password", qsTr("Password"), qsTr("Change password..."), "button",
                         "AccountsService + polkit", "auth.changePassword", "disabled-with-reason", "shipped"),
                root.row("fingerprints", qsTr("Fingerprints"), qsTr("Fingerprints"), "list+add",
                         "fprintd", "biometrics.fingerprints", "disabled-with-reason", "shipped"),
                root.row("enrollWizard", qsTr("Fingerprints"),
                         qsTr("Add Fingerprint..."), "wizard",
                         "fprintd", "biometrics.enrollment", "disabled-with-reason", "shipped"),
                root.row("unlock", qsTr("Fingerprints"), qsTr("Use fingerprint to unlock"), "toggle",
                         "PAM", "biometrics.unlock", "read-only", "shipped"),
                root.row("auth", qsTr("Fingerprints"), qsTr("Use fingerprint for authentication"), "toggle",
                         "PAM", "biometrics.auth", "read-only", "shipped"),
                root.row("userSwitching", qsTr("Fingerprints"), qsTr("Fingerprint for user switching"), "toggle",
                         "omit", "omit", "omitted", "omitted"),
                root.row("purchases", qsTr("Fingerprints"), qsTr("Fingerprint purchases"), "omit",
                         "omit", "omit", "omitted", "omitted")
            ],
            "users-groups": [
                root.row("users", qsTr("Users & Groups"), qsTr("Users"), "list",
                         "accountsservice", "users.list", "read-only", "shipped"),
                root.row("guest", qsTr("Users & Groups"), qsTr("Guest user"), "toggle",
                         "distro adapter", "users.guest", "hidden"),
                root.row("addUser", qsTr("Users & Groups"), qsTr("Add user..."),
                         "sheet+password", "distro adapter", "users.add",
                         "disabled-with-reason", "shipped"),
                root.row("password", qsTr("Users & Groups"), qsTr("Change/reset password"),
                         "button", "accountsservice/polkit", "users.password",
                         "disabled-with-reason", "shipped"),
                root.row("addGroup", qsTr("Users & Groups"), qsTr("Add group..."), "sheet",
                         "distro adapter", "users.addGroup", "disabled-with-reason", "shipped"),
                root.row("autoLogin", qsTr("Users & Groups"), qsTr("Automatic login"), "popup",
                         "display-manager adapter", "users.autologin", "disabled-with-reason", "shipped"),
                root.row("directory", qsTr("Users & Groups"), qsTr("Directory account"), "button",
                         "SSSD/FreeIPA adapter", "users.directory", "delegated", "shipped"),
                root.row("adminGroup", qsTr("Users & Groups"), qsTr("Admin group"), "info",
                         "distro adapter", "users.adminGroup", "read-only", "shipped"),
                root.row("deleteGroup", qsTr("Users & Groups"), qsTr("Delete group"),
                         "confirm", "distro adapter", "users.deleteGroup",
                         "disabled-with-reason", "shipped")
            ],
            "online-accounts": [
                root.row("list", qsTr("Online Accounts"), qsTr("Online accounts"), "list",
                         "Online Accounts service", "onlineAccounts.list", "delegated", "shipped"),
                root.row("absent", qsTr("Online Accounts"),
                         qsTr("Online accounts are not installed"), "button",
                         "distribution package", "onlineAccounts.absent",
                         "disabled-with-reason", "shipped"),
                root.row("add", qsTr("Online Accounts"), qsTr("Add account..."), "sheet",
                         "Online Accounts service", "onlineAccounts.add", "delegated", "shipped"),
                root.row("sync", qsTr("Online Accounts"), qsTr("Sync toggles"), "toggles",
                         "Online Accounts service", "onlineAccounts.sync", "delegated", "shipped"),
                root.row("details", qsTr("Online Accounts"), qsTr("Details..."), "sheet",
                         "Online Accounts service", "onlineAccounts.details", "delegated", "shipped"),
                root.row("signIn", qsTr("Online Accounts"), qsTr("Sign in again"), "button",
                         "Online Accounts service", "onlineAccounts.signIn", "delegated", "shipped"),
                root.row("remove", qsTr("Online Accounts"), qsTr("Remove account"), "button",
                         "Online Accounts service", "onlineAccounts.remove", "delegated", "shipped")
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
                         "CUPS", "printers.options", "disabled-with-reason", "shipped"),
                root.row("scanners", qsTr("Printers & Scanners"), qsTr("Scanners"), "list",
                         "SANE", "printers.scanners", "disabled-with-reason", "shipped"),
                root.row("remove", qsTr("Printers & Scanners"), qsTr("Remove printer"), "confirm",
                         "CUPS", "printers.remove", "disabled-with-reason", "shipped"),
                root.row("test", qsTr("Printers & Scanners"), qsTr("Print test page"), "button",
                         "CUPS", "printers.testPage", "disabled-with-reason", "shipped"),
                root.row("share", qsTr("Printers & Scanners"), qsTr("Share this printer"), "toggle",
                         "CUPS", "printers.shared", "disabled-with-reason", "shipped"),
                root.row("scannerAction", qsTr("Printers & Scanners"), qsTr("Open scanner"),
                         "button", "SANE", "printers.openScanner", "delegated", "shipped")
            ],
            "keyboard": [
                root.row("repeatRate", qsTr("Keyboard"), qsTr("Key repeat rate"), "slider",
                         "settingsd", "input.repeatRate", "live-on-defaults", "shipped"),
                root.row("repeatDelay", qsTr("Keyboard"), qsTr("Repeat delay"), "slider",
                         "settingsd", "input.repeatDelay", "live-on-defaults", "shipped"),
                root.row("autoBrightness", qsTr("Keyboard"), qsTr("Auto keyboard brightness"), "toggle",
                         "settingsd", "input.adjustBrightnessLowLight", "disabled-with-reason", "shipped", true),
                root.row("brightness", qsTr("Keyboard"), qsTr("Keyboard brightness"), "slider",
                         "settingsd", "input.keyboardBrightness", "disabled-with-reason", "shipped", true),
                root.row("backlightOff", qsTr("Keyboard"), qsTr("Backlight off after"), "popup",
                         "settingsd", "input.backlightOffAfter", "disabled-with-reason", "shipped", true),
                root.row("superKey", qsTr("Keyboard"), qsTr("Press Super key to"), "popup",
                         "settingsd", "input.emojiKeyAction", "live-on-defaults", "shipped", true),
                root.row("navigation", qsTr("Keyboard"), qsTr("Keyboard navigation"), "toggle",
                         "settingsd", "input.keyboardNavigation", "live-on-defaults", "shipped", true),
                root.row("shortcuts", qsTr("Keyboard"), qsTr("Keyboard shortcuts..."), "sheet",
                         "shortcut engine", "shortcuts.*", "delegated", "shipped"),
                root.row("sources", qsTr("Keyboard"), qsTr("Input sources"), "popup+edit",
                         "ibus/fcitx5 + localectl", "input.sources", "delegated", "shipped",
                         false, "shell/compositor keymap, ibus/fcitx5 delegate"),
                root.row("inputMethod", qsTr("Keyboard"), qsTr("Input method"), "button",
                         "ibus/fcitx5", "input.method", "disabled-with-reason", "shipped",
                         false, "ibus/fcitx5 own configuration (not edited by Settings)"),
                root.row("composeKey", qsTr("Keyboard"), qsTr("Compose key"), "button",
                         "compositor/input (XKB)", "input.composeKey", "disabled-with-reason", "shipped",
                         false, "compositor/input keyboard layout (XKB option not exposed)"),
                root.row("replacements", qsTr("Keyboard"), qsTr("Text replacements..."), "sheet",
                         "settingsd/IME", "input.replacements", "live-on-defaults", "shipped"),
                root.row("dictation", qsTr("Keyboard"), qsTr("Dictation"), "toggle+options",
                         "speech-to-text delegate", "input.dictation", "delegated", "shipped"),
                root.row("dictationOptions", qsTr("Keyboard"), qsTr("Dictation options"), "sheet",
                         "speech-to-text delegate + shortcut engine",
                         "input.dictationLanguage/input.dictationMicrophone/input.dictationShortcut/input.dictationAutoPunctuation",
                         "delegated", "shipped")
            ],
            "mouse": [
                root.row("speed", qsTr("Mouse"), qsTr("Pointer speed"), "slider",
                         "settingsd", "input.mouse.pointerSpeed", "live-on-defaults", "shipped", true),
                root.row("natural", qsTr("Mouse"), qsTr("Natural scrolling"), "toggle",
                         "settingsd", "input.mouse.naturalScroll", "live-on-defaults", "shipped", true),
                root.row("acceleration", qsTr("Mouse"), qsTr("Pointer acceleration"), "toggle",
                         "settingsd", "input.mouse.pointerAcceleration", "live-on-defaults", "shipped", true),
                root.row("smartZoom", qsTr("Mouse"), qsTr("Smart zoom"), "toggle",
                         "settingsd", "input.mouse.smartZoom", "live-on-defaults", "shipped", true),
                root.row("secondary", qsTr("Mouse"), qsTr("Secondary click"), "popup",
                         "settingsd", "input.mouse.secondaryClick", "live-on-defaults", "shipped", true),
                root.row("scrollSpeed", qsTr("Mouse"), qsTr("Scroll speed"), "slider",
                         "settingsd", "input.mouse.scrollSpeed", "live-on-defaults", "shipped", true),
                root.row("doubleClick", qsTr("Mouse"), qsTr("Double-click speed"), "slider",
                         "settingsd", "input.mouse.doubleClickSpeed", "live-on-defaults", "shipped", true),
                root.row("leftHanded", qsTr("Mouse"), qsTr("Left-handed"), "toggle",
                         "settingsd", "input.mouse.leftHanded", "live-on-defaults", "shipped", true),
                root.row("scrollMethod", qsTr("Mouse"), qsTr("Scroll method"), "popup",
                         "settingsd", "input.mouse.scrollMethod", "live-on-defaults", "shipped", true),
                root.row("gestures", qsTr("Mouse"), qsTr("More gestures"), "group",
                         "settingsd", "gestures.pages/gestures.fullscreen/gestures.notifications/gestures.appWindows/gestures.showDesktop",
                         "live-on-defaults", "shipped", true)
            ],
            "trackpad": [
                root.row("speed", qsTr("Trackpad"), qsTr("Pointer speed"), "slider",
                         "settingsd", "input.trackpad.pointerSpeed", "live-on-defaults", "shipped", true),
                root.row("pressure", qsTr("Trackpad"), qsTr("Click pressure"), "slider",
                         "settingsd", "input.trackpad.clickPressure", "live-on-defaults", "shipped", true),
                root.row("haptics", qsTr("Trackpad"), qsTr("Haptic feedback"), "toggle",
                         "settingsd", "input.haptics", "live-on-defaults", "shipped"),
                root.row("lookUp", qsTr("Trackpad"), qsTr("Look up"), "popup",
                         "settingsd", "input.lookUp", "live-on-defaults", "shipped"),
                root.row("secondary", qsTr("Trackpad"), qsTr("Secondary click"), "popup",
                         "settingsd", "input.trackpad.secondaryClick", "live-on-defaults", "shipped", true),
                root.row("tap", qsTr("Trackpad"), qsTr("Tap to click"), "toggle",
                         "settingsd", "input.tapToClick", "live-on-defaults", "shipped"),
                root.row("scrollZoom", qsTr("Trackpad"), qsTr("Scroll & zoom"), "tab",
                         "settingsd", "input.trackpad.naturalScroll/input.trackpad.pinchZoom/input.trackpad.smartZoom/input.trackpad.rotate",
                         "live-on-defaults", "shipped", true),
                root.row("gestures", qsTr("Trackpad"), qsTr("More gestures"), "tab",
                         "settingsd", "gestures.enabled/gestures.spaceSwitch/gestures.missionControl/gestures.pages/gestures.fullscreen/gestures.notifications/gestures.appWindows/gestures.showDesktop",
                         "live-on-defaults", "shipped",
                         false, "compositor/input (live: enabled/spaceSwitch/missionControl; stored-only: the rest)"),
                root.row("pair", qsTr("Trackpad"), qsTr("Pair trackpad..."), "button",
                         "BlueZ", "input.pairTrackpad", "disabled-with-reason", "shipped")
            ],
            "game-controllers": [
                root.row("list", qsTr("Game Controllers"), qsTr("Controllers"), "list",
                         "udev/SDL", "controllers.list", "disabled-with-reason", "shipped"),
                root.row("mapping", qsTr("Game Controllers"), qsTr("Mapping"), "sheet",
                         "SDL mapping", "controllers.mapping", "delegated", "shipped"),
                root.row("test", qsTr("Game Controllers"), qsTr("Test"), "button",
                         "SDL", "controllers.test", "disabled-with-reason", "shipped"),
                root.row("rumble", qsTr("Game Controllers"), qsTr("Rumble test"), "button",
                         "SDL", "controllers.rumble", "disabled-with-reason", "shipped")
            ]
        });
    }

    // T281: the audited Linux-specific additions from
    // `docs/design/24-system-settings.md` §5. Each entry names the row that
    // gives the setting a home in a macOS-familiar pane; the row itself owns
    // the owner/key/absence/consumer contract. The validator proves the audit
    // and the executable map cannot drift apart.
    readonly property var linuxAdditions: [
        { area: qsTr("Wallpaper"), setting: qsTr("Per-workspace wallpaper"),
          pane: "wallpaper", rows: ["showAll"], task: "T278" },
        { area: qsTr("Wallpaper"), setting: qsTr("Slideshow / shuffle"),
          pane: "wallpaper", rows: ["shuffle"], task: "T278" },
        { area: qsTr("Wallpaper"), setting: qsTr("Slideshow interval"),
          pane: "wallpaper", rows: ["slideshow"], task: "T281" },
        { area: qsTr("Desktop"), setting: qsTr("Drag to tile"),
          pane: "desktop-dock", rows: ["dragToTile"], task: "T276" },
        { area: qsTr("Desktop"), setting: qsTr("Drag to fill"),
          pane: "desktop-dock", rows: ["dragToFill"], task: "T276" },
        { area: qsTr("Desktop"), setting: qsTr("Tiling modifier"),
          pane: "desktop-dock", rows: ["tileModifier"], task: "T276" },
        { area: qsTr("Desktop"), setting: qsTr("Tiling gaps"),
          pane: "desktop-dock", rows: ["tileMargins"], task: "T276" },
        { area: qsTr("Desktop"), setting: qsTr("Application window rules"),
          pane: "desktop-dock", rows: ["windowRules"], task: "T281" },
        { area: qsTr("Desktop"), setting: qsTr("Global app menu"),
          pane: "menu-bar", rows: ["globalMenu"], task: "T276" },
        // T285 removed the top-level Overview pane. Its workspace rows move to
        // Desktop & Dock -> Workspace Overview in T292; the audit entries for
        // workspace count and per-display workspaces return with that body.
        { area: qsTr("Appearance"), setting: qsTr("Icon theme"),
          pane: "appearance", rows: ["iconTheme"], task: "T274" },
        { area: qsTr("Appearance"), setting: qsTr("Cursor theme"),
          pane: "appearance", rows: ["cursorTheme"], task: "T274" },
        { area: qsTr("Appearance"), setting: qsTr("Cursor size"),
          pane: "appearance", rows: ["cursorSize"], task: "T274" },
        { area: qsTr("Appearance"), setting: qsTr("Window control layout"),
          pane: "appearance", rows: ["windowControls"], task: "T274" },
        { area: qsTr("Appearance"), setting: qsTr("Material, performance, rollout"),
          pane: "appearance", rows: ["material"], task: "T274" },
        { area: qsTr("Displays"), setting: qsTr("Per-output configuration"),
          pane: "displays", rows: ["perOutput"], task: "T273" },
        { area: qsTr("Displays"), setting: qsTr("Night light"),
          pane: "displays", rows: ["nightLight"], task: "T273" },
        { area: qsTr("Displays"), setting: qsTr("Night light schedule"),
          pane: "displays", rows: ["nightLightSchedule"], task: "T273" },
        { area: qsTr("Displays"), setting: qsTr("Night light temperature"),
          pane: "displays", rows: ["nightLightTemperature"], task: "T273" },
        { area: qsTr("Displays"), setting: qsTr("High dynamic range"),
          pane: "displays", rows: ["hdr"], task: "T273" },
        { area: qsTr("Displays"), setting: qsTr("Variable refresh rate"),
          pane: "displays", rows: ["vrr"], task: "T273" },
        { area: qsTr("Displays"), setting: qsTr("Refresh rate"),
          pane: "displays", rows: ["refreshRate"], task: "T273" },
        { area: qsTr("Sound"), setting: qsTr("Per-app volume"),
          pane: "sound", rows: ["perApp"], task: "T280" },
        { area: qsTr("Keyboard"), setting: qsTr("Input method"),
          pane: "keyboard", rows: ["inputMethod"], task: "T281" },
        { area: qsTr("Keyboard"), setting: qsTr("Compose key"),
          pane: "keyboard", rows: ["composeKey"], task: "T281" },
        { area: qsTr("Keyboard"), setting: qsTr("Layout, model, options"),
          pane: "keyboard", rows: ["sources"], task: "T280" },
        { area: qsTr("Trackpad"), setting: qsTr("Gesture mapping"),
          pane: "trackpad", rows: ["gestures"], task: "T280" },
        { area: qsTr("Power"), setting: qsTr("Charge threshold"),
          pane: "battery", rows: ["adapter.chargeLimit"], task: "T273" },
        { area: qsTr("Session"), setting: qsTr("Dim screen stage"),
          pane: "lock-screen", rows: ["idleDim"], task: "T281" },
        { area: qsTr("Session"), setting: qsTr("Display-off stage"),
          pane: "lock-screen", rows: ["displayBattery"], task: "T278" },
        { area: qsTr("Session"), setting: qsTr("Lock stage"),
          pane: "lock-screen", rows: ["requirePassword"], task: "T278" },
        { area: qsTr("Session"), setting: qsTr("Suspend stage"),
          pane: "lock-screen", rows: ["idleSuspend"], task: "T281" },
        { area: qsTr("Network"), setting: qsTr("Firewall rules and zones"),
          pane: "network", rows: ["firewallOptions"], task: "T272" },
        { area: qsTr("Network"), setting: qsTr("Bridge, bond, VLAN"),
          pane: "network", rows: ["addService"], task: "T272" },
        { area: qsTr("Network"), setting: qsTr("WireGuard"),
          pane: "vpn", rows: ["vpn.wireguard"], task: "T272" },
        { area: qsTr("Updates"), setting: qsTr("Package sources and Flatpak remotes"),
          pane: "privacy", rows: ["sources"], task: "T269" },
        { area: qsTr("Backups"), setting: qsTr("Backup backend"),
          pane: "general", rows: ["backup.backend"], task: "T270" },
        { area: qsTr("Privacy"), setting: qsTr("Portal permission tables"),
          pane: "privacy", rows: ["location"], task: "T279" },
        { area: qsTr("Privacy"), setting: qsTr("USB accessories"),
          pane: "privacy", rows: ["accessories"], task: "T279" },
        { area: qsTr("Search"), setting: qsTr("Clipboard history"),
          pane: "search", rows: ["clipboard"], task: "T281" },
        { area: qsTr("Menu Bar"), setting: qsTr("Tray overflow"),
          pane: "menu-bar", rows: ["overflow"], task: "T277" },
        { area: qsTr("Menu Bar"), setting: qsTr("Workspace indicator"),
          pane: "menu-bar", rows: ["workspaces"], task: "T277" }
    ]

    // Validate the Linux-specific audit against the executable rows. Every
    // entry must resolve to a pane and to shipped rows with an owner, key,
    // absence path, and a consumer (or an explicit stored-only mark).
    function validateLinuxAdditions() {
        var errors = [];
        var seenSettings = {};
        for (var i = 0; i < root.linuxAdditions.length; ++i) {
            var entry = root.linuxAdditions[i];
            var name = entry !== null && entry.setting !== undefined
                    ? String(entry.setting) : ("#" + i);
            if (!entry || !entry.area || !entry.setting || !entry.task)
                errors.push("linux addition " + name + " is missing area/setting/task");
            if (seenSettings[name] === true)
                errors.push("duplicate linux setting " + name);
            seenSettings[name] = true;
            if (root.pane(entry.pane) === null)
                errors.push("linux addition " + name + " names unknown pane " + entry.pane);
            if (!entry.rows || entry.rows.length === 0) {
                errors.push("linux addition " + name + " has no row home");
                continue;
            }
            for (var r = 0; r < entry.rows.length; ++r) {
                var rowId = entry.rows[r];
                var mapped = root.rowFor(entry.pane, rowId);
                if (mapped === null) {
                    errors.push("linux addition " + name + " names missing row "
                                + entry.pane + "/" + rowId);
                    continue;
                }
                if (mapped.state !== "shipped" && mapped.state !== "omitted")
                    errors.push("linux row " + entry.pane + "/" + rowId
                                + " is not a live or explicit omission");
                if (!mapped.owner || mapped.owner === "omit")
                    errors.push("linux row " + entry.pane + "/" + rowId + " has no owner");
                if (!mapped.key || mapped.key === "omit")
                    errors.push("linux row " + entry.pane + "/" + rowId + " has no key");
                if (root.absenceKinds.indexOf(mapped.absence) < 0)
                    errors.push("linux row " + entry.pane + "/" + rowId
                                + " has an unknown absence");
                if (!mapped.consumer || String(mapped.consumer).length === 0) {
                    errors.push("linux row " + entry.pane + "/" + rowId
                                + " records no consumer");
                } else if (mapped.storedOnly
                           && String(mapped.consumer).indexOf("stored-only") !== 0) {
                    errors.push("stored-only linux row " + entry.pane + "/" + rowId
                                + " does not say stored-only");
                } else if (!mapped.storedOnly
                           && mapped.absence === "live-on-defaults"
                           && String(mapped.consumer).indexOf("stored-only") === 0) {
                    errors.push("live linux row " + entry.pane + "/" + rowId
                                + " is described as stored-only");
                }
            }
        }
        return errors;
    }

    function linuxAdditionFor(setting) {
        for (var i = 0; i < root.linuxAdditions.length; ++i) {
            if (root.linuxAdditions[i].setting === setting)
                return root.linuxAdditions[i];
        }
        return null;
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

    // The General group is one pane with in-place sub-panes. A row's surface is
    // the sub-pane that owns it, a dialog that has to be opened first, or the
    // pane itself. Prefixes are stable map ids, so this table survives a
    // localized section title.
    readonly property var generalRowSubPanes: ({
        "about": "about",
        "updates": "software-update",
        "storage": "storage",
        "passwords": "passwords",
        "login": "login",
        "language": "language",
        "time": "date-time",
        "sharing": "sharing",
        "backup": "backup",
        "startup": "startup",
        "reset": "reset"
    })

    // T291a: the Accessibility big list is one pane with in-place sub-panes.
    // Each child adds its own prefix as it lands a body; a row whose prefix is
    // absent stays unlandable instead of deep-linking into a missing surface.
    readonly property var accessibilityRowSubPanes: ({
        "vision": "vision",
        "motor": "motor",
        "hearing": "hearing",
        "speech": "speech",
        "shortcut": "general"
    })

    // T291a: a section deep link (the Appearance footer's "More in
    // Accessibility Settings…" names Display) opens the sub-pane that owns the
    // section. Keys are normalized section ids, so the match is
    // case-insensitive.
    readonly property var accessibilitySectionSubPanes: ({
        "screen reader": "vision",
        "zoom": "vision",
        "display": "vision",
        "motion": "vision",
        "read & speak": "vision",
        "voice control": "motor",
        "keyboard": "motor",
        "pointer control": "motor",
        "switch control": "motor",
        "hearing devices": "hearing",
        "audio": "hearing",
        "subtitles and captioning": "hearing",
        "live captions": "hearing",
        "spoken name": "hearing",
        "live speech": "speech",
        "custom voice": "speech",
        "vocal shortcuts": "speech",
        "accessibility shortcut": "general",
        "shortcut": "general"
    })

    // The sub-pane id that renders a section, or "" when the section is on the
    // pane itself.
    function subPaneForSection(paneId, section) {
        if (paneId !== "accessibility" || !section)
            return "";
        var wanted = String(section).trim().toLowerCase();
        var mapped = root.accessibilitySectionSubPanes[wanted];
        return mapped !== undefined ? mapped : "";
    }

    // The sub-pane id that renders a row, or "" when the row is not inside an
    // in-place General or Accessibility sub-pane.
    function subPaneForRow(paneId, rowId) {
        if ((paneId !== "general" && paneId !== "accessibility") || !rowId)
            return "";
        var row = root.rowFor(paneId, rowId);
        if (row === null)
            return "";
        if (paneId === "accessibility") {
            var accessDot = String(row.id).indexOf(".");
            if (accessDot <= 0)
                return "";
            var accessMapped = root.accessibilityRowSubPanes[
                        String(row.id).substring(0, accessDot)];
            return accessMapped !== undefined ? accessMapped : "";
        }
        if (row.id === "reset.startupDisk")
            return "startup";
        if (row.id === "reset.reset")
            return "reset";
        var dot = String(row.id).indexOf(".");
        if (dot <= 0)
            return "";
        var mapped = root.generalRowSubPanes[String(row.id).substring(0, dot)];
        return mapped !== undefined ? mapped : "";
    }

    // T290f: the Appearance rows that live inside the Advanced Appearance
    // sheet. The sheet is an organization change, so the ids (not localized
    // section titles) are the stable table.
    readonly property var appearanceDialogRows: ({
        "iconTheme": true,
        "cursorTheme": true,
        "cursorSize": true,
        "windowControls": true,
        "material": true,
        "accentFromWallpaper": true
    })

    // T292a: the Desktop & Dock rows that live in the Advanced Dock and
    // Advanced Windows sheets. Their section titles are the stable key.
    readonly property var desktopDockDialogRows: ({
        "chooserOnHover": true,
        "minimizeReaction": true,
        "windowRules": true
    })

    // Where a row lives: "subpane", "dialog", or "pane". "" means the row
    // cannot be landed and must not be indexed or deep-linked.
    function rowSurface(paneId, rowId) {
        var row = root.rowFor(paneId, rowId);
        if (row === null)
            return "";
        if (root.subPaneForRow(paneId, rowId).length > 0)
            return "subpane";
        if (paneId === "appearance"
                && root.appearanceDialogRows[rowId] === true)
            return "dialog";
        if (paneId === "desktop-dock"
                && root.desktopDockDialogRows[rowId] === true)
            return "dialog";
        return "pane";
    }

    // A result is only indexed/accepted when its row is shipped and the shell
    // can open the surface that renders it.
    function isRowLandable(paneId, rowId) {
        var row = root.rowFor(paneId, rowId);
        return row !== null && row.state === "shipped"
                && root.rowSurface(paneId, rowId).length > 0;
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
