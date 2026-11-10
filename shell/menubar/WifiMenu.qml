// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
pragma ComponentBehavior: Bound

// The Wi-Fi status menu (T-07.5a, T232): the radio, the current network with
// details, the saved (known) networks, the other visible networks, join, and
// the Settings link, drawn in the design-system popup. The model is the bridge
// host's `wifi` view (services/system-status); the popover never talks to a
// daemon, it raises the explicit actions and the shell forwards them.
//
// `rows` is the normalized action model a test can assert without touching
// the scene: known networks first (connected first), then the remaining
// visible networks. The same list drives pointer and keyboard selection.
Item {
    id: root

    // The host's decoded Wi-Fi view ({state, glyph, label, networks,
    // knownNetworks, activeSsid, activeAddress, readOnly, ...}).
    property var model: ({})
    property Item anchorItem: null
    property alias popup: popup
    property alias open: popup.open
    // T-21.2c: the menu bar pushes the compositor's Popup role state here.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    property string materialPreset: "statusMenu"
    property string materialBackdrop: "trusted"
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    // The selected network awaiting a password (a secured SSID). Empty when
    // no join is in progress.
    property string selectedSsid: ""
    property string secret: ""
    // T232: the network whose inline details are open. Empty means no details.
    property string detailsSsid: ""
    property string headerLabel: model.label !== undefined ? model.label : ""
    readonly property var networks: model.networks !== undefined ? model.networks : []
    readonly property bool readOnly: model.readOnly === true
    readonly property bool available: model.state === "available"
    readonly property bool radioEnabled: model.radioEnabled === undefined
        ? true : model.radioEnabled === true
    readonly property var knownNetworks: model.knownNetworks !== undefined
        ? model.knownNetworks : []
    readonly property string errorMessage: model.error !== undefined
        ? String(model.error) : ""
    readonly property string wifiStateName: model.wifiState !== undefined
        ? String(model.wifiState) : ""
    // The host does not expose an explicit scan flag; a connecting/disconnecting
    // device is the one honest "busy" state NetworkManager reports. A test or a
    // future host may also set `scanning` directly.
    readonly property bool scanning: model.scanning === true
        || wifiStateName === "connecting" || wifiStateName === "disconnecting"
    readonly property bool scanningNoteVisible: root.available
        && root.radioEnabled && root.scanning
    readonly property string activeSsid: {
        if (model.activeSsid !== undefined && String(model.activeSsid).length > 0)
            return String(model.activeSsid);
        for (var i = 0; i < root.networks.length; ++i) {
            if (root.networks[i] && root.networks[i].active === true)
                return String(root.networks[i].ssid);
        }
        return "";
    }

    function networkFor(ssid) {
        for (var i = 0; i < root.networks.length; ++i) {
            if (String(root.networks[i].ssid) === String(ssid))
                return root.networks[i];
        }
        return null;
    }

    function knownFor(ssid) {
        for (var i = 0; i < root.knownNetworks.length; ++i) {
            if (String(root.knownNetworks[i].ssid) === String(ssid))
                return root.knownNetworks[i];
        }
        return null;
    }

    // One unified action list: known networks first (connected first), then
    // the visible networks that are not saved. `section` lets the view draw
    // the two reference groups while keyboard selection walks one list.
    readonly property var rows: {
        var out = [];
        var seen = {};
        for (var k = 0; k < root.knownNetworks.length; ++k) {
            var known = root.knownNetworks[k] || {};
            var knownSsid = known.ssid !== undefined ? String(known.ssid) : "";
            if (knownSsid.length === 0)
                continue;
            seen[knownSsid] = true;
            var knownNetwork = root.networkFor(knownSsid);
            out.push({
                index: out.length,
                ssid: knownSsid,
                strength: knownNetwork !== null && knownNetwork.strength !== undefined
                    ? Number(knownNetwork.strength) : 0,
                security: known.security !== undefined && String(known.security).length > 0
                    ? String(known.security)
                    : (knownNetwork !== null && knownNetwork.security !== undefined
                       ? String(knownNetwork.security) : ""),
                secured: known.secured === true
                    || (knownNetwork !== null && knownNetwork.secured === true),
                active: known.connected === true
                    || (knownNetwork !== null && knownNetwork.active === true),
                band: knownNetwork !== null && knownNetwork.band !== undefined
                    ? String(knownNetwork.band) : "",
                known: true,
                autoconnect: known.autoconnect === true,
                lastUsed: known.lastUsed !== undefined ? Number(known.lastUsed) : 0,
                uuid: known.uuid !== undefined ? String(known.uuid) : "",
                section: "known",
                action: "join",
                enabled: !root.readOnly && root.available && root.radioEnabled
            });
        }
        for (var i = 0; i < root.networks.length; ++i) {
            var network = root.networks[i] || {};
            var ssid = network.ssid !== undefined ? String(network.ssid) : "";
            if (ssid.length === 0 || seen[ssid] === true)
                continue;
            out.push({
                index: out.length,
                ssid: ssid,
                strength: network.strength !== undefined ? Number(network.strength) : 0,
                security: network.security !== undefined ? String(network.security) : "",
                secured: network.secured === true,
                active: network.active === true,
                band: network.band !== undefined ? String(network.band) : "",
                known: false,
                autoconnect: false,
                lastUsed: 0,
                uuid: "",
                section: "other",
                action: "join",
                enabled: !root.readOnly && root.available && root.radioEnabled
            });
        }
        return out;
    }
    readonly property var knownRows: root.rows.filter(function(row) {
        return row.section === "known";
    })
    readonly property var otherRows: root.rows.filter(function(row) {
        return row.section === "other";
    })
    readonly property bool passwordPromptVisible: root.selectedSsid.length > 0
                                                 && !root.readOnly
    // Keyboard selection (T-07.5b): the row Up/Down walks and Return joins.
    property int highlightIndex: -1
    // Test surface (tst_menubar.qml).
    property alias scanningNote: scanningNote
    property alias stateNote: stateNote

    signal joinRequested(string ssid, string secret)
    signal toggleRequested(bool enabled)
    signal refreshRequested()
    signal settingsRequested()
    // Raised when the popup closes for any reason (Escape, click-away), so
    // the bar can clear its open-status state (T-07.5b keyboard a11y).
    signal closed()

    function activateRow(index) {
        if (index < 0 || index >= root.rows.length)
            return;
        var row = root.rows[index];
        if (!row || !row.enabled)
            return;
        if (row.secured && !row.known) {
            // A secured network needs its secret first; the prompt appears and
            // `joinSelected` performs the one join.
            root.selectedSsid = row.ssid;
            root.secret = "";
            return;
        }
        root.selectedSsid = "";
        root.joinRequested(row.ssid, "");
    }

    function joinSelected() {
        if (root.selectedSsid.length === 0 || root.readOnly)
            return;
        root.joinRequested(root.selectedSsid, root.secret);
        root.selectedSsid = "";
        root.secret = "";
    }

    function cancelPassword() {
        root.selectedSsid = "";
        root.secret = "";
    }

    function toggleDetails(ssid) {
        root.detailsSsid = root.detailsSsid === ssid ? "" : ssid;
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
        return network !== null && network.band !== undefined
            && String(network.band).length > 0 ? String(network.band) : qsTr("Unknown");
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
        return new Date(Number(known.lastUsed) * 1000).toLocaleDateString();
    }

    function moveHighlight(delta) {
        var n = root.rows.length;
        if (n <= 0) {
            root.highlightIndex = -1;
            return;
        }
        var i = root.highlightIndex < 0 ? (delta > 0 ? -1 : 0) : root.highlightIndex;
        for (var step = 0; step < n; ++step) {
            i = ((i + delta) % n + n) % n;
            if (root.rows[i].enabled) {
                root.highlightIndex = i;
                return;
            }
        }
        root.highlightIndex = -1;
    }

    function activateHighlight() {
        if (root.highlightIndex >= 0)
            root.activateRow(root.highlightIndex);
    }

    Popup {
        id: popup
        objectName: "wifiPopup"
        anchorItem: root.anchorItem
        preferredWidth: 300
        accessibleRole: Accessible.PopupMenu
        accessibleName: qsTr("Wi-Fi")
        escapeCloses: true
        motionPreset: "menu"
        materialPreset: root.materialPreset
        materialBackdrop: root.materialBackdrop
        materialBlurActive: root.materialBlurActive
        materialTier: root.materialTier
        materialFallback: root.materialFallback

        onOpened: {
            root.selectedSsid = "";
            root.secret = "";
            root.detailsSsid = "";
            root.highlightIndex = -1;
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: {
            root.cancelPassword();
            root.detailsSsid = "";
            root.highlightIndex = -1;
            root.closed();
        }

        Keys.onUpPressed: (event) => {
            root.moveHighlight(-1);
            event.accepted = true;
        }
        Keys.onDownPressed: (event) => {
            root.moveHighlight(1);
            event.accepted = true;
        }
        Keys.onReturnPressed: (event) => {
            root.activateHighlight();
            event.accepted = true;
        }

        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                StatusGlyph {
                    name: root.model.glyph !== undefined ? root.model.glyph : "wifi"
                    size: Theme.controls.menuBar.iconSize + 2
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    text: root.headerLabel
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    font.weight: Theme.primitive.font.weightMedium
                    elide: Text.ElideRight
                    width: parent.width - Theme.controls.menuBar.iconSize - 2
                            - Theme.primitive.spacing.sm
                            - wifiToggle.width - Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                }
                Toggle {
                    id: wifiToggle
                    objectName: "wifiMenuToggle"
                    text: ""
                    accessibleName: qsTr("Wi-Fi")
                    checked: root.radioEnabled
                    enabled: root.available && !root.readOnly
                    anchors.verticalCenter: parent.verticalCenter
                    onToggled: (checked) => root.toggleRequested(checked)
                }
            }

            Text {
                visible: root.readOnly
                width: parent.width
                text: root.model.note !== undefined && root.model.note.length > 0
                      ? root.model.note
                      : qsTr("Read-only: joining needs authorization")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                id: stateNote
                objectName: "wifiStateNote"
                visible: !root.available || root.model.state === "error"
                width: parent.width
                text: root.model.state === "error"
                      ? (root.errorMessage.length > 0 ? root.errorMessage
                                                      : qsTr("Wi-Fi is unavailable"))
                      : qsTr("Wi-Fi unavailable")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                id: scanningNote
                objectName: "wifiScanningNote"
                visible: root.scanningNoteVisible
                width: parent.width
                text: qsTr("Scanning…")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                visible: root.available && !root.radioEnabled
                width: parent.width
                text: qsTr("Wi-Fi is off. Turn it on to see networks.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            // ── Current network ─────────────────────────────────────────
            Item {
                objectName: "wifiCurrentRow"
                visible: root.available && root.radioEnabled
                width: parent.width
                height: Theme.controls.popup.rowHeight

                Icon {
                    visible: root.activeSsid.length > 0
                    name: "check"
                    size: Theme.controls.button.iconSize
                    color: Theme.color.accent
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.primitive.spacing.xs
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    id: currentName
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.controls.button.iconSize
                                     + Theme.primitive.spacing.sm
                    anchors.right: detailsButton.left
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.activeSsid.length > 0 ? root.activeSsid
                                                     : qsTr("Not Connected")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    elide: Text.ElideRight
                }

                Button {
                    id: detailsButton
                    objectName: "wifiDetailsButton"
                    visible: root.activeSsid.length > 0
                    text: root.detailsSsid === root.activeSsid
                          ? qsTr("Hide Details") : qsTr("Details…")
                    variant: "ghost"
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    accessibleName: qsTr("Details for %1").arg(root.activeSsid)
                    onClicked: root.toggleDetails(root.activeSsid)
                }

                TapHandler {
                    onTapped: {
                        if (root.activeSsid.length > 0)
                            root.toggleDetails(root.activeSsid);
                    }
                }
                Accessible.role: Accessible.MenuItem
                Accessible.name: root.activeSsid.length > 0
                    ? qsTr("%1, connected").arg(root.activeSsid)
                    : qsTr("Not connected")
                Accessible.onPressAction: {
                    if (root.activeSsid.length > 0)
                        root.toggleDetails(root.activeSsid);
                }
            }

            // Inline current-network details (the reference's Details… sheet
            // in a compact popover form).
            Column {
                objectName: "wifiDetails"
                visible: root.detailsSsid.length > 0
                width: parent.width
                spacing: 2
                leftPadding: Theme.controls.button.iconSize
                             + Theme.primitive.spacing.sm
                rightPadding: Theme.primitive.spacing.xs

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
                    text: qsTr("Address: %1").arg(
                        root.detailsSsid === root.activeSsid
                        && root.model.activeAddress !== undefined
                        && String(root.model.activeAddress).length > 0
                            ? String(root.model.activeAddress)
                            : qsTr("Not connected"))
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
                Text {
                    width: parent.width
                    text: qsTr("Last used: %1").arg(root.lastUsedLabel(root.detailsSsid))
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            Rectangle {
                visible: root.available && root.rows.length > 0
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Text {
                visible: root.available && root.knownRows.length > 0
                width: parent.width
                text: qsTr("Known Networks")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Repeater {
                objectName: "wifiKnownRows"
                model: root.knownRows

                delegate: wifiRowDelegate
            }

            Text {
                visible: root.available && root.radioEnabled
                         && root.otherRows.length > 0
                width: parent.width
                text: qsTr("Other Networks")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Repeater {
                objectName: "wifiRows"
                model: root.otherRows

                delegate: wifiRowDelegate
            }

            Text {
                objectName: "wifiEmptyNote"
                visible: root.available && root.radioEnabled && !root.scanning
                         && root.rows.length === 0
                width: parent.width
                text: qsTr("No networks found")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Rectangle {
                visible: root.available
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Button {
                objectName: "wifiSettingsButton"
                width: parent.width
                text: qsTr("Wi-Fi Settings…")
                variant: "secondary"
                accessibleName: qsTr("Open Wi-Fi Settings")
                onClicked: root.settingsRequested()
            }
        }
    }

    Component {
        id: wifiRowDelegate

        Item {
            id: rowItem
            required property var modelData
            objectName: "wifiRow-" + rowItem.modelData.ssid
            readonly property int strength: rowItem.modelData.strength !== undefined
                                            ? rowItem.modelData.strength : 0
            readonly property bool secured: rowItem.modelData.secured === true
            readonly property bool active: rowItem.modelData.active === true
            readonly property bool known: rowItem.modelData.known === true
            width: parent.width
            height: secured && root.selectedSsid === rowItem.modelData.ssid
                    ? Theme.controls.popup.rowHeight * 2
                    : Theme.controls.popup.rowHeight
            opacity: rowItem.modelData.enabled || active ? 1.0 : 0.4

            Rectangle {
                anchors.fill: parent
                radius: Theme.controls.focusRing.radius
                color: (rowHover.hovered || root.highlightIndex === rowItem.modelData.index)
                       ? Theme.color.controlHover : "transparent"
                antialiasing: true
            }

            Icon {
                visible: rowItem.active
                name: "check"
                size: Theme.controls.button.iconSize
                color: Theme.color.accent
                anchors.left: parent.left
                anchors.leftMargin: Theme.primitive.spacing.xs
                anchors.verticalCenter: rowContent.verticalCenter
            }

            Row {
                id: rowContent
                anchors.left: parent.left
                anchors.leftMargin: Theme.controls.button.iconSize
                                 + Theme.primitive.spacing.sm
                anchors.right: parent.right
                anchors.rightMargin: Theme.primitive.spacing.xs
                anchors.top: parent.top
                height: Theme.controls.popup.rowHeight
                spacing: Theme.primitive.spacing.sm

                Text {
                    width: parent.width - strengthBars.width
                           - securityText.width - 2 * parent.spacing
                    text: rowItem.modelData.ssid.length > 0 ? rowItem.modelData.ssid
                                                    : qsTr("Hidden network")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    elide: Text.ElideRight
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    id: securityText
                    visible: rowItem.modelData.secured
                    text: rowItem.modelData.security
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    anchors.verticalCenter: parent.verticalCenter
                }

                Row {
                    id: strengthBars
                    spacing: 2
                    anchors.verticalCenter: parent.verticalCenter

                    Repeater {
                        model: 4
                        delegate: Rectangle {
                            required property int index
                            width: 3
                            height: 5 + index * 3
                            radius: 1
                            color: (rowItem.strength * 4 / 100 > index)
                                   ? Theme.color.textPrimary
                                   : Theme.color.separator
                            anchors.bottom: parent.bottom
                        }
                    }
                }
            }

            // Secured-network password row, revealed when the SSID is
            // selected. Return joins; Escape cancels.
            Row {
                visible: rowItem.secured && root.selectedSsid === rowItem.modelData.ssid
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: Theme.controls.button.iconSize
                                   + Theme.primitive.spacing.sm
                anchors.rightMargin: Theme.primitive.spacing.xs
                anchors.top: rowContent.bottom
                height: Theme.controls.popup.rowHeight
                spacing: Theme.primitive.spacing.xs

                Rectangle {
                    width: parent.width - joinButton.width - parent.spacing
                    height: Theme.controls.button.height
                    radius: Theme.controls.button.radius
                    color: Theme.color.controlFill
                    border.width: Theme.controls.window.borderWidth
                    border.color: secretInput.activeFocus ? Theme.color.focusRing
                                                          : Theme.color.border
                    anchors.verticalCenter: parent.verticalCenter

                    TextInput {
                        id: secretInput
                        anchors.fill: parent
                        anchors.leftMargin: Theme.controls.button.paddingH
                        anchors.rightMargin: Theme.controls.button.paddingH
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        echoMode: TextInput.Password
                        selectByMouse: true
                        text: root.secret
                        onTextChanged: root.secret = text
                        onAccepted: root.joinSelected()
                    }
                }

                Button {
                    id: joinButton
                    text: qsTr("Join")
                    variant: "primary"
                    anchors.verticalCenter: parent.verticalCenter
                    onClicked: root.joinSelected()
                }
            }

            HoverHandler { id: rowHover }

            TapHandler {
                onTapped: root.activateRow(rowItem.modelData.index)
            }

            Accessible.role: Accessible.MenuItem
            Accessible.name: (rowItem.modelData.ssid.length > 0 ? rowItem.modelData.ssid
                                                        : qsTr("Hidden network"))
                             + (rowItem.active ? qsTr(", connected")
                                               : (rowItem.known ? qsTr(", saved")
                                                                : ""))
            Accessible.description: rowItem.modelData.enabled ? ""
                : (root.readOnly ? qsTr("Read-only: joining needs authorization")
                                 : (!root.radioEnabled ? qsTr("Wi-Fi is off")
                                                       : qsTr("Already connected")))
            Accessible.onPressAction: root.activateRow(rowItem.modelData.index)
        }
    }
}
