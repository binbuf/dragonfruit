// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Accessibility -> Speech (T291d).
//
// The fourth section of the mockup's Accessibility list. The build has no
// first-party speech synthesis service and no voice model, so the section
// keeps exactly one group — **Live Speech** — with one real owner: the
// speech-dispatcher / espeak-ng delegate the Vision pane's Read & Speak row
// already uses. The sample speaks through `Settings.speakAccessibilityText`,
// which tries `spd-say` then `espeak-ng`; when neither is installed the row
// stays visible and disables only its action with the missing-program reason.
//
// Custom Voice and Vocal Shortcuts have no Linux owner (no voice-cloning model,
// no vocal-shortcut trigger engine). They are `omitted` map rows with a
// reason; they are neither rendered nor search-indexed (ADR 0276, ADR 0297,
// ADR 0301-accessibility-speech).
Item {
    id: root

    // --- External delegate ---------------------------------------------
    readonly property bool speechAvailable: Settings.accessibilitySpeechAvailable

    // Test/capture surface.
    property alias liveSpeechGroup: liveSpeechGroup
    property alias speakSampleRow: speakSampleRow

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Live Speech ─────────────────────────────────────────────────
        SettingsGroup {
            id: liveSpeechGroup
            width: parent.width
            sectionId: "live-speech"
            title: qsTr("Live Speech")

            SettingsRow {
                id: speakSampleRow
                objectName: "accessibilityLiveSpeechRow"
                width: parent.width
                settingsId: "speech.liveSpeech"
                owner: "speech-dispatcher/espeak-ng"
                settingsKey: "spd-say"
                label: qsTr("Speak sample")
                description: root.speechAvailable
                    ? qsTr("Delegated to the session speech engine. A sample "
                           + "phrase proves the delegate is wired; typed text "
                           + "speaks with the engine's own voice settings.")
                    : qsTr("Not available: neither spd-say nor espeak-ng is "
                           + "installed.")
                showSeparator: false
                trailingAction: root.speechAvailable
                    ? qsTr("Speak Sample") : qsTr("Unavailable")
                trailingActionEnabled: root.speechAvailable
                onTrailingActionClicked:
                    Settings.speakAccessibilityText(qsTr("Speech is ready."))
            }
        }
    }
}
