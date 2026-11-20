// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Accessibility -> Hearing (T291c).
//
// The third section of the mockup's Accessibility list. Only the **Audio**
// group has a real Linux owner: the durable `sound.balance` preference, the
// same key the Sound pane writes. The remaining mockup groups have no owner and
// stay `omitted` map rows with a reason: Hearing Devices (no hearing-aid or LE
// Audio service), Subtitles and Captioning (no caption delegate), Live Captions
// (no live-caption engine), and Spoken Name (no on-device name model). Omitted
// rows are neither rendered nor search-indexed (ADR 0276, ADR 0297, ADR 0299).
//
// Mono audio, Flash the screen on alert, and Background sounds belong to the
// mockup's Audio group but have no key or service in this build; they are
// omissions, not disabled placeholders.
Item {
    id: root

    // --- Durable preference (settingsd) ---------------------------------
    readonly property real balance:
        Settings.values["sound.balance"] !== undefined
            ? Number(Settings.values["sound.balance"]) : 0.5

    // Test/capture surface.
    property alias audioGroup: audioGroup
    property alias balanceSlider: balanceSlider

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Audio ────────────────────────────────────────────────────────
        SettingsGroup {
            id: audioGroup
            width: parent.width
            sectionId: "audio"
            title: qsTr("Audio")

            SettingsKeyRow {
                width: parent.width
                settingsId: "hearing.balance"
                owner: "apps/settings"
                settingsKey: "sound.balance"
                label: qsTr("Balance")
                description: qsTr("Stored-only: the audio engine has no balance "
                                  + "consumer yet. The Sound pane writes the "
                                  + "same key.")
                showSeparator: false
                controlData: Slider {
                    id: balanceSlider
                    objectName: "accessibilityHearingBalanceSlider"
                    from: 0.0
                    to: 1.0
                    stepSize: 0.05
                    minLabel: qsTr("Left")
                    maxLabel: qsTr("Right")
                    accessibleName: qsTr("Balance")
                    onCommitted: (value) => Settings.set("sound.balance", value)
                }
            }
        }
    }

    // Keep the control in step with external changes.
    Binding { target: balanceSlider; property: "value"; value: root.balance }
}
