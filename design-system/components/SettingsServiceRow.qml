// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// One service-owned Settings row (T286): a leading glyph tile, the service
// name, a status line, and either a switch or a status dot. The pane supplies
// the real service state; this component only draws the shared anatomy.
SettingsRow {
    id: root

    // "dot" shows the connection state, "switch" shows the service toggle.
    property string trailingKind: "dot"
    property string statusText: ""
    property bool serviceActive: true
    property string serviceGlyph: ""
    property color serviceColor: Theme.color.accent
    // T288: a pane that owns the switch state (the firewall adapter) binds
    // `serviceActive` to its live view and handles `switchToggled`. The row
    // then never writes `serviceActive` itself, so a refused host write
    // reverts from the view instead of leaving a stale local value.
    property bool switchBound: false
    property bool switchEnabled: true
    property alias statusDot: statusDot
    property alias serviceSwitch: serviceSwitch

    signal switchToggled(bool checked)

    description: root.statusText
    rowHeight: Theme.controls.settingsRow.iconHeight
    leadingWidth: Theme.controls.settingsRow.leadingTile
    leadingHeight: Theme.controls.settingsRow.leadingTile
    labelIndent: Theme.controls.settingsRow.paddingH
                 + Theme.controls.settingsRow.leadingTile
                 + Theme.primitive.spacing.md

    leadingData: Item {
        id: serviceTile
        width: Theme.controls.settingsRow.leadingTile
        height: width

        Rectangle {
            anchors.fill: parent
            radius: Theme.primitive.radius.sm
            color: Theme.color.controlFill
            antialiasing: true
        }

        Icon {
            anchors.centerIn: parent
            name: root.serviceGlyph
            size: Theme.controls.button.iconSize
            color: root.serviceActive ? root.serviceColor
                                      : Theme.color.textTertiary
        }
    }

    controlData: Item {
        width: root.trailingKind === "switch"
               ? serviceSwitch.implicitWidth : statusDot.width
        height: root.trailingKind === "switch"
                ? serviceSwitch.implicitHeight : statusDot.height

        Item {
            id: statusDot
            objectName: "settingsServiceStatusDot"
            visible: root.trailingKind === "dot"
            width: Theme.primitive.spacing.sm
            height: width
            anchors.centerIn: parent

            Rectangle {
                anchors.fill: parent
                radius: width / 2
                color: root.serviceActive ? Theme.color.success
                                          : Theme.color.textTertiary
                antialiasing: true
            }

            Accessible.role: Accessible.StaticText
            Accessible.name: root.statusText
        }

        Toggle {
            id: serviceSwitch
            visible: root.trailingKind === "switch"
            anchors.centerIn: parent
            enabled: root.switchEnabled && root.enabled
            checked: root.serviceActive
            rowLabel: root.label
            onToggled: (checked) => {
                if (root.switchBound) {
                    root.switchToggled(checked);
                    return;
                }
                root.serviceActive = checked;
            }
        }
    }
}
