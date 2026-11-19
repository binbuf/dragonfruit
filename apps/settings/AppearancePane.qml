// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Appearance pane (T274). One card per macOS group, never a card per row:
//
//   Appearance              Light / Dark / Auto
//   Glass                   the rendered attached-menu preview + tint slider
//   Theme                   accent swatches, Text highlight popup, icon-style
//                           tiles, Folder color popup, sidebar icon size
//   Windows and scroll bars wallpaper tint, scroll bars, track click,
//                           motion link
//   Desktop integration     the Linux-only icon/cursor/window rows
//
// T290d: the old `Color` and `Icon & widget style` cards are one `Theme` card
// in the mockup order. The accent keeps its swatch row and custom-hex popup;
// highlight and folder color are one swatch popup each; icon style is four
// `SettingsModeTile`s over the four `appearance.iconStyle` values.
//
// Every settingsd-backed row is a `SettingsKeyRow`, so a rejected write
// reverts its control and shows the owner's message inline. A row with no
// consumer is disabled with the reason instead of pretending to be live.
//
// The Glass preview is the rendered resolver: it reads the same `Theme`
// colours and the same `Glass.resolve` output the live surfaces use, not a
// second formula. The tint slider still writes `appearance.transparency`,
// which only the light attached-menu preset follows.
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
    readonly property string sourceRevision: "t290d"

    // Test/capture accessor (T290a). Tests and captures reach a control
    // through a pane-body property or alias like this one, never through an id
    // declared inside an inline `component` (those are not reachable through
    // the instance).
    readonly property Item schemeRowHandle: schemeRow
    readonly property Item schemeTiles: schemeTilesRow
    property alias schemeTileRepeater: schemeTileRepeater

    property alias schemeRow: schemeRow
    property alias themeGroup: themeGroup
    property alias accentRepeater: accentRepeater
    property alias accentRow: accentRow
    property alias accentPopup: accentPopup
    property alias accentInput: accentPopup.input
    property alias highlightRow: highlightRow
    property alias highlightSelect: highlightSelect
    property alias highlightSelectPopup: highlightChoicePopup
    property alias iconStyleTiles: iconStyleTiles
    property alias iconStyleTileRepeater: iconStyleTileRepeater
    property alias iconStyleRow: iconStyleRow
    property alias folderColorRow: folderColorRow
    property alias folderColorSelect: folderColorSelect
    property alias folderColorSelectPopup: folderColorChoicePopup
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
    property alias glassGroup: glassGroup
    property alias glassPreview: glassPreview
    property alias previewPlate: previewPlate
    property alias motionRow: motionRow
    property alias motionButton: motionButton

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot: the shell's Loader is the content width, so
    // the card tracks it instead of keeping the 480 design width and clipping
    // its right-hand controls. A standalone instance keeps the implicit width.
    width: parent ? parent.width : implicitWidth

    // T290b: the live scheme key is the single source of truth for the tile
    // ring. An external settingsd write and a tile click both read this.
    readonly property string currentScheme:
        Settings.values["appearance.colorScheme"] || "auto"

    // Index of the active scheme in the reference order; unknown/auto falls to
    // the Auto tile.
    function schemeIndex() {
        for (var i = 0; i < root.schemeOptions.length; ++i) {
            if (root.schemeOptions[i].value === root.currentScheme)
                return i;
        }
        return root.schemeOptions.length - 1;
    }

    // The three tiles share the control slot, which is capped at 60% of the
    // row. Derive one tile width from the row (never from `parent.width`
    // inside a layout) so all three stay inside the card at 640×640 and at
    // the measured 740×820 window. The token width is the upper bound.
    readonly property real schemeTileWidth: {
        var available = schemeRow.width * 0.6
                - 2 * Theme.primitive.spacing.sm;
        var perTile = available / root.schemeOptions.length;
        return Math.max(48, Math.min(Theme.controls.settingsModeTile.width,
                                   perTile));
    }
    readonly property real schemeTileRowHeight:
        schemeTilesRow.implicitHeight + 2 * Theme.primitive.spacing.sm

    // The thumbnails use the design-system neutral tokens, not the mockup's
    // CSS hex values: Light is the lightest plate, Dark the darkest, Auto
    // follows the live scheme's muted surface.
    function schemePreviewFill(value) {
        if (value === "light")
            return Theme.primitive.color.neutral0;
        if (value === "dark")
            return Theme.primitive.color.neutral900;
        return Theme.dark ? Theme.primitive.color.neutral800
                          : Theme.primitive.color.neutral50;
    }

    function schemePreviewAccent(value) {
        return value === "dark" ? Theme.primitive.color.azure400
                                : Theme.color.accent;
    }

    // T290d: the four icon-style tiles share the same capped control slot as
    // the scheme row. Size each tile from the row (never from `parent.width`
    // inside a Row) and let the row derive its height from the tiles.
    readonly property real iconStyleTileWidth: {
        var count = Math.max(1, root.iconStyleOptions.length);
        var available = iconStyleRow.width * 0.6
                - (count - 1) * Theme.primitive.spacing.sm;
        var perTile = available / count;
        return Math.max(44, Math.min(Theme.controls.settingsModeTile.width,
                                   perTile));
    }
    readonly property real iconStyleTileRowHeight:
        iconStyleTiles.implicitHeight + 2 * Theme.primitive.spacing.sm

    // The tile thumbnails are the design-system palette: Default is the light
    // plate with the live accent, Dark the dark plate, Clear the transparent
    // plate, Tinted the accent wash. They are the four `appearance.iconStyle`
    // values, never the installed theme list.
    function iconStylePreviewFill(value) {
        if (value === "dark")
            return Theme.primitive.color.neutral900;
        if (value === "clear")
            return Theme.primitive.color.neutral0;
        if (value === "tinted")
            return Theme.color.accentMuted;
        return Theme.primitive.color.neutral100;
    }

    function iconStylePreviewAccent(value) {
        if (value === "dark")
            return Theme.primitive.color.azure400;
        if (value === "clear")
            return Theme.primitive.color.neutral400;
        return Theme.color.accent;
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

    // Left/Right move to the next available style, matching the segmented
    // control this replaces. Unavailable styles are skipped, not written.
    function iconStyleMove(delta, fromIndex) {
        var count = root.iconStyleOptions.length;
        if (count === 0)
            return;
        var index = fromIndex;
        for (var step = 0; step < count; ++step) {
            index = (index + delta + count) % count;
            var option = root.iconStyleOptions[index];
            if (option && option.enabled !== false) {
                var tile = iconStyleTileRepeater.itemAt(index);
                if (tile)
                    tile.forceActiveFocus();
                root.iconStyleRejectedReason = "";
                Settings.set("appearance.iconStyle", String(option.value));
                return;
            }
        }
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
                // The tile row is taller than the 33 pt token. Deriving the
                // row height from the control keeps the group body and the
                // centered label in step without `fixedRowHeight`.
                rowHeight: root.schemeTileRowHeight

                controlData: Row {
                    id: schemeTilesRow
                    spacing: Theme.primitive.spacing.sm

                    Accessible.role: Accessible.Grouping
                    Accessible.name: qsTr("Appearance")

                    Repeater {
                        id: schemeTileRepeater
                        model: root.schemeOptions

                        delegate: SettingsModeTile {
                            required property var modelData
                            required property int index

                            width: root.schemeTileWidth
                            label: modelData.label
                            previewFill: root.schemePreviewFill(modelData.value)
                            previewAccent: root.schemePreviewAccent(modelData.value)
                            selected: root.currentScheme === modelData.value
                            onClicked: Settings.set("appearance.colorScheme",
                                                    modelData.value)
                        }
                    }

                    // Left/Right are the radio-group movement keys. The tiles
                    // are already Tab stops; this also matches the shared
                    // SettingsRadioGroup behavior. The tile accepts
                    // Return/Enter/Space itself, so only arrows reach here.
                    Keys.onPressed: (event) => {
                        if (event.key !== Qt.Key_Left
                                && event.key !== Qt.Key_Right)
                            return;
                        var delta = event.key === Qt.Key_Right ? 1 : -1;
                        var index = root.schemeIndex();
                        for (var i = 0; i < schemeTileRepeater.count; ++i) {
                            var candidate = schemeTileRepeater.itemAt(i);
                            if (candidate && candidate.activeFocus) {
                                index = i;
                                break;
                            }
                        }
                        var next = Math.max(0, Math.min(
                            root.schemeOptions.length - 1, index + delta));
                        var tile = schemeTileRepeater.itemAt(next);
                        if (tile) {
                            tile.forceActiveFocus();
                            Settings.set("appearance.colorScheme",
                                         root.schemeOptions[next].value);
                        }
                        event.accepted = true;
                    }
                }
            }
        }

        // T290c: the Glass card is the preview. The plate is the rendered
        // `Glass.resolve("menuAttached", ...)` output the live menus use, so
        // the slider and the attached-menu material cannot drift. The tint
        // only follows the light attached-menu preset; the description says
        // so rather than claiming a global re-tint.
        SettingsGroup {
            id: glassGroup
            width: parent.width
            title: qsTr("Glass")
            sectionId: "Glass"

            Item {
                id: glassPreview
                width: parent.width
                height: 120

                // A token-only backdrop so the plate's transmission is
                // visible without shipping another wallpaper asset.
                Rectangle {
                    anchors.fill: parent
                    radius: Theme.controls.settingsGroup.radius
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: Theme.color.accentMuted }
                        GradientStop { position: 1.0; color: Theme.color.surfaceMuted }
                    }
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border
                    antialiasing: true
                }

                // The attached-menu sample. Its fill, rim, and edge width are
                // all resolver outputs, not a second alpha formula.
                Rectangle {
                    id: previewPlate
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.rightMargin: Theme.primitive.spacing.md
                    anchors.topMargin: Theme.primitive.spacing.md
                    height: 60
                    radius: Theme.controls.settingsGroup.radius
                    color: Qt.rgba(root.previewResolved.tint,
                                   root.previewResolved.tint,
                                   root.previewResolved.tint,
                                   root.previewResolved.alpha)
                    border.width: Math.max(Theme.controls.window.borderWidth,
                                           Math.round(root.previewResolved.edgeWidth))
                    border.color: Qt.rgba(1, 1, 1, root.previewResolved.edge)
                    antialiasing: true

                    Row {
                        id: glassDots
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Repeater {
                            model: 3
                            delegate: Rectangle {
                                width: 7
                                height: 7
                                radius: 3.5
                                color: Qt.rgba(1, 1, 1, 0.85)
                            }
                        }
                    }

                    Rectangle {
                        anchors.left: glassDots.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        height: 20
                        radius: height / 2
                        color: Qt.rgba(1, 1, 1, 0.35)

                        Text {
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.primitive.spacing.sm
                            anchors.verticalCenter: parent.verticalCenter
                            text: qsTr("Search")
                            color: Qt.rgba(1, 1, 1, 0.95)
                            font.pixelSize: Theme.primitive.font.sizeXs
                        }
                    }
                }

                // The resolver readout the card already owned: scheme, live
                // render mode, and the tint percent.
                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.primitive.spacing.md
                    anchors.rightMargin: Theme.primitive.spacing.md
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: Theme.primitive.spacing.sm
                    text: qsTr("%1 · %2 · %3")
                          .arg(root.previewScheme)
                          .arg(root.previewMode)
                          .arg(root.previewPercent)
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeXs
                    elide: Text.ElideRight
                }
            }

            // The one `appearance.transparency` row. It stays a
            // `SettingsKeyRow` so a rejected write reverts inline; the two end
            // glyphs are the design-system icon vocabulary, not new artwork.
            SettingsKeyRow {
                id: transparencyRow
                width: parent.width
                label: qsTr("Glass tint")
                settingsId: "transparency"
                owner: "settingsd + shell"
                settingsKey: "appearance.transparency"
                enabled: !root.reduceTransparency
                description: root.reduceTransparency
                    ? qsTr("Unavailable while Reduce Transparency is on. Turn it "
                           + "off in Accessibility to use this slider.")
                    : qsTr("Attached menus only. The Dock, menu bar, and windows "
                           + "keep their own materials; Reduce Transparency "
                           + "overrides this slider.")
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Item {
                        width: 16
                        height: transparencySlider.height

                        TablerIcon {
                            anchors.centerIn: parent
                            name: "layout-bottombar"
                            size: 16
                            color: Theme.color.textTertiary
                        }
                    }

                    Slider {
                        id: transparencySlider
                        width: 160
                        from: 0.0
                        to: 1.0
                        minLabel: qsTr("Opaque")
                        maxLabel: qsTr("Clear")
                        accessibleName: qsTr("Glass tint")
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

                    Item {
                        width: 16
                        height: transparencySlider.height

                        TablerIcon {
                            anchors.centerIn: parent
                            name: "app-window"
                            size: 16
                            color: Theme.color.textTertiary
                        }
                    }
                }
            }
        }

        // T290d: one Theme card in the mockup order. The accent keeps its
        // swatch row and custom-hex popup (the one colour with a custom entry
        // point); highlight and folder color are one swatch popup each; icon
        // style is four tiles over the four `appearance.iconStyle` values.
        // Sidebar icon size stays at the end of this card until T290e.
        SettingsGroup {
            id: themeGroup
            width: parent.width
            title: qsTr("Theme")
            sectionId: "Theme"

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
                label: qsTr("Text highlight color") + root.labelSuffix
                settingsId: "highlight"
                owner: "settingsd + shell"
                settingsKey: "appearance.highlight"
                description: qsTr("The selection wash in search fields and pickers. "
                                  + "Default follows the active color scheme.")

                // One popup that shows the current swatch, not a swatch row
                // plus a button. A custom hex set elsewhere is shown as its
                // hex label and is never cleared by the popup.
                controlData: ColorSelectButton {
                    id: highlightSelect
                    accessibleLabel: qsTr("Text highlight color")
                    popup: highlightChoicePopup
                }
            }

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
                // The tiles are taller than the 33 pt row. Derive the height
                // from the control instead of `fixedRowHeight`.
                rowHeight: root.iconStyleTileRowHeight

                controlData: Row {
                    id: iconStyleTiles
                    spacing: Theme.primitive.spacing.sm

                    Accessible.role: Accessible.Grouping
                    Accessible.name: qsTr("Icon & widget style")

                    Repeater {
                        id: iconStyleTileRepeater
                        model: root.iconStyleOptions

                        delegate: SettingsModeTile {
                            required property var modelData
                            required property int index

                            // A style the provider cannot map to a theme stays
                            // visible and dimmed. It is still a click target so
                            // the rejection reason reaches the row description,
                            // exactly like the segmented control it replaces.
                            readonly property bool available:
                                modelData.enabled !== false

                            width: root.iconStyleTileWidth
                            label: modelData.label
                            previewFill: root.iconStylePreviewFill(modelData.value)
                            previewAccent: root.iconStylePreviewAccent(modelData.value)
                            selected: root.currentIconStyle === modelData.value
                            opacity: available ? 1.0 : 0.45
                            Accessible.description:
                                available ? "" : String(modelData.reason || "")
                            onClicked: {
                                if (!available) {
                                    root.iconStyleRejectedReason =
                                        String(modelData.reason
                                               || qsTr("%1 is unavailable on this system.")
                                                      .arg(modelData.label));
                                    return;
                                }
                                root.iconStyleRejectedReason = "";
                                Settings.set("appearance.iconStyle",
                                             String(modelData.value));
                            }
                        }
                    }

                    Keys.onPressed: (event) => {
                        if (event.key !== Qt.Key_Left
                                && event.key !== Qt.Key_Right)
                            return;
                        var delta = event.key === Qt.Key_Right ? 1 : -1;
                        var index = root.iconStyleIndex();
                        for (var i = 0; i < iconStyleTileRepeater.count; ++i) {
                            var tile = iconStyleTileRepeater.itemAt(i);
                            if (tile && tile.activeFocus) {
                                index = i;
                                break;
                            }
                        }
                        root.iconStyleMove(delta, index);
                        event.accepted = true;
                    }
                }
            }

            SettingsKeyRow {
                id: folderColorRow
                width: parent.width
                label: qsTr("Folder color") + root.labelSuffix
                settingsId: "folderColor"
                owner: "settingsd + Files"
                settingsKey: "appearance.folderColor"
                description: qsTr("Tints the plain folder icon in Files and the Dock. "
                                  + "Automatic keeps the shipped folder artwork.")

                controlData: ColorSelectButton {
                    id: folderColorSelect
                    accessibleLabel: qsTr("Folder color")
                    popup: folderColorChoicePopup
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

    // T290d: the two single-colour popups. Each owns the palette, shows the
    // current value as a swatch and label on its trigger, and writes the key
    // only when an option is chosen. An externally-set custom hex is shown as
    // its hex label and never cleared.
    ColorChoicePopup {
        id: highlightChoicePopup
        anchorItem: highlightSelect
        palette: root.highlightPalette
        currentValue: root.currentHighlight
        fallbackColor: Theme.color.selection
        accessibleName: qsTr("Text highlight color")
        onChosen: (value) => Settings.set("appearance.highlight", value)
    }

    ColorChoicePopup {
        id: folderColorChoicePopup
        anchorItem: folderColorSelect
        palette: root.folderPalette
        currentValue: root.currentFolderColor
        fallbackColor: Theme.color.textSecondary
        accessibleName: qsTr("Folder color")
        onChosen: (value) => Settings.set("appearance.folderColor", value)
    }

    // The trigger for a single-colour popup: current swatch, current label,
    // and a chevron. It is a pane-local control, not a fork of `Select`: the
    // value is a colour and the open surface is the swatch palette above.
    component ColorSelectButton: Item {
        id: trigger

        property string accessibleLabel: ""
        property var popup: null

        readonly property string currentLabel:
            trigger.popup ? String(trigger.popup.currentLabel) : ""
        readonly property color swatchColor:
            trigger.popup ? trigger.popup.currentSwatchColor
                          : Theme.color.controlFill
        readonly property bool opened:
            trigger.popup ? trigger.popup.open === true : false

        implicitWidth: Math.max(Theme.controls.select.minWidth,
                                Theme.controls.select.paddingH + swatch.width
                                + Theme.primitive.spacing.sm
                                + labelText.implicitWidth
                                + Theme.primitive.spacing.sm
                                + chevron.width
                                + Theme.controls.select.paddingH)
        implicitHeight: Theme.controls.select.height
        activeFocusOnTab: true

        function toggle() {
            if (!trigger.popup)
                return;
            if (trigger.popup.open)
                trigger.popup.hide();
            else
                trigger.popup.show();
        }

        function open() {
            if (trigger.popup)
                trigger.popup.show();
        }

        function close() {
            if (trigger.popup)
                trigger.popup.hide();
        }

        function optionAt(index) {
            return trigger.popup ? trigger.popup.optionAt(index) : null;
        }

        function chooseIndex(index) {
            if (trigger.popup)
                trigger.popup.chooseIndex(index);
        }

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.select.radius
            color: selectHover.hovered ? Theme.color.controlHover
                                       : Theme.color.controlFill
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Rectangle {
            id: swatch
            width: 14
            height: 14
            radius: width / 2
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.select.paddingH
            anchors.verticalCenter: parent.verticalCenter
            color: trigger.swatchColor
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Text {
            id: labelText
            anchors.left: swatch.right
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.right: chevron.left
            anchors.rightMargin: Theme.primitive.spacing.sm
            anchors.verticalCenter: parent.verticalCenter
            text: trigger.currentLabel
            color: Theme.color.textPrimary
            elide: Text.ElideRight
            font.pixelSize: Theme.controls.button.fontSize
        }

        TablerIcon {
            id: chevron
            name: "chevron-down"
            size: Theme.controls.select.chevronSize
            color: Theme.color.textTertiary
            anchors.right: parent.right
            anchors.rightMargin: Theme.controls.select.paddingH
            anchors.verticalCenter: parent.verticalCenter
        }

        HoverHandler { id: selectHover }

        TapHandler {
            onTapped: trigger.toggle()
        }

        FocusRing {
            target: trigger
            cornerRadius: Theme.controls.select.radius
            shown: trigger.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter || event.key === Qt.Key_Down) {
                trigger.toggle();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.ComboBox
        Accessible.name: trigger.accessibleLabel
        Accessible.description: qsTr("Current value %1")
                                 .arg(trigger.currentLabel)
        Accessible.focusable: true
        Accessible.onPressAction: trigger.toggle()
    }

    // The palette surface behind a `ColorSelectButton`. One row per palette
    // entry, each a real radio button; the current entry carries the check.
    component ColorChoicePopup: Popup {
        id: chooser

        property var palette: []
        property string currentValue: ""
        property color fallbackColor: Theme.color.selection

        readonly property var currentEntry: {
            for (var i = 0; i < chooser.palette.length; ++i) {
                if (String(chooser.palette[i].value) === chooser.currentValue)
                    return chooser.palette[i];
            }
            return null;
        }
        readonly property string currentLabel: {
            if (chooser.currentEntry)
                return String(chooser.currentEntry.label);
            if (chooser.currentValue.length > 0)
                return chooser.currentValue;
            return chooser.palette.length > 0
                    ? String(chooser.palette[0].label) : "";
        }
        readonly property color currentSwatchColor: {
            if (chooser.currentEntry
                    && String(chooser.currentEntry.value) !== "")
                return String(chooser.currentEntry.value);
            if (!chooser.currentEntry
                    && root.isHexColor(chooser.currentValue))
                return chooser.currentValue;
            return chooser.fallbackColor;
        }
        readonly property int optionCount: optionRepeater.count

        signal chosen(string value)

        anchorItem: null
        preferredWidth: 220
        accessibleName: qsTr("Color")

        function optionAt(index) {
            return optionRepeater.itemAt(index);
        }

        function chooseIndex(index) {
            var entry = chooser.palette[index];
            if (!entry)
                return;
            chooser.chosen(String(entry.value));
            chooser.hide();
        }

        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.xxs

            Repeater {
                id: optionRepeater
                model: chooser.palette

                delegate: ColorOption {
                    currentValue: chooser.currentValue
                    fallbackColor: chooser.fallbackColor
                    onChosen: (value) => {
                        chooser.chosen(value);
                        chooser.hide();
                    }
                }
            }
        }
    }

    // One row in the colour popup: swatch, label, and the current check.
    component ColorOption: Item {
        id: option

        required property var modelData
        property string currentValue: ""
        property color fallbackColor: Theme.color.selection

        readonly property bool selected:
            String(option.currentValue) === String(option.modelData.value)
        readonly property color swatchColor:
            String(option.modelData.value) === "" ? option.fallbackColor
                                                  : String(option.modelData.value)

        signal chosen(string value)

        width: parent ? parent.width : 0
        height: Theme.controls.settingsRow.height
        activeFocusOnTab: true

        function activate() {
            option.chosen(String(option.modelData.value));
        }

        Rectangle {
            anchors.fill: parent
            radius: Theme.primitive.radius.sm
            color: option.selected ? Theme.color.accentMuted
                   : optionHover.hovered ? Theme.color.controlHover
                                         : "transparent"
            antialiasing: true
        }

        Rectangle {
            id: optionSwatch
            width: 14
            height: 14
            radius: width / 2
            anchors.left: parent.left
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.verticalCenter: parent.verticalCenter
            color: option.swatchColor
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        Text {
            anchors.left: optionSwatch.right
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.right: checkGlyph.left
            anchors.rightMargin: Theme.primitive.spacing.xs
            anchors.verticalCenter: parent.verticalCenter
            text: option.modelData.label
            color: Theme.color.textPrimary
            elide: Text.ElideRight
            font.pixelSize: Theme.controls.button.fontSize
        }

        TablerIcon {
            id: checkGlyph
            name: "check"
            size: Theme.controls.select.chevronSize
            color: Theme.color.accent
            visible: option.selected
            anchors.right: parent.right
            anchors.rightMargin: Theme.primitive.spacing.sm
            anchors.verticalCenter: parent.verticalCenter
        }

        HoverHandler { id: optionHover }

        TapHandler {
            onTapped: option.activate()
        }

        FocusRing {
            target: option
            cornerRadius: Theme.primitive.radius.sm
            shown: option.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                option.activate();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: option.modelData.label
        Accessible.checkable: true
        Accessible.checked: option.selected
        Accessible.focusable: true
        Accessible.onPressAction: option.activate()
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
