// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Menu Bar pane (T-15.9b).
//
// The pane and the Control Center Menu Bar tile are one functional unit. The
// menu bar is shell-native: the shell's `MenuBar` (`shell/menubar/MenuBar.qml`)
// owns the chrome, the status row, and the clock, the menu-broker
// (`services/menu-broker`) resolves the focused app's menus, and settingsd
// owns the durable preferences. This pane is the *configuration* half and
// writes only settingsd keys, so every control applies live and persists:
//
//   * `menu.autoHide`, `menu.showBackground`, and `menu.recentItems` are the
//     behavior rows;
//   * `menu.clock.showDate` / `menu.clock.showSeconds` are the `Clock
//     Options...` rows;
//   * `menu.control.<id>` gates each status item. The shell applies it in
//     `applyStatusItems` (additive with the adapter's own availability), so a
//     control shows only when its daemon is present *and* the user wants it.
//
// The adapter (`dragonfruit-menubar-adapter`, T-15.9a) stays read-only; the
// durable preference is the single source of truth the shell summarizes in the
// Control Center tile.
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * `AirDrop`, `Screen Mirroring`, and `Display` are Apple-only and have no
//     Linux equivalent, so they are omitted (the adapter omits them too).
//   * `Spotlight` is our deferred Search pane, so it is not a menu-bar control
//     here.
//   * `Add Controls...`, `Battery Options...`, and the per-control popups
//     (`Show When Active`) have no Linux provider yet, so they are omitted
//     rather than shipped as dead controls.
//   * the `Recent documents...` stepper is drawn as a popup (`Select`) because
//     the design system has no stepper component; the value is stored policy
//     the recent-items consumer will read.
//   * the Apple Account/sidebar row and the trailing `?` help are omitted
//     project-wide.
Item {
    id: root

    // ── The behavior keys (revision 16, ADR 0135) ───────────────────────
    readonly property string autoHide:
        Settings.values["menu.autoHide"] !== undefined
            ? String(Settings.values["menu.autoHide"]) : "full-screen"
    readonly property bool showBackground:
        Settings.values["menu.showBackground"] !== false
    readonly property int recentItems:
        Settings.values["menu.recentItems"] !== undefined
            ? Number(Settings.values["menu.recentItems"]) : 10

    // ── The clock option keys ───────────────────────────────────────────
    readonly property bool clockShowDate:
        Settings.values["menu.clock.showDate"] !== false
    readonly property bool clockShowSeconds:
        Settings.values["menu.clock.showSeconds"] === true

    // ── The OS menu glyph (revision 26) ─────────────────────────────────
    readonly property string osGlyph:
        Settings.values["menu.osGlyph"] !== undefined
            ? String(Settings.values["menu.osGlyph"]) : "dragonfruit"

    // The two shipped marks. Each preview renders through the same
    // `SystemMenuGlyph` the shell uses, at the menu bar's real icon size, so
    // the choice shows the foreground, wallpaper shadow and hover/open plate
    // exactly as the bar will.
    readonly property var glyphOptions: [
        { value: "dragonfruit", label: qsTr("Dragonfruit"),
          description: qsTr("The Dragonfruit mark.") },
        { value: "tux", label: qsTr("Tux"),
          description: qsTr("The Linux penguin mark.") }
    ]

    // The auto-hide popup choices, in the adapter's `ALL` order.
    readonly property var autoHideOptions: [
        { value: "never", label: qsTr("Never") },
        { value: "always", label: qsTr("Always") },
        { value: "full-screen", label: qsTr("In Full Screen Only") }
    ]

    // The recent-items popup choices; `None` disables the history.
    readonly property var recentOptions: [
        { value: 0, label: qsTr("None") },
        { value: 5, label: qsTr("5") },
        { value: 10, label: qsTr("10") },
        { value: 15, label: qsTr("15") },
        { value: 20, label: qsTr("20") },
        { value: 30, label: qsTr("30") },
        { value: 50, label: qsTr("50") }
    ]

    // The Menu Bar Controls list: every live `menu.control.*` key, in the
    // registry's reference order. The key gates the shell's status row; an
    // item that does not fit moves into the overflow menu instead of being
    // clipped (T233).
    readonly property var controlRows: [
        { key: "menu.control.wifi", label: qsTr("Wi-Fi") },
        { key: "menu.control.bluetooth", label: qsTr("Bluetooth") },
        { key: "menu.control.battery", label: qsTr("Battery") },
        { key: "menu.control.volume", label: qsTr("Sound") },
        { key: "menu.control.search", label: qsTr("Search") },
        { key: "menu.control.notifications", label: qsTr("Notifications") },
        { key: "menu.control.focus", label: qsTr("Focus") },
        { key: "menu.control.clock", label: qsTr("Date & Time") },
        { key: "menu.control.accessibility", label: qsTr("Accessibility") },
        { key: "menu.control.controlCenter", label: qsTr("Quick Settings") }
    ]

    function controlVisible(key) {
        return Settings.values[key] !== false;
    }

    // Test surface (used by tst_settings_menu_bar.qml).
    property alias autoHideSelect: autoHideSelect
    property alias backgroundToggle: backgroundToggle
    property alias recentSelect: recentSelect
    property alias clockOptionsButton: clockOptionsButton
    property alias clockDialog: clockDialog
    property alias clockDateToggle: clockDateToggle
    property alias clockSecondsToggle: clockSecondsToggle
    property alias glyphRepeater: glyphRepeater
    property alias controlsRepeater: controlsRepeater
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function indexFor(options, value) {
        for (var i = 0; i < options.length; ++i) {
            if (Number(options[i].value) === Number(value))
                return i;
        }
        return options.length - 1;
    }

    function autoHideIndex(value) {
        for (var i = 0; i < root.autoHideOptions.length; ++i) {
            if (root.autoHideOptions[i].value === value)
                return i;
        }
        return root.autoHideOptions.length - 1;
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !Settings.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "menuBarAbsenceNote"
                kind: "live-on-defaults"
                showWhenLive: true
                reason: qsTr("The settings daemon is not running, so these "
                             + "preferences apply from the built-in defaults. "
                             + "The menu bar itself is drawn by the shell.")
            }
        }

        // ── Behavior ──────────────────────────────────────────────────────
        SettingsGroup {
            id: behaviorGroup
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Automatically hide and show the menu bar")
                controlData: Select {
                    id: autoHideSelect
                    accessibleName: qsTr("Automatically hide and show the menu bar")
                    model: root.autoHideOptions
                    onActivated: (index) => Settings.set(
                        "menu.autoHide", root.autoHideOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show menu bar background")
                controlData: Toggle {
                    id: backgroundToggle
                    accessibleName: qsTr("Show menu bar background")
                    onToggled: (checked) => Settings.set(
                        "menu.showBackground", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Recent documents, applications, and servers")
                showSeparator: false
                controlData: Select {
                    id: recentSelect
                    accessibleName: qsTr("Recent documents, applications, and servers")
                    model: root.recentOptions
                    onActivated: (index) => Settings.set(
                        "menu.recentItems", root.recentOptions[index].value)
                }
            }
        }

        // ── OS Menu Glyph ─────────────────────────────────────────────────
        SettingsGroup {
            id: glyphGroup
            width: parent.width
            title: qsTr("OS Menu Glyph")

            Text {
                objectName: "menuBarGlyphDescription"
                width: parent.width
                text: qsTr("Choose the mark at the far left of the menu bar. "
                           + "Each preview is shown at the menu bar's real "
                           + "size with the same shadow and hover treatment.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: glyphRepeater
                model: root.glyphOptions

                delegate: GlyphChoiceRow {
                    required property int index

                    width: parent.width
                    showSeparator: index < glyphRepeater.count - 1
                }
            }
        }

        // ── Menu Bar Controls ─────────────────────────────────────────────
        SettingsGroup {
            id: controlsGroup
            width: parent.width
            title: qsTr("Menu Bar Controls")

            Text {
                objectName: "menuBarControlsDescription"
                width: parent.width
                text: qsTr("System and app controls can be configured to "
                           + "appear in both Quick Settings and the menu bar.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                objectName: "menuBarOverflowDescription"
                width: parent.width
                text: qsTr("Items that do not fit move into the overflow "
                           + "menu; they are never clipped. Only the active "
                           + "display owns the application menu.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Clock")
                controlData: Button {
                    id: clockOptionsButton
                    objectName: "menuBarClockOptionsButton"
                    text: qsTr("Clock Options\u2026")
                    onClicked: clockDialog.show()
                }
            }

            Repeater {
                id: controlsRepeater
                model: root.controlRows

                delegate: SettingsRow {
                    required property var modelData
                    required property int index

                    property alias toggle: controlToggle
                    width: parent.width
                    label: modelData.label
                    showSeparator: index < controlsRepeater.count - 1
                    controlData: Toggle {
                        id: controlToggle
                        objectName: "menuBarControlToggle"
                        accessibleName: modelData.label
                        onToggled: (checked) => Settings.set(modelData.key,
                                                             checked)
                    }

                    Binding {
                        target: controlToggle
                        property: "checked"
                        value: root.controlVisible(modelData.key)
                    }
                }
            }
        }
    }

    // The `Clock Options...` sheet: the two clock options the adapter and the
    // shell's clock consume.
    Dialog {
        id: clockDialog
        objectName: "menuBarClockDialog"
        title: qsTr("Clock Options")
        message: qsTr("Choose what the menu-bar clock shows.")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.md

            SettingsRow {
                width: parent.width
                label: qsTr("Show date")
                controlData: Toggle {
                    id: clockDateToggle
                    accessibleName: qsTr("Show date")
                    onToggled: (checked) => Settings.set(
                        "menu.clock.showDate", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Display the time with seconds")
                showSeparator: false
                controlData: Toggle {
                    id: clockSecondsToggle
                    accessibleName: qsTr("Display the time with seconds")
                    onToggled: (checked) => Settings.set(
                        "menu.clock.showSeconds", checked)
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: clockDialog.accept()
            }
        }
    }

    // Keep the controls in step with external changes.
    Binding {
        target: autoHideSelect
        property: "currentIndex"
        value: root.autoHideIndex(root.autoHide)
    }
    Binding {
        target: backgroundToggle
        property: "checked"
        value: root.showBackground
    }
    Binding {
        target: recentSelect
        property: "currentIndex"
        value: root.indexFor(root.recentOptions, root.recentItems)
    }
    Binding {
        target: clockDateToggle
        property: "checked"
        value: root.clockShowDate
    }
    Binding {
        target: clockSecondsToggle
        property: "checked"
        value: root.clockShowSeconds
    }

    // One OS-menu glyph choice: a radio row with a real-size menu-bar preview.
    // The preview uses the same `SystemMenuGlyph` and wallpaper-contrast shadow
    // as the bar, so the row is a faithful miniature rather than a generic
    // icon tile.
    component GlyphChoiceRow: Item {
        id: choice

        required property var modelData
        property bool showSeparator: true

        readonly property bool selected:
            root.osGlyph === choice.modelData.value
        readonly property bool lightLettering: {
            var c = Theme.color.textPrimary;
            return (0.299 * c.r + 0.587 * c.g + 0.114 * c.b) > 0.5;
        }
        readonly property color labelShadowColor:
            choice.lightLettering ? Theme.controls.wallpaperText.shadowColorOnDark
                                  : Theme.controls.wallpaperText.shadowColorOnLight
        readonly property real labelShadowOpacity:
            choice.lightLettering ? Theme.controls.wallpaperText.shadowOpacityOnDark
                                  : Theme.controls.wallpaperText.shadowOpacityOnLight
        readonly property color glyphShadowColor:
            Qt.rgba(choice.labelShadowColor.r, choice.labelShadowColor.g,
                    choice.labelShadowColor.b, choice.labelShadowOpacity * 0.9)

        function choose() {
            Settings.set("menu.osGlyph", choice.modelData.value);
        }

        width: parent ? parent.width : implicitWidth
        implicitHeight: Theme.controls.settingsRow.height
        activeFocusOnTab: true

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.settingsGroup.radius
            color: Qt.rgba(Theme.color.textPrimary.r, Theme.color.textPrimary.g,
                           Theme.color.textPrimary.b, 0.05)
            visible: choiceHover.hovered
            antialiasing: true
        }

        // Radio indicator, left-aligned like the platform's choice rows.
        Rectangle {
            id: radioOuter
            x: Theme.controls.settingsRow.paddingH
            anchors.verticalCenter: parent.verticalCenter
            width: 16
            height: 16
            radius: width / 2
            color: "transparent"
            border.width: Theme.controls.window.borderWidth
            border.color: choice.selected ? Theme.color.accent : Theme.color.textTertiary
            antialiasing: true

            Rectangle {
                anchors.centerIn: parent
                width: 8
                height: 8
                radius: width / 2
                color: Theme.color.accent
                visible: choice.selected
                antialiasing: true
            }
        }

        Column {
            id: choiceLabelColumn
            anchors.left: radioOuter.right
            anchors.leftMargin: Theme.primitive.spacing.md
            anchors.right: previewPlate.left
            anchors.rightMargin: Theme.primitive.spacing.md
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.primitive.spacing.xxs

            Text {
                width: parent.width
                text: choice.modelData.label
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                text: choice.modelData.description
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                elide: Text.ElideRight
            }
        }

        // The menu-bar sample. It is exactly `menuBar.height` tall and renders
        // the glyph at `menuBar.iconSize`, with the same neutral hover wash the
        // bar paints behind the system menu.
        Rectangle {
            id: previewPlate
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.settingsRow.paddingH
            anchors.verticalCenter: parent.verticalCenter
            width: 112
            height: Theme.controls.menuBar.height
            radius: Theme.controls.menuBar.radius
            color: Theme.color.chrome
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true

            Rectangle {
                anchors.centerIn: parent
                width: Theme.controls.menuBar.iconSize
                        + 2 * Theme.controls.menuBarMenu.barPaddingH
                height: Theme.controls.menuBarMenu.barHeight
                radius: Theme.controls.focusRing.radius
                color: Qt.rgba(Theme.color.textPrimary.r,
                               Theme.color.textPrimary.g,
                               Theme.color.textPrimary.b,
                               Theme.controls.menuBar.hoverOpacity)
                visible: choiceHover.hovered || choice.activeFocus
                antialiasing: true
            }

            SystemMenuGlyph {
                anchors.centerIn: parent
                glyph: choice.modelData.value
                size: Theme.controls.menuBar.iconSize
                color: Theme.color.textPrimary
                shadowColor: choice.glyphShadowColor
                shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)
            }
        }

        Rectangle {
            visible: choice.showSeparator
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.settingsRow.paddingH
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.settingsRow.paddingH
            anchors.bottom: parent.bottom
            height: Theme.controls.window.borderWidth
            color: Theme.color.separator
        }

        HoverHandler { id: choiceHover }

        TapHandler {
            onTapped: choice.choose()
        }

        FocusRing {
            target: choice
            shown: choice.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                choice.choose();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: choice.modelData.label
        Accessible.description: choice.modelData.description
        Accessible.checkable: true
        Accessible.checked: choice.selected
        Accessible.focusable: true
        Accessible.onPressAction: choice.choose()
    }
}