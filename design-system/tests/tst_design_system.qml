// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Gallery

// Per-component and per-token tests for the design system (T-08, FR-1…FR-6).
// Runs headless on the offscreen platform + software scene graph; the visual
// assertions sample real grabbed pixels, so they are deterministic without
// depending on font rasterization.
//
// `stage` is the visual root so it is shown in the QuickTest window and
// `grabImage` returns real pixels.
Item {
    id: stage
    width: 600
    height: 400

TestCase {
    id: testCase
    name: "DesignSystem"
    when: windowShown

    Component { id: toggleComponent; Toggle { } }
    Component { id: lightsComponent; TrafficLights { } }
    Component { id: titleBarComponent; TitleBar { } }
    Component { id: ssdComponent; SsdTitlebarReference { } }
    Component { id: popupComponent; Popup { } }
    Component { id: menuComponent; MenuBarMenu { } }
    Component { id: rectComponent; Rectangle { } }
    Component { id: buttonComponent; Button { } }
    Component { id: toolbarComponent; Toolbar { } }
    Component { id: splitViewComponent; SplitView { } }
    Component { id: settingsRowComponent; SettingsRow { } }
    Component { id: settingsGroupComponent; SettingsGroup { } }
    Component { id: segmentedComponent; SegmentedControl { } }
    Component { id: contextMenuComponent; ContextMenu { } }
    Component { id: searchFieldComponent; SearchField { } }
    Component { id: sourceListComponent; SourceList { } }
    Component { id: sidebarComponent; Sidebar { } }
    Component { id: dialogComponent; Dialog { } }
    Component { id: sheetComponent; Sheet { } }
    Component { id: popoverComponent; Popover { } }
    Component {
        id: scrollViewComponent
        ScrollView {
            width: 200
            height: 100
            Item { width: 200; height: 400 }
        }
    }

    SignalSpy { id: closeSpy; signalName: "closeClicked" }
    SignalSpy { id: zoomSpy; signalName: "zoomRequested" }
    SignalSpy { id: menuSpy; signalName: "menuRequested" }
    SignalSpy { id: triggeredSpy; signalName: "triggered" }
    SignalSpy { id: popupClosedSpy; signalName: "closed" }
    SignalSpy { id: buttonSpy; signalName: "clicked" }
    SignalSpy { id: segmentedSpy; signalName: "activated" }
    SignalSpy { id: sidebarSpy; signalName: "activated" }
    SignalSpy { id: sourceListSpy; signalName: "activated" }
    SignalSpy { id: acceptedSpy; signalName: "accepted" }
    SignalSpy { id: rejectedSpy; signalName: "rejected" }

    function make(component, props) {
        var obj = createTemporaryObject(component, stage, props || {});
        waitForRendering(stage);
        return obj;
    }

    function channel(value) {
        return Math.round(value * 255);
    }

    // -- Tokens (FR-2) ------------------------------------------------------

    function test_tokens_have_three_tiers() {
        compare(Theme.primitive.radius.md, 10);
        compare(Theme.primitive.spacing.sm, 8);
        compare(Theme.controls.titlebar.height, 40);
        compare(Theme.controls.trafficLights.diameter, 12);
        compare(Theme.motion.menuOpen.fullDuration, 160);
        verify(Theme.color.accent !== undefined);
    }

    function test_light_and_dark_semantics() {
        var saved = Theme.dark;
        Theme.dark = false;
        compare(String(Theme.color.surface), "#ffffff");
        Theme.dark = true;
        compare(String(Theme.color.surface), "#1d1723");
        Theme.dark = saved;
    }

    function test_reduced_motion_collapses_durations() {
        var saved = Theme.reducedMotion;
        Theme.reducedMotion = false;
        compare(Theme.motion.popupOpen.duration, 160);
        Theme.reducedMotion = true;
        compare(Theme.motion.popupOpen.duration, 0);
        Theme.reducedMotion = saved;
    }

    // -- Toggle (FR-1) ------------------------------------------------------

    function test_toggle_keyboard_and_accessible_role() {
        var t = make(toggleComponent, { text: "Wi-Fi", checked: false });
        compare(t.Accessible.role, Accessible.Switch);
        compare(t.Accessible.name, "Wi-Fi");
        compare(t.Accessible.checkable, true);
        compare(t.Accessible.checked, false);

        t.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Space);
        compare(t.checked, true);
        compare(t.Accessible.checked, true);
    }

    function test_toggle_visual_track_color() {
        var t = make(toggleComponent, { checked: true });
        var img = grabImage(t);
        var cy = Math.round(Theme.controls.toggle.height / 2);
        compare(img.red(4, cy), channel(Theme.color.accent.r));
        compare(img.green(4, cy), channel(Theme.color.accent.g));
        compare(img.blue(4, cy), channel(Theme.color.accent.b));
    }

    // -- Traffic lights (FR-1, FR-3) ---------------------------------------

    function test_traffic_light_signals_and_roles() {
        var lights = make(lightsComponent, {});
        compare(lights.closeButton.Accessible.role, Accessible.Button);
        compare(lights.closeButton.Accessible.name, "Close");

        closeSpy.target = lights;
        closeSpy.clear();
        var d = Theme.controls.trafficLights.diameter;
        mouseClick(lights, d / 2, d / 2);
        compare(closeSpy.count, 1);
    }

    function test_traffic_light_visual_colors() {
        var lights = make(lightsComponent, {});
        var img = grabImage(lights);
        var d = Theme.controls.trafficLights.diameter;
        var gap = Theme.controls.trafficLights.gap;
        compare(img.red(d / 2, d / 2), channel(Theme.color.close.r));
        compare(img.red(d + gap + d / 2, d / 2), channel(Theme.color.minimize.r));
        compare(img.red(2 * (d + gap) + d / 2, d / 2), channel(Theme.color.zoom.r));
    }

    // -- Popup (FR-1, FR-5) -------------------------------------------------

    function test_popup_escape_closes() {
        var p = make(popupComponent, { open: true });
        compare(p.open, true);
        compare(p.Accessible.role, Accessible.Pane);
        p.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(p.open, false);
    }

    // -- MenuBarMenu (FR-1, FR-4) ------------------------------------------

    function test_menu_model_is_publishable() {
        var m = make(menuComponent, {
            title: "File",
            model: [
                { label: "New" },
                { type: "separator" },
                { label: "Show Sidebar", checked: true, checkable: true }
            ]
        });
        compare(m.menuModel.length, 3);
        compare(m.menuModel[0].label, "New");
        compare(m.menuModel[1].type, "separator");
        compare(m.menuModel[2].checked, true);
        compare(m.menuModel[2].checkable, true);
        verify(JSON.stringify(m.menuModel).length > 0);
    }

    function test_menu_keyboard_activation() {
        var m = make(menuComponent, {
            title: "File",
            model: [{ label: "First" }, { label: "Second" }]
        });
        triggeredSpy.target = m;
        triggeredSpy.clear();
        m.openMenu();
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        keyClick(Qt.Key_Return);
        compare(m.open, false);
        compare(triggeredSpy.count, 1);
        compare(triggeredSpy.signalArguments[0][0], 1);
    }

    // -- TitleBar intent ----------------------------------------------------

    function test_titlebar_emits_intent() {
        var bar = make(titleBarComponent, { width: 320, title: "T" });
        menuSpy.target = bar;
        menuSpy.clear();
        mouseClick(bar, 160, 20, Qt.RightButton);
        compare(menuSpy.count, 1);

        zoomSpy.target = bar;
        zoomSpy.clear();
        mouseDoubleClickSequence(bar, 160, 20);
        compare(zoomSpy.count, 1);
    }

    // -- FR-3: app TitleBar vs compositor SSD reference ---------------------

    function test_fr3_titlebar_matches_ssd_reference() {
        var bar = make(titleBarComponent, { width: 520, title: "Dragonfruit", active: true });
        var ref = make(ssdComponent, { width: 520, title: "Dragonfruit", active: true });
        var app = grabImage(bar);
        var ssd = grabImage(ref);
        compare(app.width, ssd.width);
        compare(app.height, ssd.height);
        // Prove the images are non-trivial (not two blank surfaces): the close
        // traffic light must be present at its token-derived position.
        var d = Theme.controls.trafficLights.diameter;
        var cy = Math.round(Theme.controls.titlebar.height / 2);
        compare(app.red(Theme.controls.trafficLights.inset + d / 2, cy),
                channel(Theme.color.close.r));
        compare(ssd.red(Theme.controls.trafficLights.inset + d / 2, cy),
                channel(Theme.color.close.r));
        verify(app.equals(ssd),
               "app TitleBar and compositor SSD reference must render identically at the same tokens");
    }

    // -- Button (FR-1) ------------------------------------------------------

    function test_button_keyboard_and_role() {
        var b = make(buttonComponent, { text: "Save" });
        compare(b.Accessible.role, Accessible.Button);
        compare(b.Accessible.name, "Save");
        buttonSpy.target = b;
        buttonSpy.clear();
        b.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Space);
        compare(buttonSpy.count, 1);
    }

    // -- Toolbar (FR-1) -----------------------------------------------------

    function test_toolbar_role_and_height() {
        var t = make(toolbarComponent, { width: 400, title: "Documents" });
        compare(t.Accessible.role, Accessible.ToolBar);
        compare(t.Accessible.name, "Documents");
        compare(t.implicitHeight, Theme.controls.toolbar.height);
    }

    // -- SplitView (FR-1) ---------------------------------------------------

    function test_splitview_keyboard_resizes() {
        var s = make(splitViewComponent, { width: 500, height: 200 });
        compare(s.divider.Accessible.role, Accessible.Splitter);
        var before = s.firstSize;
        s.divider.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Right);
        verify(s.firstSize > before, "Right arrow must widen the first pane");
        keyClick(Qt.Key_Home);
        compare(s.firstSize, s.minimumPane);
    }

    // -- SettingsRow / SettingsGroup (FR-1) ---------------------------------

    function test_settings_roles() {
        var row = make(settingsRowComponent, { label: "Wi-Fi" });
        compare(row.Accessible.role, Accessible.Grouping);
        compare(row.Accessible.name, "Wi-Fi");

        var group = make(settingsGroupComponent, { width: 300, title: "General" });
        compare(group.Accessible.role, Accessible.Grouping);
        compare(group.Accessible.name, "General");
    }

    // -- SegmentedControl (FR-1) -------------------------------------------

    function test_segmented_keyboard_selection() {
        var seg = make(segmentedComponent, { model: ["One", "Two", "Three"], currentIndex: 0 });
        compare(seg.Accessible.role, Accessible.Grouping);
        segmentedSpy.target = seg;
        segmentedSpy.clear();
        var first = seg.currentSegment;
        verify(first !== null, "the current segment must be instantiated");
        compare(first.Accessible.role, Accessible.RadioButton);
        first.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Right);
        compare(seg.currentIndex, 1);
        keyClick(Qt.Key_Return);
        compare(segmentedSpy.count, 1);
        compare(segmentedSpy.signalArguments[0][0], 1);
    }

    // -- ContextMenu (FR-1, FR-4) ------------------------------------------

    function test_context_menu_keyboard_activation() {
        var cm = make(contextMenuComponent, {
            model: [{ label: "One" }, { label: "Two" }]
        });
        compare(cm.Accessible.role, Accessible.PopupMenu);
        compare(cm.menuModel.length, 2);
        triggeredSpy.target = cm;
        triggeredSpy.clear();
        cm.showAt(10, 10);
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        keyClick(Qt.Key_Return);
        compare(cm.open, false);
        compare(triggeredSpy.count, 1);
        compare(triggeredSpy.signalArguments[0][0], 1);
    }

    function test_context_menu_keep_open_item_does_not_dismiss() {
        var cm = make(contextMenuComponent, {
            model: [{ label: "More", keepOpen: true }, { label: "Done" }]
        });
        triggeredSpy.target = cm;
        triggeredSpy.clear();
        cm.showAt(10, 10);
        waitForRendering(stage);
        cm.activate(0);
        compare(triggeredSpy.count, 1);
        compare(cm.open, true);
        cm.activate(1);
        compare(cm.open, false);
    }

    function test_context_menu_disabled_item_is_not_activated() {
        var cm = make(contextMenuComponent, {
            model: [{ label: "Empty Trash", enabled: false }]
        });
        triggeredSpy.target = cm;
        triggeredSpy.clear();
        cm.activate(0);
        compare(triggeredSpy.count, 0);
    }

    // -- SearchField (FR-1) -------------------------------------------------

    function test_search_field_escape_clears() {
        var sf = make(searchFieldComponent, { width: 200, text: "abc" });
        compare(sf.Accessible.role, Accessible.EditableText);
        compare(sf.Accessible.searchEdit, true);
        sf.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(sf.text, "");
    }

    // -- SourceList (FR-1) --------------------------------------------------

    function test_source_list_navigation_and_expansion() {
        var sl = make(sourceListComponent, {
            model: [
                { label: "A", hasChildren: true, expanded: false },
                { label: "B", depth: 1 },
                { label: "C" }
            ]
        });
        compare(sl.Accessible.role, Accessible.Tree);
        sl.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        compare(sl.currentIndex, 0);
        keyClick(Qt.Key_Right);
        verify(sl.isExpanded(0), "Right arrow must expand a collapsed branch");
        keyClick(Qt.Key_Right);
        compare(sl.currentIndex, 1);
    }

    // -- Sidebar (FR-1) -----------------------------------------------------

    function test_sidebar_navigation_and_role() {
        var sb = make(sidebarComponent, {
            sections: [{ title: "Favorites", items: [{ label: "One" }, { label: "Two" }] }]
        });
        compare(sb.Accessible.role, Accessible.List);
        sidebarSpy.target = sb;
        sidebarSpy.clear();
        sb.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Down);
        compare(sb.currentIndex, 1);
        keyClick(Qt.Key_Return);
        compare(sidebarSpy.count, 1);
    }

    // -- Dialog / Sheet (FR-1) ----------------------------------------------

    function test_dialog_accept_and_reject() {
        var d = make(dialogComponent, { open: true, title: "Replace?" });
        compare(d.Accessible.role, Accessible.Dialog);
        acceptedSpy.target = d;
        rejectedSpy.target = d;
        acceptedSpy.clear();
        rejectedSpy.clear();
        d.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(rejectedSpy.count, 1);
        compare(d.open, false);

        d.show();
        waitForRendering(stage);
        d.forceActiveFocus();
        keyClick(Qt.Key_Return);
        compare(acceptedSpy.count, 1);
    }

    function test_sheet_reject() {
        var s = make(sheetComponent, { open: true, title: "Go to Folder" });
        compare(s.Accessible.role, Accessible.Dialog);
        rejectedSpy.target = s;
        rejectedSpy.clear();
        s.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(rejectedSpy.count, 1);
        compare(s.open, false);
    }

    // -- Popover (FR-1) -----------------------------------------------------

    function test_popover_escape_closes() {
        var p = make(popoverComponent, { open: true });
        compare(p.Accessible.role, Accessible.Pane);
        p.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_Escape);
        compare(p.open, false);
    }

    // -- ScrollView (FR-1) --------------------------------------------------

    function test_scroll_view_keyboard_scroll() {
        var sv = make(scrollViewComponent, {});
        compare(sv.Accessible.role, Accessible.Pane);
        sv.forceActiveFocus();
        waitForRendering(stage);
        keyClick(Qt.Key_End);
        verify(sv.flickable.contentY > 0, "End must scroll to the bottom");
    }
}
}
