// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Date & Time sub-pane (T226).
//
// The live clock, NTP, time zone, and manual set are the host's
// (`timedatectl`); the 24-hour, seconds, and date rows are settingsd keys the
// shell's menu-bar clock consumes. The pane is opened in place from General.
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

    property alias ntpToggle: ntpToggle
    property alias timezoneSelect: timezoneSelect
    property alias manualField: manualField
    property alias setTimeButton: setTimeButton
    property alias hour24Toggle: hour24Toggle
    property alias secondsToggle: secondsToggle
    property alias showDateToggle: showDateToggle
    property alias previewText: previewText
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function applyManualTime() {
        if (root.available && manualField.text.length > 0)
            Settings.setTime(manualField.text);
    }

    function previewTime() {
        const date = new Date(root.localTimeMs);
        const time = root.use24Hour
            ? Qt.formatTime(date, root.showSeconds ? "HH:mm:ss" : "HH:mm")
            : Qt.formatTime(date, root.showSeconds ? "h:mm:ss AP" : "h:mm AP");
        return root.showDate ? Qt.formatDate(date, Locale.LongFormat) + " · " + time : time;
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            width: parent.width
            text: qsTr("Set the system clock, time zone, and the format the menu-bar clock uses.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            visible: root.available

            SettingsRow {
                width: parent.width
                label: qsTr("Set time and date automatically")
                description: root.ntpEnabled ? qsTr("Using network time") : qsTr("Manual time")
                controlData: Toggle {
                    id: ntpToggle
                    objectName: "dateTimeNtpToggle"
                    checked: root.ntpEnabled
                    enabled: root.canNtp
                    accessibleName: qsTr("Set time and date automatically")
                    onToggled: (checked) => Settings.setNtp(checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Time zone")
                controlData: Select {
                    id: timezoneSelect
                    objectName: "dateTimeTimezoneSelect"
                    model: root.timezones
                    currentIndex: root.currentTimezoneIndex
                    accessibleName: qsTr("Time zone")
                    onActivated: (index) => {
                        if (index >= 0 && index < root.timezones.length)
                            Settings.setTimezone(root.timezones[index]);
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Date and time")
                description: qsTr("Requires administrator authentication.")
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Rectangle {
                        width: 190
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
                            enabled: !root.ntpEnabled
                            selectByMouse: true
                            inputMask: "0000-00-00 00:00:00"
                        }
                    }

                    Button {
                        id: setTimeButton
                        objectName: "dateTimeSetButton"
                        text: qsTr("Set")
                        enabled: !root.ntpEnabled && manualField.text.length > 0
                        onClicked: root.applyManualTime()
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Clock Format")
            visible: root.available

            SettingsRow {
                width: parent.width
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
                showSeparator: false
                controlData: Toggle {
                    id: showDateToggle
                    objectName: "dateTimeShowDateToggle"
                    checked: root.showDate
                    accessibleName: qsTr("Show date")
                    onToggled: (checked) => Settings.set("menu.clock.showDate", checked)
                }
            }
        }

        Text {
            id: previewText
            objectName: "dateTimePreview"
            width: parent.width
            visible: root.available
            horizontalAlignment: Text.AlignHCenter
            text: root.previewTime()
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeXl
            font.weight: Theme.primitive.font.weightSemibold
        }

        SettingsGroup {
            width: parent.width
            visible: !root.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "dateTimeAbsenceNote"
                kind: "disabled-with-reason"
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Date & Time is unavailable because timedatectl is not running.")
            }
        }
    }
}
