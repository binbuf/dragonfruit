// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Network pane (T225; T272 service list; T288 visual parity).
//
// One NetworkManager model serves the Wi-Fi disclosure, the service list and
// order, the VPN list, and the firewall sheet. The pane never caches a
// connected state: every write goes to the bridge host and its re-read pushes
// the new view back. NetworkManager owns services, order
// (`autoconnect-priority`), and VPN connections; the firewall adapter owns
// detection and the firewall writes.
//
// T288 follows the mockup's structure
// (docs/design/25-system-settings-visual-parity.md §5.2): one service card
// with the Wi-Fi and Firewall rows, an Other Services card of the real
// NetworkManager services, and a More popup with Set Service Order…/Add
// Service…. Rows use `SettingsServiceRow`. The VPN section, the per-service
// details sheet, the firewall options sheet, and Add Service stay.
//
// T272 keeps the sections independent: a missing NetworkManager does not hide
// the firewall, and a read-only VPN adapter does not disable it.
Item {
    id: root

    readonly property var view: Settings.network
    readonly property bool available: Settings.networkAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var services: root.view.services !== undefined
        ? root.view.services : []
    readonly property var firewall: root.view.firewall !== undefined
        ? root.view.firewall : ({})
    readonly property var vpnView: Settings.vpn
    readonly property bool vpnAvailable: Settings.vpnAvailable
    readonly property bool vpnReady: root.vpnAvailable && root.vpnView.state === "available"
    readonly property var vpnConnections: root.vpnView.connections !== undefined
        ? root.vpnView.connections : []
    readonly property var vpnPlugins: root.vpnView.plugins !== undefined
        ? root.vpnView.plugins : []
    readonly property bool vpnWireguardNative: root.vpnView.wireguardNative === true
    readonly property bool vpnEditorAvailable: root.vpnView.editorAvailable === true
    readonly property string vpnEditorName: String(root.vpnView.editor || "")
    readonly property bool vpnCanAdd: root.vpnWireguardNative
        || (root.vpnPlugins.length > 0 && root.vpnEditorAvailable)
    readonly property string vpnAddReason: String(root.vpnView.addReason || "")
    readonly property int connectionCount: root.vpnConnections.length

    // T272: the two halves degrade independently. A read-only NetworkManager
    // never disables the firewall, and a read-only VPN adapter never disables
    // it either.
    readonly property bool networkReadOnly: root.view.readOnly === true
    readonly property bool vpnReadOnly: root.vpnView.readOnly === true
    readonly property bool firewallReadOnly: root.firewall.readOnly === true
    readonly property string networkReadOnlyMessage: String(root.view.note || "")
    readonly property string vpnReadOnlyMessage: String(root.vpnView.note || "")
    readonly property string firewallReadOnlyMessage: String(
        root.firewall.readOnlyNote || "")
    readonly property bool firewallDetected: root.firewall.detected === true
    readonly property bool firewallStatusKnown: root.firewall.statusKnown === true
    readonly property bool firewallEnabled: root.firewall.enabled === true
    readonly property bool firewallWritable: root.firewallDetected
        && root.firewallStatusKnown && !root.firewallReadOnly
    readonly property string firewallBackendLabel: String(root.firewall.backendLabel || "")
    readonly property string firewallNote: String(root.firewall.note || "")
    readonly property bool firewallRulesPresent: root.firewall.rulesPresent === true
    readonly property string firewallManageWith: String(root.firewall.manageWith || "")
    readonly property string firewallAuthNote: String(root.firewall.authNote || "")

    // Other services: everything except Wi-Fi (its own disclosure) and VPN (its
    // own group), matching the reference's Other Services list.
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

    // The service whose Details sheet or actions menu is open.
    property var detailsService: null
    property var menuService: null
    readonly property var detailRows: root.detailsService !== null
        ? root.buildDetailRows(root.detailsService) : []
    readonly property string writeState: String(Settings.networkWriteState || "idle")
    readonly property string writeError: String(Settings.networkWriteError || "")
    readonly property bool writeFailed: root.writeState === "failed"
        || root.writeState === "denied" || root.writeState === "absent"

    signal paneRequested(string paneId)

    // Test surface (tst_settings_network.qml).
    property alias wifiRow: wifiRow
    property alias serviceListGroup: serviceListGroup
    // T288: Wi-Fi and Firewall are the mockup's two service rows in one card;
    // the older alias keeps the absence tests anchored to that card.
    property alias firewallGroup: serviceListGroup
    property alias firewallRow: firewallRow
    property alias firewallToggle: firewallRow.serviceSwitch
    property alias firewallOptionsRow: firewallOptionsRow
    property alias firewallDialog: firewallDialog
    property alias servicesGroup: servicesGroup
    property alias servicesRepeater: servicesRepeater
    property alias addServiceRow: addServiceRow
    property alias addServiceDialog: addServiceDialog
    property alias addServiceResult: addServiceResult
    property alias moreButton: moreButton
    property alias moreMenu: moreMenu
    property alias serviceOrderDialog: serviceOrderDialog
    property alias serviceOrderRepeater: serviceOrderRepeater
    property alias vpnGroup: vpnGroup
    property alias vpnConnectionRepeater: vpnConnectionRepeater
    property alias connectionRepeater: vpnConnectionRepeater
    property alias addVpnRow: addVpnRow
    property alias addVpnDialog: addVpnDialog
    property alias absenceNote: absenceNote
    property alias readOnlyNote: readOnlyNoteItem
    property alias emptyNote: emptyNote
    property alias vpnAbsenceNote: vpnAbsenceNote
    property alias detailsDialog: detailsDialog
    property alias serviceMenu: serviceMenu
    property alias firewallPortRepeater: firewallPortRepeater
    property alias firewallServiceRepeater: firewallServiceRepeater
    property alias writeNote: writeNote

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

    function statusColor(status) {
        if (status === "green")
            return Theme.color.success;
        if (status === "yellow")
            return Theme.color.warning;
        if (status === "red")
            return Theme.color.danger;
        return Theme.color.textTertiary;
    }

    // The live view carries `state`; the fixture also carries a precomputed
    // `status`. Derive from `state` so a live NetworkManager read draws the
    // same dot as the fixture.
    function serviceStatus(service) {
        var state = String((service || {}).state || "");
        if (state === "connected")
            return "green";
        if (state === "connecting" || state === "disconnecting")
            return "yellow";
        if (state === "failed")
            return "red";
        return "gray";
    }

    function prefixMask(prefix) {
        var bits = Number(prefix);
        if (!isFinite(bits) || bits <= 0)
            return "";
        var mask = bits >= 32 ? 0xffffffff
                              : (0xffffffff << (32 - bits)) >>> 0;
        return [ (mask >>> 24) & 0xff, (mask >>> 16) & 0xff,
                 (mask >>> 8) & 0xff, mask & 0xff ].join(".");
    }

    function meteredLabel(value) {
        var code = Number(value);
        if (code === 1)
            return qsTr("Yes");
        if (code === 2)
            return qsTr("No");
        if (code === 3)
            return qsTr("Yes (automatic)");
        if (code === 4)
            return qsTr("No (automatic)");
        return qsTr("Unknown");
    }

    function listLabel(list, empty) {
        if (list === undefined || list === null || list.length === 0)
            return empty;
        return list.join(", ");
    }

    function buildDetailRows(service) {
        var s = service || {};
        var ipv6 = String(s.ipv6 || "");
        var ipv6Addresses = root.listLabel(s.ipv6Addresses, "");
        if (ipv6Addresses.length > 0)
            ipv6 = ipv6.length > 0 ? ipv6 + " · " + ipv6Addresses
                                   : ipv6Addresses;
        return [
            { label: qsTr("Status"), value: String(s.stateLabel || "") },
            { label: qsTr("Interface"), value: String(s.interface || qsTr("None")) },
            { label: qsTr("Hardware Address"),
              value: String(s.hardware || qsTr("Unavailable")) },
            { label: qsTr("IPv4 Address"),
              value: String(s.address || qsTr("None")) },
            { label: qsTr("Subnet Mask"),
              value: root.prefixMask(s.prefix) || qsTr("None") },
            { label: qsTr("Router"), value: String(s.gateway || qsTr("None")) },
            { label: qsTr("DNS Servers"), value: root.listLabel(s.dns, qsTr("None")) },
            { label: qsTr("Search Domains"),
              value: root.listLabel(s.searchDomains, qsTr("None")) },
            { label: qsTr("IPv6"), value: ipv6.length > 0 ? ipv6 : qsTr("None") },
            { label: qsTr("Proxy"), value: String(s.proxy || qsTr("Off")) },
            { label: qsTr("MAC Randomization"),
              value: String(s.clonedMac || qsTr("Preserve")) },
            { label: qsTr("Metered"), value: root.meteredLabel(s.metered) },
            { label: qsTr("802.1X"),
              value: String(s.keyMgmt || qsTr("Not configured")) },
            { label: qsTr("Auto-connect"),
              value: s.autoconnect === true ? qsTr("On") : qsTr("Off") },
        ];
    }

    function openDetails(service) {
        root.detailsService = service;
        detailsDialog.show();
    }

    function buildServiceMenu(service) {
        var s = service || {};
        var connected = s.connected === true;
        var order = Number(s.order || 0);
        return [
            { label: connected ? qsTr("Disconnect") : qsTr("Connect"),
              action: connected ? "deactivate" : "activate", key: "toggle" },
            { type: "separator" },
            { label: qsTr("Move Up"), action: "up", enabled: order > 0 },
            { label: qsTr("Move Down"), action: "down",
              enabled: order < root.services.length - 1 },
            { type: "separator" },
            { label: qsTr("Make Inactive"), action: "inactive",
              enabled: connected },
            { label: qsTr("Delete Service"), action: "delete" },
            { type: "separator" },
            { label: qsTr("Add Service…"), action: "add" }
        ];
    }

    function openServiceMenu(service, anchor) {
        root.menuService = service;
        serviceMenu.model = root.buildServiceMenu(service);
        serviceMenu.anchorItem = anchor;
        serviceMenu.show();
    }

    // The mockup's service-list glyph: Wi-Fi keeps the Wi-Fi mark, VPN the
    // shield, and the wired/virtual kinds the network globe.
    function serviceGlyph(service) {
        var kind = String((service || {}).kind || "");
        if (kind === "wifi")
            return "wifi";
        if (kind === "vpn" || kind === "wireguard")
            return "vpn";
        return "network";
    }

    function openServiceOrder() {
        root.moreMenu.hide();
        serviceOrderDialog.show();
    }

    function moveService(uuid, delta) {
        if (!root.ready || root.networkReadOnly || !uuid)
            return;
        Settings.moveNetworkService(String(uuid), Number(delta));
    }

    function runServiceAction(action) {
        var service = root.menuService;
        if (service === null || service === undefined)
            return;
        var uuid = String(service.uuid || "");
        if (action === "activate")
            Settings.activateNetworkService(uuid);
        else if (action === "deactivate" || action === "inactive")
            Settings.deactivateNetworkService(uuid);
        else if (action === "up")
            Settings.moveNetworkService(uuid, -1);
        else if (action === "down")
            Settings.moveNetworkService(uuid, 1);
        else if (action === "delete")
            Settings.forgetNetworkService(uuid);
        else if (action === "add")
            root.openAddService();
        else if (action === "details")
            root.openDetails(service);
    }

    function toggleFirewall() {
        if (!root.firewallWritable)
            return;
        Settings.setFirewallEnabled(!root.firewallEnabled);
    }

    function openAddService() {
        addServiceName.text = "";
        addServiceInterface.text = "";
        addServiceResult.text = "";
        addServiceDialog.show();
    }

    function addService() {
        var name = addServiceName.text.trim();
        var kind = addServiceKind.currentValue || "ethernet";
        if (name.length === 0) {
            addServiceResult.text = qsTr("A service name is required.");
            return;
        }
        if ((kind === "bridge" || kind === "bond" || kind === "vlan")
                && addServiceInterface.text.trim().length === 0) {
            addServiceResult.text = qsTr("An interface name is required.");
            return;
        }
        // VPN is not an Add Service kind: a VPN profile must be finishable, so
        // it is created from the VPN group's Add row instead.
        Settings.addNetworkService(name, kind, "", "", addServiceInterface.text);
        addServiceResult.text = qsTr("Adding “%1”…").arg(name);
    }

    function addVpn() {
        addVpnDialog.prepare();
    }

    function submitAddVpn(kind, plugin, values) {
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

    function completeVpnSetup(uuid) {
        Settings.editVpn(String(uuid));
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

    // One write outcome line per pane, so a denial or failure is never silent.
    Connections {
        target: Settings
        function onNetworkWriteChanged() {
            var state = String(Settings.networkWriteState || "");
            if (state === "applied" || state === "failed"
                    || state === "denied" || state === "absent") {
                if (String(Settings.networkWriteOperation || "") === "add") {
                    addServiceResult.text = state === "applied"
                        ? qsTr("Service added.")
                        : String(Settings.networkWriteError || qsTr("The service was not added."));
                }
            }
        }
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

        // ── Service list (Wi-Fi + Firewall) ───────────────────────────────
        SettingsGroup {
            id: serviceListGroup
            objectName: "networkServiceList"
            width: parent.width

            SettingsServiceRow {
                id: wifiRow
                objectName: "networkWifiServiceRow"
                width: parent.width
                label: qsTr("Wi-Fi")
                statusText: {
                    if (!root.ready)
                        return qsTr("Unavailable");
                    var ssid = root.view.activeSsid !== undefined
                        ? String(root.view.activeSsid) : "";
                    if (ssid.length === 0)
                        return root.view.radioEnabled === true
                            ? qsTr("Not connected") : qsTr("Off");
                    return qsTr("Connected to %1").arg(ssid);
                }
                serviceActive: root.ready && root.view.activeSsid !== undefined
                               && String(root.view.activeSsid).length > 0
                serviceGlyph: "wifi"
                settingsId: "wifi"
                owner: "NetworkManager"
                settingsKey: "WirelessEnabled"
                disclosure: true
                enabled: root.ready && !root.networkReadOnly
                showSeparator: true
                onDisclosureClicked: root.paneRequested("wifi")
            }

            SettingsServiceRow {
                id: firewallRow
                objectName: "networkFirewallRow"
                width: parent.width
                label: qsTr("Firewall")
                statusText: {
                    if (!root.firewallDetected)
                        return qsTr("No firewall detected");
                    var backend = root.firewallBackendLabel;
                    if (!root.firewallStatusKnown)
                        return qsTr("%1 · Unavailable").arg(backend);
                    return qsTr("%1 · %2").arg(backend)
                                           .arg(root.firewallEnabled
                                                ? qsTr("On") : qsTr("Off"));
                }
                serviceActive: root.firewallEnabled
                serviceGlyph: "privacy"
                serviceColor: root.firewallEnabled ? Theme.color.accent
                                                   : Theme.color.textTertiary
                trailingKind: "switch"
                switchBound: true
                switchEnabled: root.firewallWritable
                settingsId: "firewall"
                owner: "firewall adapter"
                settingsKey: "firewall.status"
                showSeparator: true
                onSwitchToggled: (checked) => Settings.setFirewallEnabled(checked)
            }

            SettingsRow {
                id: firewallOptionsRow
                width: parent.width
                label: qsTr("Firewall Options…")
                description: {
                    if (!root.firewallStatusKnown)
                        return root.firewallNote.length > 0
                            ? root.firewallNote : qsTr("Status unavailable");
                    if (!root.firewallRulesPresent && root.firewallNote.length > 0)
                        return root.firewallNote;
                    var zone = String(root.firewall.defaultZone || "");
                    return zone.length > 0
                        ? qsTr("%1 zone · %2 open ports, %3 services")
                              .arg(zone)
                              .arg(Number(root.firewall.portCount || 0))
                              .arg(Number(root.firewall.serviceCount || 0))
                        : qsTr("%1 open ports, %2 services")
                              .arg(Number(root.firewall.portCount || 0))
                              .arg(Number(root.firewall.serviceCount || 0));
                }
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

            Repeater {
                id: servicesRepeater
                model: root.otherServices
                delegate: SettingsServiceRow {
                    id: serviceRow
                    required property var modelData
                    required property int index
                    objectName: "networkServiceRow"
                    width: parent.width
                    label: String(serviceRow.modelData.id)
                    statusText: qsTr("%1 · %2 · %3")
                        .arg(String(serviceRow.modelData.kindLabel))
                        .arg(String(serviceRow.modelData.stateLabel))
                        .arg(String(serviceRow.modelData.address || qsTr("No address")))
                    serviceActive: serviceRow.modelData.state === "connected"
                    serviceGlyph: root.serviceGlyph(serviceRow.modelData)
                    serviceColor: root.statusColor(root.serviceStatus(
                        serviceRow.modelData))
                    settingsId: "services"
                    owner: "NetworkManager"
                    settingsKey: "Devices"
                    showSeparator: serviceRow.index < root.otherServices.length - 1
                    disclosure: true
                    trailingAction: qsTr("⋯")
                    trailingActionEnabled: root.ready && !root.networkReadOnly
                    onDisclosureClicked: root.openDetails(serviceRow.modelData)
                    onTrailingActionClicked: root.openServiceMenu(
                        serviceRow.modelData, serviceRow)
                }
            }

            SettingsRow {
                objectName: "networkServicesEmpty"
                width: parent.width
                visible: root.ready && root.otherServices.length === 0
                label: qsTr("No other services.")
                description: qsTr("Add a service below.")
                showSeparator: true
            }

            SettingsRow {
                id: addServiceRow
                width: parent.width
                label: qsTr("Add Service…")
                description: root.ready
                    ? qsTr("Create an Ethernet, Wi-Fi, bridge, bond, or VLAN "
                           + "service. Add a VPN from the VPN section so it can "
                           + "be completed.")
                    : qsTr("NetworkManager is not running.")
                settingsId: "addService"
                owner: "NetworkManager"
                settingsKey: "AddConnection"
                showSeparator: false
                trailingAction: qsTr("Add…")
                enabled: root.ready && !root.networkReadOnly
                onTrailingActionClicked: root.openAddService()
            }
        }

        // ── More footer (mockup) ──────────────────────────────────────────
        SettingsFooter {
            id: moreFooter
            objectName: "networkMoreFooter"
            width: parent.width

            Button {
                id: moreButton
                objectName: "networkMoreButton"
                text: qsTr("More")
                onClicked: root.moreMenu.show()
            }
        }

        // ── VPN ───────────────────────────────────────────────────────────
        SettingsGroup {
            id: vpnGroup
            width: parent.width
            title: qsTr("VPN")

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

                        Rectangle {
                            objectName: "networkVpnStatusDot"
                            width: 8
                            height: 8
                            radius: 4
                            anchors.verticalCenter: parent.verticalCenter
                            color: connectionRow.modelData.connected === true
                                ? Theme.color.success : Theme.color.textTertiary
                        }
                        Toggle {
                            id: connectionToggle
                            objectName: "networkVpnToggle"
                            text: ""
                            accessibleName: qsTr("Connect %1")
                                .arg(connectionRow.modelData.id)
                            visible: connectionRow.modelData.needsSetup !== true
                            enabled: root.vpnReady && !root.vpnReadOnly
                            onToggled: (checked) =>
                                root.setConnected(connectionRow.modelData.uuid, checked)

                            Binding {
                                target: connectionToggle
                                property: "checked"
                                value: root.isConnected(connectionRow.modelData.uuid)
                            }
                        }
                        Button {
                            objectName: "networkVpnCompleteSetup"
                            text: qsTr("Complete Setup…")
                            variant: "ghost"
                            visible: connectionRow.modelData.needsSetup === true
                                     && root.vpnEditorAvailable
                            enabled: root.vpnReady && !root.vpnReadOnly
                            onClicked: root.completeVpnSetup(
                                String(connectionRow.modelData.uuid))
                        }
                        Text {
                            text: qsTr("Auto")
                            visible: connectionRow.modelData.needsSetup !== true
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
                            visible: connectionRow.modelData.needsSetup !== true
                            enabled: root.vpnReady && !root.vpnReadOnly
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
                id: vpnAbsenceNote
                objectName: "networkVpnAbsenceNote"
                width: parent.width
                visible: !root.vpnReady
                label: qsTr("VPN is unavailable.")
                description: qsTr("NetworkManager is not running, so VPN connections "
                                  + "cannot be read or changed.")
                showSeparator: true
            }

            SettingsRow {
                id: emptyNote
                objectName: "networkEmptyNote"
                width: parent.width
                visible: root.vpnReady && root.vpnConnections.length === 0
                label: qsTr("No VPN connection is configured.")
                description: qsTr("Add a VPN below to connect.")
                showSeparator: root.vpnPlugins.length > 0
            }

            SettingsRow {
                id: addVpnRow
                width: parent.width
                label: qsTr("Add VPN…")
                description: {
                    if (!root.vpnCanAdd) {
                        if (root.vpnPlugins.length > 0 && !root.vpnEditorAvailable)
                            return qsTr("Install %1 to finish plugin profiles.")
                                .arg(root.vpnEditorName.length > 0
                                     ? root.vpnEditorName : "nm-connection-editor");
                        return root.vpnAddReason.length > 0
                            ? root.vpnAddReason
                            : qsTr("No VPN plugin is installed and native "
                                   + "WireGuard is not available.");
                    }
                    if (root.vpnPlugins.length > 0 && !root.vpnEditorAvailable)
                        return qsTr("Install %1 to finish plugin profiles, or "
                                    + "add a native WireGuard profile.")
                            .arg(root.vpnEditorName.length > 0
                                 ? root.vpnEditorName : "nm-connection-editor");
                    var parts = [];
                    if (root.vpnWireguardNative)
                        parts.push(qsTr("WireGuard (native)"));
                    if (root.vpnPlugins.length > 0)
                        parts.push(qsTr("%1 plugin(s)").arg(root.vpnPlugins.length));
                    return parts.join(qsTr(" · "));
                }
                settingsId: "vpn"
                owner: "NetworkManager"
                settingsKey: "Settings.Connection"
                showSeparator: false
                trailingAction: root.vpnCanAdd ? qsTr("Add…") : ""
                enabled: root.vpnReady && !root.vpnReadOnly && root.vpnCanAdd
                onTrailingActionClicked: root.addVpn()
            }
        }

        // ── Write outcome ─────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.writeFailed

            Text {
                id: writeNote
                objectName: "networkWriteNote"
                width: parent.width
                text: root.writeError.length > 0
                    ? root.writeError
                    : qsTr("The last network change did not apply.")
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Read-only degradation (NetworkManager only) ───────────────────
        SettingsGroup {
            width: parent.width
            visible: root.networkReadOnly

            Text {
                id: readOnlyNoteItem
                objectName: "networkReadOnlyNote"
                width: parent.width
                text: root.networkReadOnlyMessage.length > 0
                    ? qsTr("Connections can be viewed but not changed: %1")
                          .arg(root.networkReadOnlyMessage)
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
                             + "so services and VPN connections are unavailable. "
                             + "The firewall keeps its own state and stays usable.")
                width: parent.width
            }
        }
    }

    // ── Service actions menu ──────────────────────────────────────────────
    ContextMenu {
        id: serviceMenu
        objectName: "networkServiceMenu"
        accessibleName: qsTr("Service actions")
        onTriggered: (index, item) => root.runServiceAction(item.action)
    }

    // ── More menu (mockup: Set Service Order… / Add Service…) ─────────────
    ContextMenu {
        id: moreMenu
        objectName: "networkMoreMenu"
        accessibleName: qsTr("More network actions")
        anchorItem: moreButton
        model: [
            { label: qsTr("Set Service Order…"), action: "order" },
            { label: qsTr("Add Service…"), action: "add" }
        ]
        onTriggered: (index, item) => {
            if (item.action === "order")
                root.openServiceOrder();
            else if (item.action === "add")
                root.openAddService();
        }
    }

    // ── Service order (real NetworkManager priority writes) ───────────────
    Dialog {
        id: serviceOrderDialog
        objectName: "networkServiceOrderDialog"
        title: qsTr("Set Service Order")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Text {
                width: parent.width
                text: qsTr("NetworkManager tries services in this order. "
                           + "Moving a service writes its autoconnect priority.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: serviceOrderRepeater
                model: root.services
                delegate: Row {
                    required property var modelData
                    required property int index
                    width: parent.width
                    height: Theme.controls.settingsRow.height
                    spacing: Theme.primitive.spacing.xs

                    Text {
                        width: parent.width - orderButtons.width
                        text: String(modelData.id)
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Text.ElideRight
                    }

                    Row {
                        id: orderButtons
                        spacing: Theme.primitive.spacing.xxs
                        anchors.verticalCenter: parent.verticalCenter

                        Button {
                            objectName: "networkServiceOrderUp"
                            text: qsTr("Up")
                            variant: "ghost"
                            enabled: root.ready && !root.networkReadOnly
                                     && index > 0
                            onClicked: root.moveService(modelData.uuid, -1)
                        }
                        Button {
                            objectName: "networkServiceOrderDown"
                            text: qsTr("Down")
                            variant: "ghost"
                            enabled: root.ready && !root.networkReadOnly
                                     && index < root.services.length - 1
                            onClicked: root.moveService(modelData.uuid, 1)
                        }
                    }
                }
            }

            Text {
                width: parent.width
                visible: !root.ready
                text: qsTr("NetworkManager is not running, so the service "
                           + "order cannot be changed.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Close")
                onClicked: serviceOrderDialog.hide()
            }
        }
    }

    // ── Service details ───────────────────────────────────────────────────
    Dialog {
        id: detailsDialog
        objectName: "networkDetailsDialog"
        title: root.detailsService !== null
            ? qsTr("Details — %1").arg(String(root.detailsService.id || ""))
            : qsTr("Details")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Repeater {
                model: root.detailRows
                delegate: Row {
                    required property var modelData
                    width: parent.width
                    height: Math.max(Theme.controls.settingsRow.height * 0.8,
                                     detailValue.implicitHeight)
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        id: detailLabel
                        width: Math.min(180, parent.width * 0.45)
                        text: String(modelData.label)
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.verticalCenter: parent.verticalCenter
                        wrapMode: Text.WordWrap
                    }
                    Text {
                        id: detailValue
                        objectName: "networkDetailValue"
                        width: parent.width - detailLabel.width - parent.spacing
                        text: String(modelData.value)
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.verticalCenter: parent.verticalCenter
                        wrapMode: Text.WordWrap
                    }
                }
            }

            Text {
                width: parent.width
                text: qsTr("DNS, proxies, MAC randomization, metered, IPv6, and "
                           + "802.1X are read live from NetworkManager. The full "
                           + "connection editor owns the writes.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Close")
                onClicked: detailsDialog.hide()
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
                text: root.firewallDetected
                    ? qsTr("Backend: %1 · Zone: %2")
                          .arg(String(root.firewall.backendLabel || qsTr("Unknown")))
                          .arg(String(root.firewall.defaultZone || qsTr("None")))
                    : qsTr("No firewall detected. Install firewalld, ufw, or nftables "
                           + "to manage rules.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: root.firewallNote.length > 0
                text: root.firewallNote
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: root.firewallAuthNote.length > 0 && root.firewallWritable
                text: root.firewallAuthNote
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: root.firewallStatusKnown && root.firewallRulesPresent
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
                            enabled: root.firewallWritable && root.firewallRulesPresent
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
                        enabled: root.firewallWritable && root.firewallRulesPresent
                    }
                    Select {
                        id: addPortProtocol
                        objectName: "networkFirewallProtocol"
                        model: [ "tcp", "udp" ]
                        enabled: root.firewallWritable && root.firewallRulesPresent
                    }
                    Button {
                        objectName: "networkFirewallOpenPort"
                        text: qsTr("Open Port")
                        enabled: root.firewallWritable && root.firewallRulesPresent
                        onClicked: root.addPort()
                    }
                }
            }

            Text {
                width: parent.width
                visible: root.firewallStatusKnown && root.firewallRulesPresent
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
                            enabled: root.firewallWritable && root.firewallRulesPresent
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
                        enabled: root.firewallWritable && root.firewallRulesPresent
                    }
                    Button {
                        objectName: "networkFirewallOpenService"
                        text: qsTr("Open Service")
                        enabled: root.firewallWritable && root.firewallRulesPresent
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

            Text {
                width: parent.width
                text: qsTr("Name")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: addServiceName
                objectName: "networkAddServiceName"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                width: parent.width
                text: qsTr("Type")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Select {
                id: addServiceKind
                objectName: "networkAddServiceKind"
                model: [
                    { label: qsTr("Ethernet"), value: "ethernet" },
                    { label: qsTr("Wi-Fi"), value: "wifi" },
                    { label: qsTr("Bridge"), value: "bridge" },
                    { label: qsTr("Bond"), value: "bond" },
                    { label: qsTr("VLAN"), value: "vlan" }
                ]
            }

            Text {
                width: parent.width
                visible: addServiceKind.currentValue === "bridge"
                         || addServiceKind.currentValue === "bond"
                         || addServiceKind.currentValue === "vlan"
                text: qsTr("Interface")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            TextInput {
                id: addServiceInterface
                objectName: "networkAddServiceInterface"
                visible: addServiceKind.currentValue === "bridge"
                         || addServiceKind.currentValue === "bond"
                         || addServiceKind.currentValue === "vlan"
                width: parent.width
                height: Theme.controls.button.height
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                width: parent.width
                visible: addServiceKind.currentValue === "vpn"
                text: qsTr("VPN plugin")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: addServiceResult
                objectName: "networkAddServiceResult"
                width: parent.width
                color: root.writeFailed ? Theme.color.danger
                                        : Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
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
    VpnAddDialog {
        id: addVpnDialog
        objectName: "networkAddVpnDialog"
        width: root.Window.window ? root.Window.window.width : root.width
        height: root.Window.window ? root.Window.window.height : root.height
        plugins: root.vpnPlugins
        wireguardNative: root.vpnWireguardNative
        editorAvailable: root.vpnEditorAvailable
        editorName: root.vpnEditorName
        onSubmitted: (kind, plugin, values) =>
            root.submitAddVpn(kind, plugin, values)
    }
}
