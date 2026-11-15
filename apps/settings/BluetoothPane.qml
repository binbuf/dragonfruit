// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit
import Dragonfruit.Settings

// The Bluetooth pane (T-15.1b; T271 completion).
//
// The pane and the Control Center tile are one functional unit: both read the
// same BlueZ adapter through the bridge host, and the explicit writes (power,
// discovery, pair, connect, forget) go through the `Settings` singleton to the
// adapter — never to D-Bus from QML. Opening the pane asks for a re-read and
// starts an inquiry while it is open; turning the radio on starts discovery
// too. Closing it stops the inquiry.
//
// The macOS reference's `?` help button and the `Learn more…` web link are
// omitted (Apple-only, ADR 0118). The discoverable row names the computer, not
// the adapter id; the adapter address lives in Advanced. Absence is
// structural: every group stays and only the host controls disable with a
// reason (ADR 0262).
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
    property alias powerGroup: powerGroup
    property alias powerToggle: powerToggle
    property alias powerSubtitle: powerSubtitle
    property alias caption: caption
    property alias absenceNote: absenceNote
    property alias myDevicesGroup: myDevicesGroup
    property alias nearbyGroup: nearbyGroup
    property alias knownRepeater: knownRepeater
    property alias nearbyRepeater: nearbyRepeater
    property alias searchingRow: searchingRow
    property alias spinner: spinner
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

        // ── The Bluetooth toggle card ─────────────────────────────────────
        SettingsGroup {
            id: powerGroup
            width: parent.width

            Item {
                width: parent.width
                height: Math.max(48, powerRow.implicitHeight)

                Row {
                    id: powerRow
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.md

                    Rectangle {
                        width: 40
                        height: 40
                        radius: 10
                        color: root.powered ? Theme.color.accentMuted
                                            : Theme.color.controlFill
                        anchors.verticalCenter: parent.verticalCenter

                        Icon {
                            anchors.centerIn: parent
                            name: "bluetooth"
                            size: 22
                            color: root.powered ? Theme.color.accent
                                                : Theme.color.textSecondary
                        }
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - 40 - powerToggle.width
                               - 2 * Theme.primitive.spacing.md
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "bluetoothPowerTitle"
                            text: qsTr("Bluetooth")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            font.weight: Theme.primitive.font.weightMedium
                        }

                        Text {
                            id: powerSubtitle
                            objectName: "bluetoothPowerSubtitle"
                            width: parent.width
                            text: root.ready
                                ? (root.discovering ? qsTr("Discovering\u2026")
                                                    : (root.powered ? qsTr("On")
                                                                    : qsTr("Off")))
                                : (root.failed ? qsTr("Unavailable") : qsTr("Off"))
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Toggle {
                        id: powerToggle
                        objectName: "bluetoothPowerToggle"
                        accessibleName: qsTr("Bluetooth")
                        enabled: root.ready
                        anchors.verticalCenter: parent.verticalCenter
                        onToggled: (checked) => root.setPowered(checked)

                        Binding {
                            target: powerToggle
                            property: "checked"
                            value: root.powered
                        }
                    }
                }
            }

            Text {
                id: caption
                objectName: "bluetoothDiscoverableCaption"
                width: parent.width
                visible: root.ready && root.powered
                text: root.discoverableCaption
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
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

        // ── Discoverable as ───────────────────────────────────────────────
        SettingsGroup {
            id: discoverableRow
            width: parent.width
            title: qsTr("Discoverable As")

            SettingsRow {
                width: parent.width
                label: root.computerName
                description: root.ready
                    ? qsTr("Discoverable while Bluetooth Settings is open.")
                    : qsTr("Unavailable")
                settingsId: "discoverable"
                owner: "BlueZ"
                settingsKey: "Adapter1.Discoverable"
                showSeparator: false
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
                delegate: Item {
                    id: knownRow
                    required property var modelData
                    width: parent.width
                    height: 44

                    Text {
                        objectName: "bluetoothDeviceName"
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: knownState.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: knownRow.modelData.name
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Text {
                        id: knownState
                        objectName: "bluetoothDeviceState"
                        anchors.right: infoButton.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: knownRow.modelData.connected ? qsTr("Connected")
                                                           : qsTr("Not Connected")
                        color: knownRow.modelData.connected ? Theme.color.accent
                                                            : Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Rectangle {
                        id: infoButton
                        objectName: "bluetoothInfoButton"
                        width: 22
                        height: 22
                        radius: 11
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        color: "transparent"
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        Text {
                            anchors.centerIn: parent
                            text: "i" // df-allow-untranslated: info glyph
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.weight: Theme.primitive.font.weightMedium
                        }

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Info for %1").arg(knownRow.modelData.name)

                        TapHandler {
                            onTapped: root.showDeviceInfo(knownRow.modelData.address)
                        }
                    }

                    Rectangle {
                        width: parent.width
                        height: Theme.controls.window.borderWidth
                        anchors.bottom: parent.bottom
                        color: Theme.color.separator
                    }
                }
            }
        }

        // ── Nearby Devices ────────────────────────────────────────────────
        SettingsGroup {
            id: nearbyGroup
            width: parent.width
            title: qsTr("Nearby Devices")

            Row {
                id: searchingRow
                objectName: "bluetoothSearchingRow"
                width: parent.width
                spacing: Theme.primitive.spacing.sm
                visible: root.searching
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs

                Shape {
                    id: spinner
                    objectName: "bluetoothSpinner"
                    width: 16
                    height: 16
                    anchors.verticalCenter: parent.verticalCenter

                    ShapePath {
                        strokeColor: Theme.color.accent
                        strokeWidth: 2
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap

                        PathAngleArc {
                            centerX: 8
                            centerY: 8
                            radiusX: 6
                            radiusY: 6
                            startAngle: 0
                            sweepAngle: 270
                        }
                    }

                    RotationAnimator on rotation {
                        from: 0
                        to: 360
                        duration: 900
                        loops: Animation.Infinite
                        running: spinner.visible && !Theme.reducedMotion
                    }
                }

                Text {
                    objectName: "bluetoothSearchingLabel"
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Searching\u2026")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
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
                delegate: Item {
                    id: nearbyRow
                    required property var modelData
                    width: parent.width
                    height: 44

                    Text {
                        objectName: "bluetoothNearbyName"
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: nearbyAction.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: nearbyRow.modelData.name
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Text {
                        id: nearbyAction
                        objectName: "bluetoothNearbyAction"
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        text: nearbyRow.modelData.connected ? qsTr("Disconnect")
                                                            : qsTr("Connect")
                        color: Theme.color.accent
                        font.pixelSize: Theme.primitive.font.sizeSm
                        Accessible.role: Accessible.Button
                        Accessible.name: nearbyRow.modelData.name + ", " + text
                        TapHandler {
                            onTapped: {
                                if (nearbyRow.modelData.connected)
                                    root.toggleDevice(nearbyRow.modelData.address, true);
                                else
                                    Settings.pairBluetooth(nearbyRow.modelData.address);
                            }
                        }
                    }

                    Rectangle {
                        width: parent.width
                        height: Theme.controls.window.borderWidth
                        anchors.bottom: parent.bottom
                        color: Theme.color.separator
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
