// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Notifications pane (T-15.7b).
//
// The pane and the Control Center Focus tile are one functional unit over the
// notification service: this pane reads the Focus/DND policy and the observed
// per-app notification list through the bridge host's notifications adapter
// (T-15.7a/b), and it owns the four global "Notification Center" presentation
// preferences as settingsd keys (ADR 0131) — every control applies live.
//
// The Focus mode and the per-app allow list are the sibling Focus pane's
// edits; they share this same adapter view. The per-app "Application
// Notifications" rows here are the observed inventory, so they carry no
// disclosure chevron (the macOS per-app detail sheet is Apple-only surface
// with no Linux provider yet); the state subtext names whether the app is on
// the allow list.
//
// Deviations from the macOS capture (ADR 0122), recorded so later tasks do not
// re-litigate them:
//   * the header's `Learn more...` link is omitted (Apple-only, ADR 0118);
//   * `Show previews` maps onto the three settingsd choices
//     (`Always`/`When Unlocked`/`Never`); the sleeping/locked/mirroring rows
//     are settingsd booleans rather than the capture's mixed popup/toggle
//     shapes, so one control vocabulary covers all three;
//   * the Apple Account/sidebar row and the `?` help control are omitted
//     project-wide.
Item {
    id: root

    // The bridge host's notifications view (empty when absent).
    readonly property var view: Settings.notifications
    readonly property bool available: Settings.notificationsAvailable
    readonly property bool ready: root.available && view.state === "available"
    readonly property var apps: view.apps !== undefined ? view.apps : []
    readonly property string focusLabel:
        view.modeLabel !== undefined ? String(view.modeLabel) : qsTr("Off")

    // The settingsd-owned presentation preferences (schema revision 14).
    readonly property string showPreviews:
        Settings.values["notifications.showPreviews"] !== undefined
            ? String(Settings.values["notifications.showPreviews"]) : "when-unlocked"
    readonly property bool showWhenSleeping:
        Settings.values["notifications.showWhenSleeping"] === true
    readonly property bool showWhenLocked:
        Settings.values["notifications.showWhenLocked"] !== false
    readonly property bool showWhenMirroring:
        Settings.values["notifications.showWhenMirroring"] === true

    readonly property var previewOptions: [
        { value: "always", label: qsTr("Always") },
        { value: "when-unlocked", label: qsTr("When Unlocked") },
        { value: "never", label: qsTr("Never") }
    ]

    // Test surface (used by tst_settings_notifications.qml).
    property alias centerGroup: centerGroup
    property alias previewSelect: previewSelect
    property alias sleepingToggle: sleepingToggle
    property alias lockedToggle: lockedToggle
    property alias mirroringToggle: mirroringToggle
    property alias appGroup: appGroup
    property alias appList: appList
    property alias absenceNote: absenceNote
    property alias noAppsNote: noAppsNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function previewIndex(value) {
        for (var i = 0; i < root.previewOptions.length; ++i) {
            if (root.previewOptions[i].value === value)
                return i;
        }
        return 1;
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
                objectName: "notificationsAbsenceNote"
                width: parent.width
                text: qsTr("The system status service or the notification "
                           + "service is not running, so per-app notification "
                           + "settings are not available. The preferences below "
                           + "are still applied.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Notification Center ───────────────────────────────────────────
        SettingsGroup {
            id: centerGroup
            width: parent.width
            title: qsTr("Notification Center")

            Text {
                objectName: "notificationsCenterDescription"
                width: parent.width
                text: qsTr("Notification Center shows your notifications in the "
                           + "top-right corner of your screen. You can show and "
                           + "hide Notification Center by clicking the clock in "
                           + "the menu bar.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                bottomPadding: Theme.primitive.spacing.xs
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show previews")
                controlData: Select {
                    id: previewSelect
                    accessibleName: qsTr("Show previews")
                    model: root.previewOptions
                    onActivated: (index) => Settings.set(
                        "notifications.showPreviews",
                        root.previewOptions[index].value)
                }
            }

            Text {
                objectName: "notificationsShowLabel"
                width: parent.width
                text: qsTr("Show Notifications:")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                topPadding: Theme.primitive.spacing.xs
            }

            SettingsRow {
                width: parent.width
                label: qsTr("When display is sleeping")
                controlData: Toggle {
                    id: sleepingToggle
                    accessibleName: qsTr("When display is sleeping")
                    onToggled: (checked) => Settings.set(
                        "notifications.showWhenSleeping", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("When screen is locked")
                controlData: Toggle {
                    id: lockedToggle
                    accessibleName: qsTr("When screen is locked")
                    onToggled: (checked) => Settings.set(
                        "notifications.showWhenLocked", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("When mirroring or sharing the display")
                showSeparator: false
                controlData: Toggle {
                    id: mirroringToggle
                    accessibleName: qsTr("When mirroring or sharing the display")
                    onToggled: (checked) => Settings.set(
                        "notifications.showWhenMirroring", checked)
                }
            }
        }

        // ── Application Notifications ─────────────────────────────────────
        SettingsGroup {
            id: appGroup
            width: parent.width
            title: qsTr("Application Notifications")
            visible: root.ready

            Text {
                id: noAppsNote
                objectName: "notificationsNoApps"
                width: parent.width
                visible: appList.count === 0
                text: qsTr("No applications have sent a notification yet.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: appList
                objectName: "notificationsAppList"
                model: root.apps
                delegate: Item {
                    id: appRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    height: Math.max(44, appName.implicitHeight + appStatus.implicitHeight
                                         + 2 * Theme.primitive.spacing.xs)

                    Rectangle {
                        id: appTile
                        width: 28
                        height: 28
                        radius: 8
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        color: appRow.modelData.allowed === true
                            ? Theme.color.accentMuted : Theme.color.controlFill

                        Icon {
                            anchors.centerIn: parent
                            name: "bell"
                            size: 15
                            color: appRow.modelData.allowed === true
                                ? Theme.color.accent : Theme.color.textSecondary
                        }
                    }

                    Column {
                        anchors.left: appTile.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: appStatusText.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            id: appName
                            objectName: "notificationsAppName"
                            width: parent.width
                            text: appRow.modelData.name
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Text {
                            id: appStatus
                            objectName: "notificationsAppState"
                            width: parent.width
                            text: {
                                var count = Number(appRow.modelData.notifications || 0);
                                var state = String(appRow.modelData.status || "Default");
                                if (count <= 0)
                                    return state;
                                var countText = count === 1
                                    ? qsTr("1 notification")
                                    : qsTr("%1 notifications").arg(count);
                                return state + " \u00b7 " + countText;
                            }
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        id: appStatusText
                        objectName: "notificationsAppBadge"
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: appRow.modelData.allowed === true ? qsTr("Allowed")
                                                                : qsTr("Default")
                        color: appRow.modelData.allowed === true
                            ? Theme.color.accent : Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Rectangle {
                        width: parent.width
                        height: Theme.controls.window.borderWidth
                        anchors.bottom: parent.bottom
                        visible: appRow.index < appList.count - 1
                        color: Theme.color.separator
                    }
                }
            }

            Text {
                objectName: "notificationsAppsFocusNote"
                width: parent.width
                visible: appList.count > 0
                text: qsTr("Apps on the Focus allow list are marked Allowed; "
                           + "manage the list in the Focus pane.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.xs
            }
        }
    }

    // Keep the popup in step with an external settingsd change.
    Binding {
        target: previewSelect
        property: "currentIndex"
        value: root.previewIndex(root.showPreviews)
    }
    Binding {
        target: sleepingToggle
        property: "checked"
        value: root.showWhenSleeping
    }
    Binding {
        target: lockedToggle
        property: "checked"
        value: root.showWhenLocked
    }
    Binding {
        target: mirroringToggle
        property: "checked"
        value: root.showWhenMirroring
    }
}