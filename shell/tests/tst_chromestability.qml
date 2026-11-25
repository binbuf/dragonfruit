// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.MenuBar
import Dragonfruit.Dock

// Chrome stability fixtures (T349).
//
// This suite is the deterministic, no-native-seat half of the chrome gate.
// Every case names one invariant the popup/transport work must hold:
//
//   * status anchors (Wi-Fi, Volume, Battery, Bluetooth, clock)
//   * the real Dock chooser card and arrow footprint
//   * attached/detached menu silhouettes on a patterned underlay
//   * cold/warm open, close, switch, and reopen for both popup families
//
// The four status-anchor cases and the chooser crop case are pre-fix
// reproducers: they fail on the starting revision and are listed in
// `tests/chrome-stability/expected-failures.json` with their owning task.
// `tests/chrome-stability/run_fixtures.py` is the gate; it requires each
// expected-failure row to still fail until its owner flips it to `pass`.
//
// `grabImage()` runs on the software scene graph, so the pixel cases check the
// QML silhouette only. They are not compositor import or native scanout
// evidence; the GPU smoke rail and the nested captures carry those classes.
Item {
    id: stage
    width: 1000
    height: 640

    TestCase {
        id: testCase
        name: "ChromeStability"
        when: windowShown

        Component { id: menuBarComponent; MenuBar { } }
        Component { id: dockComponent; Dock { } }
        Component {
            id: popupComponent
            Popup {
                Column {
                    Rectangle {
                        width: 100
                        height: 60
                        color: "#ffffff"
                    }
                }
            }
        }

        function init() {
            Glass.rollout = "legacy";
            Theme.reducedMotion = false;
            Theme.dark = false;
        }

        function make(component, props) {
            var obj = createTemporaryObject(component, stage, props || {});
            waitForRendering(stage);
            return obj;
        }

        function saveShot(item, name) {
            if (typeof ChromeStabilityEnvironment === "undefined"
                    || ChromeStabilityEnvironment.captureDir.length === 0)
                return;
            grabImage(item).save(ChromeStabilityEnvironment.captureDir
                                 + "/" + name);
        }

        // --- Deterministic status/menu models -----------------------------

        function statusItems(withBluetooth) {
            return [
                { id: "wifi", icon: "wifi", accessibleName: "Wi-Fi",
                  available: true },
                { id: "bluetooth", icon: "bluetooth",
                  accessibleName: "Bluetooth",
                  available: withBluetooth === true },
                { id: "volume", icon: "volume", accessibleName: "Volume",
                  available: true },
                { id: "battery", icon: "battery", accessibleName: "Battery",
                  available: true }
            ];
        }

        function wifiModel() {
            return {
                kind: "wifi", state: "available", glyph: "wifi-secure",
                label: "home \u00b7 82%", readOnly: false,
                networks: [
                    { ssid: "home", strength: 82, security: "WPA2",
                      secured: true, active: true, band: "5 GHz" },
                    { ssid: "cafe", strength: 40, security: "Open",
                      secured: false, active: false, band: "2.4 GHz" }
                ]
            };
        }

        function audioModel() {
            return {
                kind: "audio", state: "available", glyph: "volume",
                volume: 0.5, percent: 50, muted: false,
                sinks: [ { id: 7, name: "speakers",
                           description: "Built-in Speakers", volume: 0.5,
                           percent: 50, muted: false, default: true } ]
            };
        }

        function batteryModel() {
            return {
                kind: "battery", state: "available", present: true,
                glyph: "battery", label: "82% charging", percent: 82,
                level: 0.82, charging: true, plugged: true, onBattery: false,
                timeToEmpty: null, timeToFull: 5400
            };
        }

        function bluetoothModel() {
            return {
                state: "available", present: true, powered: true,
                discovering: false, readOnly: false, label: "Bluetooth on",
                knownDevices: [
                    { address: "AA:BB:CC:DD:EE:FF", name: "Headset",
                      connected: true }
                ]
            };
        }

        function makeBar(props) {
            var base = {
                width: 800,
                statusItems: statusItems(false),
                wifiMenu: wifiModel(),
                volumeMenu: audioModel(),
                batteryMenu: batteryModel(),
                bluetoothMenu: bluetoothModel()
            };
            for (var key in (props || {}))
                base[key] = props[key];
            return make(menuBarComponent, base);
        }

        function multiWindow(id, name, windowList) {
            return { id: id, appId: id, name: name, kind: "pinned",
                     pinned: true, running: true, desktopId: id + ".desktop",
                     windows: windowList.length, windowList: windowList };
        }

        function windowList(prefix, count) {
            var out = [];
            for (var i = 0; i < count; ++i)
                out.push({ windowId: prefix + i, title: prefix + " " + i,
                           focused: i === 0, workspaceName: "Space 1" });
            return out;
        }

        // --- Status anchors ------------------------------------------------

        // The committed origin is the popup's own position mapped into the bar
        // and the controller's `dropdownRect`. A nonzero width is not enough:
        // the starting revision opens the four status menus at x=0 because
        // their root Item has no width to clamp against.
        function verifyStatusAnchor(id, bar) {
            var menuObjectName = id === "wifi" ? "wifiMenu"
                : id === "volume" ? "volumeMenu"
                : id === "battery" ? "batteryMenu"
                : id === "bluetooth" ? "bluetoothMenu"
                : "dateTimePanel";
            bar.openStatusMenu(id);
            waitForRendering(stage);
            compare(bar.openStatusItem, id, id + " opens");
            var host = findChild(bar, menuObjectName);
            verify(host !== null, menuObjectName + " exists");
            var popup = host.popup;
            verify(popup.open, id + " popup is open");
            verify(popup.width > 0, id + " popup has a positive width");

            var anchor = bar.statusItemFor(id);
            verify(anchor !== null, id + " status item exists");
            var anchorPoint = anchor.mapToItem(bar, 0, anchor.height);
            var expectedX = Math.max(0,
                Math.min(bar.width - popup.width, anchorPoint.x));
            var expectedY = anchorPoint.y + popup.anchorOffset;

            compare(popup.x, expectedX, id + " popup x follows the status item");
            compare(popup.y, expectedY, id + " popup y follows the status item");
            compare(bar.dropdownX, expectedX,
                    id + " committed target x follows the status item");
            compare(bar.dropdownY, expectedY,
                    id + " committed target y follows the status item");
            verify(bar.dropdownWidth > 0, id + " committed target has width");
            saveShot(stage, "t349-status-" + id + ".png");
            bar.closeStatusMenu(true);
        }

        function test_status_anchor_wifi() {
            verifyStatusAnchor("wifi", makeBar({}));
        }

        function test_status_anchor_volume() {
            verifyStatusAnchor("volume", makeBar({}));
        }

        function test_status_anchor_battery() {
            verifyStatusAnchor("battery", makeBar({}));
        }

        function test_status_anchor_bluetooth() {
            verifyStatusAnchor("bluetooth",
                               makeBar({ statusItems: statusItems(true) }));
        }

        // The clock is the working control: matching its box makes the shared
        // Popup clamp use the bar width. T350 must keep this behaviour while
        // fixing the four status hosts.
        function test_status_anchor_clock() {
            var bar = makeBar({ statusItems: [], showDate: true });
            var clock = bar.statusItemFor("clock");
            verify(clock !== null);
            mouseClick(clock, clock.width / 2, clock.height / 2);
            waitForRendering(stage);
            compare(bar.openStatusItem, "clock");
            var panel = findChild(bar, "dateTimePanel");
            verify(panel !== null);
            var popup = panel.popup;
            verify(popup.open);
            var anchorPoint = clock.mapToItem(bar, 0, clock.height);
            var expectedX = Math.max(0,
                Math.min(bar.width - popup.width, anchorPoint.x));
            compare(popup.x, expectedX, "clock popup x stays on the clock");
            compare(bar.dropdownX, expectedX,
                    "clock committed target x stays on the clock");
            bar.closeStatusMenu(true);
        }

        // Right-edge clamp: a status item near the right edge must place the
        // popup inside the bar, not at x=0 and not off the right edge.
        function test_status_anchor_right_edge_clamp() {
            var bar = makeBar({ width: 360, statusItems: statusItems(true) });
            bar.openStatusMenu("battery");
            waitForRendering(stage);
            var host = findChild(bar, "batteryMenu");
            var popup = host.popup;
            verify(popup.width <= bar.width);
            compare(popup.x, bar.width - popup.width,
                    "the right-edge popup clamps to the bar");
            bar.closeStatusMenu(true);
        }

        // --- Dock chooser footprint ----------------------------------------

        function chooserArrowBounds(dock, chooser) {
            var arrow = findChild(chooser, "chooserArrow");
            verify(arrow !== null, "chooser arrow exists");
            var size = chooser.arrowSize;
            var halfDiagonal = size * Math.SQRT2 / 2;
            var left = chooser.x + arrow.x + size / 2 - halfDiagonal;
            var top = chooser.y + arrow.y + size / 2 - halfDiagonal;
            return { x: left, y: top, w: halfDiagonal * 2,
                     h: halfDiagonal * 2 };
        }

        function test_chooser_arrow_is_inside_the_committed_crop() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", windowList("w", 2)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            verify(chooser !== null && chooser.open);
            var arrow = chooserArrowBounds(dock, chooser);
            var rect = dock.popoverRect;
            verify(rect.w > 0 && rect.h > 0, "the chooser commits a crop");
            verify(rect.x <= arrow.x + 0.01,
                   "the crop includes the arrow left edge");
            verify(rect.y <= arrow.y + 0.01,
                   "the crop includes the arrow top edge");
            verify(rect.x + rect.w + 0.01 >= arrow.x + arrow.w,
                   "the crop includes the arrow right edge");
            verify(rect.y + rect.h + 0.01 >= arrow.y + arrow.h,
                   "the crop includes the rotated arrow tip");
            saveShot(stage, "t349-chooser-arrow.png");
            dock.closePopovers();
        }

        // The material descriptor is the card silhouette, not the bounding
        // box: the empty bands beside the arrow must stay transparent.
        function test_chooser_material_region_excludes_the_arrow_gaps() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", windowList("w", 2)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            var regions = dock.activePopoverSurfaceRegions;
            compare(regions.length, 1, "the chooser has one material card");
            var rect = dock.popoverRect;
            var cardBottom = chooser.y - rect.y + chooser.height
                             - chooser.arrowSize / 2;
            verify(regions[0].y + regions[0].h <= cardBottom + 0.5,
                   "the material region stops at the card, not the arrow gap");
            dock.closePopovers();
        }

        // --- Menu silhouettes on a patterned underlay -----------------------

        // A checkerboard makes transparent corners observable: a square
        // attached top edge paints over it, while a detached arc leaves it.
        Component {
            id: checkerItemComponent
            Item {
                id: checker
                property int cellSize: 16
                property int cols: Math.max(1, Math.ceil(width / cellSize))
                property int rows: Math.max(1, Math.ceil(height / cellSize))
                Repeater {
                    model: checker.cols * checker.rows
                    delegate: Rectangle {
                        required property int index
                        readonly property int col: index % checker.cols
                        readonly property int row: Math.floor(index / checker.cols)
                        x: col * checker.cellSize
                        y: row * checker.cellSize
                        width: checker.cellSize
                        height: checker.cellSize
                        color: (row + col) % 2 === 0 ? "#ff00ff" : "#00ffff"
                    }
                }
            }
        }

        function checkerColor(x, y) {
            var cell = 16;
            var col = Math.floor(x / cell);
            var row = Math.floor(y / cell);
            return (row + col) % 2 === 0 ? "#ff00ff" : "#00ffff";
        }

        function nearColor(image, x, y, color, tolerance) {
            var actual = image.pixel(x, y);
            var expected = Qt.color(color);
            var tol = tolerance === undefined ? 0.06 : tolerance;
            return Math.abs(actual.r - expected.r) <= tol
                && Math.abs(actual.g - expected.g) <= tol
                && Math.abs(actual.b - expected.b) <= tol;
        }

        function makeMenuOnChecker(preset) {
            var under = createTemporaryObject(checkerItemComponent, stage,
                { x: 0, y: 0, width: stage.width, height: stage.height });
            var menu = createTemporaryObject(popupComponent, stage, {
                x: 300, y: 200, open: true,
                preferredWidth: 140,
                materialPreset: preset, materialBackdrop: "local",
                materialBlurActive: false
            });
            // Isolate the card silhouette. The QML drop shadow is a separate
            // paint extent and its crop is the T351/T352 defect; this fixture
            // asserts the rounded body, not the shadow reach.
            for (var i = 0; i < menu.children.length; ++i) {
                if (menu.children[i].cutoutBounds !== undefined)
                    menu.children[i].visible = false;
            }
            waitForRendering(stage);
            wait(40);
            return { under: under, menu: menu };
        }

        function test_attached_menu_has_square_top_corners_on_a_patterned_underlay() {
            var fixture = makeMenuOnChecker("menuAttached");
            var menu = fixture.menu;
            var image = grabImage(stage);
            var topLeft = checkerColor(menu.x + 1, menu.y + 1);
            verify(!nearColor(image, Math.round(menu.x + 1),
                              Math.round(menu.y + 1), topLeft),
                   "an attached menu paints its square top-left corner");
            var topRight = checkerColor(menu.x + menu.width - 2, menu.y + 1);
            verify(!nearColor(image, Math.round(menu.x + menu.width - 2),
                              Math.round(menu.y + 1), topRight),
                   "an attached menu paints its square top-right corner");
            var bottomLeft = checkerColor(menu.x + 1,
                                          menu.y + menu.height - 2);
            verify(nearColor(image, Math.round(menu.x + 1),
                             Math.round(menu.y + menu.height - 2),
                             bottomLeft),
                   "an attached menu keeps its rounded bottom corner");
            verify(!nearColor(image, Math.round(menu.x + menu.width / 2),
                              Math.round(menu.y + menu.height / 2),
                              checkerColor(menu.x + menu.width / 2,
                                           menu.y + menu.height / 2)),
                   "the card body is painted");
            saveShot(stage, "t349-menu-attached-underlay.png");
        }

        function test_detached_menu_has_rounded_corners_on_a_patterned_underlay() {
            var fixture = makeMenuOnChecker("menuDetached");
            var menu = fixture.menu;
            var image = grabImage(stage);
            saveShot(stage, "t349-menu-detached-underlay.png");
            var corners = [
                [menu.x + 1, menu.y + 1],
                [menu.x + menu.width - 2, menu.y + 1],
                [menu.x + 1, menu.y + menu.height - 2],
                [menu.x + menu.width - 2, menu.y + menu.height - 2]
            ];
            for (var i = 0; i < corners.length; ++i) {
                var x = Math.round(corners[i][0]);
                var y = Math.round(corners[i][1]);
                verify(nearColor(image, x, y, checkerColor(x, y)),
                       "a detached menu leaves corner " + i + " transparent");
            }
            verify(!nearColor(image, Math.round(menu.x + menu.width / 2),
                              Math.round(menu.y + menu.height / 2),
                              checkerColor(menu.x + menu.width / 2,
                                           menu.y + menu.height / 2)),
                   "the card body is painted");
            saveShot(stage, "t349-menu-detached-underlay.png");
        }

        // --- Cold/warm open, close, switch, reopen -------------------------

        function test_status_popover_cold_warm_open_close_reopen() {
            var bar = makeBar({});
            bar.openStatusMenu("wifi");
            waitForRendering(stage);
            var host = findChild(bar, "wifiMenu");
            var coldX = host.popup.x;
            var coldWidth = bar.dropdownWidth;
            verify(coldWidth > 0, "the cold open commits a target");
            bar.closeStatusMenu(true);
            waitForRendering(stage);
            compare(bar.dropdownWidth, 0, "the closed popup commits no target");
            bar.openStatusMenu("wifi");
            waitForRendering(stage);
            compare(host.popup.x, coldX, "the warm open keeps the anchor");
            compare(bar.dropdownWidth, coldWidth,
                    "the warm open keeps the target width");
            bar.closeStatusMenu(true);
        }

        function test_dock_chooser_open_close_switch_and_reopen() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [
                    multiWindow("files", "Files", windowList("f", 2)),
                    multiWindow("notes", "Notes", windowList("n", 3))
                ]
            });
            var chooser = findChild(dock, "windowChooser");
            verify(chooser !== null);

            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            compare(chooser.open, true, "the cold open is live");
            compare(chooser.windows.length, 2);
            verify(dock.popoverRect.w > 0 && dock.popoverRect.h > 0);
            compare(dock.popoverInputActive, true);
            dock.closePopovers();
            waitForRendering(stage);
            compare(chooser.open, false, "the close retires the logical popup");
            compare(dock.popoverInputActive, false,
                    "the close retires input before the visual tail");

            dock.openChooser(dock.items[1]);
            waitForRendering(stage);
            compare(chooser.open, true, "the switch opens the new entry");
            compare(chooser.windows.length, 3,
                    "the switch uses the new entry's windows");
            dock.closePopovers();
            waitForRendering(stage);

            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            compare(chooser.open, true, "the warm reopen is live");
            compare(chooser.windows.length, 2,
                    "the warm reopen restores the original entry");
            verify(dock.popoverRect.w > 0 && dock.popoverRect.h > 0);
            dock.closePopovers();
        }

        function test_menubar_menu_close_and_reopen_keeps_the_current_card() {
            var bar = makeBar({
                appName: "Test",
                appMenuModel: [ { title: "File", items: [
                    { label: "New" }, { label: "Open" } ] } ]
            });
            bar.openMenu(2);
            waitForRendering(stage);
            verify(bar.dropdownWidth > 0, "the cold menu commits a target");
            compare(bar.dropdownPreset, "menuAttached");
            bar.closeMenus(true);
            waitForRendering(stage);
            compare(bar.dropdownWidth, 0, "the closed menu commits no target");
            bar.openMenu(2);
            waitForRendering(stage);
            verify(bar.dropdownWidth > 0, "the warm reopen commits a target");
            compare(bar.dropdownPreset, "menuAttached");
            bar.closeMenus(true);
        }
    }
}
