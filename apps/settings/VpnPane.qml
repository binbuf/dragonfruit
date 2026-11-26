// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The top-level VPN pane (T272; T288 visual parity).
//
// T288 makes the pane always visible: the old optional-profile gate is gone,
// so an empty host shows the mockup's "No VPN configurations" state and the
// Add VPN Configuration… footer instead of losing the pane. The pane reads
// the same one VPN view the Network pane uses, so there is no second model.
// Connect/disconnect, auto-connect, Add VPN, and Info all report their
// outcome. The WireGuard capability row stays below the connection list.
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

    property alias intro: intro
    property alias connectionRepeater: connectionRepeater
    property alias connectionsGroup: connectionsGroup
    property alias wireguardGroup: wireguardGroup
    property alias addButton: addButton
    property alias addReasonNote: addReasonNote
    property alias addVpnDialog: addVpnDialog
    property alias infoDialog: infoDialog
    property alias absenceNote: absenceNote
    property alias emptyNote: emptyNote
    property alias writeNote: writeNote

    // Why Add is unavailable, in the same words the old row used. The footer
    // button stays visible so the pane never loses the mockup action; only
    // its enabled state and the note change.
    readonly property string addReasonText: {
        if (!root.canAdd) {
            if (root.plugins.length > 0 && !root.editorAvailable)
                return qsTr("Install %1 to finish plugin profiles.")
                    .arg(root.editorName.length > 0
                         ? root.editorName : "nm-connection-editor");
            return root.addReason.length > 0
                ? root.addReason
                : qsTr("No VPN plugin is installed and native WireGuard is "
                       + "not available.");
        }
        if (root.plugins.length > 0 && !root.editorAvailable)
            return qsTr("Install %1 to finish plugin profiles, or add a "
                       + "native WireGuard profile.")
                .arg(root.editorName.length > 0
                     ? root.editorName : "nm-connection-editor");
        return "";
    }

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
        if (!root.ready || root.readOnly || !root.canAdd)
            return;
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

        // ── Intro (mockup §5.2) ───────────────────────────────────────────
        SettingsIntro {
            id: intro
            objectName: "vpnIntro"
            width: parent.width
            text: qsTr("VPN configurations connect this computer to a private "
                       + "network. Profiles are owned by the system network "
                       + "service.")
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
                label: qsTr("No VPN configurations")
                description: qsTr("Add a VPN configuration to connect to a "
                                   + "private network.")
                showSeparator: false
            }

            // A present-but-unavailable service keeps the section and names
            // the missing half (ADR 0262); the group never renders empty.
            SettingsRow {
                objectName: "vpnConnectionsUnavailable"
                width: parent.width
                visible: !root.ready
                label: qsTr("Connections unavailable")
                description: qsTr("The VPN service is not running.")
                showSeparator: false
            }
        }

        // The mockup's empty state is the pane's default; a configured host
        // shows the same group with its real connection rows.
        SettingsGroup {
            id: wireguardGroup
            objectName: "vpnWireguardGroup"
            width: parent.width
            title: qsTr("WireGuard")

            SettingsRow {
                width: parent.width
                label: qsTr("WireGuard")
                description: root.wireguardNative
                    ? qsTr("Native WireGuard profiles connect through "
                           + "NetworkManager; no plugin is required.")
                    : qsTr("Native WireGuard is not available on this host.")
                settingsId: "vpn.wireguard"
                owner: "NetworkManager VPN"
                settingsKey: "wireguard"
                showSeparator: false
                controlData: Text {
                    text: root.wireguardNative ? qsTr("Built in")
                                               : qsTr("Unavailable")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }
        }

        // ── Add VPN Configuration… footer ─────────────────────────────────
        SettingsFooter {
            id: addFooter
            objectName: "vpnAddFooter"
            width: parent.width

            Button {
                id: addButton
                objectName: "vpnAddButton"
                text: qsTr("Add VPN Configuration…")
                enabled: root.ready && !root.readOnly && root.canAdd
                onClicked: root.addVpn()
            }
        }

        SettingsNote {
            id: addReasonNote
            objectName: "vpnAddReason"
            width: parent.width
            visible: root.ready && root.addReasonText.length > 0
            text: root.addReasonText
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
