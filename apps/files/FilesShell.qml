// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Files app shell (T-10.4a): the window chrome (titlebar + traffic
// lights), the sidebar with the Favorites/Locations sections, the toolbar
// (back/forward, the view switch, local search), and the footer path bar.
//
// The browsing model is `FilesBrowser`; the locations and breadcrumb come
// from the `Files` platform singleton. Everything visual is a design-system
// component — this file owns only the information architecture and the
// navigation state. The list/icon views over the directory model land in
// T-10.4b, so the detail area holds the location's empty state until then.
//
// Keyboard: the sidebar owns Up/Down/Home/End/Return; Alt+Left/Alt+Right step
// history; Ctrl+F focuses search; Escape clears it.
Item {
    id: root

    property string searchText: ""
    // Guards the sidebar selection sync so setting `currentIndex`
    // programmatically is not mistaken for a user navigation.
    property bool syncing: false

    property alias browser: browser
    property alias sidebar: sidebar
    property alias toolbar: toolbar
    property alias titleBar: titleBar
    property alias appWindow: appWindow
    property alias searchField: searchField
    property alias backButton: backButton
    property alias forwardButton: forwardButton
    property alias viewControl: viewControl
    property alias pathBar: pathBar
    property alias emptyState: emptyState

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal moveRequested(real x, real y)
    signal menuRequested(real x, real y)

    readonly property string locationTitle: Files.displayName(browser.currentUri)
    readonly property var breadcrumb: Files.breadcrumb(browser.currentUri)
    readonly property bool hasSearch: root.searchText.length > 0

    // The sidebar sections the design doc names: Favorites (real XDG user
    // directories), then Locations (Computer, mounted volumes, Trash last).
    readonly property var sidebarSections: {
        var favorites = [];
        var favs = Files.favorites;
        for (var i = 0; i < favs.length; ++i) {
            favorites.push({ label: favs[i].label, icon: favs[i].icon, uri: favs[i].uri });
        }
        var locations = [{ label: Files.userName, icon: "home",
                           uri: Files.homeUri },
                         { label: qsTr("Computer"), icon: "computer",
                           uri: Files.computerUri }];
        var vols = Files.volumes;
        for (var v = 0; v < vols.length; ++v) {
            locations.push({ label: vols[v].label, icon: vols[v].icon,
                             uri: vols[v].uri });
        }
        locations.push({ label: qsTr("Trash"), icon: "trash", uri: Files.trashUri });
        return [
            { title: qsTr("Favorites"), items: favorites },
            { title: qsTr("Locations"), items: locations }
        ];
    }

    // The location a sidebar entry points at. The Sidebar flattens sections
    // (adding header rows), so the source section/item is recovered by index.
    function uriForEntry(entry) {
        if (!entry || entry.type !== "item")
            return "";
        var section = root.sidebarSections[entry.section];
        if (!section)
            return "";
        var item = section.items[entry.itemIndex];
        return item ? item.uri : "";
    }

    function entryIndexForUri(uri) {
        var entries = sidebar.entries;
        for (var i = 0; i < entries.length; ++i) {
            if (root.uriForEntry(entries[i]) === uri)
                return i;
        }
        return -1;
    }

    function syncSelection() {
        root.syncing = true;
        sidebar.currentIndex = root.entryIndexForUri(browser.currentUri);
        root.syncing = false;
    }

    function focusSearch() {
        searchField.forceActiveFocus();
    }

    Component.onCompleted: {
        var start = Files.homeUri;
        // `DF_FILES_START_URI` opens a specific location for captures and
        // scripted checks; an unusable URI falls back to Home.
        if (Files.startUri.length > 0 && Files.isBrowsable(Files.startUri))
            start = Files.startUri;
        browser.reset(start);
        root.syncSelection();
    }

    onSearchTextChanged: {
        if (searchField.text !== root.searchText)
            searchField.text = root.searchText;
    }

    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Left:
            if (event.modifiers & Qt.AltModifier) {
                browser.back();
                event.accepted = true;
            }
            break;
        case Qt.Key_Right:
            if (event.modifiers & Qt.AltModifier) {
                browser.forward();
                event.accepted = true;
            }
            break;
        case Qt.Key_F:
            if (event.modifiers & Qt.ControlModifier) {
                root.focusSearch();
                event.accepted = true;
            }
            break;
        case Qt.Key_BracketLeft:
            if (event.modifiers & Qt.ControlModifier) {
                browser.back();
                event.accepted = true;
            }
            break;
        case Qt.Key_BracketRight:
            if (event.modifiers & Qt.ControlModifier) {
                browser.forward();
                event.accepted = true;
            }
            break;
        }
    }

    FilesBrowser {
        id: browser
        defaultView: "icon"
        onCurrentUriChanged: root.syncSelection()
        onNavigated: root.syncSelection()
    }

    AppWindow {
        id: appWindow
        anchors.fill: parent
        title: root.locationTitle
        active: true

        titleBarData: TitleBar {
            id: titleBar
            title: root.locationTitle
            active: true
            onCloseRequested: root.closeRequested()
            onMinimizeRequested: root.minimizeRequested()
            onZoomRequested: root.zoomRequested()
            onMoveRequested: (x, y) => root.moveRequested(x, y)
            onMenuRequested: (x, y) => root.menuRequested(x, y)
        }

        Row {
            anchors.fill: parent
            spacing: 0

            // Sidebar: the Favorites and Locations source lists.
            Rectangle {
                id: sidebarPane
                width: 240
                height: parent.height
                color: Theme.color.surfaceMuted

                Sidebar {
                    id: sidebar
                    anchors.fill: parent
                    anchors.margins: Theme.controls.sidebar.padding
                    sections: root.sidebarSections

                    onActivated: (index, item) => {
                        var uri = root.uriForEntry(item);
                        if (uri)
                            browser.navigate(uri);
                    }
                    onCurrentIndexChanged: {
                        if (root.syncing)
                            return;
                        var uri = root.uriForEntry(sidebar.entries[sidebar.currentIndex]);
                        if (uri)
                            browser.navigate(uri);
                    }
                }
            }

            // Detail: toolbar, the location's content, and the path bar.
            Rectangle {
                width: parent.width - sidebarPane.width
                height: parent.height
                color: Theme.color.surface

                Column {
                    anchors.fill: parent
                    spacing: 0

                    Toolbar {
                        id: toolbar
                        width: parent.width

                        Button {
                            id: backButton
                            icon: "chevron-left"
                            variant: "ghost"
                            accessibleName: qsTr("Back")
                            enabled: browser.canGoBack
                            onClicked: browser.back()
                        }
                        Button {
                            id: forwardButton
                            icon: "chevron-right"
                            variant: "ghost"
                            accessibleName: qsTr("Forward")
                            enabled: browser.canGoForward
                            onClicked: browser.forward()
                        }

                        SegmentedControl {
                            id: viewControl
                            model: [
                                { label: qsTr("Icon View"), icon: "icon-view" },
                                { label: qsTr("List View"), icon: "list-view" }
                            ]
                            onActivated: (index) => browser.setView(index === 1
                                                                    ? "list" : "icon")
                        }

                        trailingData: SearchField {
                            id: searchField
                            width: 220
                            placeholderText: qsTr("Search")
                            onTextChanged: root.searchText = searchField.text
                            onCleared: root.searchText = ""
                        }
                    }

                    // The location content. The directory listing and its
                    // list/icon views are T-10.4b; until then this is the
                    // designed empty/search state.
                    Item {
                        id: contentArea
                        width: parent.width
                        height: Math.max(0, parent.height - toolbar.height - pathBar.height)

                        Column {
                            id: emptyState
                            anchors.centerIn: parent
                            width: Math.min(parent.width - 2 * Theme.primitive.spacing.xl, 360)
                            spacing: Theme.primitive.spacing.sm

                            Icon {
                                anchors.horizontalCenter: parent.horizontalCenter
                                name: root.hasSearch ? "search" : "folder"
                                size: Theme.primitive.spacing.xxl
                                color: Theme.color.textTertiary
                            }

                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                text: root.hasSearch
                                      ? qsTr("Search \u201c%1\u201d in %2")
                                            .arg(root.searchText).arg(root.locationTitle)
                                      : root.locationTitle
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeLg
                                font.weight: Theme.primitive.font.weightMedium
                                elide: Text.ElideRight
                            }

                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                text: root.hasSearch
                                      ? qsTr("Results will appear here.")
                                      : qsTr("This folder\u2019s items will appear here.")
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    PathBar {
                        id: pathBar
                        width: parent.width
                        model: root.breadcrumb
                        onNavigated: (uri) => browser.navigate(uri)
                    }
                }
            }
        }
    }

    // Keep the view switch in step with the location's persisted view state;
    // the control writes on activation, this re-reads it on navigation.
    Binding {
        target: viewControl
        property: "currentIndex"
        value: browser.currentView === "list" ? 1 : 0
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Files")
}