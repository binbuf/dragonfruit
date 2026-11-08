// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Displays pane (T-09.5). Every control is a stock design-system
// component bound to the `Settings` singleton (never D-Bus directly) with the
// T-09.1b write-on-interaction / bind-to-`Settings.values` pattern, so a user
// edit and an external settingsd change converge without a restart.
//
// The selection is forwarded by the shell to the compositor's private output
// API: `display.scale` becomes `df_output.set_scale` (the reference pane's
// scaled-resolution tiles, `Larger Text`…`More Space`), and `display.rotation`
// becomes `df_output.set_transform`. The compositor reconfigures the output
// live. Wave 1 has one logical display and no mode enumeration; per-display
// targeting and true mode lists are T-16 items. The reference pane's
// brightness, True Tone, Preset, Refresh rate, Night Shift and Advanced rows
// have no provider yet and are omitted (the no-half-panes rule), as is the
// `Arrange…` button (multi-display, T-16).
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
    // T227: the one advanced capability table. Supported rows write the real
    // display keys; unsupported rows render disabled with their OPEN reason.
    readonly property var advancedRows: Settings.displayAdvancedRows()

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

    // Every instantiated resolution tile, in declaration order, for tests.
    property var tileItems: []
    // The shell sets this to its root item so the advanced popup is not
    // clipped by the detail ScrollView. Standalone tests keep the pane.
    property Item popupParent: null

    // Test surface (used by tst_settings_displays.qml).
    property alias displayGroup: displayGroup
    property alias resolutionGroup: resolutionGroup
    property alias rotationGroup: rotationGroup
    property alias preview: preview
    property alias previewLabel: previewLabel
    property alias tileRow: tileRow
    property alias footer: footer
    property alias rotationSelect: rotationSelect
    property alias brightnessSlider: brightnessSlider
    property alias brightnessGroup: brightnessGroup
    property alias advancedButton: advancedButton
    property alias advancedPopup: advancedPopup
    property alias advancedList: advancedList

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

    function chooseScale(value) {
        Settings.set("display.scale", value);
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            id: displayGroup
            width: parent.width
            title: qsTr("Built-in Display")

            // The reference pane's display illustration; our own artwork.
            Rectangle {
                id: preview
                width: parent.width
                height: 160
                radius: Theme.controls.settingsGroup.radius
                color: Theme.color.surfaceMuted
                clip: true

                Column {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: Theme.primitive.spacing.md
                    spacing: Theme.primitive.spacing.sm

                    Rectangle {
                        id: screen
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: Math.min(preview.width - 96, 150)
                        height: width * 0.625
                        radius: Theme.controls.button.radius
                        color: Theme.color.surface
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border
                    }

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: screen.width * 0.36
                        height: 4
                        radius: 2
                        color: Theme.color.border
                    }
                }

                Text {
                    id: previewLabel
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.margins: Theme.primitive.spacing.md
                    horizontalAlignment: Text.AlignHCenter
                    text: qsTr("Built-in Display")
                    color: Theme.color.textPrimary
                    elide: Text.ElideRight
                    font.pixelSize: Theme.controls.button.fontSize
                    font.weight: Theme.primitive.font.weightMedium
                }

                Accessible.role: Accessible.Graphic
                Accessible.name: qsTr("Built-in Display")
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
                showSeparator: false

                controlData: Slider {
                    id: brightnessSlider
                    width: 220
                    from: 0.0
                    to: 1.0
                    minLabel: qsTr("Dim")
                    maxLabel: qsTr("Bright")
                    accessibleName: qsTr("Display brightness")
                    onMoved: (value) => Settings.set("display.brightness", value)
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
                text: qsTr("Using a scaled resolution may affect performance.")
                color: Theme.color.textSecondary
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.primitive.font.sizeSm
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
                description: qsTr("Rotation, refresh, arrangement, night light, "
                                  + "color, HDR, VRR, and brightness. Unsupported "
                                  + "rows name the OPEN compositor/colord half.")
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

    // The advanced sheet. Supported rows are live controls; unsupported rows
    // are disabled and carry the reason from the capability table.
    Popup {
        id: advancedPopup
        parent: root.popupParent ? root.popupParent : root
        anchorItem: advancedButton
        // Open above the button: the advanced row is at the bottom of the
        // pane, so a downward popup would fall past the window edge.
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

                    controlData: Loader {
                        sourceComponent: modelData.id === "brightness"
                                         ? advancedBrightness
                                       : modelData.id === "rotation"
                                         ? advancedRotation : advancedUnavailable
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

    Component {
        id: advancedBrightness
        Slider {
            width: 160
            from: 0.0
            to: 1.0
            accessibleName: qsTr("Display brightness")
            value: root.brightnessValue
            onMoved: (value) => Settings.set("display.brightness", value)
        }
    }

    Component {
        id: advancedRotation
        Select {
            accessibleName: qsTr("Display rotation")
            model: root.rotationOptions
            currentIndex: root.rotationIndex()
            onActivated: (index) => Settings.set(
                            "display.rotation",
                            root.rotationOptions[index].value)
        }
    }

    Component {
        id: advancedUnavailable
        Text {
            text: qsTr("Unavailable")
            color: Theme.color.textTertiary
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