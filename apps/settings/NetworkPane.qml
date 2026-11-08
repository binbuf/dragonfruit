// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Network pane (T225).
//
// One NetworkManager model serves the Wi-Fi disclosure, the service order,
// the VPN list, and the firewall sheet. The pane never caches a connected
// state: every write goes to the bridge host and its re-read pushes the new
// view back. NetworkManager owns services, order (`autoconnect-priority`), and
// VPN connections; the firewall adapter owns detection and the firewall
// writes. A missing manager is an honest absence, never a fake Off.
//
// The macOS reference's `?` help and `Learn more…` are omitted project-wide.
// `Thunderbolt Bridge` is Apple-hardware-specific and appears only when the
// host actually has such a bridge.
Item {
    id: root

    readonly property var view: Settings.network
    readonly property bool available: Settings.networkAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var services: root.view.services !== undefined
        ? root.view.services : []
    readonly property var knownNetworks: root.view.knownNetworks !== undefined
        ? root.view.knownNetworks : []
    readonly property var firewall: root.view.firewall !== undefined
        ? root.view.firewall : ({})
    readonly property var vpnView: Settings.vpn
    readonly property bool vpnAvailable: Settings.vpnAvailable
    readonly property bool vpnReady: root.vpnAvailable && root.vpnView.state === "available"
    readonly property var vpnConnections: root.vpnView.connections !== undefined
        ? root.vpnView.connections : []
    readonly property var vpnPlugins: root.vpnView.plugins !== undefined
        ? root.vpnView.plugins : []
    readonly property int connectionCount: root.vpnConnections.length
    readonly property bool readOnly: root.view.readOnly === true
        || root.vpnView.readOnly === true
        || root.firewall.readOnly === true
    readonly property string readOnlyMessage: {
        if (root.vpnView.readOnly === true)
            return String(root.vpnView.note || "");
        if (root.firewall.readOnly === true)
            return String(root.firewall.readOnlyNote || "");
        return String(root.view.note || "");
    }
    readonly property bool firewallDetected: root.firewall.detected === true
    readonly property bool firewallStatusKnown: root.firewall.statusKnown === true
    readonly property bool firewallEnabled: root.firewall.enabled === true

    // Other services: everything except Wi-Fi (its own pane) and VPN (its own
    // group), matching the reference's Other Services list.
    readonly property var otherServices: {
        var out = [];
        for (var i = 0; i < root.services.length; ++i) {
            var service = root.services[i] || {};
            if (service.kind === "wifi" || service.kind === "vpn"
                    || service.kind === "wireguard")
                continue;
            out.push(service);
        }
        return out;
    }

    signal paneRequested(string paneId)

    // Test surface (tst_settings_network.qml).
    property alias wifiRow: wifiRow
    property alias firewallGroup: firewallGroup
    property alias firewallToggle: firewallToggle
    property alias firewallOptionsRow: firewallOptionsRow
    property alias firewallDialog: firewallDialog
    property alias servicesGroup: servicesGroup
    property alias servicesRepeater: servicesRepeater
    property alias addServiceRow: addServiceRow
    property alias addServiceDialog: addServiceDialog
    property alias vpnGroup: vpnGroup
    property alias vpnConnectionRepeater: vpnConnectionRepeater
    property alias connectionRepeater: vpnConnectionRepeater
    property alias addVpnRow: addVpnRow
    property alias addVpnDialog: addVpnDialog
    property alias absenceNote: absenceNote
    property alias readOnlyNote: readOnlyNoteItem
    property alias emptyNote: emptyNote
    property alias advancedRow: advancedRow
    property alias advancedDialog: advancedDialog
    property alias firewallPortRepeater: firewallPortRepeater
    property alias firewallServiceRepeater: firewallServiceRepeater

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Component.onCompleted: {
        Settings.refreshNetwork();
        Settings.refreshVpn();
    }

    function serviceFor(uuid) {
        for (var i = 0; i < root.services.length; ++i)
            if (String(root.services[i].uuid) === String(uuid))
                return root.services[i];
        return null;
    }

    function isConnected(uuid) {
        for (var i = 0; i < root.vpnConnections.length; ++i)
            if (String(root.vpnConnections[i].uuid) === String(uuid))
                return root.vpnConnections[i].connected === true;
        return false;
    }

    function setConnected(uuid, connected) {
        if (connected)
            Settings.connectVpn(String(uuid));
        else
            Settings.disconnectVpn(String(uuid));
    }

    function toggleFirewall() {
        if (!root.ready || !root.firewallStatusKnown || root.readOnly)
            return;
        Settings.setFirewallEnabled(!root.firewallEnabled);
    }

    function addService() {
        var name = addServiceName.text.trim();
        var kind = addServiceKind.currentValue || "ethernet";
        if (name.length === 0)
            return;
        if (kind === "vpn" && addServicePlugin.currentValue === undefined)
            return;
        Settings.addNetworkService(name, kind,
                                   addServicePlugin.currentValue || "",
                                   addServiceGateway.text);
        addServiceName.text = "";
        addServiceGateway.text = "";
        addServiceDialog.hide();
    }

    function addVpn() {
        var name = addVpnName.text.trim();
        if (name.length === 0 || addVpnPlugin.currentValue === undefined)
            return;
        Settings.addVpn(String(addVpnPlugin.currentValue), name, addVpnGateway.text);
        addVpnName.text = "";
        addVpnGateway.text = "";
        addVpnDialog.hide();
    }

    function addPort() {
        var port = parseInt(addPortField.text, 10);
        if (!isFinite(port) || port <= 0 || port > 65535)
            return;
        Settings.openFirewallPort(port, addPortProtocol.currentValue || "tcp");
        addPortField.text = "";
    }

    function addFirewallService() {
        var service = addServiceField.text.trim();
        if (service.length === 0)
            return;
        Settings.openFirewallService(service);
        addServiceField.text = "";
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Wi-Fi disclosure ──────────────────────────────────────────────
        SettingsGroup {
            width: parent.width

            SettingsRow {
                id: wifiRow
                width: parent.width
                label: qsTr("Wi-Fi")
                description: {
                    var ssid = root.view.activeSsid !== undefined
                        ? String(root.view.activeSsid) : "";
                    if (!root.ready)
                        return qsTr("Unavailable");
                    if (ssid.length === 0)
                        return root.view.radioEnabled === true
                            ? qsTr("Not connected") : qsTr("Off");
                    return qsTr("Connected to %1").arg(ssid);
                }
                settingsId: "wifi"
                owner: "NetworkManager"
                settingsKey: "WirelessEnabled"
                disclosure: true
                showSeparator: false
                onDisclosureClicked: root.paneRequested("wifi")
            }
        }

        // ── Firewall ──────────────────────────────────────────────────────
        SettingsGroup {
            id: firewallGroup
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Firewall")
                description: {
                    if (!root.firewallDetected)
                        return qsTr("No firewall detected");
                    if (!root.firewallStatusKnown)
                        return qsTr("%1 · Unavailable").arg(
                            String(root.firewall.backendLabel || ""));
                    return qsTr("%1 · %2").arg(String(root.firewall.backendLabel || ""))
                                           .arg(root.firewallEnabled
                                                ? qsTr("On") : qsTr("Off"));
                }
                settingsId: "firewall"
                owner: "firewall adapter"
                settingsKey: "firewall.status"
                showSeparator: root.firewallDetected
                controlData: Toggle {
                    id: firewallToggle
                    objectName: "networkFirewallToggle"
                    text: ""
                    accessibleName: qsTr("Firewall")
                    enabled: root.ready && root.firewallStatusKnown && !root.readOnly
                    onToggled: (checked) => Settings.setFirewallEnabled(checked)

                    Binding {
                        target: firewallToggle
                        property: "checked"
                        value: root.firewallEnabled
                    }
                }
            }

            SettingsRow {
                id: firewallOptionsRow
                width: parent.width
                visible: root.firewallDetected
                label: qsTr("Firewall Options…")
                description: root.firewallStatusKnown
                    ? qsTr("%1 open ports, %2 services")
                          .arg(Number(root.firewall.portCount || 0))
                          .arg(Number(root.firewall.serviceCount || 0))
                    : String(root.firewall.note || qsTr("Status unavailable"))
                settingsId: "firewallOptions"
                owner: "firewall adapter"
                settingsKey: "firewall.rules"
                trailingAction: qsTr("Open…")
                showSeparator: false
                onTrailingActionClicked: root.firewallDialog.show()
            }
        }

        // ── Other services ────────────────────────────────────────────────
        SettingsGroup {
            id: servicesGroup
            width: parent.width
            title: qsTr("Other Services")
            visible: root.ready

            Repeater {
                id: servicesRepeater
                model: root.otherServices
                delegate: SettingsRow {
                    id: serviceRow
                    required property var modelData
                    required property int index
                    objectName: "networkServiceRow"
                    width: parent.width
                    label: String(serviceRow.modelData.id)
                    description: qsTr("%1 · %2 · %3")
                        .arg(String(serviceRow.modelData.kindLabel))
                        .arg(String(serviceRow.modelData.stateLabel))
                        .arg(String(serviceRow.modelData.address || ""))
                    settingsId: "services"
                    owner: "NetworkManager"
                    settingsKey: "Devices"
                    showSeparator: serviceRow.index < root.otherServices.length - 1
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Button {
                            objectName: "networkServiceUp"
                            text: qsTr("Up")
                            variant: "ghost"
                            enabled: Number(serviceRow.modelData.order) > 0
                                     && !root.readOnly
                            onClicked: Settings.moveNetworkService(
                                String(serviceRow.modelData.uuid), -1)
                        }
                        Button {
                            objectName: "networkServiceDown"
                            text: qsTr("Down")
                            variant: "ghost"
                            enabled: Number(serviceRow.modelData.order)
                                     < root.services.length - 1 && !root.readOnly
                            onClicked: Settings.moveNetworkService(
                                String(serviceRow.modelData.uuid), 1)
                        }
                        Text {
                            text: qsTr("Auto")
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Toggle {
                            id: serviceAutoToggle
                            objectName: "networkServiceAutoconnect"
                            text: ""
                            accessibleName: qsTr("Auto-connect %1")
                                .arg(serviceRow.modelData.id)
                            enabled: !root.readOnly
                            onToggled: (checked) => Settings.setNetworkAutoconnect(
                                String(serviceRow.modelData.uuid), checked)

                            Binding {
                                target: serviceAutoToggle
                                property: "checked"
                                value: serviceRow.modelData.autoconnect === true
                            }
                        }
                        Button {
                            objectName: "networkServiceConnect"
                            text: serviceRow.modelData.connected === true
                                ? qsTr("Disconnect") : qsTr("Connect")
                            enabled: !root.readOnly
                            onClicked: {
                                if (serviceRow.modelData.connected === true)
                                    Settings.deactivateNetworkService(
                                        String(serviceRow.modelData.uuid));
                                else
                                    Settings.activateNetworkService(
                                        String(serviceRow.modelData.uuid));
                            }
                        }
                    }
                }
            }

            SettingsRow {
                id: addServiceRow
                width: parent.width
                label: qsTr("Add Service…")
                description: qsTr("Create an Ethernet, Wi-Fi, or VPN service.")
                settingsId: "addService"
                owner: "NetworkManager"
                settingsKey: "AddConnection"
                showSeparator: false
                trailingAction: qsTr("Add…")
                onTrailingActionClicked: {
                    addServiceName.text = "";
                    addServiceGateway.text = "";
                    addServiceDialog.show();
                }
            }
        }

        // ── VPN ───────────────────────────────────────────────────────────
        SettingsGroup {
            id: vpnGroup
            width: parent.width
            title: qsTr("VPN")
            visible: root.vpnReady

            Repeater {
                id: vpnConnectionRepeater
                model: root.vpnConnections
                delegate: SettingsRow {
                    id: connectionRow
                    required property var modelData
                    required property int index
                    objectName: "networkVpnRow"
                    width: parent.width
                    label: String(connectionRow.modelData.id)
                    description: String(connectionRow.modelData.label)
                    settingsId: "vpn"
                    owner: "NetworkManager"
                    settingsKey: "Settings.Connection"
                    showSeparator: connectionRow.index < root.vpnConnections.length - 1
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Toggle {
                            id: connectionToggle
                            objectName: "networkVpnToggle"
                            text: ""
                            accessibleName: qsTr("Connect %1")
                                .arg(connectionRow.modelData.id)
                            enabled: !root.readOnly
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
                            id: vpnAutoToggle
                            objectName: "networkVpnAutoconnect"
                            text: ""
                            accessibleName: qsTr("Auto-connect %1")
                                .arg(connectionRow.modelData.id)
                            enabled: !root.readOnly
                            onToggled: (checked) => Settings.setVpnAutoconnect(
                                String(connectionRow.modelData.uuid), checked)

                            Binding {
                                target: vpnAutoToggle
                                property: "checked"
                                value: connectionRow.modelData.autoconnect === true
                            }
                        }
                    }
                }
            }

            SettingsRow {
                id: emptyNote
                objectName: "networkEmptyNote"
                width: parent.width
                visible: root.vpnConnections.length === 0
                label: qsTr("No VPN connection is configured.")
                description: qsTr("Add a VPN below to connect.")
                showSeparator: root.vpnPlugins.length > 0
            }

            SettingsRow {
                id: addVpnRow
                width: parent.width
                label: qsTr("Add VPN…")
                description: root.vpnPlugins.length > 0
                    ? qsTr("%1 plugin(s) installed").arg(root.vpnPlugins.length)
                    : qsTr("No NetworkManager VPN plugin is installed.")
                settingsId: "vpn"
                owner: "NetworkManager"
                settingsKey: "Settings.Connection"
                showSeparator: false
                trailingAction: root.vpnPlugins.length > 0 ? qsTr("Add…") : ""
                onTrailingActionClicked: {
                    addVpnName.text = "";
                    addVpnGateway.text = "";
                    addVpnDialog.show();
                }
            }
        }

        // ── Advanced ──────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width

            SettingsRow {
                id: advancedRow
                width: parent.width
                label: qsTr("Advanced…")
                description: qsTr("TCP/IP, hardware, and the active service.")
                settingsId: "advanced"
                owner: "NetworkManager"
                settingsKey: "nm.connection"
                showSeparator: false
                trailingAction: qsTr("Open…")
                onTrailingActionClicked: root.advancedDialog.show()
            }
        }

        // ── Read-only degradation ─────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready && root.readOnly

            Text {
                id: readOnlyNoteItem
                objectName: "networkReadOnlyNote"
                width: parent.width
                text: root.readOnlyMessage.length > 0
                    ? qsTr("Connections can be viewed but not changed: %1")
                          .arg(root.readOnlyMessage)
                    : qsTr("Connections can be viewed but not changed.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Host absent ───────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "networkAbsenceNote"
                kind: "disabled-with-reason"
                reason: qsTr("The network service (NetworkManager) is not running, "
                             + "so services and VPN connections are unavailable.")
                width: parent.width
            }
        }
    }

    // ── Firewall options ──────────────────────────────────────────────────
    Dialog {
        id: firewallDialog
        objectName: "networkFirewallDialog"
        title: qsTr("Firewall Options")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                visible: !root.firewallStatusKnown
                text: String(root.firewall.note || qsTr("This firewall cannot report status."))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: root.firewallStatusKnown
                text: qsTr("Open ports")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Column {
                width: parent.width
                spacing: Theme.primitive.spacing.xxs

                Repeater {
                    id: firewallPortRepeater
                    model: root.firewall.ports !== undefined ? root.firewall.ports : []
                    delegate: Row {
                        required property var modelData
                        width: parent.width
                        height: Theme.controls.settingsRow.height

                        Text {
                            width: parent.width - closePort.width
                            text: String(modelData.label)
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Button {
                            id: closePort
                            text: qsTr("Close")
                            variant: "ghost"
                            enabled: root.firewallStatusKnown && !root.readOnly
                            onClicked: Settings.closeFirewallPort(
                                Number(modelData.port), String(modelData.protocol))
                        }
                    }
                }

                Row {
                    width: parent.width
                    spacing: Theme.primitive.spacing.xs

                    TextInput {
                        id: addPortField
                        objectName: "networkFirewallPortField"
                        width: 96
                        height: Theme.controls.button.height
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        enabled: root.firewallStatusKnown && !root.readOnly
                    }
                    Select {
                        id: addPortProtocol
                        objectName: "networkFirewallProtocol"
                        model: [ "tcp", "udp" ]
                        enabled: root.firewallStatusKnown && !root.readOnly
                    }
                    Button {
                        objectName: "networkFirewallOpenPort"
                        text: qsTr("Open Port")
                        enabled: root.firewallStatusKnown && !root.readOnly
                        onClicked: root.addPort()
                    }
                }
            }

            Text {
                width: parent.width
                visible: root.firewallStatusKnown
                text: qsTr("Open services")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Column {
                width: parent.width
                spacing: Theme.primitive.spacing.xxs

                Repeater {
                    id: firewallServiceRepeater
                    model: root.firewall.services !== undefined
                        ? root.firewall.services : []
                    delegate: Row {
                        required property var modelData
                        width: parent.width
                        height: Theme.controls.settingsRow.height

                        Text {
                            width: parent.width - closeService.width
                            text: String(modelData)
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Button {
                            id: closeService
                            text: qsTr("Close")
                            variant: "ghost"
                            enabled: root.firewallStatusKnown && !root.readOnly
                            onClicked: Settings.closeFirewallService(String(modelData))
                        }
                    }
                }

                Row {
                    width: parent.width
                    spacing: Theme.primitive.spacing.xs

                    TextInput {
                        id: addServiceField
                        objectName: "networkFirewallServiceField"
                        width: 160
                        height: Theme.controls.button.height
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        enabled: root.firewallStatusKnown && !root.readOnly
                    }
                    Button {
                        objectName: "networkFirewallOpenService"
                        text: qsTr("Open Service")
                        enabled: root.firewallStatusKnown && !root.readOnly
                        onClicked: root.addFirewallService()
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Close")
                onClicked: firewallDialog.hide()
            }
        }
    }

    // ── Add service ───────────────────────────────────────────────────────
    Dialog {
        id: addServiceDialog
        objectName: "networkAddServiceDialog"
        title: qsTr("Add Service")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            TextInput {
                id: addServiceName
                objectName: "networkAddServiceName"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Select {
                id: addServiceKind
                objectName: "networkAddServiceKind"
                model: root.vpnPlugins.length > 0
                    ? [ { label: qsTr("Ethernet"), value: "ethernet" },
                        { label: qsTr("Wi-Fi"), value: "wifi" },
                        { label: qsTr("VPN"), value: "vpn" } ]
                    : [ { label: qsTr("Ethernet"), value: "ethernet" },
                        { label: qsTr("Wi-Fi"), value: "wifi" } ]
            }
            Select {
                id: addServicePlugin
                objectName: "networkAddServicePlugin"
                visible: addServiceKind.currentValue === "vpn"
                model: root.vpnPlugins.map(function(plugin) {
                    return { label: String(plugin.label), value: String(plugin.serviceType) };
                })
            }
            TextInput {
                id: addServiceGateway
                objectName: "networkAddServiceGateway"
                visible: addServiceKind.currentValue === "vpn"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Cancel")
                onClicked: addServiceDialog.hide()
            }
            Button {
                objectName: "networkAddServiceConfirm"
                text: qsTr("Add")
                variant: "primary"
                onClicked: root.addService()
            }
        }
    }

    // ── Add VPN ───────────────────────────────────────────────────────────
    Dialog {
        id: addVpnDialog
        objectName: "networkAddVpnDialog"
        title: qsTr("Add VPN")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Select {
                id: addVpnPlugin
                objectName: "networkAddVpnPlugin"
                model: root.vpnPlugins.map(function(plugin) {
                    return { label: String(plugin.label), value: String(plugin.serviceType) };
                })
            }
            TextInput {
                id: addVpnName
                objectName: "networkAddVpnName"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: addVpnGateway
                objectName: "networkAddVpnGateway"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("Credentials are edited in the full connection editor.")
                color: Theme.color.textTertiary
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
                objectName: "networkAddVpnConfirm"
                text: qsTr("Add")
                variant: "primary"
                onClicked: root.addVpn()
            }
        }
    }

    // ── Advanced ──────────────────────────────────────────────────────────
    Dialog {
        id: advancedDialog
        objectName: "networkAdvancedDialog"
        title: qsTr("Network Advanced")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Repeater {
                model: root.services
                delegate: Text {
                    required property var modelData
                    width: parent.width
                    text: qsTr("%1 — %2 · %3 · %4")
                        .arg(String(modelData.id))
                        .arg(String(modelData.kindLabel))
                        .arg(String(modelData.address || qsTr("No address")))
                        .arg(String(modelData.interface || qsTr("No interface")))
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }
            Text {
                width: parent.width
                text: qsTr("DNS, proxies, and MAC randomization are edited in the "
                           + "full connection editor; this first pass shows the live "
                           + "service state.")
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
