// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Mission Control overview chrome (T-11 Slice B).
//
// The compositor renders the *live* window surfaces (translated) while the
// overview is open; this shell surface is deliberately transparent and only
// draws the chrome the compositor does not own:
//
//   * the workspace strip along the top (including the dedicated Spaces of
//     fullscreen windows, FR-10), clickable to activate a Space and a drop
//     target for moving windows between Spaces;
//   * the minimized-window bottom strip (FR-6), clickable to restore;
//   * the window grid in the middle (FR-7): one draggable card per visible
//     window (title/app_label, never a content thumbnail — the compositor
//     still renders the live surface underneath), dropped onto a Space card
//     to move the window there;
//   * the selection entry point: a click calls the existing
//     `select_overview_toplevel` round-trip through the shell controller
//     (FR-5).
//
// The compositor owns workspace truth; this item is a pure projection of the
// `df_workspace`/`df_toplevel` events (never a second copy). `progress` is the
// shared pipeline's 0..1 sample, so the strips fade/slide with the same curve
// as the gesture.
Item {
    id: root

    // Injected by the shell controller from the private protocol.
    property var workspaces: []
    property var minimizedWindows: []
    property var windows: []
    property real progress: 0
    property bool active: false

    // Drag state for moving a window between Spaces (FR-7). `dragWorkspaceIndex`
    // is the Space card under the pointer, or -1 outside any card.
    property string draggingWindowId: ""
    property int dragWorkspaceIndex: -1
    property point dragScenePos: Qt.point(0, 0)

    signal workspaceActivated(int index)
    signal windowActivated(string windowId)
    signal windowMovedToWorkspace(string windowId, int index)
    signal dismissRequested()

    focus: root.active

    Keys.onEscapePressed: root.dismissRequested()

    // The generated Space cards, in strip order (the Repeater reparents its
    // delegates onto the strip Row).
    function workspaceCards() {
        var strip = workspaceStrip;
        var out = [];
        for (var i = 0; i < strip.children.length; ++i) {
            if (strip.children[i].objectName === "spaceCard")
                out.push(strip.children[i]);
        }
        return out;
    }

    // Press-and-hold then move a window card lifts it; the pointer's scene
    // position decides the target Space. Split into begin/update/drop so tests
    // can drive the model without synthesizing a full drag.
    function beginWindowDrag(windowId) {
        draggingWindowId = windowId;
        dragWorkspaceIndex = -1;
        dragScenePos = Qt.point(0, 0);
    }

    function updateWindowDrag(windowId, sceneX, sceneY) {
        if (draggingWindowId.length === 0 || windowId !== draggingWindowId)
            return;
        dragScenePos = Qt.point(sceneX, sceneY);
        dragWorkspaceIndex = -1;
        var list = workspaceCards();
        for (var i = 0; i < list.length; ++i) {
            var card = list[i];
            var topLeft = card.mapToItem(null, 0, 0);
            var bottomRight = card.mapToItem(null, card.width, card.height);
            if (sceneX >= topLeft.x && sceneX <= bottomRight.x
                    && sceneY >= topLeft.y && sceneY <= bottomRight.y) {
                dragWorkspaceIndex = card.modelData.index;
                break;
            }
        }
    }

    function dropWindowOnWorkspace(index) {
        var windowId = draggingWindowId;
        cancelWindowDrag();
        if (windowId.length === 0 || index < 0)
            return;
        windowMovedToWorkspace(windowId, index);
    }

    function dropWindow(windowId) {
        if (windowId !== draggingWindowId)
            return;
        dropWindowOnWorkspace(dragWorkspaceIndex);
    }

    function cancelWindowDrag() {
        draggingWindowId = "";
        dragWorkspaceIndex = -1;
        dragScenePos = Qt.point(0, 0);
    }

    // A faint scrim so the strips read against any wallpaper. The compositor
    // still draws the live surfaces underneath; this only deepens contrast.
    Rectangle {
        objectName: "scrim"
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: Theme.controls.overview.scrimOpacity * root.progress
    }

    // --- workspace strip --------------------------------------------------
    Row {
        id: workspaceStrip
        objectName: "workspaceStrip"
        anchors.top: parent.top
        anchors.topMargin: Theme.controls.menuBar.height
                          + Theme.controls.overview.stripMargin
                          - Theme.controls.overview.stripMargin * (1.0 - root.progress)
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Theme.controls.overview.stripGap
        // Fade in with the shared progress curve.
        opacity: Math.min(1.0, root.progress * 1.5)

        Repeater {
            model: root.workspaces

            delegate: Rectangle {
                id: spaceCard
                required property var modelData
                required property int index

                objectName: "spaceCard"
                readonly property bool spaceActive: modelData.active === true
                readonly property bool fullscreen: modelData.fullscreen === true
                readonly property bool dropTarget: root.dragWorkspaceIndex === modelData.index

                width: Theme.controls.overview.cardWidth
                height: Theme.controls.overview.cardHeight
                radius: Theme.controls.overview.cardRadius
                color: spaceActive ? Theme.color.accentMuted : Theme.color.chrome
                border.width: (spaceActive || dropTarget) ? 2 : 1
                border.color: (spaceActive || dropTarget) ? Theme.color.accent : Theme.color.border

                Accessible.role: Accessible.ListItem
                Accessible.name: fullscreen
                    ? qsTr("Fullscreen space %1").arg(modelData.name)
                    : qsTr("Space %1").arg(modelData.name)
                Accessible.focusable: true
                Accessible.selected: spaceActive

                Text {
                    id: spaceName
                    objectName: "spaceName"
                    anchors.centerIn: parent
                    width: parent.width - Theme.controls.overview.cardPadding * 2
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                    text: modelData.name
                    color: spaceCard.spaceActive
                           ? Theme.color.accentContent : Theme.color.textPrimary
                    font.pixelSize: Theme.controls.overview.fontSize
                    font.weight: spaceCard.spaceActive ? Font.DemiBold : Font.Normal
                }

                // A fullscreen window owns a dedicated Space (T-05 FR-3); mark
                // it so the strip reads differently while it exists.
                Rectangle {
                    objectName: "fullscreenBadge"
                    visible: spaceCard.fullscreen
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.margins: Theme.controls.overview.cardPadding
                    width: height
                    height: Theme.controls.overview.fontSize
                    radius: height / 2
                    color: Theme.color.textPrimary
                    opacity: 0.7
                }

                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    onTapped: root.workspaceActivated(spaceCard.modelData.index)
                }
            }
        }
    }

    // --- window grid (FR-7) -----------------------------------------------
    // One card per visible window, draggable onto a Space card to move the
    // window between Spaces. The card is a title/app label only: the
    // compositor keeps rendering the live surface underneath, so this is
    // chrome, not a thumbnail substitution.
    Flow {
        id: windowGrid
        objectName: "windowGrid"
        anchors.centerIn: parent
        width: Math.min(parent.width - Theme.controls.overview.stripMargin * 2,
                        Theme.controls.overview.cardWidth * 4
                        + Theme.controls.overview.stripGap * 3)
        spacing: Theme.controls.overview.stripGap
        opacity: Math.min(1.0, root.progress * 1.5)

        Repeater {
            model: root.windows

            delegate: Rectangle {
                id: windowCard
                required property var modelData
                required property int index

                objectName: "windowCard"
                readonly property string windowId: String(modelData.windowId)
                readonly property bool beingDragged:
                    root.draggingWindowId === windowId

                width: Theme.controls.overview.cardWidth
                height: Theme.controls.overview.cardHeight
                radius: Theme.controls.overview.cardRadius
                color: Theme.color.chrome
                border.width: modelData.focused === true ? 2 : 1
                border.color: modelData.focused === true
                              ? Theme.color.accent : Theme.color.border
                opacity: beingDragged ? 0.6 : 1.0
                scale: beingDragged ? 1.05 : 1.0

                Accessible.role: Accessible.ListItem
                Accessible.name: qsTr("Window %1").arg(modelData.title)
                Accessible.focusable: true

                Column {
                    anchors.centerIn: parent
                    width: parent.width - Theme.controls.overview.cardPadding * 2
                    spacing: 2

                    Text {
                        objectName: "windowTitle"
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        text: modelData.title.length > 0
                              ? modelData.title : modelData.appId
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.overview.fontSize
                    }

                    Text {
                        objectName: "windowSpace"
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        text: modelData.workspaceName
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.overview.titleSize
                    }
                }

                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    onTapped: root.windowActivated(windowCard.windowId)
                }

                DragHandler {
                    id: windowDragHandler
                    objectName: "windowDragHandler"
                    acceptedButtons: Qt.LeftButton
                    dragThreshold: 8

                    onActiveChanged: {
                        var id = windowCard.windowId;
                        if (active) {
                            var p = centroid.scenePosition;
                            root.beginWindowDrag(id);
                            root.updateWindowDrag(id, p.x, p.y);
                        } else {
                            root.dropWindow(id);
                        }
                    }
                    onCentroidChanged: {
                        if (active) {
                            var p = centroid.scenePosition;
                            root.updateWindowDrag(windowCard.windowId, p.x, p.y);
                        }
                    }
                }
            }
        }
    }

    // --- minimized-window bottom strip (FR-6) -----------------------------
    Row {
        id: minimizedStrip
        objectName: "minimizedStrip"
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.controls.overview.stripMargin
                              + Theme.controls.overview.stripMargin * (1.0 - root.progress)
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Theme.controls.overview.chipGap
        opacity: Math.min(1.0, root.progress * 1.5)

        Repeater {
            model: root.minimizedWindows

            delegate: Rectangle {
                id: windowChip
                required property var modelData
                required property int index

                objectName: "windowChip"
                height: Theme.controls.overview.chipHeight
                width: Math.max(120, chipLabel.implicitWidth
                                     + Theme.controls.overview.chipPadding * 2)
                radius: Theme.controls.overview.chipRadius
                color: Theme.color.chrome
                border.width: 1
                border.color: Theme.color.border

                Accessible.role: Accessible.ListItem
                Accessible.name: qsTr("Restore %1").arg(modelData.title)
                Accessible.focusable: true

                Text {
                    id: chipLabel
                    objectName: "chipLabel"
                    anchors.centerIn: parent
                    width: parent.width - Theme.controls.overview.chipPadding * 2
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                    text: modelData.title.length > 0 ? modelData.title : modelData.appId
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.overview.titleSize
                }

                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    onTapped: root.windowActivated(String(windowChip.modelData.windowId))
                }
            }
        }
    }
}
