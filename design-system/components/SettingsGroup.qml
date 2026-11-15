// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A titled card of SettingsRows (or any content) on an elevated surface. The
// group owns the card, the optional section header, and the trailing gap; the
// rows are supplied through the default property so apps never draw the card.
//
// `sectionId` (T223) links the card to the executable map's section so a
// search result can deep-link to a section when the pane's rows are not
// individual SettingsRows. `highlighted` is the deep-link landing state.
Column {
    id: root

    property string title: ""
    property string sectionId: ""
    // T266: a group in a pane's spaced Column owns one bottom gap (the
    // Column's spacing). The group's own margin is opt-in for a standalone
    // card, so the two gaps are never added together.
    property bool reserveBottomMargin: false
    default property alias contentData: rows.data
    property alias rows: rows
    property alias titleText: titleText
    property alias highlightRect: highlightRect
    property bool highlighted: false

    spacing: 0

    Text {
        id: titleText
        visible: root.title.length > 0
        width: parent.width
        // T266: the section header aligns with the row labels inside the card
        // and wraps (or elides) rather than running past the card.
        leftPadding: Theme.controls.settingsGroup.padding
                     + Theme.controls.settingsRow.paddingH
        rightPadding: Theme.controls.settingsGroup.padding
        bottomPadding: Theme.primitive.spacing.sm
        text: root.title
        color: Theme.color.textSecondary
        font.pixelSize: Theme.primitive.font.sizeSm
        font.weight: Theme.primitive.font.weightMedium
        wrapMode: Text.Wrap
        maximumLineCount: 2
        elide: Text.ElideRight
    }

    Item {
        id: body
        width: root.width
        height: rows.implicitHeight + 2 * Theme.controls.settingsGroup.padding

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.settingsGroup.radius
            color: Theme.color.surfaceElevated
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        // T223: a section-level deep-link landing wash. It is a rim rather than
        // a fill so the card's rows stay readable.
        Rectangle {
            id: highlightRect
            anchors.fill: parent
            radius: Theme.controls.settingsGroup.radius
            color: "transparent"
            border.width: root.highlighted ? 2 : 0
            border.color: Theme.color.accent
            opacity: root.highlighted ? 1.0 : 0.0
            antialiasing: true

            Behavior on opacity {
                NumberAnimation {
                    duration: Theme.motion.hover.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.hover.curve
                }
            }
        }

        Column {
            id: rows
            // T266: SettingsRow reads this to suppress the last row's
            // separator without rewriting any row's declared state.
            property bool settingsGroupRows: true
            x: Theme.controls.settingsGroup.padding
            y: Theme.controls.settingsGroup.padding
            width: body.width - 2 * Theme.controls.settingsGroup.padding
            spacing: Theme.controls.settingsGroup.rowGap
        }
    }

    Item {
        width: parent.width
        height: root.reserveBottomMargin ? Theme.controls.settingsGroup.marginBottom : 0
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
}
