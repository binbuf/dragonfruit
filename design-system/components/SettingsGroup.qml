// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// A titled card of SettingsRows (or any content) on an elevated surface. The
// group owns the card, the optional section header, and the trailing gap; the
// rows are supplied through the default property so apps never draw the card.
//
// T286: the card has no inner padding (rows own their 12 pt inset), the
// section heading sits outside the card at the card edge with 26 pt above and
// 8 pt below, and the 10 pt radius is the mockup's card geometry.
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
        // T286: the heading aligns to the card edge, not the row-label inset
        // (the mockup's `.heading` sits outside the card).
        topPadding: Theme.controls.settingsGroup.headingTop
        bottomPadding: Theme.controls.settingsGroup.headingBottom
        text: root.title
        color: Glass.increaseContrast ? Theme.color.textPrimary
                                      : Theme.color.textSecondary
        font.pixelSize: Theme.controls.settingsGroup.headingFontSize
        font.weight: Theme.controls.settingsGroup.headingFontWeight
        wrapMode: Text.Wrap
        maximumLineCount: 2
        elide: Text.ElideRight
        horizontalAlignment: root.LayoutMirroring.enabled
                             ? Text.AlignRight : Text.AlignLeft
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
            border.color: Glass.increaseContrast ? Theme.color.textPrimary
                                                 : Theme.color.border
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
