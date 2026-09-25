// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files path bar (T-10.4a): a footer breadcrumb, root to current
// location, with chevrons between the segments. Segments are history anchors
// (clicking one navigates); drop targets land with the views (T-10.4b).
// Segments and chevrons are design-system components.
Item {
    id: root

    // [{ label, uri }] from the platform seam (Files.breadcrumb).
    property var model: []

    signal navigated(string uri)

    implicitHeight: Theme.controls.button.height + 2 * Theme.primitive.spacing.xs

    Rectangle {
        anchors.fill: parent
        color: Theme.color.chrome
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.controls.window.borderWidth
        color: Theme.color.border
    }

    Row {
        id: crumbs
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.toolbar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xxs

        Repeater {
            model: root.model

            delegate: Row {
                id: segment
                required property var modelData
                required property int index

                spacing: Theme.primitive.spacing.xxs

                Button {
                    text: segment.modelData.label
                    variant: "ghost"
                    onClicked: root.navigated(segment.modelData.uri)
                    Accessible.name: qsTr("Go to %1").arg(segment.modelData.label)
                }

                Icon {
                    visible: segment.index < root.model.length - 1
                    name: "chevron-right"
                    size: Theme.controls.searchField.iconSize
                    color: Theme.color.textTertiary
                    anchors.verticalCenter: parent.verticalCenter
                }
            }
        }
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Path bar")
}