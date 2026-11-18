// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Date & Time sub-pane (T226; T270 completion).
//
// The live clock, NTP, time zone, and manual set are the host's
// (`timedatectl`); the 24-hour, seconds, and date rows are settingsd keys the
// shell's menu-bar clock consumes. The pane is opened in place from General.
//
// T270: the time zone is a searchable picker instead of a raw IANA list, the
// manual field is prefilled and validated, and every host write reports its
// busy/denied/error outcome. The settingsd clock rows stay live when
// `timedatectl` is absent; only the host half disables with its reason.
Item {
    id: root

    readonly property var view: Settings.general.time !== undefined
        ? Settings.general.time : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var timezones: root.view.timezones !== undefined
        ? root.view.timezones : []
    readonly property string timezone: root.view.timezone !== undefined
        ? String(root.view.timezone) : ""
    readonly property bool ntpEnabled: root.view.ntpEnabled === true
    readonly property bool canNtp: root.view.canNtp !== false
    readonly property int currentTimezoneIndex: Math.max(0, root.timezones.indexOf(root.timezone))
    readonly property real localTimeMs: root.view.localTimeMs !== undefined
        ? Number(root.view.localTimeMs) : Date.now()
    readonly property bool use24Hour: Settings.values["clock.use24Hour"] === true
    readonly property bool showSeconds: Settings.values["menu.clock.showSeconds"] === true
    readonly property bool showDate: Settings.values["menu.clock.showDate"] !== false
    readonly property string sourceLabel: {
        if (!root.available)
            return qsTr("Unavailable");
        if (root.view.source !== undefined && String(root.view.source).length > 0)
            return String(root.view.source);
        return root.ntpEnabled ? qsTr("Network time (NTP)") : qsTr("Manual");
    }
    // The host reports IANA zone ids, so the city is the zone's last segment.
    // It is a derivation, never a second list that could disagree.
    readonly property string cityLabel: {
        const zone = root.timezone;
        if (zone.length === 0)
            return qsTr("Not reported");
        const parts = zone.split("/");
        return String(parts[parts.length - 1]).replace(/_/g, " ");
    }

    // T270: one host write's live state. `busy` disables the field while the
    // host answers; a denied/failed write keeps the reason next to the row.
    readonly property bool timeBusy: Settings.generalWriteState === "busy"
        && Settings.generalWriteOperation === "time"
    readonly property string timeError: {
        if (Settings.generalWriteOperation !== "time")
            return "";
        if (Settings.generalWriteState === "denied")
            return Settings.generalWriteError.length > 0
                ? qsTr("Authentication was denied: %1").arg(Settings.generalWriteError)
                : qsTr("Authentication was denied.");
        if (Settings.generalWriteState === "failed"
                || Settings.generalWriteState === "absent")
            return Settings.generalWriteError.length > 0
                ? Settings.generalWriteError
                : qsTr("The system clock could not be set.");
        return "";
    }
    readonly property string zoneError: Settings.generalWriteOperation === "timezone"
        && (Settings.generalWriteState === "denied"
            || Settings.generalWriteState === "failed")
        ? Settings.generalWriteError : ""

    // The manual field is prefilled from the host clock and stays editable
    // until the user changes it; a host re-read never overwrites a draft.
    property string manualText: ""
    property bool manualEdited: false
    readonly property string manualDraft: manualField.text
    readonly property string manualError: root.validateManual(root.manualDraft)
    readonly property bool canSetTime: root.available && !root.ntpEnabled
        && !root.timeBusy && root.manualDraft.length > 0
        && root.manualError.length === 0

    property alias ntpToggle: ntpToggle
    property alias timezonePicker: timezonePicker
    property alias timezoneSelect: timezonePicker
    property alias autoZoneToggle: autoZoneToggle
    property alias manualField: manualField
    property alias setTimeButton: setTimeButton
    property alias hour24Toggle: hour24Toggle
    property alias secondsToggle: secondsToggle
    property alias showDateToggle: showDateToggle
    property alias previewText: previewText
    property alias clockOptionsButton: clockOptionsButton
    property alias clockOptionsDialog: clockOptionsDialog
    property alias absenceNote: absenceNote
    property alias hostGroup: hostGroup
    property alias zoneGroup: zoneGroup
    property alias clockGroup: clockGroup
    property alias sourceRow: sourceRow
    property alias cityRow: cityRow

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function pad(value) {
        return value < 10 ? "0" + value : String(value);
    }

    function formatManual(ms) {
        const date = new Date(ms);
        return date.getFullYear() + "-" + root.pad(date.getMonth() + 1) + "-"
            + root.pad(date.getDate()) + " " + root.pad(date.getHours()) + ":"
            + root.pad(date.getMinutes()) + ":" + root.pad(date.getSeconds());
    }

    function refreshManual() {
        if (root.manualEdited)
            return;
        root.manualText = root.formatManual(root.localTimeMs);
    }

    // One `YYYY-MM-DD HH:MM:SS` value, or a reason it is not one.
    function validateManual(text) {
        const value = String(text || "").trim();
        if (value.length === 0)
            return qsTr("Enter a date and time.");
        const match = /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})$/.exec(value);
        if (match === null)
            return qsTr("Use YYYY-MM-DD HH:MM:SS.");
        const year = Number(match[1]);
        const month = Number(match[2]);
        const day = Number(match[3]);
        const hour = Number(match[4]);
        const minute = Number(match[5]);
        const second = Number(match[6]);
        if (month < 1 || month > 12 || day < 1 || day > 31
                || hour > 23 || minute > 59 || second > 59)
            return qsTr("That date and time is not valid.");
        const probe = new Date(year, month - 1, day, hour, minute, second);
        if (probe.getFullYear() !== year || probe.getMonth() !== month - 1
                || probe.getDate() !== day)
            return qsTr("That date and time is not valid.");
        return "";
    }

    function applyManualTime() {
        if (root.canSetTime)
            Settings.setTime(root.manualDraft.trim());
    }

    function previewTime() {
        const date = new Date(root.localTimeMs);
        const time = root.use24Hour
            ? Qt.formatTime(date, root.showSeconds ? "HH:mm:ss" : "HH:mm")
            : Qt.formatTime(date, root.showSeconds ? "h:mm:ss AP" : "h:mm AP");
        return root.showDate ? Qt.formatDate(date, Locale.LongFormat) + " · " + time : time;
    }

    Component.onCompleted: root.refreshManual()
    onLocalTimeMsChanged: root.refreshManual()
    Connections {
        target: Settings
        function onGeneralChanged() { root.refreshManual(); }
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsIntro {
            width: parent.width
            text: qsTr("Set the system clock, time zone, and the format the menu-bar clock uses.")
        }

        // ── The host half (timedatectl) ──────────────────────────────────
        SettingsGroup {
            id: hostGroup
            width: parent.width
            title: qsTr("Date & Time")

            SettingsRow {
                width: parent.width
                settingsId: "time.auto"
                label: qsTr("Set time and date automatically")
                description: root.available
                    ? (root.ntpEnabled ? qsTr("Using network time")
                                       : qsTr("Manual time"))
                    : qsTr("Unavailable")
                controlData: Toggle {
                    id: ntpToggle
                    objectName: "dateTimeNtpToggle"
                    checked: root.ntpEnabled
                    enabled: root.available && root.canNtp
                    accessibleName: qsTr("Set time and date automatically")
                    onToggled: (checked) => Settings.setNtp(checked)
                }
            }

            SettingsRow {
                id: sourceRow
                objectName: "dateTimeSourceRow"
                width: parent.width
                settingsId: "time.source"
                label: qsTr("Source")
                description: qsTr("The time source reported by the host.")
                controlData: Text {
                    objectName: "dateTimeSource"
                    text: root.sourceLabel
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "time.manual"
                label: qsTr("Date and time")
                description: root.ntpEnabled
                    ? qsTr("Turn off automatic time to set the clock by hand.")
                    : qsTr("Requires administrator authentication.")
                errorText: root.timeError
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Rectangle {
                        width: 168
                        height: Theme.controls.select.height
                        radius: Theme.controls.select.radius
                        color: Theme.color.controlFill
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        TextInput {
                            id: manualField
                            objectName: "dateTimeManualField"
                            anchors.fill: parent
                            anchors.leftMargin: Theme.controls.select.paddingH
                            anchors.rightMargin: Theme.controls.select.paddingH
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            enabled: root.available && !root.ntpEnabled && !root.timeBusy
                            selectByMouse: true
                            inputMask: "0000-00-00 00:00:00"
                            text: root.manualText
                            onTextEdited: {
                                root.manualEdited = true;
                                root.manualText = text;
                            }
                            Accessible.name: qsTr("Date and time")
                            Accessible.description: root.manualError
                        }
                    }

                    Button {
                        id: setTimeButton
                        objectName: "dateTimeSetButton"
                        text: root.timeBusy ? qsTr("Setting…") : qsTr("Set")
                        enabled: root.canSetTime
                        onClicked: root.applyManualTime()
                    }
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "time.hour24"
                label: qsTr("24-hour time")
                description: qsTr("The default follows the system locale.")
                controlData: Toggle {
                    id: hour24Toggle
                    objectName: "dateTimeHour24Toggle"
                    checked: root.use24Hour
                    accessibleName: qsTr("24-hour time")
                    onToggled: (checked) => Settings.set("clock.use24Hour", checked)
                }
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "dateTimeAbsenceNote"
                kind: "disabled-with-reason"
                show: !root.available
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("The system clock is unavailable because timedatectl is not running. The clock format still applies.")
            }
        }

        SettingsGroup {
            id: zoneGroup
            width: parent.width
            title: qsTr("Time Zone")

            SettingsRow {
                width: parent.width
                settingsId: "time.autoZone"
                label: qsTr("Set time zone automatically")
                // No location service is a documented Linux gap. The row is a
                // real, inert control with the reason, not a hidden row.
                description: qsTr("Needs a location service, which is not installed.")
                controlData: Toggle {
                    id: autoZoneToggle
                    objectName: "dateTimeAutoZoneToggle"
                    checked: false
                    enabled: false
                    accessibleName: qsTr("Set time zone automatically")
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "time.zone"
                label: qsTr("Time zone")
                description: root.timezone.length > 0 ? root.timezone : qsTr("Not reported")
                errorText: root.zoneError
                controlData: SearchablePicker {
                    id: timezonePicker
                    objectName: "dateTimeTimezonePicker"
                    model: root.timezones
                    currentIndex: root.currentTimezoneIndex
                    enabled: root.available
                    accessibleName: qsTr("Time zone")
                    placeholderText: qsTr("Search time zones")
                    emptyText: qsTr("No time zone matches.")
                    onActivated: (index) => {
                        if (index >= 0 && index < root.timezones.length)
                            Settings.setTimezone(root.timezones[index]);
                    }
                }
            }

            SettingsRow {
                id: cityRow
                objectName: "dateTimeCityRow"
                width: parent.width
                settingsId: "time.city"
                label: qsTr("Time zone city")
                description: qsTr("Derived from the selected time zone.")
                showSeparator: false
                controlData: Text {
                    objectName: "dateTimeCity"
                    text: root.cityLabel
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── The settingsd clock rows stay live ───────────────────────────
        SettingsGroup {
            id: clockGroup
            width: parent.width
            title: qsTr("Clock Format")

            SettingsRow {
                width: parent.width
                label: qsTr("Show seconds")
                controlData: Toggle {
                    id: secondsToggle
                    objectName: "dateTimeSecondsToggle"
                    checked: root.showSeconds
                    accessibleName: qsTr("Show seconds")
                    onToggled: (checked) => Settings.set("menu.clock.showSeconds", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show date")
                controlData: Toggle {
                    id: showDateToggle
                    objectName: "dateTimeShowDateToggle"
                    checked: root.showDate
                    accessibleName: qsTr("Show date")
                    onToggled: (checked) => Settings.set("menu.clock.showDate", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Clock options…")
                showSeparator: false
                controlData: Button {
                    id: clockOptionsButton
                    objectName: "dateTimeClockOptionsButton"
                    text: qsTr("Open…")
                    onClicked: root.clockOptionsDialog.show()
                }
            }
        }

        Text {
            id: previewText
            objectName: "dateTimePreview"
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: root.previewTime()
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeXl
            font.weight: Theme.primitive.font.weightSemibold
        }
    }

    // The same three settingsd rows in a sheet, matching the reference's
    // "Clock options…" surface without inventing a second clock policy.
    Dialog {
        id: clockOptionsDialog
        objectName: "dateTimeClockOptionsDialog"
        title: qsTr("Clock Options")
        message: qsTr("The menu-bar clock uses these settings.")
        dismissible: true

        contentData: Column {
            width: parent ? parent.width : 0
            spacing: Theme.primitive.spacing.sm

            SettingsRow {
                width: parent.width
                label: qsTr("24-hour time")
                controlData: Toggle {
                    checked: root.use24Hour
                    accessibleName: qsTr("24-hour time")
                    onToggled: (checked) => Settings.set("clock.use24Hour", checked)
                }
            }
            SettingsRow {
                width: parent.width
                label: qsTr("Show seconds")
                controlData: Toggle {
                    checked: root.showSeconds
                    accessibleName: qsTr("Show seconds")
                    onToggled: (checked) => Settings.set("menu.clock.showSeconds", checked)
                }
            }
            SettingsRow {
                width: parent.width
                label: qsTr("Show date")
                showSeparator: false
                controlData: Toggle {
                    checked: root.showDate
                    accessibleName: qsTr("Show date")
                    onToggled: (checked) => Settings.set("menu.clock.showDate", checked)
                }
            }
        }

        buttonsData: Button {
            text: qsTr("Done")
            variant: "primary"
            onClicked: clockOptionsDialog.accept()
        }
    }
}
