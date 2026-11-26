// SPDX-License-Identifier: GPL-3.0-only
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
                // The bar's output bounds are deterministic in the fixture;
                // a test overrides them for a secondary output or a resize.
                outputWidth: 800,
                outputHeight: 640,
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
        // the starting revision opened the four status menus at x=0 because
        // their root Item had no width to clamp against. T350 supplies the
        // owning output's bounds in the popup host's parent coordinates, so the
        // shared Popup clamp lands at the status item and stays output-local.
        function statusHostObjectName(id) {
            return id === "wifi" ? "wifiMenu"
                : id === "volume" ? "volumeMenu"
                : id === "battery" ? "batteryMenu"
                : id === "bluetooth" ? "bluetoothMenu"
                : "dateTimePanel";
        }

        function expectedStatusPopupPosition(bar, id, popup) {
            var anchor = bar.statusItemFor(id);
            verify(anchor !== null, id + " status item exists");
            var bounds = bar.hostBounds;
            var anchorPoint = anchor.mapToItem(bar, 0, anchor.height);
            return {
                x: Math.max(bounds.x,
                            Math.min(bounds.x + bounds.w - popup.width,
                                     anchorPoint.x)),
                y: Math.max(bounds.y,
                            Math.min(bounds.y + bounds.h - popup.height,
                                     anchorPoint.y + popup.anchorOffset)),
                bounds: bounds,
                anchorPoint: anchorPoint
            };
        }

        function verifyStatusAnchor(id, bar, closeAfter) {
            var menuObjectName = statusHostObjectName(id);
            bar.openStatusMenu(id);
            waitForRendering(stage);
            compare(bar.openStatusItem, id, id + " opens");
            var host = findChild(bar, menuObjectName);
            verify(host !== null, menuObjectName + " exists");
            var popup = host.popup;
            verify(popup.open, id + " popup is open");
            verify(popup.width > 0, id + " popup has a positive width");

            var expected = expectedStatusPopupPosition(bar, id, popup);
            fuzzyCompare(popup.x, expected.x, 0.001,
                         id + " popup x follows the status item");
            fuzzyCompare(popup.y, expected.y, 0.001,
                         id + " popup y follows the status item");
            fuzzyCompare(bar.dropdownX, expected.x, 0.001,
                         id + " committed target x follows the status item");
            fuzzyCompare(bar.dropdownY, expected.y, 0.001,
                         id + " committed target y follows the status item");
            verify(bar.dropdownWidth > 0, id + " committed target has width");
            verify(bar.dropdownHeight > 0, id + " committed target has height");
            verify(popup.x >= expected.bounds.x - 0.001
                   && popup.x + popup.width
                      <= expected.bounds.x + expected.bounds.w + 0.001,
                   id + " popup stays inside the output horizontally");
            verify(popup.y >= expected.bounds.y - 0.001
                   && popup.y + popup.height
                      <= expected.bounds.y + expected.bounds.h + 0.001,
                   id + " popup stays inside the output vertically");
            saveShot(stage, "t349-status-" + id + ".png");
            if (closeAfter !== false)
                bar.closeStatusMenu(true);
            return { popup: popup, expected: expected };
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

        // A secondary output has its own logical bounds. The bar scene is
        // output-local, and the popup clamp must use the owning output's
        // bounds rather than the bar's own width. The compositor applies the
        // output's layout origin when it places the popup layer surface.
        function test_status_anchor_on_a_non_primary_output_origin() {
            var bar = makeBar({
                width: 360, outputWidth: 1280, outputHeight: 720,
                statusItems: statusItems(true)
            });
            var result = verifyStatusAnchor("battery", bar, false);
            verify(result.popup.x + result.popup.width <= 1280 + 0.001,
                   "the popup stays inside the secondary output");
            verify(result.popup.x > 360 - result.popup.width,
                   "the popup uses the output bounds, not the bar width");
            verify(bar.dropdownX >= 0 && bar.dropdownX < 1280,
                   "the committed target is output-local");
            bar.closeStatusMenu(true);
        }

        // The clock is the working control: it already supplied a valid host
        // width. T350 keeps its placement while routing it through the same
        // output bounds as the other status popups.
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
            var expected = expectedStatusPopupPosition(bar, "clock", popup);
            fuzzyCompare(popup.x, expected.x, 0.001,
                         "clock popup x stays on the clock");
            fuzzyCompare(popup.y, expected.y, 0.001,
                         "clock popup y stays on the clock");
            fuzzyCompare(bar.dropdownX, expected.x, 0.001,
                         "clock committed target x stays on the clock");
            bar.closeStatusMenu(true);
        }

        // Right-edge clamp: a status item near the right edge must place the
        // popup inside the output, not at x=0 and not off the right edge.
        function test_status_anchor_right_edge_clamp() {
            var bar = makeBar({ width: 360, outputWidth: 360,
                                outputHeight: 640,
                                statusItems: statusItems(true) });
            verifyStatusAnchor("battery", bar);
            var popup = findChild(bar, "batteryMenu").popup;
            fuzzyCompare(popup.x + popup.width, bar.outputWidth, 0.001,
                         "the right-edge popup clamps to the output");
        }

        // Left-edge clamp: an output narrower than the popup forces the
        // surface to the output origin instead of a negative x.
        function test_status_anchor_left_edge_clamp() {
            var bar = makeBar({ width: 200, outputWidth: 200,
                                outputHeight: 640,
                                statusOverflowCapacity: 7,
                                statusItems: statusItems(true) });
            verifyStatusAnchor("battery", bar);
            var popup = findChild(bar, "batteryMenu").popup;
            fuzzyCompare(popup.x, 0, 0.001,
                         "the narrow output clamps the popup to the left edge");
            verify(popup.width <= 200 + 0.001,
                   "the popup never grows past the output");
        }

        // A fractional bar edge must still resolve to a finite output-local
        // origin and a right edge inside the output.
        function test_status_anchor_fractional_edge() {
            var bar = makeBar({ width: 360.5, outputWidth: 361,
                                outputHeight: 640,
                                statusItems: statusItems(true) });
            verifyStatusAnchor("battery", bar);
            var popup = findChild(bar, "batteryMenu").popup;
            verify(popup.x + popup.width <= 361 + 0.001,
                   "a fractional edge stays inside the output");
        }

        // Pointer and keyboard activation both go through
        // `activateStatusItem` -> `openStatusMenu`, so the shared anchor
        // calculation must produce the same origin for both.
        function test_status_anchor_pointer_and_keyboard_agree() {
            var bar = makeBar({});
            var wifi = bar.statusItemFor("wifi");
            verify(wifi !== null);
            mouseClick(wifi, wifi.width / 2, wifi.height / 2);
            waitForRendering(stage);
            compare(bar.openStatusItem, "wifi");
            var popup = findChild(bar, "wifiMenu").popup;
            var pointerX = popup.x;
            var pointerY = popup.y;
            bar.closeStatusMenu(true);
            waitForRendering(stage);

            var keyboardIndex = -1;
            for (var i = 0; i < bar.statusRowEntries.length; ++i) {
                var item = bar.statusItemAt(i);
                if (item && item.itemId === "wifi") {
                    keyboardIndex = i;
                    break;
                }
            }
            verify(keyboardIndex >= 0, "the keyboard can select Wi-Fi");
            bar.keyboardFocused = true;
            bar.keyboardStatusIndex = keyboardIndex;
            bar.activateKeyboardStatus();
            waitForRendering(stage);
            compare(bar.openStatusItem, "wifi");
            fuzzyCompare(popup.x, pointerX, 0.001,
                         "pointer and keyboard share the x anchor");
            fuzzyCompare(popup.y, pointerY, 0.001,
                         "pointer and keyboard share the y anchor");
            bar.closeStatusMenu(true);
            bar.endKeyboardNavigation();
        }

        // A status row relayout (here the clock grows a date) must move the
        // open popup with its owning item instead of leaving it at the old x.
        function test_status_anchor_survives_status_row_relayout() {
            var bar = makeBar({ showDate: false });
            bar.openStatusMenu("wifi");
            waitForRendering(stage);
            var popup = findChild(bar, "wifiMenu").popup;
            var before = expectedStatusPopupPosition(bar, "wifi", popup);
            fuzzyCompare(popup.x, before.x, 0.001,
                         "the first layout anchors the popup");

            bar.showDate = true;
            waitForRendering(stage);
            var after = expectedStatusPopupPosition(bar, "wifi", popup);
            verify(after.anchorPoint.x !== before.anchorPoint.x,
                   "the status row actually moved");
            fuzzyCompare(popup.x, after.x, 0.001,
                         "the popup follows the relaid-out status row");
            fuzzyCompare(bar.dropdownX, after.x, 0.001,
                         "the committed target follows the relaid-out row");
            bar.closeStatusMenu(true);
        }

        // An output size change must re-run the clamp while the popup is open.
        function test_status_anchor_survives_output_resize() {
            var bar = makeBar({ width: 360, outputWidth: 360,
                                outputHeight: 640,
                                statusItems: statusItems(true) });
            bar.openStatusMenu("battery");
            waitForRendering(stage);
            var popup = findChild(bar, "batteryMenu").popup;
            fuzzyCompare(popup.x + popup.width, 360, 0.001,
                         "the narrow output clamps the popup");

            bar.width = 800;
            bar.outputWidth = 800;
            waitForRendering(stage);
            var expected = expectedStatusPopupPosition(bar, "battery", popup);
            fuzzyCompare(popup.x, expected.x, 0.001,
                         "the popup re-clamps to the resized output");
            fuzzyCompare(bar.dropdownX, expected.x, 0.001,
                         "the committed target re-clamps to the resized output");
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
            var card = dock.activePopoverRect;
            verify(rect.w > 0 && rect.h > 0, "the chooser commits a crop");
            verify(rect.x <= arrow.x + 0.01,
                   "the crop includes the arrow left edge");
            verify(rect.y <= arrow.y + 0.01,
                   "the crop includes the arrow top edge");
            verify(rect.x + rect.w + 0.01 >= arrow.x + arrow.w,
                   "the crop includes the arrow right edge");
            verify(rect.y + rect.h + 0.01 >= arrow.y + arrow.h,
                   "the crop includes the rotated arrow tip");
            // T351: the paint extent is the card plus the full shadow ring,
            // not only the arrow band. With no pre-sized budget in this
            // fixture the rect is the exact un-clamped extent.
            verify(rect.x <= card.x - dock.popoverShadowLeft + 0.01,
                   "the crop includes the left shadow reach");
            verify(rect.y <= card.y - dock.popoverShadowTop + 0.01,
                   "the crop includes the top shadow reach");
            verify(rect.x + rect.w + 0.01
                   >= card.x + card.w + dock.popoverShadowRight,
                   "the crop includes the right shadow reach");
            verify(rect.y + rect.h + 0.01
                   >= card.y + card.h + dock.popoverShadowBottom,
                   "the crop includes the bottom shadow reach");
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
            // The arrow is drawn in QML over the same card fill; the empty
            // bands on either side never receive the material descriptor.
            verify(regions[0].h < chooser.height,
                   "the material card excludes the arrow-side band");
            dock.closePopovers();
        }

        function popoverInputContains(dock, x, y) {
            var rect = dock.popoverRect;
            var lx = x - Math.floor(rect.x);
            var ly = y - Math.floor(rect.y);
            var rects = dock.activePopoverInputRects;
            for (var i = 0; i < rects.length; ++i) {
                var r = rects[i];
                if (lx >= r.x && lx < r.x + r.w
                        && ly >= r.y && ly < r.y + r.h)
                    return true;
            }
            return false;
        }

        // The picking region is the card body: the shadow ring, the empty
        // arrow-side band, and the transparent rounded corners stay clear.
        function test_chooser_input_region_excludes_the_arrow_band_and_shadow() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", windowList("w", 2)) ]
            });
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            var chooser = findChild(dock, "windowChooser");
            var card = dock.activePopoverCardRect;
            verify(dock.activePopoverInputRects.length > 0,
                   "the chooser publishes a picking region");
            verify(popoverInputContains(dock, card.x + card.w / 2,
                                        card.y + card.h / 2),
                   "the card body is interactive");
            var bandY = card.y + card.h + chooser.arrowSize / 4;
            verify(!popoverInputContains(dock, card.x + 2, bandY),
                   "the empty arrow-side band stays click-through");
            verify(!popoverInputContains(dock,
                                         card.x + card.w + 6,
                                         card.y + card.h / 2),
                   "the shadow ring stays click-through");
            verify(!popoverInputContains(dock, card.x + 1, card.y + 1),
                   "the transparent card corner stays click-through");
            dock.closePopovers();
        }

        // The same paint/material/input split holds for the other Dock
        // anchors; the persistent plate is only an anchor and never part of
        // the popup footprint.
        function test_dock_popup_footprint_for_bottom_left_and_right_anchors() {
            var anchors = [
                { position: "bottom", width: 1280, height: 160 },
                { position: "left", width: 124, height: 720 },
                { position: "right", width: 124, height: 720 }
            ];
            for (var i = 0; i < anchors.length; ++i) {
                var spec = anchors[i];
                var dock = make(dockComponent, {
                    width: spec.width, height: spec.height,
                    position: spec.position,
                    entries: [ multiWindow("files", "Files",
                                           windowList("w", 2)) ]
                });
                dock.openChooser(dock.items[0]);
                waitForRendering(stage);
                var chooser = findChild(dock, "windowChooser");
                verify(chooser !== null && chooser.open,
                       spec.position + ": the chooser is open");
                var paint = dock.popoverRect;
                var card = dock.activePopoverRect;
                verify(paint.x <= card.x - dock.popoverShadowLeft + 0.01,
                       spec.position + ": paint includes the left shadow");
                verify(paint.y <= card.y - dock.popoverShadowTop + 0.01,
                       spec.position + ": paint includes the top shadow");
                verify(paint.x + paint.w + 0.01
                       >= card.x + card.w + dock.popoverShadowRight,
                       spec.position + ": paint includes the right shadow");
                verify(paint.y + paint.h + 0.01
                       >= card.y + card.h + dock.popoverShadowBottom,
                       spec.position + ": paint includes the bottom shadow");
                verify(paint.y + paint.h + 0.01
                       >= chooser.y + chooser.height + dock.popoverArrowTip,
                       spec.position + ": paint includes the rotated arrow tip");
                var regions = dock.activePopoverSurfaceRegions;
                compare(regions.length, 1,
                        spec.position + ": one material card");
                verify(regions[0].h < chooser.height,
                       spec.position + ": material stops at the card body");
                var body = dock.activePopoverCardRect;
                verify(popoverInputContains(dock, body.x + body.w / 2,
                                            body.y + body.h / 2),
                       spec.position + ": the card body is interactive");
                verify(!popoverInputContains(dock,
                                             body.x + body.w + 6,
                                             body.y + body.h / 2),
                       spec.position + ": the shadow stays click-through");
                dock.closePopovers();
            }
        }

        // The context menu shares the same footprint split: no arrow, but the
        // shadow is a paint extent while picking stays on the card.
        function test_dock_context_menu_paint_and_input_bounds() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", windowList("w", 2)) ]
            });
            dock.openEntryMenu(dock.items[0]);
            waitForRendering(stage);
            var menu = findChild(dock, "entryMenu");
            verify(menu !== null && menu.open);
            var paint = dock.popoverRect;
            var card = dock.activePopoverRect;
            verify(paint.x <= card.x - dock.popoverShadowLeft + 0.01,
                   "the menu crop includes the left shadow");
            verify(paint.y <= card.y - dock.popoverShadowTop + 0.01,
                   "the menu crop includes the top shadow");
            verify(paint.x + paint.w + 0.01
                   >= card.x + card.w + dock.popoverShadowRight,
                   "the menu crop includes the right shadow");
            verify(paint.y + paint.h + 0.01
                   >= card.y + card.h + dock.popoverShadowBottom,
                   "the menu crop includes the bottom shadow");
            var regions = dock.activePopoverSurfaceRegions;
            compare(regions.length, 1, "the menu has one material card");
            verify(popoverInputContains(dock, card.x + card.w / 2,
                                        card.y + card.h / 2),
                   "the menu body is interactive");
            verify(!popoverInputContains(dock, card.x + card.w + 6,
                                         card.y + card.h / 2),
                   "the menu shadow stays click-through");
            dock.closePopovers();
        }

        function popupShadow(item) {
            for (var i = 0; i < item.children.length; ++i) {
                if (item.children[i].cutoutBounds !== undefined)
                    return item.children[i];
            }
            return null;
        }

        // A patterned underlay makes the silhouette observable. The shadow is
        // hidden for the body/gap/corner phase (it is a separate paint extent,
        // covered by the next phase) so the card itself is the only thing
        // over the underlay. Then the shadow returns and must paint outside
        // the card inside the committed crop.
        function test_chooser_silhouette_and_shadow_on_a_patterned_underlay() {
            var under = createTemporaryObject(checkerItemComponent, stage,
                { x: 0, y: 0, width: stage.width, height: stage.height });
            var dock = make(dockComponent, {
                x: 0, y: 420, width: 800, height: 160,
                entries: [ multiWindow("files", "Files", windowList("w", 2)) ]
            });
            var baseline = grabImage(stage);
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            wait(60);
            var chooser = findChild(dock, "windowChooser");
            var paint = dock.popoverRect;
            var card = dock.activePopoverCardRect;
            function stageX(localX) { return Math.round(dock.x + localX); }
            function stageY(localY) { return Math.round(dock.y + localY); }

            var shadow = popupShadow(chooser);
            verify(shadow !== null, "the chooser draws a shadow");
            shadow.visible = false;
            waitForRendering(stage);
            wait(40);
            var image = grabImage(stage);

            var bodyX = stageX(card.x + card.w / 2);
            var bodyY = stageY(card.y + card.h / 2);
            verify(!nearColor(image, bodyX, bodyY,
                              baseline.pixel(bodyX, bodyY)),
                   "the card body paints over the underlay");

            var bandX = stageX(card.x + 3);
            var bandY = stageY(card.y + card.h + chooser.arrowSize / 4);
            verify(nearColor(image, bandX, bandY,
                             baseline.pixel(bandX, bandY)),
                   "the empty arrow-side band leaves the underlay clear");

            var cornerX = stageX(card.x + 2);
            var cornerY = stageY(card.y + 2);
            verify(nearColor(image, cornerX, cornerY,
                             baseline.pixel(cornerX, cornerY)),
                   "the rounded card corner leaves the underlay clear");

            var arrow = findChild(chooser, "chooserArrow");
            var arrowX = stageX(chooser.x + arrow.x + arrow.width / 2);
            var arrowY = stageY(chooser.y + arrow.y + arrow.height / 2 + 1);
            verify(!nearColor(image, arrowX, arrowY,
                              baseline.pixel(arrowX, arrowY)),
                   "the arrow paints with the card treatment");

            shadow.visible = true;
            waitForRendering(stage);
            wait(40);
            var shadowImage = grabImage(stage);
            var shadowX = stageX(card.x + card.w / 2);
            var shadowY = stageY(card.y - 2);
            verify(shadowX >= stageX(paint.x)
                   && shadowX <= stageX(paint.x + paint.w),
                   "the shadow sample is inside the committed crop");
            verify(shadowY >= stageY(paint.y)
                   && shadowY <= stageY(paint.y + paint.h),
                   "the shadow sample is inside the committed crop height");
            verify(!nearColor(shadowImage, shadowX, shadowY,
                              baseline.pixel(shadowX, shadowY)),
                   "the shadow paints outside the card");
            saveShot(stage, "t351-chooser-underlay.png");
            dock.closePopovers();
        }

        // The persistent plate is not part of the popup footprint. Its
        // geometry and its painted pixels below the popup shadow stay
        // identical before, during, and after a chooser interaction.
        function test_dock_plate_geometry_survives_a_popup_interaction() {
            var dock = make(dockComponent, {
                width: 1280, height: 160,
                entries: [ multiWindow("files", "Files", windowList("w", 2)) ]
            });
            var plate = dock.plateRect;
            var panel = dock.panelRect;
            var before = grabImage(dock);
            saveShot(dock, "t351-plate-before.png");
            var samples = [];
            var sampleY = Math.round(panel.y + panel.h * 0.8);
            for (var i = 1; i < 10; ++i) {
                var sampleX = Math.round(panel.x + panel.w * i / 10);
                samples.push({ x: sampleX, y: sampleY,
                               color: before.pixel(sampleX, sampleY) });
            }
            dock.openChooser(dock.items[0]);
            waitForRendering(stage);
            compare(dock.plateRect.x, plate.x, "plate x during the popup");
            compare(dock.plateRect.y, plate.y, "plate y during the popup");
            compare(dock.plateRect.w, plate.w, "plate width during the popup");
            compare(dock.plateRect.h, plate.h, "plate height during the popup");
            compare(dock.panelRect.x, panel.x, "panel x during the popup");
            compare(dock.panelRect.y, panel.y, "panel y during the popup");
            compare(dock.panelRect.w, panel.w, "panel width during the popup");
            compare(dock.panelRect.h, panel.h, "panel height during the popup");
            var during = grabImage(dock);
            saveShot(dock, "t351-plate-during.png");
            for (var j = 0; j < samples.length; ++j) {
                verify(nearColor(during, samples[j].x, samples[j].y,
                                 samples[j].color, 0.01),
                       "the plate paints the same below the popup shadow");
            }
            dock.closePopovers();
            waitForRendering(stage);
            compare(dock.plateRect.x, plate.x, "plate x after the popup");
            compare(dock.plateRect.y, plate.y, "plate y after the popup");
            compare(dock.plateRect.w, plate.w, "plate width after the popup");
            compare(dock.plateRect.h, plate.h, "plate height after the popup");
            compare(dock.panelRect.x, panel.x, "panel x after the popup");
            compare(dock.panelRect.y, panel.y, "panel y after the popup");
            compare(dock.panelRect.w, panel.w, "panel width after the popup");
            compare(dock.panelRect.h, panel.h, "panel height after the popup");
            var after = grabImage(dock);
            saveShot(dock, "t351-plate-after.png");
            for (var k = 0; k < samples.length; ++k) {
                verify(nearColor(after, samples[k].x, samples[k].y,
                                 samples[k].color, 0.01),
                       "the plate returns to the baseline after the popup");
            }
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

        // --- Menu Bar popup footprint (T352) -------------------------------

        function pointInRect(rect, x, y) {
            return x >= rect.x && x < rect.x + rect.w
                && y >= rect.y && y < rect.y + rect.h;
        }

        function regionCoversPoint(regions, x, y) {
            for (var i = 0; i < regions.length; ++i) {
                if (pointInRect(regions[i], x, y))
                    return true;
            }
            return false;
        }

        function inputContains(bar, x, y) {
            var crop = bar.dropdownCropRect;
            var lx = x - Math.floor(crop.x);
            var ly = y - Math.floor(crop.y);
            var rects = bar.dropdownSurfaceInputRects;
            for (var i = 0; i < rects.length; ++i) {
                if (pointInRect(rects[i], lx, ly))
                    return true;
            }
            return false;
        }

        function makeSubmenuBar(props) {
            var base = {
                appName: "Test",
                appMenuModel: [ { title: "File", items: [
                    { label: "New" },
                    { label: "Open With", type: "submenu",
                      submenu: [ { label: "Text Editor" },
                                 { label: "Preview" } ] }
                ] } ]
            };
            for (var key in (props || {}))
                base[key] = props[key];
            return makeBar(base);
        }

        // The committed crop is the card plus the full shadow reach, clamped
        // to the visible output. Each attached menu and detached submenu stays
        // its own material card; the gap between them carries no material and
        // no input, and the shadow ring is click-through.
        function test_menubar_popup_crop_includes_shadow_and_separate_cards() {
            var bar = makeSubmenuBar({ width: 800, outputWidth: 800,
                                       outputHeight: 640 });
            bar.openMenu(2);
            waitForRendering(stage);
            var menu = bar.appMenuAt(2);
            menu.openSubmenu(1);
            waitForRendering(stage);
            var crop = bar.dropdownCropRect;
            var paint = bar.dropdownPaintRect;
            var bounds = bar.hostBounds;
            verify(crop.w > 0 && crop.h > 0, "the menu commits a crop");
            verify(paint.w > 0 && paint.h > 0, "the menu has a paint extent");
            verify(crop.x >= bounds.x && crop.y >= bounds.y,
                   "the crop starts inside the visible output");
            verify(crop.x + crop.w <= bounds.x + bounds.w + 0.01,
                   "the crop stays inside the output horizontally");
            verify(crop.y + crop.h <= bounds.y + bounds.h + 0.01,
                   "the crop stays inside the output vertically");

            var regions = bar.dropdownRegions;
            verify(regions.length >= 2,
                   "the attached menu and detached submenu are separate cards");
            compare(bar.dropdownSurfaceRegions.length, regions.length,
                    "every card publishes a surface region");
            for (var i = 0; i < regions.length; ++i) {
                var region = regions[i];
                var left = Math.max(bounds.x,
                                    region.x - bar.dropdownShadowLeft);
                var top = Math.max(bounds.y,
                                   region.y - bar.dropdownShadowTop);
                var right = Math.min(bounds.x + bounds.w,
                                     region.x + region.w
                                     + bar.dropdownShadowRight);
                var bottom = Math.min(bounds.y + bounds.h,
                                      region.y + region.h
                                      + bar.dropdownShadowBottom);
                verify(crop.x <= left + 0.01,
                       "card " + i + " keeps its left shadow reach");
                verify(crop.y <= top + 0.01,
                       "card " + i + " keeps its top shadow reach");
                verify(crop.x + crop.w + 0.01 >= right,
                       "card " + i + " keeps its right shadow reach");
                verify(crop.y + crop.h + 0.01 >= bottom,
                       "card " + i + " keeps its bottom shadow reach");
            }

            // The gap between the two cards is inside the crop but outside
            // every material card.
            var gapX = regions[0].x + regions[0].w + menu.submenuGap / 2;
            var gapY = regions[0].y + 20;
            verify(gapX < regions[1].x + 0.01,
                   "the submenu is a separate card across a gap");
            verify(pointInRect(crop, gapX, gapY),
                   "the crop covers the gap between the cards");
            verify(!regionCoversPoint(regions, gapX, gapY),
                   "no material region covers the menu/submenu gap");
            verify(!inputContains(bar, gapX, gapY),
                   "the menu/submenu gap is click-through");

            var body = regions[0];
            verify(inputContains(bar, body.x + body.w / 2,
                                 body.y + body.h / 2),
                   "the card body is interactive");
            verify(inputContains(bar, body.x + 1, body.y + 1),
                   "an attached menu keeps its square top corner interactive");
            verify(!inputContains(bar, body.x + 1,
                                  body.y + body.h - 2),
                   "the rounded bottom corner is click-through");
            verify(!inputContains(bar, body.x - 4, body.y + body.h / 2),
                   "the shadow ring is click-through");
            saveShot(stage, "t352-menubar-crop.png");
            bar.closeMenus(true);
        }

        // A live model change must settle the new width before the next
        // committed frame: the crop, material card and picking rects all agree
        // with the popup the first time it is rendered.
        function test_menubar_popup_first_frame_after_a_width_change() {
            var bar = makeBar({
                appName: "Test",
                appMenuModel: [ { title: "File", items: [
                    { label: "New" } ] } ]
            });
            bar.openMenu(2);
            waitForRendering(stage);
            var menu = bar.appMenuAt(2);
            var narrow = bar.dropdownCropWidth;
            verify(narrow > 0, "the first model commits a width");

            // Replace the model with a much wider row while the menu is open.
            bar.appMenuModel = [ { title: "File", items: [
                { label: "Open a very wide document" } ] } ];
            waitForRendering(stage);
            verify(bar.dropdownCropWidth > narrow,
                   "the changed model grows the committed crop");
            compare(bar.dropdownSurfaceRegions[0].w,
                    menu.popup.width,
                    "the material card matches the settled popup width");
            var crop = bar.dropdownCropRect;
            verify(crop.w >= menu.popup.width,
                   "the crop contains the settled card width");
            verify(inputContains(bar, bar.dropdownRegions[0].x + 1,
                                 bar.dropdownRegions[0].y
                                 + bar.dropdownRegions[0].h - 2) === false,
                   "the settled rounded corner stays click-through");

            bar.closeMenus(true);
            waitForRendering(stage);
            // A cold reopen with the wide model uses the same final width, not
            // the narrow first model's warm value.
            bar.openMenu(2);
            waitForRendering(stage);
            compare(bar.dropdownCropWidth,
                    Math.ceil(crop.w), "the warm reopen keeps the settled crop");
            bar.closeMenus(true);
        }

        // Quick title/status switches retarget the one popup owner. A logical
        // close publishes no card or input, and reduced motion carries a
        // zero-duration exit so the shell can unmap immediately.
        function test_menubar_popup_quick_switch_close_and_reduced_motion() {
            var bar = makeBar({
                appName: "Test",
                appMenuModel: [
                    { title: "File", items: [ { label: "New" } ] },
                    { title: "Edit", items: [ { label: "Undo" },
                                              { label: "Redo" } ] }
                ]
            });
            bar.openMenu(2);
            waitForRendering(stage);
            var file = bar.appMenuAt(2);
            var fileWidth = file.popup.width;
            bar.openMenu(3);
            waitForRendering(stage);
            var edit = bar.appMenuAt(3);
            compare(bar.openMenuIndex, 3, "the title switch retargets the menu");
            compare(file.open, false, "the old title is closed");
            compare(bar.dropdownSurfaceRegions[0].w,
                    edit.popup.width,
                    "the crop follows the new title's card");
            verify(edit.popup.width !== fileWidth
                   || bar.dropdownSurfaceRegions.length === 1,
                   "the live card is the new title's card");

            // A status switch uses the same one-owner contract.
            bar.openStatusMenu("wifi");
            waitForRendering(stage);
            var wifi = findChild(bar, "wifiMenu").popup;
            var wifiWidth = wifi.width;
            bar.openStatusMenu("volume");
            waitForRendering(stage);
            var volume = findChild(bar, "volumeMenu").popup;
            compare(bar.openStatusItem, "volume");
            compare(wifi.open, false, "the old status popup is closed");
            compare(bar.dropdownSurfaceRegions[0].w,
                    volume.width,
                    "the status crop follows the new popup");
            verify(volume.width !== wifiWidth
                   || bar.dropdownSurfaceRegions.length === 1);

            compare(bar.popupExitDuration, Theme.motion.popupClose.duration,
                    "a status close uses the popup exit token");
            bar.closeStatusMenu(true);
            waitForRendering(stage);
            compare(bar.openStatusItem, "");
            compare(bar.dropdownRegions.length, 0,
                    "a logical close publishes no material card");
            compare(bar.dropdownSurfaceInputRects.length, 0,
                    "a logical close publishes no picking region");

            // Reduced motion has a zero-duration visual exit.
            Theme.reducedMotion = true;
            var reduced = makeBar({
                appName: "Test",
                appMenuModel: [ { title: "File", items: [ { label: "New" } ] } ]
            });
            reduced.openMenu(2);
            waitForRendering(stage);
            reduced.closeMenus(true);
            waitForRendering(stage);
            compare(reduced.popupExitDuration, 0,
                    "reduced motion closes without a frost tail");
            compare(reduced.dropdownRegions.length, 0);
            compare(reduced.dropdownSurfaceInputRects.length, 0);
            Theme.reducedMotion = false;
        }

    }
}
