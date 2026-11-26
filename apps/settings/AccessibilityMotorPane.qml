// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Accessibility -> Physical and Motor (T291b).
//
// The second section of the mockup's Accessibility list. It keeps only rows
// with a real Linux owner: the stored keyboard-navigation preference, the
// on-screen-keyboard delegate, and the compositor's live pointer
// double-click interval. The reference-only rows (Voice Control, Sticky Keys, Slow
// Keys, Mouse Keys, Ignore built-in trackpad, Switch Control) are `omitted`
// map rows with a reason; they are neither rendered nor search-indexed
// (ADR 0276, ADR 0297).
//
// A present-but-unavailable delegate stays a row: the on-screen keyboard
// action is disabled with the missing-program reason, never hidden.
Item {
    id: root

    // --- Durable preferences (settingsd) --------------------------------
    readonly property bool keyboardNavigation:
        Settings.values["input.keyboardNavigation"] === true
    readonly property int doubleClickSpeed: {
        var value = Number(Settings.values["input.doubleClickSpeed"]);
        return isFinite(value) ? Math.round(value) : 400;
    }

    // --- External delegates ---------------------------------------------
    readonly property bool onScreenKeyboardAvailable:
        Settings.accessibilityOnScreenKeyboardAvailable

    // Test/capture surface.
    property alias keyboardGroup: keyboardGroup
    property alias pointerGroup: pointerGroup
    property alias keyboardNavigationToggle: keyboardNavigationToggle
    property alias onScreenKeyboardRow: onScreenKeyboardRow
    property alias doubleClickSpeedSlider: doubleClickSpeedSlider

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Keyboard ────────────────────────────────────────────────────
        SettingsGroup {
            id: keyboardGroup
            width: parent.width
            sectionId: "keyboard"
            title: qsTr("Keyboard")

            SettingsKeyRow {
                width: parent.width
                settingsId: "motor.keyboard.fullKeyboardAccess"
                owner: "settingsd"
                settingsKey: "input.keyboardNavigation"
                label: qsTr("Full keyboard access")
                description: qsTr("Stored-only: the compositor does not read "
                                  + "this preference yet. Tab and Shift Tab move "
                                  + "focus between controls.")
                controlData: Toggle {
                    id: keyboardNavigationToggle
                    objectName: "accessibilityKeyboardNavigationToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("input.keyboardNavigation", checked)
                }
            }

            SettingsRow {
                id: onScreenKeyboardRow
                objectName: "accessibilityOnScreenKeyboardRow"
                width: parent.width
                settingsId: "motor.keyboard.onScreenKeyboard"
                owner: "on-screen keyboard (onboard/wvkbd)"
                settingsKey: "onboard"
                label: qsTr("On-screen keyboard")
                description: root.onScreenKeyboardAvailable
                    ? qsTr("Delegated to the session on-screen keyboard. "
                           + "It types into the focused window.")
                    : qsTr("Not available: neither onboard nor wvkbd is "
                           + "installed.")
                showSeparator: false
                trailingAction: root.onScreenKeyboardAvailable
                    ? qsTr("Open") : qsTr("Not installed")
                trailingActionEnabled: root.onScreenKeyboardAvailable
                onTrailingActionClicked: Settings.openAccessibilityKeyboard()
            }
        }

        // ── Pointer Control ─────────────────────────────────────────────
        SettingsGroup {
            id: pointerGroup
            width: parent.width
            sectionId: "pointer-control"
            title: qsTr("Pointer Control")

            SettingsKeyRow {
                width: parent.width
                settingsId: "motor.pointer.doubleClickSpeed"
                owner: "compositor/input pointer"
                settingsKey: "input.doubleClickSpeed"
                label: qsTr("Double-click speed")
                description: qsTr("Applied live by the compositor pointer. The "
                                  + "Mouse and Trackpad panes own their "
                                  + "per-device values.")
                showSeparator: false
                controlData: Slider {
                    id: doubleClickSpeedSlider
                    objectName: "accessibilityDoubleClickSpeedSlider"
                    from: 100
                    to: 900
                    stepSize: 25
                    minLabel: qsTr("Fast")
                    maxLabel: qsTr("Slow")
                    onCommitted: (value) =>
                        Settings.set("input.doubleClickSpeed", Math.round(value))
                }
            }
        }
    }

    // Keep the controls in step with external changes.
    Binding { target: keyboardNavigationToggle; property: "checked"
              value: root.keyboardNavigation }
    Binding { target: doubleClickSpeedSlider; property: "value"
              value: root.doubleClickSpeed }
}
