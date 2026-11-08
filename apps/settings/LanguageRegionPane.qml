// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Language & Region sub-pane (T226).
//
// The system locale is the host's (`localectl`); the calendar, temperature,
// measurement, date-format, number-format, and first-day rows are durable
// settingsd keys with a live preview. The pane is opened in place from
// General.
Item {
    id: root

    readonly property var view: Settings.general.locale !== undefined
        ? Settings.general.locale : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var locales: root.view.availableLocales !== undefined
        ? root.view.availableLocales : []
    readonly property string locale: root.view.locale !== undefined
        ? String(root.view.locale) : ""
    readonly property int currentLocaleIndex: Math.max(0, root.locales.indexOf(root.locale))

    readonly property var calendarModel: [
        { label: qsTr("Gregorian"), value: "gregorian" },
        { label: qsTr("ISO 8601"), value: "iso" },
        { label: qsTr("Buddhist"), value: "buddhist" },
        { label: qsTr("Japanese"), value: "japanese" }
    ]
    readonly property var temperatureModel: [
        { label: qsTr("Celsius (°C)"), value: "celsius" },
        { label: qsTr("Fahrenheit (°F)"), value: "fahrenheit" }
    ]
    readonly property var measurementModel: [
        { label: qsTr("Metric"), value: "metric" },
        { label: qsTr("Imperial"), value: "imperial" }
    ]
    readonly property var dateFormatModel: [
        { label: qsTr("Short"), value: "short" },
        { label: qsTr("Medium"), value: "medium" },
        { label: qsTr("Long"), value: "long" }
    ]
    readonly property var numberFormatModel: [
        { label: qsTr("Use locale"), value: "locale" },
        { label: qsTr("Decimal point (1,234.5)"), value: "decimal" },
        { label: qsTr("Decimal comma (1.234,5)"), value: "comma" }
    ]
    readonly property var firstDayModel: [
        { label: qsTr("Use locale"), value: "locale" },
        { label: qsTr("Sunday"), value: "sunday" },
        { label: qsTr("Monday"), value: "monday" },
        { label: qsTr("Saturday"), value: "saturday" }
    ]

    function indexOf(model, value) {
        for (var i = 0; i < model.length; ++i) {
            if (model[i].value === value)
                return i;
        }
        return 0;
    }

    function valueOf(key, fallback) {
        return Settings.values[key] !== undefined ? String(Settings.values[key]) : fallback;
    }

    readonly property string calendar: root.valueOf("locale.calendar", "gregorian")
    readonly property string temperature: root.valueOf("locale.temperature", "celsius")
    readonly property string measurement: root.valueOf("locale.measurement", "metric")
    readonly property string dateFormat: root.valueOf("locale.dateFormat", "medium")
    readonly property string numberFormat: root.valueOf("locale.numberFormat", "locale")
    readonly property string firstDay: root.valueOf("locale.firstDay", "locale")

    property alias languageSelect: languageSelect
    property alias calendarSelect: calendarSelect
    property alias temperatureSelect: temperatureSelect
    property alias measurementSelect: measurementSelect
    property alias dateFormatSelect: dateFormatSelect
    property alias numberFormatSelect: numberFormatSelect
    property alias firstDaySelect: firstDaySelect
    property alias previewText: previewText
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function previewDate() {
        const sample = new Date(2026, 10, 7, 14, 5, 0);
        const style = root.dateFormat === "short" ? Locale.ShortFormat
            : root.dateFormat === "long" ? Locale.LongFormat
            : Locale.LongFormat;
        const date = Qt.formatDate(sample, style);
        const time = Qt.formatTime(sample, Qt.locale().timeFormat(Locale.ShortFormat));
        return date + " · " + time;
    }

    function previewNumber() {
        if (root.numberFormat === "comma")
            return "1.234,5";
        if (root.numberFormat === "decimal")
            return "1,234.5";
        return "1,234.5";
    }

    function previewTemperature() {
        return root.temperature === "fahrenheit" ? "68 °F" : "20 °C";
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            width: parent.width
            text: qsTr("Choose the system language and the formats the desktop uses.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Language")
            visible: root.available

            SettingsRow {
                width: parent.width
                label: qsTr("Language")
                description: qsTr("Some apps may need to be restarted.")
                showSeparator: false
                controlData: Select {
                    id: languageSelect
                    objectName: "languageLocaleSelect"
                    model: root.locales
                    currentIndex: root.currentLocaleIndex
                    accessibleName: qsTr("Language")
                    onActivated: (index) => {
                        if (index >= 0 && index < root.locales.length)
                            Settings.setLocale(root.locales[index]);
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Region")
            visible: root.available

            SettingsRow {
                width: parent.width
                label: qsTr("Region")
                description: qsTr("The locale's region and formatting.")
                controlData: Text {
                    text: root.view.formatsLocale !== undefined
                        ? String(root.view.formatsLocale) : "—"
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Calendar")
                controlData: Select {
                    id: calendarSelect
                    objectName: "languageCalendarSelect"
                    model: root.calendarModel
                    currentIndex: root.indexOf(root.calendarModel, root.calendar)
                    accessibleName: qsTr("Calendar")
                    onActivated: (index) => Settings.set(
                        "locale.calendar", root.calendarModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Temperature")
                controlData: Select {
                    id: temperatureSelect
                    objectName: "languageTemperatureSelect"
                    model: root.temperatureModel
                    currentIndex: root.indexOf(root.temperatureModel, root.temperature)
                    accessibleName: qsTr("Temperature")
                    onActivated: (index) => Settings.set(
                        "locale.temperature", root.temperatureModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Measurement")
                controlData: Select {
                    id: measurementSelect
                    objectName: "languageMeasurementSelect"
                    model: root.measurementModel
                    currentIndex: root.indexOf(root.measurementModel, root.measurement)
                    accessibleName: qsTr("Measurement")
                    onActivated: (index) => Settings.set(
                        "locale.measurement", root.measurementModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Date format")
                controlData: Select {
                    id: dateFormatSelect
                    objectName: "languageDateFormatSelect"
                    model: root.dateFormatModel
                    currentIndex: root.indexOf(root.dateFormatModel, root.dateFormat)
                    accessibleName: qsTr("Date format")
                    onActivated: (index) => Settings.set(
                        "locale.dateFormat", root.dateFormatModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Number format")
                controlData: Select {
                    id: numberFormatSelect
                    objectName: "languageNumberFormatSelect"
                    model: root.numberFormatModel
                    currentIndex: root.indexOf(root.numberFormatModel, root.numberFormat)
                    accessibleName: qsTr("Number format")
                    onActivated: (index) => Settings.set(
                        "locale.numberFormat", root.numberFormatModel[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("First day of week")
                showSeparator: false
                controlData: Select {
                    id: firstDaySelect
                    objectName: "languageFirstDaySelect"
                    model: root.firstDayModel
                    currentIndex: root.indexOf(root.firstDayModel, root.firstDay)
                    accessibleName: qsTr("First day of week")
                    onActivated: (index) => Settings.set(
                        "locale.firstDay", root.firstDayModel[index].value)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Preview")
            visible: root.available

            Text {
                id: previewText
                objectName: "languagePreview"
                width: parent.width
                text: qsTr("%1 · %2 · %3")
                    .arg(root.previewDate())
                    .arg(root.previewNumber())
                    .arg(root.previewTemperature())
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        SettingsGroup {
            width: parent.width
            visible: !root.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "languageAbsenceNote"
                kind: "disabled-with-reason"
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Language & Region is unavailable because localectl is not running.")
            }
        }
    }
}
