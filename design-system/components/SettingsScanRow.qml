// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// A "Scanning..." / "Searching..." Settings row (T286). It is the shared
// SettingsRow anatomy with a live spinner in the leading cell, so a scan row
// never re-rolls the row layout.
SettingsRow {
    id: root

    property string scanningText: qsTr("Scanning\u2026")
    property alias spinner: spinner

    label: root.scanningText
    rowHeight: Theme.controls.settingsRow.height

    leadingWidth: Theme.controls.spinner.size
    leadingHeight: Theme.controls.spinner.size
    labelIndent: Theme.controls.settingsRow.paddingH
                 + Theme.controls.spinner.size + Theme.primitive.spacing.md

    leadingData: Spinner {
        id: spinner
        width: Theme.controls.spinner.size
        height: width
        running: root.visible
        accessibleName: root.scanningText
    }

    Accessible.description: root.scanningText
}
