// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The Settings pane intro (T286): body copy plus an optional "Learn More..."
// link. It replaces the old header card: the toolbar title is the only pane
// title, and a pane that has intro text opens with this paragraph.
Column {
    id: root

    property string text: ""
    property string learnMoreText: ""
    property bool learnMoreVisible: root.learnMoreText.length > 0
    property alias textItem: introText
    property alias learnMoreLink: learnMoreArea

    signal learnMoreClicked()

    Accessible.role: Accessible.StaticText
    Accessible.name: root.text
    Accessible.description: root.learnMoreText

    width: parent ? parent.width : implicitWidth
    spacing: Theme.primitive.spacing.xs

    Text {
        id: introText
        objectName: "settingsIntroText"
        width: parent.width
        text: root.text
        color: Theme.color.textSecondary
        font.pixelSize: Theme.controls.button.fontSize
        lineHeight: 1.3
        wrapMode: Text.WordWrap
        horizontalAlignment: root.LayoutMirroring.enabled
                             ? Text.AlignRight : Text.AlignLeft
    }

    Item {
        id: learnMoreArea
        visible: root.learnMoreVisible
        width: learnMoreLabel.implicitWidth
        height: learnMoreLabel.implicitHeight
        activeFocusOnTab: true

        Text {
            id: learnMoreLabel
            text: root.learnMoreText
            color: Theme.color.accent
            font.pixelSize: Theme.controls.button.fontSize
        }

        HoverHandler { id: learnMoreHover }

        TapHandler {
            onTapped: root.learnMoreClicked()
        }

        FocusRing {
            target: learnMoreArea
            cornerRadius: Theme.primitive.radius.xs
            shown: learnMoreArea.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                    || event.key === Qt.Key_Space) {
                root.learnMoreClicked();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.Link
        Accessible.name: root.learnMoreText
        Accessible.focusable: true
        Accessible.onPressAction: root.learnMoreClicked()
    }
}
