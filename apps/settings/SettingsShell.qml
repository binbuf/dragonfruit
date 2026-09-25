// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings app shell (T-09.1a): the window chrome (titlebar + traffic
// lights), the sidebar with local pane search, the back/forward history
// affordances, and the detail-pane header card.
//
// Everything visual is a design-system component; this file only owns the
// information architecture and the pane navigation state. Pane bodies land in
// T-09.2…T-09.5 and register below; until a body exists the shell shows the
// pane's header card and a one-line note, and the pane is not advertised in
// the catalog (`SettingsPanes.shipped`) — the no-half-panes rule.
//
// Keyboard: the sidebar owns Up/Down/Home/End/Return; Alt+Left/Alt+Right step
// history; Ctrl+F focuses search; Escape clears it.
Item {
    id: root

    property string searchText: ""
    property string currentPaneId: ""
    property var history: []
    property int historyIndex: -1
    // Guards the sidebar selection sync so setting `currentIndex`
    // programmatically is not mistaken for a user navigation.
    property bool syncing: false

    property alias searchField: searchField
    property alias sidebar: sidebar
    property alias titleBar: titleBar
    property alias appWindow: appWindow
    property alias header: header
    property alias backButton: backButton
    property alias forwardButton: forwardButton
    property alias paneBody: paneBody

    // The app-local theme owner (T-09.2). The shell's `ThemeBinding` lives in
    // the shell process; the Settings app is a separate process, so these
    // bindings mirror the same settings keys onto this engine's `Theme`
    // singleton. A change in the Appearance pane restyles the app immediately,
    // and an external settingsd change reaches it through `Settings.values`.
    // `auto` tracks the host style hint live.
    readonly property string colorScheme: Settings.values["appearance.colorScheme"] || "auto"
    readonly property bool hostDark: Application.styleHints.colorScheme === Qt.Dark

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal moveRequested(real x, real y)
    signal menuRequested(real x, real y)

    readonly property var visiblePanes: SettingsPanes.filter(root.searchText)
    readonly property var currentPane: SettingsPanes.paneById(root.currentPaneId)
    readonly property bool canGoBack: root.historyIndex > 0
    readonly property bool canGoForward: root.historyIndex >= 0
                                       && root.historyIndex < root.history.length - 1

    implicitWidth: 900
    implicitHeight: 600

    function visibleIndexOf(id) {
        for (var i = 0; i < root.visiblePanes.length; ++i) {
            if (root.visiblePanes[i].id === id)
                return i;
        }
        return -1;
    }

    // Show a pane, recording the move in history. Selecting the current pane
    // is a no-op so arrow-key echo does not grow the stack.
    function selectPane(id) {
        if (!id || id === root.currentPaneId)
            return;
        var next = root.history.slice(0, root.historyIndex + 1);
        next.push(id);
        root.history = next;
        root.historyIndex = next.length - 1;
        root.currentPaneId = id;
        root.syncSelection();
    }

    function back() {
        if (!root.canGoBack)
            return;
        root.historyIndex -= 1;
        root.currentPaneId = root.history[root.historyIndex];
        root.syncSelection();
    }

    function forward() {
        if (!root.canGoForward)
            return;
        root.historyIndex += 1;
        root.currentPaneId = root.history[root.historyIndex];
        root.syncSelection();
    }

    function syncSelection() {
        root.syncing = true;
        sidebar.currentIndex = root.visibleIndexOf(root.currentPaneId);
        root.syncing = false;
    }

    function focusSearch() {
        searchField.forceActiveFocus();
    }

    // The pane-body registry: a pane appears here once its content has landed.
    // T-09.3…T-09.5 add their components alongside "appearance".
    function paneComponent(id) {
        switch (id) {
        case "appearance":
            return appearancePaneComponent;
        case "wallpaper":
            return wallpaperPaneComponent;
        default:
            return null;
        }
    }

    Component.onCompleted: {
        var first = root.visiblePanes.length > 0 ? root.visiblePanes[0].id : "";
        // `DF_SETTINGS_START_PANE` opens a specific shipped pane for captures
        // and scripted checks; an unknown or unshipped id is ignored.
        var requested = Settings.startPane;
        if (requested.length > 0 && SettingsPanes.indexOf(requested) >= 0)
            first = requested;
        if (first.length > 0) {
            root.history = [first];
            root.historyIndex = 0;
            root.currentPaneId = first;
            root.syncSelection();
        }
    }

    onSearchTextChanged: {
        if (searchField.text !== root.searchText)
            searchField.text = root.searchText;
        root.syncSelection();
    }

    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Left:
            if (event.modifiers & Qt.AltModifier) {
                root.back();
                event.accepted = true;
            }
            break;
        case Qt.Key_Right:
            if (event.modifiers & Qt.AltModifier) {
                root.forward();
                event.accepted = true;
            }
            break;
        case Qt.Key_F:
            if (event.modifiers & Qt.ControlModifier) {
                root.focusSearch();
                event.accepted = true;
            }
            break;
        }
    }

    Binding {
        target: Theme
        property: "dark"
        value: root.colorScheme === "dark" ? true
             : root.colorScheme === "light" ? false : root.hostDark
    }
    Binding {
        target: Theme
        property: "accentOverride"
        value: Settings.values["appearance.accent"] || ""
    }
    Binding {
        target: Theme
        property: "reducedMotion"
        value: Settings.values["accessibility.reduceMotion"] === true
    }

    Component {
        id: appearancePaneComponent
        AppearancePane { }
    }

    Component {
        id: wallpaperPaneComponent
        WallpaperPane { }
    }

    AppWindow {
        id: appWindow
        anchors.fill: parent
        title: qsTr("Settings")
        active: true

        titleBarData: TitleBar {
            id: titleBar
            title: qsTr("Settings")
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

            // Sidebar: a muted source list with local pane search on top.
            Rectangle {
                id: sidebarPane
                width: 240
                height: parent.height
                color: Theme.color.surfaceMuted

                Column {
                    anchors.fill: parent
                    anchors.margins: Theme.controls.sidebar.padding
                    spacing: Theme.primitive.spacing.md

                    SearchField {
                        id: searchField
                        width: parent.width
                        placeholderText: qsTr("Search")
                        onTextChanged: root.searchText = searchField.text
                        onCleared: root.searchText = ""
                    }

                    Sidebar {
                        id: sidebar
                        width: parent.width
                        height: Math.max(0, parent.height - searchField.height - parent.spacing)
                        sections: [{
                            items: root.visiblePanes.map(function(pane) {
                                return { label: pane.title, icon: pane.icon };
                            })
                        }]

                        onActivated: (index, item) => {
                            if (index >= 0 && index < root.visiblePanes.length)
                                root.selectPane(root.visiblePanes[index].id);
                        }
                        onCurrentIndexChanged: {
                            if (!root.syncing && sidebar.currentIndex >= 0
                                    && sidebar.currentIndex < root.visiblePanes.length)
                                root.selectPane(root.visiblePanes[sidebar.currentIndex].id);
                        }
                    }
                }
            }

            // Detail pane: history chrome, the header card, and the pane body.
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
                        title: root.currentPane ? root.currentPane.title : ""

                        Button {
                            id: backButton
                            icon: "chevron-left"
                            variant: "ghost"
                            accessibleName: qsTr("Back")
                            enabled: root.canGoBack
                            onClicked: root.back()
                        }
                        Button {
                            id: forwardButton
                            icon: "chevron-right"
                            variant: "ghost"
                            accessibleName: qsTr("Forward")
                            enabled: root.canGoForward
                            onClicked: root.forward()
                        }
                    }

                    ScrollView {
                        id: detailScroll
                        width: parent.width
                        height: Math.max(0, parent.height - toolbar.height)
                        scrollbarVisible: true

                        Column {
                            x: Theme.primitive.spacing.xl
                            width: Math.max(0, detailScroll.width - 2 * Theme.primitive.spacing.xl)
                            spacing: Theme.primitive.spacing.lg

                            Item { width: 1; height: Theme.primitive.spacing.sm }

                            PaneHeader {
                                id: header
                                width: parent.width
                                pane: root.currentPane
                            }

                            Loader {
                                id: paneBody
                                width: parent.width
                                sourceComponent: root.paneComponent(root.currentPaneId)
                            }

                            SettingsGroup {
                                width: parent.width
                                reserveBottomMargin: false
                                visible: paneBody.status !== Loader.Ready

                                Text {
                                    width: parent.width
                                    text: root.currentPane
                                          ? qsTr("The controls for this pane arrive with its own step of the Settings wave.")
                                          : qsTr("Choose a pane from the sidebar.")
                                    color: Theme.color.textSecondary
                                    font.pixelSize: Theme.controls.button.fontSize
                                    wrapMode: Text.WordWrap
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Settings")
}