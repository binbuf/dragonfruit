// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// A SettingsRow wired to the common apply path (T224). It is the same row
// anatomy as `SettingsRow`, plus the write error for `settingsKey`: a failed
// write reverts the bound control and the reason appears under the label.
//
// Panes use this for every settingsd-backed row; a row with no key keeps the
// plain anatomy and never shows a stale error.
SettingsRow {
    id: root

    // Read the map property (not the `errorFor()` helper) so this binding
    // tracks `writeStatesChanged`; a method call alone would not re-evaluate.
    errorText: settingsKey.length > 0
        ? String(Settings.writeErrors[settingsKey] || "") : ""
}
