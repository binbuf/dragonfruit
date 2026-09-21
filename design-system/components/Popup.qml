// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A floating surface used for menus, popovers, and control-center panels.
// Open/close is progress-based and interruptible: the animation can be
// retargeted mid-flight and the reduced-motion variant removes the scale
// while keeping the show/hide legible.
Item {
    id: root

    property bool open: false
    property Item anchorItem: null
    property int preferredWidth: Theme.controls.popup.minWidth
    property int horizontalPadding: Theme.controls.popup.padding
    property int verticalPadding: Theme.controls.popup.padding
    property int accessibleRole: Accessible.Pane
    property string accessibleName: ""
    // When false the owner handles Escape itself (e.g. a menu with an open
    // submenu closes the submenu first) and the popup stays open.
    property bool escapeCloses: true

    default property alias contentData: contentColumn.data
    property alias content: contentColumn

    signal opened()
    signal closed()
    signal escapePressed()

    function show() { root.open = true; }
    function hide() { root.open = false; }
    function toggle() { root.open = !root.open; }

    focus: open
    activeFocusOnTab: open
    visible: opacity > 0
    z: 1000

    readonly property point anchorPoint: {
        if (!anchorItem || !root.parent)
            return Qt.point((root.parent ? root.parent.width : 0) / 2 - width / 2,
                            (root.parent ? root.parent.height : 0) / 2 - height / 2);
        var p = anchorItem.mapToItem(root.parent, 0, anchorItem.height);
        return Qt.point(p.x, p.y + Theme.controls.popup.offset);
    }

    width: Math.max(preferredWidth, contentColumn.implicitWidth + 2 * horizontalPadding)
    height: contentColumn.implicitHeight + 2 * verticalPadding
    x: anchorPoint.x
    y: anchorPoint.y
    scale: open ? 1.0 : 0.97
    transformOrigin: Item.Top
    opacity: open ? 1.0 : 0.0

    onOpenChanged: {
        if (open) {
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
        radius: Theme.controls.popup.radius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.popup.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Column {
        id: contentColumn
        x: root.horizontalPadding
        y: root.verticalPadding
        width: root.width - 2 * root.horizontalPadding
    }

    Keys.onEscapePressed: (event) => {
        root.escapePressed();
        if (root.escapeCloses)
            root.hide();
        event.accepted = true;
    }

    Accessible.role: root.accessibleRole
    Accessible.name: root.accessibleName
}
