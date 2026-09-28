// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The shared Keyboard / Mouse / Trackpad pane body (T-15.4b).
//
// One body serves the three catalog panes through `section`; the three thin
// wrappers (KeyboardPane/MousePane/TrackpadPane) set it. Every preference row
// is a settingsd key (`input.*`, revision 12) written through the `Settings`
// singleton, so it applies live and persists. The device inventory comes from
// the read-only libinput adapter (T-15.4a) through the bridge host: it lists
// what the session has, and its absence is a normal state (a one-line note,
// never an error).
//
// Deviations from the macOS capture (ADR 0122): the `Dictation` section, the
// `Text Input`/`Input Sources` rows, `Force Click and haptic feedback`, and
// `Look up & data detectors` are Apple-only and omitted; `Keyboard Shortcuts…`
// opens a shortcut editor that does not exist yet and is omitted; the `⌘` glyph
// is our Super key label; the keyboard-backlight and emoji-panel hardware
// bridges are deferred (the rows persist their preference now). The `Mouse`
// capture does not exist, so its rows follow the Trackpad `Point & Click`
// controls (`Tracking speed`, natural scrolling, handedness).
Item {
    id: root

    // Which catalog pane this body renders.
    property string section: "keyboard"
    readonly property bool keyboard: root.section === "keyboard"
    readonly property bool mouse: root.section === "mouse"
    readonly property bool trackpad: root.section === "trackpad"

    // The read-only libinput inventory (empty when the host is absent).
    readonly property var view: Settings.input
    readonly property bool available: Settings.inputAvailable
    readonly property bool present: view.state === "available"
    readonly property bool ready: root.available && root.present
    readonly property var devices: view.devices !== undefined ? view.devices : []

    // The settingsd-backed preference values.
    readonly property int repeatRate: Settings.values["input.repeatRate"] !== undefined
        ? Number(Settings.values["input.repeatRate"]) : 25
    readonly property int repeatDelay: Settings.values["input.repeatDelay"] !== undefined
        ? Number(Settings.values["input.repeatDelay"]) : 200
    readonly property real pointerSpeed:
        Settings.values["input.pointerSpeed"] !== undefined
            ? Number(Settings.values["input.pointerSpeed"]) : 0.0
    readonly property bool naturalScroll:
        Settings.values["input.naturalScroll"] === true
    readonly property bool tapToClick: Settings.values["input.tapToClick"] === true
    readonly property bool leftHanded: Settings.values["input.leftHanded"] === true
    readonly property string scrollMethod:
        Settings.values["input.scrollMethod"] || "two-finger"
    readonly property real keyboardBrightness:
        Settings.values["input.keyboardBrightness"] !== undefined
            ? Number(Settings.values["input.keyboardBrightness"]) : 0.5
    readonly property bool adjustBrightnessLowLight:
        Settings.values["input.adjustBrightnessLowLight"] === true
    readonly property int backlightOffAfter:
        Settings.values["input.backlightOffAfter"] !== undefined
            ? Number(Settings.values["input.backlightOffAfter"]) : 0
    readonly property bool keyboardNavigation:
        Settings.values["input.keyboardNavigation"] === true
    readonly property string emojiKeyAction:
        Settings.values["input.emojiKeyAction"] || "emoji"
    readonly property bool gesturesEnabled: Settings.values["gestures.enabled"] === true
    readonly property bool gestureSpaceSwitch:
        Settings.values["gestures.spaceSwitch"] === true
    readonly property bool gestureMissionControl:
        Settings.values["gestures.missionControl"] === true

    // The Trackpad segmented control: 0 = Point & Click, 1 = Scroll & Zoom,
    // 2 = More Gestures.
    property int tabIndex: 0
    readonly property bool pointClick: root.tabIndex === 0
    readonly property bool scrollZoom: root.tabIndex === 1
    readonly property bool moreGestures: root.tabIndex === 2

    readonly property var tabOptions: [
        { label: qsTr("Point & Click") },
        { label: qsTr("Scroll & Zoom") },
        { label: qsTr("More Gestures") }
    ]
    readonly property var scrollMethodOptions: [
        { value: "two-finger", label: qsTr("Two Fingers") },
        { value: "edge", label: qsTr("Edge Scrolling") },
        { value: "button", label: qsTr("Button Scrolling") }
    ]
    readonly property var backlightOffOptions: [
        { value: 0, label: qsTr("Never") },
        { value: 5, label: qsTr("After 5 seconds") },
        { value: 30, label: qsTr("After 30 seconds") },
        { value: 60, label: qsTr("After 1 minute") },
        { value: 300, label: qsTr("After 5 minutes") }
    ]
    readonly property var emojiKeyOptions: [
        { value: "emoji", label: qsTr("Show Emoji & Symbols") },
        { value: "none", label: qsTr("Do Nothing") }
    ]

    // Test surface (used by tst_settings_input.qml).
    property alias inventoryGroup: inventoryGroup
    property alias deviceRepeater: deviceRepeater
    property alias absenceNote: absenceNote
    property alias repeatRateSlider: repeatRateSlider
    property alias repeatDelaySlider: repeatDelaySlider
    property alias keyboardBrightnessSlider: keyboardBrightnessSlider
    property alias adjustBrightnessToggle: adjustBrightnessToggle
    property alias backlightOffSelect: backlightOffSelect
    property alias keyboardNavigationToggle: keyboardNavigationToggle
    property alias emojiKeySelect: emojiKeySelect
    property alias tabControl: tabControl
    property alias pointerSpeedSlider: pointerSpeedSlider
    property alias tapToClickToggle: tapToClickToggle
    property alias naturalScrollToggle: naturalScrollToggle
    property alias leftHandedToggle: leftHandedToggle
    property alias scrollMethodSelect: scrollMethodSelect
    property alias gesturesEnabledToggle: gesturesEnabledToggle
    property alias gestureSpaceToggle: gestureSpaceToggle
    property alias gestureMissionToggle: gestureMissionToggle

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function scrollMethodIndex() {
        for (var i = 0; i < root.scrollMethodOptions.length; ++i) {
            if (root.scrollMethodOptions[i].value === root.scrollMethod)
                return i;
        }
        return 0;
    }

    function backlightOffIndex() {
        for (var i = 0; i < root.backlightOffOptions.length; ++i) {
            if (root.backlightOffOptions[i].value === root.backlightOffAfter)
                return i;
        }
        return 0;
    }

    function emojiKeyIndex() {
        return root.emojiKeyAction === "none" ? 1 : 0;
    }

    // The repeat-rate slider's 0..1 fraction for the settings value.
    function repeatRateFraction() { return root.repeatRate / 200.0; }
    // The delay slider is inverted to match the capture (left Long, right
    // Short): the fraction is 1 at 0 ms and 0 at 5000 ms.
    function repeatDelayFraction() { return 1.0 - (root.repeatDelay / 5000.0); }

    // Opening the pane asks the host for a re-read.
    Component.onCompleted: Settings.refreshInput()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Device inventory / absence ───────────────────────────────────
        SettingsGroup {
            id: inventoryGroup
            width: parent.width
            title: qsTr("Devices")

            Text {
                objectName: "inputNoDevices"
                width: parent.width
                visible: root.ready && root.devices.length === 0
                text: qsTr("No input devices are available on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                leftPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: deviceRepeater
                model: root.devices
                delegate: Item {
                    id: deviceRow
                    required property var modelData
                    width: parent.width
                    height: 36

                    Text {
                        objectName: "inputDeviceName"
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: deviceKindLabel.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: deviceRow.modelData.name
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Text {
                        id: deviceKindLabel
                        objectName: "inputDeviceKind"
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        text: deviceRow.modelData.kindLabel !== undefined
                            ? deviceRow.modelData.kindLabel : ""
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
        }

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "inputAbsenceNote"
                width: parent.width
                text: qsTr("No keyboard, mouse, or trackpad inventory is available. "
                           + "Your preferences below still apply.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Keyboard ──────────────────────────────────────────────────────
        SettingsGroup {
            id: keyboardGroup
            width: parent.width
            visible: root.keyboard
            title: qsTr("Keyboard")

            Item {
                width: parent.width
                implicitHeight: repeatRateColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: repeatRateColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Key repeat rate")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: repeatRateSlider
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        accessibleName: qsTr("Key repeat rate")
                        minLabel: qsTr("Off")
                        maxLabel: qsTr("Fast")
                        onMoved: (value) => Settings.set("input.repeatRate",
                                                         Math.round(value * 200.0))
                    }
                }
            }

            Item {
                width: parent.width
                implicitHeight: repeatDelayColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: repeatDelayColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Delay until repeat")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: repeatDelaySlider
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        accessibleName: qsTr("Delay until repeat")
                        minLabel: qsTr("Long")
                        maxLabel: qsTr("Short")
                        onMoved: (value) => Settings.set("input.repeatDelay",
                                                         Math.round((1.0 - value) * 5000.0))
                    }
                }
            }
        }

        SettingsGroup {
            id: keyboardBrightnessGroup
            width: parent.width
            visible: root.keyboard
            title: qsTr("Keyboard Brightness")

            SettingsRow {
                width: parent.width
                label: qsTr("Adjust keyboard brightness in low light")
                description: qsTr("The hardware backlight bridge is a follow-up.")
                controlData: Toggle {
                    id: adjustBrightnessToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.adjustBrightnessLowLight", checked)
                }
            }

            Item {
                width: parent.width
                implicitHeight: keyboardBrightnessColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: keyboardBrightnessColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Keyboard brightness")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: keyboardBrightnessSlider
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        accessibleName: qsTr("Keyboard brightness")
                        minLabel: qsTr("Dim")
                        maxLabel: qsTr("Bright")
                        onMoved: (value) =>
                            Settings.set("input.keyboardBrightness", value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Turn keyboard backlight off after inactivity")
                controlData: Select {
                    id: backlightOffSelect
                    accessibleName: qsTr("Turn keyboard backlight off after inactivity")
                    model: root.backlightOffOptions
                    onActivated: (index) => Settings.set(
                                    "input.backlightOffAfter",
                                    root.backlightOffOptions[index].value)
                }
            }
        }

        SettingsGroup {
            id: keyboardNavigationGroup
            width: parent.width
            visible: root.keyboard
            title: qsTr("Keyboard Navigation")

            SettingsRow {
                width: parent.width
                label: qsTr("Keyboard navigation")
                description: qsTr("Use keyboard navigation to move focus between "
                                  + "controls. Press the Tab key to move focus forward "
                                  + "and Shift Tab to move focus backward.")
                controlData: Toggle {
                    id: keyboardNavigationToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.keyboardNavigation", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Press Super key to")
                showSeparator: false
                controlData: Select {
                    id: emojiKeySelect
                    accessibleName: qsTr("Press Super key to")
                    model: root.emojiKeyOptions
                    onActivated: (index) => Settings.set(
                                    "input.emojiKeyAction",
                                    root.emojiKeyOptions[index].value)
                }
            }
        }

        // ── Mouse ─────────────────────────────────────────────────────────
        SettingsGroup {
            id: mouseGroup
            width: parent.width
            visible: root.mouse
            title: qsTr("Mouse")

            Item {
                width: parent.width
                implicitHeight: mouseSpeedColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: mouseSpeedColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Tracking speed")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: pointerSpeedSlider
                        width: parent.width
                        from: -1.0
                        to: 1.0
                        accessibleName: qsTr("Tracking speed")
                        minLabel: qsTr("Slow")
                        maxLabel: qsTr("Fast")
                        onMoved: (value) => Settings.set("input.pointerSpeed", value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Natural scrolling")
                description: qsTr("Content moves in the same direction as your fingers.")
                controlData: Toggle {
                    id: naturalScrollToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.naturalScroll", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Left-handed")
                description: qsTr("Swap the primary and secondary buttons.")
                controlData: Toggle {
                    id: leftHandedToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.leftHanded", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Scroll method")
                showSeparator: false
                controlData: Select {
                    id: scrollMethodSelect
                    accessibleName: qsTr("Scroll method")
                    model: root.scrollMethodOptions
                    onActivated: (index) => Settings.set(
                                    "input.scrollMethod",
                                    root.scrollMethodOptions[index].value)
                }
            }
        }

        // ── Trackpad ──────────────────────────────────────────────────────
        SettingsGroup {
            id: trackpadGroup
            width: parent.width
            visible: root.trackpad
            title: qsTr("Trackpad")

            // A simple original preview: an outline with a finger dot, standing
            // in for the capture's simulated trackpad illustration.
            Item {
                width: parent.width
                height: 72

                Rectangle {
                    anchors.centerIn: parent
                    width: 96
                    height: 64
                    radius: Theme.primitive.radius.sm
                    color: Theme.color.surfaceSunken
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        anchors.bottom: parent.bottom
                        anchors.bottomMargin: 12
                        width: 12
                        height: 12
                        radius: 6
                        color: Theme.color.accent
                    }
                }
            }

            SegmentedControl {
                id: tabControl
                width: parent.width
                model: root.tabOptions
                onActivated: (index) => root.tabIndex = index
            }

            Item {
                width: parent.width
                visible: root.pointClick
                implicitHeight: trackpadSpeedColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: trackpadSpeedColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Tracking speed")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: trackpadSpeedSlider
                        width: parent.width
                        from: -1.0
                        to: 1.0
                        accessibleName: qsTr("Tracking speed")
                        minLabel: qsTr("Slow")
                        maxLabel: qsTr("Fast")
                        onMoved: (value) => Settings.set("input.pointerSpeed", value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.pointClick
                label: qsTr("Tap to click")
                description: qsTr("Tap with one finger.")
                showSeparator: false
                controlData: Toggle {
                    id: tapToClickToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.tapToClick", checked)
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Natural scrolling")
                description: qsTr("Content moves in the same direction as your fingers.")
                controlData: Toggle {
                    id: trackpadNaturalToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.naturalScroll", checked)
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Scroll method")
                showSeparator: false
                controlData: Select {
                    id: trackpadScrollMethodSelect
                    accessibleName: qsTr("Scroll method")
                    model: root.scrollMethodOptions
                    onActivated: (index) => Settings.set(
                                    "input.scrollMethod",
                                    root.scrollMethodOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Gestures")
                description: qsTr("Swipe between Spaces and open Mission Control.")
                controlData: Toggle {
                    id: gesturesEnabledToggle
                    text: ""
                    onToggled: (checked) => Settings.set("gestures.enabled", checked)
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Swipe between Spaces")
                controlData: Toggle {
                    id: gestureSpaceToggle
                    text: ""
                    onToggled: (checked) => Settings.set("gestures.spaceSwitch", checked)
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Mission Control")
                showSeparator: false
                controlData: Toggle {
                    id: gestureMissionToggle
                    text: ""
                    onToggled: (checked) => Settings.set("gestures.missionControl", checked)
                }
            }
        }
    }

    // Keep the controls in step with external changes.
    Binding {
        target: repeatRateSlider
        property: "value"
        value: root.repeatRateFraction()
    }
    Binding {
        target: repeatDelaySlider
        property: "value"
        value: root.repeatDelayFraction()
    }
    Binding {
        target: adjustBrightnessToggle
        property: "checked"
        value: root.adjustBrightnessLowLight
    }
    Binding {
        target: keyboardBrightnessSlider
        property: "value"
        value: root.keyboardBrightness
    }
    Binding {
        target: backlightOffSelect
        property: "currentIndex"
        value: root.backlightOffIndex()
    }
    Binding {
        target: keyboardNavigationToggle
        property: "checked"
        value: root.keyboardNavigation
    }
    Binding {
        target: emojiKeySelect
        property: "currentIndex"
        value: root.emojiKeyIndex()
    }
    Binding {
        target: pointerSpeedSlider
        property: "value"
        value: root.pointerSpeed
    }
    Binding {
        target: naturalScrollToggle
        property: "checked"
        value: root.naturalScroll
    }
    Binding {
        target: leftHandedToggle
        property: "checked"
        value: root.leftHanded
    }
    Binding {
        target: scrollMethodSelect
        property: "currentIndex"
        value: root.scrollMethodIndex()
    }
    Binding {
        target: trackpadSpeedSlider
        property: "value"
        value: root.pointerSpeed
    }
    Binding {
        target: tapToClickToggle
        property: "checked"
        value: root.tapToClick
    }
    Binding {
        target: trackpadNaturalToggle
        property: "checked"
        value: root.naturalScroll
    }
    Binding {
        target: trackpadScrollMethodSelect
        property: "currentIndex"
        value: root.scrollMethodIndex()
    }
    Binding {
        target: gesturesEnabledToggle
        property: "checked"
        value: root.gesturesEnabled
    }
    Binding {
        target: gestureSpaceToggle
        property: "checked"
        value: root.gestureSpaceSwitch
    }
    Binding {
        target: gestureMissionToggle
        property: "checked"
        value: root.gestureMissionControl
    }
    Binding {
        target: tabControl
        property: "currentIndex"
        value: root.tabIndex
    }

    // The trackpad speed slider shares `pointerSpeed` with the mouse one.
    property alias trackpadSpeed: trackpadSpeedSlider
}