// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Sound pane (T-15.3b).
//
// The pane and the Control Center tile are one functional unit. Device
// selection, `Output volume`, and `Mute` go through the `Settings` singleton to
// the PipeWire/WirePlumber adapter (T-15.3a) via the bridge host — selecting a
// device makes it the default. The `Sound Effects` and `Balance` rows are
// settingsd keys (`services/settingsd/src/schema.rs`, group `sound`), so they
// persist and apply live without a restart.
//
// Opening the pane asks the host for a re-read. Absence is a normal state:
// WirePlumber gone disables the routing half and shows a one-line note; the
// Sound Effects/Balance controls still work on the settingsd defaults.
//
// Deviations from the macOS capture (ADR 0122): the alert-sound preview button
// and the `?` help are omitted (no alert playback engine yet, Apple-only); an
// actual alert/UI-sound engine is a documented follow-up. The captured names
// are replaced by ours (`Chime` default).
Item {
    id: root

    // The bridge host's audio view (empty when absent).
    readonly property var view: Settings.sound
    readonly property bool available: Settings.soundAvailable
    readonly property bool present: view.state === "available"
    readonly property bool ready: root.available && root.present

    readonly property var outputs: view.sinks !== undefined ? view.sinks : []
    readonly property var inputs: view.sources !== undefined ? view.sources : []
    // The `Output & Input` segmented control: 0 = Output (the pane's opening
    // tab), 1 = Input.
    property int tabIndex: 0
    readonly property bool outputTab: root.tabIndex === 0
    readonly property var devices: root.outputTab ? root.outputs : root.inputs

    readonly property bool muted: view.muted === true
    readonly property real volume: view.volume !== undefined ? Number(view.volume) : 0.0

    // The settingsd-backed Sound Effects / Balance values.
    readonly property string alertSound: Settings.values["sound.alertSound"] || "Chime"
    readonly property real alertVolume: Settings.values["sound.alertVolume"] !== undefined
        ? Number(Settings.values["sound.alertVolume"]) : 0.8
    readonly property string playEffectsThrough:
        Settings.values["sound.playEffectsThrough"] || "output"
    readonly property real balance: Settings.values["sound.balance"] !== undefined
        ? Number(Settings.values["sound.balance"]) : 0.5
    readonly property bool playOnStartup:
        Settings.values["sound.playOnStartup"] === true
    readonly property bool uiEffects: Settings.values["sound.uiEffects"] === true
    readonly property bool volumeFeedback:
        Settings.values["sound.volumeFeedback"] === true

    readonly property var alertSoundOptions: [
        { value: "Chime", label: qsTr("Chime") },
        { value: "Marimba", label: qsTr("Marimba") },
        { value: "Pulse", label: qsTr("Pulse") },
        { value: "Woodblock", label: qsTr("Woodblock") },
        { value: "Breeze", label: qsTr("Breeze") }
    ]
    readonly property var effectsThroughOptions: [
        { value: "output", label: qsTr("Selected Sound Output Device") },
        { value: "alerts", label: qsTr("Alerts Device") }
    ]
    readonly property var tabOptions: [
        { label: qsTr("Output") },
        { label: qsTr("Input") }
    ]

    // Test surface (used by tst_settings_sound.qml).
    property alias effectsGroup: effectsGroup
    property alias alertSoundSelect: alertSoundSelect
    property alias alertVolumeSlider: alertVolumeSlider
    property alias playOnStartupToggle: playOnStartupToggle
    property alias uiEffectsToggle: uiEffectsToggle
    property alias volumeFeedbackToggle: volumeFeedbackToggle
    property alias outputInputGroup: outputInputGroup
    property alias tabControl: tabControl
    property alias deviceRepeater: deviceRepeater
    property alias outputVolumeSlider: outputVolumeSlider
    property alias muteToggle: muteToggle
    property alias balanceSlider: balanceSlider
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function alertSoundIndex() {
        for (var i = 0; i < root.alertSoundOptions.length; ++i) {
            if (root.alertSoundOptions[i].value === root.alertSound)
                return i;
        }
        return 0;
    }

    function effectsThroughIndex() {
        return root.playEffectsThrough === "alerts" ? 1 : 0;
    }

    // The reference's `Type` column: ours is derived from the device label
    // rather than invented hardware metadata.
    function deviceType(device) {
        var description = device.description !== undefined ? device.description : "";
        if (description.indexOf("Built-in") >= 0)
            return qsTr("Built-in");
        if (description.indexOf("USB") >= 0)
            return qsTr("USB");
        return qsTr("External");
    }

    // Clicking a device row makes it the default for the active tab.
    function selectDevice(id) {
        if (!root.ready || !id)
            return;
        if (root.outputTab)
            Settings.setSoundDefaultSink(id);
        else
            Settings.setSoundDefaultSource(id);
    }

    function toggleMute() {
        if (!root.ready)
            return;
        Settings.setSoundMute(!root.muted);
    }

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshSound()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Sound Effects ─────────────────────────────────────────────────
        SettingsGroup {
            id: effectsGroup
            width: parent.width
            title: qsTr("Sound Effects")

            SettingsRow {
                width: parent.width
                label: qsTr("Alert sound")
                controlData: Select {
                    id: alertSoundSelect
                    accessibleName: qsTr("Alert sound")
                    model: root.alertSoundOptions
                    onActivated: (index) => Settings.set(
                                    "sound.alertSound",
                                    root.alertSoundOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Play sound effects through")
                controlData: Select {
                    id: effectsThroughSelect
                    accessibleName: qsTr("Play sound effects through")
                    model: root.effectsThroughOptions
                    onActivated: (index) => Settings.set(
                                    "sound.playEffectsThrough",
                                    root.effectsThroughOptions[index].value)
                }
            }

            Item {
                width: parent.width
                implicitHeight: alertVolumeColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: alertVolumeColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Alert volume")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: alertVolumeSlider
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        accessibleName: qsTr("Alert volume")
                        minLabel: qsTr("Low")
                        maxLabel: qsTr("High")
                        onMoved: (value) => Settings.set("sound.alertVolume", value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Play sound on startup")
                controlData: Toggle {
                    id: playOnStartupToggle
                    text: ""
                    onToggled: (checked) => Settings.set("sound.playOnStartup", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Play user interface sound effects")
                controlData: Toggle {
                    id: uiEffectsToggle
                    text: ""
                    onToggled: (checked) => Settings.set("sound.uiEffects", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Play feedback when volume is changed")
                showSeparator: false
                controlData: Toggle {
                    id: volumeFeedbackToggle
                    text: ""
                    onToggled: (checked) => Settings.set("sound.volumeFeedback", checked)
                }
            }
        }

        // ── Output & Input ────────────────────────────────────────────────
        SettingsGroup {
            id: outputInputGroup
            width: parent.width
            title: qsTr("Output & Input")
            visible: root.ready

            SegmentedControl {
                id: tabControl
                width: parent.width
                model: root.tabOptions
                onActivated: (index) => root.tabIndex = index
            }

            // The device table: a `Name`/`Type` header plus one selectable row
            // per device. The default row is highlighted.
            Item {
                width: parent.width
                height: 28

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Name")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Text {
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Type")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            Text {
                objectName: "soundNoDevices"
                width: parent.width
                visible: root.ready && root.devices.length === 0
                text: root.outputTab ? qsTr("No output devices.")
                                     : qsTr("No input devices.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                leftPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: deviceRepeater
                model: root.ready ? root.devices : []
                delegate: Rectangle {
                    id: deviceRow
                    required property var modelData
                    width: parent.width
                    height: 40
                    radius: Theme.primitive.radius.sm
                    color: deviceRow.modelData.default === true
                        ? Theme.color.accentMuted : "transparent"

                    Text {
                        objectName: "soundDeviceName"
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: deviceTypeLabel.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: deviceRow.modelData.description !== undefined
                            ? deviceRow.modelData.description
                            : deviceRow.modelData.name
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Text {
                        id: deviceTypeLabel
                        objectName: "soundDeviceType"
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.deviceType(deviceRow.modelData)
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Accessible.role: Accessible.RadioButton
                    Accessible.name: (deviceRow.modelData.description !== undefined
                                      ? deviceRow.modelData.description
                                      : deviceRow.modelData.name)
                    Accessible.checkable: true
                    Accessible.checked: deviceRow.modelData.default === true
                    Accessible.focusable: true
                    activeFocusOnTab: true
                    Accessible.onPressAction: root.selectDevice(deviceRow.modelData.id)

                    TapHandler {
                        onTapped: root.selectDevice(deviceRow.modelData.id)
                    }
                }
            }

            // Output volume + Mute are output-only (the adapter's volume/mute
            // writes target the default sink).
            Item {
                width: parent.width
                height: root.outputTab
                    ? outputVolumeColumn.implicitHeight + 2 * Theme.primitive.spacing.sm : 0
                visible: root.outputTab

                Column {
                    id: outputVolumeColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Output volume")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: outputVolumeSlider
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        enabled: root.ready
                        accessibleName: qsTr("Output volume")
                        minLabel: qsTr("Low")
                        maxLabel: qsTr("High")
                        onMoved: (value) => Settings.setSoundVolume(value)
                    }
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.outputTab
                label: qsTr("Mute")
                controlData: Toggle {
                    id: muteToggle
                    text: ""
                    enabled: root.ready
                    onToggled: (checked) => Settings.setSoundMute(checked)
                }
            }

            // ── Balance ───────────────────────────────────────────────────
            Item {
                width: parent.width
                implicitHeight: balanceColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm

                Column {
                    id: balanceColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Balance")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Slider {
                        id: balanceSlider
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        accessibleName: qsTr("Balance")
                        minLabel: qsTr("Left")
                        maxLabel: qsTr("Right")
                        onMoved: (value) => Settings.set("sound.balance", value)
                    }
                }
            }
        }

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "soundAbsenceNote"
                width: parent.width
                text: qsTr("No audio devices are available on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }

    // The device table is a two-way surface: the controls write on
    // interaction; these keep them in step with an external change.
    Binding {
        target: alertSoundSelect
        property: "currentIndex"
        value: root.alertSoundIndex()
    }
    Binding {
        target: effectsThroughSelect
        property: "currentIndex"
        value: root.effectsThroughIndex()
    }
    Binding {
        target: playOnStartupToggle
        property: "checked"
        value: root.playOnStartup
    }
    Binding {
        target: uiEffectsToggle
        property: "checked"
        value: root.uiEffects
    }
    Binding {
        target: volumeFeedbackToggle
        property: "checked"
        value: root.volumeFeedback
    }
    Binding {
        target: tabControl
        property: "currentIndex"
        value: root.tabIndex
    }
    Binding {
        target: alertVolumeSlider
        property: "value"
        value: root.alertVolume
    }
    Binding {
        target: outputVolumeSlider
        property: "value"
        value: root.muted ? 0.0 : root.volume
    }
    Binding {
        target: muteToggle
        property: "checked"
        value: root.muted
    }
    Binding {
        target: balanceSlider
        property: "value"
        value: root.balance
    }
}