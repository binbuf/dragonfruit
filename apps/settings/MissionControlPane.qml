// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Overview pane (T-15.5b; T276 completion).
//
// Overview, workspaces, and hot corners are compositor-native: the
// compositor detects corners (`compositor/src/input/hot_corners.rs`), owns the
// one overview state machine (`compositor/src/overview/mod.rs`) and the
// workspace model, and the shell mirrors both over the private
// `df_toplevel_manager` bridge. This pane is the *configuration* half: every
// live row is a settingsd key, so it applies and persists. The shell forwards
// the corner assignments, their modifiers, the workspace count, and the
// app-Space switch in one v13 `set_overview_policy` snapshot.
//
// Rows whose runtime is not implemented yet stay visible and disabled with an
// honest reason; the owning task lives in the map and docs, never in the UI.
Item {
    id: root

    // The settingsd-backed corner assignments, spelled with the stable
    // `HotCornerAction::id()` vocabulary the compositor's wire uses.
    readonly property string topLeft:
        Settings.values["overview.hotCornerTopLeft"] || "mission-control"
    readonly property string topRight:
        Settings.values["overview.hotCornerTopRight"] || "notification-center"
    readonly property string bottomLeft:
        Settings.values["overview.hotCornerBottomLeft"] || "desktop-reveal"
    readonly property string bottomRight:
        Settings.values["overview.hotCornerBottomRight"] || "lock-screen"

    readonly property string topLeftModifier:
        Settings.values["overview.hotCornerTopLeftModifier"] || "none"
    readonly property string topRightModifier:
        Settings.values["overview.hotCornerTopRightModifier"] || "none"
    readonly property string bottomLeftModifier:
        Settings.values["overview.hotCornerBottomLeftModifier"] || "none"
    readonly property string bottomRightModifier:
        Settings.values["overview.hotCornerBottomRightModifier"] || "none"

    readonly property bool gestureMissionControl:
        Settings.values["gestures.missionControl"] === true
    readonly property bool gestureSpaceSwitch:
        Settings.values["gestures.spaceSwitch"] === true
    readonly property bool switchToAppSpace:
        Settings.values["overview.switchToAppSpace"] !== false
    readonly property int workspaceCount:
        Settings.values["workspaces.count"] === undefined
            ? 3 : Number(Settings.values["workspaces.count"])

    // The full action vocabulary shared with the compositor. The first five
    // keep their historical order so stored values and the earlier tests stay
    // stable; the additions map to the shell's existing input actions.
    readonly property var actionOptions: [
        { value: "none", label: qsTr("\u2013") },
        { value: "mission-control", label: qsTr("Overview") },
        { value: "notification-center", label: qsTr("Notifications") },
        { value: "desktop-reveal", label: qsTr("Desktop") },
        { value: "lock-screen", label: qsTr("Lock Screen") },
        { value: "launchpad", label: qsTr("Applications") },
        { value: "control-center", label: qsTr("Quick Settings") },
        { value: "show-dock", label: qsTr("Show Dock") },
        { value: "menu-bar", label: qsTr("Menu Bar") },
        { value: "focus-dock", label: qsTr("Focus Dock") }
    ]
    readonly property var modifierOptions: [
        { value: "none", label: qsTr("\u2013") },
        { value: "command", label: qsTr("Command") },
        { value: "shift", label: qsTr("Shift") },
        { value: "option", label: qsTr("Option") },
        { value: "control", label: qsTr("Control") }
    ]
    readonly property var workspaceCountOptions: {
        var options = [];
        for (var count = 1; count <= 16; ++count)
            options.push({ value: count, label: String(count) });
        return options;
    }

    // Test surface (used by tst_settings_mission_control.qml).
    property alias gestureMissionToggle: gestureMissionToggle
    property alias gestureSpaceToggle: gestureSpaceToggle
    property alias topLeftSelect: topLeftSelect
    property alias topRightSelect: topRightSelect
    property alias bottomLeftSelect: bottomLeftSelect
    property alias bottomRightSelect: bottomRightSelect
    property alias topLeftModifierSelect: topLeftModifierSelect
    property alias topRightModifierSelect: topRightModifierSelect
    property alias bottomLeftModifierSelect: bottomLeftModifierSelect
    property alias bottomRightModifierSelect: bottomRightModifierSelect
    property alias switchToAppSpaceToggle: switchToAppSpaceToggle
    property alias workspaceCountSelect: workspaceCountSelect
    property alias rearrangeRow: rearrangeRow
    property alias groupByAppRow: groupByAppRow
    property alias perDisplayRow: perDisplayRow
    property alias dragToOpenRow: dragToOpenRow
    property alias shortcutsRow: shortcutsRow
    property alias absenceNote: absenceNote
    property alias missionControlGroup: missionControlGroup
    property alias workspacesGroup: workspacesGroup
    property alias hotCornersGroup: hotCornersGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function optionIndex(options, value) {
        for (var i = 0; i < options.length; ++i) {
            if (options[i].value === value)
                return i;
        }
        return 0;
    }
    function actionIndex(value) {
        return root.optionIndex(root.actionOptions, value);
    }
    function modifierIndex(value) {
        return root.optionIndex(root.modifierOptions, value);
    }
    function workspaceCountIndex() {
        return Math.max(0, Math.min(root.workspaceCountOptions.length - 1,
                                    root.workspaceCount - 1));
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !Settings.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "missionControlAbsenceNote"
                kind: "live-on-defaults"
                showWhenLive: true
                reason: qsTr("The settings daemon is not running, so these "
                             + "preferences apply from the built-in defaults. "
                             + "Overview itself is run by the compositor.")
            }
        }

        // ── Overview ───────────────────────────────────────────────
        SettingsGroup {
            id: missionControlGroup
            width: parent.width
            title: qsTr("Window Overview")

            SettingsRow {
                id: rearrangeRow
                width: parent.width
                label: qsTr("Automatically rearrange workspaces")
                description: qsTr("Automatic workspace reordering is not available "
                                  + "yet.")
                settingsId: "rearrange"
                owner: "compositor"
                settingsKey: "overview.autoRearrange"
                enabled: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Switch to an application's workspace")
                settingsId: "switchToAppSpace"
                owner: "settingsd -> compositor"
                settingsKey: "overview.switchToAppSpace"
                description: qsTr("Activating a window on another workspace follows "
                                  + "it there.")
                controlData: Toggle {
                    id: switchToAppSpaceToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("overview.switchToAppSpace", checked)
                }
            }

            SettingsRow {
                id: groupByAppRow
                width: parent.width
                label: qsTr("Group windows by application")
                description: qsTr("Grouping windows by application is not available "
                                  + "yet.")
                settingsId: "groupByApp"
                owner: "compositor"
                settingsKey: "overview.groupByApp"
                enabled: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: perDisplayRow
                width: parent.width
                label: qsTr("Displays have separate workspaces")
                description: qsTr("Each display keeps its own workspace list in the "
                                  + "compositor; a shared-space mode is not "
                                  + "available.")
                settingsId: "perDisplay"
                owner: "compositor"
                settingsKey: ""

                controlData: Text {
                    text: qsTr("On")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            SettingsRow {
                id: dragToOpenRow
                width: parent.width
                label: qsTr("Drag windows to the top to open Overview")
                description: qsTr("Drag-to-Overview is not available yet.")
                settingsId: "dragToOpen"
                owner: "compositor"
                settingsKey: "overview.dragToOpen"
                enabled: false
                showSeparator: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }
        }

        // ── Workspaces and gestures ───────────────────────────────────────
        SettingsGroup {
            id: workspacesGroup
            width: parent.width
            title: qsTr("Workspaces")

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Workspace count")
                settingsId: "workspaceCount"
                owner: "settingsd -> compositor"
                settingsKey: "workspaces.count"
                description: qsTr("Normal workspaces every display starts with.")
                controlData: Select {
                    id: workspaceCountSelect
                    accessibleName: qsTr("Workspace count")
                    model: root.workspaceCountOptions
                    onActivated: (index) => Settings.set(
                                    "workspaces.count",
                                    root.workspaceCountOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Swipe up to open")
                settingsId: "swipeUp"
                owner: "settingsd -> compositor"
                settingsKey: "gestures.missionControl"
                description: qsTr("Reveal every open window and workspace in the "
                                  + "overview with a three-finger swipe.")
                controlData: Toggle {
                    id: gestureMissionToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.missionControl", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Swipe between Workspaces")
                settingsId: "spaceSwitch"
                owner: "settingsd -> compositor"
                settingsKey: "gestures.spaceSwitch"
                description: qsTr("Swipe left or right with three fingers to "
                                  + "move between workspaces.")
                showSeparator: false
                controlData: Toggle {
                    id: gestureSpaceToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.spaceSwitch", checked)
                }
            }
        }

        // ── Hot Corners ───────────────────────────────────────────────────
        SettingsGroup {
            id: hotCornersGroup
            width: parent.width
            title: qsTr("Hot Corners")

            Text {
                objectName: "hotCornersDescription"
                width: parent.width
                text: qsTr("Move the pointer to a corner to perform an action. "
                           + "Choose \u2013 to leave a corner inactive, or hold a "
                           + "modifier while entering the corner.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Top Left")
                settingsId: "topLeft"
                controlData: Row {
                    spacing: Theme.primitive.spacing.xs
                    Select {
                        id: topLeftSelect
                        accessibleName: qsTr("Top Left hot corner")
                        model: root.actionOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerTopLeft",
                                        root.actionOptions[index].value)
                    }
                    Select {
                        id: topLeftModifierSelect
                        accessibleName: qsTr("Top Left hot corner modifier")
                        model: root.modifierOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerTopLeftModifier",
                                        root.modifierOptions[index].value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Top Right")
                settingsId: "topRight"
                controlData: Row {
                    spacing: Theme.primitive.spacing.xs
                    Select {
                        id: topRightSelect
                        accessibleName: qsTr("Top Right hot corner")
                        model: root.actionOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerTopRight",
                                        root.actionOptions[index].value)
                    }
                    Select {
                        id: topRightModifierSelect
                        accessibleName: qsTr("Top Right hot corner modifier")
                        model: root.modifierOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerTopRightModifier",
                                        root.modifierOptions[index].value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Bottom Left")
                settingsId: "bottomLeft"
                controlData: Row {
                    spacing: Theme.primitive.spacing.xs
                    Select {
                        id: bottomLeftSelect
                        accessibleName: qsTr("Bottom Left hot corner")
                        model: root.actionOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerBottomLeft",
                                        root.actionOptions[index].value)
                    }
                    Select {
                        id: bottomLeftModifierSelect
                        accessibleName: qsTr("Bottom Left hot corner modifier")
                        model: root.modifierOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerBottomLeftModifier",
                                        root.modifierOptions[index].value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Bottom Right")
                settingsId: "bottomRight"
                showSeparator: false
                controlData: Row {
                    spacing: Theme.primitive.spacing.xs
                    Select {
                        id: bottomRightSelect
                        accessibleName: qsTr("Bottom Right hot corner")
                        model: root.actionOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerBottomRight",
                                        root.actionOptions[index].value)
                    }
                    Select {
                        id: bottomRightModifierSelect
                        accessibleName: qsTr("Bottom Right hot corner modifier")
                        model: root.modifierOptions
                        onActivated: (index) => Settings.set(
                                        "overview.hotCornerBottomRightModifier",
                                        root.modifierOptions[index].value)
                    }
                }
            }
        }

        // ── Shortcuts ─────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Shortcuts")

            SettingsRow {
                id: shortcutsRow
                width: parent.width
                label: qsTr("Shortcuts…")
                description: qsTr("The shortcut editor is not available yet; system "
                                  + "shortcuts are fixed.")
                settingsId: "shortcuts"
                owner: "compositor"
                settingsKey: "shortcuts.*"
                enabled: false
                showSeparator: false
            }
        }
    }

    // Keep the controls in step with external changes.
    Binding {
        target: gestureMissionToggle
        property: "checked"
        value: root.gestureMissionControl
    }
    Binding {
        target: gestureSpaceToggle
        property: "checked"
        value: root.gestureSpaceSwitch
    }
    Binding {
        target: switchToAppSpaceToggle
        property: "checked"
        value: root.switchToAppSpace
    }
    Binding {
        target: workspaceCountSelect
        property: "currentIndex"
        value: root.workspaceCountIndex()
    }
    Binding {
        target: topLeftSelect
        property: "currentIndex"
        value: root.actionIndex(root.topLeft)
    }
    Binding {
        target: topRightSelect
        property: "currentIndex"
        value: root.actionIndex(root.topRight)
    }
    Binding {
        target: bottomLeftSelect
        property: "currentIndex"
        value: root.actionIndex(root.bottomLeft)
    }
    Binding {
        target: bottomRightSelect
        property: "currentIndex"
        value: root.actionIndex(root.bottomRight)
    }
    Binding {
        target: topLeftModifierSelect
        property: "currentIndex"
        value: root.modifierIndex(root.topLeftModifier)
    }
    Binding {
        target: topRightModifierSelect
        property: "currentIndex"
        value: root.modifierIndex(root.topRightModifier)
    }
    Binding {
        target: bottomLeftModifierSelect
        property: "currentIndex"
        value: root.modifierIndex(root.bottomLeftModifier)
    }
    Binding {
        target: bottomRightModifierSelect
        property: "currentIndex"
        value: root.modifierIndex(root.bottomRightModifier)
    }
}
