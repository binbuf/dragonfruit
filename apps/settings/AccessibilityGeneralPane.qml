// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Accessibility -> General (T291e).
//
// The last section of the mockup's Accessibility list. It ships exactly one
// group, **Accessibility Shortcut**, with one real owner: the recorded chord in
// the durable `accessibility.shortcut` key. The compositor's shortcut engine
// installs the binding (the `accessibility-shortcut` shell action opens this
// pane), so the row is live, not stored-only.
//
// The row reuses the existing `ShortcutRecorder` (T275): it is the one chord
// editor in Settings (the Search pane's shortcut is a separate key, not a
// second recorder). The recorder builds the canonical `Modifier+Key` spelling
// `parse_binding` accepts; Backspace clears the chord and disables the shortcut.
// There is no absence state beyond the host's own: the recorder always works
// because the key is a settingsd preference. The Accessibility pane needs no
// bridge host for this row (ADR 0276, ADR 0297, ADR 0302).
Item {
    id: root

    // --- Durable preference (settingsd) ---------------------------------
    readonly property string shortcut:
        Settings.values["accessibility.shortcut"] !== undefined
            ? String(Settings.values["accessibility.shortcut"]) : ""

    // Test/capture surface.
    property alias shortcutGroup: shortcutGroup
    property alias shortcutRow: shortcutRow
    property alias recorder: recorder

    function recordShortcut(chord) {
        Settings.set("accessibility.shortcut", chord);
    }

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Accessibility Shortcut ──────────────────────────────────────
        SettingsGroup {
            id: shortcutGroup
            width: parent.width
            sectionId: "shortcut"
            title: qsTr("Accessibility Shortcut")

            SettingsRow {
                id: shortcutRow
                objectName: "accessibilityShortcutRow"
                width: parent.width
                settingsId: "shortcut.recorder"
                owner: "shell/compositor shortcut engine"
                settingsKey: "accessibility.shortcut"
                label: qsTr("Accessibility shortcut")
                description: root.shortcut.length > 0
                    ? qsTr("Opens Accessibility from anywhere. Recorded by the "
                           + "compositor's shortcut engine; Backspace clears it.")
                    : qsTr("Opens Accessibility from anywhere. Record a chord "
                           + "here; the compositor installs it.")
                showSeparator: false
                controlData: ShortcutRecorder {
                    id: recorder
                    objectName: "accessibilityShortcutRecorder"

                    onRecorded: (chord) => root.recordShortcut(chord)
                    onCleared: root.recordShortcut("")
                }
            }
        }
    }

    // Keep the recorder in step with external changes.
    Binding { target: recorder; property: "chord"; value: root.shortcut }
}
