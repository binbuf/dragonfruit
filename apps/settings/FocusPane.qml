// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Focus pane (T-15.7b; rebuilt by T278).
//
// Focus is the notification service's suppression policy: the mode and the
// per-app allow list are the same state the Notifications pane's
// "Application Notifications" inventory reads, projected by the T-15.7a
// adapter and served by the bridge host. The Control Center Focus tile writes
// the same policy, so the surfaces stay in step with no extra plumbing.
//
// T278 replaces the three duplicate state surfaces (a profile selector, a
// mode selector, and inert mode rows) with one Focus list and one detail:
//   * the list is the service's profiles plus Off and any user-added Focus;
//     every row is a real radio control, not decoration;
//   * the detail shows the active Focus, the allowed apps/people (with a
//     filter), and the schedule/shortcut;
//   * Add Focus / Delete Focus are real actions over
//     `focus.customProfiles`/`focus.customProfile`;
//   * the schedule and shortcut stay live when the service is absent.
//
// Deviations from the macOS capture (ADR 0122): there is no Focus capture in
// the reference set, so this follows the shared Settings language rather than
// the Apple-only "allowed people" sheet; the Apple Account/sidebar row and the
// `?` help control are omitted project-wide. Filters are the Linux adaptation
// of "Focus Filters" (ADR 0279).
Item {
    id: root

    // The bridge host's notifications view (empty when absent).
    readonly property var view: Settings.notifications
    readonly property bool available: Settings.notificationsAvailable
    readonly property bool ready: root.available && view.state === "available"
    readonly property var apps: view.apps !== undefined ? view.apps : []
    readonly property var people: view.people !== undefined ? view.people : []
    readonly property var profiles: view.profiles !== undefined ? view.profiles : []
    readonly property string mode:
        view.mode !== undefined ? String(view.mode) : "off"
    readonly property string profile:
        view.profile !== undefined && view.profile !== null
            ? String(view.profile) : ""
    readonly property string profileLabel:
        view.profileLabel !== undefined && view.profileLabel !== null
            && String(view.profileLabel).length > 0
            ? String(view.profileLabel) : qsTr("Off")
    readonly property int batched:
        view.batched !== undefined ? Number(view.batched) : 0

    // ── User-added Focus names (settingsd; the pane is the consumer) ─────
    readonly property var customProfiles: {
        var value = Settings.values["focus.customProfiles"];
        return value === undefined || value === null ? [] : Array.from(value);
    }
    readonly property string customProfile:
        Settings.values["focus.customProfile"] !== undefined
            ? String(Settings.values["focus.customProfile"]) : ""

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
    property string appFilter: "all"
    property string peopleFilter: "all"
    property string addError: ""

    readonly property var filterOptions: [
        { value: "all", label: qsTr("All") },
        { value: "allowed", label: qsTr("Allowed") },
        { value: "blocked", label: qsTr("Not allowed") }
    ]

    // One list: Off, the service's named profiles, then the user-added names.
    readonly property var focusEntries: {
        var out = [{ id: "off", name: qsTr("Off"), mode: "off",
                     description: qsTr("Notifications are delivered normally."),
                     custom: false }];
        var hasCustomNames = root.customProfiles.length > 0;
        for (var i = 0; i < root.profiles.length; ++i) {
            var profile = root.profiles[i];
            var id = String(profile.id);
            if (id === "custom" && hasCustomNames)
                continue;
            out.push({
                id: id,
                name: String(profile.name),
                mode: String(profile.mode),
                description: String(profile.mode) === "dnd"
                    ? qsTr("Only allow-listed apps banner; the rest are batched.")
                    : qsTr("Allow-listed apps and critical alerts banner; the "
                           + "rest are batched."),
                custom: false
            });
        }
        for (var j = 0; j < root.customProfiles.length; ++j) {
            var name = String(root.customProfiles[j]);
            out.push({
                id: "custom:" + name,
                name: name,
                mode: "focus",
                description: qsTr("A user-added Focus over the custom profile."),
                custom: true
            });
        }
        return out;
    }

    readonly property string focusId: {
        if (root.mode === "off")
            return "off";
        if (root.profile === "custom" && root.customProfile.length > 0)
            return "custom:" + root.customProfile;
        return root.profile.length > 0 ? root.profile : "custom";
    }

    readonly property var activeEntry: {
        for (var i = 0; i < root.focusEntries.length; ++i) {
            if (root.focusEntries[i].id === root.focusId)
                return root.focusEntries[i];
        }
        return root.focusEntries[0];
    }

    readonly property var filteredApps: {
        var out = [];
        for (var i = 0; i < root.apps.length; ++i) {
            var allowed = root.apps[i].allowed === true;
            if (root.appFilter === "allowed" && !allowed)
                continue;
            if (root.appFilter === "blocked" && allowed)
                continue;
            out.push(root.apps[i]);
        }
        return out;
    }

    readonly property var filteredPeople: {
        var out = [];
        for (var i = 0; i < root.people.length; ++i) {
            var allowed = root.people[i].allowed === true;
            if (root.peopleFilter === "allowed" && !allowed)
                continue;
            if (root.peopleFilter === "blocked" && allowed)
                continue;
            out.push(root.people[i]);
        }
        return out;
    }

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
    property alias focusList: focusList
    property alias focusDetail: focusDetail
    property alias summary: summary
    property alias profileSummary: profileSummary
    property alias addFocusButton: addFocusButton
    property alias deleteFocusButton: deleteFocusButton
    property alias addFocusDialog: addFocusDialog
    property alias addFocusInput: addFocusInput
    property alias deleteFocusDialog: deleteFocusDialog
    property alias allowGroup: allowGroup
    property alias allowList: allowList
    property alias appFilterSelect: appFilterSelect
    property alias absenceNote: absenceNote
    property alias noAppsNote: noAppsNote
    property alias peopleGroup: peopleGroup
    property alias peopleList: peopleList
    property alias peopleFilterSelect: peopleFilterSelect
    property alias noPeopleNote: noPeopleNote
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

    function activateFocus(id) {
        if (id === "off") {
            Settings.set("focus.customProfile", "");
            Settings.setFocusMode("off");
            return;
        }
        if (id.indexOf("custom:") === 0) {
            Settings.set("focus.customProfile", id.substring(7));
            Settings.setFocusProfile("custom");
            return;
        }
        Settings.set("focus.customProfile", "");
        Settings.setFocusProfile(id);
    }

    function focusNameExists(name) {
        var needle = String(name).trim().toLowerCase();
        if (needle.length === 0)
            return true;
        for (var i = 0; i < root.focusEntries.length; ++i) {
            if (String(root.focusEntries[i].name).trim().toLowerCase() === needle)
                return true;
        }
        return false;
    }

    function addFocus() {
        var name = String(root.addFocusInput.text).trim();
        if (name.length === 0) {
            root.addError = qsTr("Enter a name for the Focus.");
            return;
        }
        if (root.focusNameExists(name)) {
            root.addError = qsTr("A Focus with that name already exists.");
            return;
        }
        var list = root.customProfiles.slice();
        list.push(name);
        Settings.setList("focus.customProfiles", list);
        root.addError = "";
        root.addFocusDialog.accept();
        Settings.set("focus.customProfile", name);
        Settings.setFocusProfile("custom");
    }

    function deleteActiveFocus() {
        var entry = root.activeEntry;
        if (!entry || !entry.custom)
            return;
        var name = entry.name;
        var list = root.customProfiles.slice();
        var at = list.indexOf(name);
        if (at >= 0)
            list.splice(at, 1);
        Settings.setList("focus.customProfiles", list);
        if (root.customProfile === name) {
            Settings.set("focus.customProfile", "");
            Settings.setFocusMode("off");
        }
        root.deleteFocusDialog.accept();
    }

    function validTime(text) {
        var trimmed = String(text).trim();
        return /^([01][0-9]|2[0-3]):[0-5][0-9]$/.test(trimmed);
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
                .arg(root.profileLabel)
                .arg(root.batched);
        return qsTr("%1 is on. Notifications are silenced.")
            .arg(root.profileLabel);
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
                           + "service is not running, so the Focus list is not "
                           + "available. The schedule and shortcut preferences "
                           + "below are still live.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── The one Focus list ────────────────────────────────────────────
        SettingsGroup {
            id: focusDetail
            width: parent.width
            title: qsTr("Focus")
            visible: root.ready

            Repeater {
                id: focusList
                objectName: "focusList"
                model: root.focusEntries

                delegate: Item {
                    id: focusRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    height: focusRowRow.implicitHeight
                    readonly property bool active: root.focusId === modelData.id
                    property alias row: focusRowRow

                    Rectangle {
                        anchors.fill: parent
                        color: focusRow.active ? Theme.color.accentMuted
                                               : (focusHover.hovered
                                                  ? Theme.color.controlHover
                                                  : "transparent")
                        opacity: focusRow.active ? 1 : 0.5
                    }

                    SettingsRow {
                        id: focusRowRow
                        anchors.fill: parent
                        label: focusRow.modelData.name
                        description: focusRow.modelData.description
                        showSeparator: focusRow.index < focusList.count - 1

                        controlData: Icon {
                            visible: focusRow.active
                            name: "check"
                            size: Theme.controls.button.iconSize
                            color: Theme.color.accent
                        }
                    }

                    HoverHandler { id: focusHover }

                    TapHandler {
                        onTapped: root.activateFocus(focusRow.modelData.id)
                    }

                    Accessible.role: Accessible.RadioButton
                    Accessible.name: focusRow.modelData.name
                    Accessible.description: focusRow.modelData.description
                    Accessible.checkable: true
                    Accessible.checked: focusRow.active
                    Accessible.focusable: true
                    Accessible.onPressAction:
                        root.activateFocus(focusRow.modelData.id)
                }
            }

            Text {
                id: profileSummary
                objectName: "focusProfileSummary"
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                text: root.mode === "off"
                    ? qsTr("No Focus is active.")
                    : qsTr("%1 is active.").arg(root.activeEntry.name)
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
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
                topPadding: Theme.primitive.spacing.xs
            }

            Row {
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                spacing: Theme.primitive.spacing.sm
                topPadding: Theme.primitive.spacing.sm
                bottomPadding: Theme.primitive.spacing.sm

                Button {
                    id: addFocusButton
                    text: qsTr("Add Focus\u2026")
                    accessibleName: qsTr("Add a Focus")
                    onClicked: {
                        root.addError = "";
                        root.addFocusInput.text = "";
                        root.addFocusDialog.show();
                    }
                }

                Button {
                    id: deleteFocusButton
                    text: qsTr("Delete Focus\u2026")
                    variant: "ghost"
                    enabled: root.activeEntry.custom === true
                    accessibleName: qsTr("Delete the active Focus")
                    onClicked: root.deleteFocusDialog.show()
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

            SettingsRow {
                width: parent.width
                label: qsTr("Show")
                settingsId: "appFilter"
                showSeparator: false
                controlData: Select {
                    id: appFilterSelect
                    accessibleName: qsTr("Filter the allowed apps")
                    model: root.filterOptions
                    onActivated: (index) => root.appFilter =
                        root.filterOptions[index].value
                }
            }

            Text {
                id: noAppsNote
                objectName: "focusNoApps"
                width: parent.width
                visible: allowList.count === 0
                text: root.appFilter === "all"
                    ? qsTr("No applications have sent a notification yet.")
                    : qsTr("No applications match this filter.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: allowList
                objectName: "focusAllowList"
                model: root.filteredApps
                delegate: Item {
                    id: allowRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    height: row.implicitHeight
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

            Text {
                objectName: "focusPeopleDescription"
                width: parent.width
                text: qsTr("Notifications from these people are delivered even "
                           + "while Focus or Do Not Disturb is on. A person "
                           + "appears here when an app supplies the optional "
                           + "sender hint with its notification.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                bottomPadding: Theme.primitive.spacing.xs
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show")
                settingsId: "peopleFilter"
                showSeparator: false
                controlData: Select {
                    id: peopleFilterSelect
                    accessibleName: qsTr("Filter the allowed people")
                    model: root.filterOptions
                    onActivated: (index) => root.peopleFilter =
                        root.filterOptions[index].value
                }
            }

            Text {
                id: noPeopleNote
                objectName: "focusNoPeople"
                width: parent.width
                visible: peopleList.count === 0
                text: root.peopleFilter === "all"
                    ? qsTr("No people have sent a notification yet.")
                    : qsTr("No people match this filter.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: peopleList
                objectName: "focusPeopleList"
                model: root.filteredPeople
                delegate: Item {
                    id: personRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    height: personSettingsRow.implicitHeight
                    property alias personToggle: personToggle

                    SettingsRow {
                        id: personSettingsRow
                        anchors.fill: parent
                        showSeparator: personRow.index < peopleList.count - 1
                        label: personRow.modelData.name
                        description: Number(personRow.modelData.notifications || 0) > 0
                            ? qsTr("%1 recorded").arg(Number(personRow.modelData.notifications))
                            : qsTr("No notifications yet")

                        controlData: Toggle {
                            id: personToggle
                            objectName: "focusPersonToggle"
                            accessibleName: qsTr("Allow %1").arg(personRow.modelData.name)
                            onToggled: (checked) => Settings.setFocusPerson(
                                personRow.modelData.name, checked)
                        }
                    }

                    Binding {
                        target: personToggle
                        property: "checked"
                        value: personRow.modelData.allowed === true
                    }
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

    // Add Focus (T278): a named Focus over the service's custom profile.
    Dialog {
        id: addFocusDialog
        objectName: "focusAddDialog"
        title: qsTr("Add Focus")
        message: qsTr("Name the Focus. It is listed with the service's "
                      + "profiles and selects the custom profile when active.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Rectangle {
                width: parent.width
                height: 34
                radius: Theme.primitive.radius.md
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: root.addError.length > 0
                    ? Theme.color.danger : Theme.color.border

                TextInput {
                    id: addFocusInput
                    objectName: "focusAddInput"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.rightMargin: Theme.primitive.spacing.md
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: root.addFocus()
                    Accessible.role: Accessible.EditableText
                    Accessible.name: qsTr("New Focus name")
                }
            }

            Text {
                width: parent.width
                visible: root.addError.length > 0
                text: root.addError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                Accessible.role: Accessible.AlertMessage
                Accessible.name: text
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: addFocusDialog.reject()
            }
            Button {
                text: qsTr("Add")
                variant: "primary"
                onClicked: root.addFocus()
            }
        }

        onOpened: addFocusInput.forceActiveFocus()
    }

    Dialog {
        id: deleteFocusDialog
        objectName: "focusDeleteDialog"
        title: qsTr("Delete Focus")
        message: qsTr("Delete %1? The service returns to Off.").arg(
                     root.activeEntry ? root.activeEntry.name : "")
        dismissible: true

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: deleteFocusDialog.reject()
            }
            Button {
                text: qsTr("Delete")
                variant: "danger"
                onClicked: root.deleteActiveFocus()
            }
        }
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
            if (root.scheduleError.length === 0) {
                root.scheduleStartDraft = root.scheduleStart;
                root.scheduleEndDraft = root.scheduleEnd;
            }
            if (root.shortcutError.length === 0)
                root.shortcutDraft = root.shortcut;
        }
    }
}
