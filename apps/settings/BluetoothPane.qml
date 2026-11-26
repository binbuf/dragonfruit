// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Bluetooth pane (T-15.1b; T271 completion; T288 visual parity).
//
// The pane and the Quick Settings tile are one functional unit: both read the
// same BlueZ adapter through the bridge host, and the explicit writes (power,
// discovery, pair, connect, forget) go through the `Settings` singleton to the
// adapter — never to D-Bus from QML. Opening the pane asks for a re-read and
// starts an inquiry while it is open; turning the radio on starts discovery
// too. Closing it stops the inquiry.
//
// T288 follows the mockup's section order
// (docs/design/25-system-settings-visual-parity.md §5.2): intro, the Bluetooth
// toggle with the discoverable note, My Devices, and Nearby Devices with a
// real searching row. The discoverable note appears only while the pane is
// open, the radio is on, and the adapter reports itself discoverable. The
// device info and Advanced sheets stay. Absence is structural: every group
// stays and only the host controls disable with a reason (ADR 0262).
Item {
    id: root

    // The bridge host's Bluetooth view (empty when absent).
    readonly property var view: Settings.bluetooth
    readonly property bool available: Settings.bluetoothAvailable
    readonly property string viewState: root.view.state !== undefined
        ? String(root.view.state) : ""
    readonly property bool present: root.view.present === true
    readonly property bool failed: root.available && root.viewState === "error"
    readonly property string errorMessage: root.view.error !== undefined
        ? String(root.view.error) : ""
    readonly property bool powered: root.view.powered === true
    readonly property bool discovering: root.view.discovering === true
    readonly property bool readOnly: root.view.readOnly === true
    readonly property string adapterName: root.view.adapterName !== undefined
        ? root.view.adapterName : ""
    readonly property string adapterAddress: root.view.adapterAddress !== undefined
        ? String(root.view.adapterAddress) : ""
    readonly property string computerName: root.view.computerName !== undefined
        && String(root.view.computerName).length > 0
        ? String(root.view.computerName)
        : (root.adapterName.length > 0 ? root.adapterName : qsTr("Computer"))
    readonly property var knownDevices: root.view.knownDevices !== undefined
        ? root.view.knownDevices : []
    readonly property var nearbyDevices: root.view.nearbyDevices !== undefined
        ? root.view.nearbyDevices : []

    // The two hide rules (ADR 0117): `bluetoothd` absent (not available) or a
    // running daemon with no controller (`present: false`).
    readonly property bool ready: root.available && root.present
        && root.viewState === "available"
    readonly property bool searching: root.powered && root.discovering

    readonly property string discoverableCaption:
        qsTr("This computer is discoverable as “%1” while Bluetooth Settings is open.")
            .arg(root.computerName)

    // T271: the last host write's live state; the controls bind to the view,
    // so a refused write reverts and the reason appears below the toggle.
    readonly property string writeMessage: {
        if (Settings.bluetoothWriteState === "denied")
            return Settings.bluetoothWriteError.length > 0
                ? qsTr("Authentication was denied: %1").arg(Settings.bluetoothWriteError)
                : qsTr("Authentication was denied.");
        if (Settings.bluetoothWriteState === "failed"
                || Settings.bluetoothWriteState === "absent")
            return Settings.bluetoothWriteError.length > 0
                ? Settings.bluetoothWriteError
                : qsTr("The Bluetooth change could not be applied.");
        return "";
    }

    // Test surface (used by tst_settings_bluetooth.qml).
    property alias intro: intro
    property alias powerGroup: powerGroup
    property alias powerToggle: powerToggle
    property alias powerSubtitle: powerRow.descriptionText
    property alias caption: discoverableRow
    property alias absenceNote: absenceNote
    property alias myDevicesGroup: myDevicesGroup
    property alias nearbyGroup: nearbyGroup
    property alias knownRepeater: knownRepeater
    property alias nearbyRepeater: nearbyRepeater
    property alias searchingRow: searchingRow
    property alias spinner: searchingRow.spinner
    property alias discoverableRow: discoverableRow
    property alias deviceDialog: deviceDialog
    property alias deviceTitle: deviceTitle
    property alias deviceAddress: deviceAddress
    property alias deviceType: deviceType
    property alias deviceBattery: deviceBattery
    property alias deviceConnectButton: deviceConnectButton
    property alias deviceForgetButton: deviceForgetButton
    property alias advancedRow: advancedRow
    property alias advancedDialog: advancedDialog
    property alias advancedAdapterName: advancedAdapterName
    property alias advancedAdapterAddress: advancedAdapterAddress
    property alias advancedDiscoverable: advancedDiscoverable
    property alias advancedPairable: advancedPairable
    property alias writeErrorItem: writeErrorItem
    property alias myDevicesEmpty: myDevicesEmpty
    property alias nearbyEmpty: nearbyEmpty

    property string detailsAddress: ""

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function togglePower() {
        if (!root.ready)
            return;
        root.setPowered(!root.powered);
    }

    function setPowered(on) {
        if (!root.ready)
            return;
        Settings.setBluetoothPowered(on);
        // T271: turning the radio on starts the inquiry while the pane is
        // open. BlueZ rejects discovery on an unpowered adapter, so the order
        // matters.
        if (on)
            Settings.setBluetoothDiscovering(true);
    }

    function toggleDevice(address, connected) {
        if (!root.ready || !address)
            return;
        Settings.setBluetoothConnected(address, !connected);
    }

    function deviceFor(address) {
        for (var i = 0; i < root.knownDevices.length; ++i)
            if (String(root.knownDevices[i].address) === String(address))
                return root.knownDevices[i];
        for (var j = 0; j < root.nearbyDevices.length; ++j)
            if (String(root.nearbyDevices[j].address) === String(address))
                return root.nearbyDevices[j];
        return null;
    }

    function showDeviceInfo(address) {
        if (!root.ready || !address)
            return;
        root.detailsAddress = String(address);
        deviceDialog.show();
    }

    function toggleDetailsDevice() {
        var device = root.deviceFor(root.detailsAddress);
        if (device === null)
            return;
        root.toggleDevice(root.detailsAddress, device.connected === true);
    }

    function forgetDetailsDevice() {
        if (root.detailsAddress.length === 0)
            return;
        Settings.forgetBluetooth(root.detailsAddress);
        deviceDialog.hide();
    }

    // A refused write leaves the view untouched; Toggle.toggle() overwrote
    // its binding, so re-read the view's power state.
    function revertControls() {
        if (Settings.bluetoothWriteState === "failed"
                || Settings.bluetoothWriteState === "denied"
                || Settings.bluetoothWriteState === "absent")
            powerToggle.checked = root.powered;
    }

    Connections {
        target: Settings
        function onBluetoothWriteChanged() { root.revertControls(); }
    }

    function batteryLabel(address) {
        var device = root.deviceFor(address);
        if (device === null || device.battery === undefined
                || device.battery === null || Number(device.battery) < 0)
            return qsTr("Not reported");
        return qsTr("%1%").arg(Number(device.battery));
    }

    function typeLabel(address) {
        var device = root.deviceFor(address);
        return device !== null && device.type !== undefined
            && String(device.type).length > 0
            ? String(device.type) : qsTr("Device");
    }

    // The mockup's device tile: the vendored glyph set has keyboard/mouse and
    // the Bluetooth mark, so audio devices use the Bluetooth tile.
    function deviceGlyph(type) {
        var value = String(type || "").toLowerCase();
        if (value.indexOf("keyboard") >= 0)
            return "keyboard";
        if (value.indexOf("mouse") >= 0)
            return "mouse";
        return "bluetooth";
    }

    // The pane asks the host for a re-read and starts an inquiry while it is
    // open. Both are no-ops when the host or BlueZ is absent.
    Component.onCompleted: {
        Settings.refreshBluetooth();
        if (root.ready && root.powered)
            Settings.setBluetoothDiscovering(true);
    }
    Component.onDestruction: {
        if (root.ready)
            Settings.setBluetoothDiscovering(false);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Intro (mockup §5.2) ───────────────────────────────────────────
        SettingsIntro {
            id: intro
            objectName: "bluetoothIntro"
            width: parent.width
            text: qsTr("Connect to accessories you can use for activities such "
                       + "as streaming music, typing, and gaming.")
        }

        // ── The Bluetooth toggle card ─────────────────────────────────────
        SettingsGroup {
            id: powerGroup
            width: parent.width

            SettingsRow {
                id: powerRow
                objectName: "bluetoothPowerRow"
                width: parent.width
                label: qsTr("Bluetooth")
                description: root.ready
                    ? (root.discovering ? qsTr("Discovering\u2026")
                                        : (root.powered ? qsTr("On") : qsTr("Off")))
                    : (root.failed ? qsTr("Unavailable") : qsTr("Off"))
                settingsId: "power"
                owner: "BlueZ"
                settingsKey: "Adapter1.Powered"
                // The discoverable note is part of the same card, matching the
                // mockup. It is a real row so search can land on it, and it is
                // only shown while the pane is open and the radio is on.
                showSeparator: root.ready && root.powered
                               && root.view.discoverable === true
                controlData: Toggle {
                    id: powerToggle
                    objectName: "bluetoothPowerToggle"
                    text: ""
                    accessibleName: qsTr("Bluetooth")
                    enabled: root.ready
                    onToggled: (checked) => root.setPowered(checked)

                    Binding {
                        target: powerToggle
                        property: "checked"
                        value: root.powered
                    }
                }
            }

            SettingsRow {
                id: discoverableRow
                objectName: "bluetoothDiscoverableRow"
                width: parent.width
                visible: root.ready && root.powered
                         && root.view.discoverable === true
                label: root.computerName
                description: root.discoverableCaption
                               + " " + qsTr("Other devices can find this computer "
                                            + "while this pane is open.")
                settingsId: "discoverable"
                owner: "BlueZ"
                settingsKey: "Adapter1.Discoverable"
                showSeparator: false
            }

            Text {
                id: writeErrorItem
                objectName: "bluetoothWriteError"
                width: parent.width
                visible: root.writeMessage.length > 0
                text: root.writeMessage
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "bluetoothAbsenceNote"
                visible: !root.ready
                kind: "disabled-with-reason"
                reason: root.failed
                    ? qsTr("BlueZ could not be read, so Bluetooth settings are "
                           + "temporarily unavailable.")
                    : qsTr("Bluetooth is not available on this system.")
            }
        }

        // ── My Devices ────────────────────────────────────────────────────
        SettingsGroup {
            id: myDevicesGroup
            width: parent.width
            title: qsTr("My Devices")

            Text {
                id: myDevicesEmpty
                objectName: "bluetoothNoDevices"
                width: parent.width
                visible: knownRepeater.count === 0
                text: root.ready ? qsTr("No devices yet.")
                                 : qsTr("Devices are unavailable while Bluetooth is off.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: knownRepeater
                model: root.ready ? root.knownDevices : []
                delegate: SettingsDeviceRow {
                    id: knownRow
                    required property var modelData
                    required property int index
                    objectName: "bluetoothDeviceRow"
                    width: parent.width
                    deviceGlyph: root.deviceGlyph(knownRow.modelData.type)
                    label: String(knownRow.modelData.name)
                    statusText: knownRow.modelData.connected
                        ? qsTr("Connected") : qsTr("Not Connected")
                    deviceActive: knownRow.modelData.connected === true
                    settingsId: "paired"
                    owner: "BlueZ"
                    settingsKey: "Device1"
                    showSeparator: knownRow.index < root.knownDevices.length - 1
                    controlData: Button {
                        objectName: "bluetoothInfoButton"
                        text: qsTr("Info…")
                        variant: "ghost"
                        accessibleName: qsTr("Info for %1")
                            .arg(knownRow.modelData.name)
                        onClicked: root.showDeviceInfo(knownRow.modelData.address)
                    }
                }
            }
        }

        // ── Nearby Devices ────────────────────────────────────────────────
        SettingsGroup {
            id: nearbyGroup
            width: parent.width
            title: qsTr("Nearby Devices")

            SettingsScanRow {
                id: searchingRow
                objectName: "bluetoothSearchingRow"
                width: parent.width
                visible: root.searching
                scanningText: qsTr("Searching…")
            }

            Text {
                id: nearbyEmpty
                objectName: "bluetoothNoNearby"
                width: parent.width
                visible: !root.searching && nearbyRepeater.count === 0
                text: !root.ready
                    ? qsTr("Nearby devices are unavailable while Bluetooth is off.")
                    : (root.powered ? qsTr("No nearby devices found.")
                                    : qsTr("Turn on Bluetooth to find nearby devices."))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: nearbyRepeater
                model: root.ready ? root.nearbyDevices : []
                delegate: SettingsDeviceRow {
                    id: nearbyRow
                    required property var modelData
                    required property int index
                    objectName: "bluetoothNearbyRow"
                    width: parent.width
                    deviceGlyph: root.deviceGlyph(nearbyRow.modelData.type)
                    label: String(nearbyRow.modelData.name)
                    statusText: nearbyRow.modelData.connected
                        ? qsTr("Connected") : qsTr("Not Connected")
                    deviceActive: nearbyRow.modelData.connected === true
                    settingsId: "nearby"
                    owner: "BlueZ"
                    settingsKey: "discovery"
                    showSeparator: nearbyRow.index < root.nearbyDevices.length - 1
                    controlData: Button {
                        objectName: "bluetoothNearbyAction"
                        text: nearbyRow.modelData.connected
                            ? qsTr("Disconnect") : qsTr("Connect")
                        variant: "ghost"
                        accessibleName: nearbyRow.modelData.name + ", " + text
                        onClicked: {
                            if (nearbyRow.modelData.connected)
                                root.toggleDevice(nearbyRow.modelData.address, true);
                            else
                                Settings.pairBluetooth(nearbyRow.modelData.address);
                        }
                    }
                }
            }
        }

        // ── Advanced ──────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width

            SettingsRow {
                id: advancedRow
                objectName: "bluetoothAdvancedRow"
                width: parent.width
                label: qsTr("Advanced…")
                description: qsTr("Adapter, address, and pairing policy.")
                settingsId: "advanced"
                owner: "BlueZ + settingsd"
                settingsKey: "bluetooth.*"
                showSeparator: false
                trailingAction: qsTr("Open…")
                onTrailingActionClicked: root.advancedDialog.show()
            }
        }
    }

    // ── Device info sheet ─────────────────────────────────────────────────
    Dialog {
        id: deviceDialog
        objectName: "bluetoothDeviceDialog"
        showCloseAction: false
        title: qsTr("Device Info")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Text {
                id: deviceTitle
                objectName: "bluetoothDeviceDialogTitle"
                width: parent.width
                text: root.deviceFor(root.detailsAddress) !== null
                    ? String(root.deviceFor(root.detailsAddress).name) : ""
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                font.weight: Theme.primitive.font.weightMedium
            }
            Text {
                id: deviceAddress
                objectName: "bluetoothDeviceDialogAddress"
                width: parent.width
                text: qsTr("Address: %1").arg(root.detailsAddress)
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: deviceType
                objectName: "bluetoothDeviceDialogType"
                width: parent.width
                text: qsTr("Type: %1").arg(root.typeLabel(root.detailsAddress))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: deviceBattery
                objectName: "bluetoothDeviceDialogBattery"
                width: parent.width
                text: qsTr("Battery: %1").arg(root.batteryLabel(root.detailsAddress))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "bluetoothDeviceDialogState"
                width: parent.width
                text: qsTr("Status: %1")
                    .arg(root.deviceFor(root.detailsAddress) !== null
                         && root.deviceFor(root.detailsAddress).connected === true
                         ? qsTr("Connected") : qsTr("Not Connected"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                id: deviceConnectButton
                objectName: "bluetoothDeviceConnect"
                text: root.deviceFor(root.detailsAddress) !== null
                    && root.deviceFor(root.detailsAddress).connected === true
                    ? qsTr("Disconnect") : qsTr("Connect")
                enabled: root.ready && !root.readOnly
                onClicked: root.toggleDetailsDevice()
            }
            Button {
                id: deviceForgetButton
                objectName: "bluetoothDeviceForget"
                text: qsTr("Forget")
                variant: "ghost"
                enabled: root.ready && !root.readOnly
                onClicked: root.forgetDetailsDevice()
            }
            Button {
                text: qsTr("Close")
                onClicked: deviceDialog.hide()
            }
        }
    }

    // ── Advanced sheet ────────────────────────────────────────────────────
    Dialog {
        id: advancedDialog
        objectName: "bluetoothAdvancedDialog"
        showCloseAction: false
        title: qsTr("Bluetooth Advanced")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Text {
                id: advancedAdapterName
                objectName: "bluetoothAdvancedName"
                width: parent.width
                text: qsTr("Adapter: %1").arg(root.adapterName.length > 0
                                               ? root.adapterName : qsTr("None"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: advancedAdapterAddress
                objectName: "bluetoothAdvancedAddress"
                width: parent.width
                text: qsTr("Address: %1").arg(root.adapterAddress.length > 0
                                               ? root.adapterAddress : qsTr("Unavailable"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: advancedDiscoverable
                objectName: "bluetoothAdvancedDiscoverable"
                width: parent.width
                text: qsTr("Discoverable: %1")
                    .arg(root.ready && root.view.discoverable === true
                         ? qsTr("Yes") : qsTr("No"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: advancedPairable
                objectName: "bluetoothAdvancedPairable"
                width: parent.width
                text: qsTr("Pairable: %1")
                    .arg(root.ready && root.view.pairable === true
                         ? qsTr("Yes") : qsTr("No"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("LE Audio and device battery levels appear here when "
                           + "BlueZ and PipeWire expose them.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Close")
                onClicked: advancedDialog.hide()
            }
        }
    }
}
