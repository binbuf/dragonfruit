// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Displays pane (T-09.5, completed T273). Every control is a stock
// design-system component bound to the `Settings` singleton (never D-Bus
// directly) with the T-09.1b write-on-interaction / bind-to-`Settings.values`
// pattern, so a user edit and an external settingsd change converge without a
// restart.
//
// The selection is forwarded by the shell to the compositor's private output
// API: `display.scale` becomes `df_output.set_scale` (the reference pane's
// scaled-resolution tiles, `Larger Text`…`More Space`), and `display.rotation`
// becomes `df_output.set_transform`. The compositor reconfigures the output
// live. The pane lists the real platform outputs (`QGuiApplication::screens()`)
// for its preview, per-output selector, and refresh value; it never invents a
// "Built-in Display" name.
//
// The capability table (`Settings.displayAdvancedRows()`) is the one source
// for every row this pane does not implement. Supported rows (brightness,
// rotation) are live; every other row is rendered disabled with the exact
// compositor/colord/sensor half that is OPEN. The advanced sheet shows only
// the advanced-only rows, so it no longer duplicates the main pane's
// brightness and rotation controls.
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
    readonly property real outputAspect: {
        if (!root.currentOutput)
            return 16 / 10;
        var width = Number(root.currentOutput.width || 0);
        var height = Number(root.currentOutput.height || 0);
        return height > 0 ? Math.max(0.5, Math.min(4, width / height)) : 16 / 10;
    }
    readonly property real outputRefresh: root.currentOutput
        ? Number(root.currentOutput.refreshRate || 0) : 0
    readonly property bool hasOutput: root.outputs.length > 0

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

    // The scaled-resolution tiles, matching the reference pane's
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
        { value: "default", label: qsTr("Default profile") }
    ]

    // Every instantiated resolution tile, in declaration order, for tests.
    property var tileItems: []

    // Test surface (used by tst_settings_displays.qml).
    property alias displayGroup: displayGroup
    property alias displayListGroup: displayListGroup
    property alias outputRepeater: outputRepeater
    property alias resolutionGroup: resolutionGroup
    property alias rotationGroup: rotationGroup
    property alias preview: preview
    property alias previewLabel: previewLabel
    property alias previewScreen: previewScreen
    property alias tileRow: tileRow
    property alias footer: footer
    property alias rotationSelect: rotationSelect
    property alias brightnessSlider: brightnessSlider
    property alias brightnessGroup: brightnessGroup
    property alias useAsGroup: useAsGroup
    property alias useAsRow: useAsRow
    property alias useAsSelect: useAsSelect
    property alias autoBrightnessToggle: autoBrightnessToggle
    property alias refreshGroup: refreshGroup
    property alias refreshText: refreshText
    property alias arrangementGroup: arrangementGroup
    property alias arrangementButton: arrangementButton
    property alias nightLightGroup: nightLightGroup
    property alias nightLightToggle: nightLightToggle
    property alias nightLightScheduleSelect: nightLightScheduleSelect
    property alias nightLightTemperatureSlider: nightLightTemperatureSlider
    property alias colorProfileGroup: colorProfileGroup
    property alias colorProfileSelect: colorProfileSelect
    property alias tvModeSelect: tvModeSelect
    property alias advancedButton: advancedButton
    property alias advancedPopup: advancedPopup
    property alias advancedList: advancedList
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

    function scaleIndex() {
        for (var i = 0; i < root.resolutionOptions.length; ++i) {
            if (Math.abs(root.resolutionOptions[i].value - root.scaleValue) < 0.001)
                return i;
        }
        return -1;
    }

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

        SettingsGroup {
            id: displayGroup
            width: parent.width
            title: root.outputName

            // The reference pane's display illustration; our own artwork. The
            // screen is proportional to the real output's aspect ratio.
            Rectangle {
                id: preview
                width: parent.width
                height: 176
                radius: Theme.controls.settingsGroup.radius
                color: Theme.color.surfaceMuted
                clip: true

                readonly property real screenWidth: {
                    var maxWidth = Math.max(48, Math.min(preview.width - 112, 200));
                    var maxHeight = Math.max(40, preview.height - 64);
                    return Math.min(maxWidth, maxHeight * root.outputAspect);
                }

                Rectangle {
                    id: previewScreen
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: Theme.primitive.spacing.md
                    width: preview.screenWidth
                    height: Math.max(40, width / root.outputAspect)
                    radius: Theme.controls.button.radius
                    color: Theme.color.surface
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border
                    visible: root.hasOutput

                    // A tiny content wash so an absent output is obvious.
                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 6
                        radius: Theme.controls.button.radius
                        color: Theme.color.controlFill
                    }
                }

                // The stand is part of the illustration only when an output is
                // present; an absent preview stays a plain empty frame.
                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: previewScreen.bottom
                    anchors.topMargin: 2
                    width: previewScreen.width * 0.36
                    height: 4
                    radius: 2
                    color: Theme.color.border
                    visible: root.hasOutput
                }

                Text {
                    id: previewLabel
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.margins: Theme.primitive.spacing.md
                    horizontalAlignment: Text.AlignHCenter
                    text: root.outputName
                    color: Theme.color.textPrimary
                    elide: Text.ElideRight
                    font.pixelSize: Theme.controls.button.fontSize
                    font.weight: Theme.primitive.font.weightMedium
                }

                Accessible.role: Accessible.Graphic
                Accessible.name: root.hasOutput
                    ? qsTr("%1 preview").arg(root.outputName)
                    : qsTr("No display")
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "displaysAbsenceNote"
                visible: !root.hasOutput
                kind: "disabled-with-reason"
                reason: qsTr("The compositor reports no output. Display "
                             + "preferences stay live and apply when one appears.")
            }
        }

        // The per-output selector. One output is the normal laptop/desktop
        // case; the group appears only when the platform reports more.
        SettingsGroup {
            id: displayListGroup
            width: parent.width
            title: qsTr("Displays")
            visible: root.outputs.length > 1

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
            id: brightnessGroup
            width: parent.width
            title: qsTr("Brightness")

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Brightness")
                settingsId: "brightness"
                owner: "settingsd -> compositor"
                settingsKey: "display.brightness"
                description: qsTr("The compositor owns the output level and reports "
                                  + "the applied value back.")
                showSeparator: true

                controlData: Slider {
                    id: brightnessSlider
                    width: 180
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
                width: parent.width
                label: qsTr("Adaptive color")
                settingsId: "adaptiveColor"
                owner: "compositor/sensor"
                settingsKey: "display.adaptiveColor"
                description: root.capabilityReason("adaptiveColor")
                showSeparator: false
                controlData: Toggle {
                    text: ""
                    checked: false
                    enabled: root.capabilitySupported("adaptiveColor")
                }
            }
        }

        SettingsGroup {
            id: resolutionGroup
            width: parent.width
            title: qsTr("Resolution")

            Row {
                id: tileRow
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                Repeater {
                    model: root.resolutionOptions
                    delegate: ResolutionTile { }
                }
            }

            Text {
                id: footer
                width: parent.width
                text: qsTr("Using a scaled resolution may affect performance. "
                          + "The compositor applies this selection to every "
                          + "output; per-output overrides are OPEN.")
                color: Theme.color.textSecondary
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.primitive.font.sizeSm
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
            id: refreshGroup
            width: parent.width
            title: qsTr("Refresh Rate")

            SettingsRow {
                width: parent.width
                label: qsTr("Current refresh rate")
                settingsId: "refresh.current"
                owner: "compositor/output"
                settingsKey: "display.refreshRate"
                showSeparator: true
                controlData: Text {
                    id: refreshText
                    objectName: "displaysRefreshText"
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.outputRefresh > 0
                        ? qsTr("%1 Hz").arg(Math.round(root.outputRefresh))
                        : qsTr("Not reported")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Refresh rate")
                settingsId: "refreshRate"
                owner: "compositor/output"
                settingsKey: "display.refreshRate"
                description: root.capabilityReason("refreshRate")
                showSeparator: false
                controlData: Select {
                    enabled: root.capabilitySupported("refreshRate")
                    accessibleName: qsTr("Refresh rate")
                    model: root.refreshOptions
                }
            }
        }

        SettingsGroup {
            id: arrangementGroup
            width: parent.width
            title: qsTr("Arrangement")

            // A read-only arrangement preview of the real outputs, proportional
            // to their logical geometry. Selecting a card is the per-output
            // selector; dragging/arranging is the OPEN half below.
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
            id: nightLightGroup
            width: parent.width
            title: qsTr("Night Light")

            SettingsRow {
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
                    width: 180
                    from: 1000
                    to: 10000
                    value: 4500
                    enabled: root.capabilitySupported("nightLightTemperature")
                    accessibleName: qsTr("Night light temperature")
                }
            }
        }

        SettingsGroup {
            id: colorProfileGroup
            width: parent.width
            title: qsTr("Color Profile")

            SettingsRow {
                width: parent.width
                label: qsTr("Color profile")
                settingsId: "colorProfile"
                owner: "colord"
                settingsKey: "display.colorProfile"
                description: root.capabilityReason("colorProfile")
                showSeparator: true
                controlData: Select {
                    id: colorProfileSelect
                    enabled: root.capabilitySupported("colorProfile")
                    accessibleName: qsTr("Color profile")
                    model: root.colorProfileOptions
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("When connected to TV")
                settingsId: "tvMode"
                owner: "compositor/output"
                settingsKey: "display.tvMode"
                description: root.capabilityReason("tvMode")
                showSeparator: false
                controlData: Select {
                    id: tvModeSelect
                    enabled: root.capabilitySupported("tvMode")
                    accessibleName: qsTr("When connected to TV")
                    model: [{ value: "auto", label: qsTr("Automatic") }]
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Advanced")

            SettingsRow {
                width: parent.width
                label: qsTr("Advanced display settings")
                settingsId: "advanced"
                owner: "compositor/colord"
                settingsKey: "display.advanced"
                description: qsTr("Resolution list, HDR, VRR, and overscan. "
                                  + "Unsupported rows name the OPEN "
                                  + "compositor/colord half; brightness and "
                                  + "rotation stay in their own groups above.")
                showSeparator: false

                controlData: Button {
                    id: advancedButton
                    text: qsTr("Advanced\u2026")
                    accessibleName: qsTr("Open advanced display settings")
                    onClicked: advancedPopup.show()
                }
            }
        }
    }

    // The advanced sheet. It shows only the advanced-only capability rows;
    // brightness and rotation are not duplicated here.
    Popup {
        id: advancedPopup
        anchorItem: advancedButton
        anchorOffset: -advancedPopup.height - 8
        preferredWidth: 340
        accessibleName: qsTr("Advanced display settings")

        Text {
            width: parent.width
            text: qsTr("Advanced display settings")
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            font.weight: Theme.primitive.font.weightSemibold
        }

        Column {
            id: advancedList
            width: parent.width
            spacing: Theme.primitive.spacing.xs

            Repeater {
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

        Text {
            width: parent.width
            text: qsTr("A disabled row is not advertised as a capability.")
            color: Theme.color.textSecondary
            wrapMode: Text.WordWrap
            font.pixelSize: Theme.primitive.font.sizeSm
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
        target: brightnessSlider
        property: "value"
        value: root.brightnessValue
    }

    // One scaled-resolution choice. A tile is a radio choice: selecting it
    // writes `display.scale`; the shell forwards it to the compositor.
    component ResolutionTile: Item {
        id: tile

        required property var modelData
        required property int index

        readonly property bool selected:
            Math.abs(tile.modelData.value - root.scaleValue) < 0.001

        width: (tileRow.width - 4 * tileRow.spacing) / 5
        height: glyph.height + caption.implicitHeight + Theme.primitive.spacing.xs
        activeFocusOnTab: true

        function choose() {
            root.chooseScale(tile.modelData.value);
        }

        Component.onCompleted: root.tileItems = root.tileItems.concat([tile])
        Component.onDestruction: root.tileItems = root.tileItems.filter(function(item) {
            return item !== tile;
        })

        Rectangle {
            id: glyph
            width: parent.width
            height: 56
            radius: Theme.controls.button.radius
            color: tile.selected ? Theme.color.accentMuted : Theme.color.surfaceMuted
            border.width: tile.selected ? Theme.controls.focusRing.width
                                        : Theme.controls.window.borderWidth
            border.color: tile.selected ? Theme.color.accent : Theme.color.border
            antialiasing: true

            // A tiny "Aa" preview: the same glyph at the scale the tile picks.
            Text {
                anchors.centerIn: parent
                text: qsTr("Aa")
                color: tile.selected ? Theme.color.accent : Theme.color.textSecondary
                font.pixelSize: Math.round(11 + 5 * tile.modelData.value)
                font.weight: Theme.primitive.font.weightMedium
            }
        }

        Text {
            id: caption
            anchors.top: glyph.bottom
            anchors.topMargin: Theme.primitive.spacing.xs
            width: parent.width
            text: tile.modelData.label
            color: Theme.color.textPrimary
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        TapHandler {
            onTapped: tile.choose()
        }

        FocusRing {
            target: tile
            cornerRadius: Theme.controls.button.radius
            shown: tile.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                tile.choose();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: tile.modelData.label
        Accessible.checkable: true
        Accessible.checked: tile.selected
        Accessible.focusable: true
        Accessible.onPressAction: tile.choose()
    }
}
