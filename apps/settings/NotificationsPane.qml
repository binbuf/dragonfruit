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
    property alias focusRow: focusRow
    property alias focusButton: focusButton
    property alias focusGroup: focusGroup
    property alias appDialog: appDialog
    property alias appDialogTitle: appDialogTitle
    property alias appAllowToggle: appAllowToggle
    property alias appStyleSelect: appStyleSelect
    property alias appSoundToggle: appSoundToggle
    property alias appBadgeToggle: appBadgeToggle

    // The app whose detail sheet is open ("" when closed).
    property string selectedAppName: ""
    // The draft the sheet edits; a write updates the service and the next
    // `notificationsChanged` re-seeds these from the authoritative view.
    property bool draftAllowed: true
    property string draftStyle: "banner"
    property bool draftSound: true
    property bool draftBadge: true

    readonly property var appStyleOptions: [
        { value: "banner", label: qsTr("Banners") },
        { value: "alert", label: qsTr("Alerts") },
        { value: "none", label: qsTr("None") }
    ]

    signal navigateRequested(string paneId)

    readonly property string absenceText: {
        if (root.view.state === "error" && root.view.error !== undefined)
            return qsTr("The notification service could not be read: %1. "
                        + "The preferences below are still applied.")
                .arg(String(root.view.error));
        return qsTr("The system status service or the notification service is "
                    + "not running, so per-app notification settings are not "
                    + "available. The preferences below are still applied.");
    }

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

    function styleIndex(value) {
        for (var i = 0; i < root.appStyleOptions.length; ++i) {
            if (root.appStyleOptions[i].value === value)
                return i;
        }
        return 0;
    }

    function appByName(name) {
        for (var i = 0; i < root.apps.length; ++i) {
            if (String(root.apps[i].name).toLowerCase()
                    === String(name).toLowerCase())
                return root.apps[i];
        }
        return null;
    }

    function openAppDetail(app) {
        if (!app)
            return;
        root.selectedAppName = String(app.name);
        root.draftAllowed = app.notificationsAllowed !== false;
        root.draftStyle = String(app.style || "banner");
        root.draftSound = app.sound !== false;
        root.draftBadge = app.badge !== false;
        appDialog.show();
    }

    function seedDraft() {
        var app = root.appByName(root.selectedAppName);
        if (!app)
            return;
        root.draftAllowed = app.notificationsAllowed !== false;
        root.draftStyle = String(app.style || "banner");
        root.draftSound = app.sound !== false;
        root.draftBadge = app.badge !== false;
    }

    function applyAppPreference() {
        if (root.selectedAppName.length === 0)
            return;
        Settings.setNotificationAppPreference(root.selectedAppName,
                                              root.draftStyle,
                                              root.draftSound,
                                              root.draftBadge,
                                              root.draftAllowed);
    }

    Component.onCompleted: Settings.refreshNotifications()

    Connections {
        target: Settings
        function onNotificationsChanged() {
            if (appDialog.open)
                root.seedDraft();
        }
    }

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
                text: root.absenceText
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Notification Center ───────────────────────────────────────────
        SettingsGroup {
            id: centerGroup
            width: parent.width
            title: qsTr("Notifications")
            sectionId: "Notifications"

            Text {
                objectName: "notificationsCenterDescription"
                width: parent.width
                text: qsTr("Notifications appear in the top-right corner of your "
                           + "screen. You can show and hide them by clicking the "
                           + "clock in the menu bar.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                bottomPadding: Theme.primitive.spacing.xs
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show previews")
                settingsId: "previews"
                owner: "settingsd"
                settingsKey: "notifications.showPreviews"
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
                delegate: SettingsRow {
                    id: appRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: appRow.modelData.name
                    showSeparator: appRow.index < appList.count - 1
                    disclosure: true
                    description: {
                        var count = Number(appRow.modelData.notifications || 0);
                        var state = String(appRow.modelData.status || "Default");
                        var style = String(appRow.modelData.styleLabel || "Banners");
                        var parts = [state, style];
                        if (appRow.modelData.sound === false)
                            parts.push(qsTr("No sound"));
                        if (appRow.modelData.badge === false)
                            parts.push(qsTr("No badge"));
                        if (count > 0) {
                            parts.push(count === 1
                                ? qsTr("1 notification")
                                : qsTr("%1 notifications").arg(count));
                        }
                        return parts.join(" \u00b7 ");
                    }
                    onDisclosureClicked: root.openAppDetail(appRow.modelData)

                    controlData: Text {
                        objectName: "notificationsAppBadge"
                        text: appRow.modelData.status === "Allowed" ? qsTr("Allowed")
                              : (appRow.modelData.status === "Off" ? qsTr("Off")
                                                                   : qsTr("Default"))
                        color: appRow.modelData.status === "Allowed"
                            ? Theme.color.accent : Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
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

        // ── Focus ─────────────────────────────────────────────────────────
        SettingsGroup {
            id: focusGroup
            width: parent.width
            title: qsTr("Focus")
            visible: root.ready

            SettingsRow {
                id: focusRow
                width: parent.width
                label: qsTr("Focus")
                description: qsTr("Current mode: %1. Allowed apps and the "
                                  + "schedule are managed in the Focus pane.")
                    .arg(root.focusLabel)
                settingsId: "focus"
                owner: "settingsd"
                settingsKey: "notifications.focus"
                showSeparator: false
                controlData: Button {
                    id: focusButton
                    text: qsTr("Open Focus\u2026")
                    accessibleName: qsTr("Open Focus settings")
                    onClicked: root.navigateRequested("focus")
                }
            }
        }
    }

    // The per-app detail sheet (T229a): the macOS disclosure rows collapsed
    // into one live sheet. Every control writes immediately; the service
    // re-emits and the sheet re-seeds from the authoritative view.
    Dialog {
        id: appDialog
        objectName: "notificationsAppDialog"
        title: qsTr("Notifications for %1").arg(root.selectedAppName)
        message: qsTr("Choose how this application's notifications appear.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                id: appDialogTitle
                objectName: "notificationsAppDialogTitle"
                width: parent.width
                text: root.selectedAppName
                color: Theme.color.textPrimary
                elide: Text.ElideRight
                font.pixelSize: Theme.controls.button.fontSize
                font.weight: Theme.primitive.font.weightMedium
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Allow notifications")
                showSeparator: false
                controlData: Toggle {
                    id: appAllowToggle
                    accessibleName: qsTr("Allow notifications for %1")
                                    .arg(root.selectedAppName)
                    onToggled: (checked) => {
                        root.draftAllowed = checked;
                        root.applyAppPreference();
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Style")
                controlData: Select {
                    id: appStyleSelect
                    accessibleName: qsTr("Notification style for %1")
                                    .arg(root.selectedAppName)
                    model: root.appStyleOptions
                    onActivated: (index) => {
                        root.draftStyle = root.appStyleOptions[index].value;
                        root.applyAppPreference();
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Play sound")
                controlData: Toggle {
                    id: appSoundToggle
                    accessibleName: qsTr("Play a sound for %1")
                                    .arg(root.selectedAppName)
                    onToggled: (checked) => {
                        root.draftSound = checked;
                        root.applyAppPreference();
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show in Dock badge")
                showSeparator: false
                controlData: Toggle {
                    id: appBadgeToggle
                    accessibleName: qsTr("Show a Dock badge for %1")
                                    .arg(root.selectedAppName)
                    onToggled: (checked) => {
                        root.draftBadge = checked;
                        root.applyAppPreference();
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: appDialog.accept()
            }
        }

        onOpened: root.seedDraft()
    }

    Binding {
        target: appAllowToggle
        property: "checked"
        value: root.draftAllowed
    }
    Binding {
        target: appStyleSelect
        property: "currentIndex"
        value: root.styleIndex(root.draftStyle)
    }
    Binding {
        target: appSoundToggle
        property: "checked"
        value: root.draftSound
    }
    Binding {
        target: appBadgeToggle
        property: "checked"
        value: root.draftBadge
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