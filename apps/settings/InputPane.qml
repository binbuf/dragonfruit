// SPDX-License-Identifier: GPL-3.0-only
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
    // T280: the host exposes a keyboard backlight LED. When false the three
    // brightness rows stay visible, disabled, and explain why; their stored
    // preference is kept for the next keyboard that has one.
    readonly property bool backlightAvailable: view.backlightAvailable === true

    // The settingsd-backed preference values.
    readonly property int repeatRate: Settings.values["input.repeatRate"] !== undefined
        ? Number(Settings.values["input.repeatRate"]) : 25
    readonly property int repeatDelay: Settings.values["input.repeatDelay"] !== undefined
        ? Number(Settings.values["input.repeatDelay"]) : 200
    // Legacy pointer values remain the fallback for an upgrade from a schema
    // that predates the per-device families. New writes always target the
    // mouse.* or trackpad.* key below; a shared write is a bug (T280).
    readonly property real legacyPointerSpeed:
        Settings.values["input.pointerSpeed"] !== undefined
            ? Number(Settings.values["input.pointerSpeed"]) : 0.0
    readonly property real mousePointerSpeed:
        Settings.values["input.mouse.pointerSpeed"] !== undefined
            ? Number(Settings.values["input.mouse.pointerSpeed"])
            : root.legacyPointerSpeed
    readonly property real trackpadPointerSpeed:
        Settings.values["input.trackpad.pointerSpeed"] !== undefined
            ? Number(Settings.values["input.trackpad.pointerSpeed"])
            : root.legacyPointerSpeed
    readonly property bool mouseNaturalScroll:
        Settings.values["input.mouse.naturalScroll"] !== undefined
            ? Settings.values["input.mouse.naturalScroll"] === true
            : Settings.values["input.naturalScroll"] === true
    readonly property bool trackpadNaturalScroll:
        Settings.values["input.trackpad.naturalScroll"] !== undefined
            ? Settings.values["input.trackpad.naturalScroll"] === true
            : Settings.values["input.naturalScroll"] === true
    // T320: secondary click, scroll speed, and double-click have no
    // per-device libinput setter (the compositor's button/axis path cannot
    // attribute an event to a device), so both pointer panes write the shared
    // `input.*` policy. The per-family keys remain stored-only.
    readonly property string mouseSecondaryClick:
        Settings.values["input.secondaryClick"] || "right"
    readonly property string trackpadSecondaryClick:
        Settings.values["input.secondaryClick"] || "right"
    readonly property bool mouseSmartZoom:
        Settings.values["input.mouse.smartZoom"] !== false
    readonly property bool mousePointerAcceleration:
        Settings.values["input.mouse.pointerAcceleration"] !== false
    readonly property bool mouseLeftHanded:
        Settings.values["input.mouse.leftHanded"] !== undefined
            ? Settings.values["input.mouse.leftHanded"] === true
            : Settings.values["input.leftHanded"] === true
    readonly property real mouseScrollSpeed:
        Settings.values["input.scrollSpeed"] !== undefined
            ? Number(Settings.values["input.scrollSpeed"])
            : root.scrollSpeed
    readonly property string mouseScrollMethod:
        Settings.values["input.mouse.scrollMethod"]
            || Settings.values["input.scrollMethod"] || "button"
    readonly property int mouseDoubleClickSpeed:
        Settings.values["input.doubleClickSpeed"] !== undefined
            ? Number(Settings.values["input.doubleClickSpeed"])
            : root.doubleClickSpeed
    readonly property string trackpadLookUp:
        Settings.values["input.lookUp"] || "force-click"
    readonly property real trackpadClickPressure:
        Settings.values["input.trackpad.clickPressure"] !== undefined
            ? Number(Settings.values["input.trackpad.clickPressure"]) : 0.5
    readonly property bool trackpadSmartZoom:
        Settings.values["input.trackpad.smartZoom"] !== false
    readonly property bool trackpadPinchZoom:
        Settings.values["input.trackpad.pinchZoom"] !== false
    readonly property bool trackpadRotate:
        Settings.values["input.trackpad.rotate"] !== false
    readonly property string gesturePages:
        Settings.values["gestures.pages"] || "two-finger"
    readonly property string gestureFullScreen:
        Settings.values["gestures.fullscreen"] || "three-finger"
    readonly property bool gestureNotifications:
        Settings.values["gestures.notifications"] === true
    readonly property bool gestureShowDesktop:
        Settings.values["gestures.showDesktop"] !== false
    readonly property bool gestureAppWindows:
        Settings.values["gestures.appWindows"] !== false
    readonly property bool tapToClick: Settings.values["input.tapToClick"] === true
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
    readonly property var gesturePagesOptions: [
        { value: "off", label: qsTr("Off") },
        { value: "two-finger", label: qsTr("Swipe with Two Fingers") },
        { value: "three-finger", label: qsTr("Swipe with Three Fingers") }
    ]
    readonly property var gestureFullScreenOptions: [
        { value: "off", label: qsTr("Off") },
        { value: "three-finger", label: qsTr("Swipe with Three Fingers") },
        { value: "four-finger", label: qsTr("Swipe with Four Fingers") }
    ]
    // T280: the read-only shortcut catalog the shell's accelerator table
    // exposes. Editing is delegated to the shortcut engine (T245a); the sheet
    // is a real inspector, not a fake recorder.
    readonly property var shortcutRows: [
        { keys: "Super", action: qsTr("Open the application menu") },
        { keys: "Super+Tab", action: qsTr("Switch applications") },
        { keys: "Super+`", action: qsTr("Cycle this app's windows") },
        { keys: "Super+Shift+3", action: qsTr("Screenshot the screen") },
        { keys: "Super+Shift+4", action: qsTr("Screenshot a selection") },
        { keys: "Super+Q", action: qsTr("Close the focused window") },
        { keys: "Super+M", action: qsTr("Minimize the focused window") },
        { keys: "Super+Space", action: qsTr("Open search") }
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
    readonly property string lookUp: root.trackpadLookUp
    property string newSource: ""
    property string newReplacementFrom: ""
    property string newReplacementTo: ""

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
    property alias inputMethodRow: inputMethodRow
    property alias composeKeyRow: composeKeyRow
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
    // T280 surfaces.
    property alias shortcutsRow: shortcutsRow
    property alias shortcutsDialog: shortcutsDialog
    property alias dictationOptionsButton: dictationOptionsButton
    property alias dictationOptionsDialog: dictationOptionsDialog
    property alias dictationLanguageInput: dictationLanguageInput
    property alias dictationMicrophoneInput: dictationMicrophoneInput
    property alias dictationShortcutInput: dictationShortcutInput
    property alias dictationAutoPunctuationToggle: dictationAutoPunctuationToggle
    property alias mouseSmartZoomToggle: mouseSmartZoomToggle
    property alias pointerAccelerationToggle: pointerAccelerationToggle
    property alias mouseGesturePagesSelect: mouseGesturePagesSelect
    property alias mouseGestureFullScreenSelect: mouseGestureFullScreenSelect
    property alias mouseGestureNotificationsToggle: mouseGestureNotificationsToggle
    property alias mouseGestureShowDesktopToggle: mouseGestureShowDesktopToggle
    property alias mouseGestureAppWindowsToggle: mouseGestureAppWindowsToggle
    property alias trackpadNaturalToggle: trackpadNaturalToggle
    property alias trackpadClickPressureSlider: trackpadClickPressureSlider
    property alias trackpadSmartZoomToggle: trackpadSmartZoomToggle
    property alias pinchZoomToggle: pinchZoomToggle
    property alias rotateToggle: rotateToggle
    property alias gesturePagesSelect: gesturePagesSelect
    property alias gestureFullScreenSelect: gestureFullScreenSelect
    property alias gestureNotificationsToggle: gestureNotificationsToggle
    property alias gestureShowDesktopToggle: gestureShowDesktopToggle
    property alias gestureAppWindowsToggle: gestureAppWindowsToggle
    property alias backlightAbsenceNote: backlightAbsenceNote
    property alias previewCaption: previewCaption

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

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

    function scrollMethodIndexFor(value) {
        for (var i = 0; i < root.scrollMethodOptions.length; ++i) {
            if (root.scrollMethodOptions[i].value === value)
                return i;
        }
        return 0;
    }

    function gesturePagesIndex() {
        for (var i = 0; i < root.gesturePagesOptions.length; ++i)
            if (root.gesturePagesOptions[i].value === root.gesturePages)
                return i;
        return 0;
    }

    function gestureFullScreenIndex() {
        for (var i = 0; i < root.gestureFullScreenOptions.length; ++i)
            if (root.gestureFullScreenOptions[i].value === root.gestureFullScreen)
                return i;
        return 0;
    }

    function mouseDoubleClickFraction() {
        return (root.mouseDoubleClickSpeed - 100) / 800.0;
    }

    function clickPressurePercent() {
        return Math.round(root.trackpadClickPressure * 100);
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

    // The first source is the compositor's active layout; reordering is a
    // write of the same ordered list, never a second index.
    function moveSource(value, delta) {
        var list = root.inputSources.slice();
        var at = list.indexOf(value);
        var to = at + delta;
        if (at < 0 || to < 0 || to >= list.length)
            return;
        list.splice(to, 0, list.splice(at, 1)[0]);
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
                        onCommitted: (value) => Settings.set("input.repeatRate",
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
                        onCommitted: (value) => Settings.set("input.repeatDelay",
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

            Text {
                id: backlightAbsenceNote
                objectName: "inputBacklightAbsence"
                width: parent.width
                visible: root.ready && !root.backlightAvailable
                text: qsTr("No keyboard backlight is present. The three "
                           + "brightness preferences are kept for the next "
                           + "keyboard that has one.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
            }

            Text {
                objectName: "inputBacklightStoredOnly"
                width: parent.width
                text: qsTr("Stored-only: the compositor keyboard-backlight "
                           + "bridge is deferred (T346); these three "
                           + "preferences are kept for the next keyboard.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Adjust keyboard brightness in low light")
                settingsId: "adjustBrightness"
                owner: "settingsd"
                settingsKey: "input.adjustBrightnessLowLight"
                description: qsTr("Stored-only: the compositor keyboard-backlight "
                                  + "bridge is not implemented in this build.")
                controlData: Toggle {
                    id: adjustBrightnessToggle
                    text: ""
                    enabled: root.backlightAvailable
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
                        enabled: root.backlightAvailable
                        accessibleName: qsTr("Keyboard brightness")
                        minLabel: qsTr("Dim")
                        maxLabel: qsTr("Bright")
                        onCommitted: (value) =>
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
                description: qsTr("Stored-only: the compositor keyboard-backlight "
                                  + "bridge is deferred (T346).")
                controlData: Select {
                    id: backlightOffSelect
                    enabled: root.backlightAvailable
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
                description: qsTr("Stored-only: no compositor consumer reads "
                                  + "this key yet. Press Tab to move focus forward "
                                  + "and Shift Tab to move it backward.")
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
                description: qsTr("Stored-only: the emoji-panel bridge is not "
                                  + "implemented in this build.")
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
                id: shortcutsRow
                objectName: "inputShortcutsRow"
                width: parent.width
                label: qsTr("Keyboard shortcuts")
                description: qsTr("Inspect the shell's global accelerators. "
                                  + "Editing is delegated to the shortcut engine.")
                controlData: Button {
                    objectName: "inputShortcutsOpen"
                    text: qsTr("Shortcuts…")
                    onClicked: root.shortcutsDialog.show()
                }
            }

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

            // T281: the input-method engine and the XKB compose key stay with
            // their Linux owners. The rows are honest hand-offs, not fake
            // toggles; Settings does not own either configuration.
            SettingsRow {
                id: inputMethodRow
                objectName: "inputMethodRow"
                width: parent.width
                label: qsTr("Input method")
                description: qsTr("Engines and key bindings are configured in IBus "
                                  + "or Fcitx5; Settings does not edit the "
                                  + "input-method framework.")
                settingsId: "inputMethod"
                owner: "ibus/fcitx5"
                settingsKey: "input.method"
                enabled: false

                controlData: Button {
                    text: qsTr("Open…")
                    enabled: false
                }
            }

            SettingsRow {
                id: composeKeyRow
                objectName: "inputComposeKeyRow"
                width: parent.width
                label: qsTr("Compose key")
                description: qsTr("The compose key is a keyboard layout option; "
                                  + "Dragonfruit does not edit it yet.")
                settingsId: "composeKey"
                owner: "compositor/input"
                settingsKey: "input.composeKey"
                enabled: false

                controlData: Select {
                    model: [qsTr("Off")]
                    enabled: false
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
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Toggle {
                        id: dictationToggle
                        objectName: "inputDictationToggle"
                        text: ""
                        enabled: root.dictationAvailable
                        onToggled: (checked) => Settings.set("input.dictation", checked)
                    }

                    Button {
                        id: dictationOptionsButton
                        objectName: "inputDictationOptions"
                        text: qsTr("Options…")
                        variant: "ghost"
                        enabled: root.dictationAvailable
                        onClicked: root.dictationOptionsDialog.show()
                    }
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
                        accessibleName: qsTr("Mouse tracking speed")
                        minLabel: qsTr("Slow")
                        maxLabel: qsTr("Fast")
                        onCommitted: (value) =>
                            Settings.set("input.mouse.pointerSpeed", value)
                    }
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: String(Settings.writeErrors["input.mouse.pointerSpeed"] || "")
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Natural scrolling")
                settingsId: "mouseNaturalScroll"
                owner: "settingsd"
                settingsKey: "input.mouse.naturalScroll"
                description: qsTr("Content moves in the same direction as the wheel.")
                controlData: Toggle {
                    id: naturalScrollToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.mouse.naturalScroll", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Pointer acceleration")
                settingsId: "mousePointerAcceleration"
                owner: "settingsd"
                settingsKey: "input.mouse.pointerAcceleration"
                description: qsTr("Apply pointer acceleration to mouse motion; "
                                  + "off uses a flat curve.")
                controlData: Toggle {
                    id: pointerAccelerationToggle
                    objectName: "mousePointerAcceleration"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.mouse.pointerAcceleration", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Smart zoom")
                settingsId: "mouseSmartZoom"
                owner: "settingsd"
                settingsKey: "input.mouse.smartZoom"
                description: qsTr("Double-click with one finger to zoom the "
                                  + "content under the pointer.")
                controlData: Toggle {
                    id: mouseSmartZoomToggle
                    objectName: "mouseSmartZoom"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.mouse.smartZoom", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Left-handed")
                settingsId: "mouseLeftHanded"
                owner: "settingsd"
                settingsKey: "input.mouse.leftHanded"
                description: qsTr("Swap the mouse's primary and secondary buttons.")
                controlData: Toggle {
                    id: leftHandedToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.mouse.leftHanded", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Secondary click")
                settingsId: "mouseSecondaryClick"
                owner: "settingsd"
                settingsKey: "input.secondaryClick"
                controlData: Select {
                    id: secondaryClickSelect
                    accessibleName: qsTr("Mouse secondary click")
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
                        accessibleName: qsTr("Mouse scroll speed")
                        minLabel: qsTr("Slow")
                        maxLabel: qsTr("Fast")
                        onCommitted: (value) =>
                            Settings.set("input.scrollSpeed", value)
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
                        accessibleName: qsTr("Mouse double-click speed")
                        minLabel: qsTr("Fast")
                        maxLabel: qsTr("Slow")
                        onCommitted: (value) => Settings.set(
                                        "input.doubleClickSpeed",
                                        Math.round(100 + value * 800))
                    }
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Scroll method")
                settingsId: "mouseScrollMethod"
                owner: "settingsd"
                settingsKey: "input.mouse.scrollMethod"
                showSeparator: true
                controlData: Select {
                    id: scrollMethodSelect
                    accessibleName: qsTr("Mouse scroll method")
                    model: root.scrollMethodOptions
                    onActivated: (index) => Settings.set(
                                    "input.mouse.scrollMethod",
                                    root.scrollMethodOptions[index].value)
                }
            }

            // ── More gestures (the same families as the trackpad) ────────
            SettingsKeyRow {
                width: parent.width
                label: qsTr("Swipe between pages")
                settingsId: "mouseGesturePages"
                owner: "settingsd"
                settingsKey: "gestures.pages"
                description: qsTr("Stored-only: the compositor gesture "
                                  + "recognizer does not read this family yet.")
                controlData: Select {
                    id: mouseGesturePagesSelect
                    accessibleName: qsTr("Swipe between pages")
                    model: root.gesturePagesOptions
                    onActivated: (index) => Settings.set(
                                    "gestures.pages",
                                    root.gesturePagesOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Swipe between full-screen apps")
                settingsId: "mouseGestureFullScreen"
                owner: "settingsd"
                settingsKey: "gestures.fullscreen"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                controlData: Select {
                    id: mouseGestureFullScreenSelect
                    accessibleName: qsTr("Swipe between full-screen apps")
                    model: root.gestureFullScreenOptions
                    onActivated: (index) => Settings.set(
                                    "gestures.fullscreen",
                                    root.gestureFullScreenOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Notifications")
                settingsId: "mouseGestureNotifications"
                owner: "settingsd"
                settingsKey: "gestures.notifications"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                controlData: Toggle {
                    id: mouseGestureNotificationsToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.notifications", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("App windows")
                settingsId: "mouseGestureAppWindows"
                owner: "settingsd"
                settingsKey: "gestures.appWindows"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                controlData: Toggle {
                    id: mouseGestureAppWindowsToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.appWindows", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Show desktop")
                settingsId: "mouseGestureShowDesktop"
                owner: "settingsd"
                settingsKey: "gestures.showDesktop"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                showSeparator: false
                controlData: Toggle {
                    id: mouseGestureShowDesktopToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.showDesktop", checked)
                }
            }
        }

        // ── Trackpad ──────────────────────────────────────────────────────
        SettingsGroup {
            id: trackpadGroup
            width: parent.width
            visible: root.trackpad
            title: qsTr("Trackpad")

            // An original outline preview with a finger dot and the active
            // gesture family's caption. It is decorative, never an input
            // surface, and mirrors the segmented selection.
            Item {
                width: parent.width
                height: 104

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: Theme.primitive.spacing.sm
                    width: 148
                    height: 76
                    radius: Theme.primitive.radius.md
                    color: Theme.color.surfaceSunken
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        anchors.bottom: parent.bottom
                        anchors.bottomMargin: 10
                        width: 16
                        height: 16
                        radius: 8
                        color: Theme.color.accent
                    }

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        anchors.top: parent.top
                        anchors.topMargin: 10
                        width: 36
                        height: 3
                        radius: 1.5
                        color: Theme.color.textTertiary
                    }
                }

                Text {
                    id: previewCaption
                    objectName: "trackpadPreviewCaption"
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    text: root.pointClick ? qsTr("Point & Click")
                        : (root.scrollZoom ? qsTr("Scroll & Zoom")
                                           : qsTr("More Gestures"))
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
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
                        accessibleName: qsTr("Trackpad tracking speed")
                        minLabel: qsTr("Slow")
                        maxLabel: qsTr("Fast")
                        onCommitted: (value) =>
                            Settings.set("input.trackpad.pointerSpeed", value)
                    }
                    Text {
                        width: parent.width
                        visible: text.length > 0
                        text: String(Settings.writeErrors["input.trackpad.pointerSpeed"] || "")
                        color: Theme.color.danger
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }
                }
            }

            Item {
                width: parent.width
                visible: root.pointClick
                implicitHeight: clickPressureColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: clickPressureColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Click pressure (%1%)").arg(root.clickPressurePercent())
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: trackpadClickPressureSlider
                        objectName: "trackpadClickPressure"
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        accessibleName: qsTr("Click pressure")
                        minLabel: qsTr("Light")
                        maxLabel: qsTr("Firm")
                        onCommitted: (value) =>
                            Settings.set("input.trackpad.clickPressure", value)
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
                    accessibleName: qsTr("Trackpad secondary click")
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
                showSeparator: false
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
                    ? qsTr("Stored-only: no haptic control is exposed by the "
                           + "compositor's libinput stack in this build.")
                    : qsTr("Stored-only: no trackpad is present; the preference "
                           + "is kept for the next one.")
                showSeparator: false
                controlData: Toggle {
                    id: hapticsToggle
                    text: ""
                    enabled: root.hasTouchpad
                    onToggled: (checked) => Settings.set("input.haptics", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Natural scrolling")
                settingsId: "trackpadNaturalScroll"
                owner: "settingsd"
                settingsKey: "input.trackpad.naturalScroll"
                description: qsTr("Content moves in the same direction as your fingers.")
                controlData: Toggle {
                    id: trackpadNaturalToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.trackpad.naturalScroll", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Pinch to zoom")
                settingsId: "pinchZoom"
                owner: "settingsd"
                settingsKey: "input.trackpad.pinchZoom"
                controlData: Toggle {
                    id: pinchZoomToggle
                    objectName: "trackpadPinchZoom"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.trackpad.pinchZoom", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Smart zoom")
                settingsId: "trackpadSmartZoom"
                owner: "settingsd"
                settingsKey: "input.trackpad.smartZoom"
                controlData: Toggle {
                    id: trackpadSmartZoomToggle
                    objectName: "trackpadSmartZoom"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.trackpad.smartZoom", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.scrollZoom
                label: qsTr("Rotate")
                settingsId: "rotate"
                owner: "settingsd"
                settingsKey: "input.trackpad.rotate"
                showSeparator: false
                controlData: Toggle {
                    id: rotateToggle
                    objectName: "trackpadRotate"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.trackpad.rotate", checked)
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
                label: qsTr("Swipe between pages")
                settingsId: "gesturePages"
                owner: "settingsd"
                settingsKey: "gestures.pages"
                description: qsTr("Stored-only: the compositor gesture "
                                  + "recognizer does not read this family yet.")
                controlData: Select {
                    id: gesturePagesSelect
                    accessibleName: qsTr("Swipe between pages")
                    model: root.gesturePagesOptions
                    onActivated: (index) => Settings.set(
                                    "gestures.pages",
                                    root.gesturePagesOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Swipe between full-screen apps")
                settingsId: "gestureFullScreen"
                owner: "settingsd"
                settingsKey: "gestures.fullscreen"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                controlData: Select {
                    id: gestureFullScreenSelect
                    accessibleName: qsTr("Swipe between full-screen apps")
                    model: root.gestureFullScreenOptions
                    onActivated: (index) => Settings.set(
                                    "gestures.fullscreen",
                                    root.gestureFullScreenOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Notifications")
                settingsId: "gestureNotifications"
                owner: "settingsd"
                settingsKey: "gestures.notifications"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                controlData: Toggle {
                    id: gestureNotificationsToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.notifications", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Overview")
                settingsId: "gestureMission"
                owner: "settingsd"
                settingsKey: "gestures.missionControl"
                controlData: Toggle {
                    id: gestureMissionToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.missionControl", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("App windows")
                settingsId: "gestureAppWindows"
                owner: "settingsd"
                settingsKey: "gestures.appWindows"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                controlData: Toggle {
                    id: gestureAppWindowsToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.appWindows", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                visible: root.moreGestures
                label: qsTr("Show desktop")
                settingsId: "gestureShowDesktop"
                owner: "settingsd"
                settingsKey: "gestures.showDesktop"
                description: qsTr("Stored-only until the compositor gesture "
                                  + "recognizer reads this family.")
                controlData: Toggle {
                    id: gestureShowDesktopToggle
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("gestures.showDesktop", checked)
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
        target: dictationAutoPunctuationToggle
        property: "checked"
        value: Settings.values["input.dictationAutoPunctuation"] === true
    }
    Binding {
        target: secondaryClickSelect
        property: "currentIndex"
        value: root.secondaryClickIndex(root.mouseSecondaryClick)
    }
    Binding {
        target: trackpadSecondarySelect
        property: "currentIndex"
        value: root.secondaryClickIndex(root.trackpadSecondaryClick)
    }
    Binding {
        target: scrollSpeedSlider
        property: "value"
        value: root.mouseScrollSpeed
    }
    Binding {
        target: doubleClickSlider
        property: "value"
        value: root.mouseDoubleClickFraction()
    }
    Binding {
        target: mouseSmartZoomToggle
        property: "checked"
        value: root.mouseSmartZoom
    }
    Binding {
        target: pointerAccelerationToggle
        property: "checked"
        value: root.mousePointerAcceleration
    }
    Binding {
        target: mouseGesturePagesSelect
        property: "currentIndex"
        value: root.gesturePagesIndex()
    }
    Binding {
        target: mouseGestureFullScreenSelect
        property: "currentIndex"
        value: root.gestureFullScreenIndex()
    }
    Binding {
        target: mouseGestureNotificationsToggle
        property: "checked"
        value: root.gestureNotifications
    }
    Binding {
        target: mouseGestureAppWindowsToggle
        property: "checked"
        value: root.gestureAppWindows
    }
    Binding {
        target: mouseGestureShowDesktopToggle
        property: "checked"
        value: root.gestureShowDesktop
    }
    Binding {
        target: trackpadClickPressureSlider
        property: "value"
        value: root.trackpadClickPressure
    }
    Binding {
        target: pinchZoomToggle
        property: "checked"
        value: root.trackpadPinchZoom
    }
    Binding {
        target: trackpadSmartZoomToggle
        property: "checked"
        value: root.trackpadSmartZoom
    }
    Binding {
        target: rotateToggle
        property: "checked"
        value: root.trackpadRotate
    }
    Binding {
        target: gesturePagesSelect
        property: "currentIndex"
        value: root.gesturePagesIndex()
    }
    Binding {
        target: gestureFullScreenSelect
        property: "currentIndex"
        value: root.gestureFullScreenIndex()
    }
    Binding {
        target: gestureNotificationsToggle
        property: "checked"
        value: root.gestureNotifications
    }
    Binding {
        target: gestureAppWindowsToggle
        property: "checked"
        value: root.gestureAppWindows
    }
    Binding {
        target: gestureShowDesktopToggle
        property: "checked"
        value: root.gestureShowDesktop
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
        value: root.mousePointerSpeed
    }
    Binding {
        target: naturalScrollToggle
        property: "checked"
        value: root.mouseNaturalScroll
    }
    Binding {
        target: leftHandedToggle
        property: "checked"
        value: root.mouseLeftHanded
    }
    Binding {
        target: scrollMethodSelect
        property: "currentIndex"
        value: root.scrollMethodIndexFor(root.mouseScrollMethod)
    }
    Binding {
        target: trackpadSpeedSlider
        property: "value"
        value: root.trackpadPointerSpeed
    }
    Binding {
        target: tapToClickToggle
        property: "checked"
        value: root.tapToClick
    }
    Binding {
        target: trackpadNaturalToggle
        property: "checked"
        value: root.trackpadNaturalScroll
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
                        width: parent.width - 180
                        text: index === 0
                            ? qsTr("%1 (active)").arg(modelData) : modelData
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Button {
                        objectName: "inputSourceUp"
                        text: qsTr("Up")
                        variant: "ghost"
                        enabled: index > 0
                        onClicked: root.moveSource(modelData, -1)
                    }

                    Button {
                        objectName: "inputSourceDown"
                        text: qsTr("Down")
                        variant: "ghost"
                        enabled: index < root.inputSources.length - 1
                        onClicked: root.moveSource(modelData, 1)
                    }

                    Button {
                        objectName: "inputSourceRemove"
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

    // ── Keyboard shortcuts inspector (T280) ───────────────────────────────
    Dialog {
        id: shortcutsDialog
        objectName: "inputShortcutsDialog"
        title: qsTr("Keyboard Shortcuts")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("These accelerators come from the shell's one "
                           + "shortcut table. Editing is delegated to the "
                           + "shortcut engine.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: shortcutsRepeater
                model: root.shortcutRows
                delegate: SettingsRow {
                    required property var modelData
                    width: parent.width
                    label: String(modelData.action)
                    controlData: Text {
                        text: String(modelData.keys)
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Close")
                variant: "primary"
                onClicked: shortcutsDialog.accept()
            }
        }
    }

    // ── Dictation options (T280) ──────────────────────────────────────────
    Dialog {
        id: dictationOptionsDialog
        objectName: "inputDictationOptionsDialog"
        title: qsTr("Dictation Options")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                text: qsTr("Language (BCP-47; empty follows the locale)")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: dictationLanguageInput
                objectName: "inputDictationLanguage"
                width: parent.width
                height: 28
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                verticalAlignment: TextInput.AlignVCenter
                text: Settings.values["input.dictationLanguage"] || ""
                onEditingFinished: Settings.set("input.dictationLanguage", text)
            }

            Text {
                text: qsTr("Microphone (empty uses the default source)")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: dictationMicrophoneInput
                objectName: "inputDictationMicrophone"
                width: parent.width
                height: 28
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                verticalAlignment: TextInput.AlignVCenter
                text: Settings.values["input.dictationMicrophone"] || ""
                onEditingFinished: Settings.set("input.dictationMicrophone", text)
            }

            Text {
                text: qsTr("Shortcut (empty uses the delegate default)")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: dictationShortcutInput
                objectName: "inputDictationShortcut"
                width: parent.width
                height: 28
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                verticalAlignment: TextInput.AlignVCenter
                text: Settings.values["input.dictationShortcut"] || ""
                onEditingFinished: Settings.set("input.dictationShortcut", text)
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Automatic punctuation")
                controlData: Toggle {
                    id: dictationAutoPunctuationToggle
                    objectName: "inputDictationAutoPunctuation"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.dictationAutoPunctuation", checked)
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Close")
                variant: "primary"
                onClicked: dictationOptionsDialog.accept()
            }
        }
    }

}