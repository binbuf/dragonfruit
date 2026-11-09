// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Applications drawer (T-19.2): the shell's primary "start an app"
// affordance. A full-screen overlay listing every launchable app the app-index
// corpus knows about, alphabetical by default (`buildAppsDrawerList` sorts),
// with a search field and category filter pills, and a fixed-size tile grid
// whose column count is capped at seven and whose viewport is capped at five
// rows (a longer corpus scrolls); it drops to fewer columns on a narrower
// output.
//
// The shell controller injects `apps` (the sorted/deduped `AppsDrawerRow`
// projection) and `available` (app-index reachability). Filtering by category
// and query is local, mirroring the pure helper's predicates exactly as the
// Dock's Add Application picker does (T-14.7e): the helper owns sort/dedupe/
// category mapping, the view owns the last-mile narrowing.
Item {
    id: root

    // [{ desktopId, name, iconPath, categories: [key, ...] }]
    property var apps: []
    // False when app-index is absent: render the explicit absence row.
    property bool available: true
    property bool active: false
    // The selected category pill key; "all" (the default) includes everything.
    property string category: "all"
    // The live search text.
    property string query: ""
    // The highlighted tile; -1 = none.
    property int currentIndex: -1
    // T-21.2c: the shell applies the compositor's Drawer role state. Without
    // the sampled pass the card takes the token-driven milky plate.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    signal appLaunched(string desktopId, real tileX, real tileY, real tileW, real tileH)
    signal dismissRequested()

    // Canonical pill order (the same keys `appsdrawer.cpp` maps to) and the
    // one label table. The C++ helper owns the freedesktop -> key mapping; the
    // label lives here so it is translatable.
    readonly property var categoryOrder: [
        "all", "developer-tools", "productivity", "utilities", "entertainment",
        "games", "social", "creativity", "information"
    ]
    readonly property var categoryLabels: ({
        "all": qsTr("All"),
        "developer-tools": qsTr("Developer Tools"),
        "productivity": qsTr("Productivity & Finance"),
        "utilities": qsTr("Utilities"),
        "entertainment": qsTr("Entertainment"),
        "games": qsTr("Games"),
        "social": qsTr("Social"),
        "creativity": qsTr("Creativity"),
        "information": qsTr("Information")
    })

    readonly property real reveal: Theme.reducedMotion
        ? (root.active ? 1.0 : 0.0)
        : root.shown
    property real shown: 0
    onActiveChanged: {
        root.shown = root.active ? 1.0 : 0.0
        // Every open is a fresh launcher session: no stale query, category,
        // selection, or scroll offset from the last time it was dismissed.
        // Typing filters the list, so the search field owns active focus the
        // moment the drawer opens; navigation keys are forwarded below.
        if (root.active) {
            root.resetSession()
            Qt.callLater(root.focusSearch)
        }
    }
    Behavior on shown {
        enabled: !Theme.reducedMotion
        NumberAnimation {
            duration: Theme.motion.windowOpen.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: Theme.motion.windowOpen.curve
        }
    }

    // The pills actually shown: "All" plus every category present in the
    // corpus, in canonical order, never a pill no app carries.
    readonly property var pills: {
        var present = ({});
        for (var i = 0; i < root.apps.length; ++i) {
            var cats = root.apps[i].categories;
            if (cats === undefined)
                continue;
            for (var c = 0; c < cats.length; ++c)
                present[cats[c]] = true;
        }
        var out = [];
        for (var k = 0; k < root.categoryOrder.length; ++k) {
            var key = root.categoryOrder[k];
            if (key === "all" || present[key] === true)
                out.push({ key: key, label: root.categoryLabels[key] });
        }
        return out;
    }

    readonly property int categoryIndex: {
        for (var i = 0; i < root.pills.length; ++i) {
            if (root.pills[i].key === root.category)
                return i;
        }
        return 0;
    }

    // The visible tiles: category AND query, mirroring the pure helper.
    readonly property var visibleApps: {
        var needle = root.query.trim().toLowerCase();
        var out = [];
        for (var i = 0; i < root.apps.length; ++i) {
            var app = root.apps[i];
            var id = app.desktopId !== undefined ? String(app.desktopId) : "";
            var name = app.name !== undefined ? String(app.name) : "";
            if (root.category !== "all") {
                var cats = app.categories !== undefined ? app.categories : [];
                if (cats.indexOf(root.category) < 0)
                    continue;
            }
            if (needle !== ""
                    && name.toLowerCase().indexOf(needle) < 0
                    && id.toLowerCase().indexOf(needle) < 0) {
                continue;
            }
            out.push(app);
        }
        return out;
    }

    readonly property bool showAbsence: !root.available
    readonly property bool noMatches: root.visibleApps.length === 0 && !root.showAbsence

    // The card wraps the grid (at most seven columns) with at least
    // `panelPadding` on the sides and a tighter `panelPaddingV` above and below,
    // floored at the search field's width so a short list still reads as a card,
    // and clamped to the output.
    readonly property int maxColumns: 7
    readonly property real panelPadding: Theme.controls.appsDrawer.panelPadding
    readonly property real panelPaddingV:
        Theme.controls.appsDrawer.panelPaddingVertical
    // The hairline rules under the top search section and the pill row.
    readonly property real separatorHeight: Theme.controls.window.borderWidth
    readonly property real tileWidth: Theme.controls.appsDrawer.tileSize
                                      + Theme.primitive.spacing.xl
    // One tile's full height: the artwork square plus its single-line label.
    readonly property real tileHeight:
        root.tileWidth + Math.ceil(Theme.controls.appsDrawer.labelSize * 2.2)
    readonly property real gridGap: Theme.controls.appsDrawer.gridColumnGap
    readonly property real gridRowGap: Theme.controls.appsDrawer.gridRowGap
    readonly property real gridMaxWidth:
        maxColumns * root.tileWidth + (maxColumns - 1) * root.gridGap
    // The grid viewport shows at most five rows; a longer corpus scrolls.
    readonly property int maxRows: 5
    readonly property real gridMaxHeight:
        root.maxRows * root.tileHeight + (root.maxRows - 1) * root.gridRowGap
    readonly property real panelWidth: Math.max(0, Math.min(
        Math.max(root.gridMaxWidth, Theme.controls.appsDrawer.searchMaxWidth)
            + 2 * root.panelPadding,
        root.width - 2 * Theme.controls.appsDrawer.contentPadding / 2))
    // The grid's usable width inside the card; it decides how many columns fit.
    readonly property real gridAvailableWidth:
        Math.max(root.tileWidth, root.panelWidth - 2 * root.panelPadding)
    // The columns actually laid out: capped at seven, fewer on a narrow output,
    // and fewer again when the corpus is smaller than a full row, so the row of
    // tiles keeps equal padding at both ends.
    readonly property int usedColumns: Math.max(1, Math.min(
        root.maxColumns,
        Math.min(root.visibleApps.length > 0 ? root.visibleApps.length
                                            : root.maxColumns,
                 Math.floor((root.gridAvailableWidth + root.gridGap)
                            / (root.tileWidth + root.gridGap)))))
    readonly property real gridWidth:
        root.usedColumns * root.tileWidth + (root.usedColumns - 1) * root.gridGap
    readonly property int columns: root.usedColumns
    readonly property int rows: root.visibleApps.length === 0 ? 0
        : Math.ceil(root.visibleApps.length / root.columns)

    // The card height: the top search bar, the two hairline rules, pills, the
    // grid, and the vertical padding, clamped to five grid rows and to the
    // output so a long corpus scrolls.
    readonly property real emptyContentHeight:
        Theme.controls.appsDrawer.emptyGlyphSize + Theme.primitive.spacing.xxl
    readonly property real gridContentHeight:
        (root.showAbsence || root.noMatches)
            ? root.emptyContentHeight
            : (root.rows === 0 ? 0
                               : root.rows * root.tileHeight
                                 + (root.rows - 1) * root.gridRowGap)
    readonly property real gridNaturalHeight:
        Math.min(root.gridContentHeight, root.gridMaxHeight)
    readonly property real panelNaturalHeight:
        root.panelPaddingV + searchField.height
        + Theme.primitive.spacing.lg + root.separatorHeight
        + Theme.primitive.spacing.lg + pillScroller.height
        + Theme.primitive.spacing.lg + root.separatorHeight
        + Theme.primitive.spacing.xl + root.gridNaturalHeight
        + root.panelPaddingV
    readonly property real panelMaxHeight:
        Math.max(0, root.height - 2 * Theme.controls.appsDrawer.contentPadding / 2)

    function setQueryText(text) { searchField.text = text; }

    // Clear every piece of per-session state. The drawer is a launcher, not a
    // persistent search pane: reopening it must never inherit the previous
    // query (which used to hide the whole grid behind a stale filter).
    function resetSession() {
        if (!searchField)
            return;
        if (searchField.text.length > 0)
            searchField.text = "";
        root.query = "";
        root.category = "all";
        root.currentIndex = -1;
        if (gridFlick) {
            gridFlick.contentY = 0;
            gridFlick.contentX = 0;
        }
        if (pillScroller)
            pillScroller.contentX = 0;
        searchField.input.cursorPosition = 0;
    }

    // Make the search field the one keyboard owner. The tiles are deliberately
    // not focusable: a delegate taking focus on every model rebuild was what
    // dropped Backspace/Delete and made the second keystroke behave
    // differently from the first.
    function focusSearch() {
        if (!root.active || !searchField)
            return;
        searchField.forceActiveFocus();
        searchField.input.forceActiveFocus();
    }

    // The current filter changed: start a fresh keyboard selection and return
    // the grid to the top so the first result is where the user looks.
    function filterChanged() {
        root.currentIndex = -1;
        if (gridFlick)
            gridFlick.contentY = 0;
    }

    // Keep the keyboard selection inside the five-row viewport. A launcher
    // that moves its highlight off-screen is a dead end for keyboard users.
    function revealCurrent() {
        if (root.currentIndex < 0 || !gridFlick)
            return;
        var row = Math.floor(root.currentIndex / root.columns);
        var top = row * (root.tileHeight + root.gridRowGap);
        var bottom = top + root.tileHeight;
        var maxY = Math.max(0, gridFlick.contentHeight - gridFlick.height);
        if (top < gridFlick.contentY)
            gridFlick.contentY = top;
        else if (bottom > gridFlick.contentY + gridFlick.height)
            gridFlick.contentY = Math.min(maxY, bottom - gridFlick.height);
    }

    // The scene rectangle of the tile for `desktopId`, or null when it is not
    // in the visible set. The capture seam reads this to click a known tile.
    function tileRectFor(desktopId) {
        for (var i = 0; i < root.visibleApps.length; ++i) {
            if (String(root.visibleApps[i].desktopId) !== String(desktopId))
                continue;
            var tile = tileRepeater.itemAt(i);
            if (!tile)
                return null;
            var p = tile.mapToItem(null, 0, 0);
            return { x: p.x, y: p.y, width: tile.width, height: tile.width };
        }
        return null;
    }

    // The scene rectangle of the card (T-20.1). The shell controller reads
    // this to declare the drawer's panel to the compositor, so the GPU
    // backdrop blur follows the card rather than the full-output overlay.
    function panelRect() {
        // Before the controller sizes the offscreen window the card can be
        // degenerate. Never hand the compositor a negative Size (smithay
        // panics constructing one); the drawer commits a real rect as soon as
        // it has an output size.
        if (!panel || panel.width <= 0 || panel.height <= 0)
            return { x: 0, y: 0, width: 0, height: 0 };
        var p = panel.mapToItem(null, 0, 0);
        var far = panel.mapToItem(null, panel.width, panel.height);
        return { x: p.x, y: p.y, width: far.x - p.x, height: far.y - p.y };
    }

    function launchAt(index) {
        // Return on a filtered list opens the top hit, the Spotlight/Launchpad
        // behaviour; an explicit arrow selection always wins.
        if (index < 0 && root.query.trim().length > 0 && root.visibleApps.length > 0)
            index = 0;
        if (index < 0 || index >= root.visibleApps.length)
            return;
        var tile = tileRepeater.itemAt(index);
        var app = root.visibleApps[index];
        var tl = tile ? tile.mapToItem(null, 0, 0) : Qt.point(0, 0);
        var size = tile ? tile.width : 0;
        root.appLaunched(String(app.desktopId), tl.x, tl.y, size, size);
    }

    function moveSelection(delta) {
        var count = root.visibleApps.length;
        if (count === 0) {
            root.currentIndex = -1;
            return;
        }
        if (root.currentIndex < 0)
            root.currentIndex = delta > 0 ? 0 : count - 1;
        else
            root.currentIndex = Math.max(0, Math.min(count - 1,
                                                     root.currentIndex + delta));
        root.revealCurrent();
    }

    function moveSelectionTo(index) {
        var count = root.visibleApps.length;
        if (count === 0) {
            root.currentIndex = -1;
            return;
        }
        root.currentIndex = Math.max(0, Math.min(count - 1, index));
        root.revealCurrent();
    }

    onVisibleAppsChanged: {
        // Do not auto-select the first tile. That used to (a) give the drawer a
        // permanent highlight before the user touched anything and (b) move
        // keyboard focus onto a delegate whenever the query rebuilt the model.
        if (root.visibleApps.length === 0)
            root.currentIndex = -1;
        else if (root.currentIndex >= root.visibleApps.length)
            root.currentIndex = root.visibleApps.length - 1;
    }
    onQueryChanged: root.filterChanged()
    onCategoryChanged: root.filterChanged()
    // A category pill can disappear when the app-index corpus changes while
    // the drawer is open; never leave a stale filter that matches nothing.
    onPillsChanged: {
        var found = false;
        for (var i = 0; i < root.pills.length; ++i) {
            if (root.pills[i].key === root.category) {
                found = true;
                break;
            }
        }
        if (!found)
            root.category = "all";
    }

    focus: root.active
    activeFocusOnTab: root.active

    // The faint scrim behind the panel.
    Rectangle {
        objectName: "appsDrawerScrim"
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: Theme.controls.appsDrawer.scrimOpacity * root.reveal
    }

    // A click outside the panel dismisses. The panel consumes its own clicks.
    TapHandler {
        onTapped: root.dismissRequested()
    }

    Rectangle {
        id: panel
        objectName: "appsDrawerPanel"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        width: root.panelWidth
        height: Math.min(root.panelNaturalHeight, root.panelMaxHeight)
        radius: Theme.primitive.radius.xl
        // A translucent frosted fill: the compositor's liquid-glass material
        // (blur + edge lens + specular rim + adaptive tint, T-20.2) and the
        // scrim/desktop ghost through, while the children (search, pills,
        // tiles) stay fully opaque because the opacity is in the fill, not on
        // the item. Without the sampled pass it takes the milky complement.
        color: Material.drawerFill(root.materialBlurActive, root.materialTier,
                                   root.materialFallback)
        opacity: root.reveal
        scale: 0.97 + 0.03 * root.reveal
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true


        // Consume clicks on the panel so a miss on a tile does not dismiss,
        // and return typing to the search field (a click on the pills or the
        // empty grid should not strand keyboard focus).
        TapHandler {
            onTapped: root.focusSearch()
        }

        // The search bar is the card's top section. It is flat (no pill
        // background or border) so it blends into the card's rounded top; the
        // drawer's name is its placeholder.
        SearchField {
            id: searchField
            objectName: "appsDrawerSearch"
            anchors.top: parent.top
            anchors.topMargin: root.panelPaddingV
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.panelWidth - 2 * root.panelPadding
            flat: true
            placeholderText: qsTr("Applications")
            onTextChanged: root.query = text
            // Return launches the selected tile from the text field; Escape
            // dismisses the launcher outright (Launchpad behaviour), and the
            // next open starts with an empty search.
            onAccepted: root.launchAt(root.currentIndex)
            onEscapePressed: root.dismissRequested()
            // The text field owns focus while the drawer is open. Forward the
            // grid-navigation keys here so typing filters and arrows still
            // move the tile selection.
            Keys.onDownPressed: (event) => {
                root.moveSelection(root.columns)
                event.accepted = true
            }
            Keys.onUpPressed: (event) => {
                root.moveSelection(-root.columns)
                event.accepted = true
            }
            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_PageDown) {
                    root.moveSelection(root.columns
                                       * Math.max(1, Math.floor(
                                              gridFlick.height
                                              / (root.tileHeight + root.gridRowGap))))
                    event.accepted = true
                } else if (event.key === Qt.Key_PageUp) {
                    root.moveSelection(-root.columns
                                       * Math.max(1, Math.floor(
                                              gridFlick.height
                                              / (root.tileHeight + root.gridRowGap))))
                    event.accepted = true
                }
            }
        }

        // The hairline rule under the search section (the reference's first
        // separator).
        Rectangle {
            id: topSeparator
            objectName: "appsDrawerTopSeparator"
            anchors.top: searchField.bottom
            anchors.topMargin: Theme.primitive.spacing.lg
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width - 2 * Theme.primitive.spacing.xl
            height: root.separatorHeight
            color: Theme.color.border
        }

        // The category pills. Horizontally scrollable so a narrow output never
        // clips a pill; a SegmentedControl owns the selection semantics.
        Flickable {
            id: pillScroller
            objectName: "appsDrawerPills"
            anchors.top: topSeparator.bottom
            anchors.topMargin: Theme.primitive.spacing.lg
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.panelWidth - 2 * root.panelPadding
            height: pillsControl.height
            contentWidth: pillsControl.width
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            interactive: contentWidth > width

            // A plain mouse wheel has only a vertical axis; over the pill
            // strip map it sideways so a narrow output can still reach the
            // last category without discovering a horizontal drag.
            WheelHandler {
                onWheel: (event) => {
                    const delta = event.angleDelta.y !== 0
                            ? event.angleDelta.y : event.angleDelta.x;
                    if (delta === 0)
                        return;
                    const maxX = Math.max(0, pillScroller.contentWidth
                                             - pillScroller.width);
                    pillScroller.contentX = Math.max(0, Math.min(
                        maxX, pillScroller.contentX - delta));
                    event.accepted = true;
                }
            }

            SegmentedControl {
                id: pillsControl
                objectName: "appsDrawerCategoryControl"
                model: root.pills
                currentIndex: root.categoryIndex
                onActivated: (index) => { root.category = root.pills[index].key; }
            }
        }

        // The hairline rule under the pills (the reference's second separator).
        Rectangle {
            id: pillsSeparator
            objectName: "appsDrawerPillsSeparator"
            anchors.top: pillScroller.bottom
            anchors.topMargin: Theme.primitive.spacing.lg
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width - 2 * Theme.primitive.spacing.xl
            height: root.separatorHeight
            color: Theme.color.border
        }

        // The tile grid. A `Grid` gives the column and row gaps their own
        // tokens while the column count stays matched to the real layout.
        Flickable {
            id: gridFlick
            objectName: "appsDrawerGrid"
            anchors.top: pillsSeparator.bottom
            anchors.topMargin: Theme.primitive.spacing.xl
            anchors.bottom: parent.bottom
            anchors.bottomMargin: root.panelPaddingV
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.gridWidth
            contentHeight: grid.height
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            Grid {
                id: grid
                objectName: "appsDrawerTiles"
                width: parent.width
                columns: root.columns
                rowSpacing: root.gridRowGap
                columnSpacing: root.gridGap
                visible: root.visibleApps.length > 0

                Repeater {
                    id: tileRepeater
                    model: root.visibleApps

                    delegate: Item {
                        id: tile
                        required property var modelData
                        required property int index

                        objectName: "appsDrawerTile"
                        readonly property string desktopId:
                            modelData.desktopId !== undefined ? String(modelData.desktopId) : ""
                        readonly property string name:
                            modelData.name !== undefined ? String(modelData.name) : ""
                        readonly property string iconPath:
                            modelData.iconPath !== undefined ? String(modelData.iconPath) : ""
                        readonly property string iconUrl:
                            modelData.iconUrl !== undefined ? String(modelData.iconUrl)
                                                            : (tile.iconPath.length > 0
                                                               ? "file://" + tile.iconPath : "")
                        readonly property bool selected: tile.index === root.currentIndex
                        readonly property bool hovered: tileHover.hovered
                        readonly property bool pressed: tileTap.pressed

                        width: root.tileWidth
                        height: root.tileHeight
                        // Tiles never take active focus. The search field is
                        // the drawer's one keyboard owner; a delegate that set
                        // `focus` whenever the filter rebuilt the model stole
                        // focus mid-typing (Backspace then went to the root
                        // instead of the text field).

                        function activate() {
                            root.launchAt(tile.index);
                        }

                        Rectangle {
                            id: tileHighlight
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: 0
                            width: root.tileWidth
                            height: root.tileWidth
                            radius: Theme.controls.appsDrawer.tileRadius
                            color: tile.pressed ? Theme.color.controlActive
                                 : (tile.selected ? Theme.color.accentMuted
                                    : (tile.hovered ? Theme.color.controlFill : "transparent"))
                            border.width: tile.selected ? Theme.controls.window.borderWidth : 0
                            border.color: Theme.color.accent
                        }

                        // The themed app-index icon, rendered at the tile size.
                        Image {
                            id: appIcon
                            objectName: "appsDrawerTileIcon"
                            visible: tile.iconUrl.length > 0 && status === Image.Ready
                            source: tile.iconUrl
                            sourceSize.width: Theme.controls.appsDrawer.tileSize
                            sourceSize.height: Theme.controls.appsDrawer.tileSize
                            width: Theme.controls.appsDrawer.tileSize
                            height: Theme.controls.appsDrawer.tileSize
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: (root.tileWidth - height) / 2
                            fillMode: Image.PreserveAspectFit
                            asynchronous: true
                            smooth: true
                        }

                        // The Tabler fallback while the themed icon loads,
                        // when app-index has none, or when it failed to load.
                        TablerIcon {
                            objectName: "appsDrawerTileFallback"
                            visible: tile.iconUrl.length === 0
                                     || appIcon.status !== Image.Ready
                            name: "app-window"
                            size: Theme.controls.appsDrawer.tileSize
                                     * Theme.controls.appsDrawer.fallbackGlyphRatio
                            strokeWidth: 1.5
                            color: Theme.color.textSecondary
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: (root.tileWidth - height) / 2
                        }

                        Text {
                            objectName: "appsDrawerTileLabel"
                            width: parent.width
                            y: root.tileWidth + Theme.controls.appsDrawer.tileContentGap
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                            maximumLineCount: 1
                            text: tile.name
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.appsDrawer.labelSize
                            // The label sits on the card, not on the icon
                            // highlight; using the accent's *content* colour
                            // made the selected name white-on-light (or
                            // dark-on-dark) and effectively invisible.
                            font.weight: tile.selected
                                         ? Theme.primitive.font.weightMedium
                                         : Theme.primitive.font.weightRegular
                        }

                        HoverHandler { id: tileHover }
                        TapHandler {
                            id: tileTap
                            acceptedButtons: Qt.LeftButton
                            onTapped: tile.activate()
                        }

                        Accessible.role: Accessible.ListItem
                        Accessible.name: tile.name
                        Accessible.focusable: true
                        Accessible.selected: tile.selected
                        Accessible.onPressAction: tile.activate()
                    }
                }
            }

            // The empty-filter state: an explicit row, never a blank panel.
            Item {
                objectName: "appsDrawerEmpty"
                anchors.fill: parent
                visible: root.noMatches

                Column {
                    anchors.centerIn: parent
                    spacing: Theme.primitive.spacing.sm

                    TablerIcon {
                        anchors.horizontalCenter: parent.horizontalCenter
                        name: "search"
                        size: Theme.controls.appsDrawer.emptyGlyphSize
                        strokeWidth: 1.5
                        color: Theme.color.textTertiary
                    }

                    Text {
                        objectName: "appsDrawerEmptyText"
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: qsTr("No applications found")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeMd
                    }
                }
            }

            // app-index absent: the explicit absence row (ADR 0167).
            Item {
                objectName: "appsDrawerUnavailable"
                anchors.horizontalCenter: parent.horizontalCenter
                visible: root.showAbsence
                width: parent.width
                height: parent.height

                Column {
                    anchors.centerIn: parent
                    spacing: Theme.primitive.spacing.sm

                    TablerIcon {
                        objectName: "appsDrawerUnavailableGlyph"
                        anchors.horizontalCenter: parent.horizontalCenter
                        name: "app-window"
                        size: Theme.controls.appsDrawer.emptyGlyphSize
                        strokeWidth: 1.5
                        color: Theme.color.textTertiary
                    }

                    Text {
                        id: unavailableText
                        objectName: "appsDrawerUnavailableText"
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: qsTr("Application index unavailable")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeMd
                    }
                }
            }
        }

        // A slim overlay scrollbar: it tells a pointer user that there are
        // more rows below without stealing grid width or adding chrome to the
        // card. It appears only when the corpus overflows the five-row
        // viewport and brightens on hover.
        Rectangle {
            id: gridScrollTrack
            objectName: "appsDrawerScrollTrack"
            visible: gridFlick.contentHeight > gridFlick.height + 1
            anchors.top: gridFlick.top
            anchors.bottom: gridFlick.bottom
            anchors.left: gridFlick.right
            anchors.leftMargin: Theme.primitive.spacing.md
            width: 4
            radius: width / 2
            color: "transparent"

            Rectangle {
                width: parent.width
                radius: parent.radius
                color: Theme.color.textTertiary
                opacity: gridScrollHover.hovered ? 0.8 : 0.45
                height: Math.max(
                    Theme.controls.scrollView.minThumb,
                    gridFlick.height * gridFlick.height
                        / Math.max(1, gridFlick.contentHeight))
                y: {
                    var maxY = Math.max(1, gridFlick.contentHeight
                                           - gridFlick.height);
                    return (gridFlick.contentY / maxY)
                           * (parent.height - height);
                }

                Behavior on opacity {
                    enabled: !Theme.reducedMotion
                    NumberAnimation {
                        duration: Theme.motion.hover.duration
                        easing.type: Easing.Bezier
                        easing.bezierCurve: Theme.motion.hover.curve
                    }
                }
            }

            HoverHandler { id: gridScrollHover }
        }
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Escape) {
            root.dismissRequested();
            event.accepted = true;
        } else if (event.key === Qt.Key_Left) {
            root.moveSelection(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Right) {
            root.moveSelection(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Up) {
            root.moveSelection(-root.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_Down) {
            root.moveSelection(root.columns);
            event.accepted = true;
        } else if (event.key === Qt.Key_PageDown || event.key === Qt.Key_PageUp) {
            var page = root.columns
                    * Math.max(1, Math.floor(gridFlick.height
                                             / (root.tileHeight + root.gridRowGap)));
            root.moveSelection(event.key === Qt.Key_PageDown ? page : -page);
            event.accepted = true;
        } else if (event.key === Qt.Key_Home) {
            root.moveSelectionTo(0);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.moveSelectionTo(root.visibleApps.length - 1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            // When the search field owns focus its TextInput already emits
            // `accepted`; only handle Return for the root-level fallback, or
            // the drawer launches twice.
            if (!searchField.input.activeFocus)
                root.launchAt(root.currentIndex);
            event.accepted = true;
        } else if (event.key === Qt.Key_Backspace || event.key === Qt.Key_Delete) {
            // Defensive fallback: if focus ever leaves the field, edit the
            // query instead of inserting the control character the chrome
            // key bridge carries in `event.text`.
            root.focusSearch();
            var text = searchField.text;
            if (event.key === Qt.Key_Backspace && text.length > 0)
                searchField.text = text.slice(0, -1);
            else if (event.key === Qt.Key_Delete)
                searchField.text = "";
            event.accepted = true;
        } else if (event.text.length > 0
                   && event.text.charCodeAt(0) >= 0x20
                   && !(event.modifiers & (Qt.ControlModifier
                                           | Qt.AltModifier
                                           | Qt.MetaModifier))) {
            // Focus normally lives in the search field; this fallback keeps a
            // stray printable key from being dropped if a pointer interaction
            // momentarily moved it.
            root.focusSearch();
            searchField.text += event.text;
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Applications")
}
