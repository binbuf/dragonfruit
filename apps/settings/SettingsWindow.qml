// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// Settings window placeholder — T-16 builds the real application and its
// panes; every pane must consume design-system components only.
Window {
    width: 800
    height: 600
    visible: true
    title: qsTr("Dragonfruit Settings")
    color: Theme.color.surface
}
