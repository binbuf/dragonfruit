// SPDX-License-Identifier: MIT
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
                                  "Toggle", "Popup", "Menu", "SSD", "Buttons",
                                  "Sidebar", "Toolbar", "SplitView", "Settings",
                                  "Segmented", "ContextMenu", "SearchField",
                                  "SourceList", "Dialog", "Sheet", "Popover",
                                  "ScrollView", "Icons"]
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
            case 7: return ssdPageComponent;
            case 8: return buttonsPageComponent;
            case 9: return sidebarPageComponent;
            case 10: return toolbarPageComponent;
            case 11: return splitViewPageComponent;
            case 12: return settingsPageComponent;
            case 13: return segmentedPageComponent;
            case 14: return contextMenuPageComponent;
            case 15: return searchFieldPageComponent;
            case 16: return sourceListPageComponent;
            case 17: return dialogPageComponent;
            case 18: return sheetPageComponent;
            case 19: return popoverPageComponent;
            case 20: return scrollViewPageComponent;
            default: return iconsPageComponent;
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
    Component { id: buttonsPageComponent; ButtonsPage { } }
    Component { id: sidebarPageComponent; SidebarPage { } }
    Component { id: toolbarPageComponent; ToolbarPage { } }
    Component { id: splitViewPageComponent; SplitViewPage { } }
    Component { id: settingsPageComponent; SettingsPage { } }
    Component { id: segmentedPageComponent; SegmentedPage { } }
    Component { id: contextMenuPageComponent; ContextMenuPage { } }
    Component { id: searchFieldPageComponent; SearchFieldPage { } }
    Component { id: sourceListPageComponent; SourceListPage { } }
    Component { id: dialogPageComponent; DialogPage { } }
    Component { id: sheetPageComponent; SheetPage { } }
    Component { id: popoverPageComponent; PopoverPage { } }
    Component { id: scrollViewPageComponent; ScrollViewPage { } }
    Component { id: iconsPageComponent; IconsPage { } }

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
                        { label: qsTr("Open With"), type: "submenu",
                          submenu: [
                              { label: qsTr("Text Editor") },
                              { label: qsTr("Preview") },
                              { label: qsTr("Archive Utility") }
                          ] },
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

    component ButtonsPage: Page {
        Section {
            heading: qsTr("Variants")
            Row {
                spacing: Theme.primitive.spacing.md
                Button { text: qsTr("Save"); variant: "primary" }
                Button { text: qsTr("Cancel") }
                Button { text: qsTr("Delete"); variant: "danger" }
                Button { text: qsTr("More"); variant: "ghost" }
                Button { text: qsTr("Disabled"); enabled: false }
            }
        }
        Section {
            heading: qsTr("With glyph")
            Row {
                spacing: Theme.primitive.spacing.md
                Button { text: qsTr("New Folder"); icon: "zoom"; variant: "primary" }
                Button { text: qsTr("Search"); icon: "search" }
            }
        }
    }

    component SidebarPage: Page {
        Section {
            heading: qsTr("Favorites + Locations (one selected)")
            Sidebar {
                width: 240
                height: 380
                currentIndex: 1
                sections: [
                    { title: qsTr("Favorites"), items: [
                        { label: qsTr("Recents"), icon: "search" },
                        { label: qsTr("Documents"), icon: "check" },
                        { label: qsTr("Downloads"), badge: "3" }
                    ] },
                    { title: qsTr("Locations"), items: [
                        { label: qsTr("Home") },
                        { label: qsTr("Computer") },
                        { label: qsTr("Network") }
                    ] }
                ]
            }
        }
    }

    component ToolbarPage: Page {
        Section {
            heading: qsTr("Navigation + search + actions")
            Toolbar {
                width: 620
                Button { text: qsTr("Back") }
                Button { text: qsTr("Forward"); enabled: false }
                SearchField { width: 200; placeholderText: qsTr("Search") }
                trailingData: Row {
                    spacing: Theme.controls.toolbar.spacing
                    Button { text: qsTr("Share") }
                    Button { text: qsTr("New"); variant: "primary" }
                }
            }
        }
        Section {
            heading: qsTr("Title-only")
            Toolbar {
                width: 620
                title: qsTr("Documents")
            }
        }
    }

    component SplitViewPage: Page {
        Section {
            heading: qsTr("Horizontal")
            SplitView {
                width: 520
                height: 220
                firstData: Rectangle {
                    anchors.fill: parent
                    color: Theme.color.surfaceElevated
                    Text {
                        anchors.centerIn: parent
                        text: qsTr("Sidebar pane")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
                secondData: Rectangle {
                    anchors.fill: parent
                    color: Theme.color.surface
                    Text {
                        anchors.centerIn: parent
                        text: qsTr("Content pane")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
        }
        Section {
            heading: qsTr("Vertical")
            SplitView {
                width: 520
                height: 200
                vertical: true
                splitPosition: 0.5
                firstData: Rectangle { anchors.fill: parent; color: Theme.color.surfaceElevated }
                secondData: Rectangle { anchors.fill: parent; color: Theme.color.surface }
            }
        }
    }

    component SettingsPage: Page {
        Section {
            heading: qsTr("SettingsGroup + SettingsRow")
            SettingsGroup {
                width: 520
                title: qsTr("General")
                SettingsRow {
                    width: parent.width
                    label: qsTr("Appearance")
                    controlData: SegmentedControl {
                        model: [qsTr("Light"), qsTr("Dark"), qsTr("Auto")]
                        currentIndex: 1
                    }
                }
                SettingsRow {
                    width: parent.width
                    label: qsTr("Wi-Fi")
                    description: qsTr("Connect to available networks")
                    controlData: Toggle { checked: true }
                }
                SettingsRow {
                    width: parent.width
                    label: qsTr("Play sound effects")
                    showSeparator: false
                    controlData: Toggle { checked: false }
                }
            }
        }
    }

    component SegmentedPage: Page {
        Section {
            heading: qsTr("Selection + disabled segment")
            SegmentedControl {
                model: [qsTr("Day"), qsTr("Week"), qsTr("Month")]
                currentIndex: 1
            }
        }
        Section {
            heading: qsTr("Two states")
            SegmentedControl {
                model: [{ label: qsTr("On"), enabled: true },
                        { label: qsTr("Off"), enabled: true },
                        { label: qsTr("Auto"), enabled: false }]
                currentIndex: 0
            }
        }
    }

    component ContextMenuPage: Page {
        Section {
            heading: qsTr("Pointer-anchored menu")
            Item {
                width: 520
                height: 300
                ContextMenu {
                    x: 40
                    y: 20
                    open: true
                    accessibleName: qsTr("Item actions")
                    model: [
                        { label: qsTr("Open"), shortcut: "⌘O" },
                        { label: qsTr("Open With"), type: "submenu",
                          submenu: [
                              { label: qsTr("Text Editor") },
                              { label: qsTr("Image Viewer") },
                              { type: "separator" },
                              { label: qsTr("Other…") }
                          ] },
                        { type: "separator" },
                        { label: qsTr("Get Info"), shortcut: "⌘I" },
                        { label: qsTr("Rename"), shortcut: "↵" },
                        { label: qsTr("Show in Sidebar"), checked: true, checkable: true },
                        { type: "separator" },
                        { label: qsTr("Move to Trash"), enabled: false }
                    ]
                }
            }
        }
    }

    component SearchFieldPage: Page {
        Section {
            heading: qsTr("Empty + filled")
            Column {
                spacing: Theme.primitive.spacing.md
                SearchField { width: 260; placeholderText: qsTr("Search files") }
                SearchField { width: 260; text: qsTr("dragonfruit") }
            }
        }
    }

    component SourceListPage: Page {
        Section {
            heading: qsTr("Tree with a collapsed branch")
            SourceList {
                width: 280
                height: 300
                currentIndex: 1
                model: [
                    { label: qsTr("Home"), depth: 0, hasChildren: true, expanded: true },
                    { label: qsTr("Documents"), depth: 1, hasChildren: true, expanded: false },
                    { label: qsTr("Work"), depth: 2 },
                    { label: qsTr("Personal"), depth: 2 },
                    { label: qsTr("Downloads"), depth: 1, badge: "3" },
                    { label: qsTr("Trash"), depth: 0 }
                ]
            }
        }
    }

    component DialogPage: Page {
        Section {
            heading: qsTr("Modal dialog (open)")
            Item {
                width: 520
                height: 340
                Dialog {
                    open: true
                    title: qsTr("Replace existing file?")
                    message: qsTr("A file named “report.pdf” already exists in this folder.")
                    contentData: Text {
                        text: qsTr("Keep Both creates a numbered copy.")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                    buttonsData: Row {
                        spacing: Theme.controls.dialog.buttonGap
                        Button { text: qsTr("Cancel") }
                        Button { text: qsTr("Replace"); variant: "primary" }
                    }
                }
            }
        }
    }

    component SheetPage: Page {
        Section {
            heading: qsTr("Window-attached sheet (open)")
            Item {
                width: 520
                height: 340
                Sheet {
                    open: true
                    title: qsTr("Go to Folder")
                    contentData: SearchField { width: parent.width; placeholderText: qsTr("/home/user") }
                    buttonsData: Row {
                        spacing: Theme.controls.dialog.buttonGap
                        Button { text: qsTr("Cancel") }
                        Button { text: qsTr("Go"); variant: "primary" }
                    }
                }
            }
        }
    }

    component PopoverPage: Page {
        Section {
            heading: qsTr("Anchored popover")
            Item {
                width: 520
                height: 280
                Rectangle {
                    id: popoverAnchor
                    x: 160
                    y: 0
                    width: 140
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
                Popover {
                    anchorItem: popoverAnchor
                    open: true
                    preferredWidth: 260
                    accessibleName: qsTr("Inspector")
                    Column {
                        spacing: Theme.primitive.spacing.xs
                        Text {
                            text: qsTr("Document inspector")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeMd
                            font.weight: Theme.primitive.font.weightSemibold
                        }
                        Text {
                            text: qsTr("128 KB · PDF document")
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                        }
                    }
                }
            }
        }
    }

    component ScrollViewPage: Page {
        Section {
            heading: qsTr("Scrollable list")
            ScrollView {
                width: 420
                height: 260
                Column {
                    width: parent.width
                    Repeater {
                        model: 24
                        delegate: Item {
                            required property int index
                            width: parent.width
                            height: Theme.controls.sidebar.rowHeight
                            Rectangle {
                                anchors.fill: parent
                                color: index % 2 === 0 ? Theme.color.surface : Theme.color.surfaceMuted
                            }
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                x: Theme.primitive.spacing.md
                                text: qsTr("Row %1").arg(index + 1)
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.controls.button.fontSize
                            }
                        }
                    }
                }
            }
        }
    }

    component IconsPage: Page {
        Section {
            heading: qsTr("Glyphs")
            Row {
                spacing: Theme.primitive.spacing.xl
                Repeater {
                    model: ["close", "minimize", "zoom", "check", "chevron-down",
                            "chevron-right", "search"]
                    delegate: Column {
                        required property string modelData
                        spacing: Theme.primitive.spacing.xs
                        Icon {
                            anchors.horizontalCenter: parent.horizontalCenter
                            name: modelData
                            size: Theme.primitive.spacing.xl
                            color: Theme.color.textPrimary
                        }
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: modelData
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeXs
                        }
                    }
                }
            }
        }
    }
}
