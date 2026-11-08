// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Wi-Fi pane (T225).
//
// One NetworkManager model serves this pane and the shell's Wi-Fi quick sheet:
// the `Settings.wifi` view is the same `Wifi` bridge-host projection the menu
// bar reads. The pane never caches a connected state; every write goes to the
// adapter and the host's re-read pushes the new view back.
//
// Rows follow the macOS reference (docs/reference/System_Preferences.md):
// power, the current network with Details…, Known Network, Other Networks,
// Join Other Network…, Ask to join networks/hotspots, and Advanced…. The
// reference's `Learn more…` link and `?` help are omitted project-wide.
Item {
    id: root

    readonly property var view: Settings.wifi
    readonly property bool available: Settings.wifiAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property bool radioEnabled: root.view.radioEnabled === true
    readonly property bool readOnly: root.view.readOnly === true
    readonly property string readOnlyNote: root.view.note !== undefined
        ? String(root.view.note) : ""
    readonly property string activeSsid: root.view.activeSsid !== undefined
        ? String(root.view.activeSsid) : ""
    readonly property string activeAddress: root.view.activeAddress !== undefined
        ? String(root.view.activeAddress) : ""
    readonly property var networks: root.view.networks !== undefined
        ? root.view.networks : []
    readonly property var knownNetworks: root.view.knownNetworks !== undefined
        ? root.view.knownNetworks : []
    readonly property var services: root.view.services !== undefined
        ? root.view.services : []
    // The reference lists saved networks under Known Networks and only the
    // remaining visible networks under Other Networks.
    readonly property var otherNetworks: {
        var out = [];
        for (var i = 0; i < root.networks.length; ++i) {
            var network = root.networks[i];
            if (root.knownFor(String(network.ssid)) === null)
                out.push(network);
        }
        return out;
    }

    // Test surface (tst_settings_wifi.qml).
    property alias powerGroup: powerGroup
    property alias powerToggle: powerToggle
    property alias currentGroup: currentGroup
    property alias knownGroup: knownGroup
    property alias availableGroup: availableGroup
    property alias absenceNote: absenceNote
    property alias readOnlyNoteItem: readOnlyNoteItem
    property alias knownRepeater: knownRepeater
    property alias availableRepeater: availableRepeater
    property alias joinDialog: joinDialog
    property alias hiddenDialog: hiddenDialog
    property alias detailsDialog: detailsDialog
    property alias advancedDialog: advancedDialog
    property alias joinSsidField: hiddenSsidField
    property alias joinSecretField: hiddenSecretField
    property alias detailsTitle: detailsTitle
    property alias detailsBody: detailsBody
    property alias askToJoinSelect: askToJoinSelect
    property alias askHotspotsSelect: askHotspotsSelect

    // The network awaiting a password, and the hidden-network form.
    property string pendingSsid: ""
    property bool pendingSecured: false
    property string detailsSsid: ""
    property string joinError: ""

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    // The Ask to join popup values (settingsd keys; NM autoconnect remains
    // authoritative).
    readonly property var askToJoinOptions: [
        { label: qsTr("Off"), value: "off" },
        { label: qsTr("Notify"), value: "notify" },
        { label: qsTr("Ask"), value: "ask" },
        { label: qsTr("New Networks"), value: "new" }
    ]
    readonly property var askHotspotsOptions: [
        { label: qsTr("Off"), value: "off" },
        { label: qsTr("Ask to Join"), value: "ask" },
        { label: qsTr("Automatic"), value: "automatic" }
    ]

    Component.onCompleted: Settings.refreshWifi()

    function networkFor(ssid) {
        for (var i = 0; i < root.networks.length; ++i)
            if (String(root.networks[i].ssid) === String(ssid))
                return root.networks[i];
        return null;
    }

    function knownFor(ssid) {
        for (var i = 0; i < root.knownNetworks.length; ++i)
            if (String(root.knownNetworks[i].ssid) === String(ssid))
                return root.knownNetworks[i];
        return null;
    }

    function serviceFor(ssid) {
        // The saved connection is keyed by UUID, not by the visible SSID: the
        // connection id can differ (for example "Home Wi-Fi" vs the SSID
        // "dragonfruit"). Prefer the known-network UUID join, then fall back
        // to the id for fixtures that use the SSID as the id.
        var known = root.knownFor(ssid);
        var uuid = known !== null && known.uuid !== undefined
            ? String(known.uuid) : "";
        for (var i = 0; i < root.services.length; ++i) {
            var service = root.services[i];
            if (uuid.length > 0 && String(service.uuid) === uuid)
                return service;
            if (String(service.id) === String(ssid)
                    || String(service.id) === String(ssid) + " Wi-Fi")
                return service;
        }
        return null;
    }

    function togglePower() {
        if (!root.ready)
            return;
        Settings.setWifiEnabled(!root.radioEnabled);
    }

    function joinNetwork(ssid) {
        if (!root.ready || root.readOnly || !ssid)
            return;
        var network = root.networkFor(ssid);
        if (network !== null && network.secured === true
                && root.knownFor(ssid) === null) {
            root.pendingSsid = ssid;
            root.pendingSecured = true;
            root.joinError = "";
            joinDialog.show();
            return;
        }
        Settings.joinWifi(ssid, "");
    }

    function submitJoin() {
        if (root.pendingSsid.length === 0)
            return;
        if (joinPasswordField.text.length === 0) {
            root.joinError = qsTr("Enter the network password.");
            return;
        }
        Settings.joinWifi(root.pendingSsid, joinPasswordField.text);
        root.pendingSsid = "";
        root.joinError = "";
        joinDialog.hide();
    }

    function submitHidden() {
        var ssid = hiddenSsidField.text.trim();
        if (ssid.length === 0)
            return;
        Settings.joinHiddenWifi(ssid, hiddenSecretField.text);
        hiddenSsidField.text = "";
        hiddenSecretField.text = "";
        hiddenDialog.hide();
    }

    function forget(uuid) {
        if (!root.ready || root.readOnly || uuid.length === 0)
            return;
        Settings.forgetWifi(uuid);
    }

    function showDetails(ssid) {
        root.detailsSsid = ssid;
        detailsDialog.show();
    }

    function securityLabel(ssid) {
        var network = root.networkFor(ssid);
        var known = root.knownFor(ssid);
        if (network !== null && network.security !== undefined
                && String(network.security).length > 0)
            return String(network.security);
        if (known !== null && known.security !== undefined)
            return String(known.security);
        return qsTr("Unknown");
    }

    function bandLabel(ssid) {
        var network = root.networkFor(ssid);
        if (network !== null && network.band !== undefined)
            return String(network.band);
        return qsTr("Unknown");
    }

    function signalLabel(ssid) {
        var network = root.networkFor(ssid);
        return network !== null && network.strength !== undefined
            ? qsTr("%1% signal").arg(Number(network.strength)) : qsTr("Unknown");
    }

    function lastUsedLabel(ssid) {
        var known = root.knownFor(ssid);
        if (known === null || !known.lastUsed)
            return qsTr("Never");
        var date = new Date(Number(known.lastUsed) * 1000);
        return date.toLocaleDateString();
    }

    function activeService() {
        return root.serviceFor(root.activeSsid);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Power ─────────────────────────────────────────────────────────
        SettingsGroup {
            id: powerGroup
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Wi-Fi")
                description: root.ready
                    ? (root.radioEnabled ? qsTr("On") : qsTr("Off"))
                    : qsTr("NetworkManager is not running.")
                settingsId: "power"
                owner: "NetworkManager"
                settingsKey: "WirelessEnabled"
                showSeparator: false
                controlData: Toggle {
                    id: powerToggle
                    objectName: "wifiPowerToggle"
                    text: ""
                    accessibleName: qsTr("Wi-Fi")
                    enabled: root.ready && !root.readOnly
                    onToggled: (checked) => Settings.setWifiEnabled(checked)

                    Binding {
                        target: powerToggle
                        property: "checked"
                        value: root.radioEnabled
                    }
                }
            }
        }

        // ── Current network ───────────────────────────────────────────────
        SettingsGroup {
            id: currentGroup
            width: parent.width
            title: qsTr("Current Network")
            visible: root.ready

            SettingsRow {
                id: currentRow
                objectName: "wifiCurrentRow"
                width: parent.width
                label: root.activeSsid.length > 0 ? root.activeSsid : qsTr("Not Connected")
                description: root.activeSsid.length > 0
                    ? qsTr("%1 · %2").arg(root.securityLabel(root.activeSsid))
                          .arg(root.activeAddress.length > 0
                               ? root.activeAddress : qsTr("No IP address"))
                    : qsTr("Choose a network below to connect.")
                settingsId: "current"
                owner: "NetworkManager"
                settingsKey: "ActiveConnection"
                trailingAction: root.activeSsid.length > 0 ? qsTr("Details…") : ""
                showSeparator: false
                onTrailingActionClicked: root.showDetails(root.activeSsid)
            }
        }

        // ── Known networks ────────────────────────────────────────────────
        SettingsGroup {
            id: knownGroup
            width: parent.width
            title: qsTr("Known Networks")
            visible: root.ready

            Repeater {
                id: knownRepeater
                model: root.knownNetworks
                delegate: SettingsRow {
                    id: knownRow
                    required property var modelData
                    required property int index
                    objectName: "wifiKnownRow"
                    width: parent.width
                    label: String(knownRow.modelData.ssid)
                    description: knownRow.modelData.connected === true
                        ? qsTr("Connected")
                        : (knownRow.modelData.autoconnect === true
                           ? qsTr("Auto-join on") : qsTr("Auto-join off"))
                    settingsId: "saved"
                    owner: "NetworkManager"
                    settingsKey: "Settings.Connection"
                    showSeparator: knownRow.index < root.knownNetworks.length - 1
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Toggle {
                            id: knownAutoToggle
                            objectName: "wifiKnownAutoconnect"
                            text: ""
                            accessibleName: qsTr("Auto-join %1").arg(knownRow.modelData.ssid)
                            enabled: !root.readOnly
                            onToggled: (checked) =>
                                Settings.setWifiAutoconnect(
                                    String(knownRow.modelData.uuid), checked)

                            Binding {
                                target: knownAutoToggle
                                property: "checked"
                                value: knownRow.modelData.autoconnect === true
                            }
                        }
                        Button {
                            objectName: "wifiKnownConnect"
                            visible: knownRow.modelData.connected !== true
                            text: qsTr("Connect")
                            enabled: root.radioEnabled && !root.readOnly
                            onClicked: Settings.joinWifi(
                                String(knownRow.modelData.ssid), "")
                        }
                        Button {
                            objectName: "wifiForgetButton"
                            text: qsTr("Forget")
                            variant: "ghost"
                            enabled: !root.readOnly
                            onClicked: root.forget(String(knownRow.modelData.uuid))
                        }
                    }
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.knownNetworks.length === 0
                label: qsTr("No saved networks")
                description: qsTr("Networks you join are saved here.")
                showSeparator: false
            }
        }

        // ── Available networks ────────────────────────────────────────────
        SettingsGroup {
            id: availableGroup
            width: parent.width
            title: qsTr("Other Networks")
            visible: root.ready

            Repeater {
                id: availableRepeater
                model: root.radioEnabled ? root.otherNetworks : []
                delegate: SettingsRow {
                    id: networkRow
                    required property var modelData
                    required property int index
                    objectName: "wifiNetworkRow"
                    width: parent.width
                    label: String(networkRow.modelData.ssid)
                    description: qsTr("%1 · %2 · %3")
                        .arg(String(networkRow.modelData.security))
                        .arg(String(networkRow.modelData.band))
                        .arg(qsTr("%1% signal").arg(Number(networkRow.modelData.strength)))
                    settingsId: "available"
                    owner: "NetworkManager"
                    settingsKey: "AccessPoint"
                    showSeparator: networkRow.index < root.otherNetworks.length - 1
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Text {
                            visible: networkRow.modelData.active === true
                            text: qsTr("Connected")
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Button {
                            objectName: "wifiJoinButton"
                            visible: networkRow.modelData.active !== true
                            text: qsTr("Join")
                            enabled: root.radioEnabled && !root.readOnly
                            onClicked: root.joinNetwork(String(networkRow.modelData.ssid))
                        }
                    }
                }
            }

            SettingsRow {
                id: hiddenRow
                objectName: "wifiHiddenRow"
                width: parent.width
                label: qsTr("Join Other Network…")
                description: qsTr("Connect to a hidden network by name.")
                settingsId: "hidden"
                owner: "NetworkManager"
                settingsKey: "AddAndActivateConnection"
                showSeparator: false
                trailingAction: qsTr("Join…")
                onTrailingActionClicked: {
                    root.joinError = "";
                    hiddenSsidField.text = "";
                    hiddenSecretField.text = "";
                    hiddenDialog.show();
                }
            }
        }

        // ── Prompt policy (settingsd) ─────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Ask to Join")

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Ask to join networks")
                description: qsTr("Prompt before joining a nearby network. "
                                   + "NetworkManager's own auto-join still applies.")
                settingsId: "ask"
                owner: "settingsd"
                settingsKey: "network.wifi.askToJoin"
                showSeparator: true
                controlData: Select {
                    id: askToJoinSelect
                    objectName: "wifiAskToJoin"
                    accessibleName: qsTr("Ask to join networks")
                    model: root.askToJoinOptions
                    currentIndex: {
                        var value = String(Settings.values["network.wifi.askToJoin"] || "notify");
                        for (var i = 0; i < root.askToJoinOptions.length; ++i)
                            if (root.askToJoinOptions[i].value === value)
                                return i;
                        return 1;
                    }
                    onSelected: (value) => Settings.set("network.wifi.askToJoin", value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Ask to join hotspots")
                description: qsTr("Prompt before joining a captive-portal hotspot.")
                settingsId: "askHotspots"
                owner: "settingsd"
                settingsKey: "network.wifi.askToJoinHotspots"
                showSeparator: false
                controlData: Select {
                    id: askHotspotsSelect
                    objectName: "wifiAskHotspots"
                    accessibleName: qsTr("Ask to join hotspots")
                    model: root.askHotspotsOptions
                    currentIndex: {
                        var value = String(Settings.values["network.wifi.askToJoinHotspots"] || "ask");
                        for (var i = 0; i < root.askHotspotsOptions.length; ++i)
                            if (root.askHotspotsOptions[i].value === value)
                                return i;
                        return 1;
                    }
                    onSelected: (value) => Settings.set("network.wifi.askToJoinHotspots", value)
                }
            }
        }

        // ── Advanced ──────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width

            SettingsRow {
                id: advancedRow
                objectName: "wifiAdvancedRow"
                width: parent.width
                label: qsTr("Advanced…")
                description: qsTr("TCP/IP, hardware, and the saved connection.")
                settingsId: "advanced"
                owner: "NetworkManager + settingsd"
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
                objectName: "wifiReadOnlyNote"
                width: parent.width
                text: root.readOnlyNote.length > 0
                    ? qsTr("Networks can be viewed but not changed: %1").arg(root.readOnlyNote)
                    : qsTr("Networks can be viewed but not changed.")
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
                objectName: "wifiAbsenceNote"
                kind: "disabled-with-reason"
                reason: qsTr("NetworkManager is not running, so Wi-Fi settings are unavailable.")
                width: parent.width
            }
        }
    }

    // ── Join dialog (visible secured network) ─────────────────────────────
    Dialog {
        id: joinDialog
        objectName: "wifiJoinDialog"
        title: qsTr("Join “%1”").arg(root.pendingSsid)
        width: root.width
        height: root.height
        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("Enter the network password.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
            Rectangle {
                width: parent.width
                height: Theme.controls.button.height
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: joinPasswordField
                    objectName: "wifiJoinSecret"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    echoMode: TextInput.Password
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: root.submitJoin()
                }
            }
            Text {
                visible: root.joinError.length > 0
                text: root.joinError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Cancel")
                onClicked: {
                    root.pendingSsid = "";
                    joinDialog.hide();
                }
            }
            Button {
                objectName: "wifiJoinConfirm"
                text: qsTr("Join")
                variant: "primary"
                onClicked: root.submitJoin()
            }
        }
    }

    // ── Hidden-network dialog ─────────────────────────────────────────────
    Dialog {
        id: hiddenDialog
        objectName: "wifiHiddenDialog"
        title: qsTr("Join Other Network")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("Enter the network name. Leave the password empty for an open network.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
            Rectangle {
                width: parent.width
                height: Theme.controls.button.height
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: hiddenSsidField
                    objectName: "wifiHiddenSsid"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: hiddenSecretField.forceActiveFocus()
                }
            }
            Rectangle {
                width: parent.width
                height: Theme.controls.button.height
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: hiddenSecretField
                    objectName: "wifiHiddenSecret"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    echoMode: TextInput.Password
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: root.submitHidden()
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Cancel")
                onClicked: hiddenDialog.hide()
            }
            Button {
                objectName: "wifiHiddenConfirm"
                text: qsTr("Join")
                variant: "primary"
                onClicked: root.submitHidden()
            }
        }
    }

    // ── Details dialog ────────────────────────────────────────────────────
    Dialog {
        id: detailsDialog
        objectName: "wifiDetailsDialog"
        title: root.detailsSsid
        width: root.width
        height: root.height

        contentData: Column {
            id: detailsBody
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Text {
                id: detailsTitle
                width: parent.width
                text: root.detailsSsid
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                font.weight: Theme.primitive.font.weightMedium
            }
            Text {
                width: parent.width
                text: qsTr("Security: %1").arg(root.securityLabel(root.detailsSsid))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("Band: %1").arg(root.bandLabel(root.detailsSsid))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("Signal: %1").arg(root.signalLabel(root.detailsSsid))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("Address: %1")
                    .arg(root.activeSsid === root.detailsSsid
                         && root.activeAddress.length > 0
                         ? root.activeAddress : qsTr("Not connected"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("Last used: %1").arg(root.lastUsedLabel(root.detailsSsid))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("Auto-join: %1")
                    .arg(root.knownFor(root.detailsSsid) !== null
                         && root.knownFor(root.detailsSsid).autoconnect === true
                         ? qsTr("On") : qsTr("Off"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
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

    // ── Advanced dialog ───────────────────────────────────────────────────
    Dialog {
        id: advancedDialog
        objectName: "wifiAdvancedDialog"
        title: qsTr("Wi-Fi Advanced")
        width: root.width
        height: root.height

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Text {
                objectName: "wifiAdvancedInterface"
                width: parent.width
                text: qsTr("Interface: %1")
                    .arg(root.activeService() !== null
                         && root.activeService().interface !== undefined
                         ? String(root.activeService().interface) : qsTr("None"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "wifiAdvancedHardware"
                width: parent.width
                text: qsTr("Hardware address: %1")
                    .arg(root.activeService() !== null
                         && root.activeService().hardware !== undefined
                         && String(root.activeService().hardware).length > 0
                         ? String(root.activeService().hardware) : qsTr("Unavailable"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "wifiAdvancedAddress"
                width: parent.width
                text: qsTr("TCP/IP: %1")
                    .arg(root.activeAddress.length > 0 ? root.activeAddress
                                                       : qsTr("No address"))
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("DNS, proxies, and MAC randomization are edited in the "
                           + "full connection editor; this first pass shows the live "
                           + "TCP/IP and hardware state.")
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
