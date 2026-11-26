// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit

// The optional pane footer (T286): a row of buttons 18 pt below the last
// card. `alignment` selects the mockup's right-aligned default or the
// left-aligned Add/Remove actions.
Item {
    id: root

    property string alignment: "right"
    default property alias contentData: footerRow.data
    property alias row: footerRow

    implicitWidth: 320
    implicitHeight: footerRow.implicitHeight
                     + Theme.controls.settingsGroup.marginBottom

    Row {
        id: footerRow
        anchors.top: parent.top
        anchors.right: root.alignment === "right" ? parent.right : undefined
        anchors.left: root.alignment === "left" ? parent.left : undefined
        width: implicitWidth
        height: implicitHeight
        spacing: Theme.primitive.spacing.sm
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: qsTr("Actions")
}
