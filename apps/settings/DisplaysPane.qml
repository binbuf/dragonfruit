// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Displays pane (T-09.5, completed T273; mockup order T289). Every control
// is a stock design-system component bound to the `Settings` singleton (never
// D-Bus directly) with the T-09.1b write-on-interaction / bind-to-
// `Settings.values` pattern, so a user edit and an external settingsd change
// converge without a restart.
//
// The selection is forwarded by the shell to the compositor's private output
// API: `display.scale` becomes `df_output.set_scale` and `display.rotation`
// becomes `df_output.set_transform`. The compositor reconfigures the output
// live. The pane lists the real platform outputs (`QGuiApplication::screens()`)
// for its display row, per-output sheet, and refresh value; it never invents a
// "Built-in Display" name.
//
// T289 follows the mockup (docs/design/25-system-settings-visual-parity.md
// §5.3): the display icon row, Resolution radios, the scaling note, Brightness,
// automatic brightness, Ambient color, Preset, Refresh rate, the
// "When connected to a projector" section, and the Advanced…/Night Light…
// footers. Arrangement, Rotation, Color Profile, Use as, and the advanced
// capability rows live in the Advanced… sheet; Night Light's rows live in the
// Night Light… sheet. "Preset" is the existing color-profile path and
// "Ambient color" is adaptive color; neither is a new key.
//
// The capability table (`Settings.displayAdvancedRows()`) is the one source
// for every row this pane does not implement. Supported rows (brightness,
// rotation) are live; every other row is rendered disabled with the exact
// compositor/colord/sensor half that is OPEN.
Item {
    id: root

    readonly property real scaleValue: {
        var v = Settings.values["display.scale"];
        return v === undefined ? 1.0 : Number(v);
    }
    readonly property string rotationValue:
        Settings.values["display.rotation"] || "normal"
    readonly property real brightnessValue: {
        var v = Settings.values["display.brightness"];
        return v === undefined ? 1.0 : Number(v);
    }

    // The real platform outputs, from the Settings bridge.
    readonly property var outputs:
        Settings.displayOutputsRevision >= 0 ? Settings.displayOutputs() : []
    property string selectedOutputId: ""
    readonly property var currentOutput: {
        if (root.outputs.length === 0)
            return null;
        for (var i = 0; i < root.outputs.length; ++i) {
            if (String(root.outputs[i].id) === root.selectedOutputId)
                return root.outputs[i];
        }
        return root.outputs[0];
    }
    readonly property string outputName: root.currentOutput
        ? String(root.currentOutput.name) : qsTr("No display")
    readonly property real outputRefresh: root.currentOutput
        ? Number(root.currentOutput.refreshRate || 0) : 0
    readonly property bool hasOutput: root.outputs.length > 0

    // T289: the host's TV/projector signal. The bridge marks the output from
    // the EDID-derived name/model; the pane never guesses from a size.
    readonly property var projectorOutput: {
        for (var i = 0; i < root.outputs.length; ++i) {
            if (root.outputs[i].projector === true)
                return root.outputs[i];
        }
        return null;
    }
    readonly property bool projectorPresent: root.projectorOutput !== null

    // T227/T273: the one capability table. `advancedRows` is the sheet's
    // subset; the main groups read `capability(id)` for their disabled rows.
    readonly property var capabilityRows: Settings.displayAdvancedRows()
    readonly property var advancedRows: {
        var out = [];
        for (var i = 0; i < root.capabilityRows.length; ++i) {
            if (String(root.capabilityRows[i].area) === "advanced")
                out.push(root.capabilityRows[i]);
        }
        return out;
    }

    function capability(id) {
        for (var i = 0; i < root.capabilityRows.length; ++i) {
            if (String(root.capabilityRows[i].id) === id)
                return root.capabilityRows[i];
        }
        return null;
    }
    function capabilityReason(id) {
        var row = root.capability(id);
        return row ? String(row.reason || "") : "";
    }
    function capabilitySupported(id) {
        var row = root.capability(id);
        return row ? row.supported === true : false;
    }

    // The scaled-resolution choices, matching the reference pane's
    // `Larger Text`…`Default`…`More Space` list. A larger scale is a larger
    // user interface (fewer logical pixels); a smaller scale is more space.
    readonly property var resolutionOptions: [
        { value: 1.5, label: qsTr("Larger Text") },
        { value: 1.25, label: qsTr("Large") },
        { value: 1.0, label: qsTr("Default") },
        { value: 0.875, label: qsTr("More Space") },
        { value: 0.75, label: qsTr("Most Space") }
    ]
    readonly property var rotationOptions: [
        { value: "normal", label: qsTr("Standard") },
        { value: "90", label: qsTr("90°") },
        { value: "180", label: qsTr("180°") },
        { value: "270", label: qsTr("270°") }
    ]
    readonly property var useAsOptions: [
        { value: "main", label: qsTr("Main display") },
        { value: "extend", label: qsTr("Extend display") },
        { value: "mirror", label: qsTr("Mirror display") }
    ]
    readonly property var refreshOptions: [
        { value: "current", label: qsTr("Current mode") }
    ]
    readonly property var colorProfileOptions: [
        { value: "default", label: qsTr("Standard") }
    ]

    // T289: the mockup's "Show all resolutions". Our scale model has five
    // steps; three are shown by default and the switch reveals the other two.
    // The filter is a view preference, not a new settingsd key.
    property bool showAllResolutions: false

    function scaleIndex() {
        for (var i = 0; i < root.resolutionOptions.length; ++i) {
            if (Math.abs(root.resolutionOptions[i].value - root.scaleValue) < 0.001)
                return i;
        }
        return -1;
    }
    function scaleLabel(value) {
        for (var i = 0; i < root.resolutionOptions.length; ++i) {
            if (Math.abs(root.resolutionOptions[i].value - value) < 0.001)
                return root.resolutionOptions[i].label;
        }
        return qsTr("Custom");
    }
    // The logical pixel size for one scale step: the output's real size
    // divided by the user-interface scale. No output means the label stays
    // the scale name, never a fabricated resolution.
    function resolutionLabel(value) {
        var name = root.scaleLabel(value);
        if (!root.currentOutput)
            return name;
        var width = Math.round(Number(root.currentOutput.width || 0) / value);
        var height = Math.round(Number(root.currentOutput.height || 0) / value);
        if (width <= 0 || height <= 0)
            return name;
        return qsTr("%1 × %2 (%3)").arg(width).arg(height).arg(name);
    }
    readonly property var resolutionRadioOptions: {
        var out = [];
        for (var i = 0; i < root.resolutionOptions.length; ++i) {
            out.push({ value: root.resolutionOptions[i].value,
                       label: root.resolutionLabel(root.resolutionOptions[i].value) });
        }
        return out;
    }
    readonly property var resolutionVisibleOptions: {
        var out = [];
        var selected = root.scaleIndex();
        for (var i = 0; i < root.resolutionRadioOptions.length; ++i) {
            if (root.showAllResolutions || (i >= 1 && i <= 3) || i === selected)
                out.push(root.resolutionRadioOptions[i]);
        }
        return out;
    }
    readonly property int resolutionCurrentIndex: {
        for (var i = 0; i < root.resolutionVisibleOptions.length; ++i) {
            if (Math.abs(root.resolutionVisibleOptions[i].value - root.scaleValue) < 0.001)
                return i;
        }
        return -1;
    }
    readonly property string outputModeLabel: {
        if (!root.currentOutput)
            return qsTr("No display");
        var mode = root.resolutionLabel(root.scaleValue);
        if (root.outputRefresh > 0)
            return qsTr("%1 · %2 Hz").arg(mode)
                .arg(Math.round(root.outputRefresh));
        return mode;
    }
    readonly property string refreshDescription: {
        var current = root.outputRefresh > 0
            ? qsTr("Currently %1 Hz.").arg(Math.round(root.outputRefresh))
            : qsTr("The current rate is not reported.");
        var reason = root.capabilityReason("refreshRate");
        return reason.length > 0 ? current + " " + reason : current;
    }

    // Test surface (used by tst_settings_displays.qml).
    property alias displayGroup: displayGroup
    property alias displayRow: displayRow
    property alias displayDetailsDialog: displayDetailsDialog
    property alias displayListGroup: displayListGroup
    property alias outputRepeater: outputRepeater
    property alias resolutionGroup: resolutionGroup
    property alias resolutionRadios: resolutionRadios
    property alias showAllResolutionsRow: showAllResolutionsRow
    property alias showAllResolutionsToggle: showAllResolutionsToggle
    property alias scalingNote: scalingNote
    property alias footer: scalingNote
    property alias brightnessGroup: brightnessGroup
    property alias brightnessSlider: brightnessSlider
    property alias autoBrightnessToggle: autoBrightnessToggle
    property alias ambientColorRow: ambientColorRow
    property alias ambientColorToggle: ambientColorToggle
    property alias colorPresetRow: colorPresetRow
    property alias colorProfileSelect: colorProfileSelect
    property alias refreshRow: refreshRow
    property alias refreshText: refreshRow.descriptionText
    property alias projectorGroup: projectorGroup
    property alias projectorModeRow: projectorModeRow
    property alias projectorAbsenceRow: projectorAbsenceRow
    property alias tvModeSelect: tvModeSelect
    property alias advancedButton: advancedButton
    property alias advancedPopup: advancedDialog
    property alias advancedList: advancedList
    property alias advancedRepeater: advancedRepeater
    property alias nightLightButton: nightLightButton
    property alias nightLightDialog: nightLightDialog
    property alias nightLightGroup: nightLightGroup
    property alias nightLightRow: nightLightRow
    property alias nightLightToggle: nightLightToggle
    property alias nightLightScheduleSelect: nightLightScheduleSelect
    property alias nightLightTemperatureSlider: nightLightTemperatureSlider
    property alias useAsGroup: useAsGroup
    property alias useAsRow: useAsRow
    property alias useAsSelect: useAsSelect
    property alias arrangementGroup: arrangementGroup
    property alias arrangementButton: arrangementButton
    property alias rotationGroup: rotationGroup
    property alias rotationSelect: rotationSelect
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

    function rotationIndex() {
        for (var i = 0; i < root.rotationOptions.length; ++i) {
            if (root.rotationOptions[i].value === root.rotationValue)
                return i;
        }
        return 0;
    }

    function outputIndex() {
        for (var i = 0; i < root.outputs.length; ++i) {
            if (String(root.outputs[i].id) === root.selectedOutputId)
                return i;
        }
        return 0;
    }

    function chooseScale(value) {
        Settings.set("display.scale", value);
    }

    function chooseOutput(id) {
        root.selectedOutputId = String(id);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ─━ Display row (mockup) ───────────────────────────────────────────
        SettingsGroup {
            id: displayGroup
            width: parent.width

            SettingsIconRow {
                id: displayRow
                objectName: "displaysRow"
                width: parent.width
                category: SettingsPanes.rowStyle("displays")
                label: root.outputName
                description: root.outputModeLabel
                settingsId: "display"
                owner: "platform outputs"
                settingsKey: "display.perOutput"
                disclosure: true
                showSeparator: false
                onDisclosureClicked: displayDetailsDialog.show()
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "displaysAbsenceNote"
                visible: !root.hasOutput
                kind: "disabled-with-reason"
                reason: qsTr("No display is connected. Display "
                             + "preferences stay live and apply when one appears.")
            }
        }

        // ─━ Resolution ─────────────────────────────────────────────────────
        SettingsGroup {
            id: resolutionGroup
            width: parent.width
            title: qsTr("Resolution")

            SettingsRadioGroup {
                id: resolutionRadios
                objectName: "displaysResolutionRadios"
                width: parent.width
                options: root.resolutionVisibleOptions
                accessibleName: qsTr("Display resolution")
                onActivated: (index) => root.chooseScale(
                    root.resolutionVisibleOptions[index].value)
            }

            SettingsRow {
                id: showAllResolutionsRow
                objectName: "displaysShowAllRow"
                width: parent.width
                label: qsTr("Show all resolutions")
                settingsId: "showAllResolutions"
                owner: "settingsd + compositor"
                settingsKey: "display.scale"
                description: qsTr("Reveal the full scale list. The filter is "
                                  + "not saved.")
                showSeparator: false
                controlData: Toggle {
                    id: showAllResolutionsToggle
                    objectName: "displaysShowAllToggle"
                    text: ""
                    onToggled: (checked) => root.showAllResolutions = checked
                }
            }
        }

        SettingsNote {
            id: scalingNote
            objectName: "displaysScalingNote"
            width: parent.width
            text: qsTr("Using a scaled resolution may affect performance.")
        }

        // ─━ Brightness ─────────────────────────────────────────────────────
        SettingsGroup {
            id: brightnessGroup
            width: parent.width
            title: qsTr("Brightness")

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Brightness")
                settingsId: "brightness"
                owner: "settingsd -> compositor"
                settingsKey: "display.brightness"
                description: qsTr("The brightness level applies to the display "
                                  + "and is reported back.")
                showSeparator: true

                controlData: Slider {
                    id: brightnessSlider
                    width: Theme.controls.slider.minWidth
                    from: 0.0
                    to: 1.0
                    minLabel: qsTr("Dim")
                    maxLabel: qsTr("Bright")
                    accessibleName: qsTr("Display brightness")
                    onCommitted: (value) => Settings.set("display.brightness", value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Automatically adjust brightness")
                settingsId: "autoBrightness"
                owner: "compositor/sensor"
                settingsKey: "display.autoBrightness"
                description: root.capabilityReason("autoBrightness")
                showSeparator: true
                controlData: Toggle {
                    id: autoBrightnessToggle
                    text: ""
                    checked: false
                    enabled: root.capabilitySupported("autoBrightness")
                }
            }

            SettingsRow {
                id: ambientColorRow
                objectName: "displaysAmbientColorRow"
                width: parent.width
                label: qsTr("Ambient color")
                settingsId: "adaptiveColor"
                owner: "compositor/sensor"
                settingsKey: "display.adaptiveColor"
                description: root.capabilityReason("adaptiveColor")
                showSeparator: true
                controlData: Toggle {
                    id: ambientColorToggle
                    text: ""
                    checked: false
                    enabled: root.capabilitySupported("adaptiveColor")
                }
            }

            SettingsRow {
                id: colorPresetRow
                objectName: "displaysColorPresetRow"
                width: parent.width
                label: qsTr("Preset")
                settingsId: "colorProfile"
                owner: "colord"
                settingsKey: "display.colorProfile"
                description: root.capabilityReason("colorProfile")
                showSeparator: true
                controlData: Select {
                    id: colorProfileSelect
                    enabled: root.capabilitySupported("colorProfile")
                    accessibleName: qsTr("Display preset")
                    model: root.colorProfileOptions
                }
            }

            SettingsRow {
                id: refreshRow
                objectName: "displaysRefreshRow"
                width: parent.width
                label: qsTr("Refresh rate")
                settingsId: "refreshRate"
                owner: "compositor/output"
                settingsKey: "display.refreshRate"
                description: root.refreshDescription
                showSeparator: false
                controlData: Select {
                    id: refreshSelect
                    enabled: root.capabilitySupported("refreshRate")
                    accessibleName: qsTr("Refresh rate")
                    model: root.refreshOptions
                }
            }
        }

        // ─━ When connected to a projector ────────────────────────────────
        SettingsGroup {
            id: projectorGroup
            objectName: "displaysProjectorGroup"
            width: parent.width
            title: qsTr("When connected to a projector")

            SettingsRow {
                id: projectorModeRow
                objectName: "displaysProjectorRow"
                width: parent.width
                visible: root.projectorPresent
                label: qsTr("Projector mode")
                settingsId: "tvMode"
                owner: "compositor/output"
                settingsKey: "display.tvMode"
                description: qsTr("Choose what to show or use the projector as "
                                  + "a secondary display. ")
                              + root.capabilityReason("tvMode")
                showSeparator: false
                controlData: Select {
                    id: tvModeSelect
                    enabled: root.capabilitySupported("tvMode")
                    accessibleName: qsTr("Projector mode")
                    model: [{ value: "ask", label: qsTr("Ask What to Show") },
                            { value: "mirror", label: qsTr("Mirror") },
                            { value: "extend", label: qsTr("Extend") }]
                }
            }

            SettingsRow {
                id: projectorAbsenceRow
                objectName: "displaysProjectorAbsenceRow"
                width: parent.width
                visible: !root.projectorPresent
                label: qsTr("Projector mode")
                settingsId: "tvMode"
                owner: "compositor/output"
                settingsKey: "display.tvMode"
                description: qsTr("No TV or projector is connected. Projector "
                                  + "options appear when one is detected.")
                showSeparator: false
                controlData: Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ─━ Footers (mockup) ──────────────────────────────────────────────
        SettingsFooter {
            id: footerActions
            objectName: "displaysFooter"
            width: parent.width

            Button {
                id: advancedButton
                objectName: "displaysAdvancedButton"
                text: qsTr("Advanced…")
                accessibleName: qsTr("Open advanced display settings")
                onClicked: advancedDialog.show()
            }

            Button {
                id: nightLightButton
                objectName: "displaysNightLightButton"
                text: qsTr("Night Light…")
                accessibleName: qsTr("Open Night Light settings")
                onClicked: nightLightDialog.show()
            }
        }
    }

    // The per-output sheet behind the display row's chevron. One output is the
    // normal laptop/desktop case; the list still names it so the row is never
    // a dead affordance.
    Dialog {
        id: displayDetailsDialog
        objectName: "displaysDetailsDialog"
        title: qsTr("Displays")
        message: root.hasOutput
            ? qsTr("Choose the display this pane scopes its mode and refresh "
                   + "readings to.")
            : qsTr("No display is connected.")

        contentData: SettingsGroup {
            id: displayListGroup
            width: parent.width
            visible: true

            Repeater {
                id: outputRepeater
                model: root.outputs
                delegate: SettingsRow {
                    id: outputRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: String(modelData.name)
                    description: qsTr("%1 × %2 · %3 Hz%4")
                        .arg(Math.round(Number(modelData.width || 0)))
                        .arg(Math.round(Number(modelData.height || 0)))
                        .arg(Math.round(Number(modelData.refreshRate || 0)))
                        .arg(modelData.primary === true ? qsTr(" · Main") : "")
                    readonly property bool selected: root.currentOutput
                        && String(root.currentOutput.id) === String(modelData.id)
                    showSeparator: index < root.outputs.length - 1
                    controlData: Text {
                        text: outputRow.selected ? qsTr("Selected") : qsTr("Select")
                        color: outputRow.selected ? Theme.color.accent
                                                  : Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                        Accessible.role: Accessible.RadioButton
                        Accessible.name: qsTr("Select %1").arg(modelData.name)
                        Accessible.checkable: true
                        Accessible.checked: outputRow.selected
                        Accessible.onPressAction: root.chooseOutput(modelData.id)
                        TapHandler {
                            onTapped: root.chooseOutput(modelData.id)
                        }
                    }
                }
            }

            SettingsRow {
                width: parent.width
                visible: !root.hasOutput
                label: qsTr("No display")
                description: qsTr("Display preferences stay live and apply when "
                                  + "one appears.")
                showSeparator: false
            }
        }
    }

    // The Advanced… sheet: Arrangement, Rotation, Color Profile, Use as, and
    // the advanced-only capability rows. Brightness is not duplicated here.
    Dialog {
        id: advancedDialog
        objectName: "displaysAdvancedDialog"
        title: qsTr("Advanced display settings")
        message: qsTr("Arrangement, rotation, color profile, and the rows the "
                      + "compositor does not advertise yet.")

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.lg

            SettingsGroup {
                id: arrangementGroup
                width: parent.width
                title: qsTr("Arrangement")

                // A read-only arrangement preview of the real outputs,
                // proportional to their logical geometry. Selecting a card is
                // the per-output selector; dragging is the OPEN half below.
                Row {
                    width: parent.width
                    spacing: Theme.primitive.spacing.sm

                    Repeater {
                        model: root.outputs
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            readonly property bool selected: root.currentOutput
                                && String(root.currentOutput.id) === String(modelData.id)
                            width: Math.max(64, Math.min(180,
                                Number(modelData.width || 0) / 16))
                            height: 64
                            radius: Theme.controls.button.radius
                            color: selected ? Theme.color.accentMuted
                                            : Theme.color.surfaceMuted
                            border.width: selected ? Theme.controls.focusRing.width
                                                   : Theme.controls.window.borderWidth
                            border.color: selected ? Theme.color.accent
                                                   : Theme.color.border
                            Text {
                                anchors.centerIn: parent
                                width: parent.width - 2 * Theme.primitive.spacing.sm
                                text: String(modelData.name)
                                color: Theme.color.textPrimary
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                            TapHandler {
                                onTapped: root.chooseOutput(modelData.id)
                            }
                            Accessible.role: Accessible.RadioButton
                            Accessible.name: String(modelData.name)
                            Accessible.checkable: true
                            Accessible.checked: selected
                            Accessible.onPressAction: root.chooseOutput(modelData.id)
                        }
                    }
                }

                SettingsRow {
                    width: parent.width
                    label: qsTr("Arrange displays")
                    settingsId: "arrangement"
                    owner: "compositor/output"
                    settingsKey: "display.arrangement"
                    description: root.capabilityReason("arrangement")
                    showSeparator: false
                    controlData: Button {
                        id: arrangementButton
                        text: qsTr("Arrange…")
                        enabled: root.capabilitySupported("arrangement")
                        accessibleName: qsTr("Arrange displays")
                    }
                }
            }

            SettingsGroup {
                id: rotationGroup
                width: parent.width
                title: qsTr("Rotation")

                SettingsRow {
                    width: parent.width
                    label: qsTr("Rotation")
                    settingsId: "rotation"
                    owner: "settingsd -> compositor"
                    settingsKey: "display.rotation"
                    showSeparator: false

                    controlData: Select {
                        id: rotationSelect
                        accessibleName: qsTr("Display rotation")
                        model: root.rotationOptions
                        onActivated: (index) => Settings.set(
                                        "display.rotation",
                                        root.rotationOptions[index].value)
                    }
                }
            }

            SettingsGroup {
                id: useAsGroup
                width: parent.width
                title: qsTr("Use as")

                SettingsRow {
                    id: useAsRow
                    width: parent.width
                    label: qsTr("Use as")
                    settingsId: "useAs"
                    owner: "compositor/output"
                    settingsKey: "display.useAs"
                    description: root.capabilityReason("useAs")
                    showSeparator: false
                    controlData: Select {
                        id: useAsSelect
                        enabled: root.capabilitySupported("useAs")
                        accessibleName: qsTr("Use this display as")
                        model: root.useAsOptions
                    }
                }
            }

            SettingsGroup {
                width: parent.width
                title: qsTr("Advanced")

                Column {
                    id: advancedList
                    width: parent.width
                    spacing: 0

                    Repeater {
                        id: advancedRepeater
                        model: root.advancedRows
                        delegate: SettingsRow {
                            required property var modelData
                            width: advancedList.width
                            label: modelData.label
                            settingsId: modelData.id
                            owner: "compositor/colord"
                            settingsKey: modelData.key
                            description: modelData.supported ? "" : modelData.reason
                            showSeparator: true

                            controlData: Text {
                                text: modelData.supported ? qsTr("Available")
                                                          : qsTr("Unavailable")
                                color: modelData.supported ? Theme.color.textSecondary
                                                           : Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                        }
                    }
                }
            }

            SettingsNote {
                width: parent.width
                text: qsTr("A disabled row is not advertised as a capability.")
            }
        }
    }

    // The Night Light… sheet: the real (capability-gated) night-light path.
    Dialog {
        id: nightLightDialog
        objectName: "displaysNightLightDialog"
        title: qsTr("Night Light")
        message: qsTr("Warm the display's color at night. This display's "
                      + "night-light pipeline is not present yet.")

        contentData: SettingsGroup {
            id: nightLightGroup
            width: parent.width

            SettingsRow {
                id: nightLightRow
                width: parent.width
                label: qsTr("Night light")
                settingsId: "nightLight"
                owner: "compositor/output"
                settingsKey: "display.nightLight"
                description: root.capabilityReason("nightLight")
                showSeparator: true
                controlData: Toggle {
                    id: nightLightToggle
                    text: ""
                    checked: false
                    enabled: root.capabilitySupported("nightLight")
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Schedule")
                settingsId: "nightLightSchedule"
                owner: "compositor/output"
                settingsKey: "display.nightLight.schedule"
                description: root.capabilityReason("nightLightSchedule")
                showSeparator: true
                controlData: Select {
                    id: nightLightScheduleSelect
                    enabled: root.capabilitySupported("nightLightSchedule")
                    accessibleName: qsTr("Night light schedule")
                    model: [{ value: "off", label: qsTr("Off") }]
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Temperature")
                settingsId: "nightLightTemperature"
                owner: "compositor/output"
                settingsKey: "display.nightLight.temperature"
                description: root.capabilityReason("nightLightTemperature")
                showSeparator: false
                controlData: Slider {
                    id: nightLightTemperatureSlider
                    width: Theme.controls.slider.minWidth
                    from: 1000
                    to: 10000
                    value: 4500
                    enabled: root.capabilitySupported("nightLightTemperature")
                    accessibleName: qsTr("Night light temperature")
                }
            }
        }
    }

    // Two-way bindings: the controls write on interaction; these keep them in
    // step with `Settings.values` (a user edit and an external settingsd change
    // converge).
    Binding {
        target: rotationSelect
        property: "currentIndex"
        value: root.rotationIndex()
    }
    Binding {
        target: resolutionRadios
        property: "currentIndex"
        value: root.resolutionCurrentIndex
    }
    Binding {
        target: brightnessSlider
        property: "value"
        value: root.brightnessValue
    }
    Binding {
        target: showAllResolutionsToggle
        property: "checked"
        value: root.showAllResolutions
    }
}
