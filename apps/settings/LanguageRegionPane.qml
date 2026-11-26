// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Language & Region sub-pane (T226; T270 completion).
//
// The system locale is the host's (`localectl`); the calendar, temperature,
// measurement, date-format, number-format, and first-day rows are durable
// settingsd keys with a live preview. The pane is opened in place from General.
//
// T270: the raw locale `Select` becomes an ordered preferred-language list with
// add/remove/reorder, Region is selectable, and the preview formats through the
// selected locale. The preferred order is the `locale.languages` settingsd key;
// its first entry is the locale the host is asked to apply. The format rows and
// preview stay live when `localectl` is absent.
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

    // The durable preferred order, seeded from the host locale until the user
    // changes it. The seed is never written on open (T268).
    readonly property var preferred: Settings.values["locale.languages"] !== undefined
        && Settings.values["locale.languages"] !== null
        ? Settings.values["locale.languages"] : []
    readonly property var effectiveLanguages: root.preferred.length > 0
        ? root.preferred : (root.locale.length > 0 ? [root.locale] : [])
    readonly property string primaryLocale: root.effectiveLanguages.length > 0
        ? String(root.effectiveLanguages[0]) : root.locale
    readonly property var addableLocales: {
        var out = [];
        for (var i = 0; i < root.locales.length; ++i) {
            if (root.effectiveLanguages.indexOf(root.locales[i]) < 0)
                out.push(root.locales[i]);
        }
        return out;
    }
    readonly property var addModel: root.addableLocales.map(function(id) {
        return { label: root.languageLabel(id), value: id };
    })
    readonly property var regionModel: root.locales.map(function(id) {
        return { label: root.languageLabel(id), value: id };
    })
    readonly property int currentRegionIndex: Math.max(0, root.locales.indexOf(root.primaryLocale))
    readonly property string languageError: Settings.generalWriteOperation === "locale"
        && (Settings.generalWriteState === "denied"
            || Settings.generalWriteState === "failed")
        ? Settings.generalWriteError : ""

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

    // A locale id without its codeset, ready for `Qt.locale`.
    function localeId(id) {
        return String(id).split(".")[0];
    }

    function languageLabel(id) {
        const value = root.localeId(id);
        if (value.length === 0)
            return "";
        const locale = Qt.locale(value);
        const language = String(locale.nativeLanguageName || "");
        const territory = String(locale.nativeTerritoryName || "");
        if (language.length === 0)
            return String(id);
        return territory.length > 0 ? language + " (" + territory + ")" : language;
    }

    readonly property string calendar: root.valueOf("locale.calendar", "gregorian")
    readonly property string temperature: root.valueOf("locale.temperature", "celsius")
    readonly property string measurement: root.valueOf("locale.measurement", "metric")
    readonly property string dateFormat: root.valueOf("locale.dateFormat", "medium")
    readonly property string numberFormat: root.valueOf("locale.numberFormat", "locale")
    readonly property string firstDay: root.valueOf("locale.firstDay", "locale")
    readonly property var previewLocale: Qt.locale(
        root.localeId(root.primaryLocale.length > 0 ? root.primaryLocale : "en_US"))

    property alias languageRepeater: languageRepeater
    property alias addLanguagePicker: addLanguagePicker
    property alias regionPicker: regionPicker
    property alias calendarSelect: calendarSelect
    property alias temperatureSelect: temperatureSelect
    property alias measurementSelect: measurementSelect
    property alias dateFormatSelect: dateFormatSelect
    property alias numberFormatSelect: numberFormatSelect
    property alias firstDaySelect: firstDaySelect
    property alias previewText: previewText
    property alias absenceNote: absenceNote
    property alias languageGroup: languageGroup
    property alias regionGroup: regionGroup
    property alias previewGroup: previewGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    // Persist the order. A changed first entry is also the host locale the
    // desktop applies; an empty list leaves the host locale alone.
    function setPreferred(list) {
        const previous = root.primaryLocale;
        Settings.setList("locale.languages", list);
        if (list.length > 0 && String(list[0]) !== previous)
            Settings.setLocale(String(list[0]));
    }

    function addLanguage(id) {
        if (id.length === 0 || root.effectiveLanguages.indexOf(id) >= 0)
            return;
        var next = root.effectiveLanguages.slice();
        next.push(id);
        root.setPreferred(next);
    }

    function removeLanguage(index) {
        if (index < 0 || index >= root.effectiveLanguages.length)
            return;
        var next = root.effectiveLanguages.slice();
        next.splice(index, 1);
        root.setPreferred(next);
    }

    function moveLanguage(index, delta) {
        var target = index + delta;
        if (index < 0 || index >= root.effectiveLanguages.length
                || target < 0 || target >= root.effectiveLanguages.length)
            return;
        var next = root.effectiveLanguages.slice();
        var moved = next.splice(index, 1)[0];
        next.splice(target, 0, moved);
        root.setPreferred(next);
    }

    function previewDateFor(style) {
        const sample = new Date(2026, 10, 7, 14, 5, 0);
        const locale = root.previewLocale;
        if (style === "short")
            return sample.toLocaleDateString(locale, Locale.ShortFormat);
        if (style === "long")
            return sample.toLocaleDateString(locale, Locale.LongFormat);
        // Medium: the locale's month name and order, without the weekday.
        // Qt exposes short and long only, so strip the long form's weekday
        // instead of inventing an English medium pattern.
        const long = sample.toLocaleDateString(locale, Locale.LongFormat);
        const weekday = locale.dayName(sample.getDay(), Locale.LongFormat);
        return long.replace(weekday + ", ", "")
                   .replace(weekday + " ", "")
                   .replace(weekday, "").trim();
    }

    function previewDate() {
        return root.previewDateFor(root.dateFormat);
    }

    function previewNumber() {
        if (root.numberFormat === "comma")
            return (1234.5).toLocaleString(Qt.locale("de_DE"));
        if (root.numberFormat === "decimal")
            return (1234.5).toLocaleString(Qt.locale("en_US"));
        return (1234.5).toLocaleString(root.previewLocale);
    }

    function previewTemperature() {
        return root.temperature === "fahrenheit" ? "68 °F" : "20 °C";
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsIntro {
            width: parent.width
            text: qsTr("Choose the system language and the formats the desktop uses.")
        }

        SettingsGroup {
            id: languageGroup
            width: parent.width
            title: qsTr("Preferred Languages")

            Repeater {
                id: languageRepeater
                model: root.effectiveLanguages
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    settingsId: index === 0 ? "language.languages" : ""
                    label: root.languageLabel(modelData)
                    description: String(modelData)
                        + (index === 0 ? " · " + qsTr("Primary") : "")
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xxs

                        Button {
                            objectName: "languageUp" + index
                            text: "↑" // df-allow-untranslated: arrow glyph
                            variant: "ghost"
                            enabled: root.available && index > 0
                            accessibleName: qsTr("Move %1 up").arg(
                                root.languageLabel(modelData))
                            onClicked: root.moveLanguage(index, -1)
                        }

                        Button {
                            objectName: "languageDown" + index
                            text: "↓" // df-allow-untranslated: arrow glyph
                            variant: "ghost"
                            enabled: root.available && index < root.effectiveLanguages.length - 1
                            accessibleName: qsTr("Move %1 down").arg(
                                root.languageLabel(modelData))
                            onClicked: root.moveLanguage(index, 1)
                        }

                        Button {
                            objectName: "languageRemove" + index
                            text: qsTr("Remove")
                            variant: "ghost"
                            enabled: root.available
                            accessibleName: qsTr("Remove %1").arg(
                                root.languageLabel(modelData))
                            onClicked: root.removeLanguage(index)
                        }
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Add language…")
                description: root.available
                    ? qsTr("Adds the language to the preferred order.")
                    : qsTr("Unavailable")
                controlData: SearchablePicker {
                    id: addLanguagePicker
                    objectName: "languageAddPicker"
                    model: root.addModel
                    currentIndex: -1
                    placeholderLabel: qsTr("Add…")
                    placeholderText: qsTr("Search languages")
                    emptyText: qsTr("No language matches.")
                    enabled: root.available && root.addModel.length > 0
                    accessibleName: qsTr("Add language")
                    onActivated: (index) => {
                        if (index >= 0 && index < root.addModel.length)
                            root.addLanguage(String(root.addModel[index].value));
                    }
                }
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "languageAbsenceNote"
                kind: "disabled-with-reason"
                show: !root.available
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("The system locale is unavailable because localectl is not running. The format rows and preview still apply.")
            }
        }

        SettingsGroup {
            id: regionGroup
            width: parent.width
            title: qsTr("Region")

            SettingsRow {
                width: parent.width
                settingsId: "language.region"
                label: qsTr("Region")
                description: qsTr("The locale whose region and formatting the desktop uses.")
                errorText: root.languageError
                controlData: SearchablePicker {
                    id: regionPicker
                    objectName: "languageRegionPicker"
                    model: root.regionModel
                    currentIndex: root.currentRegionIndex
                    enabled: root.available
                    accessibleName: qsTr("Region")
                    placeholderText: qsTr("Search regions")
                    emptyText: qsTr("No region matches.")
                    onActivated: (index) => {
                        if (index >= 0 && index < root.locales.length) {
                            Settings.setLocale(String(root.locales[index]));
                            root.setPreferred([String(root.locales[index])]
                                .concat(root.effectiveLanguages.filter(function(id) {
                                    return id !== root.locales[index];
                                })));
                        }
                    }
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "language.calendar"
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
                settingsId: "language.temperature"
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
                settingsId: "language.measurement"
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
                settingsId: "language.dateFormat"
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
                settingsId: "language.numberFormat"
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
                settingsId: "language.firstDay"
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
            id: previewGroup
            width: parent.width
            title: qsTr("Preview")

            SettingsRow {
                width: parent.width
                label: qsTr("Date")
                controlData: Text {
                    text: root.previewDate()
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Time")
                controlData: Text {
                    text: new Date(2026, 10, 7, 9, 41, 0).toLocaleTimeString(
                        root.previewLocale, Locale.ShortFormat)
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Currency")
                controlData: Text {
                    text: (1234.56).toLocaleCurrencyString(root.previewLocale)
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Number")
                controlData: Text {
                    text: root.previewNumber()
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            // One combined line keeps the long-string and temperature checks
            // readable in a single measured row.
            Text {
                id: previewText
                objectName: "languagePreview"
                width: parent.width
                text: qsTr("%1 · %2 · %3")
                    .arg(root.previewDate())
                    .arg(root.previewNumber())
                    .arg(root.previewTemperature())
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }
}
