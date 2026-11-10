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
    enabled: active
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
    // The More menu's manual refresh; the shell reloads the app-index corpus.
    signal refreshRequested()

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
        } else if (moreMenu) {
            moreMenu.hide()
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

    // The card wraps the grid (at most seven columns) with the measured
    // `panelPadding` on the sides and `panelPaddingV` above and below, floored
    // at the search field's width so a short list still reads as a card, and
    // clamped to the output.
    readonly property int maxColumns: 7
    readonly property real panelPadding: Theme.controls.appsDrawer.panelPadding
    readonly property real panelPaddingV:
        Theme.controls.appsDrawer.panelPaddingVertical
    // Measured Spotlight Apps cells: 115 pt wide, 77 pt icon, 18 pt label,
    // 102 pt row pitch. The label is a single line under the artwork.
    readonly property real tileWidth: Theme.controls.appsDrawer.tileWidth
    readonly property real tileHeight: Theme.controls.appsDrawer.tileHeight
    readonly property real iconSize: Theme.controls.appsDrawer.tileSize
    readonly property real labelHeight: Theme.controls.appsDrawer.labelHeight
    readonly property real gridGap: Theme.controls.appsDrawer.gridColumnGap
    readonly property real gridRowGap: Theme.controls.appsDrawer.gridRowGap
    readonly property real gridMaxWidth:
        maxColumns * root.tileWidth + (maxColumns - 1) * root.gridGap
    // The grid viewport shows at most five rows; a longer corpus scrolls.
    readonly property int maxRows: 5
    readonly property real gridMaxHeight:
        root.maxRows * root.tileHeight + (root.maxRows - 1) * root.gridRowGap
    // The search row is the measured card width: a Back button, the field, and
    // the More button share one line.
    readonly property real searchButtonSize:
        Theme.controls.appsDrawer.searchButtonSize
    readonly property real searchButtonGap:
        Theme.controls.appsDrawer.searchButtonGap
    readonly property real searchHeight: Theme.controls.appsDrawer.searchHeight
    readonly property real chipHeight: Theme.controls.appsDrawer.chipHeight
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

    // The card height: the search row, the category chips, the grid, and the
    // vertical padding, clamped to five grid rows and to the output so a long
    // corpus scrolls.
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
        root.panelPaddingV + root.searchHeight
        + Theme.primitive.spacing.lg + root.chipHeight
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

    // One wheel path for the whole card: the grid's Flickable handles the
    // column band, and the card-level handler below catches vertical wheels
    // over the search field, chips, and side gutters. Pixel deltas keep
    // touchpads smooth; a discrete wheel steps one row.
    function scrollGridByWheel(event) {
        // A discrete wheel notch arrives with both a small pixel delta and a
        // 120-unit angle delta from the shell. Use whichever is larger so one
        // notch steps a row instead of a few pixels, while a high-resolution
        // touchpad still uses its precise pixel delta.
        var pixel = event.pixelDelta.y;
        var angle = event.angleDelta.y / 120
                * (root.tileHeight + root.gridRowGap);
        var delta = Math.abs(angle) > Math.abs(pixel) ? angle : pixel;
        if (delta === 0)
            return;
        var maxY = Math.max(0, gridFlick.contentHeight - gridFlick.height);
        if (maxY <= 0)
            return;
        gridFlick.cancelFlick();
        gridFlick.contentY = Math.max(0, Math.min(maxY,
                                                  gridFlick.contentY - delta));
        event.accepted = true;
    }

    function openMoreMenu() {
        if (!moreButton)
            return;
        var p = moreButton.mapToItem(root, moreButton.width,
                                     moreButton.height
                                     + Theme.primitive.spacing.xs);
        moreMenu.showAt(Math.max(0, Math.min(root.width - moreMenu.width,
                                             p.x - moreMenu.width)),
                        p.y);
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
        // A full-output scrim would also darken the compositor backdrop under
        // the card; sampled mode already supplies its own readable material.
        visible: !root.materialBlurActive || Glass.rollout !== "replacement"
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: Theme.controls.appsDrawer.scrimOpacity * root.reveal
    }

    // Handle dismissal on press for every mouse button. Keeping this behind
    // the card lets its controls/Flickable own their gestures normally.
    MouseArea {
        objectName: "appsDrawerDismissArea"
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        onPressed: (mouse) => {
            var p = panel.mapFromItem(root, mouse.x, mouse.y);
            if (!panel.contains(p))
                root.dismissRequested();
            else
                mouse.accepted = false;
        }
    }

    // A soft floating shadow behind the card. The compositor supplies the
    // sampled backdrop; this keeps the card legible on the software/fallback
    // path and gives the translucent plate the reference's lifted edge.
    Shadow {
        anchors.fill: panel
        level: "overlay"
        radius: panel.radius
        opacity: root.reveal
    }

    Rectangle {
        id: panel
        objectName: "appsDrawerPanel"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        width: root.panelWidth
        height: Math.min(root.panelNaturalHeight, root.panelMaxHeight)
        radius: Theme.controls.appsDrawer.panelRadius
        // A translucent frosted fill: the compositor's liquid-glass material
        // (blur + edge lens + specular rim + adaptive tint, T-20.2) and the
        // scrim/desktop ghost through, while the children (search, pills,
        // tiles) stay opaque because the opacity is in the fill, not on the
        // item. Without the sampled pass it takes the milky complement.
        color: Material.drawerFill(root.materialBlurActive, root.materialTier,
                                   root.materialFallback)
        opacity: root.reveal
        scale: 1.0
        border.width: Theme.controls.window.borderWidth
        border.color: Glass.increaseContrast
                      ? Theme.color.textPrimary
                      : Qt.rgba(Theme.color.dockRim.r, Theme.color.dockRim.g,
                                Theme.color.dockRim.b, 0.45)
        antialiasing: true

        // A soft vertical light falloff: the top of the sheet catches a little
        // more light and the bottom sits in a slight shade, so the translucent
        // plate reads as glass rather than a flat tint.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            gradient: Gradient {
                GradientStop {
                    position: 0.0
                    color: Qt.rgba(Theme.color.dockRim.r, Theme.color.dockRim.g,
                                   Theme.color.dockRim.b, 0.12)
                }
                GradientStop {
                    position: 0.3
                    color: Qt.rgba(Theme.color.dockRim.r, Theme.color.dockRim.g,
                                   Theme.color.dockRim.b, 0.02)
                }
                GradientStop {
                    position: 1.0
                    color: Qt.rgba(Theme.color.shadowColor.r,
                                   Theme.color.shadowColor.g,
                                   Theme.color.shadowColor.b, 0.05)
                }
            }
        }

        // Consume clicks on the panel so a miss on a tile does not dismiss,
        // and return typing to the search field (a click on the pills or the
        // empty grid should not strand keyboard focus).
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.AllButtons
            onPressed: {
                if (moreMenu.open)
                    moreMenu.hide();
                root.focusSearch();
            }
        }

        // Vertical wheel over the search field, the chip row, or the card's
        // side gutters still scrolls the grid. The grid Flickable's own
        // handler accepts first when the pointer is over the column band.
        WheelHandler {
            target: null
            onWheel: (event) => {
                if (event.pixelDelta.y === 0 && event.angleDelta.y === 0)
                    return;
                root.scrollGridByWheel(event);
            }
        }

        // The measured Spotlight Apps top row: a Back button, the large
        // rounded search field, and the More button.
        Item {
            id: searchRow
            objectName: "appsDrawerSearchRow"
            anchors.top: parent.top
            anchors.topMargin: root.panelPaddingV
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.panelWidth - 2 * root.panelPadding
            height: Math.max(root.searchHeight, root.searchButtonSize)

            DrawerIconButton {
                id: backButton
                objectName: "appsDrawerBackButton"
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                icon: "chevron-left"
                accessibleName: qsTr("Close Applications")
                onClicked: root.dismissRequested()
            }

            DrawerIconButton {
                id: moreButton
                objectName: "appsDrawerMoreButton"
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                icon: "adjustments-horizontal"
                accessibleName: qsTr("Applications options")
                onClicked: {
                    if (moreMenu.open)
                        moreMenu.hide();
                    else
                        root.openMoreMenu();
                }
            }

            SearchField {
                id: searchField
                objectName: "appsDrawerSearch"
                anchors.left: backButton.right
                anchors.leftMargin: root.searchButtonGap
                anchors.right: moreButton.left
                anchors.rightMargin: root.searchButtonGap
                anchors.verticalCenter: parent.verticalCenter
                height: root.searchHeight
                fieldHeight: root.searchHeight
                fieldRadius: Theme.controls.appsDrawer.searchRadius
                fieldFill: Qt.rgba(Theme.color.surface.r, Theme.color.surface.g,
                                   Theme.color.surface.b,
                                   Theme.controls.appsDrawer.searchFillOpacity)
                fieldBorder: searchField.activeFocus
                             ? Qt.rgba(Theme.color.focusRing.r,
                                       Theme.color.focusRing.g,
                                       Theme.color.focusRing.b, 0.75)
                             : Qt.rgba(Theme.color.border.r,
                                       Theme.color.border.g,
                                       Theme.color.border.b,
                                       Theme.controls.appsDrawer.searchBorderOpacity)
                iconSize: Theme.controls.appsDrawer.searchIconSize
                paddingH: Theme.controls.appsDrawer.searchPaddingH
                fontSize: Theme.primitive.font.sizeLg
                placeholderText: qsTr("Applications")
                onTextChanged: root.query = text
                // Return launches the selected tile from the text field; Escape
                // dismisses the launcher outright (Launchpad behaviour), and
                // the next open starts with an empty search.
                onAccepted: root.launchAt(root.currentIndex)
                onEscapePressed: {
                    if (moreMenu.open) {
                        moreMenu.hide();
                        root.focusSearch();
                    } else {
                        root.dismissRequested();
                    }
                }
                // The text field owns focus while the drawer is open. Forward
                // the grid-navigation keys here so typing filters and arrows
                // still move the tile selection.
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
        }

        // The category chips: individual translucent capsules, not a segmented
        // group. Horizontally scrollable so a narrow output never clips one.
        Flickable {
            id: pillScroller
            objectName: "appsDrawerPills"
            anchors.top: searchRow.bottom
            anchors.topMargin: Theme.primitive.spacing.lg
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.panelWidth - 2 * root.panelPadding
            height: root.chipHeight
            contentWidth: pillsRow.width
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            flickableDirection: Flickable.HorizontalFlick
            interactive: contentWidth > width

            // Only a horizontal wheel (or a shift-wheel) scrolls the chips. A
            // plain vertical wheel is left to the card-level handler so it
            // scrolls the grid from anywhere over the card.
            WheelHandler {
                target: null
                onWheel: (event) => {
                    const delta = event.pixelDelta.x || event.angleDelta.x;
                    if (delta !== 0) {
                        const maxX = Math.max(0, pillScroller.contentWidth
                                                 - pillScroller.width);
                        pillScroller.contentX = Math.max(0, Math.min(
                            maxX, pillScroller.contentX - delta));
                        event.accepted = true;
                        return;
                    }
                    // A plain wheel has only the vertical axis. Forward it to
                    // the grid instead of swallowing it on the chip strip.
                    if (event.pixelDelta.y !== 0 || event.angleDelta.y !== 0)
                        root.scrollGridByWheel(event);
                }
            }

            Row {
                id: pillsRow
                objectName: "appsDrawerCategoryControl"
                spacing: Theme.controls.appsDrawer.pillGap

                Repeater {
                    model: root.pills

                    delegate: Item {
                        id: chip
                        required property var modelData
                        required property int index

                        readonly property bool current:
                            root.category === chip.modelData.key
                        readonly property bool hovered: chipHover.hovered
                        width: chipLabel.implicitWidth
                               + 2 * Theme.controls.appsDrawer.chipPaddingH
                        height: root.chipHeight
                        activeFocusOnTab: true

                        function activate() {
                            root.category = chip.modelData.key;
                        }

                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.controls.appsDrawer.chipRadius
                            color: chip.current
                                ? Qt.rgba(Theme.color.textPrimary.r,
                                          Theme.color.textPrimary.g,
                                          Theme.color.textPrimary.b,
                                          Theme.controls.appsDrawer.chipSelectedOpacity)
                                : Qt.rgba(Theme.color.textPrimary.r,
                                          Theme.color.textPrimary.g,
                                          Theme.color.textPrimary.b,
                                          chip.hovered
                                          ? Theme.controls.appsDrawer.chipHoverOpacity
                                          : Theme.controls.appsDrawer.chipFillOpacity)
                            border.width: chip.current
                                          ? 0 : Theme.controls.window.borderWidth
                            border.color: Qt.rgba(Theme.color.textPrimary.r,
                                                  Theme.color.textPrimary.g,
                                                  Theme.color.textPrimary.b, 0.18)
                            antialiasing: true

                            Behavior on color {
                                enabled: !Theme.reducedMotion
                                ColorAnimation {
                                    duration: Theme.motion.hover.duration
                                    easing.type: Easing.Bezier
                                    easing.bezierCurve: Theme.motion.hover.curve
                                }
                            }
                        }

                        Text {
                            id: chipLabel
                            anchors.centerIn: parent
                            text: chip.modelData.label
                            color: chip.current ? Theme.color.surface
                                                : Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.weight: chip.current
                                         ? Theme.primitive.font.weightMedium
                                         : Theme.primitive.font.weightRegular
                        }

                        HoverHandler { id: chipHover }
                        TapHandler {
                            onTapped: chip.activate()
                        }
                        FocusRing {
                            target: chip
                            cornerRadius: Theme.controls.appsDrawer.chipRadius
                            shown: chip.activeFocus
                        }
                        Keys.onPressed: (event) => {
                            if (event.key === Qt.Key_Left) {
                                var left = Math.max(0, root.categoryIndex - 1);
                                root.category = root.pills[left].key;
                                event.accepted = true;
                            } else if (event.key === Qt.Key_Right) {
                                var right = Math.min(root.pills.length - 1,
                                                     root.categoryIndex + 1);
                                root.category = root.pills[right].key;
                                event.accepted = true;
                            } else if (event.key === Qt.Key_Space
                                       || event.key === Qt.Key_Return
                                       || event.key === Qt.Key_Enter) {
                                chip.activate();
                                event.accepted = true;
                            }
                        }

                        Accessible.role: Accessible.RadioButton
                        Accessible.name: chip.modelData.label
                        Accessible.checkable: true
                        Accessible.checked: chip.current
                        Accessible.focusable: true
                        Accessible.onPressAction: chip.activate()
                    }
                }
            }
        }

        // The tile grid. The Flickable spans the whole card so a wheel over
        // the grid's side gutters scrolls too; the Grid itself stays at the
        // measured width and is centred.
        Flickable {
            id: gridFlick
            objectName: "appsDrawerGrid"
            anchors.top: pillScroller.bottom
            anchors.topMargin: Theme.primitive.spacing.xl
            anchors.bottom: parent.bottom
            anchors.bottomMargin: root.panelPaddingV
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width
            contentWidth: width
            contentHeight: grid.height
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            flickableDirection: Flickable.VerticalFlick
            interactive: contentHeight > height

            WheelHandler {
                target: null
                onWheel: (event) => root.scrollGridByWheel(event)
            }

            Grid {
                id: grid
                objectName: "appsDrawerTiles"
                width: root.gridWidth
                anchors.horizontalCenter: parent.horizontalCenter
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
                        readonly property real iconSize: root.iconSize
                        readonly property real highlightSize:
                            tile.iconSize + Theme.primitive.spacing.sm

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
                            y: (tile.iconSize - tile.highlightSize) / 2
                            width: tile.highlightSize
                            height: tile.highlightSize
                            radius: Theme.controls.appsDrawer.tileRadius
                            color: tile.pressed
                                   ? Qt.rgba(Theme.color.textPrimary.r,
                                             Theme.color.textPrimary.g,
                                             Theme.color.textPrimary.b,
                                             Theme.controls.appsDrawer.iconSelectedOpacity
                                             + 0.1)
                                   : (tile.selected
                                      ? Qt.rgba(Theme.color.textPrimary.r,
                                                Theme.color.textPrimary.g,
                                                Theme.color.textPrimary.b,
                                                Theme.controls.appsDrawer.iconSelectedOpacity)
                                      : (tile.hovered
                                         ? Qt.rgba(Theme.color.textPrimary.r,
                                                   Theme.color.textPrimary.g,
                                                   Theme.color.textPrimary.b,
                                                   Theme.controls.appsDrawer.iconHighlightOpacity)
                                         : "transparent"))
                            border.width: tile.selected
                                          ? Theme.controls.window.borderWidth : 0
                            border.color: Theme.color.accent
                            antialiasing: true
                        }

                        // The themed app-index icon, rendered at the measured
                        // 77 pt artwork size.
                        Image {
                            id: appIcon
                            objectName: "appsDrawerTileIcon"
                            visible: tile.iconUrl.length > 0 && status === Image.Ready
                            source: tile.iconUrl
                            sourceSize.width: Theme.controls.appsDrawer.iconSourceSize
                            sourceSize.height: Theme.controls.appsDrawer.iconSourceSize
                            width: tile.iconSize
                            height: tile.iconSize
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: 0
                            fillMode: Image.PreserveAspectFit
                            asynchronous: true
                            smooth: true
                        }

                        // The fallback is a rounded app tile, not a bare glyph:
                        // while the themed icon loads, when app-index has none,
                        // or when it failed to load, the drawer still reads as
                        // a grid of app artwork.
                        Rectangle {
                            id: fallbackTile
                            objectName: "appsDrawerTileFallback"
                            visible: tile.iconUrl.length === 0
                                     || appIcon.status !== Image.Ready
                            width: tile.iconSize
                            height: tile.iconSize
                            anchors.horizontalCenter: parent.horizontalCenter
                            y: 0
                            radius: Theme.controls.appsDrawer.tileRadius
                            color: Qt.rgba(Theme.color.textPrimary.r,
                                           Theme.color.textPrimary.g,
                                           Theme.color.textPrimary.b,
                                           Theme.controls.appsDrawer.fallbackFillOpacity)
                            border.width: Theme.controls.window.borderWidth
                            border.color: Qt.rgba(Theme.color.textPrimary.r,
                                                  Theme.color.textPrimary.g,
                                                  Theme.color.textPrimary.b,
                                                  Theme.controls.appsDrawer.fallbackBorderOpacity)
                            antialiasing: true

                            TablerIcon {
                                anchors.centerIn: parent
                                name: "app-window"
                                size: tile.iconSize
                                         * Theme.controls.appsDrawer.fallbackGlyphRatio
                                strokeWidth: 1.5
                                color: Theme.color.textSecondary
                            }
                        }

                        Text {
                            objectName: "appsDrawerTileLabel"
                            width: parent.width
                            height: root.labelHeight
                            y: tile.iconSize
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
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
            x: grid.x + grid.width + Theme.primitive.spacing.md
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

    // A click anywhere outside the open More menu closes it without swallowing
    // the click that the underlying control wants.
    MouseArea {
        visible: moreMenu.open
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        z: 1500
        onPressed: {
            moreMenu.hide();
            mouse.accepted = false;
        }
    }

    ContextMenu {
        id: moreMenu
        objectName: "appsDrawerMoreMenu"
        model: [
            { label: qsTr("Refresh Applications") },
            { type: "separator" },
            { label: qsTr("Close") }
        ]
        materialPreset: "menuDetached"
        materialBackdrop: "local"
        materialBlurActive: root.materialBlurActive
        materialTier: root.materialTier
        materialFallback: root.materialFallback
        accessibleName: qsTr("Applications options")
        onTriggered: (index, item) => {
            if (index === 0)
                root.refreshRequested();
            else if (index === 2)
                root.dismissRequested();
        }
        onClosed: root.focusSearch()
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Escape) {
            if (moreMenu.open) {
                moreMenu.hide();
                root.focusSearch();
            } else {
                root.dismissRequested();
            }
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

    // The measured 34 pt circular chrome button used by the drawer's Back and
    // More affordances. It is a translucent control on the card rather than a
    // solid toolbar button, so the glass reads through it.
    component DrawerIconButton: Item {
        id: button

        property string icon: ""
        property string accessibleName: ""
        readonly property bool hovered: buttonHover.hovered

        signal clicked()

        implicitWidth: Theme.controls.appsDrawer.searchButtonSize
        implicitHeight: Theme.controls.appsDrawer.searchButtonSize
        width: implicitWidth
        height: implicitHeight
        activeFocusOnTab: true

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.appsDrawer.searchButtonRadius
            color: Qt.rgba(Theme.color.textPrimary.r, Theme.color.textPrimary.g,
                           Theme.color.textPrimary.b,
                           button.hovered
                           ? Theme.controls.appsDrawer.searchButtonHoverOpacity
                           : Theme.controls.appsDrawer.searchButtonFillOpacity)
            border.width: Theme.controls.window.borderWidth
            border.color: Qt.rgba(Theme.color.textPrimary.r,
                                  Theme.color.textPrimary.g,
                                  Theme.color.textPrimary.b, 0.14)
            antialiasing: true

            Behavior on color {
                enabled: !Theme.reducedMotion
                ColorAnimation {
                    duration: Theme.motion.hover.duration
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.motion.hover.curve
                }
            }
        }

        TablerIcon {
            anchors.centerIn: parent
            name: button.icon
            size: Theme.controls.appsDrawer.searchButtonIconSize
            color: Theme.color.textSecondary
        }

        HoverHandler { id: buttonHover }
        TapHandler {
            onTapped: button.clicked()
        }
        FocusRing {
            target: button
            cornerRadius: Theme.controls.appsDrawer.searchButtonRadius
            shown: button.activeFocus
        }
        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                button.clicked();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.Button
        Accessible.name: button.accessibleName
        Accessible.focusable: true
        Accessible.onPressAction: button.clicked()
    }
}
