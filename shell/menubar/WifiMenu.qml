// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Wi-Fi status menu (T-07.5a): the network list and join, drawn in the
// design-system popup. The model is the bridge host's `wifi` view
// (services/system-status): the popover never talks to a daemon, it raises
// `joinRequested` and the shell forwards that to the host.
//
// `rows` is the normalized action model a test can assert without touching
// the scene: one `join` row per visible network.
Item {
    id: root

    // The host's decoded Wi-Fi view ({state, glyph, label, networks, ...}).
    property var model: ({})
    property Item anchorItem: null
    property alias popup: popup
    property alias open: popup.open

    // The selected network awaiting a password (a secured SSID). Empty when
    // no join is in progress.
    property string selectedSsid: ""
    property string secret: ""
    property string headerLabel: model.label !== undefined ? model.label : ""
    readonly property var networks: model.networks !== undefined ? model.networks : []
    readonly property bool readOnly: model.readOnly === true
    readonly property bool available: model.state === "available"

    // The menu model: one row per network, each carrying the one action the
    // menu exposes. Disabled when the host reported a polkit read-only
    // degradation.
    readonly property var rows: {
        var out = [];
        for (var i = 0; i < root.networks.length; ++i) {
            var network = root.networks[i] || {};
            out.push({
                index: i,
                ssid: network.ssid !== undefined ? network.ssid : "",
                strength: network.strength !== undefined ? network.strength : 0,
                security: network.security !== undefined ? network.security : "",
                secured: network.secured === true,
                active: network.active === true,
                band: network.band !== undefined ? network.band : "",
                action: "join",
                enabled: !root.readOnly && root.available
            });
        }
        return out;
    }
    readonly property bool passwordPromptVisible: root.selectedSsid.length > 0
                                                 && !root.readOnly

    signal joinRequested(string ssid, string secret)
    signal refreshRequested()

    function activateRow(index) {
        if (index < 0 || index >= root.rows.length)
            return;
        var row = root.rows[index];
        if (!row || !row.enabled)
            return;
        if (row.secured) {
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

    Popup {
        id: popup
        objectName: "wifiPopup"
        anchorItem: root.anchorItem
        preferredWidth: 260
        accessibleRole: Accessible.PopupMenu
        accessibleName: qsTr("Wi-Fi")
        escapeCloses: true

        onOpened: {
            root.selectedSsid = "";
            root.secret = "";
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: root.cancelPassword()

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
                    anchors.verticalCenter: parent.verticalCenter
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
                visible: !root.available
                width: parent.width
                text: qsTr("Wi-Fi unavailable")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Rectangle {
                visible: root.available && root.rows.length > 0
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Repeater {
                objectName: "wifiRows"
                model: root.rows

                delegate: Item {
                    required property var modelData
                    objectName: "wifiRow-" + modelData.ssid
                    readonly property int strength: modelData.strength !== undefined
                                                    ? modelData.strength : 0
                    readonly property bool secured: modelData.secured === true
                    readonly property bool active: modelData.active === true
                    width: parent.width
                    height: secured && root.selectedSsid === modelData.ssid
                            ? Theme.controls.popup.rowHeight * 2
                            : Theme.controls.popup.rowHeight
                    opacity: modelData.enabled ? 1.0 : 0.4

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: rowHover.hovered ? Theme.color.controlHover : "transparent"
                        antialiasing: true
                    }

                    Icon {
                        id: checkGlyph
                        visible: modelData.active
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
                            text: modelData.ssid.length > 0 ? modelData.ssid
                                                            : qsTr("Hidden network")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Text {
                            id: securityText
                            visible: modelData.secured
                            text: modelData.security
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
                                    color: (strength * 4 / 100 > index)
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
                        visible: modelData.secured && root.selectedSsid === modelData.ssid
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
                        onTapped: root.activateRow(modelData.index)
                    }

                    Accessible.role: Accessible.MenuItem
                    Accessible.name: modelData.ssid
                    Accessible.onPressAction: root.activateRow(modelData.index)
                }
            }
        }
    }
}