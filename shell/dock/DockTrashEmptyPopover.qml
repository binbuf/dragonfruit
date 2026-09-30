// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit

// The Dock's Trash empty progress/result popover (T-14.7r). The confirmation
// step still lives in the Trash context menu; confirming Empty Trash swaps to
// this popover, which shows one state at a time:
//
//   * emptying  — a busy indicator, shown only after `busyDelay` so a fast
//                 empty never flickers through it;
//   * succeeded — a check and the removed item count;
//   * failed    — the error message and a Try Again button.
//
// The operation itself runs on the shell's `TrashBridge` worker; this view
// only renders the state it is handed and reports Try Again. The shell renders
// it into the Dock's `overlay` chrome surface, anchored to the Trash entry.
FocusScope {
    id: root

    // emptying | succeeded | failed
    property string phase: "emptying"
    // Shown only once the busy delay has elapsed (never for a fast empty).
    property bool busyVisible: false
    property int removedCount: 0
    property string errorMessage: ""
    property Item anchorItem: null
    property bool open: false

    signal retryRequested()
    signal opened()
    signal closed()

    readonly property real padding: Theme.controls.contextMenu.padding
    readonly property real rowHeight: Theme.controls.contextMenu.rowHeight
    readonly property real arrowSize: Theme.controls.popover.arrowSize
    readonly property real radius: Theme.controls.contextMenu.radius
    readonly property real spinnerSize: Theme.controls.dock.trashEmpty.spinnerSize
    readonly property real spinnerStroke: Theme.controls.dock.trashEmpty.spinnerStroke
    readonly property int spinnerSpeed: Theme.controls.dock.trashEmpty.spinnerSpeed
    readonly property real resultIconSize: Theme.controls.button.iconSize

    readonly property bool isFailed: phase === "failed"
    readonly property bool isSucceeded: phase === "succeeded"

    readonly property string countText:
        removedCount === 1 ? qsTr("1 item removed")
                           : qsTr("%1 items removed").arg(removedCount)

    // The one visible status sentence. Used for the accessible name and
    // announced on every state change.
    readonly property string statusText: {
        if (isSucceeded)
            return countText;
        if (isFailed)
            return errorMessage.length > 0 ? errorMessage
                                           : qsTr("The Trash could not be emptied");
        return qsTr("Emptying the Trash…");
    }
    readonly property string accessibleStateText: {
        if (isSucceeded)
            return qsTr("Trash emptied, %1").arg(countText);
        if (isFailed)
            return qsTr("Could not empty the Trash. %1. Try Again is available.")
                .arg(statusText);
        return qsTr("Emptying the Trash");
    }

    function hide() { root.open = false; }
    function retry() { root.retryRequested(); }

    width: Math.max(Theme.controls.contextMenu.minWidth,
                    body.implicitWidth + 2 * root.padding)
    height: root.padding + headerRow.height + separator.height
            + body.implicitHeight + root.padding + root.arrowSize / 2
    visible: opacity > 0
    scale: root.open ? 1.0 : 0.97
    transformOrigin: Item.Bottom
    opacity: root.open ? 1.0 : 0.0
    z: 2000

    Behavior on opacity {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }
    Behavior on scale {
        NumberAnimation {
            duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve
                                          : Theme.motion.popupClose.curve
        }
    }

    onOpenChanged: {
        if (root.open) {
            // The safe action is the only focus target: Try Again when the
            // empty failed, otherwise the popover itself (a dismiss).
            if (root.isFailed)
                retryButton.forceActiveFocus();
            else
                root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }
    onPhaseChanged: {
        Accessible.announce(root.accessibleStateText);
        // Defer: the failure row's visibility binding settles after the
        // phase change, so focus it on the next event-loop turn.
        Qt.callLater(function() {
            if (root.open && root.isFailed)
                retryButton.forceActiveFocus();
        });
    }

    Shadow {
        width: root.width
        height: root.height
        radius: root.radius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        objectName: "trashEmptySurface"
        anchors.fill: parent
        anchors.bottomMargin: root.arrowSize / 2
        radius: root.radius
        color: Qt.rgba(Theme.color.surfaceElevated.r, Theme.color.surfaceElevated.g,
                       Theme.color.surfaceElevated.b, Theme.material.popupOpacity)
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    // The arrow ties the popover to the Trash entry (points down at the Dock).
    Rectangle {
        objectName: "trashEmptyArrow"
        width: root.arrowSize
        height: root.arrowSize
        rotation: 45
        color: Qt.rgba(Theme.color.surfaceElevated.r, Theme.color.surfaceElevated.g,
                       Theme.color.surfaceElevated.b, Theme.material.popupOpacity)
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        x: {
            if (!root.anchorItem || !root.parent)
                return root.width / 2 - width / 2;
            var p = root.anchorItem.mapToItem(root.parent, root.anchorItem.width / 2, 0);
            return Math.max(root.radius,
                            Math.min(root.width - root.radius, p.x - root.x)) - width / 2;
        }
        y: root.height - height
        z: -1
    }

    Item {
        id: headerRow
        objectName: "trashEmptyHeader"
        x: root.padding
        y: root.padding
        width: root.width - 2 * root.padding
        height: root.rowHeight

        Text {
            id: headerText
            objectName: "trashEmptyHeaderText"
            text: qsTr("Trash")
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.bold: true
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.contextMenu.padding
        }
        Accessible.role: Accessible.StaticText
        Accessible.name: qsTr("Trash")
    }

    Rectangle {
        id: separator
        x: root.padding
        y: headerRow.y + headerRow.height
        width: root.width - 2 * root.padding
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    Column {
        id: body
        x: root.padding
        y: separator.y + separator.height
        width: root.width - 2 * root.padding

        // The status row: spinner while working, a check on success, a danger
        // mark on failure. The glyph slot is always reserved so the delayed
        // busy indicator never shifts the layout.
        Item {
            id: statusRow
            objectName: "trashEmptyStatusRow"
            width: body.width
            height: root.rowHeight

            Item {
                id: glyphSlot
                width: Math.max(root.spinnerSize, root.resultIconSize)
                height: width
                anchors.left: parent.left
                anchors.leftMargin: Theme.controls.contextMenu.padding
                anchors.verticalCenter: parent.verticalCenter

                // The indeterminate ring: a 270-degree arc with a gap, rotated.
                // Under reduced motion it does not spin, but the ring/gap still
                // reads as "working" against the success check.
                Shape {
                    id: spinner
                    objectName: "trashEmptySpinner"
                    width: root.spinnerSize
                    height: root.spinnerSize
                    anchors.centerIn: parent
                    antialiasing: true
                    visible: root.phase === "emptying" && root.busyVisible

                    ShapePath {
                        strokeColor: Theme.color.textTertiary
                        strokeWidth: root.spinnerStroke
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        PathAngleArc {
                            centerX: spinner.width / 2
                            centerY: spinner.height / 2
                            radiusX: (spinner.width - root.spinnerStroke) / 2
                            radiusY: (spinner.height - root.spinnerStroke) / 2
                            startAngle: -90
                            sweepAngle: 270
                        }
                    }

                    RotationAnimator on rotation {
                        from: 0
                        to: 360
                        duration: root.spinnerSpeed
                        loops: Animation.Infinite
                        running: spinner.visible && !Theme.reducedMotion
                    }
                }

                Icon {
                    id: checkGlyph
                    objectName: "trashEmptyCheck"
                    anchors.centerIn: parent
                    visible: root.isSucceeded
                    name: "check"
                    size: root.resultIconSize
                    color: Theme.color.success
                }

                // A small hollow ring with a slash: the failure mark, distinct
                // from the success check and the busy ring.
                Item {
                    objectName: "trashEmptyFailureMark"
                    anchors.centerIn: parent
                    visible: root.isFailed
                    width: root.resultIconSize
                    height: root.resultIconSize

                    Rectangle {
                        anchors.centerIn: parent
                        width: root.resultIconSize
                        height: width
                        radius: width / 2
                        color: "transparent"
                        border.width: Math.max(1, root.resultIconSize * 0.14)
                        border.color: Theme.color.danger
                        antialiasing: true
                    }
                    Rectangle {
                        anchors.centerIn: parent
                        width: root.resultIconSize * 0.7
                        height: Math.max(1, root.resultIconSize * 0.14)
                        radius: height / 2
                        color: Theme.color.danger
                        antialiasing: true
                    }
                }
            }

            Text {
                id: statusText
                objectName: "trashEmptyStatusText"
                text: root.statusText
                color: root.isFailed ? Theme.color.danger : Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
                elide: Text.ElideRight
                wrapMode: Text.WordWrap
                anchors.left: glyphSlot.right
                anchors.leftMargin: Theme.primitive.spacing.sm
                anchors.right: parent.right
                anchors.rightMargin: Theme.controls.contextMenu.padding
                anchors.verticalCenter: parent.verticalCenter

                Accessible.role: Accessible.StaticText
                Accessible.name: root.accessibleStateText
            }
        }

        // The failure's safe action. Focus lands here when the failure shows.
        Button {
            id: retryButton
            objectName: "trashEmptyRetry"
            visible: root.isFailed
            height: visible ? implicitHeight : 0
            text: qsTr("Try Again")
            variant: "primary"
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: root.retry()
        }
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Escape) {
            root.hide();
            event.accepted = true;
        } else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                    || event.key === Qt.Key_Space) && root.isFailed) {
            root.retry();
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: root.accessibleStateText
}