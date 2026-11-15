// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings app shell (T-09.1a): the 52 pt unified window chrome
// (titlebar row, traffic lights, and back/forward), the sidebar with local
// pane search, and the detail-pane header card.
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
    // The pane whose map owns `pendingRowId`. Storage rows live under the
    // `storage` map id even though the body is General -> Storage.
    property string pendingRowPaneId: ""
    property var highlightTarget: null
    // How long the deep-link landing wash stays visible. Tests/captures raise
    // it so a slow software render cannot outrun the evidence.
    property int highlightHoldMs: 1800
    // The pane id whose body is currently mounted. `onCurrentPaneIdChanged`
    // can fire before the Loader swaps, so a pending deep link waits for this
    // to match instead of searching the outgoing pane's rows.
    property string loadedPaneId: ""
    property bool pendingAboutPage: false
    // T263: a deep link or General sub-pane request that must open after the
    // pane body exists. `pendingSubPaneId` survives the Loader swap; the
    // `pendingSection` is a section title to scroll to after the swap (the VPN
    // link lands on Network -> VPN).
    property string pendingSubPaneId: ""
    property string pendingSection: ""
    readonly property bool aboutOpen: root.currentPaneId === "general"
                                      && paneBody.item !== null
                                      && paneBody.item.subPaneId === "about"
    readonly property string currentSubPaneId:
        paneBody.item !== null && paneBody.item.subPaneId !== undefined
            ? String(paneBody.item.subPaneId) : ""
    readonly property bool subPaneOpen: root.currentSubPaneId.length > 0
    // The toolbar title and the compositor/taskbar title both follow the
    // current pane (or its open sub-pane), never a constant.
    readonly property string paneTitle: {
        if (root.subPaneOpen && paneBody.item !== null
                && paneBody.item.subPaneTitle !== undefined
                && String(paneBody.item.subPaneTitle).length > 0)
            return String(paneBody.item.subPaneTitle);
        return root.currentPane ? root.currentPane.title : qsTr("System Settings");
    }
    readonly property string windowTitle: root.currentPane
        ? root.paneTitle : qsTr("System Settings")

    readonly property bool searchActive: root.searchText.trim().length > 0
    readonly property var searchResults: SettingsSearch.search(root.searchText)

    property alias searchField: searchField
    property alias searchResultsList: searchResultsList
    property alias sidebar: sidebar
    property alias sidebarPane: sidebarPane
    property alias sidebarToolbar: sidebarToolbar
    property alias sidebarTrafficLights: sidebarTrafficLights
    // T230b: the unified toolbar is the titlebar row; keep the old alias so
    // existing chrome-material consumers read the same surface.
    property alias toolbar: titleBar
    property alias titleBar: titleBar
    property alias detailToolbarSheet: detailToolbarSheet
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
    // T227: the Appearance completion keys. Highlight is the selection wash,
    // sidebar size is mapped onto the design-system Sidebar slots, scroll bars
    // onto the ScrollView policy, and wallpaper tint onto the Glass resolver.
    readonly property string highlight:
        Settings.values["appearance.highlight"] || ""
    readonly property string scrollBars:
        Settings.values["appearance.scrollBars"] || "automatic"
    readonly property bool wallpaperTint:
        Settings.values["appearance.wallpaperTint"] !== false
    readonly property string sidebarIconSize:
        Settings.values["appearance.sidebarIconSize"] || "medium"
    readonly property real sidebarIconPixels:
        root.sidebarIconSize === "small" ? 13
      : root.sidebarIconSize === "large" ? 19 : 16
    readonly property real sidebarCategoryIconPixels:
        root.sidebarIconSize === "small" ? 16
      : root.sidebarIconSize === "large" ? 24 : 20
    // The app-local rollout/capability view. Production follows the process
    // environment and platform bridge; tests and captures may override them
    // without changing the shared singleton's binding.
    property string glassRollout: Settings.glassRollout !== ""
        ? Settings.glassRollout : Theme.glass.rollout.defaultMode
    property bool appChromeSupported: Settings.appChromeSupported

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal moveRequested(real x, real y)
    signal menuRequested(real x, real y)
    // A window-edge press. `edges` is a `Qt.Edges` flag; the host window
    // forwards it to `startSystemResize`.
    signal resizeRequested(int edges)
    signal aboutPageOpened()

    readonly property var visiblePanes: SettingsPanes.filter(root.searchText)
    // T263: the same visible list, grouped for the source list. One catalog,
    // one filter; the sidebar never gets a second pane list. Each item keeps
    // the pane id so selection and activation do not depend on a flat index.
    readonly property var sidebarSections: SettingsPanes.grouped(root.visiblePanes).map(function(section) {
        return {
            title: section.title,
            items: section.items.map(function(pane) {
                return {
                    id: pane.id,
                    label: pane.title,
                    icon: pane.icon,
                    category: SettingsPanes.categoryStyle(pane)
                };
            })
        };
    })
    readonly property var currentPane: SettingsPanes.paneById(root.currentPaneId)
    // The app's published native menu model (T-09.6a). The global menu bar
    // consumes this; it is exposed here so the app can present the same model
    // in-window and so tests can assert the publication.
    readonly property var menuModel: SettingsMenu.publishedModel
    readonly property bool canGoBack: root.historyIndex > 0
    readonly property bool canGoForward: root.historyIndex >= 0
                                       && root.historyIndex < root.history.length - 1

    // T263: the measured System Settings window. The host Window sets the
    // real size; these values keep the shell itself at the reference geometry
    // when a test or capture instantiates it directly.
    implicitWidth: Theme.controls.settingsWindow.width
    implicitHeight: Theme.controls.settingsWindow.height

    // Temporary pane-switch timing trace: run the app with
    // `DF_SETTINGS_TRACE=1` and correlate these `DFTRACE` lines with the
    // compositor's (`DRAGONFRUIT_FRAME_TRACE=1`).
    function trace(message) {
        if (Settings.trace)
            Settings.traceLog(message);
    }

    // The sidebar's current index counts section headers as well as rows.
    // Computing it from the same grouped sections the Sidebar renders keeps
    // selection in sync without a second id->index map.
    function sidebarIndexOf(id) {
        var index = 0;
        for (var s = 0; s < root.sidebarSections.length; ++s) {
            var section = root.sidebarSections[s];
            if (String(section.title || "").length > 0)
                index += 1;
            var items = section.items || [];
            for (var i = 0; i < items.length; ++i) {
                if (items[i].id === id)
                    return index;
                index += 1;
            }
        }
        return -1;
    }

    function sidebarIdAtIndex(index) {
        if (index < 0)
            return "";
        var current = 0;
        for (var s = 0; s < root.sidebarSections.length; ++s) {
            var section = root.sidebarSections[s];
            if (String(section.title || "").length > 0) {
                if (current === index)
                    return "";
                current += 1;
            }
            var items = section.items || [];
            for (var i = 0; i < items.length; ++i) {
                if (current === index)
                    return items[i].id;
                current += 1;
            }
        }
        return "";
    }

    // Open a resolved catalog target: a pane plus an optional sub-pane or
    // section anchor. Used by the sidebar links and the deep-link resolver.
    function openTarget(target) {
        if (target === null)
            return;
        root.searchText = "";
        if (root.currentPaneId !== target.paneId)
            root.selectPane(target.paneId);
        if (target.subPaneId !== undefined && target.subPaneId.length > 0)
            root.openSubPane(target.subPaneId);
        if (target.anchor !== undefined && target.anchor.length > 0) {
            root.pendingPaneId = target.paneId;
            root.pendingSection = target.anchor;
        }
        if (root.currentPaneId === target.paneId)
            Qt.callLater(root.revealPending);
    }

    function activateSidebarItem(id) {
        root.openTarget(SettingsPanes.resolve(id));
    }

    // Open a General in-place sub-pane now when the body is mounted, or after
    // the Loader swaps.
    function openSubPane(id) {
        if (!id || id.length === 0)
            return;
        if (root.currentPaneId === "general" && paneBody.item !== null
                && paneBody.item.openSubPane !== undefined) {
            paneBody.item.openSubPane(id);
            root.pendingSubPaneId = "";
            return;
        }
        root.pendingSubPaneId = id;
    }

    // Show a pane, recording the move in history. Selecting the current pane
    // is a no-op so arrow-key echo does not grow the stack.
    function selectPane(id) {
        if (!id || id === root.currentPaneId)
            return;
        root.trace("selectPane " + id);
        root.clearPendingHighlight();
        root.pendingSubPaneId = "";
        var next = root.history.slice(0, root.historyIndex + 1);
        next.push(id);
        root.history = next;
        root.historyIndex = next.length - 1;
        root.currentPaneId = id;
        root.syncSelection();
    }

    function openAboutPage() {
        root.pendingAboutPage = true;
        root.selectPane("general");
        if (root.loadedPaneId === "general" && root.paneBody.item) {
            root.paneBody.item.openSubPane("about");
            root.pendingAboutPage = false;
        }
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
        sidebar.currentIndex = root.sidebarIndexOf(root.currentPaneId);
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
        // Storage and VPN are aliases: resolve them to the pane that owns the
        // body (General -> Storage, Network -> VPN). A hidden/optional alias
        // is accepted only when its target is visible.
        var target = SettingsPanes.resolve(link.paneId);
        if (target === null)
            return false;
        var pane = SettingsPanes.paneById(target.paneId);
        if (pane === null || !pane.shipped)
            return false;
        if (!SettingsPanes.isVisible(link.paneId))
            return false;
        if (link.rowId.length > 0) {
            var row = SettingsMap.rowFor(link.paneId, link.rowId);
            if (row === null || row.state !== "shipped")
                return false;
        }
        root.searchText = "";
        if (root.currentPaneId !== target.paneId)
            root.selectPane(target.paneId);
        root.pendingPaneId = target.paneId;
        root.pendingRowId = link.rowId;
        root.pendingRowPaneId = link.paneId;
        root.pendingSection = target.anchor;
        if (target.subPaneId.length > 0)
            root.openSubPane(target.subPaneId);
        if (root.currentPaneId === target.paneId)
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
        root.pendingRowPaneId = "";
        root.pendingSection = "";
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
                ? SettingsMap.rowFor(root.pendingRowPaneId.length > 0
                                     ? root.pendingRowPaneId : root.pendingPaneId,
                                     root.pendingRowId)
                : null;
        var target = row !== null ? root.findRowTarget(body, row) : null;
        if (target === null && row !== null)
            target = root.findSectionTarget(body, row.section);
        if (target === null && root.pendingSection.length > 0)
            target = root.findSectionTarget(body, root.pendingSection);
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
        root.pendingRowPaneId = "";
        root.pendingSection = "";
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
        case "game-controllers":
            return gameControllersPaneComponent;
        case "privacy":
            return privacyPaneComponent;
        case "biometrics":
            return biometricsPaneComponent;
        case "online-accounts":
            return onlineAccountsPaneComponent;
        case "accessibility":
            return accessibilityPaneComponent;
        case "wifi":
            return wifiPaneComponent;
        case "network":
            return networkPaneComponent;
        case "control-center":
            return quickSettingsPaneComponent;
        case "search":
            return searchPaneComponent;
        case "screen-saver":
            return screenSaverPaneComponent;
        default:
            return null;
        }
    }

    Component.onCompleted: {
        var first = "";
        // T263: restore the last visited pane on a normal launch. First launch
        // (no stored preference) opens General. The persistence seam is off
        // under the headless fixture unless a test opts in, so the existing
        // pane tests keep a clean process.
        if (Settings.windowStatePersistence) {
            var restored = String(Settings.values["settings.lastPane"] || "");
            if (restored.length > 0 && SettingsPanes.isVisible(restored))
                first = SettingsPanes.resolve(restored).paneId;
        }
        if (first.length === 0 && SettingsPanes.isVisible("general"))
            first = "general";
        if (first.length === 0 && root.visiblePanes.length > 0)
            first = root.visiblePanes[0].id;
        // `DF_SETTINGS_START_PANE` opens a specific shipped pane for captures
        // and scripted checks; an unknown or unshipped id is ignored.
        var requested = Settings.startPane;
        if (requested.length > 0 && SettingsPanes.isVisible(requested))
            first = SettingsPanes.resolve(requested).paneId;
        if (first.length > 0) {
            root.history = [first];
            root.historyIndex = 0;
            root.currentPaneId = first;
            root.syncSelection();
        }
        // T232: a cold-launch `--deep-link` lands on the requested pane/row.
        // A deep link wins over the restored pane and the default.
        var deepLink = Settings.startDeepLink;
        if (deepLink.length > 0)
            root.deepLink(deepLink);
        // `DF_SETTINGS_DEBUG=1` opens the owner debug view on launch so a
        // scripted check can inspect the current pane's row metadata.
        if (Settings.debugView && first.length > 0)
            root.showDebugView(first);
    }

    onCurrentPaneIdChanged: {
        if (debugView.open && root.currentPaneId.length > 0)
            debugView.paneId = root.currentPaneId;
        // T263: remember the last visited pane. The compositor/taskbar title
        // follows the same value through `windowTitle`.
        if (Settings.windowStatePersistence && root.currentPaneId.length > 0)
            Settings.set("settings.lastPane", root.currentPaneId);
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

    onSidebarSectionsChanged: root.syncSelection()

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
        property: "highlightOverride"
        value: root.highlight
    }
    Binding {
        target: Theme
        property: "scrollBars"
        value: root.scrollBars
    }
    Binding {
        target: Theme
        property: "reducedMotion"
        value: Settings.values["accessibility.reduceMotion"] === true
    }
    Binding {
        target: Glass
        property: "wallpaperTint"
        value: root.wallpaperTint
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
        id: quickSettingsPaneComponent
        QuickSettingsPane { }
    }

    Component {
        id: searchPaneComponent
        SearchPane { }
    }

    Component {
        id: screenSaverPaneComponent
        ScreenSaverPane { }
    }

    Component {
        id: wallpaperPaneComponent
        WallpaperPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
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
        BatteryPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
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
        KeyboardPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
    }

    Component {
        id: mousePaneComponent
        MousePane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
    }

    Component {
        id: trackpadPaneComponent
        TrackpadPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
    }

    Component {
        id: notificationsPaneComponent
        NotificationsPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
    }

    Component {
        id: focusPaneComponent
        FocusPane { }
    }

    Component {
        id: lockScreenPaneComponent
        LockScreenPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
        }
    }

    Component {
        id: menuBarPaneComponent
        MenuBarPane { }
    }

    Component {
        id: generalPaneComponent
        GeneralPane {
            onNavigateRequested: (paneId) => root.selectPane(paneId)
            onAboutPageOpened: {
                root.detailScroll.flickable.contentY = 0;
                root.aboutPageOpened();
            }
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
        id: gameControllersPaneComponent
        GameControllersPane { }
    }

    Component {
        id: privacyPaneComponent
        PrivacyPane { }
    }

    Component {
        id: biometricsPaneComponent
        BiometricsPane { }
    }

    Component {
        id: onlineAccountsPaneComponent
        OnlineAccountsPane { }
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
        title: root.windowTitle
        active: root.windowActive
        // T230b: Settings is a 52 pt unified-toolbar window. The style is
        // declared once here; the split toolbars inherit it.
        windowStyle: "unified"
        // T206/T263: the sidebar material runs under the titlebar and the
        // detail sheet below the toolbar stays opaque.
        chromeTransparent: true

        Row {
            anchors.fill: parent
            spacing: 0

            // Sidebar column: full window height, so its material runs under
            // the unified toolbar (T263). Traffic lights live in this half.
            Item {
                id: sidebarColumn
                width: Theme.controls.settingsWindow.sidebarWidth
                height: parent.height

                ChromeSheet {
                    id: sidebarPane
                    anchors.fill: parent
                    color: Material.appChromeFill("settingsSidebar",
                                                 Theme.color.surfaceMuted)
                    topLeftRadius: Theme.controls.window.radius
                    bottomLeftRadius: Theme.controls.window.radius
                }

                Item {
                    id: sidebarToolbar
                    width: parent.width
                    height: Theme.controls.settingsWindow.chromeHeight

                    // The whole strip drags the window, like the reference's
                    // unified toolbar. The titlebar's own drag area is on the
                    // detail side.
                    MouseArea {
                        id: sidebarDrag
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        hoverEnabled: true
                        onPressed: (mouse) => {
                            if (mouse.button === Qt.LeftButton)
                                root.moveRequested(mouse.x, mouse.y);
                        }
                        onDoubleClicked: root.zoomRequested()
                        onClicked: (mouse) => {
                            if (mouse.button === Qt.RightButton)
                                root.menuRequested(mouse.x, mouse.y);
                        }
                    }

                    TrafficLights {
                        id: sidebarTrafficLights
                        active: root.windowActive
                        anchors.left: parent.left
                        anchors.leftMargin: WindowStyle.trafficLightInset(parent.height)
                        anchors.verticalCenter: parent.verticalCenter

                        onCloseClicked: root.closeRequested()
                        onMinimizeClicked: root.minimizeRequested()
                        onZoomClicked: root.zoomRequested()
                    }

                    Rectangle {
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: Theme.controls.window.borderWidth
                        color: Theme.color.border
                    }
                }

                Column {
                    anchors.top: sidebarToolbar.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
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
                            // T227: the Appearance pane's sidebar icon size.
                            iconSize: root.sidebarIconPixels
                            categoryIconSize: root.sidebarCategoryIconPixels
                            // The outer ChromeSheet owns the app-local material.
                            fillColor: "transparent"
                            sections: root.sidebarSections

                            onActivated: (index, item) => {
                                if (item && item.id)
                                    root.activateSidebarItem(item.id);
                            }
                            onCurrentIndexChanged: {
                                if (root.syncing)
                                    return;
                                var id = root.sidebarIdAtIndex(sidebar.currentIndex);
                                if (id.length > 0)
                                    root.activateSidebarItem(id);
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

            // Detail column: the 52 pt split toolbar over the opaque sheet.
            Item {
                id: detailColumn
                width: parent.width - sidebarColumn.width
                height: parent.height

                ChromeSheet {
                    id: detailSheet
                    anchors.fill: parent
                    color: Theme.color.surface
                    topRightRadius: Theme.controls.window.radius
                    bottomRightRadius: Theme.controls.window.radius
                }

                // The detail half of the unified toolbar. The sidebar material
                // behind its left edge is a separate surface, so the split is
                // visible at the sidebar width.
                ChromeSheet {
                    id: detailToolbarSheet
                    anchors.top: parent.top
                    anchors.left: parent.left
                    anchors.right: parent.right
                    height: Theme.controls.settingsWindow.chromeHeight
                    color: Material.appChromeFill("settingsToolbar",
                                                 Theme.color.chrome)
                    topRightRadius: Theme.controls.window.radius
                }

                TitleBar {
                    id: titleBar
                    anchors.top: parent.top
                    anchors.left: parent.left
                    anchors.right: parent.right
                    height: Theme.controls.settingsWindow.chromeHeight
                    title: root.paneTitle
                    active: root.windowActive
                    // The traffic lights and sidebar live left of the split;
                    // Back/Forward and the title live here, centred over the
                    // detail pane rather than the whole window.
                    showTrafficLights: false
                    fillColor: "transparent"
                    borderColor: Material.appChromeBorder("settingsToolbar",
                                                         Theme.color.border)
                    leadingData: Row {
                        spacing: Theme.controls.toolbar.spacing
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
                    onCloseRequested: root.closeRequested()
                    onMinimizeRequested: root.minimizeRequested()
                    onZoomRequested: root.zoomRequested()
                    onMoveRequested: (x, y) => root.moveRequested(x, y)
                    onMenuRequested: (x, y) => root.menuRequested(x, y)
                }

                ScrollView {
                    id: detailScroll
                    anchors.top: titleBar.bottom
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    scrollbarVisible: true
                    scrollBars: root.scrollBars

                    Column {
                        id: detailContent
                        x: Theme.controls.settingsWindow.contentInset
                        width: Math.max(0, detailScroll.width
                                          - 2 * Theme.controls.settingsWindow.contentInset)
                        spacing: Theme.primitive.spacing.lg

                        PaneHeader {
                            id: header
                            width: parent.width
                            // Only panes whose macOS capture has a header card
                            // show one; the settings-list panes open at their
                            // first group. A General sub-pane replaces the
                            // header with its own back chrome.
                            visible: !root.subPaneOpen
                                     && SettingsPanes.hasHeader(root.currentPane)
                            pane: root.currentPane
                        }

                        // Panes may ask the shell to open another pane (the
                        // Appearance pane's Motion link, T227).
                        Connections {
                            target: paneBody.item
                            ignoreUnknownSignals: true
                            function onNavigationRequested(paneId) {
                                root.selectPane(paneId);
                            }
                        }

                        Loader {
                            id: paneBody
                            width: parent.width
                            sourceComponent: root.paneComponent(root.currentPaneId)
                            onLoaded: {
                                root.loadedPaneId = root.currentPaneId;
                                // A General sub-pane requested before the
                                // Loader swapped (Storage deep link).
                                if (root.pendingSubPaneId.length > 0
                                        && root.currentPaneId === "general") {
                                    paneBody.item.openSubPane(root.pendingSubPaneId);
                                    root.pendingSubPaneId = "";
                                }
                                if (root.pendingAboutPage && root.currentPaneId === "general") {
                                    paneBody.item.openSubPane("about");
                                    root.pendingAboutPage = false;
                                }
                                // Panes with an in-pane popup get the shell
                                // root as its parent so the detail
                                // ScrollView's clip cannot cut it off.
                                if (paneBody.item.popupParent !== undefined)
                                    paneBody.item.popupParent = root;
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
    Accessible.name: qsTr("System Settings")
}
