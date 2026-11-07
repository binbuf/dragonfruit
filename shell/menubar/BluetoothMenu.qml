// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Bluetooth status menu (T202): the adapter's power state and known
// devices, drawn in the design-system status popup. The model is the bridge
// host's `bluetooth` view (services/system-status). The popover never talks to
// BlueZ; it raises the same explicit actions as the Control Center tile and the
// shell forwards them to the existing bridge-host slots.
//
// Absence and read-only are the adapter's contract: an absent daemon hides the
// slot in the bar, an error view is visible but inert, and a future `readOnly`
// view disables every write while keeping the live state readable.
Item {
    id: root

    property var model: ({})
    property Item anchorItem: null
    property alias popup: popup
    property alias open: popup.open
    // T-21.2c/T202: the menu bar pushes the compositor's Popup role state and
    // the shared status-menu preset here.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    property string materialPreset: "statusMenu"
    property string materialBackdrop: "trusted"
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    readonly property bool available: model.state === "available"
    readonly property bool present: model.present !== false
    readonly property bool readOnly: model.readOnly === true || !root.available
    readonly property bool powered: model.powered === true
    readonly property bool discovering: model.discovering === true
    readonly property string label: model.label !== undefined && model.label !== ""
                                  ? model.label
                                  : (root.powered ? qsTr("Bluetooth on")
                                                  : qsTr("Bluetooth off"))
    readonly property var knownDevices:
        model.knownDevices !== undefined ? model.knownDevices : []
    readonly property bool writable: root.available && root.present && !root.readOnly

    signal powerToggleRequested(bool enabled)
    signal deviceToggled(string address, bool connected)
    signal settingsRequested()
    signal refreshRequested()
    signal closed()

    function togglePower() {
        if (!root.writable)
            return;
        root.powerToggleRequested(!root.powered);
    }

    function toggleDevice(address) {
        if (!root.writable || !address)
            return;
        for (var i = 0; i < root.knownDevices.length; ++i) {
            var device = root.knownDevices[i];
            if (device && device.address === address) {
                root.deviceToggled(address, device.connected !== true);
                return;
            }
        }
    }

    Popup {
        id: popup
        objectName: "bluetoothPopup"
        anchorItem: root.anchorItem
        preferredWidth: 260
        accessibleRole: Accessible.PopupMenu
        accessibleName: qsTr("Bluetooth")
        escapeCloses: true
        motionPreset: "menu"
        materialPreset: root.materialPreset
        materialBackdrop: root.materialBackdrop
        materialBlurActive: root.materialBlurActive
        materialTier: root.materialTier
        materialFallback: root.materialFallback

        onOpened: {
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: root.closed()

        Keys.onReturnPressed: (event) => {
            root.togglePower();
            event.accepted = true;
        }

        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Item {
                width: parent.width
                height: Theme.controls.button.fontSize + 4

                Text {
                    text: qsTr("Bluetooth")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    font.weight: Theme.primitive.font.weightMedium
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    objectName: "bluetoothStatus"
                    text: root.label
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    elide: Text.ElideRight
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, parent.width / 2)
                }
            }

            Text {
                visible: !root.available
                width: parent.width
                text: qsTr("Bluetooth unavailable")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                visible: root.available && root.readOnly
                width: parent.width
                text: model.note !== undefined && model.note.length > 0
                      ? model.note
                      : qsTr("Read-only: changes need authorization")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Button {
                objectName: "bluetoothPowerButton"
                width: parent.width
                text: root.powered ? qsTr("Turn Bluetooth Off")
                                   : qsTr("Turn Bluetooth On")
                variant: "secondary"
                enabled: root.writable
                onClicked: root.togglePower()
            }

            Rectangle {
                visible: root.available && root.knownDevices.length > 0
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Repeater {
                objectName: "bluetoothRows"
                model: root.knownDevices

                delegate: Item {
                    required property var modelData
                    objectName: "bluetoothRow-" + modelData.address
                    readonly property bool connected: modelData.connected === true
                    width: parent.width
                    height: Theme.controls.popup.rowHeight
                    opacity: root.writable ? 1.0 : 0.5

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: rowHover.hovered ? Theme.color.controlHover
                                                : "transparent"
                        antialiasing: true
                    }

                    Text {
                        id: deviceName
                        text: modelData.name !== undefined && modelData.name !== ""
                              ? modelData.name : modelData.address
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.xs
                        anchors.right: deviceAction.left
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Text {
                        id: deviceAction
                        objectName: "bluetoothDeviceAction"
                        text: parent.connected ? qsTr("Disconnect") : qsTr("Connect")
                        color: parent.connected ? Theme.color.accent
                                                : Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.primitive.spacing.xs
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    HoverHandler { id: rowHover }

                    TapHandler {
                        onTapped: root.toggleDevice(modelData.address)
                    }

                    Accessible.role: Accessible.MenuItem
                    Accessible.name: (modelData.name !== undefined
                                      && modelData.name !== ""
                                      ? modelData.name : modelData.address)
                                     + (parent.connected ? qsTr(", connected")
                                                         : qsTr(", disconnected"))
                    Accessible.onPressAction: root.toggleDevice(modelData.address)
                }
            }

            Button {
                objectName: "bluetoothSettingsButton"
                width: parent.width
                text: qsTr("Bluetooth Settings…")
                variant: "secondary"
                onClicked: root.settingsRequested()
            }
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: qsTr("Bluetooth")
}
