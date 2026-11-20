// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Desktop & Dock pane (T-09.4; T276 completion; T292a restyle).
//
// The pane follows the mockup's section order and labels:
//
//   Dock                    size, magnification, position, minimize, title bar
//                           double-click, minimize-into-icon, auto-hide, animate
//                           opening, indicators, recent apps
//   Desktop & Workspaces    Show items, Click wallpaper, Workspace Focus,
//                           recent apps in Workspace Focus, windows from an app
//   Widgets                 Show Widgets, Dim widgets, Default web browser
//   Windows                 tabs, keep changes, close on quit, the tiling rows
//   Window Overview         the temporary **Workspace Overview** entry until
//                           T292b merges the former Mission Control surface
//   Footer                  Shortcuts… and Hot Corners… open the existing
//                           surfaces; they are not duplicated inline.
//
// The Linux-only rows keep their real owners and keys but leave the main pane
// for two advanced sheets (T292a): **Advanced Dock** owns the hover chooser and
// the minimize reaction, **Advanced Windows** owns the application window
// rules. A deep link to an advanced row opens its sheet through
// `openRowSurface` / `rowTargetFor`, the same closed-dialog path Appearance
// uses (see T290f).
//
// Every live control is a stock design-system component bound to the `Settings`
// singleton (never D-Bus directly) with the T-09.1b write-on-interaction /
// bind-to-`Settings.values` pattern, so a user edit and an external settingsd
// change converge without a restart. A row whose runtime is not implemented
// yet is disabled with an honest reason (the owning task lives in the map and
// docs, never in the UI).
//
// The **Show application menus in the menu bar** row (`menu.global`) left this
// pane in T292a; the Menu Bar pane owns it (T293a).
//
// Test and capture access: read a control through a pane-body `property`/
// `property alias` declared below, or through a row's `control.children[0]`.
Item {
    id: root

    // Pane-to-pane navigation (existing callers).
    signal navigateRequested(string paneId)
    // T292a: a footer/row action that lands on a section of this pane.
    signal sectionRequested(string paneId, string section)

    readonly property real sizeValue: {
        var v = Settings.values["dock.size"];
        return v === undefined ? 0.5 : Number(v);
    }
    readonly property real magnificationValue: {
        var v = Settings.values["dock.magnification"];
        return v === undefined ? 0.5 : Number(v);
    }

    readonly property var positionOptions: [
        { value: "bottom", label: qsTr("Bottom") },
        { value: "left", label: qsTr("Left") },
        { value: "right", label: qsTr("Right") }
    ]
    readonly property var minimizedAnimationOptions: [
        { value: "genie", label: qsTr("Genie Effect") },
        { value: "scale", label: qsTr("Scale Effect") },
        { value: "none", label: qsTr("None") }
    ]
    readonly property var titlebarDoubleClickOptions: [
        { value: "zoom", label: qsTr("Zoom") },
        { value: "minimize", label: qsTr("Minimize") },
        { value: "none", label: qsTr("None") }
    ]
    // T227/T276: Desktop & Workspaces Manager. `desktop.showItems` is consumed
    // by the Files desktop process; the remaining rows name their missing
    // runtime in the description and stay disabled (T242/T245b).
    readonly property var desktopItemsOptions: [
        { value: "on-desktop", label: qsTr("On Desktop") },
        { value: "in-stage", label: qsTr("In Workspace Focus") }
    ]
    readonly property string desktopItemsValue:
        Settings.values["desktop.showItems"] || "on-desktop"
    readonly property var windowsFromAppOptions: [
        { value: "all", label: qsTr("All at Once") },
        { value: "one", label: qsTr("One at a Time") }
    ]

    // T276: the default-browser row's installed choices come from app-index
    // through the Settings bridge. Empty is the honest no-browser state.
    readonly property var browserChoices: Settings.defaultBrowserChoices
    readonly property bool browserAvailable: browserChoices.length > 0

    // T292a: the rows that live in a closed advanced sheet, in rendered order.
    // `SettingsMap.rowSurface()` answers "dialog" for exactly these ids.
    readonly property var advancedDockRowIds: ["chooserOnHover", "minimizeReaction"]
    readonly property var advancedWindowsRowIds: ["windowRules"]

    // Test surface (used by tst_settings_desktop_dock.qml).
    //
    // The three advanced rows live inside a closed Dialog, and a Dialog adopts
    // the window overlay layer, so they are reached through body-level
    // accessors that stay valid whether the sheet is open or closed.
    property alias chooserOnHoverRowHandle: chooserOnHoverRow
    property alias minimizeReactionRowHandle: minimizeReactionRow
    property alias windowRulesRowHandle: windowRulesRow
    property alias sizeSlider: sizeSlider
    property alias magnificationSlider: magnificationSlider
    property alias positionSelect: positionSelect
    property alias minimizedAnimationSelect: minimizedAnimationSelect
    property alias titlebarSelect: titlebarSelect
    property alias minimizeIntoTileToggle: minimizeIntoTileToggle
    property alias autohideToggle: autohideToggle
    property alias animateOpeningToggle: animateOpeningToggle
    property alias showIndicatorsToggle: showIndicatorsToggle
    property alias showRecentAppsToggle: showRecentAppsToggle
    property alias chooserOnHoverToggle: chooserOnHoverToggle
    property alias minimizeReactionToggle: minimizeReactionToggle
    property alias dockGroup: dockGroup
    property alias desktopItemsControl: desktopItemsControl
    property alias desktopGroup: desktopGroup
    property alias clickWallpaperRow: clickWallpaperRow
    property alias stageRow: stageRow
    property alias stageToggle: stageToggle
    property alias stageRecentRow: stageRecentRow
    property alias stageRecentToggle: stageRecentToggle
    property alias windowsFromAppRow: windowsFromAppRow
    property alias widgetsGroup: widgetsGroup
    property alias showWidgetsRow: showWidgetsRow
    property alias dimWidgetsRow: dimWidgetsRow
    property alias defaultBrowserRow: defaultBrowserRow
    property alias defaultBrowserSelect: defaultBrowserSelect
    property alias windowsGroup: windowsGroup
    property alias preferTabsRow: preferTabsRow
    property alias askToKeepChangesRow: askToKeepChangesRow
    property alias closeOnQuitRow: closeOnQuitRow
    property alias dragToTileRow: dragToTileRow
    property alias dragToFillRow: dragToFillRow
    property alias holdModifierRow: holdModifierRow
    property alias tiledMarginsRow: tiledMarginsRow
    property alias missionControlGroup: missionControlGroup
    property alias overviewRow: overviewRow
    property alias footer: footer
    property alias shortcutsButton: shortcutsButton
    property alias hotCornersButton: hotCornersButton
    property alias advancedDockButton: advancedDockButton
    property alias advancedDockDialog: advancedDockDialog
    property alias advancedDockGroup: advancedDockGroup
    property alias advancedWindowsButton: advancedWindowsButton
    property alias advancedWindowsDialog: advancedWindowsDialog
    property alias advancedWindowsGroup: advancedWindowsGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

    // T292a: an invisible build canary, so a stale copy fails loudly.
    readonly property string sourceRevision: "t292a"

    Component.onCompleted: Settings.refreshDefaultBrowsers()

    // Index of `value` in an option list; unknown values fall to `fallback`.
    function optionIndex(options, value, fallback) {
        for (var i = 0; i < options.length; ++i) {
            if (options[i].value === value)
                return i;
        }
        return fallback;
    }

    function positionIndex() {
        return root.optionIndex(root.positionOptions,
                                Settings.values["dock.position"], 0);
    }
    function minimizedAnimationIndex() {
        return root.optionIndex(root.minimizedAnimationOptions,
                                Settings.values["dock.minimizedAnimation"], 1);
    }
    function titlebarDoubleClickIndex() {
        return root.optionIndex(root.titlebarDoubleClickOptions,
                                Settings.values["dock.titlebarDoubleClick"], 0);
    }
    function browserIndex() {
        return root.optionIndex(root.browserChoices,
                                Settings.defaultBrowser, 0);
    }

    // -- Advanced sheets (T292a) ---------------------------------------------

    function isAdvancedRow(rowId) {
        var id = String(rowId);
        return root.advancedDockRowIds.indexOf(id) >= 0
                || root.advancedWindowsRowIds.indexOf(id) >= 0;
    }

    // The sheet that owns a row id, or "" for a main-pane row.
    function advancedSheetFor(rowId) {
        var id = String(rowId);
        if (root.advancedDockRowIds.indexOf(id) >= 0)
            return "dock";
        if (root.advancedWindowsRowIds.indexOf(id) >= 0)
            return "windows";
        return "";
    }

    // The row item behind an advanced row id. A Dialog adopts the window
    // overlay layer, so the shell cannot find it by walking the pane body; it
    // asks the pane instead.
    function advancedRowFor(rowId) {
        switch (String(rowId)) {
        case "chooserOnHover": return chooserOnHoverRow;
        case "minimizeReaction": return minimizeReactionRow;
        case "windowRules": return windowRulesRow;
        }
        return null;
    }

    function rowTargetFor(rowId) {
        return root.isAdvancedRow(rowId) ? root.advancedRowFor(rowId) : null;
    }

    function openAdvancedDock() {
        advancedDockDialog.show();
    }
    function closeAdvancedDock() {
        advancedDockDialog.hide();
    }
    function openAdvancedWindows() {
        advancedWindowsDialog.show();
    }

    // The capture/T276 entry point for the existing hot-corner surface.
    function openHotCorners() {
        root.navigateRequested("mission-control");
    }

    // T292a: the four footer actions share one row. These helpers derive the
    // shared action width from the natural text widths (measured once, at the
    // button font) so the row shrinks to fit instead of overflowing.
    readonly property real footerGlyphWidth: {
        var padding = Theme.controls.button.paddingH;
        var widest = 0;
        for (var i = 0; i < footerTexts.length; ++i)
            widest = Math.max(widest, footerTexts[i].contentWidth);
        return Math.max(Theme.controls.button.height, widest + 2 * padding);
    }
    readonly property real footerNaturalWidth:
        2 * root.footerGlyphWidth
        + Math.max(root.footerGlyphWidth, Theme.controls.button.height * 2)
        + 3 * Theme.primitive.spacing.sm
    readonly property var footerTexts: [footerMeasure0, footerMeasure1,
                                        footerMeasure2]

    function footerItemWidth(available, natural) {
        var count = 4;
        var gaps = (count - 1) * Theme.primitive.spacing.sm;
        var even = (available - gaps) / count;
        var item = natural > 0 ? even * available / natural : even;
        return Math.max(Theme.controls.button.height, Math.min(even, item));
    }
    function closeAdvancedWindows() {
        advancedWindowsDialog.hide();
    }

    // The shell's closed-surface hook: open the owning sheet before it looks
    // for the row. Returns whether this pane owns the row.
    function openRowSurface(rowId) {
        if (!root.isAdvancedRow(rowId))
            return false;
        if (root.advancedSheetFor(rowId) === "dock")
            root.openAdvancedDock();
        else
            root.openAdvancedWindows();
        return true;
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            id: dockGroup
            width: parent.width
            title: qsTr("Dock")
            sectionId: "Dock"

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Dock Size")
                settingsId: "size"
                owner: "settingsd"
                settingsKey: "dock.size"
                controlData: Slider {
                    id: sizeSlider
                    // 60% of the row, not a fixed 220 pt: at the measured
                    // 740 pt window the row's control slot is narrower than
                    // the slider's own minimum, and a wider control clips.
                    width: parent ? parent.width : implicitWidth
                    from: 0.0
                    to: 1.0
                    minLabel: qsTr("Small")
                    maxLabel: qsTr("Large")
                    accessibleName: qsTr("Dock size")
                    onCommitted: (value) => Settings.set("dock.size", value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Dock Magnification Size")
                settingsId: "magnification"
                owner: "settingsd"
                settingsKey: "dock.magnification"
                controlData: Slider {
                    id: magnificationSlider
                    width: parent ? parent.width : implicitWidth
                    from: 0.0
                    to: 1.0
                    minLabel: qsTr("Off")
                    midLabel: qsTr("Small")
                    maxLabel: qsTr("Large")
                    accessibleName: qsTr("Dock magnification")
                    onCommitted: (value) => Settings.set("dock.magnification", value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Dock position on screen")
                settingsId: "position"
                owner: "settingsd"
                settingsKey: "dock.position"
                controlData: Select {
                    id: positionSelect
                    accessibleName: qsTr("Dock position on screen")
                    model: root.positionOptions
                    onActivated: (index) => Settings.set("dock.position",
                                                         root.positionOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Minimized window animation")
                settingsId: "minimizeAnimation"
                owner: "settingsd"
                settingsKey: "dock.minimizedAnimation"
                controlData: Select {
                    id: minimizedAnimationSelect
                    accessibleName: qsTr("Minimized window animation")
                    model: root.minimizedAnimationOptions
                    onActivated: (index) => Settings.set(
                                    "dock.minimizedAnimation",
                                    root.minimizedAnimationOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Window title bar double-click action")
                settingsId: "titlebarDoubleClick"
                owner: "settingsd"
                settingsKey: "dock.titlebarDoubleClick"
                controlData: Select {
                    id: titlebarSelect
                    accessibleName: qsTr("Window title bar double-click action")
                    model: root.titlebarDoubleClickOptions
                    onActivated: (index) => Settings.set(
                                    "dock.titlebarDoubleClick",
                                    root.titlebarDoubleClickOptions[index].value)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Minimize windows into application icon")
                settingsId: "minimizeIntoTileIcon"
                owner: "settingsd"
                settingsKey: "dock.minimizeIntoTileIcon"
                controlData: Toggle {
                    id: minimizeIntoTileToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.minimizeIntoTileIcon", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Automatically hide and show the Dock")
                settingsId: "autohide"
                owner: "settingsd"
                settingsKey: "dock.autohide"
                controlData: Toggle {
                    id: autohideToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.autohide", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Animate opening applications")
                settingsId: "animateOpening"
                owner: "settingsd"
                settingsKey: "dock.animateOpening"
                controlData: Toggle {
                    id: animateOpeningToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.animateOpening", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Show indicators for open applications")
                settingsId: "showIndicators"
                owner: "settingsd"
                settingsKey: "dock.showIndicators"
                controlData: Toggle {
                    id: showIndicatorsToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.showIndicators", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Show suggested and recent apps in Dock")
                settingsId: "showRecentApps"
                owner: "settingsd"
                settingsKey: "dock.showRecentApps"
                showSeparator: false
                controlData: Toggle {
                    id: showRecentAppsToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.showRecentApps", checked)
                }
            }
        }

        // ── Desktop & Workspaces ──────────────────────────────────────────
        SettingsGroup {
            id: desktopGroup
            width: parent.width
            title: qsTr("Desktop & Workspaces")
            sectionId: "Desktop & Workspaces"

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Show items")
                settingsId: "desktopItems"
                owner: "settingsd -> apps/files desktop"
                settingsKey: "desktop.showItems"
                description: qsTr("On Desktop shows the Files desktop icons; In "
                                  + "Workspace Focus leaves them to Workspace Focus.")
                controlData: SegmentedControl {
                    id: desktopItemsControl
                    model: root.desktopItemsOptions.map(function(option) {
                        return option.label;
                    })
                    onActivated: (index) => Settings.set(
                                    "desktop.showItems",
                                    root.desktopItemsOptions[index].value)
                }
            }

            SettingsRow {
                id: clickWallpaperRow
                width: parent.width
                label: qsTr("Click wallpaper to show desktop")
                description: qsTr("Reveal-on-click is not available yet; the desktop "
                                  + "layer owns background clicks.")
                settingsId: "clickWallpaper"
                owner: "compositor"
                settingsKey: "desktop.clickWallpaper"
                enabled: false

                controlData: Select {
                    model: [qsTr("Always"), qsTr("Only in Workspace Focus"),
                            qsTr("Never")]
                    enabled: false
                }
            }

            SettingsRow {
                id: stageRow
                width: parent.width
                label: qsTr("Workspace Focus")
                description: qsTr("Workspace Focus is not available yet; window "
                                  + "staging is not implemented.")
                settingsId: "stage"
                owner: "compositor"
                settingsKey: "stageManager.enabled"
                enabled: false

                controlData: Toggle {
                    id: stageToggle
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: stageRecentRow
                width: parent.width
                label: qsTr("Show recent apps in Workspace Focus")
                description: qsTr("Available with Workspace Focus.")
                settingsId: "stageRecent"
                owner: "compositor"
                settingsKey: "stageManager.showRecentApps"
                enabled: false

                controlData: Toggle {
                    id: stageRecentToggle
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: windowsFromAppRow
                width: parent.width
                label: qsTr("Show windows from an application")
                description: qsTr("Available with Workspace Focus.")
                settingsId: "stageWindowMode"
                owner: "compositor"
                settingsKey: "stageManager.windowMode"
                enabled: false
                showSeparator: false

                controlData: Select {
                    model: root.windowsFromAppOptions
                    enabled: false
                }
            }
        }

        // ── Widgets ───────────────────────────────────────────────────────
        SettingsGroup {
            id: widgetsGroup
            width: parent.width
            title: qsTr("Widgets")
            sectionId: "Widgets"

            SettingsRow {
                id: showWidgetsRow
                width: parent.width
                label: qsTr("Show Widgets")
                description: qsTr("Desktop widgets are not available yet.")
                settingsId: "showWidgets"
                owner: "shell + files desktop"
                settingsKey: "widgets.show"
                enabled: false

                controlData: SegmentedControl {
                    model: [qsTr("Off"), qsTr("On Desktop")]
                    enabled: false
                }
            }

            SettingsRow {
                id: dimWidgetsRow
                width: parent.width
                label: qsTr("Dim widgets on desktop")
                description: qsTr("Available with desktop widgets.")
                settingsId: "dimWidgets"
                owner: "shell + files desktop"
                settingsKey: "widgets.dim"
                enabled: false

                controlData: Select {
                    model: [qsTr("Automatically"), qsTr("Always"), qsTr("Never")]
                    enabled: false
                }
            }

            // T292a: the mockup places the default browser in Widgets. The row
            // is the existing live `defaults.browser` delegate, moved here.
            SettingsRow {
                id: defaultBrowserRow
                width: parent.width
                label: qsTr("Default web browser")
                description: root.browserAvailable
                    ? (Settings.defaultBrowserError.length > 0
                       ? Settings.defaultBrowserError
                       : qsTr("Used for web links and web-search results."))
                    : qsTr("No installed web browser was found.")
                settingsId: "defaultBrowser"
                owner: "settingsd + default-apps delegate"
                settingsKey: "defaults.browser"
                enabled: root.browserAvailable
                showSeparator: false

                controlData: Select {
                    id: defaultBrowserSelect
                    accessibleName: qsTr("Default web browser")
                    model: root.browserChoices
                    enabled: root.browserAvailable
                    onActivated: (index) => {
                        if (index >= 0 && index < root.browserChoices.length)
                            Settings.setDefaultBrowser(
                                root.browserChoices[index].value);
                    }
                }
            }
        }

        // ── Windows ───────────────────────────────────────────────────────
        SettingsGroup {
            id: windowsGroup
            width: parent.width
            title: qsTr("Windows")
            sectionId: "Windows"

            SettingsRow {
                id: preferTabsRow
                width: parent.width
                label: qsTr("Prefer tabs when opening documents")
                description: qsTr("No first-party app opens tabbed documents yet.")
                settingsId: "preferTabs"
                owner: "apps/files + first-party apps"
                settingsKey: "windows.preferTabs"
                enabled: false

                controlData: Select {
                    model: [qsTr("Always"), qsTr("In Full Screen"), qsTr("Never")]
                    enabled: false
                }
            }

            SettingsRow {
                id: askToKeepChangesRow
                width: parent.width
                label: qsTr("Ask to keep changes when closing documents")
                description: qsTr("No first-party document app asks yet.")
                settingsId: "askToKeepChanges"
                owner: "first-party document apps"
                settingsKey: "windows.askToKeepChanges"
                enabled: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: closeOnQuitRow
                width: parent.width
                label: qsTr("Close windows when quitting an application")
                description: qsTr("No first-party app lifecycle consumer yet.")
                settingsId: "closeOnQuit"
                owner: "first-party app lifecycle"
                settingsKey: "windows.closeOnQuit"
                enabled: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: dragToTileRow
                width: parent.width
                label: qsTr("Drag windows to left or right edge of screen to tile")
                description: qsTr("Window tiling is not available yet.")
                settingsId: "dragToTile"
                owner: "compositor"
                settingsKey: "tiling.dragToTile"
                enabled: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: dragToFillRow
                width: parent.width
                label: qsTr("Drag windows to menu bar to fill screen")
                description: qsTr("Window tiling is not available yet.")
                settingsId: "dragToFill"
                owner: "compositor"
                settingsKey: "tiling.dragToFill"
                enabled: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: holdModifierRow
                width: parent.width
                label: qsTr("Hold Super key while dragging windows to tile")
                description: qsTr("Window tiling is not available yet.")
                settingsId: "tileModifier"
                owner: "compositor"
                settingsKey: "tiling.modifier"
                enabled: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

            SettingsRow {
                id: tiledMarginsRow
                width: parent.width
                label: qsTr("Tiled windows have margins")
                description: qsTr("Window tiling is not available yet.")
                settingsId: "tileMargins"
                owner: "compositor"
                settingsKey: "tiling.margins"
                enabled: false
                showSeparator: false

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }
        }

        // ── Window Overview ───────────────────────────────────────────────
        // T292a keeps one live entry into the former Mission Control surface;
        // T292b merges its rows into this section and deletes that pane. The
        // entry is a disclosure into the real surface, never a dead action.
        SettingsGroup {
            id: missionControlGroup
            width: parent.width
            title: qsTr("Window Overview")
            sectionId: "Workspace Overview"

            SettingsRow {
                id: overviewRow
                width: parent.width
                label: qsTr("Workspace Overview")
                description: qsTr("Workspaces, hot corners, gestures, and window "
                                  + "arrangement.")
                settingsId: "overview"
                owner: "settingsd + compositor"
                // T292a: the entry itself writes nothing (it opens the former
                // Mission Control surface, which owns the `overview.*` keys).
                // A navigation row carries no `settingsKey`; the map row names
                // the `overview.*` family for search and the debug view.
                settingsKey: ""
                disclosure: true
                showSeparator: false
                onDisclosureClicked: root.navigateRequested("mission-control")
            }
        }

        // T292a: the mockup footer. Both actions open an existing surface —
        // there is no inline duplicate of the shortcut editor or the hot
        // corners — and the Advanced sheets hold the Linux-only rows.
        SettingsFooter {
            id: footer
            objectName: "desktopDockFooter"
            width: parent.width

            // T292a: four ghost actions share one right-aligned row. When the
            // row is narrower than their natural widths they shrink together
            // down to the 28 pt button height, so the group never overflows
            // the 468 pt content column at the measured 740 pt window. The
            // width comes from pane-level helpers, never from the buttons'
            // own implicit widths, so there is no binding loop.
            readonly property real availableWidth:
                width - 2 * Theme.controls.settingsRow.paddingH
            readonly property real naturalWidth: root.footerNaturalWidth
            readonly property real itemWidth: root.footerItemWidth(footer.availableWidth,
                                                                   footer.naturalWidth)

            Button {
                id: advancedDockButton
                objectName: "desktopDockAdvancedDockButton"
                variant: "ghost"
                width: footer.itemWidth
                implicitWidth: footer.itemWidth
                text: qsTr("Advanced…")
                accessibleName: qsTr("Open advanced Dock settings")
                onClicked: root.openAdvancedDock()
            }

            Button {
                id: advancedWindowsButton
                objectName: "desktopDockAdvancedWindowsButton"
                variant: "ghost"
                width: footer.itemWidth
                implicitWidth: footer.itemWidth
                text: qsTr("Advanced…")
                accessibleName: qsTr("Open advanced window settings")
                onClicked: root.openAdvancedWindows()
            }

            Button {
                id: shortcutsButton
                objectName: "desktopDockShortcutsButton"
                variant: "ghost"
                width: footer.itemWidth
                implicitWidth: footer.itemWidth
                text: qsTr("Shortcuts…")
                accessibleName: qsTr("Open keyboard shortcuts")
                onClicked: root.sectionRequested("keyboard", "shortcuts")
            }

            Button {
                id: hotCornersButton
                objectName: "desktopDockHotCornersButton"
                variant: "ghost"
                width: footer.itemWidth
                implicitWidth: footer.itemWidth
                text: qsTr("Hot Corners…")
                accessibleName: qsTr("Open hot corner settings")
                onClicked: root.openHotCorners()
            }
        }

        Item {
            width: parent.width
            height: 0
        }
    }

    // T292a: the Advanced Dock sheet. The two Linux-only Dock rows keep their
    // real owner, key, and order; the sheet is an organization change.
    Dialog {
        id: advancedDockDialog
        objectName: "desktopDockAdvancedDockDialog"
        title: qsTr("Advanced Dock")
        message: qsTr("Dock behaviour that is specific to this desktop.")

        contentData: Item {
            implicitWidth: 480
            implicitHeight: advancedDockGroup.implicitHeight
            width: parent.width
            height: implicitHeight

            SettingsGroup {
                id: advancedDockGroup
                width: parent.width
                title: qsTr("Advanced Dock")
                sectionId: "Advanced Dock"

                SettingsKeyRow {
                    id: chooserOnHoverRow
                    width: parent.width
                    label: qsTr("Open a window chooser by hovering")
                    settingsId: "chooserOnHover"
                    owner: "settingsd"
                    settingsKey: "dock.chooserOnHover"
                    description: qsTr("Open a grouped app's window chooser when the "
                                      + "pointer dwells on its tile.")
                    controlData: Toggle {
                        id: chooserOnHoverToggle
                        text: ""
                        onToggled: (checked) => Settings.set("dock.chooserOnHover", checked)
                    }
                }

                SettingsKeyRow {
                    id: minimizeReactionRow
                    width: parent.width
                    label: qsTr("Bounce app icons when windows minimize")
                    settingsId: "minimizeReaction"
                    owner: "settingsd"
                    settingsKey: "dock.minimizeReaction"
                    description: qsTr("Bounce an app's Dock tile once when one of "
                                      + "its windows minimizes.")
                    showSeparator: false
                    controlData: Toggle {
                        id: minimizeReactionToggle
                        text: ""
                        onToggled: (checked) => Settings.set("dock.minimizeReaction", checked)
                    }
                }
            }
        }
    }

    // T292a: the Advanced Windows sheet. The window-rules row keeps its
    // disabled-with-reason state and its real owner until a window-rule engine
    // exists.
    Dialog {
        id: advancedWindowsDialog
        objectName: "desktopDockAdvancedWindowsDialog"
        title: qsTr("Advanced Windows")
        message: qsTr("Per-window placement and rule settings. A row the host does "
                      + "not provide stays visible and disabled with the reason.")

        contentData: Item {
            implicitWidth: 480
            implicitHeight: advancedWindowsGroup.implicitHeight
            width: parent.width
            height: implicitHeight

            SettingsGroup {
                id: advancedWindowsGroup
                width: parent.width
                title: qsTr("Advanced Windows")
                sectionId: "Advanced Windows"

                SettingsRow {
                    id: windowRulesRow
                    width: parent.width
                    label: qsTr("Application window rules…")
                    description: qsTr("Per-window placement and size rules are not "
                                      + "available yet.")
                    settingsId: "windowRules"
                    owner: "compositor"
                    settingsKey: "windows.rules"
                    enabled: false
                    showSeparator: false

                    controlData: Button {
                        text: qsTr("Edit…")
                        enabled: false
                    }
                }
            }
        }
    }

    // The footer's text measurements (see `footerGlyphWidth`). One Text per
    // label, invisible and outside the pane column, so they never participate
    // in layout and never form a binding cycle with the buttons.
    Text {
        id: footerMeasure0
        visible: false
        x: -10000
        text: qsTr("Advanced…")
        font.pixelSize: Theme.controls.button.fontSize
        font.weight: Theme.controls.button.fontWeight
    }
    Text {
        id: footerMeasure1
        visible: false
        x: -10000
        text: qsTr("Shortcuts…")
        font.pixelSize: Theme.controls.button.fontSize
        font.weight: Theme.controls.button.fontWeight
    }
    Text {
        id: footerMeasure2
        visible: false
        x: -10000
        text: qsTr("Hot Corners…")
        font.pixelSize: Theme.controls.button.fontSize
        font.weight: Theme.controls.button.fontWeight
    }

    // Two-way bindings: the controls write on interaction; these keep them in
    // step with `Settings.values` (a user edit and an external settingsd change
    // converge).
    Binding {
        target: sizeSlider
        property: "value"
        value: root.sizeValue
    }
    Binding {
        target: magnificationSlider
        property: "value"
        value: root.magnificationValue
    }
    Binding {
        target: positionSelect
        property: "currentIndex"
        value: root.positionIndex()
    }
    Binding {
        target: minimizedAnimationSelect
        property: "currentIndex"
        value: root.minimizedAnimationIndex()
    }
    Binding {
        target: titlebarSelect
        property: "currentIndex"
        value: root.titlebarDoubleClickIndex()
    }
    Binding {
        target: minimizeIntoTileToggle
        property: "checked"
        value: Settings.values["dock.minimizeIntoTileIcon"] === true
    }
    Binding {
        target: autohideToggle
        property: "checked"
        value: Settings.values["dock.autohide"] === true
    }
    Binding {
        target: animateOpeningToggle
        property: "checked"
        value: Settings.values["dock.animateOpening"] !== false
    }
    Binding {
        target: showIndicatorsToggle
        property: "checked"
        value: Settings.values["dock.showIndicators"] !== false
    }
    Binding {
        target: showRecentAppsToggle
        property: "checked"
        value: Settings.values["dock.showRecentApps"] === true
    }
    Binding {
        target: chooserOnHoverToggle
        property: "checked"
        value: Settings.values["dock.chooserOnHover"] === true
    }
    Binding {
        target: minimizeReactionToggle
        property: "checked"
        value: Settings.values["dock.minimizeReaction"] === true
    }
    Binding {
        target: desktopItemsControl
        property: "currentIndex"
        value: root.optionIndex(root.desktopItemsOptions,
                                root.desktopItemsValue, 0)
    }
    Binding {
        target: defaultBrowserSelect
        property: "currentIndex"
        value: root.browserIndex()
    }
}
