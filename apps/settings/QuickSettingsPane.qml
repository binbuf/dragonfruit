// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Quick Settings pane (T227).
//
// The pane is a customization surface, not a second Control Center: every
// change writes `quickSettings.modules` / `quickSettings.menuBarOnly` in
// settingsd, and the live shell panel reads the same keys. The panel's module
// order is the list order; menu-bar-only items stay in the menu bar and are
// removed from the panel.
Item {
    id: root

    // The Control Center module ids, in the panel's built-in order. The ids
    // match `ControlCenter.qml`'s tile ids exactly.
    readonly property var moduleCatalog: [
        { id: "wifi", label: qsTr("Wi-Fi") },
        { id: "bluetooth", label: qsTr("Bluetooth") },
        { id: "storage", label: qsTr("Storage") },
        { id: "focus", label: qsTr("Focus") },
        { id: "volume", label: qsTr("Sound") },
        { id: "brightness", label: qsTr("Display") },
        { id: "dark", label: qsTr("Dark Mode") },
        { id: "keyboard", label: qsTr("Keyboard") },
        { id: "mission-control", label: qsTr("Overview") },
        { id: "battery", label: qsTr("Battery") },
        { id: "lock-screen", label: qsTr("Lock Screen") },
        { id: "menu-bar", label: qsTr("Menu Bar") },
        { id: "software-update", label: qsTr("Software Update") },
        { id: "users", label: qsTr("Users") },
        { id: "printers", label: qsTr("Printers") },
        { id: "privacy", label: qsTr("Privacy") },
        { id: "accessibility", label: qsTr("Accessibility") },
        { id: "vpn", label: qsTr("VPN") },
        // T235: the measured catalogue's addable modules.
        { id: "now-playing", label: qsTr("Now Playing") },
        { id: "airdrop", label: qsTr("Nearby Transfer") },
        { id: "stage-manager", label: qsTr("Stage") },
        { id: "screen-mirroring", label: qsTr("Screen Casting") },
        { id: "capture-screen", label: qsTr("Capture Screen") },
        { id: "airplay-audio", label: qsTr("Audio Casting") },
        { id: "edit-controls", label: qsTr("Edit Controls") },
        { id: "keyboard-brightness", label: qsTr("Keyboard Brightness") },
        { id: "fast-user-switching", label: qsTr("Fast User Switching") },
        { id: "clock", label: qsTr("Clock") },
        { id: "screenshot", label: qsTr("Screenshot") },
        { id: "shortcuts", label: qsTr("Shortcuts") },
        { id: "utilities", label: qsTr("Utilities") },
        { id: "voice-memos", label: qsTr("Voice Memos") },
        { id: "home-control", label: qsTr("Home") },
        { id: "notes", label: qsTr("Notes") },
        { id: "hearing-accessibility", label: qsTr("Hearing Accessibility") },
        { id: "motor-accessibility", label: qsTr("Motor Accessibility") },
        { id: "vision-accessibility", label: qsTr("Vision Accessibility") }
    ]

    // The items that can be kept in the menu bar only. Each maps to the
    // existing `menu.control.*` visibility key the Menu Bar pane owns.
    readonly property var menuBarCatalog: [
        { id: "wifi", label: qsTr("Wi-Fi"), key: "menu.control.wifi" },
        { id: "bluetooth", label: qsTr("Bluetooth"), key: "menu.control.bluetooth" },
        { id: "focus", label: qsTr("Focus"), key: "menu.control.focus" },
        { id: "volume", label: qsTr("Sound"), key: "menu.control.volume" },
        { id: "battery", label: qsTr("Battery"), key: "menu.control.battery" },
        { id: "accessibility", label: qsTr("Accessibility"),
          key: "menu.control.accessibility" }
    ]

    readonly property var defaultModules: [
        "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
        "screen-mirroring", "dark", "capture-screen", "focus", "brightness",
        "volume", "airplay-audio", "edit-controls"
    ]

    readonly property var modules: {
        var value = Settings.values["quickSettings.modules"];
        var list = value === undefined || value === null ? [] : Array.from(value);
        return list.length > 0 ? list : root.defaultModules;
    }
    readonly property var menuBarOnly: {
        var value = Settings.values["quickSettings.menuBarOnly"];
        return value === undefined || value === null ? [] : Array.from(value);
    }

    function moduleLabel(id) {
        for (var i = 0; i < root.moduleCatalog.length; ++i) {
            if (root.moduleCatalog[i].id === id)
                return root.moduleCatalog[i].label;
        }
        return id;
    }

    function moduleEnabled(id) {
        return root.modules.indexOf(id) >= 0;
    }

    function moduleIndex(id) {
        return root.modules.indexOf(id);
    }

    function writeModules(list) {
        Settings.setList("quickSettings.modules", list);
    }

    function setModuleEnabled(id, enabled) {
        var list = root.modules.slice();
        var at = list.indexOf(id);
        if (enabled && at < 0)
            list.push(id);
        else if (!enabled && at >= 0)
            list.splice(at, 1);
        else
            return;
        root.writeModules(list);
    }

    function moveModule(id, delta) {
        var list = root.modules.slice();
        var at = list.indexOf(id);
        var to = at + delta;
        if (at < 0 || to < 0 || to >= list.length)
            return;
        list.splice(at, 1);
        list.splice(to, 0, id);
        root.writeModules(list);
    }

    function isMenuBarOnly(id) {
        return root.menuBarOnly.indexOf(id) >= 0;
    }

    function menuBarKey(id) {
        for (var i = 0; i < root.menuBarCatalog.length; ++i) {
            if (root.menuBarCatalog[i].id === id)
                return root.menuBarCatalog[i].key;
        }
        return "";
    }

    function setMenuBarOnly(id, enabled) {
        var only = root.menuBarOnly.slice();
        var at = only.indexOf(id);
        if (enabled && at < 0)
            only.push(id);
        else if (!enabled && at >= 0)
            only.splice(at, 1);
        else
            return;
        Settings.setList("quickSettings.menuBarOnly", only);
        // A menu-bar-only module leaves the panel; unchecking it returns to
        // the panel. The menu-bar mark itself stays live through the existing
        // `menu.control.*` key.
        var list = root.modules.slice();
        var moduleAt = list.indexOf(id);
        if (enabled && moduleAt >= 0)
            list.splice(moduleAt, 1);
        else if (!enabled && moduleAt < 0)
            list.push(id);
        root.writeModules(list);
        var key = root.menuBarKey(id);
        if (key.length > 0 && Settings.values[key] !== true)
            Settings.set(key, true);
    }

    function resetModules() {
        Settings.setList("quickSettings.modules", root.defaultModules.slice());
        Settings.setList("quickSettings.menuBarOnly", []);
        Settings.setList("quickSettings.page2", []);
        Settings.setList("quickSettings.page3", []);
        Settings.set("quickSettings.pageCount", 1);
        Settings.set("quickSettings.activePage", 0);
        Settings.setList("quickSettings.spans", []);
        Settings.set("quickSettings.perDisplay", false);
    }

    // Test surface (used by tst_settings_quick_settings.qml).
    property alias moduleRepeater: moduleRepeater
    property alias menuBarRepeater: menuBarRepeater
    property alias resetButton: resetButton
    property alias perDisplayRow: perDisplayRow

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            title: qsTr("Modules")

            Repeater {
                id: moduleRepeater
                model: root.moduleCatalog

                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: content.width
                    label: modelData.label
                    description: root.isMenuBarOnly(modelData.id)
                                 ? qsTr("Menu bar only") : ""
                    settingsId: modelData.id
                    owner: "settingsd -> shell/control-center"
                    settingsKey: "quickSettings.modules"

                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Button {
                            text: qsTr("Up")
                            accessibleName: qsTr("Move %1 up").arg(modelData.label)
                            enabled: root.moduleIndex(modelData.id) > 0
                            onClicked: root.moveModule(modelData.id, -1)
                        }
                        Button {
                            text: qsTr("Down")
                            accessibleName: qsTr("Move %1 down").arg(modelData.label)
                            enabled: root.moduleIndex(modelData.id) >= 0
                                     && root.moduleIndex(modelData.id)
                                        < root.modules.length - 1
                            onClicked: root.moveModule(modelData.id, 1)
                        }
                        Toggle {
                            text: ""
                            accessibleName: qsTr("Show %1 in Quick Settings")
                                            .arg(modelData.label)
                            checked: root.moduleEnabled(modelData.id)
                            onToggled: (checked) => root.setModuleEnabled(modelData.id,
                                                                          checked)
                        }
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Menu Bar Only")

            Text {
                width: parent.width
                text: qsTr("Keep these items in the menu bar and remove them from "
                          + "the Quick Settings panel.")
                color: Theme.color.textSecondary
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: menuBarRepeater
                model: root.menuBarCatalog

                delegate: SettingsRow {
                    required property var modelData
                    width: content.width
                    label: modelData.label
                    settingsId: "menu-bar-" + modelData.id
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: modelData.key

                    controlData: Toggle {
                        text: ""
                        accessibleName: qsTr("Keep %1 in the menu bar only")
                                        .arg(modelData.label)
                        checked: root.isMenuBarOnly(modelData.id)
                        onToggled: (checked) => root.setMenuBarOnly(modelData.id,
                                                                    checked)
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Displays")

            SettingsRow {
                id: perDisplayRow
                width: parent.width
                label: qsTr("Show on all displays")
                description: qsTr("Off shows Quick Settings on the active display "
                                  + "only. The per-display surfaces themselves "
                                  + "remain OPEN.")
                settingsId: "perDisplay"
                owner: "settingsd -> shell/control-center"
                settingsKey: "quickSettings.perDisplay"
                showSeparator: false

                controlData: Toggle {
                    text: ""
                    accessibleName: qsTr("Show Quick Settings on all displays")
                    checked: Settings.values["quickSettings.perDisplay"] === true
                    onToggled: (checked) =>
                        Settings.set("quickSettings.perDisplay", checked)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Reset")

            SettingsRow {
                width: parent.width
                label: qsTr("Restore the default modules")
                showSeparator: false

                controlData: Button {
                    id: resetButton
                    text: qsTr("Reset")
                    accessibleName: qsTr("Restore the default Quick Settings modules")
                    onClicked: root.resetModules()
                }
            }
        }
    }
}
