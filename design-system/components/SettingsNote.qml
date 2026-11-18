// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A muted footnote under a Settings card (T286): 12 pt type on a 16 pt line.
Text {
    id: root

    width: parent ? parent.width : implicitWidth
    color: Glass.increaseContrast ? Theme.color.textPrimary
                                  : Theme.color.textTertiary
    font.pixelSize: Theme.primitive.font.sizeSm
    lineHeight: 1.34
    wrapMode: Text.WordWrap
    horizontalAlignment: root.LayoutMirroring.enabled
                         ? Text.AlignRight : Text.AlignLeft

    Accessible.role: Accessible.StaticText
    Accessible.name: root.text
}
