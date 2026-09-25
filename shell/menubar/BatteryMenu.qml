// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The read-only battery status menu (T-07.5b): the charge state, level, and
// time remaining for the system battery, drawn in the design-system popup.
// The model is the bridge host's `battery` view (services/system-status); the
// popover never writes and only raises `refreshRequested`, which the shell
// forwards to the host.
Item {
    id: root

    // The host's decoded battery view ({state, present, glyph, label, ...}).
    property var model: ({})
    property Item anchorItem: null
    property alias popup: popup
    property alias open: popup.open

    readonly property bool available: model.state === "available"
    readonly property bool present: model.present === true
    readonly property bool charging: model.charging === true
    readonly property bool onBattery: model.onBattery === true
    readonly property int percent: model.percent !== undefined ? model.percent : 0
    readonly property real level: model.level !== undefined ? model.level : 0.0
    readonly property string label: model.label !== undefined ? model.label : ""

    signal refreshRequested()
    // Raised when the popup closes for any reason, for the bar's state
    // (T-07.5b keyboard a11y).
    signal closed()

    // A compact "H:MM" duration for a non-zero second count.
    function formatDuration(seconds) {
        if (seconds === undefined || seconds === null || seconds <= 0)
            return "";
        var totalMinutes = Math.floor(seconds / 60);
        var hours = Math.floor(totalMinutes / 60);
        var minutes = totalMinutes % 60;
        if (hours <= 0)
            return qsTr("%1 min").arg(minutes);
        return qsTr("%1:%2").arg(hours).arg(minutes < 10 ? "0" + minutes : minutes);
    }

    readonly property string remaining: {
        if (root.charging)
            return root.formatDuration(root.model.timeToFull);
        if (root.onBattery)
            return root.formatDuration(root.model.timeToEmpty);
        return "";
    }

    Popup {
        id: popup
        objectName: "batteryPopup"
        anchorItem: root.anchorItem
        preferredWidth: 240
        accessibleRole: Accessible.PopupMenu
        accessibleName: qsTr("Battery")
        escapeCloses: true

        onOpened: {
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: root.closed()

        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Item {
                width: parent.width
                height: Theme.controls.button.fontSize + 4

                Text {
                    text: qsTr("Battery")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    font.weight: Theme.primitive.font.weightMedium
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    visible: root.remaining.length > 0
                    text: root.charging
                          ? qsTr("%1 until full").arg(root.remaining)
                          : qsTr("%1 left").arg(root.remaining)
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            // The glyph carries the fill; the percentage is the label.
            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                StatusGlyph {
                    name: root.charging ? "battery-charging" : "battery"
                    size: Theme.controls.menuBar.iconSize + 8
                    level: root.level
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    objectName: "batteryPercent"
                    text: root.label.length > 0 ? root.label
                                                : qsTr("%1%").arg(root.percent)
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            Text {
                width: parent.width
                text: root.charging ? qsTr("Charging")
                                    : (root.onBattery ? qsTr("On battery")
                                                      : qsTr("Plugged in"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: qsTr("Battery")
}