// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Accessibility -> Vision (T291a).
//
// The Vision sub-pane is the first section of the mockup's Accessibility
// list. It keeps only rows with a real Linux owner: the read-only AT-SPI
// screen-reader/bridge status, the compositor magnifier, the settingsd display
// accommodations, the motion keys, and the speech-dispatcher delegate. Every
// Apple-only row is omitted in `SettingsMap` with a reason instead of being
// rendered as an inert control.
//
// A row is one of the honest shapes from ADR 0276: a settingsd control, a
// read-only status, a delegate action, a stored preference (marked in the
// map), or an explicit omission. The screen reader is status plus `Open Orca`,
// never a switch: `org.a11y.Status` has no setter (ADR 0144).
Item {
    id: root

    // --- Live AT-SPI status (read-only) ---------------------------------
    readonly property var view: Settings.accessibility
    readonly property bool available: Settings.accessibilityAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property bool enabledOn: root.ready && root.view.enabled === true
    readonly property bool screenReaderOn: root.ready && root.view.screenReader === true
    readonly property string bridgeLabel: root.ready && root.view.enabledLabel !== undefined
        ? String(root.view.enabledLabel) : qsTr("Unavailable")
    readonly property string screenReaderLabel:
        root.ready && root.view.screenReaderLabel !== undefined
            ? String(root.view.screenReaderLabel) : qsTr("Unavailable")

    // --- Durable preferences (settingsd) --------------------------------
    readonly property bool reduceMotion:
        Settings.values["accessibility.reduceMotion"] === true
    readonly property bool reduceTransparency:
        Settings.values["accessibility.reduceTransparency"] === true
    readonly property bool increaseContrast:
        Settings.values["accessibility.increaseContrast"] === true
    readonly property bool differentiateWithoutColor:
        Settings.values["accessibility.differentiateWithoutColor"] === true
    readonly property string textSize:
        Settings.values["accessibility.textSize"] || "default"
    readonly property int textSizeIndex:
        root.textSize === "small" ? 0 : root.textSize === "large" ? 2
      : root.textSize === "extra-large" ? 3 : 1
    readonly property bool dimFlashingLights:
        Settings.values["accessibility.dimFlashingLights"] === true
    readonly property bool magnifier:
        Settings.values["accessibility.magnifier"] === true
    readonly property real magnifierZoom: {
        var value = Number(Settings.values["accessibility.magnifierZoom"]);
        return isFinite(value) ? value : 2.0;
    }
    readonly property string magnifierMode:
        Settings.values["accessibility.magnifierMode"] || "follow-focus"
    readonly property int magnifierModeIndex:
        root.magnifierMode === "follow-caret" ? 1 : 0
    readonly property int pointerSize: {
        var value = Number(Settings.values["appearance.cursorSize"]);
        return isFinite(value) ? Math.round(value) : 24;
    }

    // --- External delegates --------------------------------------------
    readonly property bool screenReaderAvailable: Settings.accessibilityScreenReaderAvailable
    readonly property bool speechAvailable: Settings.accessibilitySpeechAvailable

    // Test/capture surface.
    property alias screenReaderGroup: screenReaderGroup
    property alias visionGroup: screenReaderGroup
    property alias zoomGroup: zoomGroup
    property alias magnifierGroup: zoomGroup
    property alias displayGroup: displayGroup
    property alias contrastGroup: displayGroup
    property alias motionGroup: motionGroup
    property alias readSpeakGroup: readSpeakGroup
    property alias absenceNote: absenceNote
    property alias bridgeRow: bridgeRow
    property alias screenReaderRow: screenReaderRow
    property alias reduceMotionToggle: reduceMotionToggle
    property alias reduceTransparencyToggle: reduceTransparencyToggle
    property alias increaseContrastToggle: increaseContrastToggle
    property alias differentiateWithoutColorToggle: differentiateWithoutColorToggle
    property alias textSizeSelect: textSizeSelect
    property alias magnifierToggle: magnifierToggle
    property alias magnifierZoomSlider: magnifierZoomSlider
    property alias magnifierModeSelect: magnifierModeSelect
    property alias dimFlashingLightsToggle: dimFlashingLightsToggle
    property alias readSpeakRow: readSpeakRow
    property alias pointerSizeSlider: pointerSizeSlider

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Screen Reader ───────────────────────────────────────────────
        SettingsGroup {
            id: screenReaderGroup
            width: parent.width
            sectionId: "screen-reader"
            title: qsTr("Screen Reader")

            SettingsRow {
                id: screenReaderRow
                objectName: "accessibilityScreenReaderRow"
                width: parent.width
                settingsId: "vision.screenReader"
                owner: "Orca (AT-SPI)"
                settingsKey: "org.a11y.Status.ScreenReaderEnabled"
                label: qsTr("Screen reader")
                description: root.ready
                    ? qsTr("Reported by the assistive technology over AT-SPI. "
                           + "Open Orca to change its settings.")
                    : qsTr("The accessibility service (AT-SPI) is not running, "
                           + "so the live screen-reader status is unavailable.")
                trailingAction: root.screenReaderAvailable
                    ? qsTr("Open Orca") : qsTr("Orca not installed")
                trailingActionEnabled: root.screenReaderAvailable
                onTrailingActionClicked: Settings.openAccessibilityScreenReader()
                controlData: Text {
                    objectName: "accessibilityScreenReaderValue"
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.screenReaderLabel
                    color: root.screenReaderOn ? Theme.color.accent
                                               : Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                id: bridgeRow
                objectName: "accessibilityBridgeRow"
                width: parent.width
                settingsId: "vision.bridge"
                owner: "AT-SPI"
                settingsKey: "org.a11y.Status.IsEnabled"
                label: qsTr("Accessibility bridge")
                description: qsTr("The toolkit accessibility bridge that "
                                  + "screen readers and other assistive tools use.")
                showSeparator: false
                controlData: Text {
                    objectName: "accessibilityBridgeValue"
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.bridgeLabel
                    color: root.enabledOn ? Theme.color.accent
                                          : Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            Text {
                id: absenceNote
                objectName: "accessibilityAbsenceNote"
                width: parent.width
                visible: !root.ready
                text: qsTr("The accessibility service (AT-SPI) is not running, so "
                           + "the live screen reader and bridge status are "
                           + "unavailable. The preferences below still apply.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Zoom ────────────────────────────────────────────────────────
        SettingsGroup {
            id: zoomGroup
            width: parent.width
            sectionId: "zoom"
            title: qsTr("Zoom")

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.magnifier"
                owner: "compositor/magnifier"
                settingsKey: "accessibility.magnifier"
                label: qsTr("Use keyboard shortcuts to zoom")
                description: qsTr("The compositor magnifies the whole output and "
                                  + "maps pointer input back to the content.")
                controlData: Toggle {
                    id: magnifierToggle
                    objectName: "accessibilityMagnifierToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.magnifier", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.magnifier.zoom"
                owner: "compositor/magnifier"
                settingsKey: "accessibility.magnifierZoom"
                label: qsTr("Zoom level")
                description: qsTr("Applied live by the compositor magnifier.")
                enabled: root.magnifier
                controlData: Slider {
                    id: magnifierZoomSlider
                    objectName: "accessibilityMagnifierZoomSlider"
                    from: 1.0
                    to: 8.0
                    stepSize: 0.25
                    minLabel: qsTr("1×")
                    maxLabel: qsTr("8×")
                    onCommitted: (value) =>
                        Settings.set("accessibility.magnifierZoom", value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.magnifier.mode"
                owner: "compositor/magnifier"
                settingsKey: "accessibility.magnifierMode"
                label: qsTr("Follow")
                description: qsTr("Follow the focused window, or its text caret "
                                  + "when the compositor has a caret source.")
                showSeparator: false
                enabled: root.magnifier
                controlData: Select {
                    id: magnifierModeSelect
                    objectName: "accessibilityMagnifierModeSelect"
                    model: [qsTr("Focused window"), qsTr("Text caret")]
                    onActivated: (index) => Settings.set(
                        "accessibility.magnifierMode",
                        index === 1 ? "follow-caret" : "follow-focus")
                }
            }
        }

        // ── Display ─────────────────────────────────────────────────────
        SettingsGroup {
            id: displayGroup
            width: parent.width
            sectionId: "display"
            title: qsTr("Display")

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.display.increaseContrast"
                owner: "shell/design-system Glass"
                settingsKey: "accessibility.increaseContrast"
                label: qsTr("Increase contrast")
                description: qsTr("Strengthen glass edges and foreground contrast. "
                                  + "Applies with Reduce Transparency as well.")
                controlData: Toggle {
                    id: increaseContrastToggle
                    objectName: "accessibilityIncreaseContrastToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.increaseContrast", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.display.reduceTransparency"
                owner: "shell/design-system Glass"
                settingsKey: "accessibility.reduceTransparency"
                label: qsTr("Reduce transparency")
                description: qsTr("Replace glass on the Dock, menus, panels, and "
                                  + "window chrome with an opaque plate. Overrides "
                                  + "the Appearance transparency slider and works "
                                  + "with the appearance rollout off.")
                controlData: Toggle {
                    id: reduceTransparencyToggle
                    objectName: "accessibilityReduceTransparencyToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.reduceTransparency", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.display.differentiateWithoutColor"
                owner: "shell/design-system Theme/Toggle"
                settingsKey: "accessibility.differentiateWithoutColor"
                label: qsTr("Differentiate without color")
                description: qsTr("Add a non-colour cue to controls. The checked "
                                  + "toggle carries a check glyph as well as the "
                                  + "accent fill.")
                controlData: Toggle {
                    id: differentiateWithoutColorToggle
                    objectName: "accessibilityDifferentiateWithoutColorToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.differentiateWithoutColor", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.display.textSize"
                owner: "shell/design-system Theme"
                settingsKey: "accessibility.textSize"
                label: qsTr("Text size")
                description: qsTr("Scale interface text through the one "
                                  + "design-system type scale.")
                controlData: Select {
                    id: textSizeSelect
                    objectName: "accessibilityTextSizeSelect"
                    model: [qsTr("Small"), qsTr("Default"), qsTr("Large"),
                            qsTr("Extra Large")]
                    onActivated: (index) => {
                        var ids = ["small", "default", "large", "extra-large"];
                        Settings.set("accessibility.textSize",
                                     ids[Math.max(0, Math.min(3, index))]);
                    }
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.display.pointerSize"
                owner: "settingsd + compositor cursor"
                settingsKey: "appearance.cursorSize"
                label: qsTr("Pointer size")
                description: qsTr("Applied by the compositor at the next session "
                                  + "start. The Appearance pane owns the same key.")
                showSeparator: false
                controlData: Slider {
                    id: pointerSizeSlider
                    objectName: "accessibilityPointerSizeSlider"
                    from: 16
                    to: 64
                    stepSize: 2
                    minLabel: qsTr("Small")
                    maxLabel: qsTr("Large")
                    onCommitted: (value) =>
                        Settings.set("appearance.cursorSize", Math.round(value))
                }
            }
        }

        // ── Motion ──────────────────────────────────────────────────────
        SettingsGroup {
            id: motionGroup
            width: parent.width
            sectionId: "motion"
            title: qsTr("Motion")

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.motion.reduceMotion"
                owner: "shell/design-system Theme, compositor/window motion"
                settingsKey: "accessibility.reduceMotion"
                label: qsTr("Reduce motion")
                description: qsTr("Collapse interface animations to reduce motion.")
                controlData: Toggle {
                    id: reduceMotionToggle
                    objectName: "accessibilityReduceMotionToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.reduceMotion", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                settingsId: "vision.motion.dimFlashingLights"
                owner: "shell/Dock attention bounce"
                settingsKey: "accessibility.dimFlashingLights"
                label: qsTr("Dim flashing lights")
                description: qsTr("Keep attention cues but reduce their amplitude "
                                  + "so a flash does not dominate the scene.")
                showSeparator: false
                controlData: Toggle {
                    id: dimFlashingLightsToggle
                    objectName: "accessibilityDimFlashingLightsToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.dimFlashingLights", checked)
                }
            }
        }

        // ── Read & Speak ────────────────────────────────────────────────
        SettingsGroup {
            id: readSpeakGroup
            width: parent.width
            sectionId: "read-speak"
            title: qsTr("Read & Speak")

            SettingsRow {
                id: readSpeakRow
                objectName: "accessibilityReadSpeakRow"
                width: parent.width
                settingsId: "vision.readSpeak"
                owner: "speech-dispatcher/espeak-ng"
                settingsKey: "spd-say"
                label: qsTr("Read & speak")
                description: root.speechAvailable
                    ? qsTr("Delegated to the session speech engine. The sample "
                           + "proves the delegate is wired.")
                    : qsTr("Not available: neither spd-say nor espeak-ng is "
                           + "installed.")
                showSeparator: false
                trailingAction: root.speechAvailable
                    ? qsTr("Speak Sample") : qsTr("Unavailable")
                trailingActionEnabled: root.speechAvailable
                onTrailingActionClicked:
                    Settings.speakAccessibilityText(qsTr("Accessibility is ready."))
            }
        }
    }

    // Keep the controls in step with external changes.
    Binding { target: reduceMotionToggle; property: "checked"; value: root.reduceMotion }
    Binding { target: reduceTransparencyToggle; property: "checked"; value: root.reduceTransparency }
    Binding { target: increaseContrastToggle; property: "checked"; value: root.increaseContrast }
    Binding { target: differentiateWithoutColorToggle; property: "checked"
              value: root.differentiateWithoutColor }
    Binding { target: magnifierToggle; property: "checked"; value: root.magnifier }
    Binding { target: dimFlashingLightsToggle; property: "checked"
              value: root.dimFlashingLights }
    Binding { target: pointerSizeSlider; property: "value"; value: root.pointerSize }
    Binding { target: magnifierZoomSlider; property: "value"; value: root.magnifierZoom }
    Binding { target: textSizeSelect; property: "currentIndex"
              value: root.textSizeIndex }
    Binding { target: magnifierModeSelect; property: "currentIndex"
              value: root.magnifierModeIndex }
}
