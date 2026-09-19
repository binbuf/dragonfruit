// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// A two-pane split with a draggable, keyboard-operable divider. The panes are
// filled through `firstData`/`secondData`; the divider is progress-based (it
// follows the pointer and is interruptible) and reports an AT-SPI Splitter.
Item {
    id: root

    property alias firstData: firstPane.data
    property alias secondData: secondPane.data
    property alias firstPane: firstPane
    property alias secondPane: secondPane
    property alias divider: divider
    // Fraction of the available axis given to the first pane.
    property real splitPosition: 0.4
    property bool vertical: false
    property bool dividerHovered: dividerMouse.containsMouse
    property bool dividerActive: dividerMouse.pressed

    readonly property int dividerWidth: Theme.controls.splitView.dividerWidth
    readonly property real available: Math.max(0, (root.vertical ? root.height : root.width)
                                              - root.dividerWidth)
    readonly property real minimumPane: Theme.controls.splitView.minPaneWidth
    // When the view is too small for two minimum panes, fall back to an even
    // split rather than collapsing a pane to zero.
    readonly property real firstSize: root.available > 2 * root.minimumPane
                                      ? Math.max(root.minimumPane,
                                                 Math.min(root.splitPosition * root.available,
                                                          root.available - root.minimumPane))
                                      : root.available / 2

    implicitWidth: 480
    implicitHeight: 320
    clip: true

    function setFirstSize(size) {
        if (root.available <= 0)
            return;
        var clamped = Math.max(root.minimumPane,
                               Math.min(size, root.available - root.minimumPane));
        root.splitPosition = clamped / root.available;
    }

    Item {
        id: firstPane
        clip: true
        x: 0
        y: 0
        width: root.vertical ? root.width : root.firstSize
        height: root.vertical ? root.firstSize : root.height
    }

    Item {
        id: secondPane
        clip: true
        x: root.vertical ? 0 : root.firstSize + root.dividerWidth
        y: root.vertical ? root.firstSize + root.dividerWidth : 0
        width: root.vertical ? root.width
                             : Math.max(0, root.width - root.firstSize - root.dividerWidth)
        height: root.vertical ? Math.max(0, root.height - root.firstSize - root.dividerWidth)
                              : root.height
    }

    Rectangle {
        id: divider
        x: root.vertical ? 0 : root.firstSize
        y: root.vertical ? root.firstSize : 0
        width: root.vertical ? root.width : root.dividerWidth
        height: root.vertical ? root.dividerWidth : root.height
        color: (root.dividerHovered || root.dividerActive || divider.activeFocus)
               ? Theme.color.accent : Theme.color.separator
        antialiasing: true
        activeFocusOnTab: true

        Behavior on color {
            ColorAnimation {
                duration: Theme.motion.hover.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.hover.curve
            }
        }

        FocusRing {
            target: divider
            cornerRadius: 0
            shown: divider.activeFocus
        }

        Keys.onPressed: (event) => {
            var forward = root.vertical ? Qt.Key_Down : Qt.Key_Right;
            var backward = root.vertical ? Qt.Key_Up : Qt.Key_Left;
            if (event.key === forward) {
                root.setFirstSize(root.firstSize + Theme.primitive.spacing.lg);
                event.accepted = true;
            } else if (event.key === backward) {
                root.setFirstSize(root.firstSize - Theme.primitive.spacing.lg);
                event.accepted = true;
            } else if (event.key === Qt.Key_Home) {
                root.setFirstSize(0);
                event.accepted = true;
            } else if (event.key === Qt.Key_End) {
                root.setFirstSize(root.available);
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.Splitter
        Accessible.name: qsTr("Split divider")
        Accessible.focusable: true
    }

    MouseArea {
        id: dividerMouse
        anchors.fill: divider
        hoverEnabled: true
        cursorShape: root.vertical ? Qt.SizeVerCursor : Qt.SizeHorCursor
        property real startPointer: 0
        property real startSize: 0

        onPressed: (mouse) => {
            dividerMouse.startSize = root.firstSize;
            var p = dividerMouse.mapToItem(root, mouse.x, mouse.y);
            dividerMouse.startPointer = root.vertical ? p.y : p.x;
        }
        onPositionChanged: (mouse) => {
            if (!dividerMouse.pressed)
                return;
            var p = dividerMouse.mapToItem(root, mouse.x, mouse.y);
            var delta = (root.vertical ? p.y : p.x) - dividerMouse.startPointer;
            root.setFirstSize(dividerMouse.startSize + delta);
        }
    }
}
