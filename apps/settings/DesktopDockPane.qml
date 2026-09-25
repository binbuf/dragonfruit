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
    property alias dockGroup: dockGroup

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

            SettingsRow {
                width: parent.width
                label: qsTr("Size")
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

            SettingsRow {
                width: parent.width
                label: qsTr("Magnification")
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

            SettingsRow {
                width: parent.width
                label: qsTr("Dock position on screen")
                controlData: Select {
                    id: positionSelect
                    accessibleName: qsTr("Dock position on screen")
                    model: root.positionOptions
                    onActivated: (index) => Settings.set("dock.position",
                                                         root.positionOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Minimized window animation")
                controlData: Select {
                    id: minimizedAnimationSelect
                    accessibleName: qsTr("Minimized window animation")
                    model: root.minimizedAnimationOptions
                    onActivated: (index) => Settings.set(
                                    "dock.minimizedAnimation",
                                    root.minimizedAnimationOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Window title bar double-click action")
                controlData: Select {
                    id: titlebarSelect
                    accessibleName: qsTr("Window title bar double-click action")
                    model: root.titlebarDoubleClickOptions
                    onActivated: (index) => Settings.set(
                                    "dock.titlebarDoubleClick",
                                    root.titlebarDoubleClickOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Minimize windows into application icon")
                controlData: Toggle {
                    id: minimizeIntoTileToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.minimizeIntoTileIcon", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Automatically hide and show the Dock")
                controlData: Toggle {
                    id: autohideToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.autohide", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Animate opening applications")
                controlData: Toggle {
                    id: animateOpeningToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.animateOpening", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show indicators for open applications")
                controlData: Toggle {
                    id: showIndicatorsToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.showIndicators", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show suggested and recent apps in Dock")
                showSeparator: false
                controlData: Toggle {
                    id: showRecentAppsToggle
                    text: ""
                    onToggled: (checked) => Settings.set("dock.showRecentApps", checked)
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
}