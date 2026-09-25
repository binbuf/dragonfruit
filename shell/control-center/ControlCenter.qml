// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Control Center panel (T-11.3a/T-11.3b): one curated quick-settings
// surface opened from the menu-bar Control Center item (or its keyboard
// shortcut). It is rendered into its own top-right `overlay` layer surface by
// `ShellController`; this item is the panel's whole scene.
//
// The panel is a pure view with one test seam: `tiles` is the normalized
// model a headless test can assert without touching the scene, and every
// gesture is raised as a signal the shell forwards to the owning adapter or
// owner (`services/system-status`, the notification service, settingsd). The
// QML never talks to a daemon itself.
//
// T-11.3a shipped the Wi-Fi, Sound, and Display tiles. T-11.3b adds the
// Focus/DND and dark-mode tiles (live apply) and the accessible-role pass.
Item {
    id: root

    // The decoded bridge-host views (services/system-status) and the
    // settingsd brightness / appearance values, pushed by ShellController.
    property var wifi: ({})
    property var audio: ({})
    property real brightness: 1.0
    // The notification service's Focus/DND policy view
    // (`{mode, allowList, batchedCount}`); empty when the service is absent.
    property var focusPolicy: ({})
    // The effective design-system scheme: true when the resolved Theme is
    // dark (`appearance.colorScheme` resolved against the host for `auto`).
    property bool dark: false

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

    // The notification service's mode (`off`/`focus`/`dnd`). The toggle is Do
    // Not Disturb: `focus` also lights it, because both suppress banners.
    readonly property string focusMode: root.focusPolicy.mode !== undefined
        ? root.focusPolicy.mode : "off"
    readonly property bool focusOn: root.focusMode === "focus"
        || root.focusMode === "dnd"
    readonly property string focusLabel: {
        if (root.focusMode === "dnd")
            return qsTr("Do Not Disturb");
        if (root.focusMode === "focus")
            return qsTr("Focus");
        return qsTr("Off");
    }
    readonly property int focusBatched: root.focusPolicy.batchedCount !== undefined
        ? root.focusPolicy.batchedCount : 0
    readonly property string focusSubtitle: {
        if (root.focusOn && root.focusBatched > 0)
            return qsTr("%1 \u00b7 %2 silenced").arg(root.focusLabel)
                .arg(root.focusBatched);
        return root.focusLabel;
    }

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
            id: "focus",
            kind: "toggle",
            title: qsTr("Focus"),
            subtitle: root.focusSubtitle,
            checked: root.focusOn,
            enabled: true
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
        },
        {
            id: "dark",
            kind: "toggle",
            title: qsTr("Dark Mode"),
            subtitle: root.dark ? qsTr("On") : qsTr("Off"),
            checked: root.dark,
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
    // Focus/DND is owned by the notification service (T-11.2a). On = Do Not
    // Disturb (`dnd`), off clears the policy (`off`).
    signal focusToggleRequested(bool enabled)
    signal focusSettingsRequested()
    // Dark mode is owned by settingsd (`appearance.colorScheme`); the shell's
    // ThemeBinding applies it live.
    signal darkModeToggleRequested(bool dark)
    signal appearanceSettingsRequested()

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

    // Toggle Do Not Disturb without the switch (keyboard/AT-SPI and tests).
    function toggleFocus() {
        root.focusToggleRequested(!root.focusOn);
    }

    // Toggle the absolute color scheme (the panel never writes `auto`).
    function toggleDarkMode() {
        root.darkModeToggleRequested(!root.dark);
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
            spacing: Theme.primitive.spacing.sm

            // ── Wi-Fi ────────────────────────────────────────────────────
            Rectangle {
                id: wifiTile
                objectName: "wifiTile"
                width: parent.width
                implicitHeight: wifiColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Wi-Fi")

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
                            accessibleName: qsTr("Wi-Fi")
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

                    TextLink {
                        objectName: "wifiSettingsLink"
                        text: qsTr("Wi-Fi Settings\u2026")
                        accessibleName: qsTr("Open Wi-Fi Settings")
                        onActivated: root.wifiSettingsRequested()
                    }
                }
            }

            // ── Focus / Do Not Disturb ───────────────────────────────────
            Rectangle {
                id: focusTile
                objectName: "focusTile"
                width: parent.width
                implicitHeight: focusColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Focus")

                Column {
                    id: focusColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "focusIcon"
                            name: "focus"
                            tileSize: 32
                            iconSize: 18
                            active: root.focusOn
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 32 - focusToggle.width
                                   - 2 * Theme.primitive.spacing.md

                            Text {
                                objectName: "focusTitle"
                                text: qsTr("Focus")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                font.weight: Theme.primitive.font.weightMedium
                            }

                            Text {
                                objectName: "focusSubtitle"
                                width: parent.width
                                text: root.focusSubtitle
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }
                        }

                        Toggle {
                            id: focusToggle
                            objectName: "focusToggle"
                            accessibleName: qsTr("Do Not Disturb")
                            anchors.verticalCenter: parent.verticalCenter
                            onToggled: (checked) => root.focusToggleRequested(checked)
                        }
                    }

                    TextLink {
                        objectName: "focusSettingsLink"
                        text: qsTr("Focus Settings\u2026")
                        accessibleName: qsTr("Open Focus Settings")
                        onActivated: root.focusSettingsRequested()
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
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Sound")

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

                    TextLink {
                        objectName: "muteButton"
                        text: root.muted ? qsTr("Unmute") : qsTr("Mute")
                        accessibleName: root.muted ? qsTr("Unmute") : qsTr("Mute")
                        visible: root.audioAvailable
                        onActivated: root.muteToggleRequested()
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
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Display")

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

            // ── Dark Mode ────────────────────────────────────────────────
            Rectangle {
                id: darkTile
                objectName: "darkTile"
                width: parent.width
                implicitHeight: darkColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Dark Mode")

                Column {
                    id: darkColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "darkIcon"
                            name: "appearance"
                            tileSize: 32
                            iconSize: 18
                            active: root.dark
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 32 - darkToggle.width
                                   - 2 * Theme.primitive.spacing.md

                            Text {
                                objectName: "darkTitle"
                                text: qsTr("Dark Mode")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                font.weight: Theme.primitive.font.weightMedium
                            }

                            Text {
                                objectName: "darkSubtitle"
                                width: parent.width
                                text: root.dark ? qsTr("On") : qsTr("Off")
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }
                        }

                        Toggle {
                            id: darkToggle
                            objectName: "darkToggle"
                            accessibleName: qsTr("Dark Mode")
                            anchors.verticalCenter: parent.verticalCenter
                            onToggled: (checked) => root.darkModeToggleRequested(checked)
                        }
                    }

                    TextLink {
                        objectName: "appearanceSettingsLink"
                        text: qsTr("Appearance Settings\u2026")
                        accessibleName: qsTr("Open Appearance Settings")
                        onActivated: root.appearanceSettingsRequested()
                    }
                }
            }
        }
    }

    // The three external-state switches use a `Binding` element rather than a
    // direct `checked:` binding: `Toggle` writes its own `checked` when the
    // user taps it, which would otherwise break the binding and stop a
    // daemon-originated change (settingsd/notification service) from
    // reflecting back into the panel.
    Binding {
        target: toggle
        property: "checked"
        value: root.wifiOn
    }
    Binding {
        target: focusToggle
        property: "checked"
        value: root.focusOn
    }
    Binding {
        target: darkToggle
        property: "checked"
        value: root.dark
    }

    // A trailing text link (the macOS sheet anatomy). Keyboard- and
    // AT-SPI-operable so the whole panel is reachable without a pointer.
    component TextLink: Text {
        property string accessibleName: text
        signal activated()

        activeFocusOnTab: true
        Accessible.role: Accessible.Button
        Accessible.name: accessibleName
        Accessible.focusable: true
        Accessible.onPressAction: activated()
        color: linkHover.hovered ? Theme.color.accentHover : Theme.color.accent
        font.pixelSize: Theme.primitive.font.sizeSm

        HoverHandler { id: linkHover }

        TapHandler {
            onTapped: activated()
        }

        FocusRing {
            target: parent
            shown: parent.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                activated();
                event.accepted = true;
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