// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Menu Bar pane (T-15.9b, completed by T277).
//
// The pane and the Quick Settings Menu Bar tile are one functional unit. The
// menu bar is shell-native: the shell's `MenuBar` owns the chrome, the status
// row, and the clock, the menu-broker resolves the focused app's menus, and
// settingsd owns the durable preferences. This pane is the configuration half
// and writes only settingsd keys, so every control applies live and persists:
//
//   * `menu.autoHide`, `menu.showBackground`, and `menu.recentItems` are the
//     behavior rows;
//   * `menu.clock.showDate` / `menu.clock.showSeconds` are the Clock options;
//   * `menu.control.<id>` gates each status item. The shell applies it in
//     `applyStatusItems` (additive with the adapter's own availability), so a
//     control shows only when its daemon is present *and* the user wants it;
//   * `menu.control.focusAlways`, `menu.control.volumeWhenActive`, and
//     `power.showPercentage` are the per-control options;
//   * `menu.tray.hidden` is the "Allow in the Menu Bar" set for third-party
//     StatusNotifier items;
//   * `menu.overflow` and `menu.control.workspaces` are the Linux-only rows.
//
// A control with no menu-bar provider is shown disabled with the exact reason
// and is never advertised as live. The Display and Screen Casting controls
// live in Quick Settings, not in the menu bar.
Item {
    id: root

    // ── Behavior keys ───────────────────────────────────────────────────
    readonly property string autoHide:
        Settings.values["menu.autoHide"] !== undefined
            ? String(Settings.values["menu.autoHide"]) : "full-screen"
    readonly property bool showBackground:
        Settings.values["menu.showBackground"] !== false
    readonly property int recentItems:
        Settings.values["menu.recentItems"] !== undefined
            ? Number(Settings.values["menu.recentItems"]) : 10

    // ── Clock option keys ───────────────────────────────────────────────
    readonly property bool clockShowDate:
        Settings.values["menu.clock.showDate"] !== false
    readonly property bool clockShowSeconds:
        Settings.values["menu.clock.showSeconds"] === true

    // ── OS menu glyph ───────────────────────────────────────────────────
    readonly property string osGlyph:
        Settings.values["menu.osGlyph"] !== undefined
            ? String(Settings.values["menu.osGlyph"]) : "dragonfruit"

    readonly property var glyphOptions: [
        { value: "dragonfruit", label: qsTr("Dragonfruit"),
          description: qsTr("The Dragonfruit mark.") },
        { value: "tux", label: qsTr("Tux"),
          description: qsTr("The Linux penguin mark.") }
    ]

    readonly property var autoHideOptions: [
        { value: "never", label: qsTr("Never") },
        { value: "always", label: qsTr("Always") },
        { value: "full-screen", label: qsTr("In Full Screen Only") }
    ]

    readonly property var recentOptions: [
        { value: 0, label: qsTr("None") },
        { value: 5, label: qsTr("5") },
        { value: 10, label: qsTr("10") },
        { value: 15, label: qsTr("15") },
        { value: 20, label: qsTr("20") },
        { value: 30, label: qsTr("30") },
        { value: 50, label: qsTr("50") }
    ]

    readonly property var overflowOptions: [
        { value: "automatic", label: qsTr("Automatic") },
        { value: "always", label: qsTr("Always show the overflow menu") }
    ]

    // The Menu Bar Controls list. `options` opens the per-control options;
    // `disabled` is an honest absence (no menu-bar item exists), never a fake
    // toggle. Clock is the one separate row with its own options.
    readonly property var controlRows: [
        { key: "menu.control.wifi", label: qsTr("Wi-Fi") },
        { key: "menu.control.bluetooth", label: qsTr("Bluetooth") },
        { key: "menu.control.battery", label: qsTr("Battery"),
          options: "battery" },
        { key: "menu.control.volume", label: qsTr("Sound"),
          options: "sound" },
        { key: "menu.control.search", label: qsTr("Search") },
        { key: "menu.control.notifications", label: qsTr("Notifications") },
        { key: "menu.control.focus", label: qsTr("Focus"),
          options: "focus" },
        { key: "menu.control.display", label: qsTr("Display"),
          disabled: true,
          reason: qsTr("Display has no menu-bar item; the Display module "
                      + "lives in Quick Settings.") },
        { key: "menu.control.screenMirroring", label: qsTr("Screen Casting"),
          disabled: true,
          reason: qsTr("Screen casting has no menu-bar item; start casting "
                      + "from Quick Settings.") },
        { key: "menu.control.accessibility", label: qsTr("Accessibility") },
        { key: "menu.control.controlCenter", label: qsTr("Quick Settings") },
        { key: "menu.control.workspaces", label: qsTr("Workspaces") }
    ]

    readonly property var thirdPartyItems: Settings.thirdPartyMenuBarItems
    readonly property bool thirdPartyAvailable: Settings.thirdPartyMenuBarAvailable

    function controlVisible(key) {
        return Settings.values[key] !== false;
    }

    function controlRow(index) {
        return root.controlRows[index];
    }

    function controlDisabled(row) {
        return row !== undefined && row.disabled === true;
    }

    function controlReason(row) {
        return row !== undefined && row.reason !== undefined
            ? String(row.reason) : "";
    }

    // The executable map's row id for a control. Sound is the one renamed row.
    function controlSettingsId(key) {
        if (key === "menu.control.volume")
            return "sound";
        return String(key).replace("menu.control.", "");
    }

    // ── Per-control options ─────────────────────────────────────────────
    function optionKey(kind) {
        switch (kind) {
        case "battery": return "power.showPercentage";
        case "focus": return "menu.control.focusAlways";
        case "sound": return "menu.control.volumeWhenActive";
        default: return "";
        }
    }

    function optionTitle(kind) {
        switch (kind) {
        case "battery": return qsTr("Battery Options");
        case "focus": return qsTr("Focus Options");
        case "sound": return qsTr("Sound Options");
        default: return qsTr("Options");
        }
    }

    function optionLabel(kind) {
        switch (kind) {
        case "battery": return qsTr("Show percentage");
        case "focus": return qsTr("Show when Focus is off");
        case "sound": return qsTr("Hide while muted");
        default: return qsTr("Option");
        }
    }

    function optionDescription(kind) {
        switch (kind) {
        case "battery":
            return qsTr("Draw the battery percentage beside the glyph.");
        case "focus":
            return qsTr("Show the Focus mark even when no Focus mode is on.");
        case "sound":
            return qsTr("Hide the Sound mark while the output is muted.");
        default:
            return "";
        }
    }

    function optionChecked(kind) {
        var key = root.optionKey(kind);
        return key.length > 0 && Settings.values[key] === true;
    }

    function setOption(kind, checked) {
        var key = root.optionKey(kind);
        if (key.length > 0)
            Settings.set(key, checked);
    }

    // ── Third-party StatusNotifier items ────────────────────────────────
    readonly property var hiddenTray: {
        var value = Settings.values["menu.tray.hidden"];
        return value === undefined || value === null ? [] : Array.from(value);
    }

    function trayAllowed(name) {
        return root.hiddenTray.indexOf(name) < 0;
    }

    function setTrayAllowed(name, allowed) {
        var hidden = root.hiddenTray.slice();
        var at = hidden.indexOf(name);
        if (allowed && at >= 0)
            hidden.splice(at, 1);
        else if (!allowed && at < 0)
            hidden.push(name);
        else
            return;
        Settings.setList("menu.tray.hidden", hidden);
    }

    // ── Linux-only rows ─────────────────────────────────────────────────
    readonly property string overflowPolicy:
        Settings.values["menu.overflow"] !== undefined
            ? String(Settings.values["menu.overflow"]) : "automatic"
    readonly property bool workspacesVisible:
        Settings.values["menu.control.workspaces"] !== false
    readonly property bool globalMenuEnabled:
        Settings.values["menu.global"] !== false
    readonly property bool perDisplayPanel:
        Settings.values["quickSettings.perDisplay"] === true

    function overflowIndex() {
        for (var i = 0; i < root.overflowOptions.length; ++i) {
            if (root.overflowOptions[i].value === root.overflowPolicy)
                return i;
        }
        return 0;
    }

    // ── Reset ───────────────────────────────────────────────────────────
    function resetMenuBar() {
        Settings.set("menu.autoHide", "full-screen");
        Settings.set("menu.showBackground", true);
        Settings.set("menu.recentItems", 10);
        Settings.set("menu.osGlyph", "dragonfruit");
        Settings.set("menu.clock.showDate", true);
        Settings.set("menu.clock.showSeconds", false);
        Settings.set("menu.control.clock", true);
        Settings.set("menu.control.focusAlways", false);
        Settings.set("menu.control.volumeWhenActive", false);
        Settings.set("power.showPercentage", false);
        Settings.set("menu.overflow", "automatic");
        Settings.setList("menu.tray.hidden", []);
        Settings.set("menu.global", true);
        Settings.set("quickSettings.perDisplay", false);
        for (var i = 0; i < root.controlRows.length; ++i) {
            var row = root.controlRows[i];
            if (row.disabled === true)
                continue;
            Settings.set(row.key, true);
        }
    }

    // ── Test surface ────────────────────────────────────────────────────
    property alias autoHideSelect: autoHideSelect
    property alias backgroundToggle: backgroundToggle
    property alias recentSelect: recentSelect
    property alias clockOptionsButton: clockOptionsButton
    property alias clockDialog: clockDialog
    property alias clockDateToggle: clockDateToggle
    property alias clockSecondsToggle: clockSecondsToggle
    property alias clockVisibilityToggle: clockVisibilityToggle
    property alias glyphRepeater: glyphRepeater
    property alias controlsRepeater: controlsRepeater
    property alias absenceNote: absenceNote
    property alias thirdPartyRepeater: thirdPartyRepeater
    property alias thirdPartyGroup: thirdPartyGroup
    property alias addControlsButton: addControlsButton
    property alias addControlsDialog: addControlsDialog
    property alias controlOptionsDialog: controlOptionsDialog
    property alias controlOptionsToggle: controlOptionsToggle
    property alias overflowSelect: overflowSelect
    property alias workspacesToggle: workspacesToggle
    property alias globalMenuToggle: globalMenuToggle
    property alias perDisplayPanelRow: perDisplayPanelRow
    property alias resetButton: resetButton
    property alias clockRow: clockRow

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function indexFor(options, value) {
        for (var i = 0; i < options.length; ++i) {
            if (Number(options[i].value) === Number(value))
                return i;
        }
        return options.length - 1;
    }

    function autoHideIndex(value) {
        for (var i = 0; i < root.autoHideOptions.length; ++i) {
            if (root.autoHideOptions[i].value === value)
                return i;
        }
        return root.autoHideOptions.length - 1;
    }

    Component.onCompleted: Settings.refreshThirdPartyMenuBarItems()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ─────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !Settings.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "menuBarAbsenceNote"
                kind: "live-on-defaults"
                showWhenLive: true
                reason: qsTr("The settings daemon is not running, so these "
                             + "preferences apply from the built-in defaults. "
                             + "The menu bar itself is drawn by the shell.")
            }
        }

        // ── Behavior ────────────────────────────────────────────────────
        SettingsGroup {
            id: behaviorGroup
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Automatically hide and show the menu bar")
                settingsId: "autoHide"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.autoHide"
                controlData: Select {
                    id: autoHideSelect
                    accessibleName: qsTr("Automatically hide and show the menu bar")
                    model: root.autoHideOptions
                    onActivated: (index) => Settings.set(
                        "menu.autoHide", root.autoHideOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show menu bar background")
                settingsId: "background"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.showBackground"
                controlData: Toggle {
                    id: backgroundToggle
                    accessibleName: qsTr("Show menu bar background")
                    onToggled: (checked) => Settings.set(
                        "menu.showBackground", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Recent documents, applications, and servers")
                settingsId: "recentItems"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.recentItems"
                showSeparator: false
                controlData: Select {
                    id: recentSelect
                    accessibleName: qsTr("Recent documents, applications, and servers")
                    model: root.recentOptions
                    onActivated: (index) => Settings.set(
                        "menu.recentItems", root.recentOptions[index].value)
                }
            }
        }

        // ── OS Menu Glyph ───────────────────────────────────────────────
        SettingsGroup {
            id: glyphGroup
            width: parent.width
            title: qsTr("OS Menu Glyph")

            Text {
                objectName: "menuBarGlyphDescription"
                property string settingsId: "osGlyph"
                width: parent.width
                text: qsTr("Choose the mark at the far left of the menu bar. "
                           + "Each preview is shown at the menu bar's real "
                           + "size with the same shadow and hover treatment.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: glyphRepeater
                model: root.glyphOptions

                delegate: GlyphChoiceRow {
                    required property int index

                    width: parent.width
                    showSeparator: index < glyphRepeater.count - 1
                }
            }
        }

        // ── Menu Bar Controls ───────────────────────────────────────────
        SettingsGroup {
            id: controlsGroup
            width: parent.width
            title: qsTr("Menu Bar Controls")

            Text {
                objectName: "menuBarControlsDescription"
                width: parent.width
                text: qsTr("System and app controls can be configured to "
                           + "appear in both Quick Settings and the menu bar.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            // The one Clock row, with its options. It is not duplicated in
            // the control list below.
            SettingsRow {
                id: clockRow
                width: parent.width
                label: qsTr("Clock")
                description: qsTr("Show the date and time in the menu bar.")
                settingsId: "clock"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.control.clock"
                controlData: Row {
                    spacing: Theme.primitive.spacing.xs

                    Toggle {
                        id: clockVisibilityToggle
                        text: ""
                        accessibleName: qsTr("Show the clock in the menu bar")
                        checked: root.controlVisible("menu.control.clock")
                        onToggled: (checked) =>
                            Settings.set("menu.control.clock", checked)
                    }

                    Button {
                        id: clockOptionsButton
                        objectName: "menuBarClockOptionsButton"
                        text: qsTr("Options\u2026")
                        accessibleName: qsTr("Clock options")
                        onClicked: clockDialog.show()
                    }
                }
            }

            Repeater {
                id: controlsRepeater
                model: root.controlRows

                delegate: SettingsRow {
                    required property var modelData
                    required property int index

                    property alias toggle: controlToggle
                    width: parent.width
                    label: modelData.label
                    description: root.controlDisabled(modelData)
                        ? root.controlReason(modelData) : ""
                    settingsId: root.controlSettingsId(modelData.key)
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: modelData.key
                    enabled: !root.controlDisabled(modelData)
                    showSeparator: index < controlsRepeater.count - 1

                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Toggle {
                            id: controlToggle
                            objectName: "menuBarControlToggle"
                            text: ""
                            enabled: !root.controlDisabled(modelData)
                            accessibleName: modelData.label
                            onToggled: (checked) =>
                                Settings.set(modelData.key, checked)
                        }

                        Button {
                            objectName: "menuBarControlOptions-" + index
                            visible: modelData.options !== undefined
                            enabled: !root.controlDisabled(modelData)
                            text: qsTr("Options\u2026")
                            accessibleName: qsTr("%1 options").arg(modelData.label)
                            onClicked: {
                                root.controlOptionsDialog.optionKind =
                                    String(modelData.options);
                                root.controlOptionsDialog.show();
                            }
                        }
                    }

                    Binding {
                        target: controlToggle
                        property: "checked"
                        value: root.controlVisible(modelData.key)
                    }
                }
            }

            Text {
                objectName: "menuBarOverflowDescription"
                width: parent.width
                text: qsTr("Items that do not fit move into the overflow "
                           + "menu; they are never clipped. Only the active "
                           + "display owns the application menu.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Add controls\u2026")
                description: qsTr("Choose which system controls appear in the "
                                  + "menu bar.")
                settingsId: "addControls"
                owner: "settingsd + shell/MenuBar"
                settingsKey: ""
                showSeparator: false

                controlData: Button {
                    id: addControlsButton
                    objectName: "menuBarAddControls"
                    text: qsTr("Add Controls\u2026")
                    onClicked: addControlsDialog.show()
                }
            }
        }

        // ── Third-party StatusNotifier items ────────────────────────────
        SettingsGroup {
            id: thirdPartyGroup
            width: parent.width
            title: qsTr("Allow in the Menu Bar")

            SettingsRow {
                width: parent.width
                label: qsTr("Third-party items")
                description: root.thirdPartyAvailable
                    ? qsTr("Third-party applications can add their own "
                           + "StatusNotifier items. Turn one off to keep it "
                           + "out of the menu bar; its application keeps "
                           + "running.")
                    : qsTr("The StatusNotifier host is not running, so no "
                           + "third-party items are available.")
                settingsId: "thirdParty"
                owner: "StatusNotifier registry"
                settingsKey: ""
                showSeparator: root.thirdPartyItems.length > 0
            }

            Repeater {
                id: thirdPartyRepeater
                model: root.thirdPartyItems

                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.label !== undefined
                        ? String(modelData.label) : String(modelData.name)
                    description: String(modelData.name)
                    settingsId: "thirdParty-" + index
                    owner: "StatusNotifier registry"
                    settingsKey: "menu.tray.hidden"
                    showSeparator: index < thirdPartyRepeater.count - 1

                    controlData: Toggle {
                        objectName: "menuBarThirdPartyToggle-" + index
                        text: ""
                        accessibleName: qsTr("Allow %1 in the menu bar")
                            .arg(String(modelData.label))
                        checked: root.trayAllowed(String(modelData.name))
                        onToggled: (checked) =>
                            root.setTrayAllowed(String(modelData.name), checked)
                    }
                }
            }

            Text {
                visible: root.thirdPartyAvailable
                         && root.thirdPartyItems.length === 0
                width: parent.width
                text: qsTr("No third-party menu-bar items are installed.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }
        }

        // ── Linux-only rows ─────────────────────────────────────────────
        SettingsGroup {
            id: linuxGroup
            width: parent.width
            title: qsTr("Menu Bar Behavior")

            SettingsRow {
                width: parent.width
                label: qsTr("Overflow policy")
                description: qsTr("Automatic moves items into the overflow "
                                  + "menu only when the measured bar runs out "
                                  + "of room. Always keeps the overflow menu "
                                  + "visible and moves third-party items into "
                                  + "it first.")
                settingsId: "overflow"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.overflow"
                controlData: Select {
                    id: overflowSelect
                    objectName: "menuBarOverflowPolicy"
                    accessibleName: qsTr("Overflow policy")
                    model: root.overflowOptions
                    onActivated: (index) => Settings.set(
                        "menu.overflow", root.overflowOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Workspace indicator")
                description: qsTr("Show the active workspace number in the "
                                  + "menu bar. It is live from the compositor "
                                  + "and hidden when only one workspace exists.")
                settingsId: "workspaces"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.control.workspaces"
                controlData: Toggle {
                    id: workspacesToggle
                    objectName: "menuBarWorkspaces"
                    text: ""
                    accessibleName: qsTr("Show the workspace indicator")
                    checked: root.workspacesVisible
                    onToggled: (checked) =>
                        Settings.set("menu.control.workspaces", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show application menus in the menu bar")
                description: qsTr("When off, first-party applications keep "
                                  + "their own local menus.")
                settingsId: "globalMenu"
                owner: "settingsd -> shell/MenuBar"
                settingsKey: "menu.global"
                controlData: Toggle {
                    id: globalMenuToggle
                    objectName: "menuBarGlobalMenu"
                    text: ""
                    accessibleName: qsTr("Show application menus in the menu bar")
                    checked: root.globalMenuEnabled
                    onToggled: (checked) => Settings.set("menu.global", checked)
                }
            }

            SettingsRow {
                id: perDisplayPanelRow
                width: parent.width
                label: qsTr("Show on every display")
                description: qsTr("Disabled: the shell ships one Quick "
                                  + "Settings and menu-bar surface. Per-display "
                                  + "panels need a compositor overlay per "
                                  + "output.")
                settingsId: "perDisplayPanel"
                owner: "settingsd -> shell"
                settingsKey: "quickSettings.perDisplay"
                enabled: false
                showSeparator: false
                controlData: Toggle {
                    text: ""
                    checked: root.perDisplayPanel
                    enabled: false
                    accessibleName: qsTr("Show on every display")
                }
            }
        }

        // ── Reset ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Reset")

            SettingsRow {
                width: parent.width
                label: qsTr("Reset Quick Settings")
                description: qsTr("Restores the menu-bar behavior, clock "
                                  + "options, control visibility, per-control "
                                  + "options, third-party allowances, and the "
                                  + "Linux-only rows.")
                settingsId: "reset"
                owner: "settingsd"
                settingsKey: ""
                showSeparator: false

                controlData: Button {
                    id: resetButton
                    objectName: "menuBarReset"
                    text: qsTr("Reset")
                    accessibleName: qsTr("Reset the menu bar")
                    onClicked: root.resetMenuBar()
                }
            }
        }
    }

    // The per-control options sheet. One dialog carries the one option the
    // selected control owns; the key is the durable preference.
    Dialog {
        id: controlOptionsDialog
        objectName: "menuBarControlOptionsDialog"
        property string optionKind: "battery"
        title: root.optionTitle(optionKind)
        message: qsTr("Choose how this control appears in the menu bar.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            SettingsRow {
                width: parent.width
                label: root.optionLabel(controlOptionsDialog.optionKind)
                description: root.optionDescription(
                                 controlOptionsDialog.optionKind)
                showSeparator: false

                controlData: Toggle {
                    id: controlOptionsToggle
                    objectName: "menuBarControlOptionToggle"
                    text: ""
                    accessibleName: root.optionLabel(
                                        controlOptionsDialog.optionKind)
                    onToggled: (checked) =>
                        root.setOption(controlOptionsDialog.optionKind, checked)
                }

                Binding {
                    target: controlOptionsToggle
                    property: "checked"
                    value: root.optionChecked(controlOptionsDialog.optionKind)
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: controlOptionsDialog.accept()
            }
        }
    }

    // The Clock Options sheet: the two clock options the shell's clock reads.
    Dialog {
        id: clockDialog
        objectName: "menuBarClockDialog"
        title: qsTr("Clock Options")
        message: qsTr("Choose what the menu-bar clock shows.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            SettingsRow {
                width: parent.width
                label: qsTr("Show date")
                controlData: Toggle {
                    id: clockDateToggle
                    accessibleName: qsTr("Show date")
                    onToggled: (checked) => Settings.set(
                        "menu.clock.showDate", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Display the time with seconds")
                showSeparator: false
                controlData: Toggle {
                    id: clockSecondsToggle
                    accessibleName: qsTr("Display the time with seconds")
                    onToggled: (checked) => Settings.set(
                        "menu.clock.showSeconds", checked)
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: clockDialog.accept()
            }
        }
    }

    // The Add Controls sheet: the same visibility keys as the list, without a
    // second control catalogue.
    Dialog {
        id: addControlsDialog
        objectName: "menuBarAddControlsDialog"
        title: qsTr("Add Menu Bar Controls")
        message: qsTr("Choose which controls appear in the menu bar.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Repeater {
                model: root.controlRows

                delegate: SettingsRow {
                    required property var modelData
                    width: parent.width
                    label: modelData.label
                    description: root.controlDisabled(modelData)
                        ? root.controlReason(modelData) : ""
                    settingsId: "add-" + modelData.key.replace("menu.control.", "")
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: modelData.key
                    enabled: !root.controlDisabled(modelData)
                    showSeparator: false

                    controlData: Toggle {
                        objectName: "menuBarAddToggle-" + modelData.key
                        text: ""
                        enabled: !root.controlDisabled(modelData)
                        accessibleName: qsTr("Show %1 in the menu bar")
                            .arg(modelData.label)
                        checked: root.controlVisible(modelData.key)
                        onToggled: (checked) =>
                            Settings.set(modelData.key, checked)
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: addControlsDialog.accept()
            }
        }
    }

    // Keep the controls in step with external changes.
    Binding {
        target: autoHideSelect
        property: "currentIndex"
        value: root.autoHideIndex(root.autoHide)
    }
    Binding {
        target: backgroundToggle
        property: "checked"
        value: root.showBackground
    }
    Binding {
        target: recentSelect
        property: "currentIndex"
        value: root.indexFor(root.recentOptions, root.recentItems)
    }
    Binding {
        target: clockDateToggle
        property: "checked"
        value: root.clockShowDate
    }
    Binding {
        target: clockSecondsToggle
        property: "checked"
        value: root.clockShowSeconds
    }
    Binding {
        target: overflowSelect
        property: "currentIndex"
        value: root.overflowIndex()
    }

    // One OS-menu glyph choice: a radio row with a real-size menu-bar preview.
    // The preview uses the same `SystemMenuGlyph` and wallpaper-contrast shadow
    // as the bar, so the row is a faithful miniature rather than a generic
    // icon tile.
    component GlyphChoiceRow: Item {
        id: choice

        required property var modelData
        property bool showSeparator: true

        readonly property bool selected:
            root.osGlyph === choice.modelData.value
        readonly property bool lightLettering: {
            var c = Theme.color.textPrimary;
            return (0.299 * c.r + 0.587 * c.g + 0.114 * c.b) > 0.5;
        }
        readonly property color labelShadowColor:
            choice.lightLettering ? Theme.controls.wallpaperText.shadowColorOnDark
                                  : Theme.controls.wallpaperText.shadowColorOnLight
        readonly property real labelShadowOpacity:
            choice.lightLettering ? Theme.controls.wallpaperText.shadowOpacityOnDark
                                  : Theme.controls.wallpaperText.shadowOpacityOnLight
        readonly property color glyphShadowColor:
            Qt.rgba(choice.labelShadowColor.r, choice.labelShadowColor.g,
                    choice.labelShadowColor.b, choice.labelShadowOpacity * 0.9)

        function choose() {
            Settings.set("menu.osGlyph", choice.modelData.value);
        }

        width: parent ? parent.width : implicitWidth
        implicitHeight: Theme.controls.settingsRow.height
        activeFocusOnTab: true

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.settingsGroup.radius
            color: Qt.rgba(Theme.color.textPrimary.r, Theme.color.textPrimary.g,
                           Theme.color.textPrimary.b, 0.05)
            visible: choiceHover.hovered
            antialiasing: true
        }

        Rectangle {
            id: radioOuter
            x: Theme.controls.settingsRow.paddingH
            anchors.verticalCenter: parent.verticalCenter
            width: 16
            height: 16
            radius: width / 2
            color: "transparent"
            border.width: Theme.controls.window.borderWidth
            border.color: choice.selected ? Theme.color.accent : Theme.color.textTertiary
            antialiasing: true

            Rectangle {
                anchors.centerIn: parent
                width: 8
                height: 8
                radius: width / 2
                color: Theme.color.accent
                visible: choice.selected
                antialiasing: true
            }
        }

        Column {
            id: choiceLabelColumn
            anchors.left: radioOuter.right
            anchors.leftMargin: Theme.primitive.spacing.md
            anchors.right: previewPlate.left
            anchors.rightMargin: Theme.primitive.spacing.md
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.primitive.spacing.xxs

            Text {
                width: parent.width
                text: choice.modelData.label
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                text: choice.modelData.description
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                elide: Text.ElideRight
            }
        }

        Rectangle {
            id: previewPlate
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.settingsRow.paddingH
            anchors.verticalCenter: parent.verticalCenter
            width: 112
            height: Theme.controls.menuBar.height
            radius: Theme.controls.menuBar.radius
            color: Theme.color.chrome
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true

            Rectangle {
                anchors.centerIn: parent
                width: Theme.controls.menuBar.iconSize
                        + 2 * Theme.controls.menuBarMenu.barPaddingH
                height: Theme.controls.menuBarMenu.barHeight
                radius: Theme.controls.focusRing.radius
                color: Qt.rgba(Theme.color.textPrimary.r,
                               Theme.color.textPrimary.g,
                               Theme.color.textPrimary.b,
                               Theme.controls.menuBar.hoverOpacity)
                visible: choiceHover.hovered || choice.activeFocus
                antialiasing: true
            }

            SystemMenuGlyph {
                anchors.centerIn: parent
                glyph: choice.modelData.value
                size: Theme.controls.menuBar.iconSize
                color: Theme.color.textPrimary
                shadowColor: choice.glyphShadowColor
                shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)
            }
        }

        Rectangle {
            visible: choice.showSeparator
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.settingsRow.paddingH
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.settingsRow.paddingH
            anchors.bottom: parent.bottom
            height: Theme.controls.window.borderWidth
            color: Theme.color.separator
        }

        HoverHandler { id: choiceHover }

        TapHandler {
            onTapped: choice.choose()
        }

        FocusRing {
            target: choice
            shown: choice.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                choice.choose();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: choice.modelData.label
        Accessible.description: choice.modelData.description
        Accessible.checkable: true
        Accessible.checked: choice.selected
        Accessible.focusable: true
        Accessible.onPressAction: choice.choose()
    }
}
