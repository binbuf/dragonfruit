// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
pragma ComponentBehavior: Bound

// The Bluetooth status menu (T202, T232): the adapter's power state, the
// paired devices, discovery and nearby devices, and the Settings link, drawn
// in the design-system status popup. The model is the bridge host's
// `bluetooth` view (services/system-status). The popover never talks to BlueZ;
// it raises the same explicit actions as the Control Center tile and the shell
// forwards them to the existing bridge-host slots.
//
// Absence and read-only are the adapter's contract: an absent daemon hides the
// slot in the bar, an error view is visible but inert, and a `readOnly` view
// disables every write while keeping the live state readable.
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
    readonly property string errorMessage: model.error !== undefined
        ? String(model.error) : ""
    readonly property string label: model.label !== undefined && model.label !== ""
                                  ? model.label
                                  : (root.powered ? qsTr("Bluetooth on")
                                                  : qsTr("Bluetooth off"))
    readonly property var knownDevices:
        model.knownDevices !== undefined ? model.knownDevices : []
    readonly property var nearbyDevices:
        model.nearbyDevices !== undefined ? model.nearbyDevices : []
    readonly property bool writable: root.available && root.present && !root.readOnly

    // One unified action list: paired devices first, then nearby devices.
    readonly property var rows: {
        var out = [];
        for (var i = 0; i < root.knownDevices.length; ++i) {
            var known = root.knownDevices[i] || {};
            out.push({
                index: out.length,
                address: known.address !== undefined ? String(known.address) : "",
                name: known.name !== undefined ? String(known.name) : "",
                connected: known.connected === true,
                paired: true,
                signal: known.signal !== undefined ? Number(known.signal) : 0,
                section: "known"
            });
        }
        for (var j = 0; j < root.nearbyDevices.length; ++j) {
            var nearby = root.nearbyDevices[j] || {};
            var address = nearby.address !== undefined ? String(nearby.address) : "";
            if (address.length === 0)
                continue;
            var duplicate = false;
            for (var k = 0; k < out.length; ++k) {
                if (out[k].address === address) {
                    duplicate = true;
                    break;
                }
            }
            if (duplicate)
                continue;
            out.push({
                index: out.length,
                address: address,
                name: nearby.name !== undefined ? String(nearby.name) : "",
                connected: nearby.connected === true,
                paired: nearby.paired === true,
                signal: nearby.signal !== undefined ? Number(nearby.signal) : 0,
                section: "nearby"
            });
        }
        return out;
    }
    readonly property var knownRows: root.rows.filter(function(row) {
        return row.section === "known";
    })
    readonly property var nearbyRows: root.rows.filter(function(row) {
        return row.section === "nearby";
    })
    property int highlightIndex: -1

    signal powerToggleRequested(bool enabled)
    signal deviceToggled(string address, bool connected)
    signal pairRequested(string address)
    signal discoveringToggleRequested(bool discovering)
    signal settingsRequested()
    signal refreshRequested()
    signal closed()

    function togglePower() {
        if (!root.writable)
            return;
        root.powerToggleRequested(!root.powered);
    }

    function toggleDiscovery() {
        if (!root.writable)
            return;
        root.discoveringToggleRequested(!root.discovering);
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
        // A nearby device that is not paired needs pairing first.
        root.pairRequested(address);
    }

    function activateRow(index) {
        if (index < 0 || index >= root.rows.length || !root.writable)
            return;
        var row = root.rows[index];
        if (!row || row.address.length === 0)
            return;
        if (row.paired)
            root.deviceToggled(row.address, !row.connected);
        else
            root.pairRequested(row.address);
    }

    function moveHighlight(delta) {
        var n = root.rows.length;
        if (n <= 0 || !root.writable) {
            root.highlightIndex = -1;
            return;
        }
        var i = root.highlightIndex < 0 ? (delta > 0 ? -1 : 0) : root.highlightIndex;
        root.highlightIndex = ((i + delta) % n + n) % n;
    }

    function activateHighlight() {
        if (root.highlightIndex >= 0)
            root.activateRow(root.highlightIndex);
        else
            root.togglePower();
    }

    Popup {
        id: popup
        objectName: "bluetoothPopup"
        anchorItem: root.anchorItem
        preferredWidth: 300
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
            root.highlightIndex = -1;
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: {
            root.highlightIndex = -1;
            root.closed();
        }

        Keys.onUpPressed: (event) => {
            root.moveHighlight(-1);
            event.accepted = true;
        }
        Keys.onDownPressed: (event) => {
            root.moveHighlight(1);
            event.accepted = true;
        }
        Keys.onReturnPressed: (event) => {
            root.activateHighlight();
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
                    text: root.discovering ? qsTr("Discovering…") : root.label
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    elide: Text.ElideRight
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, parent.width / 2)
                }
            }

            Text {
                objectName: "bluetoothStateNote"
                visible: !root.available || root.model.state === "error"
                width: parent.width
                text: root.model.state === "error"
                      ? (root.errorMessage.length > 0 ? root.errorMessage
                                                      : qsTr("Bluetooth is unavailable"))
                      : qsTr("Bluetooth unavailable")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                visible: root.available && !root.present
                width: parent.width
                text: qsTr("No Bluetooth controller")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                visible: root.available && root.present && root.readOnly
                width: parent.width
                text: root.model.note !== undefined && root.model.note.length > 0
                      ? root.model.note
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
                visible: root.available && root.knownRows.length > 0
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Text {
                visible: root.available && root.knownRows.length > 0
                width: parent.width
                text: qsTr("My Devices")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Repeater {
                objectName: "bluetoothRows"
                model: root.knownRows

                delegate: bluetoothRowDelegate
            }

            Rectangle {
                visible: root.available && root.present
                         && (root.discovering || root.nearbyRows.length > 0)
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Text {
                visible: root.available && root.present
                         && (root.discovering || root.nearbyRows.length > 0)
                width: parent.width
                text: qsTr("Nearby Devices")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Text {
                objectName: "bluetoothScanningNote"
                visible: root.available && root.present && root.discovering
                         && root.nearbyRows.length === 0
                width: parent.width
                text: qsTr("Scanning…")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                objectName: "bluetoothNearbyRows"
                model: root.nearbyRows

                delegate: bluetoothRowDelegate
            }

            Button {
                objectName: "bluetoothDiscoveryButton"
                width: parent.width
                text: root.discovering ? qsTr("Stop Scanning")
                                       : qsTr("Scan for Devices")
                variant: "secondary"
                enabled: root.writable
                onClicked: root.toggleDiscovery()
            }

            Button {
                objectName: "bluetoothSettingsButton"
                width: parent.width
                text: qsTr("Bluetooth Settings…")
                variant: "secondary"
                accessibleName: qsTr("Open Bluetooth Settings")
                onClicked: root.settingsRequested()
            }
        }
    }

    Component {
        id: bluetoothRowDelegate

        Item {
            id: rowItem
            required property var modelData
            objectName: "bluetoothRow-" + rowItem.modelData.address
            readonly property bool connected: rowItem.modelData.connected === true
            readonly property bool paired: rowItem.modelData.paired === true
            width: parent.width
            height: Theme.controls.popup.rowHeight
            opacity: root.writable ? 1.0 : 0.5

            Rectangle {
                anchors.fill: parent
                radius: Theme.controls.focusRing.radius
                color: (rowHover.hovered || root.highlightIndex === rowItem.modelData.index)
                       ? Theme.color.controlHover : "transparent"
                antialiasing: true
            }

            Text {
                id: deviceName
                text: rowItem.modelData.name !== undefined && rowItem.modelData.name !== ""
                      ? rowItem.modelData.name : rowItem.modelData.address
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
                text: rowItem.connected ? qsTr("Disconnect")
                      : (rowItem.paired ? qsTr("Connect") : qsTr("Pair"))
                color: rowItem.connected ? Theme.color.accent
                                         : Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                anchors.right: parent.right
                anchors.rightMargin: Theme.primitive.spacing.xs
                anchors.verticalCenter: parent.verticalCenter
            }

            HoverHandler { id: rowHover }

            TapHandler {
                onTapped: root.activateRow(rowItem.modelData.index)
            }

            Accessible.role: Accessible.MenuItem
            Accessible.name: (rowItem.modelData.name !== undefined
                              && rowItem.modelData.name !== ""
                              ? rowItem.modelData.name : rowItem.modelData.address)
                             + (rowItem.connected ? qsTr(", connected")
                                                  : (rowItem.paired ? qsTr(", paired")
                                                                    : qsTr(", nearby")))
            Accessible.description: root.writable ? "" : qsTr("Read-only")
            Accessible.onPressAction: root.activateRow(rowItem.modelData.index)
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: qsTr("Bluetooth")
}
