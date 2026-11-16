// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Accessibility pane (T275).
//
// The macOS Tahoe information architecture is the familiarity contract:
// Vision, Hearing, Motor, Speech, and the Accessibility Shortcut. The Linux
// owner is the truth. A row is one of five honest shapes:
//
//   * a settingsd toggle/slider/popup with a live consumer (Theme, Glass,
//     the Dock, the compositor magnifier, or the shortcut engine);
//   * a read-only live status (the AT-SPI screen-reader/bridge flags);
//   * a delegate that launches the real external program (Orca, the speech
//     engine, the on-screen keyboard) and disables itself when absent;
//   * a stored preference whose consumer lands later (marked in the row);
//   * an explicit omission with the reason.
//
// The pane never invents a fake toggle. The screen reader is a status plus
// `Open Orca`, not a switch: `org.a11y.Status` has no setter (ADR 0144). The
// compositor magnifier and the recorded shortcut are the two live
// compositor-owned accommodations (v12 `set_accessibility_policy`).
//
// The pane stays structured when AT-SPI is absent: only the live status rows
// and the delegate actions that need a program are disabled, with reasons.
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
    readonly property string accessibilityShortcut:
        Settings.values["accessibility.shortcut"] || ""
    readonly property real balance: {
        var value = Number(Settings.values["sound.balance"]);
        return isFinite(value) ? value : 0.5;
    }
    readonly property int doubleClickSpeed: {
        var value = Number(Settings.values["input.doubleClickSpeed"]);
        return isFinite(value) ? Math.round(value) : 400;
    }
    readonly property int pointerSize: {
        var value = Number(Settings.values["appearance.cursorSize"]);
        return isFinite(value) ? Math.round(value) : 24;
    }

    // --- External delegates --------------------------------------------
    readonly property bool screenReaderAvailable: Settings.accessibilityScreenReaderAvailable
    readonly property bool speechAvailable: Settings.accessibilitySpeechAvailable
    readonly property bool onScreenKeyboardAvailable:
        Settings.accessibilityOnScreenKeyboardAvailable

    // The same effective resolver the shell/menus use, so the preview proves
    // the override combination rather than a second local formula.
    readonly property var previewResolved: Glass.resolve("menuAttached", {
        scheme: Glass.scheme,
        rollout: Glass.rollout,
        slider: Glass.slider,
        reduceTransparency: root.reduceTransparency,
        increaseContrast: root.increaseContrast,
        tier: Glass.tier,
        software: Glass.software,
        sampledBlur: Glass.sampledBlur
    })

    // Test/capture surface.
    property alias visionGroup: visionGroup
    property alias statusGroup: visionGroup
    property alias displayGroup: displayGroup
    property alias contrastGroup: displayGroup
    property alias magnifierGroup: magnifierGroup
    property alias motionGroup: motionGroup
    property alias readSpeakGroup: readSpeakGroup
    property alias hearingGroup: hearingGroup
    property alias motorGroup: motorGroup
    property alias speechGroup: speechGroup
    property alias shortcutGroup: shortcutGroup
    property alias previewGroup: previewGroup
    property alias previewPlate: previewPlate
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
    property alias balanceSlider: balanceSlider
    property alias doubleClickSpeedSlider: doubleClickSpeedSlider
    property alias pointerSizeSlider: pointerSizeSlider
    property alias liveSpeechRow: liveSpeechRow
    property alias shortcutRecorder: shortcutRecorder
    property alias onScreenKeyboardRow: onScreenKeyboardRow

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshAccessibility()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Vision ───────────────────────────────────────────────────────
        SettingsGroup {
            id: visionGroup
            width: parent.width
            sectionId: "vision"
            title: qsTr("Vision")

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
                controlData: Text {
                    objectName: "accessibilityBridgeValue"
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.bridgeLabel
                    color: root.enabledOn ? Theme.color.accent
                                          : Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.hoverText"
                owner: "shell overlay + settingsd"
                settingsKey: "shell/hover-text"
                label: qsTr("Hover text")
                description: qsTr("Not available: this build has no hover-text "
                                  + "overlay that can read the text under the pointer.")
                showSeparator: false
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
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

        // ── Display accommodations ──────────────────────────────────────
        SettingsGroup {
            id: displayGroup
            width: parent.width
            sectionId: "display"
            title: qsTr("Display")

            SettingsRow {
                width: parent.width
                settingsId: "vision.display.invertColors"
                owner: "compositor"
                settingsKey: "compositor/invert-colors"
                label: qsTr("Invert colors")
                description: qsTr("Not available: the compositor has no "
                                  + "whole-scene colour-inversion pass.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

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

            SettingsRow {
                width: parent.width
                settingsId: "vision.display.titleIcons"
                owner: "compositor/window decoration"
                settingsKey: "compositor/title-icons"
                label: qsTr("Show window title icons")
                description: qsTr("Not available: the compositor's window "
                                  + "decoration has no title-icon slot yet.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.display.toolbarShapes"
                owner: "design system"
                settingsKey: "design-system/toolbar-shapes"
                label: qsTr("Show toolbar button shapes")
                description: qsTr("Not available: the design system has no "
                                  + "shape-cue pass for toolbar buttons.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.display.displayContrast"
                owner: "compositor"
                settingsKey: "compositor/display-contrast"
                label: qsTr("Display contrast")
                description: qsTr("Not available: the compositor has no "
                                  + "display-contrast adjustment pass.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
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
                settingsId: "vision.display.menuBarSize"
                owner: "shell/MenuBar"
                settingsKey: "shell/menu-bar-size"
                label: qsTr("Menu bar size")
                description: qsTr("Not available: the compositor allocates a "
                                  + "fixed menu-bar height at session start.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
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

            SettingsRow {
                width: parent.width
                settingsId: "vision.display.pointerColors"
                owner: "compositor cursor"
                settingsKey: "compositor/cursor-colors"
                label: qsTr("Pointer colors")
                description: qsTr("Not available: the cursor theme has no "
                                  + "per-colour override in this build.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.display.colorFilters"
                owner: "compositor"
                settingsKey: "compositor/color-filters"
                label: qsTr("Color filters")
                description: qsTr("Not available: the compositor has no "
                                  + "colour-filter pass.")
                showSeparator: false
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Zoom / Magnifier ────────────────────────────────────────────
        SettingsGroup {
            id: magnifierGroup
            width: parent.width
            sectionId: "vision"
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

        // ── Motion ──────────────────────────────────────────────────────
        SettingsGroup {
            id: motionGroup
            width: parent.width
            sectionId: "vision"
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
                controlData: Toggle {
                    id: dimFlashingLightsToggle
                    objectName: "accessibilityDimFlashingLightsToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.dimFlashingLights", checked)
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.motion.autoPlayImages"
                owner: "shell"
                settingsKey: "shell/auto-play-images"
                label: qsTr("Auto-play animated images")
                description: qsTr("Not available: the shell has no animated-image "
                                  + "surface to gate in this build.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.motion.nonBlinkingCursor"
                owner: "design system"
                settingsKey: "design-system/non-blinking-cursor"
                label: qsTr("Prefer non-blinking cursor")
                description: qsTr("Not available: the text cursor blink is "
                                  + "toolkit-owned; no per-app override exists.")
                showSeparator: false
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Read & Speak ────────────────────────────────────────────────
        SettingsGroup {
            id: readSpeakGroup
            width: parent.width
            sectionId: "vision"
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
                trailingAction: root.speechAvailable
                    ? qsTr("Speak Sample") : qsTr("Unavailable")
                trailingActionEnabled: root.speechAvailable
                onTrailingActionClicked:
                    Settings.speakAccessibilityText(qsTr("Accessibility is ready."))
            }

            SettingsRow {
                width: parent.width
                settingsId: "vision.audioDescriptions"
                owner: "omit"
                settingsKey: "omit"
                label: qsTr("Audio descriptions")
                description: qsTr("Not available: audio descriptions are "
                                  + "content-level, not a shell preference.")
                showSeparator: false
                enabled: false
                controlData: Text {
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Hearing ─────────────────────────────────────────────────────
        SettingsGroup {
            id: hearingGroup
            width: parent.width
            sectionId: "hearing"
            title: qsTr("Hearing")

            SettingsRow {
                width: parent.width
                settingsId: "hearing.flashScreen"
                owner: "shell"
                settingsKey: "shell/flash-screen"
                label: qsTr("Flash the screen on alert")
                description: qsTr("Not available: the shell has no full-output "
                                  + "flash surface in this build.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "hearing.monoAudio"
                owner: "PipeWire"
                settingsKey: "PipeWire/mono-downmix"
                label: qsTr("Mono audio")
                description: qsTr("Not available: PipeWire has no per-session "
                                  + "mono downmix control in this build.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "hearing.balance"
                owner: "apps/settings"
                settingsKey: "sound.balance"
                label: qsTr("Balance")
                description: qsTr("Output balance from Left to Right. The Sound "
                                  + "pane owns the same key; the audio engine's "
                                  + "balance consumer is still deferred.")
                controlData: Slider {
                    id: balanceSlider
                    objectName: "accessibilityBalanceSlider"
                    from: 0.0
                    to: 1.0
                    stepSize: 0.05
                    minLabel: qsTr("Left")
                    maxLabel: qsTr("Right")
                    onCommitted: (value) => Settings.set("sound.balance", value)
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "hearing.backgroundSounds"
                owner: "audio delegate"
                settingsKey: "audio/background-sounds"
                label: qsTr("Background sounds")
                description: qsTr("Not available: no background-sound player is "
                                  + "installed.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "hearing.captions"
                owner: "live-caption delegate"
                settingsKey: "caption/delegate"
                label: qsTr("Captions")
                description: qsTr("Not available: no live-caption delegate is "
                                  + "installed.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "hearing.rtt"
                owner: "omit"
                settingsKey: "omit"
                label: qsTr("RTT")
                description: qsTr("Not available: the session has no real-time "
                                  + "text service.")
                enabled: false
                controlData: Text {
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "hearing.nameRecognition"
                owner: "omit"
                settingsKey: "omit"
                label: qsTr("Name recognition")
                description: qsTr("Not available: there is no on-device name model.")
                showSeparator: false
                enabled: false
                controlData: Text {
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Motor ───────────────────────────────────────────────────────
        SettingsGroup {
            id: motorGroup
            width: parent.width
            sectionId: "motor"
            title: qsTr("Motor")

            SettingsRow {
                width: parent.width
                settingsId: "motor.voiceControl"
                owner: "speech-to-text delegate"
                settingsKey: "speech-to-text/delegate"
                label: qsTr("Voice control")
                description: qsTr("Not available: no speech-to-text service is "
                                  + "installed.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "motor.keyboard.fullKeyboardAccess"
                owner: "compositor/input"
                settingsKey: "input/keyboard-navigation"
                label: qsTr("Full keyboard access")
                description: qsTr("Not available: this build has no global "
                                  + "full-keyboard mode; the Keyboard pane's "
                                  + "navigation key is the current surface.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "motor.keyboard.stickyKeys"
                owner: "compositor/input"
                settingsKey: "compositor/sticky-keys"
                label: qsTr("Sticky keys")
                description: qsTr("Not available: the compositor has no sticky-key "
                                  + "state machine.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "motor.keyboard.slowKeys"
                owner: "compositor/input"
                settingsKey: "compositor/slow-keys"
                label: qsTr("Slow keys")
                description: qsTr("Not available: the compositor has no slow-key "
                                  + "acceptance delay.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                id: onScreenKeyboardRow
                objectName: "accessibilityOnScreenKeyboardRow"
                width: parent.width
                settingsId: "motor.keyboard.onScreenKeyboard"
                owner: "on-screen keyboard delegate"
                settingsKey: "onboard"
                label: qsTr("On-screen keyboard")
                description: root.onScreenKeyboardAvailable
                    ? qsTr("Launch the session's on-screen keyboard.")
                    : qsTr("Not available: no on-screen keyboard program is "
                           + "installed.")
                trailingAction: root.onScreenKeyboardAvailable
                    ? qsTr("Open") : qsTr("Unavailable")
                trailingActionEnabled: root.onScreenKeyboardAvailable
                onTrailingActionClicked: Settings.openAccessibilityKeyboard()
            }

            SettingsKeyRow {
                width: parent.width
                settingsId: "motor.pointer.doubleClickSpeed"
                owner: "compositor/input pointer"
                settingsKey: "input.doubleClickSpeed"
                label: qsTr("Double-click speed")
                description: qsTr("Maximum time between two clicks for a "
                                  + "double-click. Applied live by the compositor.")
                controlData: Slider {
                    id: doubleClickSpeedSlider
                    objectName: "accessibilityDoubleClickSpeedSlider"
                    from: 100
                    to: 900
                    stepSize: 50
                    minLabel: qsTr("Fast")
                    maxLabel: qsTr("Slow")
                    onCommitted: (value) =>
                        Settings.set("input.doubleClickSpeed", Math.round(value))
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "motor.pointer.mouseKeys"
                owner: "compositor/input"
                settingsKey: "compositor/mouse-keys"
                label: qsTr("Mouse keys")
                description: qsTr("Not available: the compositor has no "
                                  + "keyboard-driven pointer mode.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "motor.pointer.ignoreTrackpad"
                owner: "compositor/input"
                settingsKey: "compositor/ignore-trackpad"
                label: qsTr("Ignore built-in trackpad")
                description: qsTr("Not available: the input backend has no "
                                  + "per-device ignore policy in this build.")
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "motor.switchControl"
                owner: "switch-control delegate"
                settingsKey: "switch-control/delegate"
                label: qsTr("Switch control")
                description: qsTr("Not available: no switch-control service is "
                                  + "installed.")
                showSeparator: false
                enabled: false
                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Speech ──────────────────────────────────────────────────────
        SettingsGroup {
            id: speechGroup
            width: parent.width
            sectionId: "speech"
            title: qsTr("Speech")

            SettingsRow {
                id: liveSpeechRow
                objectName: "accessibilityLiveSpeechRow"
                width: parent.width
                settingsId: "speech.liveSpeech"
                owner: "speech-dispatcher/espeak-ng"
                settingsKey: "spd-say"
                label: qsTr("Live speech")
                description: root.speechAvailable
                    ? qsTr("Delegated to the session speech engine.")
                    : qsTr("Not available: neither spd-say nor espeak-ng is "
                           + "installed.")
                trailingAction: root.speechAvailable
                    ? qsTr("Speak Sample") : qsTr("Unavailable")
                trailingActionEnabled: root.speechAvailable
                onTrailingActionClicked:
                    Settings.speakAccessibilityText(qsTr("Live speech is ready."))
            }

            SettingsRow {
                width: parent.width
                settingsId: "speech.personalVoice"
                owner: "omit"
                settingsKey: "omit"
                label: qsTr("Personal voice")
                description: qsTr("Not available: there is no personal-voice "
                                  + "equivalent in this build.")
                enabled: false
                controlData: Text {
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                width: parent.width
                settingsId: "speech.vocalShortcuts"
                owner: "omit"
                settingsKey: "omit"
                label: qsTr("Vocal shortcuts")
                description: qsTr("Not available: there is no vocal-shortcut "
                                  + "engine in this build.")
                showSeparator: false
                enabled: false
                controlData: Text {
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Accessibility Shortcut ──────────────────────────────────────
        SettingsGroup {
            id: shortcutGroup
            width: parent.width
            sectionId: "shortcut"
            title: qsTr("Accessibility Shortcut")

            SettingsKeyRow {
                width: parent.width
                settingsId: "shortcut.recorder"
                owner: "shell/compositor shortcut engine"
                settingsKey: "accessibility.shortcut"
                label: qsTr("Accessibility shortcut")
                description: qsTr("Record a chord that opens this pane from any "
                                  + "application. The compositor's one shortcut "
                                  + "engine owns the binding.")
                showSeparator: false
                controlData: ShortcutRecorder {
                    id: shortcutRecorder
                    objectName: "accessibilityShortcutRecorder"
                    onRecorded: (value) =>
                        Settings.set("accessibility.shortcut", value)
                    onCleared: () => Settings.set("accessibility.shortcut", "")
                }
            }
        }

        // ── Live preview of the glass overrides ─────────────────────────
        SettingsGroup {
            id: previewGroup
            width: parent.width
            sectionId: "display"
            title: qsTr("Preview")

            Item {
                width: parent.width
                height: 96

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.controls.settingsGroup.radius
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: "#2f6fde" }
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

                    Text {
                        objectName: "accessibilityPreviewMode"
                        anchors.centerIn: parent
                        text: root.previewResolved.renderMode
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
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
    Binding { target: balanceSlider; property: "value"; value: root.balance }
    Binding { target: doubleClickSpeedSlider; property: "value"
              value: root.doubleClickSpeed }
    Binding { target: textSizeSelect; property: "currentIndex"
              value: root.textSizeIndex }
    Binding { target: magnifierModeSelect; property: "currentIndex"
              value: root.magnifierModeIndex }
    Binding { target: shortcutRecorder; property: "chord"
              value: root.accessibilityShortcut }
}
