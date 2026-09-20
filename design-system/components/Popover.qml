// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A pointer-anchored popover: like Popup but with an arrow that ties the
// surface to its anchor. Used for inspector-style controls and rich tooltips
// where a menu's rectangular dropdown would lose the connection.
Item {
    id: root

    property bool open: false
    property Item anchorItem: null
    property int preferredWidth: Theme.controls.dialog.minWidth
    property int horizontalPadding: Theme.controls.popover.padding
    property int verticalPadding: Theme.controls.popover.padding
    property string accessibleName: ""

    default property alias contentData: contentColumn.data
    property alias content: contentColumn

    signal opened()
    signal closed()

    readonly property int arrowSize: Theme.controls.popover.arrowSize
    readonly property point anchorPoint: {
        if (!anchorItem || !root.parent)
            return Qt.point((root.parent ? root.parent.width : 0) / 2 - width / 2,
                            (root.parent ? root.parent.height : 0) / 2 - height / 2);
        var p = anchorItem.mapToItem(root.parent, 0, anchorItem.height);
        return Qt.point(p.x, p.y + Theme.controls.popup.offset);
    }
    readonly property real arrowX: {
        if (!anchorItem || !root.parent)
            return width / 2;
        var p = anchorItem.mapToItem(root.parent, anchorItem.width / 2, 0);
        return Math.max(Theme.controls.popover.radius,
                        Math.min(width - Theme.controls.popover.radius, p.x - root.x));
    }

    function show() { root.open = true; }
    function hide() { root.open = false; }
    function toggle() { root.open = !root.open; }

    focus: root.open
    activeFocusOnTab: root.open
    visible: opacity > 0
    z: 1500
    width: Math.max(preferredWidth, contentColumn.implicitWidth + 2 * horizontalPadding)
    height: contentColumn.implicitHeight + 2 * verticalPadding
    x: anchorPoint.x
    y: anchorPoint.y
    scale: root.open ? 1.0 : 0.97
    transformOrigin: Item.Top
    opacity: root.open ? 1.0 : 0.0

    onOpenChanged: {
        if (root.open) {
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
        }
    }

    Shadow {
        width: root.width
        height: root.height
        radius: Theme.controls.popover.radius
        blur: Theme.controls.popover.shadowBlur
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.popover.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Rectangle {
        id: arrow
        width: root.arrowSize
        height: root.arrowSize
        x: root.arrowX - width / 2
        y: -height / 2
        rotation: 45
        color: Theme.color.surfaceElevated
        antialiasing: true
    }

    Column {
        id: contentColumn
        x: root.horizontalPadding
        y: root.verticalPadding
        width: root.width - 2 * root.horizontalPadding
    }

    Keys.onEscapePressed: (event) => {
        root.hide();
        event.accepted = true;
    }

    Accessible.role: Accessible.Pane
    Accessible.name: root.accessibleName
}
