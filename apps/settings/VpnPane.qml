// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The optional top-level VPN pane (T272).
//
// The sidebar shows this pane only when NetworkManager reports at least one
// VPN profile; otherwise Network → VPN is the only entry point. The pane reads
// the same one VPN view the Network pane uses, so there is no second model.
// Connect/disconnect, auto-connect, Add VPN, and Info all report their outcome.
Item {
    id: root

    readonly property var view: Settings.vpn
    readonly property bool available: Settings.vpnAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var connections: root.view.connections !== undefined
        ? root.view.connections : []
    readonly property var plugins: root.view.plugins !== undefined
        ? root.view.plugins : []
    readonly property bool readOnly: root.view.readOnly === true
    readonly property string readOnlyMessage: String(root.view.note || "")
    readonly property int connectedCount: Number(root.view.connectedCount || 0)
    readonly property string activeName: String(root.view.activeName || "")
    readonly property string writeState: String(Settings.vpnWriteState || "idle")
    readonly property string writeError: String(Settings.vpnWriteError || "")
    readonly property bool writeFailed: root.writeState === "failed"
        || root.writeState === "denied" || root.writeState === "absent"
    property var infoConnection: null

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    property alias connectionRepeater: connectionRepeater
    property alias addVpnRow: addVpnRow
    property alias addVpnDialog: addVpnDialog
    property alias infoDialog: infoDialog
    property alias absenceNote: absenceNote
    property alias emptyNote: emptyNote
    property alias writeNote: writeNote

    Component.onCompleted: Settings.refreshVpn()

    function isConnected(uuid) {
        for (var i = 0; i < root.connections.length; ++i)
            if (String(root.connections[i].uuid) === String(uuid))
                return root.connections[i].connected === true;
        return false;
    }

    function setConnected(uuid, connected) {
        if (connected)
            Settings.connectVpn(String(uuid));
        else
            Settings.disconnectVpn(String(uuid));
    }

    function openInfo(connection) {
        root.infoConnection = connection;
        infoDialog.show();
    }

    function addVpn() {
        var name = addVpnName.text.trim();
        if (name.length === 0) {
            addVpnResult.text = qsTr("A VPN name is required.");
            return;
        }
        if (addVpnPlugin.currentValue === undefined) {
            addVpnResult.text = qsTr("Choose a VPN plugin.");
            return;
        }
        Settings.addVpn(String(addVpnPlugin.currentValue), name,
                        addVpnGateway.text);
        addVpnResult.text = qsTr("Adding “%1”…").arg(name);
    }

    Connections {
        target: Settings
        function onVpnWriteChanged() {
            var state = String(Settings.vpnWriteState || "");
            if (state === "applied" || state === "failed"
                    || state === "denied" || state === "absent") {
                if (String(Settings.vpnWriteOperation || "") === "add") {
                    addVpnResult.text = state === "applied"
                        ? qsTr("VPN added.")
                        : String(Settings.vpnWriteError || qsTr("The VPN was not added."));
                }
            }
        }
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            title: qsTr("VPN")

            SettingsRow {
                width: parent.width
                label: qsTr("Status")
                description: !root.ready
                    ? qsTr("Unavailable")
                    : (root.connectedCount === 0
                       ? qsTr("Not connected")
                       : qsTr("Connected to %1").arg(root.activeName))
                settingsId: "vpn.status"
                owner: "NetworkManager VPN"
                settingsKey: "ActiveConnection"
                showSeparator: false
            }
        }

        SettingsGroup {
            id: connectionsGroup
            width: parent.width
            title: qsTr("Connections")

            Repeater {
                id: connectionRepeater
                model: root.connections
                delegate: SettingsRow {
                    id: connectionRow
                    required property var modelData
                    required property int index
                    objectName: "vpnConnectionRow"
                    width: parent.width
                    label: String(connectionRow.modelData.id)
                    description: String(connectionRow.modelData.label)
                    settingsId: "vpn"
                    owner: "NetworkManager VPN"
                    settingsKey: "Settings.Connection"
                    showSeparator: connectionRow.index < root.connections.length - 1
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Rectangle {
                            objectName: "vpnStatusDot"
                            width: 8
                            height: 8
                            radius: 4
                            anchors.verticalCenter: parent.verticalCenter
                            color: connectionRow.modelData.connected === true
                                ? Theme.color.success : Theme.color.textTertiary
                        }
                        Toggle {
                            id: connectionToggle
                            objectName: "vpnConnectToggle"
                            text: ""
                            accessibleName: qsTr("Connect %1")
                                .arg(connectionRow.modelData.id)
                            enabled: root.ready && !root.readOnly
                            onToggled: (checked) =>
                                root.setConnected(connectionRow.modelData.uuid, checked)

                            Binding {
                                target: connectionToggle
                                property: "checked"
                                value: root.isConnected(connectionRow.modelData.uuid)
                            }
                        }
                        Text {
                            text: qsTr("Auto")
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Toggle {
                            id: autoToggle
                            objectName: "vpnAutoToggle"
                            text: ""
                            accessibleName: qsTr("Auto-connect %1")
                                .arg(connectionRow.modelData.id)
                            enabled: root.ready && !root.readOnly
                            onToggled: (checked) => Settings.setVpnAutoconnect(
                                String(connectionRow.modelData.uuid), checked)

                            Binding {
                                target: autoToggle
                                property: "checked"
                                value: connectionRow.modelData.autoconnect === true
                            }
                        }
                        Button {
                            objectName: "vpnInfoButton"
                            text: qsTr("Info…")
                            variant: "ghost"
                            enabled: root.ready
                            onClicked: root.openInfo(connectionRow.modelData)
                        }
                    }
                }
            }

            SettingsRow {
                id: emptyNote
                objectName: "vpnEmptyNote"
                width: parent.width
                visible: root.ready && root.connections.length === 0
                label: qsTr("No VPN connection is configured.")
                description: qsTr("Add a VPN below to connect.")
                showSeparator: root.plugins.length > 0
            }

            SettingsRow {
                id: addVpnRow
                width: parent.width
                label: qsTr("Add VPN Configuration…")
                description: root.plugins.length > 0
                    ? qsTr("%1 plugin(s) installed").arg(root.plugins.length)
                    : qsTr("No NetworkManager VPN plugin is installed.")
                settingsId: "vpn.add"
                owner: "NetworkManager VPN"
                settingsKey: "AddConnection"
                showSeparator: false
                trailingAction: root.plugins.length > 0 ? qsTr("Add…") : ""
                enabled: root.ready && !root.readOnly && root.plugins.length > 0
                onTrailingActionClicked: {
                    addVpnName.text = "";
                    addVpnGateway.text = "";
                    addVpnResult.text = "";
                    addVpnDialog.show();
                }
            }
        }

        SettingsGroup {
            width: parent.width
            visible: root.writeFailed

            Text {
                id: writeNote
                objectName: "vpnWriteNote"
                width: parent.width
                text: root.writeError.length > 0
                    ? root.writeError
                    : qsTr("The last VPN change did not apply.")
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        SettingsGroup {
            width: parent.width
            visible: root.readOnly

            Text {
                width: parent.width
                text: root.readOnlyMessage.length > 0
                    ? qsTr("VPN connections can be viewed but not changed: %1")
                          .arg(root.readOnlyMessage)
                    : qsTr("VPN connections can be viewed but not changed.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        SettingsGroup {
            width: parent.width
            visible: !root.ready

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "vpnAbsenceNote"
                kind: "disabled-with-reason"
                reason: qsTr("The VPN service (NetworkManager) is not running, so "
                             + "VPN connections are unavailable.")
                width: parent.width
            }
        }
    }

    Dialog {
        id: infoDialog
        objectName: "vpnInfoDialog"
        title: root.infoConnection !== null
            ? qsTr("VPN — %1").arg(String(root.infoConnection.id || ""))
            : qsTr("VPN")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Repeater {
                model: root.infoConnection !== null ? [
                    { label: qsTr("Type"),
                      value: String(root.infoConnection.kindLabel || "") },
                    { label: qsTr("State"),
                      value: String(root.infoConnection.stateLabel || "") },
                    { label: qsTr("Auto-connect"),
                      value: root.infoConnection.autoconnect === true
                          ? qsTr("On") : qsTr("Off") },
                    { label: qsTr("UUID"),
                      value: String(root.infoConnection.uuid || "") }
                ] : []
                delegate: Row {
                    required property var modelData
                    width: parent.width
                    height: Theme.controls.settingsRow.height * 0.8
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        id: infoLabel
                        width: Math.min(160, parent.width * 0.4)
                        text: String(modelData.label)
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        width: parent.width - infoLabel.width - parent.spacing
                        text: String(modelData.value)
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.verticalCenter: parent.verticalCenter
                        wrapMode: Text.WordWrap
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Close")
                onClicked: infoDialog.hide()
            }
        }
    }

    Dialog {
        id: addVpnDialog
        objectName: "vpnAddDialog"
        title: qsTr("Add VPN Configuration")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("VPN plugin")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Select {
                id: addVpnPlugin
                objectName: "vpnAddPlugin"
                model: root.plugins.map(function(plugin) {
                    return { label: String(plugin.label), value: String(plugin.serviceType) };
                })
            }
            Text {
                width: parent.width
                text: qsTr("Name")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: addVpnName
                objectName: "vpnAddName"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("Gateway")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: addVpnGateway
                objectName: "vpnAddGateway"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: addVpnResult
                objectName: "vpnAddResult"
                width: parent.width
                color: root.writeFailed ? Theme.color.danger
                                        : Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Cancel")
                onClicked: addVpnDialog.hide()
            }
            Button {
                objectName: "vpnAddConfirm"
                text: qsTr("Add")
                variant: "primary"
                onClicked: root.addVpn()
            }
        }
    }
}
