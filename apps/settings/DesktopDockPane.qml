// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Desktop & Dock pane (T-09.4; T276 completion).
//
// Every live control is a stock design-system component bound to the `Settings`
// singleton (never D-Bus directly) with the T-09.1b write-on-interaction /
// bind-to-`Settings.values` pattern, so a user edit and an external settingsd
// change converge without a restart.
//
// The Dock group ships every `dock.*` key the shell applies live plus the
// existing Linux extras (window chooser, minimize reaction, global app menu).
// The Desktop & Stage, Widgets, and Windows groups follow the macOS row
// inventory; a row whose runtime is not implemented yet is disabled with an
// honest reason (the owning task lives in the map and docs, never in the UI).
// The Mission Control group links to the Overview pane rather than duplicating
// its corner controls.
Item {
    id: root

    signal navigateRequested(string paneId)

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
    // T227/T276: Desktop & Stage Manager. `desktop.showItems` is consumed by
    // the Files desktop process; the remaining rows name their missing runtime
    // in the description and stay disabled (T242/T245b).
    readonly property var desktopItemsOptions: [
        { value: "on-desktop", label: qsTr("On Desktop") },
        { value: "in-stage", label: qsTr("In Stage") }
    ]
    readonly property string desktopItemsValue:
        Settings.values["desktop.showItems"] || "on-desktop"
    readonly property var windowsFromAppOptions: [
        { value: "all", label: qsTr("All") },
        { value: "one", label: qsTr("One") }
    ]

    // T276: the default-browser row's installed choices come from app-index
    // through the Settings bridge. Empty is the honest no-browser state.
    readonly property var browserChoices: Settings.defaultBrowserChoices
    readonly property bool browserAvailable: browserChoices.length > 0

    // Test surface (used by tst_settings_desktop_dock.qml).
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
    property alias globalMenuToggle: globalMenuToggle
    property alias menuGroup: menuGroup
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
    property alias windowsGroup: windowsGroup
    property alias preferTabsRow: preferTabsRow
    property alias askToKeepChangesRow: askToKeepChangesRow
    property alias closeOnQuitRow: closeOnQuitRow
    property alias dragToTileRow: dragToTileRow
    property alias dragToFillRow: dragToFillRow
    property alias holdModifierRow: holdModifierRow
    property alias tiledMarginsRow: tiledMarginsRow
    property alias windowRulesRow: windowRulesRow
    property alias missionControlGroup: missionControlGroup
    property alias overviewRow: overviewRow
    property alias hotCornersRow: hotCornersRow
    property alias shortcutsRow: shortcutsRow
    property alias defaultBrowserRow: defaultBrowserRow
    property alias defaultBrowserSelect: defaultBrowserSelect

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

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
                label: qsTr("Size")
                settingsId: "size"
                owner: "settingsd"
                settingsKey: "dock.size"
                controlData: Slider {
                    id: sizeSlider
                    width: 220
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
                label: qsTr("Magnification")
                settingsId: "magnification"
                owner: "settingsd"
                settingsKey: "dock.magnification"
                controlData: Slider {
                    id: magnificationSlider
                    width: 220
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
                label: qsTr("Position on screen")
                settingsId: "position"
                owner: "settingsd"
                settingsKey: "dock.position"
                controlData: Select {
                    id: positionSelect
                    accessibleName: qsTr("Position on screen")
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
                controlData: Toggle {
                    id: showRecentAppsToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.showRecentApps", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Open a window chooser by hovering")
                settingsId: "chooserOnHover"
                owner: "settingsd"
                settingsKey: "dock.chooserOnHover"
                controlData: Toggle {
                    id: chooserOnHoverToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.chooserOnHover", checked)
                }
            }

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Bounce app icons when windows minimize")
                settingsId: "minimizeReaction"
                owner: "settingsd"
                settingsKey: "dock.minimizeReaction"
                showSeparator: false
                controlData: Toggle {
                    id: minimizeReactionToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.minimizeReaction", checked)
                }
            }
        }

        // ── Desktop & Stage ───────────────────────────────────────────────
        SettingsGroup {
            id: desktopGroup
            width: parent.width
            title: qsTr("Desktop & Stage")

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Show items")
                settingsId: "desktopItems"
                owner: "settingsd -> apps/files desktop"
                settingsKey: "desktop.showItems"
                description: qsTr("On Desktop shows the Files desktop icons; In Stage "
                                  + "leaves them to Stage.")
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
                label: qsTr("Click wallpaper to reveal desktop")
                description: qsTr("Reveal-on-click is not available yet; the desktop "
                                  + "layer owns background clicks.")
                settingsId: "clickWallpaper"
                owner: "compositor"
                settingsKey: "desktop.clickWallpaper"
                enabled: false

                controlData: Select {
                    model: [qsTr("Always"), qsTr("Only in Stage"), qsTr("Never")]
                    enabled: false
                }
            }

            SettingsRow {
                id: stageRow
                width: parent.width
                label: qsTr("Stage")
                description: qsTr("Stage is not available yet; window staging is not "
                                  + "implemented.")
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
                label: qsTr("Recent apps in Stage")
                description: qsTr("Available with Stage.")
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
                description: qsTr("Available with Stage.")
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
                showSeparator: false

                controlData: Select {
                    model: [qsTr("Always"), qsTr("When windows are open"), qsTr("Never")]
                    enabled: false
                }
            }
        }

        // ── Windows ───────────────────────────────────────────────────────
        SettingsGroup {
            id: windowsGroup
            width: parent.width
            title: qsTr("Windows")

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
                label: qsTr("Drag windows to screen edges to tile")
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
                label: qsTr("Drag windows to the menu bar to fill the screen")
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
                label: qsTr("Hold Option while dragging to tile")
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

                controlData: Toggle {
                    text: ""
                    enabled: false
                }
            }

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

        // ── Mission Control / Overview ────────────────────────────────────
        SettingsGroup {
            id: missionControlGroup
            width: parent.width
            title: qsTr("Window Overview")

            SettingsRow {
                id: overviewRow
                width: parent.width
                label: qsTr("Overview")
                description: qsTr("Workspaces, gestures, and window arrangement.")
                settingsId: "overview"
                owner: "settingsd + compositor"
                settingsKey: ""
                disclosure: true
                onDisclosureClicked: root.navigateRequested("mission-control")
            }

            SettingsRow {
                id: hotCornersRow
                width: parent.width
                label: qsTr("Hot Corners…")
                description: qsTr("Choose an action and optional modifier for each "
                                  + "screen corner.")
                settingsId: "hotCorners"
                owner: "settingsd + compositor"
                settingsKey: ""
                disclosure: true
                onDisclosureClicked: root.navigateRequested("mission-control")
            }

            SettingsRow {
                id: shortcutsRow
                width: parent.width
                label: qsTr("Shortcuts…")
                description: qsTr("The shortcut editor is not available yet; system "
                                  + "shortcuts are fixed.")
                settingsId: "shortcuts"
                owner: "compositor"
                settingsKey: "shortcuts.*"
                enabled: false
                showSeparator: false
            }
        }

        // ── Default browser ───────────────────────────────────────────────
        SettingsGroup {
            id: defaultAppsGroup
            width: parent.width
            title: qsTr("Default web browser")

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

        // T-14.2b: the global application-menu toggle. On macOS this lives
        // under "Menu Bar"; the dedicated pane is T-15.9b, so the one control
        // ships here in its own group until then.
        SettingsGroup {
            id: menuGroup
            width: parent.width
            title: qsTr("Menu Bar")

            SettingsKeyRow {
                width: parent.width
                label: qsTr("Show application menus in the menu bar")
                settingsId: "menuBarToggle"
                owner: "settingsd"
                settingsKey: "menu.global"
                showSeparator: false
                controlData: Toggle {
                    id: globalMenuToggle
                    text: ""
                    onToggled: (checked) => Settings.set("menu.global", checked)
                }
            }
        }
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
        target: globalMenuToggle
        property: "checked"
        value: Settings.values["menu.global"] !== false
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
