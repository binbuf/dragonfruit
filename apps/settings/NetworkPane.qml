// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Network advanced (VPN) pane (T-15.15b).
//
// The pane and the Control Center tile are one functional unit: both read the
// same host-stack adapter (NetworkManager's `vpn`/`wireguard` connections)
// through the bridge host. The macOS Network capture
// (SystemSettings_Network.md / System_Preferences.md) shows Wi-Fi, Firewall,
// and Other Services rows and no VPN sub-pane, so the reference does not pin a
// VPN layout; per ADR 0122 and ADR 0146 the VPN rows and the connections
// behind the service chevrons map to this adapter. Wi-Fi has its own sidebar
// pane (`wifi`) and Firewall has no Linux host owner yet, so this pane carries
// the honest VPN surface: one row per configured connection with a live
// connect/disconnect control, driven by the adapter's two explicit writes.
//
// Deviations from the macOS capture reference (recorded so later tasks do not
// re-litigate them):
//   * `Wi-Fi` is the separate `wifi` pane; `Firewall` has no Linux host owner
//     yet and is not invented as a dead row.
//   * `Thunderbolt Bridge` is Apple-hardware-specific (ADR 0122); host
//     bridges/ethernet are out of scope for the VPN adapter.
//   * Apple-only services (`iCloud Private Relay`) do not appear in the
//     captures and have no Linux owner.
//   * `Learn more...` and the trailing `?` help are omitted project-wide.
Item {
    id: root

    // The bridge host's VPN view (empty when absent).
    readonly property var view: Settings.vpn
    readonly property bool available: Settings.vpnAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var connections: root.view.connections !== undefined
        ? root.view.connections : []
    readonly property int connectionCount: root.view.connectionCount !== undefined
        ? Number(root.view.connectionCount) : 0
    // The polkit degradation: reads stay live, the writes disable, and the
    // note explains why (ADR 0146).
    readonly property bool readOnly: root.view.readOnly === true
    readonly property string readOnlyMessage: root.view.note !== undefined
        ? String(root.view.note) : ""

    // Test surface (used by tst_settings_network.qml).
    property alias vpnGroup: vpnGroup
    property alias absenceNote: absenceNote
    property alias emptyNote: emptyNote
    property alias readOnlyNote: readOnlyNote
    property alias connectionRepeater: connectionRepeater

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshVpn()

    function isConnected(uuid) {
        for (var i = 0; i < root.connections.length; ++i)
            if (String(root.connections[i].uuid) === String(uuid))
                return root.connections[i].connected === true;
        return false;
    }

    // The live connect/disconnect write for one connection. The host re-reads
    // and pushes the new view back through `vpnChanged`.
    function setConnected(uuid, connected) {
        if (connected)
            Settings.connectVpn(String(uuid));
        else
            Settings.disconnectVpn(String(uuid));
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The configured VPN connections ────────────────────────────────
        SettingsGroup {
            id: vpnGroup
            width: parent.width
            visible: root.ready
            title: qsTr("VPN")

            Repeater {
                id: connectionRepeater
                model: root.connections
                delegate: SettingsRow {
                    id: connectionRow
                    required property var modelData
                    required property int index
                    objectName: "networkVpnRow"
                    width: parent.width
                    label: connectionRow.modelData.id !== undefined
                        ? String(connectionRow.modelData.id) : ""
                    description: connectionRow.modelData.label !== undefined
                        ? String(connectionRow.modelData.label)
                        : String(connectionRow.modelData.kindLabel || "")
                    showSeparator: connectionRow.index < root.connections.length - 1
                    controlData: Toggle {
                        id: connectionToggle
                        objectName: "networkVpnToggle"
                        text: ""
                        accessibleName: qsTr("Connect %1").arg(connectionRow.modelData.id)
                        enabled: !root.readOnly
                        onToggled: (checked) =>
                            root.setConnected(connectionRow.modelData.uuid, checked)

                        // Follow the live state, so a write's round-trip
                        // converges without rebuilding the row.
                        Binding {
                            target: connectionToggle
                            property: "checked"
                            value: root.isConnected(connectionRow.modelData.uuid)
                        }
                    }
                }
            }

            Text {
                id: emptyNote
                objectName: "networkEmptyNote"
                width: parent.width
                visible: root.ready && root.connectionCount === 0
                text: qsTr("No VPN connection is configured.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
            }
        }

        // ── A polkit refusal is not absence ───────────────────────────────
        SettingsGroup {
            width: parent.width

            Text {
                id: readOnlyNote
                objectName: "networkReadOnlyNote"
                width: parent.width
                visible: root.ready && root.readOnly
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

            Text {
                id: absenceNote
                objectName: "networkAbsenceNote"
                width: parent.width
                text: qsTr("The network service (NetworkManager) is not running, "
                           + "so VPN connections are unavailable.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }
}