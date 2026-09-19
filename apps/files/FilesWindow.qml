// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// Files window placeholder — T-18 builds the real application on the
// files-core model; every surface consumes design-system components only.
Window {
    width: 900
    height: 600
    visible: true
    title: qsTr("Dragonfruit Files")
    color: Theme.color.surface
}
