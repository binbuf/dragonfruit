// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Appearance pane (T274). One card per macOS group, never a card per row:
//
//   Appearance              Light / Dark / Auto
//   Color                   accent + highlight (swatches, multicolor, custom)
//   Icon & widget style     icon style, folder color, sidebar icon size
//   Windows and scroll bars wallpaper tint, scroll bars, track click,
//                           transparency, motion link
//   Desktop integration     the Linux-only icon/cursor/window rows
//   Preview                 the live scheme/accent/highlight/material render
//
// Every settingsd-backed row is a `SettingsKeyRow`, so a rejected write
// reverts its control and shows the owner's message inline. A row with no
// consumer is disabled with the reason instead of pretending to be live.
//
// The preview is the rendered resolver: it reads the same `Theme` colours and
// the same `Glass.resolve` output the live surfaces use, not a second formula.
//
// Test and capture access (T290a): reach a control through a pane-body
// `property`/`property alias` declared below (for example `schemeRowHandle` or
// `schemeRow`), or through a row's `control.children[0]` /
// `controlSlot.controlItem`. Never alias an id declared inside an inline
// `component` in this file and never expect a function declared inside one to
// be visible through the instance: inline-component internals are not
// reachable from the instance.
Item {
    id: root

    signal navigationRequested(string paneId)

    readonly property var schemeOptions: [
        { value: "light", label: qsTr("Light") },
        { value: "dark", label: qsTr("Dark") },
        { value: "auto", label: qsTr("Auto") }
    ]

    // T274: the empty value is the macOS "Multicolor" choice: the active
    // scheme's token accent (system blue after the Tahoe review), with the
    // per-scheme accent still overridable. The rest are the primitive palette.
    readonly property var accentPalette: [
        { value: "", label: qsTr("Multicolor") },
        { value: "#7c5cff", label: qsTr("Violet") },
        { value: "#4a7dff", label: qsTr("Blue") },
        { value: "#2bb673", label: qsTr("Green") },
        { value: "#e0a43a", label: qsTr("Gold") },
        { value: "#e8556d", label: qsTr("Coral") }
    ]

    // T227: highlight is the selection wash (search fields, pickers), kept
    // separate from the accent. Empty is the scheme's token selection.
    readonly property var highlightPalette: [
        { value: "", label: qsTr("Default") },
        { value: "#b3a1ff", label: qsTr("Violet") },
        { value: "#7fb2ff", label: qsTr("Blue") },
        { value: "#7fd6a4", label: qsTr("Green") },
        { value: "#f0c46a", label: qsTr("Gold") },
        { value: "#ff8fa3", label: qsTr("Coral") }
    ]

    // T274: folder color is a Files/Theme consumer. Empty keeps the shipped
    // folder artwork; a colour replaces the generic folder glyph.
    readonly property var folderPalette: [
        { value: "", label: qsTr("Automatic") },
        { value: "#4a7dff", label: qsTr("Blue") },
        { value: "#7c5cff", label: qsTr("Violet") },
        { value: "#2bb673", label: qsTr("Green") },
        { value: "#e0a43a", label: qsTr("Gold") },
        { value: "#e8556d", label: qsTr("Coral") }
    ]

    readonly property var sidebarIconSizeOptions: [
        { value: "small", label: qsTr("Small") },
        { value: "medium", label: qsTr("Medium") },
        { value: "large", label: qsTr("Large") }
    ]
    readonly property var scrollBarOptions: [
        { value: "automatic", label: qsTr("Automatically") },
        { value: "when-scrolling", label: qsTr("When scrolling") },
        { value: "always", label: qsTr("Always") }
    ]
    readonly property var scrollBarClickOptions: [
        { value: "jump-to-next-page", label: qsTr("Jump to the next page") },
        { value: "jump-to-spot", label: qsTr("Jump to the spot that's clicked") }
    ]

    readonly property string currentAccent: Settings.values["appearance.accent"] || ""
    readonly property string currentHighlight:
        Settings.values["appearance.highlight"] || ""
    readonly property string currentFolderColor:
        Settings.values["appearance.folderColor"] || ""
    readonly property string currentSidebarIconSize:
        Settings.values["appearance.sidebarIconSize"] || "medium"
    readonly property string currentScrollBars:
        Settings.values["appearance.scrollBars"] || "automatic"
    readonly property string currentScrollBarClick:
        Settings.values["appearance.scrollBarClick"] || "jump-to-next-page"
    readonly property string currentIconStyle:
        Settings.values["appearance.iconStyle"] || "default"
    readonly property string currentIconTheme:
        Settings.values["appearance.iconTheme"] || ""
    readonly property string currentCursorTheme:
        Settings.values["appearance.cursorTheme"] || ""
    readonly property real currentCursorSize: {
        var value = Number(Settings.values["appearance.cursorSize"]);
        return isFinite(value) ? Math.max(16, Math.min(64, value)) : 24;
    }

    // T274: the real host icon/cursor theme inventory. The bridge's one
    // provider signal refreshes all three lists; a fixture swap (tests and
    // captures) therefore re-reads them without a second inventory model.
    property var iconStyleOptions: Settings.iconStyleOptions()
    property var iconThemeOptions: Settings.iconThemes()
    property var cursorThemeOptions: Settings.cursorThemes()

    function refreshThemeProviders() {
        root.iconStyleOptions = Settings.iconStyleOptions();
        root.iconThemeOptions = Settings.iconThemes();
        root.cursorThemeOptions = Settings.cursorThemes();
    }

    Connections {
        target: Settings
        function onIconThemesChanged() { root.refreshThemeProviders(); }
    }
    readonly property bool iconProviderAvailable: root.iconThemeOptions.length > 1
    readonly property bool cursorProviderAvailable: root.cursorThemeOptions.length > 1
    readonly property string iconProviderReason:
        qsTr("No icon theme is installed. Install an icon theme to use this row.")
    readonly property string cursorProviderReason:
        qsTr("No cursor theme is installed. Install a cursor theme to use this row.")
    readonly property string iconStyleUnavailable:
        root.iconStyleUnavailableNames.length > 0
            ? qsTr("%1 unavailable on this system.")
                  .arg(root.iconStyleUnavailableNames.join(", "))
            : ""
    readonly property var iconStyleUnavailableNames: {
        var names = [];
        var options = root.iconStyleOptions;
        for (var i = 0; i < options.length; ++i) {
            if (options[i].enabled === false)
                names.push(String(options[i].label));
        }
        return names;
    }
    property string iconStyleRejectedReason: ""

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

    // Test/capture surface: a long localized label proves the row elides and
    // keeps its control inside the card at the measured 740×820 window.
    property string labelSuffix: ""

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
    readonly property string previewScheme:
        Theme.dark ? qsTr("Dark") : qsTr("Light")
    readonly property string previewAccentLabel:
        root.currentAccent.length > 0 ? root.currentAccent
                                      : qsTr("Multicolor")
    readonly property string previewHighlightLabel:
        root.currentHighlight.length > 0 ? root.currentHighlight
                                         : qsTr("Default")
    readonly property string previewFolderLabel:
        root.currentFolderColor.length > 0 ? root.currentFolderColor
                                           : qsTr("Automatic")

    // Coalesce live drag writes: the knob and preview move on every `moved`,
    // the settings owner sees at most one write per timer window, and
    // `committed` always flushes the final value. A write is never accepted
    // while Reduce Transparency owns the result.
    function queueTransparency(value) {
        if (root.reduceTransparency)
            return;
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
        if (root.reduceTransparency) {
            // Reduce Transparency is the winner: snap the knob back to the
            // stored value instead of writing a value that cannot apply.
            root.previewTransparency = root.transparency;
            return;
        }
        Settings.set("appearance.transparency", root.pendingTransparency);
    }

    onTransparencyChanged: {
        if (!root.transparencyWritePending)
            root.previewTransparency = root.transparency;
    }
    onReduceTransparencyChanged: {
        if (root.reduceTransparency) {
            transparencyWriteTimer.stop();
            root.transparencyWritePending = false;
            root.previewTransparency = root.transparency;
        }
    }

    // Test surface (used by tst_settings_appearance.qml).
    //
    // Build canary (T290a): never user-visible and never translated. It exists
    // so a pane served from a stale compiled resource fails
    // `tst_settings_appearance` instead of silently passing. A stale build copy
    // is caught separately by `tst_settings_qml_copy_sync`.
    readonly property string sourceRevision: "t290a"

    // Test/capture accessor (T290a). Tests and captures reach a control
    // through a pane-body property or alias like this one, never through an id
    // declared inside an inline `component` (those are not reachable through
    // the instance).
    readonly property Item schemeRowHandle: schemeRow

    property alias schemeControl: schemeControl
    property alias schemeRow: schemeRow
    property alias accentRepeater: accentRepeater
    property alias accentRow: accentRow
    property alias accentPopup: accentPopup
    property alias accentInput: accentPopup.input
    property alias highlightRepeater: highlightRepeater
    property alias highlightRow: highlightRow
    property alias highlightPopup: highlightPopup
    property alias highlightInput: highlightPopup.input
    property alias folderColorRepeater: folderColorRepeater
    property alias folderColorRow: folderColorRow
    property alias folderColorPopup: folderColorPopup
    property alias folderColorInput: folderColorPopup.input
    property alias iconStyleRow: iconStyleRow
    property alias iconStyleControl: iconStyleControl
    property alias iconThemeRow: iconThemeRow
    property alias iconThemeSelect: iconThemeSelect
    property alias cursorThemeRow: cursorThemeRow
    property alias cursorThemeSelect: cursorThemeSelect
    property alias cursorSizeRow: cursorSizeRow
    property alias cursorSizeSlider: cursorSizeSlider
    property alias sidebarIconSizeRow: sidebarIconSizeRow
    property alias sidebarIconSizeSelect: sidebarIconSizeSelect
    property alias wallpaperTintRow: wallpaperTintRow
    property alias wallpaperTintToggle: wallpaperTintToggle
    property alias scrollBarsRow: scrollBarsRow
    property alias scrollBarsSelect: scrollBarsSelect
    property alias scrollBarClickRow: scrollBarClickRow
    property alias scrollBarClickSelect: scrollBarClickSelect
    property alias transparencySlider: transparencySlider
    property alias transparencyRow: transparencyRow
    property alias transparencyGroup: windowsGroup
    property alias windowsGroup: windowsGroup
    property alias linuxGroup: linuxGroup
    property alias windowControlsRow: windowControlsRow
    property alias materialRow: materialRow
    property alias accentFromWallpaperRow: accentFromWallpaperRow
    property alias previewPlate: previewPlate
    property alias previewGroup: previewGroup
    property alias previewWindow: previewWindow
    property alias previewAccentChip: previewAccentChip
    property alias previewHighlightChip: previewHighlightChip
    property alias previewFolderIcon: previewFolderIcon
    property alias motionRow: motionRow
    property alias motionButton: motionButton

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

    function optionIndex(options, value, fallback) {
        for (var i = 0; i < options.length; ++i) {
            if (options[i].value === value)
                return i;
        }
        return fallback;
    }

    function iconStyleIndex() {
        return root.optionIndex(root.iconStyleOptions, root.currentIconStyle, 0);
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
                label: qsTr("Appearance") + root.labelSuffix
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

        // macOS groups the two colour rows in one card. The highlight is a
        // first-class custom colour, not a fixed enum (T274).
        SettingsGroup {
            width: parent.width
            title: qsTr("Color")
            sectionId: "Color"

            SettingsKeyRow {
                id: accentRow
                width: parent.width
                label: qsTr("Accent color")
                settingsId: "accent"
                owner: "settingsd + compositor"
                settingsKey: "appearance.accent"

                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Repeater {
                        id: accentRepeater
                        model: root.accentPalette
                        delegate: ColorSwatch {
                            currentValue: root.currentAccent
                            fallbackColor: Theme.color.accent
                            multicolor: modelData.value === ""
                            onChosen: (value) => Settings.set("appearance.accent", value)
                        }
                    }

                    Button {
                        id: customAccentButton
                        text: qsTr("Custom…")
                        accessibleName: qsTr("Custom accent color")
                        onClicked: accentPopup.open ? accentPopup.hide() : accentPopup.show()
                    }
                }
            }

            SettingsKeyRow {
                id: highlightRow
                width: parent.width
                label: qsTr("Highlight color")
                settingsId: "highlight"
                owner: "settingsd + shell"
                settingsKey: "appearance.highlight"
                description: qsTr("The selection wash in search fields and pickers. "
                                  + "Default follows the active color scheme.")
                showSeparator: false

                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Repeater {
                        id: highlightRepeater
                        model: root.highlightPalette
                        delegate: ColorSwatch {
                            currentValue: root.currentHighlight
                            fallbackColor: Theme.color.selection
                            onChosen: (value) => Settings.set("appearance.highlight", value)
                        }
                    }

                    Button {
                        id: customHighlightButton
                        text: qsTr("Custom…")
                        accessibleName: qsTr("Custom highlight color")
                        onClicked: highlightPopup.open ? highlightPopup.hide()
                                                       : highlightPopup.show()
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Icon & widget style")
            sectionId: "Icon & widget style"

            SettingsKeyRow {
                id: iconStyleRow
                width: parent.width
                label: qsTr("Icon & widget style") + root.labelSuffix
                settingsId: "iconStyle"
                owner: "settingsd + icon-theme provider"
                settingsKey: "appearance.iconStyle"
                enabled: root.iconProviderAvailable
                description: !root.iconProviderAvailable
                    ? root.iconProviderReason
                    : (root.iconStyleRejectedReason.length > 0
                       ? root.iconStyleRejectedReason
                       : (root.iconStyleUnavailable.length > 0
                          ? qsTr("Maps to an installed icon theme. ") + root.iconStyleUnavailable
                          : qsTr("Maps to an installed icon theme.")))

                controlData: SegmentedControl {
                    id: iconStyleControl
                    model: root.iconStyleOptions
                    onActivated: (index) => Settings.set(
                                    "appearance.iconStyle",
                                    String(root.iconStyleOptions[index].value))
                    onRejected: (index, reason) => root.iconStyleRejectedReason = reason
                }
            }

            SettingsKeyRow {
                id: folderColorRow
                width: parent.width
                label: qsTr("Folder color")
                settingsId: "folderColor"
                owner: "settingsd + Files"
                settingsKey: "appearance.folderColor"
                description: qsTr("Tints the plain folder icon in Files and the Dock. "
                                  + "Automatic keeps the shipped folder artwork.")

                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Repeater {
                        id: folderColorRepeater
                        model: root.folderPalette
                        delegate: ColorSwatch {
                            currentValue: root.currentFolderColor
                            fallbackColor: Theme.color.textSecondary
                            onChosen: (value) => Settings.set("appearance.folderColor", value)
                        }
                    }

                    Button {
                        id: customFolderButton
                        text: qsTr("Custom…")
                        accessibleName: qsTr("Custom folder color")
                        onClicked: folderColorPopup.open ? folderColorPopup.hide()
                                                        : folderColorPopup.show()
                    }
                }
            }

            SettingsKeyRow {
                id: sidebarIconSizeRow
                width: parent.width
                label: qsTr("Sidebar icon size")
                settingsId: "sidebarIconSize"
                owner: "settingsd + apps/settings"
                settingsKey: "appearance.sidebarIconSize"
                showSeparator: false

                controlData: Select {
                    id: sidebarIconSizeSelect
                    accessibleName: qsTr("Sidebar icon size")
                    model: root.sidebarIconSizeOptions
                    onActivated: (index) => Settings.set(
                                    "appearance.sidebarIconSize",
                                    root.sidebarIconSizeOptions[index].value)
                }
            }
        }

        SettingsGroup {
            id: windowsGroup
            width: parent.width
            title: qsTr("Windows and scroll bars")
            sectionId: "Windows and scroll bars"

            SettingsKeyRow {
                id: wallpaperTintRow
                width: parent.width
                label: qsTr("Allow wallpaper tinting")
                settingsId: "wallpaperTint"
                owner: "settingsd + shell"
                settingsKey: "appearance.wallpaperTint"
                description: qsTr("Off uses the neutral glass plate instead of "
                                  + "sampling the wallpaper's colors.")
                controlData: Toggle {
                    id: wallpaperTintToggle
                    text: ""
                    onToggled: (checked) => Settings.set("appearance.wallpaperTint", checked)
                }
            }

            SettingsKeyRow {
                id: scrollBarsRow
                width: parent.width
                label: qsTr("Show scroll bars")
                settingsId: "scrollBars"
                owner: "settingsd + shell"
                settingsKey: "appearance.scrollBars"
                description: qsTr("Automatic shows bars while a view is focused, hovered, "
                                  + "or scrolling; When scrolling hides them at rest; "
                                  + "Always keeps them visible.")
                controlData: Select {
                    id: scrollBarsSelect
                    accessibleName: qsTr("Show scroll bars")
                    model: root.scrollBarOptions
                    onActivated: (index) => Settings.set(
                                    "appearance.scrollBars",
                                    root.scrollBarOptions[index].value)
                }
            }

            SettingsKeyRow {
                id: scrollBarClickRow
                width: parent.width
                label: qsTr("Click in the scroll bar to")
                settingsId: "scrollBarClick"
                owner: "settingsd + shell"
                settingsKey: "appearance.scrollBarClick"
                controlData: Select {
                    id: scrollBarClickSelect
                    accessibleName: qsTr("Click in the scroll bar to")
                    model: root.scrollBarClickOptions
                    onActivated: (index) => Settings.set(
                                    "appearance.scrollBarClick",
                                    root.scrollBarClickOptions[index].value)
                }
            }

            // T274: the transparency control is disabled while Reduce
            // Transparency owns the result, and the row says why. A drag that
            // was already in flight when the override turned on is discarded,
            // never written.
            SettingsKeyRow {
                id: transparencyRow
                width: parent.width
                label: qsTr("Attached-menu transparency")
                settingsId: "transparency"
                owner: "settingsd + shell"
                settingsKey: "appearance.transparency"
                enabled: !root.reduceTransparency
                description: root.reduceTransparency
                    ? qsTr("Unavailable while Reduce Transparency is on. Turn it off in "
                           + "Accessibility to use this slider.")
                    : qsTr("Light attached menus only; higher values are clearer. "
                           + "The Dock, menu bar, windows, and panels keep their own "
                           + "materials. Reduce Transparency overrides this slider.")
                controlData: Slider {
                    id: transparencySlider
                    width: 220
                    from: 0.0
                    to: 1.0
                    minLabel: qsTr("Opaque")
                    maxLabel: qsTr("Clear")
                    accessibleName: qsTr("Attached-menu transparency")
                    enabled: !root.reduceTransparency
                    onMoved: (value) => root.queueTransparency(value)
                    onCommitted: (value) => {
                        if (root.reduceTransparency) {
                            root.previewTransparency = root.transparency;
                            return;
                        }
                        root.pendingTransparency =
                            Math.round(Math.max(0, Math.min(1, value)) * 10000) / 10000;
                        root.transparencyWritePending = true;
                        root.flushTransparency();
                    }
                }
            }

            SettingsRow {
                id: motionRow
                width: parent.width
                label: qsTr("Motion")
                description: qsTr("Reduced motion is the Accessibility preference; "
                                  + "it disables window and control animation.")
                showSeparator: false

                controlData: Button {
                    id: motionButton
                    text: qsTr("Open Accessibility")
                    accessibleName: qsTr("Open Accessibility motion settings")
                    onClicked: root.navigationRequested("accessibility")
                }
            }
        }

        // The Linux-only rows. Each names its real owner. The icon rows are
        // backed by the host's installed themes; a host with no provider shows
        // a disabled row with the reason rather than silently omitting it.
        SettingsGroup {
            id: linuxGroup
            width: parent.width
            title: qsTr("Desktop integration")
            sectionId: "Desktop integration"

            SettingsKeyRow {
                id: iconThemeRow
                width: parent.width
                label: qsTr("Icon theme")
                settingsId: "iconTheme"
                owner: "settingsd + icon-theme provider"
                settingsKey: "appearance.iconTheme"
                enabled: root.iconProviderAvailable
                description: root.iconProviderAvailable
                    ? qsTr("Overrides the icon & widget style with a specific "
                           + "installed theme.")
                    : root.iconProviderReason
                controlData: Select {
                    id: iconThemeSelect
                    accessibleName: qsTr("Icon theme")
                    model: root.iconThemeOptions
                    onActivated: (index) => Settings.set(
                                    "appearance.iconTheme",
                                    String(root.iconThemeOptions[index].value))
                }
            }

            SettingsKeyRow {
                id: cursorThemeRow
                width: parent.width
                label: qsTr("Cursor theme")
                settingsId: "cursorTheme"
                owner: "settingsd + compositor"
                settingsKey: "appearance.cursorTheme"
                enabled: root.cursorProviderAvailable
                description: root.cursorProviderAvailable
                    ? qsTr("Applied the next time you sign in.")
                    : root.cursorProviderReason
                controlData: Select {
                    id: cursorThemeSelect
                    accessibleName: qsTr("Cursor theme")
                    model: root.cursorThemeOptions
                    onActivated: (index) => Settings.set(
                                    "appearance.cursorTheme",
                                    String(root.cursorThemeOptions[index].value))
                }
            }

            SettingsKeyRow {
                id: cursorSizeRow
                width: parent.width
                label: qsTr("Cursor size")
                settingsId: "cursorSize"
                owner: "settingsd + compositor"
                settingsKey: "appearance.cursorSize"
                description: qsTr("Applied the next time you sign in.")
                controlData: Slider {
                    id: cursorSizeSlider
                    width: 220
                    from: 16
                    to: 64
                    stepSize: 1
                    minLabel: qsTr("Small")
                    maxLabel: qsTr("Large")
                    accessibleName: qsTr("Cursor size")
                    onCommitted: (value) => Settings.set("appearance.cursorSize",
                                                         Math.round(value))
                }
            }

            SettingsRow {
                id: windowControlsRow
                width: parent.width
                label: qsTr("Window control layout")
                description: qsTr("Dragonfruit keeps the traffic lights on the left. "
                                  + "Moving them is not available yet.")
                settingsId: "windowControls"
                owner: "compositor/window decoration"
                settingsKey: "appearance.windowControls"
                controlData: Select {
                    model: [{ value: "left", label: qsTr("Left") }]
                    enabled: false
                    accessibleName: qsTr("Window control layout")
                }
            }

            SettingsRow {
                id: materialRow
                width: parent.width
                label: qsTr("Material effects")
                description: qsTr("Dragonfruit chooses the glass detail automatically "
                                  + "from the graphics hardware; there is no manual "
                                  + "override.")
                settingsId: "material"
                owner: "shell/design-system Glass"
                settingsKey: "appearance.material"
                controlData: Select {
                    model: [{ value: "auto", label: qsTr("Automatic") }]
                    enabled: false
                    accessibleName: qsTr("Material effects")
                }
            }

            SettingsRow {
                id: accentFromWallpaperRow
                width: parent.width
                label: qsTr("Accent from wallpaper")
                description: qsTr("Needs the wallpaper color sampler; use a "
                                  + "custom accent above until it is available.")
                settingsId: "accentFromWallpaper"
                owner: "shell/design-system Theme"
                settingsKey: "appearance.accentFromWallpaper"
                controlData: Toggle {
                    checked: false
                    enabled: false
                    accessibleName: qsTr("Accent from wallpaper")
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
            sectionId: "Preview"

            Item {
                width: parent.width
                height: 238

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
                    id: previewWindow
                    anchors.fill: parent
                    anchors.margins: Theme.primitive.spacing.sm
                    radius: Theme.controls.window.radius
                    color: Theme.color.surface
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border
                    clip: true

                    // Sidebar: the accent selection wash and the scheme's
                    // surface/ink pair.
                    Rectangle {
                        id: previewSidebar
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: 92
                        color: Theme.color.surfaceMuted

                        Column {
                            anchors.fill: parent
                            anchors.margins: Theme.primitive.spacing.sm
                            spacing: Theme.primitive.spacing.xs

                            Rectangle {
                                width: parent.width
                                height: 22
                                radius: Theme.primitive.radius.sm
                                color: Theme.color.accentMuted

                                Text {
                                    anchors.left: parent.left
                                    anchors.leftMargin: Theme.primitive.spacing.xs
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: qsTr("Appearance")
                                    color: Theme.color.accent
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    font.weight: Theme.primitive.font.weightMedium
                                }
                            }
                            Repeater {
                                model: [qsTr("Color"), qsTr("Sidebar"), qsTr("Scroll bars")]
                                delegate: Text {
                                    required property string modelData
                                    text: modelData
                                    color: Theme.color.textSecondary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                }
                            }
                        }
                    }

                    // Content: highlight wash, accent action, folder glyph,
                    // and the material plate in one place.
                    Column {
                        anchors.left: previewSidebar.right
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: Theme.primitive.spacing.sm
                        anchors.leftMargin: Theme.primitive.spacing.md
                        spacing: Theme.primitive.spacing.sm

                        Text {
                            text: qsTr("Preview")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            font.weight: Theme.primitive.font.weightSemibold
                        }

                        // A search field with a selected word: the highlight
                        // wash is the live Theme selection colour.
                        Rectangle {
                            width: parent.width
                            height: 24
                            radius: Theme.primitive.radius.sm
                            color: Theme.color.controlFill
                            border.width: Theme.controls.window.borderWidth
                            border.color: Theme.color.border

                            Rectangle {
                                id: previewHighlightChip
                                x: Theme.primitive.spacing.xs
                                anchors.verticalCenter: parent.verticalCenter
                                width: 46
                                height: 16
                                radius: Theme.primitive.radius.xs
                                color: Theme.color.selection
                            }
                            Text {
                                anchors.left: previewHighlightChip.right
                                anchors.leftMargin: Theme.primitive.spacing.xs
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                text: qsTr("selected")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeXs
                                elide: Text.ElideRight
                            }
                        }

                        Row {
                            spacing: Theme.primitive.spacing.sm

                            Rectangle {
                                id: previewAccentChip
                                width: 64
                                height: 22
                                radius: Theme.controls.button.radius
                                color: Theme.color.accent

                                Text {
                                    anchors.centerIn: parent
                                    text: qsTr("Accent")
                                    color: Theme.color.accentContent
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    font.weight: Theme.primitive.font.weightMedium
                                }
                            }

                            Icon {
                                id: previewFolderIcon
                                name: "folder"
                                size: 20
                                color: Theme.hasFolderColor ? Theme.folderColorColor
                                                           : Theme.color.textPrimary
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: qsTr("Folder")
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeXs
                            }
                        }

                        // The material plate: the rendered resolver output,
                        // not a second alpha formula.
                        Rectangle {
                            id: previewPlate
                            width: parent.width
                            height: 48
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
                                anchors.leftMargin: Theme.primitive.spacing.sm
                                anchors.right: parent.right
                                anchors.rightMargin: Theme.primitive.spacing.sm
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 0

                                Text {
                                    width: parent.width
                                    text: qsTr("Attached menu")
                                    color: Theme.color.textPrimary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    font.weight: Theme.primitive.font.weightMedium
                                    elide: Text.ElideRight
                                }
                                Text {
                                    objectName: "appearancePreviewMode"
                                    width: parent.width
                                    text: root.previewMode + " · " + root.previewPercent
                                    color: Theme.color.textSecondary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        Text {
                            width: parent.width
                            text: qsTr("%1 scheme · %2 accent")
                                  .arg(root.previewScheme)
                                  .arg(root.previewAccentLabel)
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeXs
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: qsTr("%1 highlight · %2 folders · %3 icons")
                                  .arg(root.previewHighlightLabel)
                                  .arg(root.previewFolderLabel)
                                  .arg(root.currentIconStyle)
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeXs
                            elide: Text.ElideRight
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
    Binding {
        target: sidebarIconSizeSelect
        property: "currentIndex"
        value: root.optionIndex(root.sidebarIconSizeOptions,
                                root.currentSidebarIconSize, 1)
    }
    Binding {
        target: wallpaperTintToggle
        property: "checked"
        value: Settings.values["appearance.wallpaperTint"] !== false
    }
    Binding {
        target: scrollBarsSelect
        property: "currentIndex"
        value: root.optionIndex(root.scrollBarOptions, root.currentScrollBars, 0)
    }
    Binding {
        target: scrollBarClickSelect
        property: "currentIndex"
        value: root.optionIndex(root.scrollBarClickOptions,
                                root.currentScrollBarClick, 0)
    }
    Binding {
        target: iconStyleControl
        property: "currentIndex"
        value: root.iconStyleIndex()
    }
    Binding {
        target: iconThemeSelect
        property: "currentIndex"
        value: root.optionIndex(root.iconThemeOptions, root.currentIconTheme, 0)
    }
    Binding {
        target: cursorThemeSelect
        property: "currentIndex"
        value: root.optionIndex(root.cursorThemeOptions, root.currentCursorTheme, 0)
    }
    Binding {
        target: cursorSizeSlider
        property: "value"
        value: root.currentCursorSize
    }

    // Three custom-color popups share one hex-entry anatomy. The popup owns
    // validation and reports an applied value; the row owns the settings write.
    component ColorPickerPopup: Popup {
        id: picker

        property string initial: ""
        property alias input: hexInput
        signal applied(string value)

        anchorItem: null
        preferredWidth: 220
        accessibleName: qsTr("Custom color")

        function acceptableText(text) {
            return /^#[0-9a-fA-F]{6}$/.test((text || "").trim());
        }

        function applyCustom() {
            if (picker.acceptableText(hexInput.text)) {
                picker.applied(hexInput.text.trim().toLowerCase());
                picker.hide();
            }
        }

        onOpened: hexInput.text = picker.initial

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
                color: hexInput.acceptable ? hexInput.text : Theme.color.controlFill
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
                border.color: hexInput.acceptable ? Theme.color.border : Theme.color.danger

                TextInput {
                    id: hexInput
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: picker.applyCustom()
                    readonly property bool acceptable: picker.acceptableText(text)

                    function applyCustom() {
                        picker.applyCustom();
                    }
                }
            }
        }

        Text {
            width: parent.width
            visible: hexInput.text.length > 0 && !hexInput.acceptable
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
            enabled: hexInput.acceptable
            onClicked: picker.applyCustom()
        }
    }

    ColorPickerPopup {
        id: accentPopup
        anchorItem: customAccentButton
        initial: root.currentAccent
        accessibleName: qsTr("Custom accent color")
        onApplied: (value) => Settings.set("appearance.accent", value)
    }

    ColorPickerPopup {
        id: highlightPopup
        anchorItem: customHighlightButton
        initial: root.currentHighlight
        accessibleName: qsTr("Custom highlight color")
        onApplied: (value) => Settings.set("appearance.highlight", value)
    }

    ColorPickerPopup {
        id: folderColorPopup
        anchorItem: customFolderButton
        initial: root.currentFolderColor
        accessibleName: qsTr("Custom folder color")
        onApplied: (value) => Settings.set("appearance.folderColor", value)
    }

    // One color choice. Selected state follows the settings value, so an
    // external change repaints the row too. The empty value is the default:
    // for the accent it draws the macOS multicolor wheel.
    component ColorSwatch: Item {
        id: swatch

        required property var modelData
        property string currentValue: ""
        property color fallbackColor: Theme.color.accent
        property bool multicolor: false
        signal chosen(string value)

        readonly property bool selected: swatch.currentValue === swatch.modelData.value

        width: 20
        height: 20
        activeFocusOnTab: true

        function choose() {
            swatch.chosen(swatch.modelData.value);
        }

        Rectangle {
            anchors.centerIn: parent
            width: 16
            height: 16
            radius: width / 2
            visible: !swatch.multicolor
            color: swatch.modelData.value === "" ? swatch.fallbackColor
                                                 : swatch.modelData.value
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Canvas {
            anchors.centerIn: parent
            width: 16
            height: 16
            visible: swatch.multicolor
            onPaint: {
                var ctx = getContext("2d");
                ctx.clearRect(0, 0, width, height);
                var colors = ["#ff3b30", "#ff9500", "#ffcc00",
                              "#34c759", "#007aff", "#af52de"];
                var cx = width / 2;
                var cy = height / 2;
                var radius = width / 2;
                for (var i = 0; i < colors.length; ++i) {
                    ctx.beginPath();
                    ctx.moveTo(cx, cy);
                    ctx.arc(cx, cy, radius, i * Math.PI / 3, (i + 1) * Math.PI / 3);
                    ctx.closePath();
                    ctx.fillStyle = colors[i];
                    ctx.fill();
                }
            }
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
