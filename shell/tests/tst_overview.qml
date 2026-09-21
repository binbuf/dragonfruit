// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Overview

// Mission Control overview chrome tests (T-11 Slice B): the workspace strip
// (including fullscreen Spaces), the minimized-window bottom strip, the
// selection signals, and the shared progress fade. Runs headless on the
// offscreen platform + software scene graph.
Item {
    id: stage
    width: 1280
    height: 720

    TestCase {
        id: testCase
        name: "Overview"
        when: windowShown

        Component { id: overviewComponent; Overview { } }

        SignalSpy { id: workspaceSpy; signalName: "workspaceActivated" }
        SignalSpy { id: windowSpy; signalName: "windowActivated" }
        SignalSpy { id: moveSpy; signalName: "windowMovedToWorkspace" }
        SignalSpy { id: dismissSpy; signalName: "dismissRequested" }

        function make(props) {
            var merged = { width: stage.width, height: stage.height };
            for (var key in (props || {}))
                merged[key] = props[key];
            var obj = createTemporaryObject(overviewComponent, stage, merged);
            waitForRendering(stage);
            return obj;
        }

        function spaces() {
            return [
                { index: 0, name: "Desktop 1", fullscreen: false, active: true },
                { index: 1, name: "Desktop 2", fullscreen: false, active: false },
                { index: 2, name: "Fullscreen", fullscreen: true, active: false }
            ];
        }

        function minimized() {
            return [
                { windowId: "123", title: "Document", appId: "org.example.Editor" },
                { windowId: "456", title: "Mail", appId: "org.example.Mail" }
            ];
        }

        function windows() {
            return [
                { windowId: "123", title: "Document", appId: "org.example.Editor",
                  workspaceIndex: 0, workspaceName: "Desktop 1", focused: true },
                { windowId: "456", title: "Mail", appId: "org.example.Mail",
                  workspaceIndex: 1, workspaceName: "Desktop 2", focused: false }
            ];
        }

        function cards(overview) {
            var strip = findChild(overview, "workspaceStrip");
            var out = [];
            for (var i = 0; i < strip.children.length; ++i) {
                if (strip.children[i].objectName === "spaceCard")
                    out.push(strip.children[i]);
            }
            return out;
        }

        function chips(overview) {
            var strip = findChild(overview, "minimizedStrip");
            var out = [];
            for (var i = 0; i < strip.children.length; ++i) {
                if (strip.children[i].objectName === "windowChip")
                    out.push(strip.children[i]);
            }
            return out;
        }

        function windowCards(overview) {
            var grid = findChild(overview, "windowGrid");
            var out = [];
            for (var i = 0; i < grid.children.length; ++i) {
                if (grid.children[i].objectName === "windowCard")
                    out.push(grid.children[i]);
            }
            return out;
        }

        function test_workspace_strip_renders_one_card_per_space() {
            var overview = make({ workspaces: spaces(), progress: 1, active: true });
            var list = cards(overview);
            compare(list.length, 3);
            compare(findChild(list[0], "spaceName").text, "Desktop 1");
            compare(findChild(list[2], "spaceName").text, "Fullscreen");
        }

        function test_active_and_fullscreen_spaces_are_marked() {
            var overview = make({ workspaces: spaces(), progress: 1, active: true });
            var list = cards(overview);
            verify(list[0].spaceActive, "the active Space is highlighted");
            verify(!list[1].spaceActive);
            verify(list[2].fullscreen, "the fullscreen Space is marked");
            var badge = findChild(list[2], "fullscreenBadge");
            verify(badge && badge.visible);
        }

        function test_clicking_a_workspace_activates_it() {
            var overview = make({ workspaces: spaces(), progress: 1, active: true });
            workspaceSpy.target = overview;
            workspaceSpy.clear();
            var list = cards(overview);
            mouseClick(list[1], list[1].width / 2, list[1].height / 2);
            compare(workspaceSpy.count, 1);
            compare(workspaceSpy.signalArguments[0][0], 1);
        }

        function test_minimized_strip_renders_and_restores_on_click() {
            var overview = make({ minimizedWindows: minimized(), progress: 1, active: true });
            var list = chips(overview);
            compare(list.length, 2);
            windowSpy.target = overview;
            windowSpy.clear();
            mouseClick(list[0], list[0].width / 2, list[0].height / 2);
            compare(windowSpy.count, 1);
            compare(windowSpy.signalArguments[0][0], "123");
        }

        function test_escape_requests_dismissal() {
            var overview = make({ workspaces: spaces(), progress: 1, active: true });
            overview.forceActiveFocus();
            dismissSpy.target = overview;
            dismissSpy.clear();
            keyClick(Qt.Key_Escape);
            compare(dismissSpy.count, 1);
        }

        function test_strips_fade_with_the_shared_progress() {
            var overview = make({ workspaces: spaces(), progress: 0, active: true });
            var strip = findChild(overview, "workspaceStrip");
            compare(strip.opacity, 0);
            overview.progress = 1;
            waitForRendering(stage);
            compare(strip.opacity, 1);
        }

        function test_window_grid_renders_one_card_per_visible_window() {
            var overview = make({ windows: windows(), progress: 1, active: true });
            var list = windowCards(overview);
            compare(list.length, 2);
            compare(findChild(list[0], "windowTitle").text, "Document");
            compare(findChild(list[1], "windowTitle").text, "Mail");
            compare(findChild(list[1], "windowSpace").text, "Desktop 2");
        }

        function test_clicking_a_window_selects_it() {
            var overview = make({ windows: windows(), progress: 1, active: true });
            var list = windowCards(overview);
            windowSpy.target = overview;
            windowSpy.clear();
            mouseClick(list[0], list[0].width / 2, list[0].height / 2);
            compare(windowSpy.count, 1);
            compare(windowSpy.signalArguments[0][0], "123");
        }

        function test_dragging_a_window_onto_a_space_moves_it() {
            var overview = make({ workspaces: spaces(), windows: windows(),
                                  progress: 1, active: true });
            var spaceCards = cards(overview);
            moveSpy.target = overview;
            moveSpy.clear();
            overview.beginWindowDrag("123");
            compare(overview.draggingWindowId, "123");
            var target = spaceCards[1];
            var center = target.mapToItem(null, target.width / 2, target.height / 2);
            overview.updateWindowDrag("123", center.x, center.y);
            compare(overview.dragWorkspaceIndex, 1,
                    "the Space card under the pointer is the drop target");
            verify(target.dropTarget, "the target card is highlighted");
            overview.dropWindow("123");
            compare(overview.draggingWindowId, "");
            compare(moveSpy.count, 1);
            compare(moveSpy.signalArguments[0][0], "123");
            compare(moveSpy.signalArguments[0][1], 1);
        }

        function test_dropping_a_window_outside_any_space_does_not_move_it() {
            var overview = make({ workspaces: spaces(), windows: windows(),
                                  progress: 1, active: true });
            moveSpy.target = overview;
            moveSpy.clear();
            overview.beginWindowDrag("123");
            overview.updateWindowDrag("123", 5, 700);
            compare(overview.dragWorkspaceIndex, -1);
            overview.dropWindow("123");
            compare(moveSpy.count, 0, "no Space under the pointer means no move");
            compare(overview.draggingWindowId, "");
        }
    }
}
