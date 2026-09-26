// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The design-system ScreenCast source picker (T-13.4a): the visual half of the
// portal presenter seam. `ShellController` drives every property from the
// `ScreenCastBridge` (which owns the request and returns the chosen source
// handle to the portal); this view is pure and never calls D-Bus.
//
// The available sources are the compositor's monitor/window projection, handed
// in by the controller. A row toggles its selection; `multiple` decides whether
// more than one may be checked. Share accepts, Cancel/Escape declines.
Item {
    id: root

    // One selection map per source: {id, kind, label, detail, selected}.
    // Monitors (`kind == "monitor"`) sort before windows (`kind == "window"`).
    property var sources: []
    // Whether the request allows more than one source.
    property bool multiple: false
    // The requesting application's id (shown in the subtitle when non-empty).
    property string appId: ""
    // A short message under the list (no selection, a failed request).
    property string errorText: ""

    readonly property int selectedCount: {
        var count = 0;
        for (var i = 0; i < root.sources.length; ++i) {
            if (root.sources[i].selected)
                ++count;
        }
        return count;
    }
    readonly property bool canAccept: root.selectedCount > 0
    readonly property bool hasMonitors: {
        for (var i = 0; i < root.sources.length; ++i) {
            if (root.sources[i].kind === "monitor")
                return true;
        }
        return false;
    }
    readonly property bool hasWindows: {
        for (var i = 0; i < root.sources.length; ++i) {
            if (root.sources[i].kind === "window")
                return true;
        }
        return false;
    }

    function activate() {
        if (root.canAccept)
            root.accepted();
    }

    signal sourceToggled(string id)
    signal accepted()
    signal cancelled()

    Accessible.role: Accessible.Dialog
    Accessible.name: qsTr("Share your screen")

    // The subtle full-bleed scrim; the surface is exactly the card's size.
    Rectangle {
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: 0.35
    }

    Shadow {
        anchors.fill: card
        radius: card.radius
        level: "overlay"
    }

    Rectangle {
        id: card
        objectName: "screencastCard"
        anchors.centerIn: parent
        width: parent.width
        height: parent.height
        radius: Theme.controls.window.radius
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Column {
            id: body
            anchors.fill: parent
            anchors.margins: Theme.primitive.spacing.lg
            spacing: Theme.primitive.spacing.md

            Text {
                id: heading
                objectName: "screencastTitle"
                width: parent.width
                text: qsTr("Share your screen")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            Text {
                id: subtitle
                objectName: "screencastSubtitle"
                width: parent.width
                visible: root.appId.length > 0
                text: qsTr("%1 wants to record a screen or window. Choose a source to share.")
                          .arg(root.appId)
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            // The source list, grouped by kind (sections need the model sorted
            // monitors-first, which the controller guarantees).
            Rectangle {
                id: listFrame
                width: parent.width
                height: parent.height - y - statusText.height - footer.height
                        - 2 * Theme.primitive.spacing.md
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.separator
                clip: true

                ListView {
                    id: sourceList
                    objectName: "screencastList"
                    anchors.fill: parent
                    anchors.margins: Theme.primitive.spacing.xs
                    clip: true
                    model: root.sources
                    boundsBehavior: Flickable.StopAtBounds
                    activeFocusOnTab: true
                    keyNavigationEnabled: true
                    section.property: "kind"
                    section.delegate: Text {
                        objectName: "screencastSection"
                        width: sourceList.width
                        height: 26
                        leftPadding: Theme.primitive.spacing.sm
                        verticalAlignment: Text.AlignVCenter
                        text: section === "monitor" ? qsTr("Screens") : qsTr("Windows")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.weight: Theme.primitive.font.weightMedium
                    }

                    delegate: Rectangle {
                        id: row
                        objectName: "screencastRow"
                        required property int index
                        required property var modelData
                        width: sourceList.width
                        height: 40
                        radius: Theme.primitive.radius.sm
                        color: row.modelData.selected ? Theme.color.selection
                             : (rowHover.hovered ? Theme.color.controlHover : "transparent")
                        antialiasing: true

                        Row {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.leftMargin: Theme.primitive.spacing.sm
                            anchors.rightMargin: Theme.primitive.spacing.sm
                            spacing: Theme.primitive.spacing.sm

                            Icon {
                                anchors.verticalCenter: parent.verticalCenter
                                name: row.modelData.kind === "monitor" ? "displays" : "file"
                                size: 18
                                color: row.modelData.selected ? Theme.color.accentContent
                                                              : Theme.color.textSecondary
                            }

                            Column {
                                anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - 18 - parent.spacing - indicator.width
                                        - parent.spacing
                                spacing: 0

                                Text {
                                    objectName: "screencastLabel"
                                    width: parent.width
                                    text: row.modelData.label
                                    color: row.modelData.selected ? Theme.color.accentContent
                                                                  : Theme.color.textPrimary
                                    font.pixelSize: Theme.controls.button.fontSize
                                    elide: Text.ElideRight
                                }
                                Text {
                                    width: parent.width
                                    visible: (row.modelData.detail || "").length > 0
                                    text: row.modelData.detail || ""
                                    color: row.modelData.selected ? Theme.color.accentContent
                                                                  : Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    elide: Text.ElideRight
                                }
                            }

                            // The selection indicator: a check when checked and a
                            // hollow outline when not, so the row reads in both
                            // single- and multi-selection modes.
                            Rectangle {
                                id: indicator
                                objectName: "screencastIndicator"
                                anchors.verticalCenter: parent.verticalCenter
                                width: 20
                                height: 20
                                radius: root.multiple ? Theme.primitive.radius.xs : 10
                                color: row.modelData.selected ? Theme.color.accent
                                                              : "transparent"
                                border.width: 2
                                border.color: row.modelData.selected
                                              ? Theme.color.accent : Theme.color.border

                                Icon {
                                    anchors.centerIn: parent
                                    visible: row.modelData.selected
                                    name: "check"
                                    size: 12
                                    color: Theme.color.accentContent
                                }
                            }
                        }

                        HoverHandler {
                            id: rowHover
                        }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.sourceToggled(row.modelData.id)
                        }
                    }

                    Keys.onReturnPressed: (event) => {
                        if (sourceList.currentIndex >= 0
                                && sourceList.currentIndex < root.sources.length)
                            root.sourceToggled(root.sources[sourceList.currentIndex].id);
                        event.accepted = true;
                    }
                    Keys.onEnterPressed: (event) => {
                        root.activate();
                        event.accepted = true;
                    }
                }
            }

            Text {
                id: statusText
                objectName: "screencastError"
                width: parent.width
                height: root.errorText.length > 0 ? implicitHeight : 0
                visible: root.errorText.length > 0
                text: root.errorText
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                elide: Text.ElideRight
            }

            Row {
                id: footer
                width: parent.width
                height: Theme.controls.button.height
                spacing: Theme.controls.dialog.buttonGap

                Text {
                    id: hint
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.multiple ? qsTr("Choose one or more sources")
                                        : qsTr("Choose a source to share")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Item {
                    width: parent.width - hint.width - cancelButton.width
                            - acceptButton.width - 3 * footer.spacing
                    height: 1
                }

                Button {
                    id: cancelButton
                    objectName: "screencastCancel"
                    text: qsTr("Cancel")
                    variant: "secondary"
                    onClicked: root.cancelled()
                }

                Button {
                    id: acceptButton
                    objectName: "screencastAccept"
                    text: qsTr("Share")
                    variant: "primary"
                    enabled: root.canAccept
                    onClicked: root.accepted()
                }
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.cancelled();
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.activate();
        event.accepted = true;
    }
}