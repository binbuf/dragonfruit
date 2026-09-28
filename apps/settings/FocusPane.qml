// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Focus pane (T-15.7b).
//
// Focus is the notification service's suppression policy: the mode and the
// per-app allow list are the same state the Notifications pane's
// "Application Notifications" inventory reads, projected by the T-15.7a
// adapter and served by the bridge host. The Control Center Focus tile writes
// the same policy, so the three surfaces stay in step with no extra plumbing.
//
// The mode selector maps onto the service's three modes (`off`/`focus`/`dnd`);
// the allow list is a whole-list replace in the service, so toggling one app
// reads the current list in the bridge, changes one entry, and writes it back.
//
// Deviations from the macOS capture (ADR 0122): there is no Focus capture in
// the reference set, so this follows the shared Settings language rather than
// the Apple-only Focus "allowed people/apps" sheet; the Apple Account/sidebar
// row and the `?` help control are omitted project-wide.
Item {
    id: root

    // The bridge host's notifications view (empty when absent).
    readonly property var view: Settings.notifications
    readonly property bool available: Settings.notificationsAvailable
    readonly property bool ready: root.available && view.state === "available"
    readonly property var apps: view.apps !== undefined ? view.apps : []
    readonly property string mode:
        view.mode !== undefined ? String(view.mode) : "off"
    readonly property int batched:
        view.batched !== undefined ? Number(view.batched) : 0

    readonly property var modeOptions: [
        { value: "off", label: qsTr("Off") },
        { value: "focus", label: qsTr("Focus") },
        { value: "dnd", label: qsTr("Do Not Disturb") }
    ]

    // Test surface (used by tst_settings_notifications.qml).
    property alias modeSelector: modeSelector
    property alias allowGroup: allowGroup
    property alias allowList: allowList
    property alias absenceNote: absenceNote
    property alias noAppsNote: noAppsNote
    property alias summary: summary

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function modeIndex(value) {
        for (var i = 0; i < root.modeOptions.length; ++i) {
            if (root.modeOptions[i].value === value)
                return i;
        }
        return 0;
    }

    readonly property string summaryText: {
        if (!root.ready)
            return qsTr("Focus is not available.");
        if (root.mode === "off")
            return qsTr("Notifications are delivered normally.");
        if (root.batched > 0)
            return qsTr("%1 \u00b7 %2 notifications silenced while active.")
                .arg(root.mode === "dnd" ? qsTr("Do Not Disturb") : qsTr("Focus"))
                .arg(root.batched);
        return root.mode === "dnd"
            ? qsTr("Do Not Disturb is on. Notifications are silenced.")
            : qsTr("Focus is on. Notifications are silenced.");
    }

    Component.onCompleted: Settings.refreshNotifications()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "focusAbsenceNote"
                width: parent.width
                text: qsTr("The system status service or the notification "
                           + "service is not running, so Focus is not available.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Focus mode ────────────────────────────────────────────────────
        SettingsGroup {
            id: modeGroup
            width: parent.width
            title: qsTr("Focus")
            visible: root.ready

            SegmentedControl {
                id: modeSelector
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                model: root.modeOptions
                onActivated: (index) => Settings.setFocusMode(
                    root.modeOptions[index].value)
            }

            Text {
                id: summary
                objectName: "focusSummary"
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                text: root.summaryText
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
            }
        }

        // ── Allowed apps ──────────────────────────────────────────────────
        SettingsGroup {
            id: allowGroup
            width: parent.width
            title: qsTr("Allowed Apps")
            visible: root.ready

            Text {
                objectName: "focusAllowDescription"
                width: parent.width
                text: qsTr("Notifications from these apps are delivered even "
                           + "while Focus or Do Not Disturb is on.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                id: noAppsNote
                objectName: "focusNoApps"
                width: parent.width
                visible: allowList.count === 0
                text: qsTr("No applications have sent a notification yet.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: allowList
                objectName: "focusAllowList"
                model: root.apps
                delegate: Item {
                    id: allowRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    height: row.implicitHeight
                    // The named seam a headless test toggles (the dynamic rows
                    // have no fixed alias of their own).
                    property alias allowToggle: allowToggle

                    SettingsRow {
                        id: row
                        anchors.fill: parent
                        showSeparator: allowRow.index < allowList.count - 1
                        label: allowRow.modelData.name
                        description: Number(allowRow.modelData.notifications || 0) > 0
                            ? qsTr("%1 recorded").arg(Number(allowRow.modelData.notifications))
                            : qsTr("No notifications yet")

                        controlData: Toggle {
                            id: allowToggle
                            objectName: "focusAllowToggle"
                            accessibleName: qsTr("Allow %1").arg(allowRow.modelData.name)
                            onToggled: (checked) => Settings.setFocusApp(
                                allowRow.modelData.name, checked)
                        }
                    }

                    Binding {
                        target: allowToggle
                        property: "checked"
                        value: allowRow.modelData.allowed === true
                    }
                }
            }
        }
    }

    Binding {
        target: modeSelector
        property: "currentIndex"
        value: root.modeIndex(root.mode)
    }
}