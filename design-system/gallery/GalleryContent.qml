// SPDX-License-Identifier: LGPL-3.0-or-later
import QtQuick
import Dragonfruit

// The component gallery. It renders the gated taste slice in every state,
// scheme, and motion variant so a human can sign off on art direction and the
// visual-regression test can sample the same surfaces. `pageIndex`, `scheme`,
// and `reducedMotion` are the knobs the snapshot driver turns.
Item {
    id: gallery

    property int pageIndex: 0
    readonly property var pages: ["Tokens", "Window", "TitleBar", "TrafficLights",
                                  "Toggle", "Popup", "Menu", "SSD"]
    property string scheme: "dark"
    property bool reducedMotion: false

    implicitWidth: 1000
    implicitHeight: 720

    onSchemeChanged: Theme.dark = (scheme === "dark")
    onReducedMotionChanged: Theme.reducedMotion = reducedMotion
    Component.onCompleted: {
        Theme.dark = (scheme === "dark");
        Theme.reducedMotion = reducedMotion;
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.color.surfaceSunken
    }

    Loader {
        anchors.fill: parent
        anchors.margins: Theme.primitive.spacing.xl
        sourceComponent: {
            switch (gallery.pageIndex) {
            case 0: return tokensPageComponent;
            case 1: return windowPageComponent;
            case 2: return titleBarPageComponent;
            case 3: return trafficLightsPageComponent;
            case 4: return togglePageComponent;
            case 5: return popupPageComponent;
            case 6: return menuPageComponent;
            default: return ssdPageComponent;
            }
        }
    }

    Component { id: tokensPageComponent; TokensPage { } }
    Component { id: windowPageComponent; WindowPage { } }
    Component { id: titleBarPageComponent; TitleBarPage { } }
    Component { id: trafficLightsPageComponent; TrafficLightsPage { } }
    Component { id: togglePageComponent; TogglePage { } }
    Component { id: popupPageComponent; PopupPage { } }
    Component { id: menuPageComponent; MenuPage { } }
    Component { id: ssdPageComponent; SsdPage { } }

    component Page: Column {
        spacing: Theme.primitive.spacing.lg
    }

    component Section: Column {
        property string heading: ""
        spacing: Theme.primitive.spacing.sm

        Text {
            text: parent.heading
            color: Theme.color.textPrimary
            font.pixelSize: Theme.primitive.font.sizeLg
            font.weight: Theme.primitive.font.weightSemibold
        }
    }

    component Swatch: Column {
        property string label: ""
        property color swatch: "transparent"
        spacing: Theme.primitive.spacing.xxs

        Rectangle {
            width: 72
            height: 44
            radius: Theme.primitive.radius.sm
            color: parent.swatch
            border.width: 1
            border.color: Theme.color.border
            antialiasing: true
        }
        Text {
            text: parent.label
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeXs
        }
    }

    component TokensPage: Page {
        Section {
            heading: qsTr("Semantic colors — %1").arg(gallery.scheme)
            Row {
                spacing: Theme.primitive.spacing.sm
                Swatch { label: "surface"; swatch: Theme.color.surface }
                Swatch { label: "elevated"; swatch: Theme.color.surfaceElevated }
                Swatch { label: "accent"; swatch: Theme.color.accent }
                Swatch { label: "textPrimary"; swatch: Theme.color.textPrimary }
                Swatch { label: "border"; swatch: Theme.color.border }
                Swatch { label: "close"; swatch: Theme.color.close }
                Swatch { label: "minimize"; swatch: Theme.color.minimize }
                Swatch { label: "zoom"; swatch: Theme.color.zoom }
            }
        }
        Section {
            heading: qsTr("Radius")
            Row {
                spacing: Theme.primitive.spacing.md
                Repeater {
                    model: ["xs", "sm", "md", "lg", "xl", "pill"]
                    delegate: Rectangle {
                        required property string modelData
                        width: 56
                        height: 56
                        radius: modelData === "pill" ? height / 2
                                                      : Theme.primitive.radius[modelData]
                        color: Theme.color.accentMuted
                        border.width: 1
                        border.color: Theme.color.border
                        antialiasing: true
                    }
                }
            }
        }
        Section {
            heading: qsTr("Motion (duration ms)")
            Column {
                spacing: Theme.primitive.spacing.xxs
                Repeater {
                    model: ["menuOpen", "popupOpen", "toggle", "spacesSwitch", "dockMagnify"]
                    delegate: Text {
                        required property string modelData
                        text: modelData + "  " + Theme.motion[modelData].duration + " ms"
                              + (gallery.reducedMotion ? "  (reduced)" : "")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
        }
    }

    component WindowPage: Page {
        Section {
            heading: qsTr("AppWindow + TitleBar")
            AppWindow {
                width: 520
                height: 320
                title: qsTr("Dragonfruit Settings")
                titleBarData: TitleBar {
                    width: parent.width
                    title: qsTr("Dragonfruit Settings")
                }
                contentData: Column {
                    anchors.centerIn: parent
                    spacing: Theme.primitive.spacing.md
                    Text {
                        text: qsTr("Content area")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeMd
                    }
                    Toggle { text: qsTr("Reduce transparency"); checked: true }
                }
            }
        }
    }

    component TitleBarPage: Page {
        Section {
            heading: qsTr("Active")
            TitleBar { width: 520; title: qsTr("Active Window"); active: true }
        }
        Section {
            heading: qsTr("Inactive")
            TitleBar { width: 520; title: qsTr("Inactive Window"); active: false }
        }
        Section {
            heading: qsTr("Traffic lights revealed")
            TitleBar { width: 520; title: qsTr("Hovered"); trafficLights.forceReveal: true }
        }
    }

    component TrafficLightsPage: Page {
        Section {
            heading: qsTr("Default (glyphs hidden)")
            TrafficLights { active: true }
        }
        Section {
            heading: qsTr("Revealed")
            TrafficLights { active: true; forceReveal: true }
        }
        Section {
            heading: qsTr("Inactive window")
            TrafficLights { active: false; forceReveal: true }
        }
    }

    component TogglePage: Page {
        Section {
            heading: qsTr("States")
            Column {
                spacing: Theme.primitive.spacing.md
                Toggle { text: qsTr("Wi-Fi"); checked: false }
                Toggle { text: qsTr("Bluetooth"); checked: true }
                Toggle { text: qsTr("Reduced motion"); checked: gallery.reducedMotion }
                Toggle { text: qsTr("Disabled"); checked: true; enabled: false }
                Toggle {
                    text: qsTr("Sound")
                    description: qsTr("Play interface sound effects")
                    checked: true
                }
            }
        }
    }

    component PopupPage: Page {
        Section {
            heading: qsTr("Popover")
            Item {
                width: 360
                height: 240
                Rectangle {
                    id: popupAnchor
                    width: 120
                    height: 28
                    radius: Theme.controls.toggle.height / 2
                    color: Theme.color.controlFill
                    Text {
                        anchors.centerIn: parent
                        text: qsTr("Anchor")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
                Popup {
                    anchorItem: popupAnchor
                    open: true
                    preferredWidth: 260
                    accessibleName: qsTr("Bluetooth")
                    Column {
                        spacing: Theme.primitive.spacing.xs
                        Row {
                            spacing: Theme.primitive.spacing.sm
                            Text {
                                text: qsTr("Bluetooth")
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeMd
                                font.weight: Theme.primitive.font.weightSemibold
                            }
                            Toggle { checked: true }
                        }
                        Rectangle {
                            width: parent.width
                            height: 1
                            color: Theme.color.separator
                        }
                        Text {
                            text: qsTr("WF-1000XM6")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeMd
                        }
                        Text {
                            text: qsTr("WH-1000XM6")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeMd
                        }
                    }
                }
            }
        }
    }

    component MenuPage: Page {
        Section {
            heading: qsTr("MenuBarMenu (open)")
            // Reserve room for the dropdown, which overflows the bar item.
            Item {
                width: parent.width
                height: 300
                MenuBarMenu {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    title: qsTr("File")
                    open: true
                    model: [
                        { label: qsTr("New Window"), shortcut: "⌘N" },
                        { label: qsTr("New Folder"), shortcut: "⇧⌘N" },
                        { type: "separator" },
                        { label: qsTr("Open…"), shortcut: "⌘O" },
                        { label: qsTr("Close Window"), shortcut: "⌘W" },
                        { type: "separator" },
                        { label: qsTr("Show in Sidebar"), checked: true, checkable: true },
                        { label: qsTr("Unavailable"), enabled: false }
                    ]
                }
            }
        }
        Section {
            heading: qsTr("MenuBarMenu (closed)")
            MenuBarMenu {
                title: qsTr("Edit")
                model: [
                    { label: qsTr("Undo"), shortcut: "⌘Z" },
                    { label: qsTr("Redo"), shortcut: "⇧⌘Z" }
                ]
            }
        }
    }

    component SsdPage: Page {
        Section {
            heading: qsTr("FR-3 — app TitleBar vs compositor SSD reference")
            Text {
                text: qsTr("These must render identically at the same tokens.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Column {
                spacing: Theme.primitive.spacing.sm
                TitleBar { width: 520; title: qsTr("Dragonfruit") }
                SsdTitlebarReference { width: 520; title: qsTr("Dragonfruit") }
            }
        }
    }
}
