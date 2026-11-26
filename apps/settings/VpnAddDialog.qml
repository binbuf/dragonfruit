// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The one Add VPN form shared by the Network and VPN panes (T272a).
//
// The form is capability-driven: a native WireGuard entry appears only when
// NetworkManager supports the `wireguard` connection type, and a plugin entry
// appears only when its plugin is installed *and* the connection editor exists
// to finish its credentials. A plugin profile is created incomplete and stays
// "Needs setup" until the editor completes it; the native WireGuard form
// collects the private key, address and one peer, so the profile it creates is
// connectable immediately.
Dialog {
    id: root

    // The installed plugins (`[{serviceType, label}]`).
    property var plugins: []
    // Whether NetworkManager supports the native `wireguard` type.
    property bool wireguardNative: false
    // Whether `nm-connection-editor` (or the reported editor) is installed.
    property bool editorAvailable: false
    property string editorName: ""
    // The validation / result line; the owning pane may also set it.
    property string resultText: ""

    // `kind` is `"wireguard"` or `"plugin"`; `plugin` is the service type for
    // a plugin profile and empty for native WireGuard; `values` carries the
    // form fields.
    signal submitted(string kind, string plugin, var values)

    title: qsTr("Add VPN Configuration")
    width: parent ? parent.width : 480
    height: parent ? parent.height : 400

    readonly property var pluginOptions: root.plugins.filter(function(plugin) {
        return root.editorAvailable;
    }).map(function(plugin) {
        return { label: String(plugin.label), value: String(plugin.serviceType),
                 kind: "plugin" };
    })
    readonly property var options: {
        var out = [];
        for (var i = 0; i < root.pluginOptions.length; ++i)
            out.push(root.pluginOptions[i]);
        if (root.wireguardNative)
            out.push({ label: qsTr("WireGuard (built in)"), value: "wireguard",
                       kind: "wireguard" });
        return out;
    }
    readonly property string selectedKind: {
        if (addKind.currentValue === "wireguard")
            return "wireguard";
        if (root.options.length === 1 && root.options[0].kind === "wireguard")
            return "wireguard";
        return "plugin";
    }

    function prepare() {
        addName.text = "";
        addGateway.text = "";
        addPrivateKey.text = "";
        addAddress.text = "";
        addPeerKey.text = "";
        addEndpoint.text = "";
        addDns.text = "";
        addMtu.text = "";
        root.resultText = "";
        show();
    }

    function submit() {
        var name = addName.text.trim();
        if (name.length === 0) {
            root.resultText = qsTr("A VPN name is required.");
            return;
        }
        if (root.selectedKind === "wireguard") {
            if (addPrivateKey.text.trim().length === 0
                    || addAddress.text.trim().length === 0
                    || addPeerKey.text.trim().length === 0
                    || addEndpoint.text.trim().length === 0) {
                root.resultText = qsTr("A private key, address, peer public key, "
                                      + "and endpoint are required.");
                return;
            }
            var mtu = parseInt(addMtu.text, 10);
            if (addMtu.text.trim().length > 0 && (!isFinite(mtu) || mtu <= 0)) {
                root.resultText = qsTr("The MTU must be a positive number.");
                return;
            }
            root.submitted("wireguard", "", {
                name: name,
                privateKey: addPrivateKey.text.trim(),
                address: addAddress.text.trim(),
                peerPublicKey: addPeerKey.text.trim(),
                endpoint: addEndpoint.text.trim(),
                dns: addDns.text.trim(),
                mtu: addMtu.text.trim().length > 0 ? mtu : 0
            });
            return;
        }
        if (!root.editorAvailable) {
            root.resultText = qsTr("The connection editor is not installed, so a "
                                  + "plugin profile cannot be finished.");
            return;
        }
        var plugin = addKind.currentValue;
        if (plugin === undefined || plugin === "wireguard") {
            for (var i = 0; i < root.options.length; ++i) {
                if (root.options[i].kind === "plugin") {
                    plugin = root.options[i].value;
                    break;
                }
            }
        }
        if (plugin === undefined || plugin === "wireguard") {
            root.resultText = qsTr("Choose a VPN plugin.");
            return;
        }
        root.submitted("plugin", String(plugin), {
            name: name,
            gateway: addGateway.text.trim()
        });
    }

    contentData: Column {
        width: parent.width
        spacing: Theme.primitive.spacing.sm

        Text {
            width: parent.width
            visible: root.options.length > 1
            text: qsTr("Type")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        Select {
            id: addKind
            objectName: "vpnAddKind"
            width: parent.width
            visible: root.options.length > 1
            model: root.options
        }
        Text {
            width: parent.width
            visible: root.selectedKind === "plugin"
            text: root.editorName.length > 0
                ? qsTr("The profile opens in %1 to finish setup; it stays "
                        + "“Needs setup” until then.").arg(root.editorName)
                : qsTr("The profile opens in the connection editor to finish "
                        + "setup; it stays “Needs setup” until then.")
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
        Text {
            width: parent.width
            visible: root.selectedKind === "wireguard"
            text: qsTr("Creates a native NetworkManager WireGuard profile. "
                       + "No VPN plugin is used.")
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        Text {
            width: parent.width
            text: qsTr("Name")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addName
            objectName: "vpnAddName"
            width: parent.width
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Text {
            width: parent.width
            visible: root.selectedKind === "plugin"
            text: qsTr("Gateway")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addGateway
            objectName: "vpnAddGateway"
            width: parent.width
            visible: root.selectedKind === "plugin"
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Text {
            width: parent.width
            visible: root.selectedKind === "wireguard"
            text: qsTr("Private key")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addPrivateKey
            objectName: "vpnAddPrivateKey"
            width: parent.width
            visible: root.selectedKind === "wireguard"
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            echoMode: TextInput.Password
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Text {
            width: parent.width
            visible: root.selectedKind === "wireguard"
            text: qsTr("Address (CIDR, e.g. 10.7.0.2/32)")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addAddress
            objectName: "vpnAddAddress"
            width: parent.width
            visible: root.selectedKind === "wireguard"
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Text {
            width: parent.width
            visible: root.selectedKind === "wireguard"
            text: qsTr("Peer public key")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addPeerKey
            objectName: "vpnAddPeerKey"
            width: parent.width
            visible: root.selectedKind === "wireguard"
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Text {
            width: parent.width
            visible: root.selectedKind === "wireguard"
            text: qsTr("Endpoint (host:port)")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addEndpoint
            objectName: "vpnAddEndpoint"
            width: parent.width
            visible: root.selectedKind === "wireguard"
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Text {
            width: parent.width
            visible: root.selectedKind === "wireguard"
            text: qsTr("DNS (optional, comma-separated)")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addDns
            objectName: "vpnAddDns"
            width: parent.width
            visible: root.selectedKind === "wireguard"
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Text {
            width: parent.width
            visible: root.selectedKind === "wireguard"
            text: qsTr("MTU (optional)")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        TextInput {
            id: addMtu
            objectName: "vpnAddMtu"
            width: parent.width
            visible: root.selectedKind === "wireguard"
            height: Theme.controls.button.height
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeSm
            inputMethodHints: Qt.ImhDigitsOnly
        }

        Text {
            id: addResult
            objectName: "vpnAddResult"
            width: parent.width
            text: root.resultText
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
    }

    buttonsData: Row {
        spacing: Theme.primitive.spacing.sm
        Button {
            text: qsTr("Cancel")
            onClicked: root.hide()
        }
        Button {
            objectName: "vpnAddConfirm"
            text: qsTr("Add")
            variant: "primary"
            enabled: root.options.length > 0
            onClicked: root.submit()
        }
    }
}
