// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import Dragonfruit
import Dragonfruit.Settings

// The Wi-Fi pane (T225; T271 completion).
//
// One NetworkManager model serves this pane and the shell's Wi-Fi quick sheet:
// the `Settings.wifi` view is the same `Wifi` bridge-host projection the menu
// bar reads. The pane never caches a connected state; every write goes to the
// adapter and the host's re-read pushes the new view back. A failed write
// leaves the view untouched, so every control is bound to the view and reverts.
//
// Rows follow the macOS reference (docs/design/24-system-settings.md §4.1):
// power, the current network with Details…, Personal Hotspot, Known Networks,
// Other Networks with a real scan/scanning/empty/error state, Join Other
// Network…, Ask to join networks/hotspots, and Advanced…. The reference's
// `Learn more…` link and `?` help are omitted project-wide.
//
// Absence is structural: the sections, the settingsd rows, and Advanced stay;
// only the NetworkManager controls disable with a reason (ADR 0262).
Item {
    id: root

    readonly property var view: Settings.wifi
    readonly property bool available: Settings.wifiAvailable
    readonly property string viewState: root.view.state !== undefined
        ? String(root.view.state) : ""
    readonly property bool ready: root.available && root.viewState === "available"
    readonly property bool failed: root.available && root.viewState === "error"
    readonly property string errorMessage: root.view.error !== undefined
        ? String(root.view.error) : ""
    readonly property bool radioEnabled: root.view.radioEnabled === true
    readonly property bool readOnly: root.view.readOnly === true
    readonly property string readOnlyNote: root.view.note !== undefined
        ? String(root.view.note) : ""
    readonly property string activeSsid: root.view.activeSsid !== undefined
        ? String(root.view.activeSsid) : ""
    readonly property string activeAddress: root.view.activeAddress !== undefined
        ? String(root.view.activeAddress) : ""
    readonly property bool scanning: root.view.scanning === true
    readonly property bool hotspotSupported: root.view.hotspotSupported === true
    readonly property bool hotspotActive: root.view.hotspotActive === true
    readonly property string hotspotSsid: root.view.hotspotSsid !== undefined
        ? String(root.view.hotspotSsid) : ""
    readonly property string hotspotUuid: root.view.hotspotUuid !== undefined
        ? String(root.view.hotspotUuid) : ""
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
    readonly property bool otherEmpty: root.ready && root.radioEnabled
        && !root.scanning && root.otherNetworks.length === 0
        && !root.failed

    // T271: the last host write's live state. `busy` disables the acting
    // control; denied/failed/absent keeps the reason next to the rows.
    readonly property string writeOperation: Settings.wifiWriteOperation
    readonly property string writeState: Settings.wifiWriteState
    readonly property string writeError: Settings.wifiWriteError
    readonly property string writeMessage: {
        if (root.writeState === "denied")
            return root.writeError.length > 0
                ? qsTr("Authentication was denied: %1").arg(root.writeError)
                : qsTr("Authentication was denied.");
        if (root.writeState === "failed" || root.writeState === "absent")
            return root.writeError.length > 0 ? root.writeError
                : qsTr("The network change could not be applied.");
        return "";
    }

    // Test surface (tst_settings_wifi.qml).
    property alias powerGroup: powerGroup
    property alias powerToggle: powerToggle
    property alias currentGroup: currentGroup
    property alias currentRow: currentRow
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
    property alias advancedRow: advancedRow
    property alias hotspotDialog: hotspotDialog
    property alias joinSsidField: hiddenSsidField
    property alias joinSecretField: hiddenSecretField
    property alias detailsTitle: detailsTitle
    property alias detailsBody: detailsBody
    property alias askToJoinSelect: askToJoinSelect
    property alias askHotspotsSelect: askHotspotsSelect
    property alias hotspotGroup: hotspotGroup
    property alias hotspotRow: hotspotRow
    property alias hotspotSsidField: hotspotSsidField
    property alias hotspotSecretField: hotspotSecretField
    property alias hotspotCreateButton: hotspotCreateButton
    property alias scanRow: scanRow
    property alias scanningRow: scanningRow
    property alias spinner: spinner
    property alias emptyRow: emptyRow
    property alias errorRow: errorRow
    property alias unavailableRow: unavailableRow
    property alias knownEmptyRow: knownEmptyRow
    property alias writeErrorItem: writeErrorItem
    property alias askNote: askNote
    property alias detailsForgetButton: detailsForgetButton
    property alias detailsDisconnectButton: detailsDisconnectButton
    property alias advancedMetered: advancedMetered
    property alias advancedIpv6: advancedIpv6
    property alias advancedDns: advancedDns
    property alias advancedProxy: advancedProxy
    property alias advancedMac: advancedMac
    property alias advancedAuth: advancedAuth

    // The network awaiting a password, and the hidden-network form.
    property string pendingSsid: ""
    property bool pendingSecured: false
    property string detailsSsid: ""
    property string joinError: ""
    property string hotspotError: ""

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    // The Ask to join popup values (settingsd keys; NM autoconnect remains
    // authoritative). The shell has no prompt consumer yet, so the map marks
    // both rows `stored-only` and the pane says so.
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

    // T271: the two popups converge after an external settingsd change. The
    // Select control reports intent; the owner re-reads the key.
    Connections {
        target: Settings
        function onValuesChanged() { root.syncPromptPolicy(); }
    }

    function syncPromptPolicy() {
        var joinValue = String(Settings.values["network.wifi.askToJoin"] || "notify");
        for (var i = 0; i < root.askToJoinOptions.length; ++i) {
            if (root.askToJoinOptions[i].value === joinValue
                    && askToJoinSelect.currentIndex !== i)
                askToJoinSelect.currentIndex = i;
        }
        var hotspotValue = String(Settings.values["network.wifi.askToJoinHotspots"] || "ask");
        for (var j = 0; j < root.askHotspotsOptions.length; ++j) {
            if (root.askHotspotsOptions[j].value === hotspotValue
                    && askHotspotsSelect.currentIndex !== j)
                askHotspotsSelect.currentIndex = j;
        }
    }

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

    function serviceForUuid(uuid) {
        for (var i = 0; i < root.services.length; ++i)
            if (String(root.services[i].uuid) === String(uuid))
                return root.services[i];
        return null;
    }

    function activeService() {
        return root.serviceFor(root.activeSsid);
    }

    function detailsService() {
        if (root.detailsSsid.length === 0)
            return root.activeService();
        if (root.detailsSsid === root.activeSsid && root.activeSsid.length > 0)
            return root.activeService();
        return root.serviceFor(root.detailsSsid);
    }

    // A refused write leaves the view untouched; the toggle assignments from
    // `Toggle.toggle()` overwrote their bindings, so re-read the view here.
    function revertControls() {
        if (root.writeState !== "failed" && root.writeState !== "denied"
                && root.writeState !== "absent")
            return;
        powerToggle.checked = root.radioEnabled;
        for (var i = 0; i < knownRepeater.count; ++i) {
            var row = knownRepeater.itemAt(i);
            var toggle = row && row.controlItem
                ? row.controlItem.children[0] : null;
            if (toggle && row.modelData)
                toggle.checked = row.modelData.autoconnect === true;
        }
    }

    Connections {
        target: Settings
        function onWifiWriteChanged() { root.revertControls(); }
    }

    function togglePower() {
        if (!root.ready || root.readOnly)
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
            joinPasswordField.text = "";
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
        joinPasswordField.text = "";
        root.pendingSsid = "";
        root.joinError = "";
        joinDialog.hide();
    }

    function cancelJoin() {
        root.pendingSsid = "";
        root.pendingSecured = false;
        root.joinError = "";
        joinPasswordField.text = "";
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
        detailsDialog.hide();
    }

    function disconnect(uuid) {
        if (!root.ready || root.readOnly || uuid.length === 0)
            return;
        Settings.disconnectWifi(uuid);
        detailsDialog.hide();
    }

    function scan() {
        if (!root.ready || root.readOnly || !root.radioEnabled)
            return;
        Settings.scanWifi();
    }

    function showDetails(ssid) {
        root.detailsSsid = ssid;
        detailsDialog.show();
    }

    function showHotspotDialog() {
        root.hotspotError = "";
        if (hotspotSsidField.text.length === 0)
            hotspotSsidField.text = qsTr("dragonfruit-hotspot");
        hotspotSecretField.text = "";
        hotspotDialog.show();
    }

    function submitHotspot() {
        var ssid = hotspotSsidField.text.trim();
        if (ssid.length === 0) {
            root.hotspotError = qsTr("Enter a hotspot name.");
            return;
        }
        if (hotspotSecretField.text.length > 0
                && hotspotSecretField.text.length < 8) {
            root.hotspotError = qsTr("The password must be at least 8 characters.");
            return;
        }
        Settings.startWifiHotspot(ssid, hotspotSecretField.text);
        hotspotSecretField.text = "";
        root.hotspotError = "";
        hotspotDialog.hide();
    }

    function toggleHotspot() {
        if (!root.ready || root.readOnly || !root.hotspotSupported)
            return;
        if (root.hotspotActive && root.hotspotUuid.length > 0)
            Settings.stopWifiHotspot(root.hotspotUuid);
        else
            root.showHotspotDialog();
    }

    function securityLabel(ssid) {
        var network = root.networkFor(ssid);
        var known = root.knownFor(ssid);
        if (network !== null && network.security !== undefined
                && String(network.security).length > 0)
            return String(network.security);
        if (known !== null && known.security !== undefined
                && String(known.security).length > 0)
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

    function meteredLabel() {
        var service = root.detailsService();
        var value = service !== null && service.metered !== undefined
            ? Number(service.metered) : -1;
        if (value === 0)
            return qsTr("No");
        if (value === 1 || value === 2)
            return qsTr("Yes");
        if (value === 3)
            return qsTr("Yes (guessed)");
        return qsTr("Automatic");
    }

    function ipv6Label() {
        var service = root.detailsService();
        if (service !== null && service.ipv6 !== undefined
                && String(service.ipv6).length > 0)
            return String(service.ipv6);
        return qsTr("Automatic");
    }

    function dnsLabel() {
        var service = root.detailsService();
        if (service !== null && service.dns !== undefined
                && service.dns.length > 0)
            return service.dns.join(", ");
        return qsTr("Automatic");
    }

    function proxyLabel() {
        var service = root.detailsService();
        if (service !== null && service.proxy !== undefined
                && String(service.proxy).length > 0)
            return String(service.proxy);
        return qsTr("None");
    }

    function macLabel() {
        var service = root.detailsService();
        if (service !== null && service.clonedMac !== undefined
                && String(service.clonedMac).length > 0)
            return String(service.clonedMac);
        return qsTr("Preserve");
    }

    function authLabel() {
        var service = root.detailsService();
        if (service !== null && service.keyMgmt !== undefined
                && String(service.keyMgmt).length > 0) {
            var value = String(service.keyMgmt);
            if (value === "wpa-eap" || value === "802-1x")
                return qsTr("802.1X");
            return value.toUpperCase();
        }
        return qsTr("None");
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
                description: !root.ready ? qsTr("Unavailable")
                           : (root.radioEnabled ? qsTr("On") : qsTr("Off"))
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

        // T271: one inline report for the last write; the control itself is
        // bound to the view, so it reverts when the host refuses.
        Text {
            id: writeErrorItem
            objectName: "wifiWriteError"
            width: parent.width
            visible: root.writeMessage.length > 0
            text: root.writeMessage
            color: Theme.color.danger
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        // ── Current network ───────────────────────────────────────────────
        SettingsGroup {
            id: currentGroup
            width: parent.width
            title: qsTr("Current Network")

            SettingsRow {
                id: currentRow
                objectName: "wifiCurrentRow"
                width: parent.width
                label: root.activeSsid.length > 0 ? root.activeSsid
                                                  : qsTr("Not Connected")
                description: root.activeSsid.length > 0
                    ? qsTr("%1 · %2").arg(root.securityLabel(root.activeSsid))
                          .arg(root.activeAddress.length > 0
                               ? root.activeAddress : qsTr("No IP address"))
                    : (root.ready ? qsTr("Choose a network below to connect.")
                                  : qsTr("Unavailable"))
                settingsId: "current"
                owner: "NetworkManager"
                settingsKey: "ActiveConnection"
                trailingAction: root.activeSsid.length > 0 && root.ready
                    ? qsTr("Details…") : ""
                showSeparator: false
                onTrailingActionClicked: root.showDetails(root.activeSsid)
            }
        }

        // ── Known networks ────────────────────────────────────────────────
        SettingsGroup {
            id: knownGroup
            width: parent.width
            title: qsTr("Known Networks")

            Repeater {
                id: knownRepeater
                model: root.ready ? root.knownNetworks : []
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
                            enabled: !root.readOnly && root.ready
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
                            objectName: "wifiKnownDetailsButton"
                            text: qsTr("Details…")
                            variant: "ghost"
                            enabled: root.ready
                            onClicked: root.showDetails(String(knownRow.modelData.ssid))
                        }
                        Button {
                            objectName: "wifiKnownConnect"
                            visible: knownRow.modelData.connected !== true
                            text: qsTr("Connect")
                            enabled: root.radioEnabled && !root.readOnly && root.ready
                            onClicked: Settings.joinWifi(
                                String(knownRow.modelData.ssid), "")
                        }
                        Button {
                            objectName: "wifiKnownDisconnect"
                            visible: knownRow.modelData.connected === true
                            text: qsTr("Disconnect")
                            enabled: !root.readOnly && root.ready
                            onClicked: root.disconnect(
                                String(knownRow.modelData.uuid))
                        }
                        Button {
                            objectName: "wifiForgetButton"
                            text: qsTr("Forget")
                            variant: "ghost"
                            enabled: !root.readOnly && root.ready
                            onClicked: root.forget(String(knownRow.modelData.uuid))
                        }
                    }
                }
            }

            SettingsRow {
                id: knownEmptyRow
                objectName: "wifiKnownEmptyRow"
                width: parent.width
                visible: !root.ready || root.knownNetworks.length === 0
                label: root.ready ? qsTr("No saved networks")
                                  : qsTr("Saved networks unavailable")
                description: root.ready ? qsTr("Networks you join are saved here.")
                    : (root.failed ? qsTr("The network list could not be read.")
                                   : qsTr("NetworkManager is not running."))
                showSeparator: false
            }
        }

        // ── Available networks ────────────────────────────────────────────
        SettingsGroup {
            id: availableGroup
            width: parent.width
            title: qsTr("Other Networks")

            Row {
                id: scanningRow
                objectName: "wifiScanningRow"
                width: parent.width
                spacing: Theme.primitive.spacing.sm
                visible: root.ready && root.radioEnabled && root.scanning
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs

                Shape {
                    id: spinner
                    objectName: "wifiSpinner"
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
                    objectName: "wifiScanningLabel"
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Scanning…")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            Text {
                id: errorRow
                objectName: "wifiErrorRow"
                width: parent.width
                visible: root.failed
                text: root.errorMessage.length > 0 ? root.errorMessage
                    : qsTr("NetworkManager could not be read.")
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                id: unavailableRow
                objectName: "wifiUnavailableRow"
                width: parent.width
                visible: !root.ready && !root.failed
                text: qsTr("NetworkManager is not running.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                objectName: "wifiRadioOffRow"
                width: parent.width
                visible: root.ready && !root.radioEnabled
                text: qsTr("Turn on Wi-Fi to see nearby networks.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                id: emptyRow
                objectName: "wifiEmptyRow"
                width: parent.width
                visible: root.otherEmpty
                text: qsTr("No networks found.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: availableRepeater
                model: root.ready && root.radioEnabled && !root.scanning
                    ? root.otherNetworks : []
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
                            enabled: root.radioEnabled && !root.readOnly && root.ready
                            onClicked: root.joinNetwork(String(networkRow.modelData.ssid))
                        }
                    }
                }
            }

            SettingsRow {
                id: scanRow
                objectName: "wifiScanRow"
                width: parent.width
                visible: root.ready && root.radioEnabled
                label: qsTr("Scan for networks")
                description: root.scanning
                    ? qsTr("A scan is in progress.") : qsTr("Look for nearby networks now.")
                settingsId: "scan"
                owner: "NetworkManager"
                settingsKey: "AccessPoint.RequestScan"
                trailingAction: qsTr("Scan")
                showSeparator: true
                onTrailingActionClicked: root.scan()
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
                // The join is a host write: it disables with the other
                // NetworkManager controls when the daemon is absent/read-only.
                enabled: root.ready && !root.readOnly
                trailingAction: qsTr("Join…")
                onTrailingActionClicked: {
                    root.joinError = "";
                    hiddenSsidField.text = "";
                    hiddenSecretField.text = "";
                    hiddenDialog.show();
                }
            }
        }

        // ── Personal hotspot ──────────────────────────────────────────────
        SettingsGroup {
            id: hotspotGroup
            width: parent.width
            title: qsTr("Personal Hotspot")

            SettingsRow {
                id: hotspotRow
                objectName: "wifiHotspotRow"
                width: parent.width
                label: root.hotspotActive && root.hotspotSsid.length > 0
                    ? root.hotspotSsid : qsTr("Personal Hotspot")
                description: !root.hotspotSupported
                    ? qsTr("No Wi-Fi device supports access-point mode.")
                    : (root.hotspotActive ? qsTr("On")
                                          : (root.ready ? qsTr("Off")
                                                        : qsTr("Unavailable")))
                settingsId: "hotspot"
                owner: "NetworkManager"
                settingsKey: "802-11-wireless.mode=ap"
                trailingAction: root.hotspotActive ? qsTr("Turn Off") : qsTr("Create…")
                showSeparator: false
                enabled: root.ready && !root.readOnly && root.hotspotSupported
                onTrailingActionClicked: root.toggleHotspot()
            }
        }

        // ── Prompt policy (settingsd; stored-only) ────────────────────────
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

            Text {
                id: askNote
                objectName: "wifiAskNote"
                width: parent.width
                text: qsTr("Stored for the shell's prompt policy; the shell has "
                           + "no Wi-Fi prompt consumer yet, so these rows do not "
                           + "change NetworkManager's auto-join.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
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
                reason: root.failed
                    ? qsTr("NetworkManager could not be read, so Wi-Fi settings "
                           + "are temporarily unavailable.")
                    : qsTr("NetworkManager is not running, so Wi-Fi settings are unavailable.")
                width: parent.width
            }
        }
    }

    // ── Join dialog (visible secured network) ─────────────────────────────
    Dialog {
        id: joinDialog
        objectName: "wifiJoinDialog"
        showCloseAction: false
        title: qsTr("Join “%1”").arg(root.pendingSsid)
        width: root.width
        height: root.height
        onClosed: root.cancelJoin()
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
                objectName: "wifiJoinCancel"
                text: qsTr("Cancel")
                onClicked: joinDialog.hide()
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
        showCloseAction: false
        title: qsTr("Join Other Network")
        width: root.width
        height: root.height
        onClosed: {
            hiddenSsidField.text = "";
            hiddenSecretField.text = "";
        }

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
                objectName: "wifiHiddenCancel"
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

    // ── Hotspot dialog ────────────────────────────────────────────────────
    Dialog {
        id: hotspotDialog
        objectName: "wifiHotspotDialog"
        title: qsTr("Personal Hotspot")
        width: root.width
        height: root.height
        onClosed: hotspotSecretField.text = ""

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("Share this computer's connection as a Wi-Fi network. "
                           + "Leave the password empty for an open hotspot.")
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
                    id: hotspotSsidField
                    objectName: "wifiHotspotSsid"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: hotspotSecretField.forceActiveFocus()
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
                    id: hotspotSecretField
                    objectName: "wifiHotspotSecret"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    echoMode: TextInput.Password
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: root.submitHotspot()
                }
            }
            Text {
                visible: root.hotspotError.length > 0
                text: root.hotspotError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                text: qsTr("Cancel")
                onClicked: hotspotDialog.hide()
            }
            Button {
                id: hotspotCreateButton
                objectName: "wifiHotspotCreate"
                text: qsTr("Create")
                variant: "primary"
                onClicked: root.submitHotspot()
            }
        }
    }

    // ── Details dialog ────────────────────────────────────────────────────
    Dialog {
        id: detailsDialog
        objectName: "wifiDetailsDialog"
        showCloseAction: false
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
            Text {
                objectName: "wifiDetailsMetered"
                width: parent.width
                text: qsTr("Metered: %1").arg(root.meteredLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "wifiDetailsIpv6"
                width: parent.width
                text: qsTr("IPv6: %1").arg(root.ipv6Label())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "wifiDetailsDns"
                width: parent.width
                text: qsTr("DNS: %1").arg(root.dnsLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "wifiDetailsProxy"
                width: parent.width
                text: qsTr("Proxy: %1").arg(root.proxyLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "wifiDetailsMac"
                width: parent.width
                text: qsTr("MAC randomization: %1").arg(root.macLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                objectName: "wifiDetailsAuth"
                width: parent.width
                text: qsTr("Authentication: %1").arg(root.authLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }

        buttonsData: Row {
            spacing: Theme.primitive.spacing.sm
            Button {
                id: detailsForgetButton
                objectName: "wifiDetailsForget"
                visible: root.knownFor(root.detailsSsid) !== null
                text: qsTr("Forget")
                variant: "ghost"
                enabled: root.ready && !root.readOnly
                onClicked: root.forget(String(root.knownFor(root.detailsSsid).uuid))
            }
            Button {
                id: detailsDisconnectButton
                objectName: "wifiDetailsDisconnect"
                visible: root.detailsSsid === root.activeSsid
                    && root.activeSsid.length > 0
                text: qsTr("Disconnect")
                enabled: root.ready && !root.readOnly
                onClicked: {
                    var known = root.knownFor(root.detailsSsid);
                    if (known !== null)
                        root.disconnect(String(known.uuid));
                }
            }
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
        showCloseAction: false
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
                id: advancedMetered
                objectName: "wifiAdvancedMetered"
                width: parent.width
                text: qsTr("Metered: %1").arg(root.meteredLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: advancedIpv6
                objectName: "wifiAdvancedIpv6"
                width: parent.width
                text: qsTr("IPv6: %1").arg(root.ipv6Label())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: advancedDns
                objectName: "wifiAdvancedDns"
                width: parent.width
                text: qsTr("DNS: %1").arg(root.dnsLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
            Text {
                id: advancedProxy
                objectName: "wifiAdvancedProxy"
                width: parent.width
                text: qsTr("Proxy: %1").arg(root.proxyLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: advancedMac
                objectName: "wifiAdvancedMac"
                width: parent.width
                text: qsTr("MAC randomization: %1").arg(root.macLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                id: advancedAuth
                objectName: "wifiAdvancedAuth"
                width: parent.width
                text: qsTr("Authentication: %1").arg(root.authLabel())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: parent.width
                text: qsTr("These Linux-only values come from the saved "
                           + "NetworkManager connection. The full editor owns "
                           + "changing DNS, proxies, MAC randomization, and 802.1X.")
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
