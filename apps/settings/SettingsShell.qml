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
    // T264: history is a list of `{ paneId, subPaneId, rowId }` entries, so
    // Back closes a General sub-pane before it changes pane and Forward
    // restores the sub-pane (and the row that was landed on).
    property var history: []
    property int historyIndex: -1
    // The row a deep link (or a history replay) landed on; part of the
    // current history entry.
    property string currentRowId: ""
    // Guards the history recorder while Back/Forward apply an entry.
    property bool restoringHistory: false
    // The entry currently being applied. It survives the Loader swap so the
    // sub-pane opens only after the pane body exists.
    property var historyApplyEntry: null
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

    // T265: the field's text is immediate, but the index scan is debounced so
    // typing never re-runs the search or rebuilds the sidebar model on every
    // keystroke. Clearing the field is immediate so the sidebar returns at once.
    property string searchQuery: ""
    // Pane ids whose "Show all" row was activated; reset by every new query.
    property var searchExpandedGroups: []
    readonly property bool searchActive: root.searchQuery.trim().length > 0
    readonly property var searchResults: SettingsSearch.search(
        root.searchQuery, { expanded: root.searchExpandedGroups })
    readonly property int searchResultCount:
        SettingsSearch.resultCount(root.searchResults)

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
    property alias overlayLayer: windowOverlay
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

    readonly property var visiblePanes: SettingsPanes.filter(root.searchQuery)
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
    // T264: the business-logic map is the one Help document. The bottom-right
    // `?` and the Help menu call the same live action; without the document
    // the action disables and explains why.
    readonly property bool helpAvailable: Settings.mapDocUrl.toString().length > 0
    readonly property string helpUnavailableReason:
        qsTr("The business-logic map is not installed.")
    // The last fragment opened, so a scripted check can assert the exact
    // anchor the control used.
    property string lastHelpUrl: ""

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

    // ── History (T264) ─────────────────────────────────────────────────────
    // One entry is a full navigation state, not a bare pane id: Back in
    // General -> About closes About before it leaves General, and Forward
    // restores the sub-pane and the row that was landed on.
    function entryFor(paneId, subPaneId, rowId) {
        return { paneId: paneId || "",
                 subPaneId: subPaneId || "",
                 rowId: rowId || "" };
    }

    function currentEntry() {
        return root.entryFor(root.currentPaneId, root.currentSubPaneId,
                             root.currentRowId);
    }

    function sameEntry(a, b) {
        return a !== null && a !== undefined && b !== null && b !== undefined
                && a.paneId === b.paneId
                && (a.subPaneId || "") === (b.subPaneId || "")
                && (a.rowId || "") === (b.rowId || "");
    }

    function recordEntry(entry) {
        if (root.restoringHistory || root.historyApplyEntry !== null)
            return;
        if (root.historyIndex >= 0 && root.historyIndex < root.history.length
                && root.sameEntry(root.history[root.historyIndex], entry))
            return;
        var next = root.history.slice(0, root.historyIndex + 1);
        next.push(entry);
        root.history = next;
        root.historyIndex = next.length - 1;
    }

    // Record the live pane/sub-pane state. Sub-pane changes call this; a pane
    // swap records its own entry explicitly because the Loader may still hold
    // the outgoing body for one frame.
    function recordHistory() {
        if (root.restoringHistory || root.historyApplyEntry !== null)
            return;
        if (root.currentPaneId.length === 0
                || root.loadedPaneId !== root.currentPaneId
                || paneBody.item === null)
            return;
        root.recordEntry(root.currentEntry());
    }

    // A deep link's row lands after the pane/sub-pane exists; update the
    // entry that navigation already recorded instead of pushing a new one.
    function updateCurrentEntry() {
        if (root.restoringHistory || root.historyApplyEntry !== null)
            return;
        if (root.historyIndex < 0 || root.historyIndex >= root.history.length)
            return;
        var entry = root.currentEntry();
        if (root.sameEntry(root.history[root.historyIndex], entry))
            return;
        var next = root.history.slice();
        next[root.historyIndex] = entry;
        root.history = next;
    }

    // Open a resolved catalog target: a pane plus an optional sub-pane or
    // section anchor. Used by the sidebar links and the deep-link resolver.
    function openTarget(target) {
        if (target === null)
            return;
        root.searchText = "";
        var samePane = root.currentPaneId === target.paneId;
        if (!samePane)
            root.selectPane(target.paneId);
        var wantsSubPane = target.subPaneId !== undefined
                && target.subPaneId.length > 0;
        if (wantsSubPane)
            root.openSubPane(target.subPaneId);
        // Clicking the already-selected General row closes its sub-pane, the
        // reference behaviour for an in-place surface.
        else if (samePane && root.subPaneOpen)
            root.closeSubPane();
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

    function closeSubPane() {
        if (root.currentPaneId === "general" && paneBody.item !== null
                && paneBody.item.closeSubPane !== undefined)
            paneBody.item.closeSubPane();
    }

    // Show a pane, recording the move in history. Selecting the current pane
    // is a no-op so arrow-key echo does not grow the stack.
    function selectPane(id) {
        if (!id || id === root.currentPaneId)
            return;
        root.trace("selectPane " + id);
        root.clearPendingHighlight();
        root.pendingSubPaneId = "";
        root.currentRowId = "";
        // Move focus off the outgoing body before the Loader destroys it; a
        // control that is about to be destroyed must never keep focus.
        root.forceActiveFocus();
        root.currentPaneId = id;
        root.syncSelection();
        root.recordEntry(root.entryFor(id, "", ""));
        Qt.callLater(root.focusAfterNavigation);
    }

    function openAboutPage() {
        root.pendingAboutPage = true;
        root.selectPane("general");
        if (root.loadedPaneId === "general" && root.paneBody.item) {
            root.paneBody.item.openSubPane("about");
            root.pendingAboutPage = false;
        }
    }

    // Apply one history entry. The sub-pane opens/closes only after the pane
    // body exists, so the entry survives the Loader swap.
    function applyHistoryEntry(entry) {
        if (entry === null || entry === undefined)
            return;
        root.clearPendingHighlight();
        root.historyApplyEntry = entry;
        root.currentRowId = entry.rowId || "";
        root.pendingPaneId = entry.paneId;
        root.pendingRowId = entry.rowId || "";
        root.pendingRowPaneId = entry.paneId;
        root.pendingSection = "";
        if (root.currentPaneId !== entry.paneId) {
            root.restoringHistory = true;
            root.forceActiveFocus();
            root.currentPaneId = entry.paneId;
            root.syncSelection();
            root.restoringHistory = false;
        }
        root.finishHistoryEntry();
    }

    function finishHistoryEntry() {
        var entry = root.historyApplyEntry;
        if (entry === null || entry === undefined)
            return;
        if (root.loadedPaneId !== entry.paneId)
            return;
        root.historyApplyEntry = null;
        root.restoringHistory = true;
        if (paneBody.item !== null) {
            if ((entry.subPaneId || "").length > 0) {
                if (paneBody.item.openSubPane !== undefined)
                    paneBody.item.openSubPane(entry.subPaneId);
            } else if (paneBody.item.closeSubPane !== undefined) {
                paneBody.item.closeSubPane();
            }
        }
        root.restoringHistory = false;
        root.detailScroll.flickable.contentY = 0;
        if ((entry.rowId || "").length > 0) {
            root.pendingPaneId = entry.paneId;
            root.pendingRowId = entry.rowId;
            root.pendingRowPaneId = entry.paneId;
            Qt.callLater(root.revealPending);
        } else {
            Qt.callLater(root.focusAfterNavigation);
        }
    }

    function back() {
        if (!root.canGoBack)
            return;
        root.historyIndex -= 1;
        root.applyHistoryEntry(root.history[root.historyIndex]);
    }

    function forward() {
        if (!root.canGoForward)
            return;
        root.historyIndex += 1;
        root.applyHistoryEntry(root.history[root.historyIndex]);
    }

    // The first keyboard stop in a pane body (a row, control, or the sub-pane
    // back button). Focus never stays on a control the Loader destroyed.
    function firstFocusable(item) {
        if (item === null || item === undefined || item.visible === false)
            return null;
        if (item.activeFocusOnTab === true && item.enabled !== false)
            return item;
        var children = item.children;
        for (var i = 0; i < children.length; ++i) {
            var found = root.firstFocusable(children[i]);
            if (found !== null)
                return found;
        }
        return null;
    }

    function focusAfterNavigation() {
        root.detailScroll.flickable.contentY = 0;
        var target = root.firstFocusable(paneBody.item);
        if (target === null && header.visible)
            target = root.firstFocusable(header);
        if (target === null)
            target = root.titleBar;
        if (target !== null && target.forceActiveFocus !== undefined)
            target.forceActiveFocus();
    }

    function syncSelection() {
        root.syncing = true;
        sidebar.currentIndex = root.sidebarIndexOf(root.currentPaneId);
        root.syncing = false;
    }

    function focusSearch() {
        searchField.forceActiveFocus();
    }

    // Apply the field's current text without waiting for the debounce. Used by
    // the Escape path and by tests/captures that drive the field directly.
    function commitSearch() {
        searchDebounce.stop();
        root.searchQuery = root.searchText;
    }

    // A "Show all" row expands its pane group in place; the query and the
    // field text stay untouched.
    function expandSearchGroup(paneId) {
        if (paneId === undefined || paneId === null || paneId.length === 0)
            return;
        if (root.searchExpandedGroups.indexOf(paneId) >= 0)
            return;
        root.searchExpandedGroups = root.searchExpandedGroups.concat([paneId]);
    }

    // T264: one live Help action. The fragment is `#pane-<id>` (the anchor the
    // business-logic map publishes), never a bare pane id. The bottom-right
    // `?` and Help -> Settings Help both call `openHelp`.
    function paneHelpUrl(paneId) {
        if (!root.helpAvailable)
            return "";
        var id = String(paneId || root.currentPaneId);
        if (id.length === 0)
            return "";
        return Settings.helpUrl("pane-" + id).toString();
    }

    function openHelp(paneId) {
        var url = root.paneHelpUrl(paneId);
        if (url.length === 0)
            return false;
        root.lastHelpUrl = url;
        Qt.openUrlExternally(url);
        return true;
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
        var row = null;
        var surface = "";
        if (link.rowId.length > 0) {
            row = SettingsMap.rowFor(link.paneId, link.rowId);
            if (row === null || row.state !== "shipped")
                return false;
            surface = SettingsMap.rowSurface(link.paneId, link.rowId);
            // A row in a closed surface is only accepted when the shell can
            // open that surface; otherwise it is not landable.
            if (surface.length === 0)
                return false;
        }
        root.searchText = "";
        if (root.currentPaneId !== target.paneId)
            root.selectPane(target.paneId);
        // A row in an in-place General sub-pane opens that sub-pane first.
        var subPaneId = target.subPaneId;
        if (subPaneId.length === 0 && row !== null)
            subPaneId = SettingsMap.subPaneForRow(link.paneId, link.rowId);
        if (subPaneId.length === 0 && surface === "dialog" && root.subPaneOpen)
            root.closeSubPane();
        root.pendingPaneId = target.paneId;
        root.pendingRowId = link.rowId;
        root.pendingRowPaneId = link.paneId;
        root.pendingSection = target.anchor;
        root.currentRowId = "";
        if (subPaneId.length > 0)
            root.openSubPane(subPaneId);
        if (root.currentPaneId === target.paneId)
            Qt.callLater(root.revealPending);
        return true;
    }

    function activateSearchResult(result) {
        if (result === null || result === undefined)
            return;
        if (result.kind === "showAll") {
            var at = searchResultsList.currentIndex;
            root.expandSearchGroup(result.paneId);
            // The first formerly-hidden row now sits where the row was.
            Qt.callLater(function() {
                if (root.searchResults.length > 0)
                    searchResultsList.currentIndex = Math.min(
                        at, root.searchResults.length - 1);
            });
            return;
        }
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
    // `settingsId` is the stable match. The label/synonym fallback is only for
    // rows that predate the map wiring: a row that carries a stable id is
    // never guessed by a bidirectional substring match.
    function findRowTarget(item, row) {
        if (item === null || item === undefined || item.visible === false)
            return null;
        if (row !== null && item.settingsId !== undefined
                && item.settingsId === row.id)
            return item;
        var hasStableId = item.settingsId !== undefined
                && String(item.settingsId).length > 0;
        if (row !== null && !hasStableId && item.label !== undefined
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
    // same-pane deep-link path. A row in a closed surface (the Software Update
    // dialog) opens that surface first; a row the resolver cannot land is
    // never deep-linked (see `SettingsMap.isRowLandable`).
    function revealPending() {
        if (root.pendingPaneId.length === 0
                || root.pendingPaneId !== root.currentPaneId
                || root.pendingPaneId !== root.loadedPaneId)
            return;
        var body = paneBody.item;
        if (body === null)
            return;
        var rowPaneId = root.pendingRowPaneId.length > 0
                ? root.pendingRowPaneId : root.pendingPaneId;
        var row = root.pendingRowId.length > 0
                ? SettingsMap.rowFor(rowPaneId, root.pendingRowId) : null;
        var surface = row !== null
                ? SettingsMap.rowSurface(rowPaneId, row.id) : "";
        // A row in an in-place sub-pane can only be found once that sub-pane
        // body exists; `subPaneLoaded` re-enters this function.
        if (surface === "subpane" && body.subPaneLoader !== undefined
                && body.subPaneLoader.status !== Loader.Ready)
            return;
        if (surface === "dialog" && body.openRowSurface !== undefined)
            body.openRowSurface(row.id);
        var target = row !== null ? root.findRowTarget(body, row) : null;
        if (target === null && row !== null)
            target = root.findSectionTarget(body, row.section);
        if (target === null && root.pendingSection.length > 0)
            target = root.findSectionTarget(body, root.pendingSection);
        if (target === null)
            target = body;
        if (surface !== "dialog") {
            var point = target.mapToItem(detailContent, 0, 0);
            detailScroll.flickable.contentY = Math.max(
                0, Math.min(detailScroll.maxY,
                            point.y - Theme.primitive.spacing.lg));
        }
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
        root.currentRowId = root.pendingRowId;
        root.pendingPaneId = "";
        root.pendingRowId = "";
        root.pendingRowPaneId = "";
        root.pendingSection = "";
        root.updateCurrentEntry();
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

    // T265: one debounce for the index scan and the sidebar filter. The field
    // itself stays immediate; only the derived models wait.
    Timer {
        id: searchDebounce
        interval: 120
        repeat: false
        onTriggered: root.searchQuery = root.searchText
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
            root.history = [root.entryFor(first, "", "")];
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
        // T264: a pane change always starts at the top of the detail view.
        if (detailScroll !== null && detailScroll.flickable !== null)
            detailScroll.flickable.contentY = 0;
        if (root.pendingPaneId.length > 0
                && root.pendingPaneId === root.currentPaneId
                && root.pendingPaneId === root.loadedPaneId)
            Qt.callLater(root.revealPending);
    }

    // A General sub-pane open/close is a navigation step of its own: Back
    // closes it before leaving the pane, Forward restores it.
    onCurrentSubPaneIdChanged: {
        if (root.restoringHistory || root.historyApplyEntry !== null)
            return;
        if (paneBody.item === null
                || root.loadedPaneId !== root.currentPaneId)
            return;
        root.currentRowId = "";
        root.detailScroll.flickable.contentY = 0;
        root.recordHistory();
    }

    onSearchTextChanged: {
        if (searchField.text !== root.searchText)
            searchField.text = root.searchText;
        root.searchExpandedGroups = [];
        if (root.searchText.trim().length === 0) {
            // Empty query restores the sidebar immediately; no debounce.
            searchDebounce.stop();
            root.searchQuery = "";
        } else {
            searchDebounce.restart();
        }
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

    // T264: the View menu's Settings Pane submenu and its Back/Forward rows
    // read the live navigation state. One model, no second menu list.
    Binding {
        target: SettingsMenu
        property: "currentPaneId"
        value: root.currentPaneId
    }
    Binding {
        target: SettingsMenu
        property: "canGoBack"
        value: root.canGoBack
    }
    Binding {
        target: SettingsMenu
        property: "canGoForward"
        value: root.canGoForward
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
                        onAccepted: {
                            // Return within the debounce window still opens the
                            // top result instead of doing nothing.
                            root.commitSearch();
                            searchResultsList.activateCurrent();
                        }
                        onEscapePressed: {
                            if (root.searchText.length > 0)
                                root.searchText = "";
                            else
                                sidebar.forceActiveFocus();
                        }
                        Keys.onDownPressed: (event) => {
                            root.commitSearch();
                            searchResultsList.focusList();
                            event.accepted = true;
                        }
                        Keys.onUpPressed: (event) => {
                            root.commitSearch();
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
                            query: root.searchQuery
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
                    // T266: the bottom-right window grip owns the corner; the
                    // scrollbars stop short of it.
                    cornerInset: Theme.controls.window.resizeCorner

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
                            function onSubPaneLoaded(subPaneId) {
                                // A deep link that needed a closed sub-pane
                                // waits for this before it scrolls.
                                if (root.pendingPaneId.length > 0)
                                    Qt.callLater(root.revealPending);
                            }
                        }

                        Loader {
                            id: paneBody
                            width: parent.width
                            sourceComponent: root.paneComponent(root.currentPaneId)
                            onLoaded: {
                                root.loadedPaneId = root.currentPaneId;
                                if (root.historyApplyEntry !== null
                                        && root.historyApplyEntry.paneId
                                           === root.currentPaneId) {
                                    // Back/Forward restores the sub-pane (and
                                    // row) only after the body exists.
                                    root.finishHistoryEntry();
                                } else {
                                    // A General sub-pane requested before the
                                    // Loader swapped (Storage deep link).
                                    if (root.pendingSubPaneId.length > 0
                                            && root.currentPaneId === "general") {
                                        paneBody.item.openSubPane(root.pendingSubPaneId);
                                        root.pendingSubPaneId = "";
                                    }
                                    if (root.pendingAboutPage
                                            && root.currentPaneId === "general") {
                                        paneBody.item.openSubPane("about");
                                        root.pendingAboutPage = false;
                                    }
                                }
                                // Panes no longer opt into a popup parent: a
                                // Dialog or Select adopts the window overlay
                                // layer itself (T266).
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
                    Accessible.description: root.helpAvailable
                                            ? qsTr("Open this pane in the business-logic map.")
                                            : root.helpUnavailableReason
                    enabled: root.helpAvailable
                    onClicked: root.openHelp(root.currentPaneId)
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

    // T266: the window overlay layer for pane Selects and Dialogs. It covers
    // the whole window above the resize grips, so a surface inside the detail
    // ScrollView is never clipped.
    WindowOverlay {
        id: windowOverlay
    }

    // The frameless window owns its edge input; the shared grips report the
    // matching `Qt.Edges` and the host forwards them to `startSystemResize`.
    // The bottom-right corner is reserved for the detail scrollbar (T266).
    WindowResizeGrips {
        id: resizeEdges
        anchors.fill: parent
        cornerAvoidance: Theme.controls.scrollView.scrollbarWidth
                         + 2 * Theme.controls.scrollView.scrollbarMargin
        onResizeRequested: (edges) => root.resizeRequested(edges)
    }

    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("System Settings")
}
