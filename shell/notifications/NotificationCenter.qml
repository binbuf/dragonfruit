// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The notification-center history (T-11.1a).
//
// The service owns the history and the shell decodes it into `history` (the
// same maps the banner uses, plus `closedAt` and `reason`). This component
// renders that list; the panel chrome and the menu-bar entry point that maps
// it are T-11.2b/T-11.3a, so it starts unmapped. Reduced motion is handled by
// the shared Theme binding, so no motion here depends on it.
Item {
    id: root

    // The decoded `History()` view, most recent first.
    property var history: []

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.window.radius
        color: Theme.color.chrome
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        Column {
            anchors.fill: parent
            anchors.margins: Theme.controls.popup.padding
            spacing: Theme.controls.popover.padding

            Text {
                id: emptyText
                width: parent.width
                objectName: "emptyHistory"
                visible: root.history.length === 0
                text: qsTr("No notifications")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeMd
                horizontalAlignment: Text.AlignHCenter
            }

            ListView {
                id: list
                objectName: "historyList"
                width: parent.width
                height: parent.height - (emptyText.visible ? emptyText.height : 0)
                clip: true
                model: root.history
                spacing: Theme.primitive.spacing.xs

                delegate: Rectangle {
                    required property var modelData
                    width: list.width
                    height: rowContent.implicitHeight + 2 * Theme.primitive.spacing.sm
                    radius: Theme.controls.sidebar.rowRadius
                    color: Theme.color.surfaceMuted

                    Column {
                        id: rowContent
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            width: parent.width
                            text: modelData.appName
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeXs
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            objectName: "historySummary"
                            text: modelData.summary
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeMd
                            font.weight: Theme.primitive.font.weightSemibold
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            visible: modelData.body.length > 0
                            text: modelData.body
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            wrapMode: Text.WordWrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Notifications")
}