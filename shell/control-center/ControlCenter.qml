// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Control Center panel (T-11.3a): one curated quick-settings surface
// opened from the menu-bar Control Center item (or its keyboard shortcut).
// It is rendered into its own top-right `overlay` layer surface by
// `ShellController`; this item is the panel's whole scene.
//
// The panel is a pure view with one test seam: `tiles` is the normalized
// model a headless test can assert without touching the scene, and every
// gesture is raised as a signal the shell forwards to the owning adapter
// (`services/system-status`) or writes to settingsd. The QML never talks to a
// daemon itself.
//
// T-11.3a ships the Wi-Fi, Sound, and Display tiles; Focus/DND, dark mode,
// and the a11y pass are T-11.3b.
Item {
    id: root

    // The decoded bridge-host views (services/system-status) and the
    // settingsd brightness value, pushed by ShellController.
    property var wifi: ({})
    property var audio: ({})
    property real brightness: 1.0

    // Wi-Fi radio writes are not exposed by the T-07 adapter yet (a T-15
    // follow-up); the toggle reflects state and is inert until then.
    property bool wifiWritable: false

    readonly property bool wifiAvailable: root.wifi.state === "available"
    readonly property bool wifiOn: root.wifi.radioEnabled === true
    readonly property string wifiLabel: {
        if (!root.wifiAvailable)
            return qsTr("Unavailable");
        if (root.wifi.label !== undefined && root.wifi.label !== "")
            return root.wifi.label;
        return root.wifiOn ? qsTr("On") : qsTr("Off");
    }

    readonly property bool audioAvailable: root.audio.state === "available"
    readonly property bool muted: root.audio.muted === true
    property real volume: root.audio.volume !== undefined ? root.audio.volume : 0.0

    // The normalized tile model, in panel order. A headless test asserts this
    // without instantiating the controls below.
    readonly property var tiles: [
        {
            id: "wifi",
            kind: "toggle",
            title: qsTr("Wi-Fi"),
            subtitle: root.wifiLabel,
            checked: root.wifiOn,
            enabled: root.wifiAvailable && root.wifiWritable
        },
        {
            id: "volume",
            kind: "slider",
            title: qsTr("Sound"),
            value: root.volume,
            muted: root.muted,
            enabled: root.audioAvailable
        },
        {
            id: "brightness",
            kind: "slider",
            title: qsTr("Display"),
            value: root.brightness,
            enabled: true
        }
    ]

    // Re-seed the local slider from external state without fighting a drag.
    onVolumeChanged: volumeSlider.value = root.volume

    signal closed()
    signal volumeSetRequested(double volume)
    signal muteToggleRequested()
    signal brightnessSetRequested(double level)
    signal wifiToggleRequested(bool enabled)
    signal wifiSettingsRequested()

    // Apply a volume fraction (0..1) and raise the request.
    function setVolume(fraction) {
        root.volume = Math.max(0.0, Math.min(1.0, fraction));
        root.volumeSetRequested(root.volume);
    }

    // Apply a brightness level (0..1) and raise the request.
    function setBrightness(level) {
        root.brightness = Math.max(0.0, Math.min(1.0, level));
        root.brightnessSetRequested(root.brightness);
    }

    function toggleWifi() {
        if (!root.wifiAvailable || !root.wifiWritable)
            return;
        root.wifiToggleRequested(!root.wifiOn);
    }

    focus: true
    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Control Center")

    Keys.onEscapePressed: (event) => {
        root.closed();
        event.accepted = true;
    }

    // The panel background. The compositor frosts this chrome surface (the
    // T-04 backdrop pass), so the panel is the material itself.
    Rectangle {
        id: background
        anchors.fill: parent
        color: Theme.color.surfaceElevated
        radius: Theme.controls.popover.radius
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        Column {
            id: content
            anchors.fill: parent
            anchors.margins: Theme.primitive.spacing.md
            spacing: Theme.primitive.spacing.md

            // ── Wi-Fi ────────────────────────────────────────────────────
            Rectangle {
                id: wifiTile
                objectName: "wifiTile"
                width: parent.width
                implicitHeight: wifiColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken

                Column {
                    id: wifiColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "wifiIcon"
                            name: "wifi"
                            tileSize: 32
                            iconSize: 18
                            active: root.wifiOn && root.wifiAvailable
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 32 - toggle.width
                                   - 2 * Theme.primitive.spacing.md

                            Text {
                                objectName: "wifiTitle"
                                text: qsTr("Wi-Fi")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                font.weight: Theme.primitive.font.weightMedium
                            }

                            Text {
                                objectName: "wifiSubtitle"
                                width: parent.width
                                text: root.wifiLabel
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }
                        }

                        Toggle {
                            id: toggle
                            objectName: "wifiToggle"
                            checked: root.wifiOn
                            enabled: root.wifiAvailable && root.wifiWritable
                            anchors.verticalCenter: parent.verticalCenter
                            onToggled: (checked) => root.wifiToggleRequested(checked)
                        }
                    }

                    Rectangle {
                        width: parent.width
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }

                    Text {
                        objectName: "wifiSettingsLink"
                        text: qsTr("Wi-Fi Settings\u2026")
                        color: Theme.color.accent
                        font.pixelSize: Theme.primitive.font.sizeSm

                        TapHandler {
                            onTapped: root.wifiSettingsRequested()
                        }
                    }
                }
            }

            // ── Sound ────────────────────────────────────────────────────
            Rectangle {
                id: volumeTile
                objectName: "volumeTile"
                width: parent.width
                implicitHeight: volumeColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                opacity: root.audioAvailable ? 1.0 : 0.5

                Column {
                    id: volumeColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "volumeIcon"
                            name: "volume"
                            tileSize: 32
                            iconSize: 18
                            active: root.audioAvailable && !root.muted
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 32 - percent.width
                                   - 2 * Theme.primitive.spacing.md

                            Text {
                                text: qsTr("Sound")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                font.weight: Theme.primitive.font.weightMedium
                            }

                            Text {
                                objectName: "volumeSubtitle"
                                text: root.muted ? qsTr("Muted") : qsTr("Output")
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                        }

                        Text {
                            id: percent
                            objectName: "volumePercent"
                            text: qsTr("%1%").arg(Math.round(root.volume * 100))
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    Slider {
                        id: volumeSlider
                        objectName: "volumeSlider"
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        value: root.volume
                        enabled: root.audioAvailable
                        accessibleName: qsTr("Volume")
                        onMoved: (value) => root.setVolume(value)
                    }

                    Text {
                        objectName: "muteButton"
                        text: root.muted ? qsTr("Unmute") : qsTr("Mute")
                        color: Theme.color.accent
                        font.pixelSize: Theme.primitive.font.sizeSm
                        visible: root.audioAvailable

                        TapHandler {
                            onTapped: root.muteToggleRequested()
                        }
                    }
                }
            }

            // ── Display / brightness ─────────────────────────────────────
            Rectangle {
                id: brightnessTile
                objectName: "brightnessTile"
                width: parent.width
                implicitHeight: brightnessColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken

                Column {
                    id: brightnessColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "brightnessIcon"
                            name: "brightness"
                            tileSize: 32
                            iconSize: 18
                            active: true
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: qsTr("Display")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            font.weight: Theme.primitive.font.weightMedium
                        }
                    }

                    Slider {
                        id: brightnessSlider
                        objectName: "brightnessSlider"
                        width: parent.width
                        from: 0.0
                        to: 1.0
                        value: root.brightness
                        accessibleName: qsTr("Brightness")
                        onMoved: (value) => root.setBrightness(value)
                    }
                }
            }
        }
    }

    // A circular icon tile (the macOS sheet anatomy): a tinted disc with the
    // design-system glyph centered. Active tiles use the accent; inactive ones
    // use the muted control fill.
    component IconTile: Rectangle {
        property string name: "wifi"
        property int tileSize: 32
        property int iconSize: 18
        property bool active: true

        width: tileSize
        height: tileSize
        radius: tileSize / 2
        color: active ? Theme.color.accentMuted : Theme.color.controlFill

        Icon {
            anchors.centerIn: parent
            name: parent.name
            size: parent.iconSize
            color: parent.active ? Theme.color.accent : Theme.color.textSecondary
        }
    }
}