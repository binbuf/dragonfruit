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
    readonly property bool wireguardNative: root.view.wireguardNative === true
    readonly property bool editorAvailable: root.view.editorAvailable === true
    readonly property string editorName: String(root.view.editor || "")
    readonly property bool canAdd: root.wireguardNative
        || (root.plugins.length > 0 && root.editorAvailable)
    readonly property string addReason: String(root.view.addReason || "")
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
        addVpnDialog.prepare();
    }

    function submitAdd(kind, plugin, values) {
        if (kind === "wireguard") {
            Settings.addVpnWireguard(String(values.name),
                                     String(values.privateKey),
                                     String(values.address),
                                     String(values.peerPublicKey),
                                     String(values.endpoint),
                                     String(values.dns), Number(values.mtu || 0));
            addVpnDialog.resultText = qsTr("Adding “%1”…").arg(String(values.name));
            return;
        }
        Settings.addVpn(String(plugin), String(values.name),
                        String(values.gateway || ""));
        addVpnDialog.resultText = qsTr("Adding “%1”…").arg(String(values.name));
    }

    function completeSetup(uuid) {
        Settings.editVpn(String(uuid));
    }

    Connections {
        target: Settings
        function onVpnWriteChanged() {
            var state = String(Settings.vpnWriteState || "");
            var operation = String(Settings.vpnWriteOperation || "");
            if (state === "applied" || state === "failed"
                    || state === "denied" || state === "absent") {
                if (operation === "add") {
                    addVpnDialog.resultText = state === "applied"
                        ? qsTr("VPN added. Finish it in the connection editor; "
                               + "it stays “Needs setup” until then.")
                        : String(Settings.vpnWriteError || qsTr("The VPN was not added."));
                } else if (operation === "add-wireguard") {
                    addVpnDialog.resultText = state === "applied"
                        ? qsTr("WireGuard VPN added.")
                        : String(Settings.vpnWriteError || qsTr("The VPN was not added."));
                } else if (operation === "edit") {
                    addVpnDialog.resultText = state === "applied"
                        ? qsTr("Setup opened.")
                        : String(Settings.vpnWriteError || qsTr("The editor did not open."));
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
                        Button {
                            objectName: "vpnCompleteSetup"
                            text: qsTr("Complete Setup…")
                            variant: "ghost"
                            visible: connectionRow.modelData.needsSetup === true
                                     && root.editorAvailable
                            enabled: root.ready && !root.readOnly
                            onClicked: root.completeSetup(
                                String(connectionRow.modelData.uuid))
                        }
                        Toggle {
                            id: connectionToggle
                            objectName: "vpnConnectToggle"
                            text: ""
                            accessibleName: qsTr("Connect %1")
                                .arg(connectionRow.modelData.id)
                            visible: connectionRow.modelData.needsSetup !== true
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
                            visible: connectionRow.modelData.needsSetup !== true
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
                            visible: connectionRow.modelData.needsSetup !== true
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
                description: {
                    if (!root.canAdd) {
                        if (root.plugins.length > 0 && !root.editorAvailable)
                            return qsTr("Install %1 to finish plugin profiles.")
                                .arg(root.editorName.length > 0
                                     ? root.editorName : "nm-connection-editor");
                        return root.addReason.length > 0
                            ? root.addReason
                            : qsTr("No VPN plugin is installed and native "
                                   + "WireGuard is not available.");
                    }
                    if (root.plugins.length > 0 && !root.editorAvailable)
                        return qsTr("Install %1 to finish plugin profiles, or "
                                    + "add a native WireGuard profile.")
                            .arg(root.editorName.length > 0
                                 ? root.editorName : "nm-connection-editor");
                    var parts = [];
                    if (root.wireguardNative)
                        parts.push(qsTr("WireGuard (native)"));
                    if (root.plugins.length > 0)
                        parts.push(qsTr("%1 plugin(s)").arg(root.plugins.length));
                    return parts.join(qsTr(" · "));
                }
                settingsId: "vpn.add"
                owner: "NetworkManager VPN"
                settingsKey: "AddConnection"
                showSeparator: false
                trailingAction: root.canAdd ? qsTr("Add…") : ""
                enabled: root.ready && !root.readOnly && root.canAdd
                onTrailingActionClicked: root.addVpn()
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

    VpnAddDialog {
        id: addVpnDialog
        objectName: "vpnAddDialog"
        width: root.Window.window ? root.Window.window.width : root.width
        height: root.Window.window ? root.Window.window.height : root.height
        plugins: root.plugins
        wireguardNative: root.wireguardNative
        editorAvailable: root.editorAvailable
        editorName: root.editorName
        onSubmitted: (kind, plugin, values) => root.submitAdd(kind, plugin, values)
    }
}
