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
    // Bound to the host Window's focus so the titlebar/controls show the
    // focused state; the app-local chrome material itself is focus-invariant.
    property bool windowActive: true
    // Guards the sidebar selection sync so setting `currentIndex`
    // programmatically is not mistaken for a user navigation.
    property bool syncing: false
    // T223: the deep-link landing. `pendingPaneId`/`pendingRowId` survive the
    // pane Loader swap and are consumed once the body exists.
    property string pendingPaneId: ""
    property string pendingRowId: ""
    property var highlightTarget: null
    // How long the deep-link landing wash stays visible. Tests/captures raise
    // it so a slow software render cannot outrun the evidence.
    property int highlightHoldMs: 1800
    // The pane id whose body is currently mounted. `onCurrentPaneIdChanged`
    // can fire before the Loader swaps, so a pending deep link waits for this
    // to match instead of searching the outgoing pane's rows.
    property string loadedPaneId: ""

    readonly property bool searchActive: root.searchText.trim().length > 0
    readonly property var searchResults: SettingsSearch.search(root.searchText)

    property alias searchField: searchField
    property alias searchResultsList: searchResultsList
    property alias sidebar: sidebar
    property alias sidebarPane: sidebarPane
    property alias toolbar: toolbar
    property alias titleBar: titleBar
    property alias appWindow: appWindow
    property alias header: header
    property alias backButton: backButton
    property alias forwardButton: forwardButton
    property alias paneBody: paneBody
    property alias detailSheet: detailSheet
    property alias detailScroll: detailScroll
    property alias detailContent: detailContent
    property alias helpButton: helpButton
    property alias resizeEdges: resizeEdges
    property alias debugView: debugView

    // The app-local theme owner (T-09.2). The shell's `ThemeBinding` lives in
    // the shell process; the Settings app is a separate process, so these
    // bindings mirror the same settings keys onto this engine's `Theme`
    // singleton. A change in the Appearance pane restyles the app immediately,
    // and an external settingsd change reaches it through `Settings.values`.
    // `auto` tracks the host style hint live.
    readonly property string colorScheme: Settings.values["appearance.colorScheme"] || "auto"
    readonly property bool hostDark: Application.styleHints.colorScheme === Qt.Dark
    // T196: the app-local Glass resolver mirrors the same durable preferences
    // the shell's ThemeBinding writes, so the Appearance/Accessibility
    // previews are the rendered resolver, not a second formula.
    readonly property real transparency: {
        var value = Number(Settings.values["appearance.transparency"]);
        return isFinite(value) ? Math.max(0, Math.min(1, value))
                               : Theme.glass.slider.defaultPosition;
    }
    readonly property bool reduceTransparency:
        Settings.values["accessibility.reduceTransparency"] === true
    readonly property bool increaseContrast:
        Settings.values["accessibility.increaseContrast"] === true
    // The app-local rollout/capability view. Production follows the process
    // environment and platform bridge; tests and captures may override them
    // without changing the shared singleton's binding.
    property string glassRollout: Settings.glassRollout
    property bool appChromeSupported: Settings.appChromeSupported

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal moveRequested(real x, real y)
    signal menuRequested(real x, real y)
    // A window-edge press. `edges` is a `Qt.Edges` flag; the host window
    // forwards it to `startSystemResize`.
    signal resizeRequested(int edges)

    readonly property var visiblePanes: SettingsPanes.filter(root.searchText)
    readonly property var currentPane: SettingsPanes.paneById(root.currentPaneId)
    // The app's published native menu model (T-09.6a). The global menu bar
    // consumes this; it is exposed here so the app can present the same model
    // in-window and so tests can assert the publication.
    readonly property var menuModel: SettingsMenu.publishedModel
    readonly property bool canGoBack: root.historyIndex > 0
    readonly property bool canGoForward: root.historyIndex >= 0
                                       && root.historyIndex < root.history.length - 1

    implicitWidth: 900
    implicitHeight: 600

    // Temporary pane-switch timing trace: run the app with
    // `DF_SETTINGS_TRACE=1` and correlate these `DFTRACE` lines with the
    // compositor's (`DRAGONFRUIT_FRAME_TRACE=1`).
    function trace(message) {
        if (Settings.trace)
            Settings.traceLog(message);
    }

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
        root.trace("selectPane " + id);
        root.clearPendingHighlight();
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
        root.clearPendingHighlight();
        root.historyIndex -= 1;
        root.currentPaneId = root.history[root.historyIndex];
        root.syncSelection();
    }

    function forward() {
        if (!root.canGoForward)
            return;
        root.clearPendingHighlight();
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

    // T223: parse `settings://pane/<id>` and
    // `settings://pane/<id>/row/<rowId>`. The row segment is optional; a row
    // link is only accepted when the map knows the row and marks it shipped.
    function parseDeepLink(url) {
        var text = String(url || "").trim();
        var prefix = "settings://pane/";
        if (text.indexOf(prefix) !== 0)
            return null;
        var rest = text.substring(prefix.length);
        var hash = rest.indexOf("#");
        if (hash >= 0)
            rest = rest.substring(0, hash);
        var parts = rest.split("/");
        var paneId = decodeURIComponent(parts[0] || "");
        var rowId = "";
        if (parts.length >= 3 && parts[1] === "row")
            rowId = decodeURIComponent(parts[2] || "");
        return { paneId: paneId, rowId: rowId };
    }

    function deepLink(url) {
        var link = root.parseDeepLink(url);
        if (link === null)
            return false;
        var pane = SettingsPanes.paneById(link.paneId);
        if (pane === null || !pane.shipped)
            return false;
        if (link.rowId.length > 0) {
            var row = SettingsMap.rowFor(link.paneId, link.rowId);
            if (row === null || row.state !== "shipped")
                return false;
        }
        root.searchText = "";
        if (root.currentPaneId !== link.paneId)
            root.selectPane(link.paneId);
        root.pendingPaneId = link.paneId;
        root.pendingRowId = link.rowId;
        if (root.currentPaneId === link.paneId)
            Qt.callLater(root.revealPending);
        return true;
    }

    function activateSearchResult(result) {
        if (result === null || result === undefined)
            return;
        root.searchText = "";
        root.deepLink(result.deepLink);
    }

    function clearPendingHighlight() {
        root.pendingPaneId = "";
        root.pendingRowId = "";
        highlightTimer.stop();
        if (root.highlightTarget !== null) {
            root.highlightTarget.highlighted = false;
            root.highlightTarget = null;
        }
    }

    // Walk the loaded pane body for the map row's rendered SettingsRow. The
    // `settingsId` is the stable match; the label/synonym match lets rows that
    // predate the T223 wiring (or a reference label with different wording)
    // still land on the right control.
    function findRowTarget(item, row) {
        if (item === null || item === undefined || item.visible === false)
            return null;
        if (row !== null && item.settingsId !== undefined
                && item.settingsId === row.id)
            return item;
        if (row !== null && item.label !== undefined
                && item.showSeparator !== undefined
                && root.rowLabelMatches(item.label, row))
            return item;
        var children = item.children;
        for (var i = 0; i < children.length; ++i) {
            var found = root.findRowTarget(children[i], row);
            if (found !== null)
                return found;
        }
        return null;
    }

    function rowLabelMatches(bodyLabel, row) {
        var body = SettingsSearch.normalize(bodyLabel);
        if (body.length === 0)
            return false;
        var candidates = [row.label];
        var synonyms = SettingsSearch.rowSynonymsFor(row.paneId, row.id);
        for (var i = 0; i < synonyms.length; ++i)
            candidates.push(synonyms[i]);
        for (var c = 0; c < candidates.length; ++c) {
            var target = SettingsSearch.normalize(candidates[c]);
            if (target.length === 0)
                continue;
            if (body === target || body.indexOf(target) >= 0
                    || target.indexOf(body) >= 0)
                return true;
        }
        return false;
    }

    function findSectionTarget(item, section) {
        if (item === null || item === undefined || item.visible === false)
            return null;
        var wanted = SettingsSearch.normalize(section);
        if (wanted.length > 0 && item.sectionId !== undefined
                && item.sectionId === section)
            return item;
        if (wanted.length > 0 && item.title !== undefined
                && item.rows !== undefined) {
            var title = SettingsSearch.normalize(item.title);
            if (title === wanted || title.indexOf(wanted) >= 0
                    || wanted.indexOf(title) >= 0)
                return item;
        }
        var children = item.children;
        for (var i = 0; i < children.length; ++i) {
            var found = root.findSectionTarget(children[i], section);
            if (found !== null)
                return found;
        }
        return null;
    }

    // Scroll the detail view to the pending row/section and hold the accent
    // wash long enough to be visible. Called from Loader.onLoaded and from the
    // same-pane deep-link path.
    function revealPending() {
        if (root.pendingPaneId.length === 0
                || root.pendingPaneId !== root.currentPaneId
                || root.pendingPaneId !== root.loadedPaneId)
            return;
        var body = paneBody.item;
        if (body === null)
            return;
        var row = root.pendingRowId.length > 0
                ? SettingsMap.rowFor(root.pendingPaneId, root.pendingRowId) : null;
        var target = row !== null ? root.findRowTarget(body, row) : null;
        if (target === null && row !== null)
            target = root.findSectionTarget(body, row.section);
        if (target === null)
            target = body;
        var point = target.mapToItem(detailContent, 0, 0);
        detailScroll.flickable.contentY = Math.max(
            0, Math.min(detailScroll.maxY,
                        point.y - Theme.primitive.spacing.lg));
        if (target.highlighted !== undefined) {
            if (root.highlightTarget !== null && root.highlightTarget !== target)
                root.highlightTarget.highlighted = false;
            root.highlightTarget = target;
            target.highlighted = true;
            highlightTimer.restart();
        }
        if (target.focusFirstControl !== undefined)
            target.focusFirstControl();
        else if (target.forceActiveFocus !== undefined)
            target.forceActiveFocus();
        root.pendingPaneId = "";
        root.pendingRowId = "";
    }

    Timer {
        id: highlightTimer
        interval: root.highlightHoldMs
        repeat: false
        onTriggered: {
            if (root.highlightTarget !== null) {
                root.highlightTarget.highlighted = false;
                root.highlightTarget = null;
            }
        }
    }

    // T222: open the pane/row owner debug view for the current pane (or an
    // explicit pane id). The Help menu and `DF_SETTINGS_DEBUG=1` call this.
    function showDebugView(paneId) {
        debugView.show(paneId || root.currentPaneId);
    }

    // The pane-body registry: a pane appears here once its content has landed.
    // T-09.3…T-09.5 add their components alongside "appearance".
    function paneComponent(id) {
        switch (id) {
        case "appearance":
            return appearancePaneComponent;
        case "wallpaper":
            return wallpaperPaneComponent;
        case "battery":
            return batteryPaneComponent;
        case "desktop-dock":
            return desktopDockPaneComponent;
        case "mission-control":
            return missionControlPaneComponent;
        case "displays":
            return displaysPaneComponent;
        case "bluetooth":
            return bluetoothPaneComponent;
        case "storage":
            return storagePaneComponent;
        case "sound":
            return soundPaneComponent;
        case "keyboard":
            return keyboardPaneComponent;
        case "mouse":
            return mousePaneComponent;
        case "trackpad":
            return trackpadPaneComponent;
        case "notifications":
            return notificationsPaneComponent;
        case "focus":
            return focusPaneComponent;
        case "lock-screen":
            return lockScreenPaneComponent;
        case "menu-bar":
            return menuBarPaneComponent;
        case "general":
            return generalPaneComponent;
        case "users-groups":
            return usersGroupsPaneComponent;
        case "printers":
            return printersPaneComponent;
        case "privacy":
            return privacyPaneComponent;
        case "accessibility":
            return accessibilityPaneComponent;
        case "wifi":
            return wifiPaneComponent;
        case "network":
            return networkPaneComponent;
        default:
            return null;
        }
    }

    Component.onCompleted: {
        var first = root.visiblePanes.length > 0 ? root.visiblePanes[0].id : "";
        // The out-of-box pane stays `appearance` while it ships, so opening
        // Settings never auto-starts a subsystem side effect (the Bluetooth
        // pane begins an inquiry while it is open, T-15.1b). Otherwise fall
        // back to the first shipped pane.
        if (SettingsPanes.indexOf("appearance") >= 0)
            first = "appearance";
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
        // `DF_SETTINGS_DEBUG=1` opens the owner debug view on launch so a
        // scripted check can inspect the current pane's row metadata.
        if (Settings.debugView && first.length > 0)
            root.showDebugView(first);
    }

    onCurrentPaneIdChanged: {
        if (debugView.open && root.currentPaneId.length > 0)
            debugView.paneId = root.currentPaneId;
        if (root.pendingPaneId.length > 0
                && root.pendingPaneId === root.currentPaneId
                && root.pendingPaneId === root.loadedPaneId)
            Qt.callLater(root.revealPending);
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
    Binding {
        target: Glass
        property: "rollout"
        value: root.glassRollout
    }
    Binding {
        target: Glass
        property: "slider"
        value: root.transparency
    }
    Binding {
        target: Glass
        property: "reduceTransparency"
        value: root.reduceTransparency
    }
    Binding {
        target: Glass
        property: "increaseContrast"
        value: root.increaseContrast
    }
    Binding {
        target: Glass
        property: "appChromeSupported"
        value: root.appChromeSupported
    }

    Component {
        id: appearancePaneComponent
        AppearancePane { }
    }

    Component {
        id: wallpaperPaneComponent
        WallpaperPane { }
    }

    Component {
        id: desktopDockPaneComponent
        DesktopDockPane { }
    }

    Component {
        id: missionControlPaneComponent
        MissionControlPane { }
    }

    Component {
        id: batteryPaneComponent
        BatteryPane { }
    }

    Component {
        id: displaysPaneComponent
        DisplaysPane { }
    }

    Component {
        id: bluetoothPaneComponent
        BluetoothPane { }
    }

    Component {
        id: storagePaneComponent
        StoragePane { }
    }

    Component {
        id: soundPaneComponent
        SoundPane { }
    }

    Component {
        id: keyboardPaneComponent
        KeyboardPane { }
    }

    Component {
        id: mousePaneComponent
        MousePane { }
    }

    Component {
        id: trackpadPaneComponent
        TrackpadPane { }
    }

    Component {
        id: notificationsPaneComponent
        NotificationsPane { }
    }

    Component {
        id: focusPaneComponent
        FocusPane { }
    }

    Component {
        id: lockScreenPaneComponent
        LockScreenPane { }
    }

    Component {
        id: menuBarPaneComponent
        MenuBarPane { }
    }

    Component {
        id: generalPaneComponent
        GeneralPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
    }

    Component {
        id: usersGroupsPaneComponent
        UsersGroupsPane { }
    }

    Component {
        id: printersPaneComponent
        PrintersScannersPane { }
    }

    Component {
        id: privacyPaneComponent
        PrivacyPane { }
    }

    Component {
        id: accessibilityPaneComponent
        AccessibilityPane { }
    }

    Component {
        id: wifiPaneComponent
        WifiPane { }
    }

    Component {
        id: networkPaneComponent
        NetworkPane {
            onPaneRequested: (paneId) => root.selectPane(paneId)
        }
    }

    AppWindow {
        id: appWindow
        anchors.fill: parent
        title: qsTr("Settings")
        active: root.windowActive
        // T206: the sidebar/toolbar carry the measured app-local alpha; the
        // detail sheet below stays opaque.
        chromeTransparent: true

        titleBarData: TitleBar {
            id: titleBar
            title: qsTr("Settings")
            active: root.windowActive
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
            ChromeSheet {
                id: sidebarPane
                width: Theme.controls.sidebar.width
                height: parent.height
                color: Material.appChromeFill("settingsSidebar",
                                             Theme.color.surfaceMuted)
                bottomLeftRadius: Theme.controls.window.radius

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
                        // T223: Down/Up move into the ranked result list; Return
                        // opens the highlighted result; Escape clears and then
                        // returns focus to the sidebar.
                        onAccepted: searchResultsList.activateCurrent()
                        onEscapePressed: {
                            if (root.searchText.length > 0)
                                root.searchText = "";
                            else
                                sidebar.forceActiveFocus();
                        }
                        Keys.onDownPressed: (event) => {
                            searchResultsList.focusList();
                            event.accepted = true;
                        }
                        Keys.onUpPressed: (event) => {
                            searchResultsList.focusList();
                            event.accepted = true;
                        }
                    }

                    Item {
                        width: parent.width
                        height: Math.max(0, parent.height - searchField.height - parent.spacing)

                        Sidebar {
                            id: sidebar
                            anchors.fill: parent
                            visible: !root.searchActive
                            // The outer ChromeSheet owns the app-local material.
                            fillColor: "transparent"
                            sections: [{
                                items: root.visiblePanes.map(function(pane) {
                                    return {
                                        label: pane.title,
                                        icon: pane.icon,
                                        category: SettingsPanes.categoryStyle(pane)
                                    };
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

                        SettingsSearchResults {
                            id: searchResultsList
                            anchors.fill: parent
                            visible: root.searchActive
                            results: root.searchResults
                            query: root.searchText
                            onActivated: (result) => root.activateSearchResult(result)
                            onDismissed: {
                                root.searchText = "";
                                searchField.forceActiveFocus();
                            }
                        }
                    }
                }
            }

            // Detail pane: history chrome, the header card, and the pane body.
            Rectangle {
                width: parent.width - sidebarPane.width
                height: parent.height
                color: "transparent"

                // The toolbar keeps the app-local material; the scrolling
                // detail sheet below it is the opaque content surface and owns
                // the bottom-right window corner.
                ChromeSheet {
                    id: detailSheet
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.topMargin: Theme.controls.toolbar.height
                    anchors.bottom: parent.bottom
                    color: Theme.color.surface
                    bottomRightRadius: Theme.controls.window.radius
                }

                Column {
                    anchors.fill: parent
                    spacing: 0

                    Toolbar {
                        id: toolbar
                        width: parent.width
                        title: root.currentPane ? root.currentPane.title : ""
                        fillColor: Material.appChromeFill("settingsToolbar",
                                                          Theme.color.chrome)
                        borderColor: Material.appChromeBorder("settingsToolbar",
                                                              Theme.color.border)

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
                            id: detailContent
                            x: Theme.primitive.spacing.xl
                            width: Math.max(0, detailScroll.width - 2 * Theme.primitive.spacing.xl)
                            spacing: Theme.primitive.spacing.lg

                            Item { width: 1; height: Theme.primitive.spacing.sm }

                            PaneHeader {
                                id: header
                                width: parent.width
                                // Only panes whose macOS capture has a header
                                // card show one; the settings-list panes open
                                // at their first group.
                                visible: SettingsPanes.hasHeader(root.currentPane)
                                pane: root.currentPane
                            }

                            Loader {
                                id: paneBody
                                width: parent.width
                                sourceComponent: root.paneComponent(root.currentPaneId)
                                onLoaded: {
                                    root.loadedPaneId = root.currentPaneId;
                                    root.trace("paneLoaded " + root.currentPaneId);
                                    if (root.pendingPaneId === root.currentPaneId)
                                        Qt.callLater(root.revealPending);
                                }
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

                // T223: the reference's bottom-right `?` help control. It
                // opens this pane's anchor in the business-logic map, so it
                // is a live document link, never a dead button. With the
                // document absent (a minimal install) the control disables
                // instead of pretending to open something.
                Button {
                    id: helpButton
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.rightMargin: Theme.primitive.spacing.lg
                    anchors.bottomMargin: Theme.primitive.spacing.lg
                    z: 10
                    text: "?" // df-allow-untranslated: help glyph
                    variant: "ghost"
                    accessibleName: root.currentPane
                                    ? qsTr("Help for %1").arg(root.currentPane.title)
                                    : qsTr("Help")
                    enabled: Settings.mapDocUrl.toString().length > 0
                    onClicked: Qt.openUrlExternally(
                                   Settings.helpUrl(root.currentPaneId))
                }
            }
        }
    }

    // T222: the executable map's debug view. It sits above the window content
    // and is opened from Help → Pane & Row Owners… or `DF_SETTINGS_DEBUG=1`.
    SettingsDebugView {
        id: debugView
        anchors.fill: parent
    }

    // The frameless window owns its edge input; the shared grips report the
    // matching `Qt.Edges` and the host forwards them to `startSystemResize`.
    WindowResizeGrips {
        id: resizeEdges
        anchors.fill: parent
        onResizeRequested: (edges) => root.resizeRequested(edges)
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Settings")
}