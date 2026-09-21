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
//     fullscreen windows, FR-10), clickable to activate a Space;
//   * the minimized-window bottom strip (FR-6), clickable to restore;
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
    property real progress: 0
    property bool active: false

    signal workspaceActivated(int index)
    signal windowActivated(string windowId)
    signal dismissRequested()

    focus: root.active

    Keys.onEscapePressed: root.dismissRequested()

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

                width: Theme.controls.overview.cardWidth
                height: Theme.controls.overview.cardHeight
                radius: Theme.controls.overview.cardRadius
                color: spaceActive ? Theme.color.accentMuted : Theme.color.chrome
                border.width: spaceActive ? 2 : 1
                border.color: spaceActive ? Theme.color.accent : Theme.color.border

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
