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
// T229 closes the input breadth: the Input Sources list (settingsd
// `input.sources`, forwarded to the compositor keymap), Text Replacements
// (`input.replacements`, consumed by the session input method), Dictation
// (enabled only when a speech-to-text command is installed), Secondary click,
// Scroll speed, Double-click speed, Look up, Haptics (where a trackpad is
// present), and a Bluetooth-trackpad hand-off. `Keyboard Shortcuts…` still
// opens no editor and is omitted; the `⌘` glyph is our Super key label; the
// keyboard-backlight and emoji-panel hardware bridges are deferred (the rows
// persist their preference now). The `Mouse` capture does not exist, so its
// rows follow the Trackpad `Point & Click` controls.
Item {
    id: root

    // Which catalog pane this body renders.
    property string section: "keyboard"
    signal navigateRequested(string paneId)
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
    // T229: the active keyboard layouts. The first entry is the compositor's
    // active source; an empty list follows the host keymap.
    readonly property var inputSources:
        Settings.values["input.sources"] !== undefined
            ? Settings.values["input.sources"] : []
    readonly property string hostKeymap:
        Settings.general !== undefined && Settings.general.locale !== undefined
            ? String(Settings.general.locale.keymap || "") : ""
    readonly property bool dictationAvailable:
        root.view.dictationAvailable === true
    readonly property bool hasTouchpad: {
        for (var i = 0; i < root.devices.length; ++i)
            if (root.devices[i].kind === "touchpad")
                return true;
        return false;
    }
    readonly property var secondaryClickOptions: [
        { value: "right", label: qsTr("Right") },
        { value: "left", label: qsTr("Left") }
    ]
    readonly property var lookUpOptions: [
        { value: "force-click", label: qsTr("Pressure Click") },
        { value: "three-finger", label: qsTr("Three-Finger Click") },
        { value: "off", label: qsTr("Off") }
    ]
    readonly property var replacementEntries: {
        var raw = Settings.values["input.replacements"] !== undefined
            ? Settings.values["input.replacements"] : [];
        var out = [];
        for (var i = 0; i < raw.length; ++i) {
            var parts = String(raw[i]).split("\t");
            out.push({ from: parts[0] || "", to: parts.length > 1 ? parts[1] : "" });
        }
        return out;
    }
    // T229: the settings keys the new rows own.
    readonly property string secondaryClick:
        Settings.values["input.secondaryClick"] || "right"
    readonly property real scrollSpeed:
        Settings.values["input.scrollSpeed"] !== undefined
            ? Number(Settings.values["input.scrollSpeed"]) : 0.0
    readonly property int doubleClickSpeed:
        Settings.values["input.doubleClickSpeed"] !== undefined
            ? Number(Settings.values["input.doubleClickSpeed"]) : 400
    readonly property bool haptics: Settings.values["input.haptics"] !== false
    readonly property string lookUp:
        Settings.values["input.lookUp"] || "force-click"
    property string newSource: ""
    property string newReplacementFrom: ""
    property string newReplacementTo: ""
    property Item popupParent: null

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
    property alias sourcesRow: sourcesRow
    property alias sourcesDialog: sourcesDialog
    property alias sourcesRepeater: sourcesRepeater
    property alias newSourceInput: newSourceInput
    property alias replacementsRow: replacementsRow
    property alias replacementsDialog: replacementsDialog
    property alias replacementsRepeater: replacementsRepeater
    property alias replacementFromInput: replacementFromInput
    property alias replacementToInput: replacementToInput
    property alias dictationToggle: dictationToggle
    property alias dictationAbsenceNote: dictationAbsenceNote
    property alias secondaryClickSelect: secondaryClickSelect
    property alias scrollSpeedSlider: scrollSpeedSlider
    property alias doubleClickSlider: doubleClickSlider
    property alias hapticsToggle: hapticsToggle
    property alias lookUpSelect: lookUpSelect
    property alias trackpadSecondarySelect: trackpadSecondarySelect
    property alias pairTrackpadRow: pairTrackpadRow

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

    function secondaryClickIndex(value) {
        return value === "left" ? 1 : 0;
    }

    function lookUpIndex() {
        for (var i = 0; i < root.lookUpOptions.length; ++i)
            if (root.lookUpOptions[i].value === root.lookUp)
                return i;
        return 0;
    }

    // The double-click slider runs Fast (left) to Slow (right): 100 ms at 0,
    // 900 ms at 1.
    function doubleClickFraction() {
        return (root.doubleClickSpeed - 100) / 800.0;
    }

    function addSource() {
        var value = root.newSource.trim();
        if (value.length === 0)
            return;
        var list = root.inputSources.slice();
        if (list.indexOf(value) < 0)
            list.push(value);
        Settings.setList("input.sources", list);
        root.newSource = "";
    }

    function removeSource(value) {
        var list = root.inputSources.slice();
        var at = list.indexOf(value);
        if (at < 0)
            return;
        list.splice(at, 1);
        Settings.setList("input.sources", list);
    }

    function addReplacement() {
        var from = root.newReplacementFrom.trim();
        var to = root.newReplacementTo;
        if (from.length === 0)
            return;
        var list = [];
        var raw = Settings.values["input.replacements"] !== undefined
            ? Settings.values["input.replacements"] : [];
        for (var i = 0; i < raw.length; ++i)
            if (String(raw[i]).split("\t")[0] !== from)
                list.push(String(raw[i]));
        list.push(from + "\t" + to);
        Settings.setList("input.replacements", list);
        root.newReplacementFrom = "";
        root.newReplacementTo = "";
    }

    function removeReplacement(from) {
        var list = [];
        var raw = Settings.values["input.replacements"] !== undefined
            ? Settings.values["input.replacements"] : [];
        for (var i = 0; i < raw.length; ++i)
            if (String(raw[i]).split("\t")[0] !== from)
                list.push(String(raw[i]));
        Settings.setList("input.replacements", list);
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
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: String(Settings.writeErrors["input.repeatRate"] || "")
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
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
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: String(Settings.writeErrors["input.repeatDelay"] || "")
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }
                }
            }
        }

        SettingsGroup {
            id: keyboardBrightnessGroup
            width: parent.width
            visible: root.keyboard
            title: qsTr("Keyboard Brightness")

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Adjust keyboard brightness in low light")
                settingsId: "adjustBrightness"
                owner: "settingsd"
                settingsKey: "input.adjustBrightnessLowLight"
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
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: String(Settings.writeErrors["input.keyboardBrightness"] || "")
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Turn keyboard backlight off after inactivity")
                settingsId: "backlightOff"
                owner: "settingsd"
                settingsKey: "input.backlightOffAfter"
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

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Keyboard navigation")
                settingsId: "keyboardNavigation"
                owner: "settingsd"
                settingsKey: "input.keyboardNavigation"
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

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Press Super key to")
                settingsId: "emojiKey"
                owner: "settingsd"
                settingsKey: "input.emojiKeyAction"
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

        // ── Input sources, text replacements, dictation (T229) ─────────────
        SettingsGroup {
            id: inputSourcesGroup
            width: parent.width
            visible: root.keyboard
            title: qsTr("Text Input")

            SettingsRow {
                id: sourcesRow
                objectName: "inputSourcesRow"
                width: parent.width
                label: qsTr("Input sources")
                description: root.inputSources.length > 0
                    ? root.inputSources.join(", ")
                    : (root.hostKeymap.length > 0
                        ? qsTr("Host layout: %1").arg(root.hostKeymap)
                        : qsTr("Follows the host keyboard layout."))
                controlData: Button {
                    objectName: "inputSourcesEdit"
                    text: qsTr("Edit…")
                    onClicked: root.sourcesDialog.show()
                }
            }

            SettingsRow {
                id: replacementsRow
                objectName: "inputReplacementsRow"
                width: parent.width
                label: qsTr("Text replacements")
                description: root.replacementEntries.length === 1
                    ? qsTr("1 replacement")
                    : qsTr("%1 replacements").arg(root.replacementEntries.length)
                controlData: Button {
                    objectName: "inputReplacementsEdit"
                    text: qsTr("Edit…")
                    onClicked: root.replacementsDialog.show()
                }
            }

            SettingsRow {
                objectName: "inputDictationRow"
                width: parent.width
                label: qsTr("Dictation")
                description: root.dictationAvailable
                    ? qsTr("Use a speech-to-text service to dictate text.")
                    : qsTr("Dictation is unavailable because no speech-to-text service is installed.")
                showSeparator: false
                controlData: Toggle {
                    id: dictationToggle
                    objectName: "inputDictationToggle"
                    text: ""
                    enabled: root.dictationAvailable
                    onToggled: (checked) => Settings.set("input.dictation", checked)
                }
            }

            Text {
                id: dictationAbsenceNote
                objectName: "inputDictationAbsence"
                width: parent.width
                visible: !root.dictationAvailable
                text: qsTr("Install nerd-dictation, whisper, or another speech-to-text command to enable dictation.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                bottomPadding: Theme.primitive.spacing.sm
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
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: String(Settings.writeErrors["input.pointerSpeed"] || "")
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Natural scrolling")
                settingsId: "naturalScroll"
                owner: "settingsd"
                settingsKey: "input.naturalScroll"
                description: qsTr("Content moves in the same direction as your fingers.")
                controlData: Toggle {
                    id: naturalScrollToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.naturalScroll", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Left-handed")
                settingsId: "leftHanded"
                owner: "settingsd"
                settingsKey: "input.leftHanded"
                description: qsTr("Swap the primary and secondary buttons.")
                controlData: Toggle {
                    id: leftHandedToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.leftHanded", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Secondary click")
                settingsId: "secondaryClick"
                owner: "settingsd"
                settingsKey: "input.secondaryClick"
                controlData: Select {
                    id: secondaryClickSelect
                    accessibleName: qsTr("Secondary click")
                    model: root.secondaryClickOptions
                    onActivated: (index) => Settings.set(
                                    "input.secondaryClick",
                                    root.secondaryClickOptions[index].value)
                }
            }

            Item {
                width: parent.width
                implicitHeight: scrollSpeedColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: scrollSpeedColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Scroll speed")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: scrollSpeedSlider
                        width: parent.width
                        from: -1.0
                        to: 1.0
                        accessibleName: qsTr("Scroll speed")
                        minLabel: qsTr("Slow")
                        maxLabel: qsTr("Fast")
                        onMoved: (value) => Settings.set("input.scrollSpeed", value)
                    }
                }
            }

            Item {
                width: parent.width
                implicitHeight: doubleClickColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: doubleClickColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Double-click speed")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: doubleClickSlider
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        accessibleName: qsTr("Double-click speed")
                        minLabel: qsTr("Fast")
                        maxLabel: qsTr("Slow")
                        onMoved: (value) => Settings.set("input.doubleClickSpeed",
                                                         Math.round(100 + value * 800))
                    }
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Scroll method")
                settingsId: "scrollMethod"
                owner: "settingsd"
                settingsKey: "input.scrollMethod"
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
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: String(Settings.writeErrors["input.pointerSpeed"] || "")
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.pointClick
                label: qsTr("Tap to click")
                settingsId: "tapToClick"
                owner: "settingsd"
                settingsKey: "input.tapToClick"
                description: qsTr("Tap with one finger.")
                showSeparator: false
                controlData: Toggle {
                    id: tapToClickToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.tapToClick", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.pointClick
                label: qsTr("Secondary click")
                settingsId: "trackpadSecondaryClick"
                owner: "settingsd"
                settingsKey: "input.secondaryClick"
                controlData: Select {
                    id: trackpadSecondarySelect
                    accessibleName: qsTr("Secondary click")
                    model: root.secondaryClickOptions
                    onActivated: (index) => Settings.set(
                                    "input.secondaryClick",
                                    root.secondaryClickOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.pointClick
                label: qsTr("Look up")
                settingsId: "lookUp"
                owner: "settingsd"
                settingsKey: "input.lookUp"
                controlData: Select {
                    id: lookUpSelect
                    accessibleName: qsTr("Look up")
                    model: root.lookUpOptions
                    onActivated: (index) => Settings.set(
                                    "input.lookUp",
                                    root.lookUpOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.pointClick
                label: qsTr("Haptic feedback")
                settingsId: "haptics"
                owner: "settingsd"
                settingsKey: "input.haptics"
                description: root.hasTouchpad
                    ? qsTr("Where the trackpad supports it.")
                    : qsTr("No trackpad is present; the preference is kept for the next one.")
                controlData: Toggle {
                    id: hapticsToggle
                    text: ""
                    enabled: root.hasTouchpad
                    onToggled: (checked) => Settings.set("input.haptics", checked)
                }
            }

            SettingsRow {
                id: pairTrackpadRow
                objectName: "trackpadPairRow"
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Set Up Bluetooth Trackpad…")
                description: Settings.bluetoothAvailable
                    ? qsTr("Pair a trackpad in Bluetooth.")
                    : qsTr("Bluetooth is unavailable on this system.")
                showSeparator: false
                controlData: Button {
                    objectName: "trackpadPairButton"
                    text: qsTr("Open Bluetooth")
                    enabled: Settings.bluetoothAvailable
                    onClicked: root.navigateRequested("bluetooth")
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Natural scrolling")
                settingsId: "trackpadNaturalScroll"
                owner: "settingsd"
                settingsKey: "input.naturalScroll"
                description: qsTr("Content moves in the same direction as your fingers.")
                controlData: Toggle {
                    id: trackpadNaturalToggle
                    text: ""
                    onToggled: (checked) => Settings.set("input.naturalScroll", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Scroll method")
                settingsId: "trackpadScrollMethod"
                owner: "settingsd"
                settingsKey: "input.scrollMethod"
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

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Gestures")
                settingsId: "gestures"
                owner: "settingsd"
                settingsKey: "gestures.enabled"
                description: qsTr("Swipe between workspaces and open Overview.")
                controlData: Toggle {
                    id: gesturesEnabledToggle
                    text: ""
                    onToggled: (checked) => Settings.set("gestures.enabled", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Swipe between Workspaces")
                settingsId: "gestureSpace"
                owner: "settingsd"
                settingsKey: "gestures.spaceSwitch"
                controlData: Toggle {
                    id: gestureSpaceToggle
                    text: ""
                    onToggled: (checked) => Settings.set("gestures.spaceSwitch", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Overview")
                settingsId: "gestureMission"
                owner: "settingsd"
                settingsKey: "gestures.missionControl"
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
        target: dictationToggle
        property: "checked"
        value: Settings.values["input.dictation"] === true
    }
    Binding {
        target: secondaryClickSelect
        property: "currentIndex"
        value: root.secondaryClickIndex(root.secondaryClick)
    }
    Binding {
        target: trackpadSecondarySelect
        property: "currentIndex"
        value: root.secondaryClickIndex(root.secondaryClick)
    }
    Binding {
        target: scrollSpeedSlider
        property: "value"
        value: root.scrollSpeed
    }
    Binding {
        target: doubleClickSlider
        property: "value"
        value: root.doubleClickFraction()
    }
    Binding {
        target: hapticsToggle
        property: "checked"
        value: root.haptics
    }
    Binding {
        target: lookUpSelect
        property: "currentIndex"
        value: root.lookUpIndex()
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

    // ── Input sources dialog ──────────────────────────────────────────────
    Dialog {
        id: sourcesDialog
        objectName: "inputSourcesDialog"
        title: qsTr("Input Sources")
        dismissible: true
        parent: root.popupParent ? root.popupParent : root

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("The first source is active. The compositor applies the layout live.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: sourcesRepeater
                model: root.inputSources
                delegate: Row {
                    required property var modelData
                    required property int index
                    width: parent.width
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        width: parent.width - 80
                        text: modelData
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Button {
                        text: qsTr("Remove")
                        variant: "ghost"
                        onClicked: root.removeSource(modelData)
                    }
                }
            }

            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                TextInput {
                    id: newSourceInput
                    objectName: "inputNewSource"
                    width: parent.width - 90
                    height: 28
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    verticalAlignment: TextInput.AlignVCenter
                    onTextChanged: root.newSource = text
                    Keys.onReturnPressed: root.addSource()
                }

                Button {
                    text: qsTr("Add")
                    enabled: root.newSource.trim().length > 0
                    onClicked: root.addSource()
                }
            }
        }
    }

    // ── Text replacements dialog ──────────────────────────────────────────
    Dialog {
        id: replacementsDialog
        objectName: "inputReplacementsDialog"
        title: qsTr("Text Replacements")
        dismissible: true
        parent: root.popupParent ? root.popupParent : root

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Repeater {
                id: replacementsRepeater
                model: root.replacementEntries
                delegate: Row {
                    required property var modelData
                    width: parent.width
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        width: parent.width - 80
                        text: modelData.from + "  →  " + modelData.to
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Button {
                        text: qsTr("Remove")
                        variant: "ghost"
                        onClicked: root.removeReplacement(modelData.from)
                    }
                }
            }

            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                TextInput {
                    id: replacementFromInput
                    objectName: "inputReplacementFrom"
                    width: (parent.width - 90) / 2
                    height: 28
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    verticalAlignment: TextInput.AlignVCenter
                    onTextChanged: root.newReplacementFrom = text
                }

                TextInput {
                    id: replacementToInput
                    objectName: "inputReplacementTo"
                    width: (parent.width - 90) / 2
                    height: 28
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    verticalAlignment: TextInput.AlignVCenter
                    onTextChanged: root.newReplacementTo = text
                }

                Button {
                    text: qsTr("Add")
                    enabled: root.newReplacementFrom.trim().length > 0
                    onClicked: root.addReplacement()
                }
            }
        }
    }
}