// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Screen Saver pane (T227).
//
// The shipped screen-saver surface is the first-party lock screen: the style
// selects its clock presentation, the idle time is the session idle engine's
// `idle.blank` key, and the options control the clock/date/seconds the lock
// scene renders. Animated screen savers are not advertised until a renderer
// exists.
Item {
    id: root

    readonly property var styleOptions: [
        { value: "clock", label: qsTr("Clock") },
        { value: "none", label: qsTr("None") }
    ]
    readonly property var startAfterOptions: [
        { value: 0, label: qsTr("Never") },
        { value: 60, label: qsTr("After 1 minute") },
        { value: 300, label: qsTr("After 5 minutes") },
        { value: 600, label: qsTr("After 10 minutes") },
        { value: 1200, label: qsTr("After 20 minutes") },
        { value: 1800, label: qsTr("After 30 minutes") },
        { value: 3600, label: qsTr("After 1 hour") }
    ]

    readonly property string style: Settings.values["screenSaver.style"] || "clock"
    readonly property int startAfter: {
        var value = Number(Settings.values["idle.blank"]);
        return isFinite(value) ? Math.max(0, Math.round(value)) : 300;
    }

    signal navigationRequested(string paneId)

    function styleIndex() {
        for (var i = 0; i < root.styleOptions.length; ++i) {
            if (root.styleOptions[i].value === root.style)
                return i;
        }
        return 0;
    }

    function startAfterIndex() {
        for (var i = 0; i < root.startAfterOptions.length; ++i) {
            if (root.startAfterOptions[i].value === root.startAfter)
                return i;
        }
        // A custom idle value keeps the nearest lower option selected.
        for (var j = root.startAfterOptions.length - 1; j >= 0; --j) {
            if (root.startAfterOptions[j].value <= root.startAfter)
                return j;
        }
        return 0;
    }

    // Test surface (used by tst_settings_screen_saver.qml).
    property alias styleControl: styleControl
    property alias startAfterSelect: startAfterSelect
    property alias clockToggle: clockToggle
    property alias showDateToggle: showDateToggle
    property alias showSecondsToggle: showSecondsToggle
    property alias hotCornersButton: hotCornersButton
    property alias styleGroup: styleGroup
    property alias optionsGroup: optionsGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            id: styleGroup
            width: parent.width
            title: qsTr("Screen Saver")

            SettingsRow {
                width: parent.width
                label: qsTr("Screen saver")
                settingsId: "style"
                owner: "settingsd/shell"
                settingsKey: "screenSaver.style"
                description: qsTr("The clock style reuses the first-party lock "
                                  + "surface. Animated savers are not shipped.")
                showSeparator: false

                controlData: SegmentedControl {
                    id: styleControl
                    model: root.styleOptions.map(function(option) { return option.label; })
                    onActivated: (index) => Settings.set(
                                    "screenSaver.style",
                                    root.styleOptions[index].value)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Start After")

            SettingsRow {
                width: parent.width
                label: qsTr("Start after")
                description: qsTr("The session idle engine blanks the display after "
                                  + "this much inactivity.")
                settingsId: "startAfter"
                owner: "settingsd -> session/idle engine"
                settingsKey: "idle.blank"
                showSeparator: false

                controlData: Select {
                    id: startAfterSelect
                    accessibleName: qsTr("Start the screen saver after")
                    model: root.startAfterOptions
                    onActivated: (index) => Settings.set(
                                    "idle.blank",
                                    root.startAfterOptions[index].value)
                }
            }
        }

        SettingsGroup {
            id: optionsGroup
            width: parent.width
            title: qsTr("Options")

            SettingsRow {
                width: parent.width
                label: qsTr("Show with clock")
                settingsId: "clock"
                owner: "settingsd -> shell/LockScreen"
                settingsKey: "screenSaver.clock"
                controlData: Toggle {
                    id: clockToggle
                    text: ""
                    onToggled: (checked) => Settings.set("screenSaver.clock", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show date")
                settingsId: "showDate"
                owner: "settingsd -> shell/LockScreen"
                settingsKey: "screenSaver.showDate"
                controlData: Toggle {
                    id: showDateToggle
                    text: ""
                    onToggled: (checked) => Settings.set("screenSaver.showDate", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show seconds")
                settingsId: "showSeconds"
                owner: "settingsd -> shell/LockScreen"
                settingsKey: "screenSaver.showSeconds"
                showSeparator: false
                controlData: Toggle {
                    id: showSecondsToggle
                    text: ""
                    onToggled: (checked) => Settings.set("screenSaver.showSeconds", checked)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Hot Corners")

            SettingsRow {
                width: parent.width
                label: qsTr("Hot corners")
                description: qsTr("The four corner assignments are shared with "
                                  + "Overview.")
                settingsId: "hotCorners"
                owner: "settingsd -> compositor/input"
                settingsKey: "overview.hotCorner*"
                showSeparator: false

                controlData: Button {
                    id: hotCornersButton
                    text: qsTr("Open Overview")
                    onClicked: root.navigationRequested("mission-control")
                }
            }
        }
    }

    Binding {
        target: styleControl
        property: "currentIndex"
        value: root.styleIndex()
    }
    Binding {
        target: startAfterSelect
        property: "currentIndex"
        value: root.startAfterIndex()
    }
    Binding {
        target: clockToggle
        property: "checked"
        value: Settings.values["screenSaver.clock"] !== false
    }
    Binding {
        target: showDateToggle
        property: "checked"
        value: Settings.values["screenSaver.showDate"] !== false
    }
    Binding {
        target: showSecondsToggle
        property: "checked"
        value: Settings.values["screenSaver.showSeconds"] === true
    }
}
