// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Menu Bar pane (T-15.9b).
//
// The pane and the Control Center Menu Bar tile are one functional unit. The
// menu bar is shell-native: the shell's `MenuBar` (`shell/menubar/MenuBar.qml`)
// owns the chrome, the status row, and the clock, the menu-broker
// (`services/menu-broker`) resolves the focused app's menus, and settingsd
// owns the durable preferences. This pane is the *configuration* half and
// writes only settingsd keys, so every control applies live and persists:
//
//   * `menu.autoHide`, `menu.showBackground`, and `menu.recentItems` are the
//     behavior rows;
//   * `menu.clock.showDate` / `menu.clock.showSeconds` are the `Clock
//     Options...` rows;
//   * `menu.control.<id>` gates each status item. The shell applies it in
//     `applyStatusItems` (additive with the adapter's own availability), so a
//     control shows only when its daemon is present *and* the user wants it.
//
// The adapter (`dragonfruit-menubar-adapter`, T-15.9a) stays read-only; the
// durable preference is the single source of truth the shell summarizes in the
// Control Center tile.
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * `AirDrop`, `Screen Mirroring`, and `Display` are Apple-only and have no
//     Linux equivalent, so they are omitted (the adapter omits them too).
//   * `Spotlight` is our deferred Search pane, so it is not a menu-bar control
//     here.
//   * `Add Controls...`, `Battery Options...`, and the per-control popups
//     (`Show When Active`) have no Linux provider yet, so they are omitted
//     rather than shipped as dead controls.
//   * the `Recent documents...` stepper is drawn as a popup (`Select`) because
//     the design system has no stepper component; the value is stored policy
//     the recent-items consumer will read.
//   * the Apple Account/sidebar row and the trailing `?` help are omitted
//     project-wide.
Item {
    id: root

    // ── The behavior keys (revision 16, ADR 0135) ───────────────────────
    readonly property string autoHide:
        Settings.values["menu.autoHide"] !== undefined
            ? String(Settings.values["menu.autoHide"]) : "full-screen"
    readonly property bool showBackground:
        Settings.values["menu.showBackground"] !== false
    readonly property int recentItems:
        Settings.values["menu.recentItems"] !== undefined
            ? Number(Settings.values["menu.recentItems"]) : 10

    // ── The clock option keys ───────────────────────────────────────────
    readonly property bool clockShowDate:
        Settings.values["menu.clock.showDate"] !== false
    readonly property bool clockShowSeconds:
        Settings.values["menu.clock.showSeconds"] === true

    // The auto-hide popup choices, in the adapter's `ALL` order.
    readonly property var autoHideOptions: [
        { value: "never", label: qsTr("Never") },
        { value: "always", label: qsTr("Always") },
        { value: "full-screen", label: qsTr("In Full Screen Only") }
    ]

    // The recent-items popup choices; `None` disables the history.
    readonly property var recentOptions: [
        { value: 0, label: qsTr("None") },
        { value: 5, label: qsTr("5") },
        { value: 10, label: qsTr("10") },
        { value: 15, label: qsTr("15") },
        { value: 20, label: qsTr("20") },
        { value: 30, label: qsTr("30") },
        { value: 50, label: qsTr("50") }
    ]

    // The Menu Bar Controls list: every control the adapter can project, in
    // the reference order. The key gates the shell's status row.
    readonly property var controlRows: [
        { key: "menu.control.wifi", label: qsTr("Wi-Fi") },
        { key: "menu.control.bluetooth", label: qsTr("Bluetooth") },
        { key: "menu.control.battery", label: qsTr("Battery") },
        { key: "menu.control.focus", label: qsTr("Focus") },
        { key: "menu.control.volume", label: qsTr("Sound") },
        { key: "menu.control.accessibility", label: qsTr("Accessibility") }
    ]

    function controlVisible(key) {
        return Settings.values[key] !== false;
    }

    // Test surface (used by tst_settings_menu_bar.qml).
    property alias autoHideSelect: autoHideSelect
    property alias backgroundToggle: backgroundToggle
    property alias recentSelect: recentSelect
    property alias clockOptionsButton: clockOptionsButton
    property alias clockDialog: clockDialog
    property alias clockDateToggle: clockDateToggle
    property alias clockSecondsToggle: clockSecondsToggle
    property alias controlsRepeater: controlsRepeater
    property alias absenceNote: absenceNote

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

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !Settings.available

            Text {
                id: absenceNote
                objectName: "menuBarAbsenceNote"
                width: parent.width
                text: qsTr("The settings daemon is not running, so these "
                           + "preferences apply from the built-in defaults. "
                           + "The menu bar itself is drawn by the shell.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Behavior ──────────────────────────────────────────────────────
        SettingsGroup {
            id: behaviorGroup
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Automatically hide and show the menu bar")
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

        // ── Menu Bar Controls ─────────────────────────────────────────────
        SettingsGroup {
            id: controlsGroup
            width: parent.width
            title: qsTr("Menu Bar Controls")

            Text {
                objectName: "menuBarControlsDescription"
                width: parent.width
                text: qsTr("System and app controls can be configured to "
                           + "appear in both Control Center and the menu bar.")
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
                label: qsTr("Clock")
                controlData: Button {
                    id: clockOptionsButton
                    objectName: "menuBarClockOptionsButton"
                    text: qsTr("Clock Options\u2026")
                    onClicked: clockDialog.show()
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
                    showSeparator: index < controlsRepeater.count - 1
                    controlData: Toggle {
                        id: controlToggle
                        objectName: "menuBarControlToggle"
                        accessibleName: modelData.label
                        onToggled: (checked) => Settings.set(modelData.key,
                                                             checked)
                    }

                    Binding {
                        target: controlToggle
                        property: "checked"
                        value: root.controlVisible(modelData.key)
                    }
                }
            }
        }
    }

    // The `Clock Options...` sheet: the two clock options the adapter and the
    // shell's clock consume.
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
}