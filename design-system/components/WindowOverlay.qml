// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The window's popup/dialog layer (T266).
//
// It sits above the app content and the resize grips, covers the whole window,
// and registers with `OverlayLayer`. Panes declare no parent for their
// `Dialog`s and `Select`s; those components adopt this layer by themselves, so
// a surface inside the detail `ScrollView` is never clipped. The layer itself
// has no pointer handlers and never eats a click meant for the app.
Item {
    id: root

    anchors.fill: parent
    z: 4000

    Component.onCompleted: OverlayLayer.register(root)
    Component.onDestruction: OverlayLayer.unregister(root)
}
