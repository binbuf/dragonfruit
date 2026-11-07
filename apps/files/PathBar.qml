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
    // T206: the path bar is unclassified chrome and stays opaque; the app
    // rounds only its bottom-right window corner.
    property color fillColor: Theme.color.chrome
    property color borderColor: Theme.color.border
    property real bottomRightRadius: 0

    signal navigated(string uri)

    implicitHeight: Theme.controls.button.height + 2 * Theme.primitive.spacing.xs

    ChromeSheet {
        anchors.fill: parent
        color: root.fillColor
        bottomRightRadius: root.bottomRightRadius
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.controls.window.borderWidth
        color: root.borderColor
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