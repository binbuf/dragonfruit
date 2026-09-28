// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit
import Dragonfruit.Settings

// The Bluetooth pane (T-15.1b).
//
// The pane and the Control Center tile are one functional unit: both read the
// same BlueZ adapter through the bridge host, and the four explicit writes
// (power, discovery, pair, connect) go through the `Settings` singleton to the
// adapter — never to D-Bus from QML. Opening the pane asks for a re-read and
// starts an inquiry while it is open (the reference caption: "This <device> is
// discoverable as \"<device>\" while Bluetooth Settings is open."); closing it
// stops the inquiry.
//
// The macOS reference's `?` help button and the `Learn more…` web link are
// omitted (Apple-only, ADR 0118); the captured device names stay as fixtures.
// Absence is a normal state: `bluetoothd` gone or no controller disables the
// controls and shows a one-line note, never an error.
Item {
    id: root

    // The bridge host's Bluetooth view (empty when absent).
    readonly property var view: Settings.bluetooth
    readonly property bool available: Settings.bluetoothAvailable
    readonly property bool present: view.present === true
    readonly property bool powered: view.powered === true
    readonly property bool discovering: view.discovering === true
    readonly property string adapterName: view.adapterName !== undefined
        ? view.adapterName : ""
    readonly property var knownDevices: view.knownDevices !== undefined
        ? view.knownDevices : []
    readonly property var nearbyDevices: view.nearbyDevices !== undefined
        ? view.nearbyDevices : []

    // The two hide rules (ADR 0117): `bluetoothd` absent (not available) or a
    // running daemon with no controller (`present: false`).
    readonly property bool ready: root.available && root.present
    readonly property bool searching: root.powered && root.discovering

    readonly property string discoverableCaption:
        qsTr("This %1 is discoverable as \"%2\" while Bluetooth Settings is open.")
            .arg(root.adapterName).arg(root.adapterName)

    // Test surface (used by tst_settings_bluetooth.qml).
    property alias powerGroup: powerGroup
    property alias powerToggle: powerToggle
    property alias powerSubtitle: powerSubtitle
    property alias caption: caption
    property alias absenceNote: absenceNote
    property alias myDevicesGroup: myDevicesGroup
    property alias nearbyGroup: nearbyGroup
    property alias searchingRow: searchingRow
    property alias spinner: spinner

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function togglePower() {
        if (!root.ready)
            return;
        Settings.setBluetoothPowered(!root.powered);
    }

    function toggleDevice(address, connected) {
        if (!root.ready || !address)
            return;
        Settings.setBluetoothConnected(address, !connected);
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
                                : qsTr("Unavailable")
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Toggle {
                        id: powerToggle
                        objectName: "bluetoothPowerToggle"
                        accessibleName: qsTr("Bluetooth")
                        checked: root.powered
                        enabled: root.ready
                        anchors.verticalCenter: parent.verticalCenter
                        onToggled: (checked) => Settings.setBluetoothPowered(checked)
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
                id: absenceNote
                objectName: "bluetoothAbsenceNote"
                width: parent.width
                visible: !root.ready
                text: qsTr("Bluetooth is not available on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
            }
        }

        // ── My Devices ────────────────────────────────────────────────────
        SettingsGroup {
            id: myDevicesGroup
            width: parent.width
            title: qsTr("My Devices")
            visible: root.ready

            Text {
                objectName: "bluetoothNoDevices"
                width: parent.width
                visible: knownRepeater.count === 0
                text: qsTr("No devices yet.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: knownRepeater
                model: root.knownDevices
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
                            text: "i"
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.weight: Theme.primitive.font.weightMedium
                        }

                        Accessible.role: Accessible.Button
                        Accessible.name: knownRow.modelData.connected
                            ? qsTr("Disconnect %1").arg(knownRow.modelData.name)
                            : qsTr("Connect %1").arg(knownRow.modelData.name)

                        TapHandler {
                            onTapped: root.toggleDevice(knownRow.modelData.address,
                                                        knownRow.modelData.connected)
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
            visible: root.ready

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
                objectName: "bluetoothNoNearby"
                width: parent.width
                visible: root.ready && !root.searching
                       && nearbyRepeater.count === 0
                text: root.powered ? qsTr("No nearby devices found.")
                                   : qsTr("Turn on Bluetooth to find nearby devices.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: nearbyRepeater
                model: root.nearbyDevices
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
    }
}