// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The screenshot selection overlay (T-13.3a): the visual half of the portal
// presenter seam and of the desktop's own Cmd+Shift+3/4 shortcut.
// `ShellController` sets `mode` from the `ScreenshotBridge` and renders this
// into a full-output `screenshot` overlay surface; the view is pure and never
// calls D-Bus.
//
// `fullscreen` is a single click; `region` is a drag with a live rectangle and
// its pixel dimensions; `window` tracks the pointer and captures the window
// under it. Escape cancels; Return accepts the current selection.
Item {
    id: root

    // "fullscreen" | "region" | "window".
    property string mode: "region"
    // The smallest useful region (a stray click is not a selection).
    property int minimumRegion: 8

    // The live selection in overlay-local pixels.
    readonly property int selectionX: Math.round(Math.min(dragStartX, dragCurrentX))
    readonly property int selectionY: Math.round(Math.min(dragStartY, dragCurrentY))
    readonly property int selectionWidth: Math.round(Math.abs(dragCurrentX - dragStartX))
    readonly property int selectionHeight: Math.round(Math.abs(dragCurrentY - dragStartY))
    readonly property bool hasSelection: selectionWidth >= root.minimumRegion
                                          && selectionHeight >= root.minimumRegion
    readonly property bool fullscreenMode: root.mode === "fullscreen"
    readonly property bool windowMode: root.mode === "window"

    property real dragStartX: 0
    property real dragStartY: 0
    property real dragCurrentX: 0
    property real dragCurrentY: 0
    property real pointerX: -1
    property real pointerY: -1
    property bool dragging: false

    signal accepted(int x, int y, int width, int height)
    signal cancelled()

    function acceptCurrent() {
        if (root.fullscreenMode) {
            root.accepted(0, 0, root.width, root.height);
            return;
        }
        if (root.windowMode) {
            if (root.pointerX < 0 || root.pointerY < 0)
                return;
            root.accepted(Math.round(root.pointerX), Math.round(root.pointerY), 1, 1);
            return;
        }
        if (root.hasSelection)
            root.accepted(root.selectionX, root.selectionY, root.selectionWidth,
                          root.selectionHeight);
    }

    Accessible.role: Accessible.Dialog
    Accessible.name: qsTr("Screenshot selection")
    focus: true

    Keys.onEscapePressed: root.cancelled()
    Keys.onReturnPressed: root.acceptCurrent()
    Keys.onEnterPressed: root.acceptCurrent()

    // The scrim. Kept subtle so the desktop stays readable while selecting.
    Rectangle {
        id: scrim
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: root.fullscreenMode ? 0.18 : 0.32
    }

    // Region: the clear selection window and its live dimensions.
    Rectangle {
        id: regionRect
        objectName: "regionRect"
        visible: root.mode === "region" && root.dragging
        x: root.selectionX
        y: root.selectionY
        width: root.selectionWidth
        height: root.selectionHeight
        color: Qt.rgba(0, 0, 0, 0)
        border.width: 2
        border.color: Theme.color.accent
        antialiasing: false

        Text {
            id: sizeLabel
            objectName: "sizeLabel"
            visible: root.selectionWidth >= 48
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top: parent.top
            anchors.topMargin: Theme.primitive.spacing.sm
            text: root.selectionWidth + " × " + root.selectionHeight
            color: Theme.color.accentContent
            padding: Theme.primitive.spacing.xs
            font.pixelSize: Theme.primitive.font.sizeSm
            font.weight: Theme.primitive.font.weightMedium
            style: Text.Outline
            styleColor: Theme.color.shadowColor
        }
    }

    // Fullscreen: the whole output reads as the selection.
    Rectangle {
        id: fullscreenRect
        objectName: "fullscreenRect"
        visible: root.fullscreenMode
        anchors.fill: parent
        anchors.margins: 2
        color: Qt.rgba(0, 0, 0, 0)
        border.width: 2
        border.color: Theme.color.accent
    }

    // Window: a crosshair follows the pointer.
    Item {
        id: crosshair
        objectName: "crosshair"
        visible: root.windowMode && root.pointerX >= 0
        x: root.pointerX
        y: root.pointerY
        width: 1
        height: 1

        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            width: 1
            height: 24
            y: -12
            color: Theme.color.accent
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 24
            height: 1
            x: -12
            color: Theme.color.accent
        }
    }

    // The mode badge and the hint, top-center.
    Rectangle {
        id: badge
        objectName: "modeBadge"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: Theme.primitive.spacing.lg
        width: badgeRow.width + Theme.primitive.spacing.lg * 2
        height: badgeRow.height + Theme.primitive.spacing.md
        radius: Theme.primitive.radius.md
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        Row {
            id: badgeRow
            anchors.centerIn: parent
            spacing: Theme.primitive.spacing.sm

            Text {
                id: modeLabel
                objectName: "modeLabel"
                anchors.verticalCenter: parent.verticalCenter
                text: root.fullscreenMode ? qsTr("Screenshot · Fullscreen")
                      : root.windowMode ? qsTr("Screenshot · Window")
                                        : qsTr("Screenshot · Region")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
            }

            Text {
                id: hintLabel
                objectName: "hintLabel"
                anchors.verticalCenter: parent.verticalCenter
                text: root.fullscreenMode ? qsTr("Click or press Return to capture")
                      : root.windowMode ? qsTr("Click a window · Esc to cancel")
                                        : qsTr("Drag to select · Esc to cancel")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }
    }

    MouseArea {
        id: input
        objectName: "selectionInput"
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        hoverEnabled: true

        onPressed: (mouse) => {
            root.dragging = true;
            root.dragStartX = mouse.x;
            root.dragStartY = mouse.y;
            root.dragCurrentX = mouse.x;
            root.dragCurrentY = mouse.y;
            root.pointerX = mouse.x;
            root.pointerY = mouse.y;
        }
        onPositionChanged: (mouse) => {
            root.pointerX = mouse.x;
            root.pointerY = mouse.y;
            if (root.dragging) {
                root.dragCurrentX = mouse.x;
                root.dragCurrentY = mouse.y;
            }
        }
        onReleased: (mouse) => {
            root.dragging = false;
            root.dragCurrentX = mouse.x;
            root.dragCurrentY = mouse.y;
            if (root.fullscreenMode || root.windowMode)
                root.acceptCurrent();
            else if (root.hasSelection)
                root.accepted(root.selectionX, root.selectionY, root.selectionWidth,
                              root.selectionHeight);
        }
    }
}