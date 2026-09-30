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
    onActiveChanged: root.shown = root.active ? 1.0 : 0.0
    Behavior on shown {
        enabled: !Theme.reducedMotion
        NumberAnimation { duration: 140; easing.type: Easing.OutCubic }
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
    readonly property real gridMaxWidth:
        maxColumns * root.tileWidth + (maxColumns - 1) * root.gridGap
    // The grid viewport shows at most five rows; a longer corpus scrolls.
    readonly property int maxRows: 5
    readonly property real gridMaxHeight:
        root.maxRows * root.tileHeight + (root.maxRows - 1) * root.gridGap
    readonly property real panelWidth: Math.min(
        Math.max(root.gridMaxWidth, Theme.controls.appsDrawer.searchMaxWidth)
            + 2 * root.panelPadding,
        root.width - 2 * Theme.controls.appsDrawer.contentPadding / 2)
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

    // The card height: the top search bar, the two hairline rules, pills, the
    // grid, and the vertical padding, clamped to five grid rows and to the
    // output so a long corpus scrolls.
    readonly property real gridContentHeight:
        (root.showAbsence || root.noMatches)
            ? Theme.controls.appsDrawer.emptyGlyphSize
              + Theme.primitive.spacing.xxl
            : grid.height
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
        root.height - 2 * Theme.controls.appsDrawer.contentPadding / 2

    function setQueryText(text) { searchField.text = text; }

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
        var p = panel.mapToItem(null, 0, 0);
        return { x: p.x, y: p.y, width: panel.width, height: panel.height };
    }

    function launchAt(index) {
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
        if (count === 0)
            return;
        if (root.currentIndex < 0)
            root.currentIndex = delta > 0 ? 0 : count - 1;
        else
            root.currentIndex = Math.max(0, Math.min(count - 1, root.currentIndex + delta));
    }

    onVisibleAppsChanged: {
        if (root.visibleApps.length === 0)
            root.currentIndex = -1;
        else if (root.currentIndex < 0)
            root.currentIndex = 0;
        else if (root.currentIndex >= root.visibleApps.length)
            root.currentIndex = root.visibleApps.length - 1;
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
        // A ~90%-opaque frosted fill: the scrim/desktop ghosts through, while
        // the children (search, pills, tiles) stay fully opaque because the
        // opacity is in the fill, not on the item.
        color: Qt.rgba(Theme.color.surface.r, Theme.color.surface.g,
                       Theme.color.surface.b,
                       Theme.controls.appsDrawer.panelOpacity)
        opacity: root.reveal
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        // Consume clicks on the panel so a miss on a tile does not dismiss.
        TapHandler { }

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
            width: Math.min(Theme.controls.appsDrawer.searchMaxWidth,
                            root.panelWidth - 2 * root.panelPadding)
            height: pillsControl.height
            contentWidth: pillsControl.width
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            interactive: contentWidth > width

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

        // The tile grid. A `Flow` keeps the tile size fixed and wraps at the
        // available width.
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

            Flow {
                id: grid
                objectName: "appsDrawerTiles"
                width: parent.width
                spacing: root.gridGap
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
                        readonly property bool selected: tile.index === root.currentIndex
                        readonly property bool hovered: tileHover.hovered

                        width: root.tileWidth
                        height: root.tileHeight
                        focus: tile.selected
                        activeFocusOnTab: true

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
                            color: tile.selected ? Theme.color.accentMuted
                                 : (tile.hovered ? Theme.color.controlFill : "transparent")
                            border.width: tile.selected ? Theme.controls.window.borderWidth : 0
                            border.color: Theme.color.accent
                        }

                        // The themed app-index icon, rendered at the tile size.
                        Image {
                            id: appIcon
                            objectName: "appsDrawerTileIcon"
                            visible: tile.iconPath.length > 0 && status === Image.Ready
                            source: tile.iconPath.length > 0 ? "file://" + tile.iconPath : ""
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

                        // The Phosphor fallback when app-index has no themed
                        // icon (or it failed to load).
                        PhosphorIcon {
                            objectName: "appsDrawerTileFallback"
                            visible: tile.iconPath.length === 0 || appIcon.status === Image.Error
                                     || appIcon.status === Image.Null
                            name: "app-window"
                            weight: "fill"
                            size: Theme.controls.appsDrawer.tileSize
                                     * Theme.controls.appsDrawer.fallbackGlyphRatio
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
                            color: tile.selected ? Theme.color.accentContent
                                                 : Theme.color.textPrimary
                            font.pixelSize: Theme.controls.appsDrawer.labelSize
                        }

                        HoverHandler { id: tileHover }
                        TapHandler {
                            acceptedButtons: Qt.LeftButton
                            onTapped: tile.activate()
                        }
                        Keys.onReturnPressed: tile.activate()
                        Keys.onEnterPressed: tile.activate()

                        Accessible.role: Accessible.ListItem
                        Accessible.name: tile.name
                        Accessible.focusable: true
                        Accessible.selected: tile.selected
                        Accessible.onPressAction: tile.activate()
                    }
                }
            }

            // The empty-filter state: an explicit row, never a blank panel.
            Text {
                objectName: "appsDrawerEmpty"
                anchors.horizontalCenter: parent.horizontalCenter
                y: Math.max(0, (parent.height - height) / 2)
                visible: root.noMatches
                text: qsTr("No applications found")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeMd
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

                    PhosphorIcon {
                        objectName: "appsDrawerUnavailableGlyph"
                        anchors.horizontalCenter: parent.horizontalCenter
                        name: "app-window"
                        size: Theme.controls.appsDrawer.emptyGlyphSize
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
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.launchAt(root.currentIndex);
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Applications")
}