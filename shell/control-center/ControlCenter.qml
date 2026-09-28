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
    // The Bluetooth view from the bridge host (T-15.1b), shaped by
    // `SystemStatusModel`: `{ state, present, powered, discovering, label,
    // glyph, knownDevices: [...] }`. Empty/absent hides the tile.
    property var bluetooth: ({})
    // The storage view from the bridge host (T-15.2b), shaped by
    // `SystemStatusModel`: `{ state, present, label, mountedCount, volumes:
    // [...], drives: [...] }`. Empty/absent hides the tile.
    property var storage: ({})
    // The read-only input inventory from the bridge host (T-15.4b), shaped by
    // `SystemStatusModel`: `{ state, present, label, glyph, deviceCount,
    // keyboardCount, pointerCount, devices: [...] }`. Empty/absent hides the
    // tile.
    property var input: ({})
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

    // Bluetooth writes are live through the T-15.1b bridge host.
    property bool bluetoothWritable: false

    // Storage writes (mount/unmount/eject) are live through the T-15.2b bridge
    // host.
    property bool storageWritable: false

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
    // The default output device's label (T-15.3b), so the tile reflects the
    // routing state the Settings pane writes; empty when the adapter named none.
    readonly property var audioOutputs: root.audio.sinks !== undefined
        ? root.audio.sinks : []
    readonly property string audioOutputName: {
        for (var i = 0; i < root.audioOutputs.length; ++i) {
            if (root.audioOutputs[i].default === true)
                return root.audioOutputs[i].description !== undefined
                    ? root.audioOutputs[i].description : "";
        }
        return "";
    }

    // Bluetooth (T-15.1b). The tile hides when `bluetoothd` is absent
    // (`unavailable`) or when the daemon is present with no controller
    // (`present: false`), mirroring the model's hide rule.
    readonly property bool bluetoothAvailable: root.bluetooth.state === "available"
    readonly property bool bluetoothPresent: root.bluetooth.present === true
    readonly property bool bluetoothVisible: root.bluetoothAvailable
        && root.bluetoothPresent
    readonly property bool bluetoothOn: root.bluetooth.powered === true
    readonly property string bluetoothLabel: {
        if (!root.bluetoothVisible)
            return qsTr("Unavailable");
        if (root.bluetooth.discovering === true)
            return qsTr("Discovering\u2026");
        if (root.bluetooth.label !== undefined && root.bluetooth.label !== "")
            return root.bluetooth.label;
        return root.bluetoothOn ? qsTr("On") : qsTr("Off");
    }
    readonly property var bluetoothDevices: root.bluetooth.knownDevices !== undefined
        ? root.bluetooth.knownDevices : []

    // Storage (T-15.2b). The tile hides when UDisks2 is absent (`unavailable`)
    // or when the daemon is present with no mountable volume (`present: false`),
    // mirroring the model's hide rule.
    readonly property bool storageAvailable: root.storage.state === "available"
    readonly property bool storagePresent: root.storage.present === true
    readonly property bool storageVisible: root.storageAvailable
        && root.storagePresent
    readonly property var storageVolumes: root.storage.volumes !== undefined
        ? root.storage.volumes : []
    readonly property int storageMountedCount:
        root.storage.mountedCount !== undefined ? root.storage.mountedCount : 0
    readonly property string storageLabel: {
        if (!root.storageVisible)
            return qsTr("Unavailable");
        if (root.storage.label !== undefined && root.storage.label !== "")
            return root.storage.label;
        return root.storageMountedCount > 0
            ? qsTr("%1 mounted").arg(root.storageMountedCount) : qsTr("No volumes");
    }

    // Input (T-15.4b). The tile hides when libinput is absent (`unavailable`) or
    // when the stack is present with no recognized device (`present: false`),
    // mirroring the model's hide rule. It is a read-only inventory summary; the
    // link opens the Keyboard pane where the preferences live.
    readonly property bool inputAvailable: root.input.state === "available"
    readonly property bool inputPresent: root.input.present === true
    readonly property bool inputVisible: root.inputAvailable && root.inputPresent
    readonly property string inputLabel: {
        if (!root.inputVisible)
            return qsTr("Unavailable");
        if (root.input.label !== undefined && root.input.label !== "")
            return root.input.label;
        return qsTr("No input devices");
    }

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

    // Clipboard history (T-13.5b): the most recent observed selections, pushed
    // by `ShellController` from the pure `ClipboardHistory` store. The panel
    // keeps its tile model unchanged and renders this as a separate section.
    property var clipboardEntries: []
    readonly property int clipboardShown: Math.min(root.clipboardEntries.length, 5)

    signal clipboardCopyRequested(int index)
    signal clipboardPinToggled(int index, bool pinned)
    signal clipboardClearRequested()

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
            id: "bluetooth",
            kind: "toggle",
            title: qsTr("Bluetooth"),
            subtitle: root.bluetoothLabel,
            checked: root.bluetoothOn,
            visible: root.bluetoothVisible,
            enabled: root.bluetoothVisible && root.bluetoothWritable
        },
        {
            id: "storage",
            kind: "storage",
            title: qsTr("Storage"),
            subtitle: root.storageLabel,
            visible: root.storageVisible,
            enabled: root.storageVisible && root.storageWritable
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
            subtitle: root.audioOutputName,
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
        },
        {
            id: "keyboard",
            kind: "info",
            title: qsTr("Keyboard"),
            subtitle: root.inputLabel,
            visible: root.inputVisible,
            enabled: root.inputVisible
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
    // Sound (T-15.3b): the tile reflects the default output device the pane
    // routes to; the link opens the pane (T-16).
    signal soundSettingsRequested()
    // Bluetooth (T-15.1b): the toggle powers the adapter; a known-device row
    // connects/disconnects it; the link opens the pane (T-16).
    signal bluetoothToggleRequested(bool enabled)
    signal bluetoothDeviceToggled(string address, bool connected)
    signal bluetoothSettingsRequested()
    // Storage (T-15.2b): mount/unmount a volume, eject the drive that owns it,
    // and the link opens the pane (T-16).
    signal storageMountRequested(string volumePath)
    signal storageUnmountRequested(string volumePath)
    signal storageEjectRequested(string drivePath)
    signal storageSettingsRequested()
    // Focus/DND is owned by the notification service (T-11.2a). On = Do Not
    // Disturb (`dnd`), off clears the policy (`off`).
    signal focusToggleRequested(bool enabled)
    signal focusSettingsRequested()
    // Dark mode is owned by settingsd (`appearance.colorScheme`); the shell's
    // ThemeBinding applies it live.
    signal darkModeToggleRequested(bool dark)
    signal appearanceSettingsRequested()
    // The input tile is read-only; the link opens the Keyboard pane (T-16).
    signal keyboardSettingsRequested()

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

    // Toggle the Bluetooth radio without the switch (keyboard/AT-SPI/tests).
    function toggleBluetooth() {
        if (!root.bluetoothVisible || !root.bluetoothWritable)
            return;
        root.bluetoothToggleRequested(!root.bluetoothOn);
    }

    // Connect or disconnect one known device (a row tap).
    function toggleBluetoothDevice(address) {
        if (!root.bluetoothVisible || !root.bluetoothWritable || !address)
            return;
        for (var i = 0; i < root.bluetoothDevices.length; ++i) {
            if (root.bluetoothDevices[i].address === address) {
                root.bluetoothDeviceToggled(address,
                                            !root.bluetoothDevices[i].connected);
                return;
            }
        }
    }

    // Mount or unmount one volume (a row tap), based on its current state.
    function toggleStorageVolume(path) {
        if (!root.storageVisible || !root.storageWritable || !path)
            return;
        for (var i = 0; i < root.storageVolumes.length; ++i) {
            if (root.storageVolumes[i].path === path) {
                if (root.storageVolumes[i].mounted)
                    root.storageUnmountRequested(path);
                else
                    root.storageMountRequested(path);
                return;
            }
        }
    }

    // Eject the removable drive that owns one volume (a row tap).
    function ejectStorageVolume(path) {
        if (!root.storageVisible || !root.storageWritable || !path)
            return;
        for (var i = 0; i < root.storageVolumes.length; ++i) {
            if (root.storageVolumes[i].path === path
                    && root.storageVolumes[i].drivePath)
                root.storageEjectRequested(root.storageVolumes[i].drivePath);
        }
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
            objectName: "controlCenterContent"
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

            // ── Bluetooth (T-15.1b) ──────────────────────────────────────
            Rectangle {
                id: bluetoothTile
                objectName: "bluetoothTile"
                width: parent.width
                visible: root.bluetoothVisible
                implicitHeight: bluetoothColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Bluetooth")

                Column {
                    id: bluetoothColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "bluetoothIcon"
                            name: "bluetooth"
                            tileSize: 32
                            iconSize: 18
                            active: root.bluetoothOn
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 32 - bluetoothToggle.width
                                   - 2 * Theme.primitive.spacing.md

                            Text {
                                objectName: "bluetoothTitle"
                                text: qsTr("Bluetooth")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                font.weight: Theme.primitive.font.weightMedium
                            }

                            Text {
                                objectName: "bluetoothSubtitle"
                                width: parent.width
                                text: root.bluetoothLabel
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }
                        }

                        Toggle {
                            id: bluetoothToggle
                            objectName: "bluetoothToggle"
                            accessibleName: qsTr("Bluetooth")
                            enabled: root.bluetoothVisible && root.bluetoothWritable
                            anchors.verticalCenter: parent.verticalCenter
                            onToggled: (checked) => root.bluetoothToggleRequested(checked)
                        }
                    }

                    Rectangle {
                        objectName: "bluetoothSeparator"
                        width: parent.width
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                        visible: bluetoothDeviceList.count > 0
                    }

                    Column {
                        id: bluetoothDeviceList
                        objectName: "bluetoothDeviceList"
                        width: parent.width
                        spacing: Theme.primitive.spacing.xs
                        readonly property int count: root.bluetoothDevices.length

                        Repeater {
                            model: root.bluetoothDevices
                            delegate: Item {
                                id: bluetoothRow
                                required property var modelData
                                width: bluetoothDeviceList.width
                                height: 24

                                Text {
                                    objectName: "bluetoothDeviceName"
                                    anchors.left: parent.left
                                    anchors.right: bluetoothDeviceAction.left
                                    anchors.rightMargin: Theme.primitive.spacing.sm
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: bluetoothRow.modelData.name
                                    color: Theme.color.textPrimary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    elide: Text.ElideRight
                                }

                                Text {
                                    id: bluetoothDeviceAction
                                    objectName: "bluetoothDeviceAction"
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.right: parent.right
                                    text: bluetoothRow.modelData.connected
                                        ? qsTr("Connected") : qsTr("Not Connected")
                                    color: bluetoothRow.modelData.connected
                                        ? Theme.color.accent : Theme.color.textSecondary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    Accessible.role: Accessible.Button
                                    Accessible.name: qsTr("%1, %2")
                                        .arg(bluetoothRow.modelData.name)
                                        .arg(text)
                                    TapHandler {
                                        onTapped: root.toggleBluetoothDevice(
                                            bluetoothRow.modelData.address)
                                    }
                                }
                            }
                        }
                    }

                    TextLink {
                        objectName: "bluetoothSettingsLink"
                        text: qsTr("Bluetooth Settings\u2026")
                        accessibleName: qsTr("Open Bluetooth Settings")
                        onActivated: root.bluetoothSettingsRequested()
                    }
                }
            }

            // ── Storage (T-15.2b) ────────────────────────────────────────
            Rectangle {
                id: storageTile
                objectName: "storageTile"
                width: parent.width
                visible: root.storageVisible
                implicitHeight: storageColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Storage")

                Column {
                    id: storageColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "storageIcon"
                            name: "storage"
                            tileSize: 32
                            iconSize: 18
                            active: root.storageMountedCount > 0
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 32 - 2 * Theme.primitive.spacing.md

                            Text {
                                objectName: "storageTitle"
                                text: qsTr("Storage")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                font.weight: Theme.primitive.font.weightMedium
                            }

                            Text {
                                objectName: "storageSubtitle"
                                width: parent.width
                                text: root.storageLabel
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }
                        }
                    }

                    Column {
                        id: storageVolumeList
                        objectName: "storageVolumeList"
                        width: parent.width
                        spacing: Theme.primitive.spacing.xs
                        readonly property int count: root.storageVolumes.length
                        visible: count > 0

                        Repeater {
                            model: root.storageVolumes
                            delegate: Item {
                                id: storageRow
                                required property var modelData
                                width: storageVolumeList.width
                                height: 22

                                Text {
                                    objectName: "storageVolumeName"
                                    anchors.left: parent.left
                                    anchors.right: storageRowAction.left
                                    anchors.rightMargin: Theme.primitive.spacing.sm
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: storageRow.modelData.name
                                    color: Theme.color.textPrimary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    elide: Text.ElideRight
                                }

                                Text {
                                    id: storageRowEject
                                    objectName: "storageVolumeEject"
                                    visible: storageRow.modelData.removable === true
                                             && storageRow.modelData.drivePath !== undefined
                                    anchors.right: parent.right
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: qsTr("Eject")
                                    color: Theme.color.accent
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    Accessible.role: Accessible.Button
                                    Accessible.name: qsTr("Eject %1")
                                        .arg(storageRow.modelData.name)
                                    TapHandler {
                                        onTapped: root.ejectStorageVolume(
                                            storageRow.modelData.path)
                                    }
                                }

                                Text {
                                    id: storageRowAction
                                    objectName: "storageVolumeAction"
                                    anchors.right: storageRowEject.visible
                                        ? storageRowEject.left : parent.right
                                    anchors.rightMargin: storageRowEject.visible
                                        ? Theme.primitive.spacing.md : 0
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: storageRow.modelData.mounted
                                        ? qsTr("Unmount") : qsTr("Mount")
                                    color: Theme.color.accent
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    Accessible.role: Accessible.Button
                                    Accessible.name: qsTr("%1 %2")
                                        .arg(text).arg(storageRow.modelData.name)
                                    TapHandler {
                                        onTapped: root.toggleStorageVolume(
                                            storageRow.modelData.path)
                                    }
                                }
                            }
                        }
                    }

                    TextLink {
                        objectName: "storageSettingsLink"
                        text: qsTr("Storage Settings\u2026")
                        accessibleName: qsTr("Open Storage Settings")
                        onActivated: root.storageSettingsRequested()
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
                                width: parent.width
                                text: root.muted
                                    ? qsTr("Muted")
                                    : (root.audioOutputName.length > 0
                                       ? root.audioOutputName : qsTr("Output"))
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
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

                    Row {
                        spacing: Theme.primitive.spacing.md

                        TextLink {
                            objectName: "muteButton"
                            text: root.muted ? qsTr("Unmute") : qsTr("Mute")
                            accessibleName: root.muted ? qsTr("Unmute") : qsTr("Mute")
                            visible: root.audioAvailable
                            onActivated: root.muteToggleRequested()
                        }

                        TextLink {
                            objectName: "soundSettingsLink"
                            text: qsTr("Sound Settings\u2026")
                            accessibleName: qsTr("Open Sound Settings")
                            onActivated: root.soundSettingsRequested()
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

            // ── Keyboard (T-15.4b) ──────────────────────────────────────
            Rectangle {
                id: keyboardTile
                objectName: "keyboardTile"
                width: parent.width
                visible: root.inputVisible
                implicitHeight: keyboardColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Keyboard")

                Column {
                    id: keyboardColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.sm

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.md

                        IconTile {
                            objectName: "keyboardIcon"
                            name: "keyboard"
                            tileSize: 32
                            iconSize: 18
                            active: root.inputVisible
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 32 - 2 * Theme.primitive.spacing.md

                            Text {
                                objectName: "keyboardTitle"
                                text: qsTr("Keyboard")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.controls.button.fontSize
                                font.weight: Theme.primitive.font.weightMedium
                            }

                            Text {
                                objectName: "keyboardSubtitle"
                                width: parent.width
                                text: root.inputLabel
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                            }
                        }
                    }

                    TextLink {
                        objectName: "keyboardSettingsLink"
                        text: qsTr("Keyboard Settings\u2026")
                        accessibleName: qsTr("Open Keyboard Settings")
                        onActivated: root.keyboardSettingsRequested()
                    }
                }
            }

            // ── Clipboard history ────────────────────────────────────────
            Rectangle {
                id: clipboardTile
                objectName: "clipboardTile"
                width: parent.width
                implicitHeight: clipboardColumn.implicitHeight
                                + 2 * Theme.controls.settingsGroup.padding
                radius: Theme.primitive.radius.md
                color: Theme.color.surfaceSunken
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Clipboard")

                Column {
                    id: clipboardColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.controls.settingsGroup.padding
                    spacing: Theme.primitive.spacing.xs

                    Text {
                        objectName: "clipboardTitle"
                        text: qsTr("Clipboard")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        font.weight: Theme.primitive.font.weightMedium
                    }

                    Text {
                        objectName: "clipboardEmpty"
                        width: parent.width
                        visible: root.clipboardEntries.length === 0
                        text: qsTr("No recent items")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Repeater {
                        model: root.clipboardShown
                        delegate: Rectangle {
                            id: clipboardRow
                            required property int index
                            readonly property var entry: root.clipboardEntries[index]
                            width: parent.width
                            height: 26
                            radius: Theme.primitive.radius.sm
                            color: rowHover.hovered ? Theme.color.surfaceElevated
                                                    : "transparent"

                            HoverHandler { id: rowHover }

                            Text {
                                objectName: "clipboardRowPreview"
                                anchors.left: parent.left
                                anchors.leftMargin: Theme.primitive.spacing.xs
                                anchors.right: pinAction.left
                                anchors.verticalCenter: parent.verticalCenter
                                text: clipboardRow.entry.preview
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                elide: Text.ElideRight
                                Accessible.role: Accessible.Button
                                Accessible.name: qsTr("Copy %1").arg(clipboardRow.entry.preview)
                                TapHandler {
                                    onTapped: root.clipboardCopyRequested(clipboardRow.entry.index)
                                }
                            }

                            Text {
                                id: pinAction
                                objectName: "clipboardRowPin"
                                anchors.right: parent.right
                                anchors.rightMargin: Theme.primitive.spacing.xs
                                anchors.verticalCenter: parent.verticalCenter
                                text: clipboardRow.entry.pinned ? qsTr("Unpin") : qsTr("Pin")
                                color: clipboardRow.entry.pinned ? Theme.color.accent
                                                                 : Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                Accessible.role: Accessible.Button
                                Accessible.name: clipboardRow.entry.pinned
                                    ? qsTr("Unpin %1").arg(clipboardRow.entry.preview)
                                    : qsTr("Pin %1").arg(clipboardRow.entry.preview)
                                TapHandler {
                                    onTapped: root.clipboardPinToggled(
                                        clipboardRow.entry.index, !clipboardRow.entry.pinned)
                                }
                            }
                        }
                    }

                    TextLink {
                        objectName: "clipboardClearLink"
                        text: qsTr("Clear Clipboard")
                        accessibleName: qsTr("Clear Clipboard")
                        visible: root.clipboardEntries.length > 0
                        onActivated: root.clipboardClearRequested()
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
        target: bluetoothToggle
        property: "checked"
        value: root.bluetoothOn
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