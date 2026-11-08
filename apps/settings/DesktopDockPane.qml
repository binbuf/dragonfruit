// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Desktop & Dock pane (T-09.4). Every control is a stock design-system
// component bound to the `Settings` singleton (never D-Bus directly) with the
// T-09.1b write-on-interaction / bind-to-`Settings.values` pattern, so a user
// edit and an external settingsd change converge without a restart.
//
// This slice ships the `Dock` group — every `dock.*` key from T-08 (size,
// magnification, position, minimized-window animation, titlebar double-click
// action, minimize-into-icon, auto-hide, animate opening, indicators, suggested
// and recent apps). The shell's `ShellController::applyDockSettings` is the one
// applier: a settingsd `Changed` re-lays-out the Dock live. `dock.pinned` has
// no reference row (the Dock reorders by drag) and stays absent here.
//
// The reference pane's "Desktop & Stage Manager" group is omitted: it maps to
// Desktop Reveal / hot corners, which have no settings schema key yet
// (the no-half-panes rule; the provider is T-15.5).
Item {
    id: root

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
    // T227: Desktop & Stage Manager. `desktop.showItems` is consumed by the
    // Files desktop process; Stage Manager itself is T242, so its rows are
    // disabled with a reason rather than advertised as live toggles.
    readonly property var desktopItemsOptions: [
        { value: "on-desktop", label: qsTr("On Desktop") },
        { value: "in-stage", label: qsTr("In Stage") }
    ]
    readonly property string desktopItemsValue:
        Settings.values["desktop.showItems"] || "on-desktop"

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
    property alias widgetsRow: widgetsRow

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

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
                    onMoved: (value) => Settings.set("dock.size", value)
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
                    onMoved: (value) => Settings.set("dock.magnification", value)
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
                settingsId: "minimizedAnimation"
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

        // T227: Desktop & Stage Manager. Only "Show items" is live today; the
        // Files desktop process consumes `desktop.showItems`. The remaining
        // rows name the task that owns their runtime (T242/T245b).
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
                                  + "leaves them to Stage (T242).")
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
                description: qsTr("Background clicks belong to the desktop layer; "
                                  + "reveal-on-click lands with T245b.")
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
                description: qsTr("Stage lands with T242.")
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
                description: qsTr("Lands with Stage (T242).")
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
                id: widgetsRow
                width: parent.width
                label: qsTr("Widgets")
                description: qsTr("No widget host is implemented; widgets remain "
                                  + "an explicit OPEN row.")
                settingsId: "widgets"
                owner: "shell"
                settingsKey: "widgets.*"
                enabled: false
                showSeparator: false

                controlData: Text {
                    text: qsTr("Unavailable")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
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
                settingsId: "globalMenu"
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
}