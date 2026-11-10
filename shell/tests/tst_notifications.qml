// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Notifications

// Notification chrome tests (T-11.1a): the banner card renders the service
// view and the notification-center list renders the history. Runs headless on
// the offscreen platform + software scene graph; the service half (the D-Bus
// round-trip) is covered by the Rust integration suite.
Item {
    id: stage
    width: 800
    height: 480

    TestCase {
        id: testCase
        name: "Notifications"
        when: windowShown

        Component { id: bannerComponent; NotificationBanner { } }
        Component { id: centerComponent; NotificationCenter { } }
        Component { id: widgetStackComponent; WidgetStack { } }
        Component { id: widgetGalleryComponent; WidgetGallery { } }

        SignalSpy { id: actionSpy; signalName: "actionInvoked" }
        SignalSpy { id: activatedSpy; signalName: "activated" }
        SignalSpy { id: centerClosedSpy; signalName: "closed" }
        SignalSpy { id: centerMoveSpy; signalName: "moveRequested" }
        SignalSpy { id: centerSelectSpy; signalName: "selectRequested" }
        SignalSpy { id: centerActivateSpy; signalName: "activateRequested" }
        SignalSpy { id: centerDismissSelectedSpy; signalName: "dismissSelectedRequested" }
        SignalSpy { id: centerActionSpy; signalName: "actionInvoked" }
        SignalSpy { id: centerEntryDismissSpy; signalName: "dismissEntryRequested" }
        SignalSpy { id: centerClearAllSpy; signalName: "clearAllRequested" }
        SignalSpy { id: centerClearHistorySpy; signalName: "clearHistoryRequested" }
        SignalSpy { id: centerEditWidgetsSpy; signalName: "editWidgetsRequested" }
        SignalSpy { id: centerWidgetRemoveSpy; signalName: "widgetRemoveRequested" }
        SignalSpy { id: widgetRemoveSpy; signalName: "removeRequested" }
        SignalSpy { id: galleryAddSpy; signalName: "addRequested" }
        SignalSpy { id: galleryRemoveSpy; signalName: "removeRequested" }
        SignalSpy { id: galleryResizeSpy; signalName: "resizeRequested" }
        SignalSpy { id: galleryCloseSpy; signalName: "closeRequested" }

        // These tests exercise the legacy sampled/fallback rail unless a
        // case explicitly selects the replacement look.
        function init() {
            Glass.rollout = "legacy";
        }

        function make(component, props) {
            var obj = createTemporaryObject(component, stage, props || {});
            waitForRendering(stage);
            return obj;
        }

        function test_the_banner_shows_the_notification() {
            var banner = make(bannerComponent, {
                appName: "Mail",
                summary: "New message",
                body: "From Ada",
                urgency: "normal"
            });
            compare(findChild(banner, "appName").text, "Mail");
            compare(findChild(banner, "summary").text, "New message");
            compare(findChild(banner, "body").text, "From Ada");
            compare(banner.Accessible.name, "New message");
        }

        function test_the_banner_hides_empty_fields() {
            var banner = make(bannerComponent, { summary: "Only a summary" });
            verify(!findChild(banner, "appName").visible);
            verify(findChild(banner, "summary").visible);
            verify(!findChild(banner, "body").visible);
        }

        function test_the_banner_renders_an_action_row_and_invokes_it() {
            var banner = make(bannerComponent, {
                appName: "Mail",
                summary: "New message",
                body: "From Ada",
                actions: [
                    { key: "reply", label: "Reply" },
                    { key: "archive", label: "Archive" }
                ]
            });
            // The card grows for the action row.
            compare(banner.cardHeight, 132);
            verify(findChild(banner, "actionsRow").visible);

            var reply = findChild(banner, "action_reply");
            verify(reply);
            // AX: the action is a named button, but the banner never takes
            // keyboard focus (the Center is the keyboard path).
            compare(reply.Accessible.role, Accessible.Button);
            compare(reply.Accessible.name, "Reply");
            compare(reply.Accessible.focusable, false);
            compare(banner.Accessible.role, Accessible.Alert);
            compare(banner.Accessible.focusable, false);
            verify(banner.Accessible.description.indexOf("2 actions") >= 0);
            actionSpy.target = banner;
            actionSpy.clear();
            mouseClick(reply, reply.width / 2, reply.height / 2);
            compare(actionSpy.count, 1);
            compare(actionSpy.signalArguments[0][0], "reply");
        }

        function test_the_banner_body_click_activates() {
            var banner = make(bannerComponent, { summary: "New message", body: "From Ada" });
            compare(banner.cardHeight, 96);
            activatedSpy.target = banner;
            activatedSpy.clear();
            mouseClick(banner, 200, 40);
            compare(activatedSpy.count, 1);
        }

        function test_the_center_renders_every_history_entry() {
            var center = make(centerComponent, {
                history: [
                    { appName: "Mail", summary: "New message", body: "From Ada" },
                    { appName: "Chat", summary: "Ping", body: "" }
                ]
            });
            var list = findChild(center, "historyList");
            compare(list.count, 2);
            // Delegates are created during rendering; wait for the first row.
            var first = null;
            for (var i = 0; i < 25 && !first; ++i) {
                wait(20);
                first = list.itemAtIndex(0);
            }
            verify(first);
            compare(findChild(first, "historySummary").text, "New message");
            compare(findChild(list.itemAtIndex(1), "historySummary").text, "Ping");
            verify(!findChild(center, "emptyHistory").visible);
        }

        function test_the_center_shows_an_empty_state() {
            var center = make(centerComponent, { history: [] });
            verify(findChild(center, "emptyHistory").visible);
            compare(findChild(center, "emptyHistory").text, "No notifications");
            compare(findChild(center, "historyList").count, 0);
            compare(center.Accessible.name, "Notifications");
        }

        function centerRow(list, index) {
            var row = null;
            for (var i = 0; i < 25 && !row; ++i) {
                wait(20);
                row = list.itemAtIndex(index);
            }
            return row;
        }

        function activeEntry(id, summary, appName) {
            return {
                key: "" + id,
                id: id,
                appName: appName || "Mail",
                summary: summary,
                body: "From Ada",
                status: "active",
                active: true,
                canAct: true,
                actions: [
                    { key: "reply", label: "Reply" },
                    { key: "default", label: "Open" }
                ]
            };
        }

        function historyEntry(id, summary, status) {
            return {
                key: "" + id,
                id: id,
                appName: "Mail",
                summary: summary,
                body: "From Ada",
                status: status,
                active: false,
                canAct: false,
                actions: []
            };
        }

        function test_the_center_separates_active_rows_from_history_rows() {
            var center = make(centerComponent, {
                history: [
                    activeEntry(7, "New message"),
                    historyEntry(6, "Old message", "expired"),
                    historyEntry(5, "Quiet message", "suppressed")
                ]
            });
            var list = findChild(center, "historyList");
            compare(list.count, 3);
            var active = centerRow(list, 0);
            var expired = centerRow(list, 1);
            var suppressed = centerRow(list, 2);
            verify(active && expired && suppressed);
            compare(active.objectName, "history_7");
            verify(findChild(active, "historyActions").visible);
            verify(findChild(active, "historyDismiss").visible);
            compare(findChild(active, "historyStatus").text, "");
            compare(findChild(expired, "historyStatus").text, "Expired");
            verify(!findChild(expired, "historyActions").visible);
            verify(!findChild(expired, "historyDismiss").visible);
            compare(findChild(suppressed, "historyStatus").text, "Suppressed by Focus");
            verify(!findChild(suppressed, "historyActions").visible);
            compare(active.Accessible.name, "New message");
            compare(active.Accessible.description, "From Ada");
        }

        function test_the_center_routes_active_actions_and_dismissal() {
            var center = make(centerComponent, {
                history: [activeEntry(7, "New message")]
            });
            var list = findChild(center, "historyList");
            var row = centerRow(list, 0);
            verify(row);
            centerActionSpy.target = center;
            centerActionSpy.clear();
            var reply = findChild(row, "historyAction_reply");
            verify(reply);
            mouseClick(reply, reply.width / 2, reply.height / 2);
            compare(centerActionSpy.count, 1);
            compare(centerActionSpy.signalArguments[0][0], 7);
            compare(centerActionSpy.signalArguments[0][1], "reply");

            centerEntryDismissSpy.target = center;
            centerEntryDismissSpy.clear();
            var dismiss = findChild(row, "historyDismiss");
            mouseClick(dismiss, dismiss.width / 2, dismiss.height / 2);
            compare(centerEntryDismissSpy.count, 1);
            compare(centerEntryDismissSpy.signalArguments[0][0], 7);
        }

        function test_the_center_shows_an_unavailable_state() {
            var center = make(centerComponent, { history: [], available: false });
            var empty = findChild(center, "emptyHistory");
            verify(empty.visible);
            compare(empty.text, "Notifications unavailable");
            compare(empty.Accessible.name, "Notifications unavailable");
        }

        function test_the_center_keyboard_traversal_raises_requests() {
            var center = make(centerComponent, {
                history: [activeEntry(7, "New message"),
                          historyEntry(6, "Old message", "expired")]
            });
            centerMoveSpy.target = center;
            centerSelectSpy.target = center;
            centerActivateSpy.target = center;
            centerDismissSelectedSpy.target = center;
            centerClosedSpy.target = center;
            center.forceActiveFocus();
            wait(20);

            keyClick(Qt.Key_Down);
            compare(centerMoveSpy.count, 1);
            compare(centerMoveSpy.signalArguments[0][0], 1);
            keyClick(Qt.Key_Up);
            compare(centerMoveSpy.count, 2);
            compare(centerMoveSpy.signalArguments[1][0], -1);
            keyClick(Qt.Key_End);
            compare(centerSelectSpy.count, 1);
            compare(centerSelectSpy.signalArguments[0][0], 1);
            keyClick(Qt.Key_Home);
            compare(centerSelectSpy.count, 2);
            compare(centerSelectSpy.signalArguments[1][0], 0);
            keyClick(Qt.Key_Return);
            compare(centerActivateSpy.count, 1);
            keyClick(Qt.Key_Delete);
            compare(centerDismissSelectedSpy.count, 1);
            keyClick(Qt.Key_Escape);
            compare(centerClosedSpy.count, 1);
        }

        function compareColor(actual, expected, label) {
            fuzzyCompare(actual.r, expected.r, 0.001, label + " red");
            fuzzyCompare(actual.g, expected.g, 0.001, label + " green");
            fuzzyCompare(actual.b, expected.b, 0.001, label + " blue");
            fuzzyCompare(actual.a, expected.a, 0.001, label + " alpha");
        }

        function test_popup_role_material_covers_the_notification_banner() {
            // T-21.2c: a banner is the transient Popup role. Its card keeps
            // the urgency stripe and rim while the fill switches to the
            // milky plate when the sampled pass is absent.
            var states = [
                { tier: 0, blur: true, fallback: 0, reason: "none", label: "full" },
                { tier: 1, blur: true, fallback: 0, reason: "none", label: "reduced" },
                { tier: 2, blur: false, fallback: 0, reason: "minimal", label: "minimal" },
                { tier: 2, blur: false, fallback: 2, reason: "software", label: "software" }
            ];
            for (var scheme = 0; scheme < 2; ++scheme) {
                Theme.dark = scheme === 1;
                for (var i = 0; i < states.length; ++i) {
                    var state = states[i];
                    var banner = make(bannerComponent, {
                        summary: "New message",
                        materialTier: state.tier,
                        materialBlurActive: state.blur,
                        materialFallback: state.fallback
                    });
                    compare(banner.materialTier, state.tier, state.label);
                    compare(banner.materialBlurActive, state.blur, state.label);
                    compare(banner.materialFallback, state.fallback, state.label);
                    compare(banner.materialFallbackReason, state.reason,
                            state.label);
                    var card = findChild(banner, "bannerCard");
                    verify(card !== null);
                    compareColor(card.color, Material.popupFill(state.blur),
                                 state.label);
                    verify(card.color.a >= 0.9 || state.blur,
                           state.label + " fallback must read as a plate");
                }
            }
            Theme.dark = false;
        }

        // --- T205: banner, stack and lower-panel materials ------------------

        function test_the_banner_uses_the_notification_banner_preset() {
            var savedRollout = Glass.rollout;
            try {
                Glass.rollout = "replacement";
                Theme.dark = false;
                var banner = make(bannerComponent, {
                    summary: "New message",
                    materialBlurActive: true,
                    materialTier: 0,
                    materialFallback: 0
                });
                compare(banner.bannerGlass.preset, "notificationBanner");
                fuzzyCompare(banner.bannerGlass.alpha, 0.549, 0.001,
                             "light banner alpha");
                fuzzyCompare(banner.bannerGlass.tint, 0.976, 0.001,
                             "light banner tint");
                compare(banner.sampledGlass, true);
                compare(banner.cardFill.a, 0.0);
                var regions = banner.materialRegions;
                compare(regions.length, 1);
                compare(regions[0].key, "banner");
                compare(regions[0].preset, "notificationBanner");
                compare(regions[0].h, 96);
                compare(findChild(banner, "bannerCard").color.a, 0.0);

                var withActions = make(bannerComponent, {
                    summary: "New message",
                    actions: [{ key: "reply", label: "Reply" }]
                });
                compare(withActions.materialRegions[0].h, 132);

                Theme.dark = true;
                var dark = make(bannerComponent, { summary: "New message" });
                compare(dark.bannerGlass.scheme, "dark");
                fuzzyCompare(dark.bannerGlass.alpha, 0.56, 0.001,
                             "dark banner alpha");
                fuzzyCompare(dark.bannerGlass.tint, 0.30, 0.001,
                             "dark banner tint");
            } finally {
                Glass.rollout = savedRollout;
                Theme.dark = false;
            }
        }

        function test_the_banner_fallbacks_use_the_shared_policy() {
            var savedRollout = Glass.rollout;
            var savedReduce = Glass.reduceTransparency;
            var savedContrast = Glass.increaseContrast;
            try {
                Glass.rollout = "replacement";
                Glass.reduceTransparency = true;
                Glass.increaseContrast = false;
                Theme.dark = false;
                var reduced = make(bannerComponent, {
                    summary: "New message",
                    materialBlurActive: true,
                    materialTier: 0,
                    materialFallback: 0
                });
                compare(reduced.bannerGlass.renderMode, "solid");
                verify(reduced.cardFill.a >= 0.95, "Reduce Transparency plate");
                compare(reduced.sampledGlass, false);

                Glass.reduceTransparency = false;
                var software = make(bannerComponent, {
                    summary: "New message",
                    materialBlurActive: true,
                    materialTier: 0,
                    materialFallback: 2
                });
                compare(software.bannerGlass.renderMode, "software-fallback");
                verify(software.cardFill.a >= 0.95, "software plate");
                compare(software.materialFallbackReason, "software");

                var minimal = make(bannerComponent, {
                    summary: "New message",
                    materialBlurActive: false,
                    materialTier: 2,
                    materialFallback: 0
                });
                compare(minimal.bannerGlass.renderMode, "solid");
                compare(minimal.materialFallbackReason, "minimal");
                verify(minimal.cardFill.a >= 0.95, "minimal plate");

                Glass.rollout = "legacy";
                var legacy = make(bannerComponent, {
                    summary: "New message",
                    materialBlurActive: true
                });
                compare(legacy.sampledGlass, false);
                compareColor(legacy.cardFill, Material.popupFill(true), "legacy");
            } finally {
                Glass.rollout = savedRollout;
                Glass.reduceTransparency = savedReduce;
                Glass.increaseContrast = savedContrast;
                Theme.dark = false;
            }
        }

        function test_the_center_declares_disjoint_stack_and_lower_regions() {
            var savedRollout = Glass.rollout;
            try {
                Glass.rollout = "replacement";
                Theme.dark = false;
                var center = make(centerComponent, {
                    history: [activeEntry(7, "New message"),
                              historyEntry(6, "Old message", "expired")]
                });
                compare(center.stackGlass.preset, "notificationStack");
                compare(center.lowerGlass.preset, "notificationLowerPanel");
                fuzzyCompare(center.stackGlass.alpha, 0.551, 0.001,
                             "light stack alpha");
                fuzzyCompare(center.stackGlass.tint, 0.706, 0.001,
                             "light stack tint");
                fuzzyCompare(center.lowerGlass.alpha, 0.407, 0.001,
                             "light lower alpha");
                fuzzyCompare(center.lowerGlass.tint, 0.852, 0.001,
                             "light lower tint");
                compare(center.stackFill.a, 0.0);
                compare(center.lowerFill.a, 0.0);

                var regions = center.materialRegions;
                compare(regions.length, 2);
                compare(regions[0].key, "stack");
                compare(regions[0].preset, "notificationStack");
                compare(regions[1].key, "lower");
                compare(regions[1].preset, "notificationLowerPanel");
                compare(regions[0].y, 0);
                compare(regions[1].y, center.height - center.lowerPanelHeight);
                verify(regions[0].y + regions[0].h <= regions[1].y,
                       "stack and lower must not overlap");
                verify(regions[1].x + regions[1].w <= center.width);
                verify(regions[1].y + regions[1].h <= center.height);

                // A co-visible banner insets the stack; the regions stay
                // disjoint and the transparent top strip declares no material.
                center.bannerInset = 140;
                waitForRendering(stage);
                regions = center.materialRegions;
                compare(regions.length, 2);
                compare(regions[0].y, 140);
                compare(regions[1].y, center.height - center.lowerPanelHeight);
                verify(regions[0].y + regions[0].h <= regions[1].y,
                       "banner inset keeps the regions disjoint");
                verify(regions.length <= 32, "region bound");
            } finally {
                Glass.rollout = savedRollout;
                Theme.dark = false;
            }
        }

        function test_the_center_light_and_dark_materials_are_distinct() {
            var savedRollout = Glass.rollout;
            try {
                Glass.rollout = "replacement";
                Theme.dark = false;
                var light = make(centerComponent, { history: [] });
                fuzzyCompare(light.stackGlass.alpha, 0.551, 0.001,
                             "light stack alpha");
                fuzzyCompare(light.stackGlass.tint, 0.706, 0.001,
                             "light stack tint");
                var lightAlpha = light.stackGlass.alpha;
                var lightTint = light.stackGlass.tint;
                Theme.dark = true;
                var dark = make(centerComponent, { history: [] });
                fuzzyCompare(dark.stackGlass.alpha, 0.507, 0.001,
                             "dark stack alpha");
                fuzzyCompare(dark.stackGlass.tint, 0.550, 0.001,
                             "dark stack tint");
                fuzzyCompare(dark.lowerGlass.alpha, 0.40, 0.001,
                             "dark lower alpha");
                fuzzyCompare(dark.lowerGlass.tint, 0.30, 0.001,
                             "dark lower tint");
                verify(lightAlpha !== dark.stackGlass.alpha
                       || lightTint !== dark.stackGlass.tint,
                       "Light and Dark stacks must be distinct");
                verify(dark.lowerGlass.alpha !== dark.stackGlass.alpha
                       || dark.lowerGlass.tint !== dark.stackGlass.tint,
                       "Dark lower panel must be distinct from the stack");
            } finally {
                Glass.rollout = savedRollout;
                Theme.dark = false;
            }
        }

        function test_the_center_empty_and_scrollable_states() {
            var center = make(centerComponent, { history: [] });
            var empty = findChild(center, "emptyHistory");
            verify(empty.visible);
            compare(empty.text, "No notifications");
            compare(center.lowerSummary, "0 notifications");
            compare(findChild(center, "historyScrollBar").visible, false);
            compare(findChild(center, "historyList").count, 0);

            var unavailable = make(centerComponent, {
                history: [],
                available: false
            });
            compare(findChild(unavailable, "emptyHistory").text,
                    "Notifications unavailable");
            compare(unavailable.lowerSummary, "Notification service unavailable");

            var rows = [];
            for (var i = 0; i < 18; ++i)
                rows.push(historyEntry(100 + i, "Message " + (i + 1),
                                       i % 2 === 0 ? "expired" : "dismissed"));
            var scrollable = make(centerComponent, { history: rows });
            var list = findChild(scrollable, "historyList");
            verify(list.contentHeight > list.height,
                   "18 rows overflow the stack viewport");
            verify(findChild(scrollable, "historyScrollBar").visible);
            compare(findChild(scrollable, "lowerPanelHint").text,
                    "Scroll for more");
            compare(scrollable.lowerSummary, "18 notifications");
        }

        function test_the_center_fallbacks_use_the_shared_policy() {
            var savedRollout = Glass.rollout;
            var savedReduce = Glass.reduceTransparency;
            var savedContrast = Glass.increaseContrast;
            try {
                Glass.rollout = "replacement";
                Glass.reduceTransparency = true;
                Glass.increaseContrast = false;
                Theme.dark = false;
                var reduced = make(centerComponent, { history: [] });
                compare(reduced.stackGlass.renderMode, "solid");
                compare(reduced.lowerGlass.renderMode, "solid");
                verify(reduced.stackFill.a >= 0.95, "stack Reduce Transparency");
                verify(reduced.lowerFill.a >= 0.95, "lower Reduce Transparency");

                Glass.reduceTransparency = false;
                var software = make(centerComponent, {
                    history: [],
                    materialBlurActive: true,
                    materialTier: 0,
                    materialFallback: 2
                });
                compare(software.stackGlass.renderMode, "software-fallback");
                compare(software.lowerGlass.renderMode, "software-fallback");
                compare(software.materialFallbackReason, "software");
                verify(software.stackFill.a >= 0.95);
                verify(software.lowerFill.a >= 0.95);

                Glass.increaseContrast = true;
                var contrast = make(centerComponent, { history: [] });
                verify(contrast.stackGlass.edge > 0.3,
                       "Increase Contrast raises the stack rim");
                verify(contrast.lowerGlass.edge > 0.3,
                       "Increase Contrast raises the lower rim");
            } finally {
                Glass.rollout = savedRollout;
                Glass.reduceTransparency = savedReduce;
                Glass.increaseContrast = savedContrast;
                Theme.dark = false;
            }
        }

        // --- T234: measured layout, header actions, grouping and widgets ----

        function test_the_center_uses_the_measured_344_pt_ramp() {
            var center = make(centerComponent, { history: [] });
            compare(center.width, 344);
            compare(center.measuredWidth, 344);
            verify(findChild(center, "centerHeader") !== null);
            compare(findChild(center, "centerTitle").text, "Notifications");
            // The lower panel keeps its measured summary and the Edit Widgets
            // action even with no widgets.
            compare(findChild(center, "lowerPanelSummary").text, "0 notifications");
            verify(findChild(center, "editWidgetsButton") !== null);
            verify(!findChild(center, "clearAllButton").visible,
                   "Clear All hides on an empty history");
        }

        function test_the_center_header_actions_raise_clear_requests() {
            var center = make(centerComponent, {
                history: [activeEntry(7, "New message")]
            });
            centerClearAllSpy.target = center;
            centerClearHistorySpy.target = center;
            centerClearAllSpy.clear();
            centerClearHistorySpy.clear();

            var clearAll = findChild(center, "clearAllButton");
            verify(clearAll.visible);
            mouseClick(clearAll, clearAll.width / 2, clearAll.height / 2);
            compare(centerClearAllSpy.count, 1);

            var clearHistory = findChild(center, "clearHistoryButton");
            verify(clearHistory.visible);
            mouseClick(clearHistory, clearHistory.width / 2, clearHistory.height / 2);
            compare(centerClearHistorySpy.count, 1);

            centerEditWidgetsSpy.target = center;
            centerEditWidgetsSpy.clear();
            var edit = findChild(center, "editWidgetsButton");
            mouseClick(edit, edit.width / 2, edit.height / 2);
            compare(centerEditWidgetsSpy.count, 1);
        }

        function test_the_center_active_banner_row_is_75_pt_and_groups_history() {
            var grouped = historyEntry(8, "Older one", "expired");
            grouped.groupStart = true;
            grouped.groupCount = 2;
            var center = make(centerComponent, {
                history: [activeEntry(9, "New message", "Chat"),
                          grouped,
                          historyEntry(7, "Older two", "dismissed")]
            });
            var list = findChild(center, "historyList");
            compare(list.count, 3);
            var active = centerRow(list, 0);
            verify(active);
            compare(active.height, 75, "measured banner-stack row");

            // The two history rows are one contiguous Mail group: the first
            // carries the group header, the second does not.
            var first = centerRow(list, 1);
            var second = centerRow(list, 2);
            verify(first && second);
            var header = findChild(first, "historyGroupHeader");
            verify(header.visible);
            compare(header.text, "Mail · 2");
            verify(!findChild(second, "historyGroupHeader").visible);
        }

        function widget(id, kind, title, available, data) {
            return {
                id: id,
                kind: kind,
                title: title,
                size: "medium",
                available: available,
                data: data || {}
            };
        }

        function test_the_widget_stack_empty_and_live_calendar_states() {
            var empty = make(widgetStackComponent, { widgets: [] });
            verify(findChild(empty, "widgetEmpty").visible);
            compare(findChild(empty, "widgetEmpty").text, "No widgets");

            var live = make(widgetStackComponent, {
                widgets: [widget("widget-1", "calendar", "Calendar", true)]
            });
            var card = findChild(live, "widget_widget-1");
            verify(card);
            compare(card.width, 164);
            compare(card.height, 164);
            verify(findChild(card, "calendarMonth").text.length > 0);
            verify(findChild(card, "calendarGrid").columns === 7);

            // The stored panel size changes the live card; the gallery
            // thumbnail size is never used in the panel.
            var small = widget("widget-2", "calendar", "Calendar", true);
            small.size = "small";
            var large = widget("widget-3", "calendar", "Calendar", true);
            large.size = "large";
            var sized = make(widgetStackComponent, { widgets: [small, large] });
            compare(findChild(sized, "widget_widget-2").width, 102);
            compare(findChild(sized, "widget_widget-3").width, sized.width);
        }

        function test_the_widget_stack_shows_provider_absence_and_removes() {
            var stack = make(widgetStackComponent, {
                widgets: [widget("widget-2", "weather", "Weather", false)]
            });
            var card = findChild(stack, "widget_widget-2");
            verify(card);
            verify(findChild(card, "widgetUnavailable").visible);
            compare(findChild(card, "widgetUnavailable").text, "Weather unavailable");

            widgetRemoveSpy.target = stack;
            widgetRemoveSpy.clear();
            mouseMove(card, card.width / 2, card.height / 2);
            wait(20);
            var remove = findChild(card, "widgetRemove");
            verify(remove.visible);
            mouseClick(remove, remove.width / 2, remove.height / 2);
            compare(widgetRemoveSpy.count, 1);
            compare(widgetRemoveSpy.signalArguments[0][0], "widget-2");
        }

        function test_the_widget_gallery_is_a_separate_measured_window() {
            var categories = [];
            for (var i = 0; i < 19; ++i)
                categories.push("Category " + i);
            var gallery = make(widgetGalleryComponent, {
                categories: categories,
                catalog: [
                    { kind: "calendar", title: "Calendar", category: "Category 0",
                      description: "Month grid", sizes: ["medium"] },
                    { kind: "weather", title: "Weather", category: "Category 1",
                      description: "Conditions", sizes: ["medium"] }
                ]
            });
            compare(gallery.width, 1027);
            compare(gallery.height, 552);
            compare(gallery.sidebarWidth, 240);
            // All + the 19 measured categories.
            compare(findChild(gallery, "galleryCategoryList").count, 20);
            compare(findChild(gallery, "galleryTitle").text, "Widget Gallery");
            compare(findChild(gallery, "galleryThumbnail").width, 102);
            compare(findChild(gallery, "galleryThumbnail").height, 102);
        }

        function test_the_widget_gallery_filters_and_raises_add_remove() {
            // A 760 pt instance keeps every control inside the 800 pt test
            // window; the measured 1027 pt window is asserted separately.
            var gallery = make(widgetGalleryComponent, {
                width: 760,
                height: 480,
                categories: ["Calendar", "Weather"],
                catalog: [
                    { kind: "calendar", title: "Calendar", category: "Calendar",
                      description: "Month grid", sizes: ["medium"] },
                    { kind: "weather", title: "Weather", category: "Weather",
                      description: "Conditions", sizes: ["medium"] }
                ]
            });
            compare(gallery.filteredCatalog.length, 2);
            gallery.searchText = "weather";
            compare(gallery.filteredCatalog.length, 1);
            gallery.searchText = "";
            gallery.selectedCategory = "Weather";
            compare(gallery.filteredCatalog.length, 1);

            galleryAddSpy.target = gallery;
            galleryAddSpy.clear();
            var add = findChild(gallery, "galleryToggle_weather");
            verify(add);
            mouseClick(add, add.width / 2, add.height / 2);
            compare(galleryAddSpy.count, 1);
            compare(galleryAddSpy.signalArguments[0][0], "weather");

            // A widget already in the panel offers Remove.
            gallery.widgets = [widget("widget-1", "weather", "Weather", true)];
            wait(50);
            galleryRemoveSpy.target = gallery;
            galleryRemoveSpy.clear();
            var remove = findChild(gallery, "galleryToggle_weather");
            verify(remove);
            mouseClick(remove, remove.width / 2, remove.height / 2);
            compare(galleryRemoveSpy.count, 1);
            compare(galleryRemoveSpy.signalArguments[0][0], "widget-1");

            galleryResizeSpy.target = gallery;
            galleryResizeSpy.clear();
            var size = findChild(gallery, "gallerySize_weather");
            verify(size);
            mouseClick(size, size.width / 2, size.height / 2);
            compare(galleryResizeSpy.count, 1);
            compare(galleryResizeSpy.signalArguments[0][0], "widget-1");
            compare(galleryResizeSpy.signalArguments[0][1], "large");

            galleryCloseSpy.target = gallery;
            galleryCloseSpy.clear();
            var done = findChild(gallery, "galleryDone");
            mouseClick(done, done.width / 2, done.height / 2);
            compare(galleryCloseSpy.count, 1);
        }

        function test_the_banner_stack_peeks_and_reduced_motion() {
            var banner = make(bannerComponent, {
                summary: "New message",
                stackCount: 3,
                reducedMotion: true
            });
            verify(findChild(banner, "bannerStackPeek1").visible);
            verify(findChild(banner, "bannerStackPeek2").visible);
            compare(banner.enterProgress, 1);
            banner.present = false;
            wait(20);
            compare(banner.enterProgress, 0);
            compare(banner.opacity, 0);
            compare(banner.materialRegions[0].y, 0, "reduced motion has no slide");
        }

        // --- T234 evidence: the measured ramp, banner stack and gallery ----

        function saveShot(item, name) {
            // QML's QImage.save() returns void through the test binding, so
            // the capture script verifies the files instead.
            grabImage(item).save(TestEnvironment.captureDir + "/" + name, "PNG");
        }

        function test_t234_capture_states() {
            if (typeof TestEnvironment === "undefined"
                    || TestEnvironment.captureDir.length === 0)
                skip("DF_NOTIFICATIONS_CAPTURE_DIR not set");
            var dir = TestEnvironment.captureDir;
            // The measured Center is 620 pt and the gallery 1027×552 pt, both
            // larger than the interaction stage; grow the offscreen window so
            // grabImage captures the whole item.
            stage.width = 1100;
            stage.height = 900;
            wait(100);

            var grouped = historyEntry(8, "Older one", "expired");
            grouped.groupStart = true;
            grouped.groupCount = 2;
            var rows = [activeEntry(9, "New message", "Mail"),
                        grouped,
                        // A Focus-suppressed row: the service keeps it in the
                        // history and the Center shows the reason.
                        historyEntry(6, "Quiet update", "suppressed"),
                        historyEntry(7, "Older two", "dismissed")];
            var calendar = widget("widget-1", "calendar", "Calendar", true);

            for (var dark = 0; dark < 2; ++dark) {
                Theme.dark = dark === 1;
                var suffix = dark === 1 ? "dark" : "light";

                var banner = make(bannerComponent, {
                    appName: "Mail",
                    summary: "New message",
                    body: "Ada Lovelace — Notes on the Analytical Engine",
                    actions: [
                        { key: "reply", label: "Reply" },
                        { key: "default", label: "Open" }
                    ],
                    stackCount: 2,
                    reducedMotion: true
                });
                saveShot(banner, "t234-banner-" + suffix + ".png");
                banner.destroy();

                var center = make(centerComponent, {
                    history: rows,
                    widgets: [calendar],
                    available: true
                });
                saveShot(center, "t234-center-populated-" + suffix + ".png");
                center.destroy();

                var empty = make(centerComponent, {
                    history: [],
                    widgets: [],
                    available: true
                });
                saveShot(empty, "t234-center-empty-" + suffix + ".png");
                empty.destroy();

                var gallery = make(widgetGalleryComponent, {
                    width: 1027,
                    height: 552,
                    categories: [
                        "Clock", "Calendar", "Weather", "Reminders", "Notes",
                        "Stocks", "Battery", "Activity", "News", "Music",
                        "Podcasts", "Books", "Software Center", "Devices", "Home",
                        "Photos", "Files", "Contacts", "Tips"
                    ],
                    catalog: [
                        { kind: "calendar", title: "Calendar",
                          category: "Calendar", description: "Live month grid",
                          sizes: ["medium"] },
                        { kind: "weather", title: "Weather",
                          category: "Weather", description: "Current conditions",
                          sizes: ["medium"] }
                    ],
                    widgets: [calendar]
                });
                saveShot(gallery, "t234-widget-gallery-" + suffix + ".png");
                gallery.destroy();
            }
            Theme.dark = false;
            stage.width = 800;
            stage.height = 480;
        }
    }
}