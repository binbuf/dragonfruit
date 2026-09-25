// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// App-switcher overlay chrome (T-06.2a).
//
// The compositor owns the Cmd-Tab chord, the recency order, and the selection
// (T-06.1). It renders the **live** preview surfaces of the running apps
// through the T-04 scene transform; this shell surface is deliberately
// transparent and draws only the chrome the compositor does not own:
//
//   * a faint scrim so the cards read over any wallpaper;
//   * one card per app in recency order, with the selected card highlighted;
//   * the selected app's name, and an accessible name on every card, as the
//     icon/name fallback the design requires.
//
// The overlay is a pure projection: it never re-derives recency, never cycles,
// and never commits (T-06.2b wires the commit path). Keyboard-only,
// single-hand operation is the compositor-side chord; the cards only announce
// it to assistive technology.
Item {
    id: root

    // Injected by the shell controller from the private protocol.
    // `entries` is a list of `{ index, appId }` maps in recency order.
    property var entries: []
    property int selectedIndex: -1
    property int direction: 0
    property bool active: false

    // Reduced motion (T-11 FR-9 / U-6): appear instantly, with no fade or
    // scale. The compositor half single-steps its transitions; this keeps the
    // chrome from interpolating either.
    readonly property bool reducedMotion: Theme.reducedMotion
    property real shown: 0
    readonly property real reveal: root.reducedMotion
        ? (root.active ? 1.0 : 0.0)
        : root.shown

    onActiveChanged: root.shown = root.active ? 1.0 : 0.0

    Behavior on shown {
        enabled: !root.reducedMotion
        NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
    }

    // The app id's last dot-segment is the readable fallback name when no
    // richer identity is available yet (T-14 supplies icons/names).
    function displayName(appId) {
        if (!appId)
            return ""
        var parts = String(appId).split(".")
        return parts.length > 1 ? parts[parts.length - 1] : appId
    }

    // The window/status label shown above the card row.
    readonly property string selectedAppId: (root.selectedIndex >= 0
            && root.selectedIndex < root.entries.length)
        ? String(root.entries[root.selectedIndex].appId) : ""

    // A faint scrim so the cards read against any wallpaper. The compositor
    // still draws the live preview surfaces underneath; this only deepens
    // contrast (the Mission Control pattern).
    Rectangle {
        objectName: "switcherScrim"
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: Theme.controls.overview.scrimOpacity * root.reveal
    }

    // The centered selected-app name, the accessible fallback for the live
    // preview above the cards.
    Text {
        id: selectedName
        objectName: "selectedName"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: cardRow.top
        anchors.bottomMargin: Theme.controls.overview.stripGap
        text: root.displayName(root.selectedAppId)
        color: Theme.color.textPrimary
        font.pixelSize: Theme.controls.overview.fontSize
        font.weight: Font.DemiBold
        opacity: Math.min(1.0, root.reveal * 1.5)
    }

    // One card per app in recency order, centered. The selected card is
    // highlighted; the compositor renders the matching live surface above.
    Row {
        id: cardRow
        objectName: "appCardRow"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.controls.overview.stripMargin
        spacing: Theme.controls.overview.stripGap
        opacity: Math.min(1.0, root.reveal * 1.5)

        Repeater {
            model: root.entries

            delegate: Rectangle {
                id: appCard
                required property var modelData
                required property int index

                objectName: "appCard"
                readonly property bool selected: index === root.selectedIndex
                readonly property string appId: String(modelData.appId)

                width: Theme.controls.overview.cardWidth
                height: Theme.controls.overview.cardHeight
                radius: Theme.controls.overview.cardRadius
                color: selected ? Theme.color.accentMuted : Theme.color.chrome
                border.width: selected ? 2 : 1
                border.color: selected ? Theme.color.accent : Theme.color.border

                Accessible.role: Accessible.ListItem
                Accessible.name: qsTr("%1, app switcher").arg(root.displayName(appId))
                Accessible.focusable: true
                Accessible.selected: selected

                Column {
                    anchors.centerIn: parent
                    width: parent.width - Theme.controls.overview.cardPadding * 2
                    spacing: 2

                    Text {
                        objectName: "appName"
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        text: root.displayName(appCard.appId)
                        color: appCard.selected
                               ? Theme.color.accentContent : Theme.color.textPrimary
                        font.pixelSize: Theme.controls.overview.fontSize
                        font.weight: appCard.selected ? Font.DemiBold : Font.Normal
                    }

                    Text {
                        objectName: "appId"
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideMiddle
                        text: appCard.appId
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.overview.titleSize
                    }
                }
            }
        }
    }
}