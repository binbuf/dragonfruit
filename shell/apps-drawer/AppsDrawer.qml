// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Applications drawer (T-19.2): the shell's primary "start an app"
// affordance. A full-screen overlay listing every launchable app the app-index
// corpus knows about, alphabetical by default (`buildAppsDrawerList` sorts),
// with a search field and category filter pills, and a fixed-size tile grid
// whose column count follows the available width.
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

    // The grid column count follows the available width; the tile size stays
    // fixed so narrow outputs wrap instead of overflowing.
    readonly property real tileWidth: Theme.controls.appsDrawer.tileSize
                                      + Theme.primitive.spacing.xl
    readonly property real gridGap: Theme.controls.appsDrawer.gridColumnGap
    readonly property int columns: Math.max(1, Math.floor(
        (root.width - 2 * Theme.controls.appsDrawer.contentPadding + root.gridGap)
        / (root.tileWidth + root.gridGap)))

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
        anchors.fill: parent
        anchors.margins: Theme.controls.appsDrawer.contentPadding / 2
        radius: Theme.primitive.radius.xl
        color: Theme.color.surface
        opacity: Theme.controls.appsDrawer.panelOpacity * root.reveal
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        // Consume clicks on the panel so a miss on a tile does not dismiss.
        TapHandler { }

        Item {
            id: header
            objectName: "appsDrawerHeader"
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.topMargin: Theme.controls.appsDrawer.headerTop
            anchors.leftMargin: Theme.controls.appsDrawer.contentPadding / 2
            anchors.rightMargin: Theme.controls.appsDrawer.contentPadding / 2
            height: titleRow.height

            Row {
                id: titleRow
                anchors.left: parent.left
                spacing: Theme.primitive.spacing.sm

                PhosphorIcon {
                    objectName: "appsDrawerTitleGlyph"
                    name: "squares-four"
                    weight: "fill"
                    size: Theme.controls.appsDrawer.titleSize
                    color: Theme.color.textPrimary
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    objectName: "appsDrawerTitle"
                    text: qsTr("Applications")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.appsDrawer.titleSize
                    font.weight: Font.DemiBold
                    anchors.verticalCenter: parent.verticalCenter
                }
            }
        }

        SearchField {
            id: searchField
            objectName: "appsDrawerSearch"
            anchors.top: header.bottom
            anchors.topMargin: Theme.primitive.spacing.xl
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(Theme.controls.appsDrawer.searchMaxWidth,
                            root.width - 2 * Theme.controls.appsDrawer.contentPadding)
            placeholderText: qsTr("Search applications")
            onTextChanged: root.query = text
        }

        // The category pills. Horizontally scrollable so a narrow output never
        // clips a pill; a SegmentedControl owns the selection semantics.
        Flickable {
            id: pillScroller
            objectName: "appsDrawerPills"
            anchors.top: searchField.bottom
            anchors.topMargin: Theme.primitive.spacing.lg
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(Theme.controls.appsDrawer.searchMaxWidth,
                            root.width - 2 * Theme.controls.appsDrawer.contentPadding)
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

        // The tile grid. A `Flow` keeps the tile size fixed and wraps at the
        // available width.
        Flickable {
            id: gridFlick
            objectName: "appsDrawerGrid"
            anchors.top: pillScroller.bottom
            anchors.topMargin: Theme.primitive.spacing.xl
            anchors.bottom: parent.bottom
            anchors.bottomMargin: Theme.controls.appsDrawer.contentPadding / 2
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(Theme.controls.appsDrawer.searchMaxWidth * 2,
                            root.width - 2 * Theme.controls.appsDrawer.contentPadding)
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
                        height: root.tileWidth + Math.ceil(
                            Theme.controls.appsDrawer.labelSize * 2.2)
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
                y: Theme.controls.appsDrawer.headerTop
                visible: root.noMatches
                text: qsTr("No applications found")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeMd
            }

            // app-index absent: the explicit absence row (ADR 0167).
            Item {
                objectName: "appsDrawerUnavailable"
                anchors.horizontalCenter: parent.horizontalCenter
                y: Theme.controls.appsDrawer.headerTop
                visible: root.showAbsence
                width: parent.width
                height: unavailableText.implicitHeight

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