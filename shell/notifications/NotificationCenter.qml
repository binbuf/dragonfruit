// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The notification-center history (T-11.1a; mapped by T204).
//
// The service owns the history, expiry, Focus/DND and banner policy. The shell
// decodes `History()` once (`NotificationCenter` in shell/src) into stable-keyed
// rows: `key`, `active`, `canAct`, `status` and the filtered `actions`. This
// component is only the view. It renders history rows, exposes keyboard
// traversal (Up/Down/Home/End/Return/Delete) and raises the action/dismissal
// requests the shell forwards through the existing notification service
// contract.
//
// History-only rows (expired, closed, dismissed or DND-suppressed) keep their
// text and stable identity but never show actions: `actions` is empty and
// `canAct` is false in the model.
Item {
    id: root

    // Standalone/test default; the shell sets the live surface size through
    // `prepareChromeScene`.
    width: 360
    height: 620

    // The normalized `History()` view, most recent first.
    property var history: []
    // The model's selected row. Kept in C++ so it follows the stable key when
    // the service replaces or reorders history.
    property int selectedIndex: -1
    // False while the notification service is absent. The Center stays open
    // with an accessible unavailable state and repopulates on reconnect.
    property bool available: true
    // T205 owns the notification-stack calibration; until then the Center
    // uses the shared Popup role complement.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)
    readonly property bool hasHistory: history.length > 0
    readonly property string emptyText: available
        ? qsTr("No notifications")
        : qsTr("Notifications unavailable")

    signal closed()
    signal moveRequested(int delta)
    signal selectRequested(int index)
    signal activateRequested()
    signal dismissSelectedRequested()
    signal actionInvoked(int id, string key)
    signal dismissEntryRequested(int id)

    focus: true
    Accessible.role: Accessible.List
    Accessible.name: qsTr("Notifications")
    Accessible.focusable: true

    Keys.onEscapePressed: (event) => {
        root.closed();
        event.accepted = true;
    }
    Keys.onUpPressed: (event) => {
        root.moveRequested(-1);
        event.accepted = true;
    }
    Keys.onDownPressed: (event) => {
        root.moveRequested(1);
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.activateRequested();
        event.accepted = true;
    }
    Keys.onEnterPressed: (event) => {
        root.activateRequested();
        event.accepted = true;
    }
    Keys.onDeletePressed: (event) => {
        root.dismissSelectedRequested();
        event.accepted = true;
    }
    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Home) {
            root.selectRequested(0);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.selectRequested(Math.max(0, root.history.length - 1));
            event.accepted = true;
        } else if (event.key === Qt.Key_Backspace) {
            root.dismissSelectedRequested();
            event.accepted = true;
        }
    }

    Shadow {
        anchors.fill: plate
        radius: plate.radius
        level: "overlay"
    }

    Rectangle {
        id: plate
        objectName: "notificationCenterPlate"
        anchors.fill: parent
        radius: Theme.controls.window.radius
        color: Material.popupFill(root.materialBlurActive)
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        Column {
            id: layout
            anchors.fill: parent
            anchors.margins: Theme.controls.popup.padding
            spacing: Theme.controls.popover.padding

            Text {
                id: titleText
                width: parent.width
                objectName: "centerTitle"
                text: qsTr("Notifications")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            Text {
                id: emptyText
                width: parent.width
                objectName: "emptyHistory"
                visible: !root.hasHistory
                text: root.emptyText
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeMd
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                Accessible.role: Accessible.StaticText
                Accessible.name: root.emptyText
            }

            ListView {
                id: list
                objectName: "historyList"
                width: parent.width
                height: parent.height - titleText.height
                        - (emptyText.visible ? emptyText.height + parent.spacing : 0)
                        - parent.spacing
                clip: true
                model: root.history
                spacing: Theme.primitive.spacing.xs
                boundsBehavior: Flickable.StopAtBounds

                onCountChanged: {
                    if (root.selectedIndex >= 0)
                        positionViewAtIndex(root.selectedIndex, ListView.Contain)
                }

                delegate: Rectangle {
                    id: row
                    required property var modelData
                    required property int index
                    readonly property bool selected: index === root.selectedIndex
                    readonly property bool active: modelData.active === true
                    readonly property bool canAct: modelData.canAct === true
                    readonly property var actions: modelData.actions || []
                    readonly property string statusText: {
                        if (modelData.status === "suppressed")
                            return qsTr("Suppressed by Focus");
                        if (modelData.status === "expired")
                            return qsTr("Expired");
                        if (modelData.status === "dismissed")
                            return qsTr("Dismissed");
                        if (modelData.status === "closed" || modelData.status === "undefined")
                            return qsTr("Closed");
                        return "";
                    }

                    objectName: "history_" + modelData.key
                    width: list.width
                    height: rowContent.implicitHeight + 2 * Theme.primitive.spacing.sm
                    radius: Theme.controls.sidebar.rowRadius
                    color: selected ? Theme.color.surfaceElevated : Theme.color.surfaceMuted
                    border.width: selected ? Theme.controls.window.borderWidth : 0
                    border.color: Theme.color.accent
                    opacity: active ? 1.0 : 0.78

                    Accessible.role: Accessible.ListItem
                    Accessible.name: modelData.summary
                    Accessible.description: {
                        var parts = [];
                        if (modelData.body)
                            parts.push(modelData.body);
                        if (row.statusText.length > 0)
                            parts.push(row.statusText);
                        return parts.join(". ");
                    }

                    MouseArea {
                        anchors.fill: parent
                        // Keep the row selector below the action/dismiss
                        // targets; otherwise the full-row click area would
                        // swallow their presses.
                        z: -1
                        onClicked: root.selectRequested(row.index)
                    }

                    Column {
                        id: rowContent
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        spacing: Theme.primitive.spacing.xxs

                        Row {
                            width: parent.width
                            spacing: Theme.primitive.spacing.sm

                            Text {
                                width: parent.width - (statusLabel.visible
                                    ? statusLabel.width + parent.spacing : 0)
                                text: modelData.appName
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeXs
                                elide: Text.ElideRight
                            }
                            Text {
                                id: statusLabel
                                objectName: "historyStatus"
                                visible: row.statusText.length > 0
                                text: row.statusText
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeXs
                                elide: Text.ElideRight
                            }
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

                        Row {
                            id: actionRow
                            objectName: "historyActions"
                            visible: row.active && row.actions.length > 0
                            width: parent.width
                            height: visible ? 26 : 0
                            spacing: Theme.primitive.spacing.sm

                            Repeater {
                                model: row.actions
                                delegate: Rectangle {
                                    required property var modelData
                                    objectName: "historyAction_" + modelData.key
                                    height: actionRow.height
                                    width: actionLabel.implicitWidth
                                           + 2 * Theme.primitive.spacing.md
                                    radius: Theme.controls.sidebar.rowRadius
                                    color: actionArea.containsMouse
                                           ? Theme.color.surfaceElevated : "transparent"
                                    border.width: Theme.controls.window.borderWidth
                                    border.color: Theme.color.border

                                    Accessible.role: Accessible.Button
                                    Accessible.name: modelData.label

                                    Text {
                                        id: actionLabel
                                        anchors.centerIn: parent
                                        text: modelData.label
                                        color: Theme.color.accent
                                        font.pixelSize: Theme.primitive.font.sizeSm
                                        font.weight: Theme.primitive.font.weightMedium
                                    }

                                    MouseArea {
                                        id: actionArea
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        onClicked: root.actionInvoked(row.modelData.id,
                                                                      modelData.key)
                                    }
                                }
                            }
                        }

                        Rectangle {
                            objectName: "historyDismiss"
                            visible: row.active
                            height: visible ? 24 : 0
                            width: dismissLabel.implicitWidth
                                   + 2 * Theme.primitive.spacing.sm
                            radius: Theme.controls.sidebar.rowRadius
                            color: dismissArea.containsMouse
                                   ? Theme.color.surfaceElevated : "transparent"

                            Accessible.role: Accessible.Button
                            Accessible.name: qsTr("Dismiss notification")

                            Text {
                                id: dismissLabel
                                anchors.centerIn: parent
                                text: qsTr("Dismiss")
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeXs
                            }

                            MouseArea {
                                id: dismissArea
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: root.dismissEntryRequested(row.modelData.id)
                            }
                        }
                    }
                }
            }
        }
    }

    onSelectedIndexChanged: {
        if (selectedIndex >= 0 && list.count > 0)
            list.positionViewAtIndex(selectedIndex, ListView.Contain)
    }
}
