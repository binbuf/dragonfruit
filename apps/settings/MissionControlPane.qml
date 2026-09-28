// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Mission Control & Hot Corners pane (T-15.5b).
//
// Mission Control and hot corners are compositor-native: the compositor
// detects corners (`compositor/src/input/hot_corners.rs`) and owns the one
// overview state machine (`compositor/src/overview/mod.rs`), and the shell
// mirrors both over the private `df_toplevel_manager` bridge. This pane is the
// *configuration* half: every row is a settingsd key, so it applies live and
// persists. The gesture rows reuse the revision-1 `gestures.*` keys the
// compositor already applies over `set_input_policy`; the four hot-corner rows
// write the revision-13 `overview.hotCorner*` keys. Applying a corner
// assignment needs the append-only compositor request ADR 0126 names and is
// deferred; the persisted preference is the single source of truth the shell
// summarizes in the Control Center tile.
//
// Deviations from the macOS capture reference (ADR 0122): the `Stage Manager`
// toggle and the `In Stage Manager` segment map to our overview/workspaces and
// are not part of Mission Control's trigger configuration, so they are omitted;
// the Apple Account sidebar row is omitted project-wide. Mission Control and
// hot corners are not in the Tahoe capture set, so this pane follows the shared
// settings-window language (grouped inset rows, live apply, no Apply button).
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

    // The gesture trio the compositor applies live.
    readonly property bool gestureMissionControl:
        Settings.values["gestures.missionControl"] === true
    readonly property bool gestureSpaceSwitch:
        Settings.values["gestures.spaceSwitch"] === true

    readonly property var actionOptions: [
        { value: "none", label: qsTr("\u2013") },
        { value: "mission-control", label: qsTr("Mission Control") },
        { value: "notification-center", label: qsTr("Notification Center") },
        { value: "desktop-reveal", label: qsTr("Desktop") },
        { value: "lock-screen", label: qsTr("Lock Screen") }
    ]

    // Test surface (used by tst_settings_mission_control.qml).
    property alias gestureMissionToggle: gestureMissionToggle
    property alias gestureSpaceToggle: gestureSpaceToggle
    property alias topLeftSelect: topLeftSelect
    property alias topRightSelect: topRightSelect
    property alias bottomLeftSelect: bottomLeftSelect
    property alias bottomRightSelect: bottomRightSelect
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function actionIndex(value) {
        for (var i = 0; i < root.actionOptions.length; ++i) {
            if (root.actionOptions[i].value === value)
                return i;
        }
        return 0;
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
                objectName: "missionControlAbsenceNote"
                width: parent.width
                text: qsTr("The settings daemon is not running, so these "
                           + "preferences apply from the built-in defaults. "
                           + "Mission Control itself is run by the compositor.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Mission Control ───────────────────────────────────────────────
        SettingsGroup {
            id: missionControlGroup
            width: parent.width
            title: qsTr("Mission Control")

            SettingsRow {
                width: parent.width
                label: qsTr("Swipe up to open")
                description: qsTr("Reveal every open window and Space in the "
                                  + "overview with a three-finger swipe.")
                controlData: Toggle {
                    id: gestureMissionToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.missionControl", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Swipe between Spaces")
                showSeparator: false
                description: qsTr("Swipe left or right with three fingers to "
                                  + "move between Spaces.")
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
                           + "Choose \u2013 to leave a corner inactive.")
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
                controlData: Select {
                    id: topLeftSelect
                    accessibleName: qsTr("Top Left hot corner")
                    model: root.actionOptions
                    onActivated: (index) => Settings.set(
                                    "overview.hotCornerTopLeft",
                                    root.actionOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Top Right")
                controlData: Select {
                    id: topRightSelect
                    accessibleName: qsTr("Top Right hot corner")
                    model: root.actionOptions
                    onActivated: (index) => Settings.set(
                                    "overview.hotCornerTopRight",
                                    root.actionOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Bottom Left")
                controlData: Select {
                    id: bottomLeftSelect
                    accessibleName: qsTr("Bottom Left hot corner")
                    model: root.actionOptions
                    onActivated: (index) => Settings.set(
                                    "overview.hotCornerBottomLeft",
                                    root.actionOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Bottom Right")
                showSeparator: false
                controlData: Select {
                    id: bottomRightSelect
                    accessibleName: qsTr("Bottom Right hot corner")
                    model: root.actionOptions
                    onActivated: (index) => Settings.set(
                                    "overview.hotCornerBottomRight",
                                    root.actionOptions[index].value)
                }
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
}