// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Focus pane (T-15.7b; completed by T229a).
//
// Focus is the notification service's suppression policy: the mode and the
// per-app allow list are the same state the Notifications pane's
// "Application Notifications" inventory reads, projected by the T-15.7a
// adapter and served by the bridge host. The Control Center Focus tile writes
// the same policy, so the three surfaces stay in step with no extra plumbing.
//
// The pane now renders the reference information architecture:
//   * Modes — the service's three modes with their admission rule and the
//     live active state;
//   * Allowed Apps — the per-app Focus allow list;
//   * Allowed People — an explicit Linux absence: the service identifies
//     senders by application, not person, so no dead per-person list is drawn;
//   * Schedule — the durable `focus.schedule.*` keys the shell's
//     `FocusSchedule` reads and applies at the window boundary (ADR 0239);
//   * Shortcut — the durable `focus.shortcut` key (registration is deferred
//     with the shell shortcut engine, like `search.shortcut`).
//
// Deviations from the macOS capture (ADR 0122): there is no Focus capture in
// the reference set, so this follows the shared Settings language rather than
// the Apple-only Focus "allowed people" sheet; the Apple Account/sidebar row
// and the `?` help control are omitted project-wide.
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

    // ── Schedule (settingsd; shell/FocusSchedule consumes it) ────────────
    readonly property bool scheduleEnabled:
        Settings.values["focus.schedule.enabled"] === true
    readonly property string scheduleStart:
        Settings.values["focus.schedule.start"] !== undefined
            ? String(Settings.values["focus.schedule.start"]) : "22:00"
    readonly property string scheduleEnd:
        Settings.values["focus.schedule.end"] !== undefined
            ? String(Settings.values["focus.schedule.end"]) : "07:00"
    readonly property var scheduleDays: {
        var value = Settings.values["focus.schedule.days"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property string shortcut:
        Settings.values["focus.shortcut"] !== undefined
            ? String(Settings.values["focus.shortcut"]) : ""

    property string scheduleStartDraft: root.scheduleStart
    property string scheduleEndDraft: root.scheduleEnd
    property string scheduleError: ""
    property string shortcutDraft: root.shortcut
    property string shortcutError: ""

    readonly property var modeOptions: [
        { value: "off", label: qsTr("Off") },
        { value: "focus", label: qsTr("Focus") },
        { value: "dnd", label: qsTr("Do Not Disturb") }
    ]

    readonly property var modeRows: [
        { id: "off", label: qsTr("Off"),
          description: qsTr("Notifications are delivered normally.") },
        { id: "focus", label: qsTr("Focus"),
          description: qsTr("Allow-listed apps and critical alerts banner; "
                            + "the rest are batched.") },
        { id: "dnd", label: qsTr("Do Not Disturb"),
          description: qsTr("Only allow-listed apps banner; everything else "
                            + "is batched.") }
    ]

    readonly property var dayOptions: [
        { id: "mon", label: qsTr("Mon") },
        { id: "tue", label: qsTr("Tue") },
        { id: "wed", label: qsTr("Wed") },
        { id: "thu", label: qsTr("Thu") },
        { id: "fri", label: qsTr("Fri") },
        { id: "sat", label: qsTr("Sat") },
        { id: "sun", label: qsTr("Sun") }
    ]

    // Test surface (used by tst_settings_notifications.qml).
    property alias modeSelector: modeSelector
    property alias modeRepeater: modeRepeater
    property alias allowGroup: allowGroup
    property alias allowList: allowList
    property alias absenceNote: absenceNote
    property alias noAppsNote: noAppsNote
    property alias summary: summary
    property alias peopleGroup: peopleGroup
    property alias peopleRow: peopleRow
    property alias peopleNote: peopleNote
    property alias scheduleGroup: scheduleGroup
    property alias scheduleToggle: scheduleToggle
    property alias scheduleStartInput: scheduleStartInput
    property alias scheduleEndInput: scheduleEndInput
    property alias scheduleApplyButton: scheduleApplyButton
    property alias scheduleErrorText: scheduleErrorText
    property alias scheduleSummary: scheduleSummaryRow.descriptionText
    property alias dayRepeater: dayRepeater
    property alias shortcutGroup: shortcutGroup
    property alias shortcutInput: shortcutInput
    property alias shortcutApplyButton: shortcutApplyButton
    property alias shortcutErrorText: shortcutErrorText

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

    function validTime(text) {
        var trimmed = String(text).trim();
        if (!/^([01][0-9]|2[0-3]):[0-5][0-9]$/.test(trimmed))
            return false;
        return true;
    }

    function dayEnabled(id) {
        return root.scheduleDays.indexOf(id) >= 0;
    }

    function setDay(id, enabled) {
        var list = root.scheduleDays.slice();
        var at = list.indexOf(id);
        if (enabled && at < 0)
            list.push(id);
        else if (!enabled && at >= 0)
            list.splice(at, 1);
        else
            return;
        Settings.setList("focus.schedule.days", list);
    }

    function applySchedule() {
        if (!root.validTime(root.scheduleStartDraft)
                || !root.validTime(root.scheduleEndDraft)) {
            root.scheduleError = qsTr("Enter both times as 24-hour HH:MM, "
                                      + "for example 22:00.");
            return;
        }
        // Capture both drafts before the first write re-emits values and the
        // convergence binding reseeds the second draft.
        var start = root.scheduleStartDraft.trim();
        var end = root.scheduleEndDraft.trim();
        root.scheduleError = "";
        Settings.set("focus.schedule.start", start);
        Settings.set("focus.schedule.end", end);
    }

    function validShortcut(text) {
        var trimmed = String(text).trim();
        if (trimmed.length === 0)
            return false;
        if (trimmed.charAt(trimmed.length - 1) === "+")
            return false;
        return trimmed.indexOf("+") > 0;
    }

    function applyShortcut() {
        var text = root.shortcutDraft.trim();
        if (text.length === 0) {
            root.shortcutError = "";
            Settings.set("focus.shortcut", "");
            return;
        }
        if (!root.validShortcut(text)) {
            root.shortcutError = qsTr("Use a modifier and a key, for example "
                                      + "Ctrl+Alt+F.");
            return;
        }
        root.shortcutError = "";
        Settings.set("focus.shortcut", text);
    }

    readonly property string scheduleSummaryText: {
        if (!root.scheduleEnabled)
            return qsTr("The schedule is off.");
        var days = [];
        if (root.scheduleDays.length === 0
                || root.scheduleDays.length === root.dayOptions.length) {
            days.push(qsTr("Every day"));
        } else {
            for (var i = 0; i < root.dayOptions.length; ++i) {
                if (root.scheduleDays.indexOf(root.dayOptions[i].id) >= 0)
                    days.push(root.dayOptions[i].label);
            }
        }
        return qsTr("%1, %2\u2013%3").arg(days.join(", "))
            .arg(root.scheduleStart).arg(root.scheduleEnd);
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
                text: root.view.state === "error" && root.view.error !== undefined
                    ? qsTr("The notification service could not be read: %1.")
                        .arg(String(root.view.error))
                    : qsTr("The system status service or the notification "
                           + "service is not running, so Focus is not available. "
                           + "The schedule and shortcut preferences below are "
                           + "still stored.")
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

            Column {
                width: parent.width
                topPadding: Theme.primitive.spacing.sm

                Repeater {
                    id: modeRepeater
                    model: root.modeRows

                    delegate: Item {
                        required property var modelData
                        required property int index
                        width: parent.width
                        height: modeLabel.implicitHeight
                                + modeDescription.implicitHeight
                                + 2 * Theme.primitive.spacing.xs
                        readonly property bool active: root.mode === modelData.id

                        Rectangle {
                            anchors.fill: parent
                            color: parent.active ? Theme.color.accentMuted
                                                 : "transparent"
                        }

                        Column {
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.controls.settingsRow.paddingH
                            anchors.right: activeMark.left
                            anchors.rightMargin: Theme.primitive.spacing.sm
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: Theme.primitive.spacing.xxs

                            Text {
                                id: modeLabel
                                width: parent.width
                                text: modelData.label
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                elide: Text.ElideRight
                            }

                            Text {
                                id: modeDescription
                                width: parent.width
                                text: modelData.description
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                wrapMode: Text.WordWrap
                            }
                        }

                        Icon {
                            id: activeMark
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.controls.settingsRow.paddingH
                            anchors.verticalCenter: parent.verticalCenter
                            visible: parent.active
                            name: "check"
                            size: Theme.controls.button.iconSize
                            color: Theme.color.accent
                        }

                        Accessible.role: Accessible.RadioButton
                        Accessible.name: modelData.label
                        Accessible.description: modelData.description
                        Accessible.checkable: true
                        Accessible.checked: active
                    }
                }
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

        // ── Allowed people ────────────────────────────────────────────────
        SettingsGroup {
            id: peopleGroup
            width: parent.width
            title: qsTr("Allowed People")
            visible: root.ready

            SettingsRow {
                id: peopleRow
                width: parent.width
                label: qsTr("Allowed People")
                description: qsTr("The notification service identifies senders "
                                  + "by application, not person, so a per-person "
                                  + "allow list is not available.")
                settingsId: "people"
                owner: "notification service"
                settingsKey: "focus.people"
                showSeparator: false
                controlData: Text {
                    id: peopleNote
                    objectName: "focusPeopleNote"
                    text: qsTr("Use Allowed Apps")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }
        }

        // ── Schedule ──────────────────────────────────────────────────────
        SettingsGroup {
            id: scheduleGroup
            width: parent.width
            title: qsTr("Schedule")

            SettingsRow {
                id: scheduleRow
                width: parent.width
                label: qsTr("Turn on Focus on a schedule")
                description: qsTr("The shell applies Focus when the window "
                                  + "starts and returns to Off when it ends.")
                settingsId: "schedule"
                owner: "settingsd"
                settingsKey: "focus.schedule.enabled"
                controlData: Toggle {
                    id: scheduleToggle
                    accessibleName: qsTr("Turn on Focus on a schedule")
                    onToggled: (checked) => Settings.set(
                        "focus.schedule.enabled", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("From")
                controlData: Rectangle {
                    width: 110
                    height: Theme.controls.button.height
                    radius: Theme.controls.button.radius
                    color: Theme.color.controlFill
                    border.width: Theme.controls.window.borderWidth
                    border.color: root.scheduleError.length > 0
                        ? Theme.color.danger : Theme.color.border

                    TextInput {
                        id: scheduleStartInput
                        objectName: "focusScheduleStart"
                        anchors.fill: parent
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        selectByMouse: true
                        enabled: root.scheduleEnabled
                        text: root.scheduleStartDraft
                        onTextEdited: {
                            root.scheduleStartDraft = text;
                            root.scheduleError = "";
                        }
                        onAccepted: root.applySchedule()
                        Accessible.role: Accessible.EditableText
                        Accessible.name: qsTr("Focus schedule start time")
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("To")
                controlData: Rectangle {
                    width: 110
                    height: Theme.controls.button.height
                    radius: Theme.controls.button.radius
                    color: Theme.color.controlFill
                    border.width: Theme.controls.window.borderWidth
                    border.color: root.scheduleError.length > 0
                        ? Theme.color.danger : Theme.color.border

                    TextInput {
                        id: scheduleEndInput
                        objectName: "focusScheduleEnd"
                        anchors.fill: parent
                        anchors.leftMargin: Theme.primitive.spacing.sm
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        selectByMouse: true
                        enabled: root.scheduleEnabled
                        text: root.scheduleEndDraft
                        onTextEdited: {
                            root.scheduleEndDraft = text;
                            root.scheduleError = "";
                        }
                        onAccepted: root.applySchedule()
                        Accessible.role: Accessible.EditableText
                        Accessible.name: qsTr("Focus schedule end time")
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Days")
                controlData: Row {
                    spacing: Theme.primitive.spacing.xs

                    Repeater {
                        id: dayRepeater
                        model: root.dayOptions

                        delegate: Button {
                            required property var modelData
                            text: modelData.label
                            variant: root.dayEnabled(modelData.id) ? "primary"
                                                                   : "secondary"
                            enabled: root.scheduleEnabled
                            accessibleName: qsTr("Focus on %1")
                                            .arg(modelData.label)
                            onClicked: root.setDay(modelData.id,
                                                   !root.dayEnabled(modelData.id))
                        }
                    }
                }
            }

            SettingsRow {
                id: scheduleSummaryRow
                width: parent.width
                label: qsTr("Apply schedule")
                description: root.scheduleSummaryText
                showSeparator: false
                controlData: Button {
                    id: scheduleApplyButton
                    text: qsTr("Apply")
                    enabled: root.scheduleEnabled
                    onClicked: root.applySchedule()
                }
            }

            Text {
                id: scheduleErrorText
                objectName: "focusScheduleError"
                width: parent.width
                visible: root.scheduleError.length > 0
                text: root.scheduleError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                Accessible.role: Accessible.AlertMessage
                Accessible.name: text
            }
        }

        // ── Shortcut ──────────────────────────────────────────────────────
        SettingsGroup {
            id: shortcutGroup
            width: parent.width
            title: qsTr("Shortcut")

            SettingsRow {
                width: parent.width
                label: qsTr("Toggle Focus")
                description: qsTr("The global shortcut the shell registers when "
                                  + "shortcut registration lands; empty disables it.")
                settingsId: "shortcut"
                owner: "settingsd"
                settingsKey: "focus.shortcut"
                showSeparator: false
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Rectangle {
                        width: 140
                        height: Theme.controls.button.height
                        radius: Theme.controls.button.radius
                        color: Theme.color.controlFill
                        border.width: Theme.controls.window.borderWidth
                        border.color: root.shortcutError.length > 0
                            ? Theme.color.danger : Theme.color.border

                        TextInput {
                            id: shortcutInput
                            objectName: "focusShortcutInput"
                            anchors.fill: parent
                            anchors.leftMargin: Theme.primitive.spacing.sm
                            anchors.rightMargin: Theme.primitive.spacing.sm
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            selectByMouse: true
                            text: root.shortcutDraft
                            onTextEdited: {
                                root.shortcutDraft = text;
                                root.shortcutError = "";
                            }
                            onAccepted: root.applyShortcut()
                            Accessible.role: Accessible.EditableText
                            Accessible.name: qsTr("Focus keyboard shortcut")
                        }
                    }

                    Button {
                        id: shortcutApplyButton
                        text: qsTr("Apply")
                        enabled: root.shortcutDraft.trim() !== root.shortcut
                        onClicked: root.applyShortcut()
                    }
                }
            }

            Text {
                id: shortcutErrorText
                objectName: "focusShortcutError"
                width: parent.width
                visible: root.shortcutError.length > 0
                text: root.shortcutError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                Accessible.role: Accessible.AlertMessage
                Accessible.name: text
            }
        }
    }

    Binding {
        target: modeSelector
        property: "currentIndex"
        value: root.modeIndex(root.mode)
    }
    Binding {
        target: scheduleToggle
        property: "checked"
        value: root.scheduleEnabled
    }
    Binding {
        target: scheduleStartInput
        property: "text"
        value: root.scheduleStartDraft
    }
    Binding {
        target: scheduleEndInput
        property: "text"
        value: root.scheduleEndDraft
    }
    Binding {
        target: shortcutInput
        property: "text"
        value: root.shortcutDraft
    }
    Connections {
        target: Settings
        function onValuesChanged() {
            // An external settingsd change converges into the drafts unless
            // the user is mid-edit with an invalid value.
            if (root.scheduleError.length === 0) {
                root.scheduleStartDraft = root.scheduleStart;
                root.scheduleEndDraft = root.scheduleEnd;
            }
            if (root.shortcutError.length === 0)
                root.shortcutDraft = root.shortcut;
        }
    }
}
