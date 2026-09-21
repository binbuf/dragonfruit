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
    }
}
