// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Appearance pane (T-09.2). Every control is a stock design-system
// component bound to the `Settings` singleton (never D-Bus directly) using the
// write-on-interaction / bind-to-`Settings.values` pattern from T-09.1b, so a
// user edit and an external settingsd change converge without a restart.
//
// Wave-1 scope: `appearance.colorScheme` (Light/Dark/Auto) and
// `appearance.accent` (our own swatch row plus a custom hex picker). The
// reference pane's Highlight color, Sidebar icon size, wallpaper tinting, and
// scroll-bar rows have no settings provider yet (T-15.x), so they are omitted
// rather than shipped half-working — the no-half-panes rule. Reduced motion is
// documented as staying the Accessibility item (see the task hand-off).
Item {
    id: root

    readonly property var schemeOptions: [
        { value: "light", label: qsTr("Light") },
        { value: "dark", label: qsTr("Dark") },
        { value: "auto", label: qsTr("Auto") }
    ]

    // The accent swatch set is ours to choose (docs/reference/System_Preferences
    // section "Adapt or omit"). Empty value = the active scheme's token accent;
    // the rest mirror the design-system primitive palette. The custom picker
    // accepts any `#rrggbb`.
    readonly property var accentPalette: [
        { value: "", label: qsTr("Default") },
        { value: "#7c5cff", label: qsTr("Violet") },
        { value: "#4a7dff", label: qsTr("Blue") },
        { value: "#2bb673", label: qsTr("Green") },
        { value: "#e0a43a", label: qsTr("Gold") },
        { value: "#e8556d", label: qsTr("Coral") }
    ]

    readonly property string currentAccent: Settings.values["appearance.accent"] || ""

    property string customAccentText: root.currentAccent

    // T196: the attached-menu transparency preference. It is a bounded double
    // (schema default 0.5255) and only the light attached-menu preset follows
    // it; the other glass roles keep their named calibration. `preview*` is
    // the same resolver input the rendered menus use, updated immediately
    // while a drag is in flight.
    readonly property real transparency: {
        var value = Number(Settings.values["appearance.transparency"]);
        return isFinite(value) ? Math.max(0, Math.min(1, value))
                               : Theme.glass.slider.defaultPosition;
    }
    readonly property bool reduceTransparency:
        Settings.values["accessibility.reduceTransparency"] === true
    readonly property bool increaseContrast:
        Settings.values["accessibility.increaseContrast"] === true
    property real previewTransparency: root.transparency
    // Tests/captures can preview the replacement look without changing the
    // process-wide rollout; the desktop still follows `Glass.rollout`.
    property string previewRollout: Glass.rollout
    property real pendingTransparency: root.transparency
    property bool transparencyWritePending: false

    // One resolver, shared with the shell and the rendered menus (T195).
    readonly property var previewResolved: Glass.resolve("menuAttached", {
        scheme: Glass.scheme,
        rollout: root.previewRollout,
        slider: root.previewTransparency,
        reduceTransparency: root.reduceTransparency,
        increaseContrast: root.increaseContrast,
        tier: Glass.tier,
        software: Glass.software,
        sampledBlur: Glass.sampledBlur
    })
    readonly property string previewMode: root.previewResolved.renderMode
    readonly property string previewPercent:
        Math.round(root.previewTransparency * 100) + "%"

    // Coalesce live drag writes: the knob and preview move on every `moved`,
    // the settings owner sees at most one write per timer window, and
    // `committed` always flushes the final value.
    function queueTransparency(value) {
        var next = Math.round(Math.max(0, Math.min(1, value)) * 10000) / 10000;
        root.previewTransparency = next;
        root.pendingTransparency = next;
        root.transparencyWritePending = true;
        transparencyWriteTimer.restart();
    }

    function flushTransparency() {
        if (!root.transparencyWritePending)
            return;
        transparencyWriteTimer.stop();
        root.transparencyWritePending = false;
        Settings.set("appearance.transparency", root.pendingTransparency);
    }

    onTransparencyChanged: {
        if (!root.transparencyWritePending)
            root.previewTransparency = root.transparency;
    }

    // Test surface (used by tst_settings_appearance.qml).
    property alias schemeControl: schemeControl
    property alias schemeRow: schemeRow
    property alias accentRepeater: accentRepeater
    property alias accentRow: accentRow
    property alias accentPopup: accentPopup
    property alias accentInput: accentInput
    property alias transparencySlider: transparencySlider
    property alias transparencyRow: transparencyRow
    property alias transparencyGroup: transparencyGroup
    property alias previewPlate: previewPlate
    property alias previewGroup: previewGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot: the shell's Loader is the content width, so
    // the card tracks it instead of keeping the 480 design width and clipping
    // its right-hand controls. A standalone instance keeps the implicit width.
    width: parent ? parent.width : implicitWidth

    // Index of the active scheme in the reference order; unknown/auto falls to
    // the Auto segment.
    function schemeIndex() {
        var current = Settings.values["appearance.colorScheme"] || "auto";
        for (var i = 0; i < root.schemeOptions.length; ++i) {
            if (root.schemeOptions[i].value === current)
                return i;
        }
        return root.schemeOptions.length - 1;
    }

    function isHexColor(text) {
        return /^#[0-9a-fA-F]{6}$/.test((text || "").trim());
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            title: qsTr("Appearance")
            sectionId: "Appearance"

            SettingsKeyRow {
                id: schemeRow
                width: parent.width
                label: qsTr("Appearance")
                settingsId: "scheme"
                owner: "settingsd + compositor"
                settingsKey: "appearance.colorScheme"
                showSeparator: false

                controlData: SegmentedControl {
                    id: schemeControl
                    model: root.schemeOptions.map(function(option) { return option.label; })
                    onActivated: (index) => Settings.set("appearance.colorScheme",
                                                         root.schemeOptions[index].value)
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Accent color")

            SettingsKeyRow {
                id: accentRow
                width: parent.width
                label: qsTr("Accent color")
                settingsId: "accent"
                owner: "settingsd + compositor"
                settingsKey: "appearance.accent"
                showSeparator: false

                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Repeater {
                        id: accentRepeater
                        model: root.accentPalette
                        delegate: AccentSwatch { }
                    }

                    Button {
                        id: customButton
                        text: qsTr("Custom…")
                        accessibleName: qsTr("Custom accent color")
                        onClicked: accentPopup.open ? accentPopup.hide() : accentPopup.show()
                    }
                }
            }
        }

        // T196: the one appearance preference that drives the glass optical
        // preset. The row names every surface it does (and does not) affect so
        // the slider is not mistaken for a global window-opacity control.
        SettingsGroup {
            id: transparencyGroup
            width: parent.width
            title: qsTr("Transparency")

            SettingsKeyRow {
                id: transparencyRow
                width: parent.width
                label: qsTr("Attached-menu transparency")
                settingsId: "transparency"
                owner: "settingsd + shell"
                settingsKey: "appearance.transparency"
                description: qsTr("Light attached menus only; higher values are clearer. "
                                  + "The Dock, menu bar, windows, and panels keep their own "
                                  + "materials. Reduce Transparency overrides this slider.")
                showSeparator: false

                controlData: Slider {
                    id: transparencySlider
                    width: 220
                    from: 0.0
                    to: 1.0
                    minLabel: qsTr("Opaque")
                    maxLabel: qsTr("Clear")
                    accessibleName: qsTr("Attached-menu transparency")
                    onMoved: (value) => root.queueTransparency(value)
                    onCommitted: (value) => {
                        root.pendingTransparency =
                            Math.round(Math.max(0, Math.min(1, value)) * 10000) / 10000;
                        root.transparencyWritePending = true;
                        root.flushTransparency();
                    }
                }
            }
        }

        // The same effective resolver the rendered menus consume. The
        // immediate value/mode readout gives feedback even while the rollout
        // still selects the legacy look.
        SettingsGroup {
            id: previewGroup
            width: parent.width
            title: qsTr("Preview")

            Item {
                width: parent.width
                height: 118

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.controls.settingsGroup.radius
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: "#2f6fde" }
                        GradientStop { position: 0.5; color: "#d65a8a" }
                        GradientStop { position: 1.0; color: "#e8b34a" }
                    }
                }

                Rectangle {
                    id: previewPlate
                    anchors.fill: parent
                    radius: Theme.controls.settingsGroup.radius
                    color: Qt.rgba(root.previewResolved.tint,
                                   root.previewResolved.tint,
                                   root.previewResolved.tint,
                                   root.previewResolved.alpha)
                    border.width: Math.max(Theme.controls.window.borderWidth,
                                           Math.round(root.previewResolved.edgeWidth))
                    border.color: Qt.rgba(1, 1, 1, root.previewResolved.edge)
                    antialiasing: true

                    Column {
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            text: qsTr("Attached menu")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            font.weight: Theme.primitive.font.weightMedium
                        }
                        Text {
                            objectName: "appearancePreviewMode"
                            text: root.previewMode + " · " + root.previewPercent
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                        }
                    }
                }
            }
        }

        Item {
            width: parent.width
            height: 0
        }
    }

    Timer {
        id: transparencyWriteTimer
        interval: 60
        repeat: false
        onTriggered: root.flushTransparency()
    }

    // Two-way binding for the scheme selector: the control writes on activation,
    // and this keeps it in step with `Settings.values` (a user edit and an
    // external settingsd change converge without a restart).
    Binding {
        target: schemeControl
        property: "currentIndex"
        value: root.schemeIndex()
    }

    // External settingsd changes converge into the knob, but an in-flight drag
    // (or a coalesced write) owns the value until it flushes.
    Binding {
        target: transparencySlider
        property: "value"
        value: root.transparency
        when: !transparencySlider.dragging && !root.transparencyWritePending
    }

    // A hex-entry color picker: our own, so it consumes the design-system
    // surface instead of pulling in a platform dialog. Live preview, applies on
    // Enter or the Apply button.
    Popup {
        id: accentPopup
        anchorItem: customButton
        preferredWidth: 220
        accessibleName: qsTr("Custom accent color")

        onOpened: {
            root.customAccentText = root.currentAccent;
            accentInput.text = root.customAccentText;
        }

        Text {
            width: parent.width
            text: qsTr("Hex color")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            font.weight: Theme.primitive.font.weightMedium
        }

        Row {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Rectangle {
                width: 24
                height: 24
                radius: width / 2
                anchors.verticalCenter: parent.verticalCenter
                color: accentInput.acceptable ? accentInput.text : Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border
                antialiasing: true
            }

            Rectangle {
                width: parent.width - 24 - parent.spacing
                height: Theme.controls.button.height
                anchors.verticalCenter: parent.verticalCenter
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: accentInput.acceptable ? Theme.color.border : Theme.color.danger

                TextInput {
                    id: accentInput
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    text: root.customAccentText
                    onTextEdited: root.customAccentText = text
                    onAccepted: accentInput.applyCustom()
                    readonly property bool acceptable: root.isHexColor(text)

                    function applyCustom() {
                        if (accentInput.acceptable) {
                            Settings.set("appearance.accent",
                                         accentInput.text.trim().toLowerCase());
                            accentPopup.hide();
                        }
                    }
                }
            }
        }

        Text {
            width: parent.width
            visible: accentInput.text.length > 0 && !accentInput.acceptable
            text: qsTr("Enter a color like #4a7dff.")
            color: Theme.color.danger
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        Item {
            width: parent.width
            height: Theme.primitive.spacing.sm
        }

        Button {
            text: qsTr("Apply")
            variant: "primary"
            enabled: accentInput.acceptable
            onClicked: accentInput.applyCustom()
        }
    }

    // One accent choice. Selected state follows `Settings.values`, so an
    // external change repaints the row too.
    component AccentSwatch: Item {
        id: swatch

        required property var modelData

        readonly property bool selected: root.currentAccent === swatch.modelData.value

        width: 20
        height: 20
        activeFocusOnTab: true

        function choose() {
            Settings.set("appearance.accent", swatch.modelData.value);
        }

        Rectangle {
            anchors.centerIn: parent
            width: 16
            height: 16
            radius: width / 2
            color: swatch.modelData.value === "" ? Theme.color.accent : swatch.modelData.value
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: "transparent"
            border.width: Theme.controls.focusRing.width
            border.color: Theme.color.accent
            visible: swatch.selected
            antialiasing: true
        }

        HoverHandler { id: swatchHover }

        TapHandler {
            onTapped: swatch.choose()
        }

        FocusRing {
            target: swatch
            cornerRadius: width / 2
            shown: swatch.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                swatch.choose();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: swatch.modelData.label
        Accessible.checkable: true
        Accessible.checked: swatch.selected
        Accessible.focusable: true
        Accessible.onPressAction: swatch.choose()
    }
}