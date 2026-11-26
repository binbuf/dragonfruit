// SPDX-License-Identifier: GPL-3.0-only
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

    // T268: `writeErrors` is a per-key property map, so this binding tracks
    // exactly this row's key instead of re-evaluating on every write.
    errorText: settingsKey.length > 0
        ? String(Settings.writeErrors[settingsKey] || "") : ""
}
