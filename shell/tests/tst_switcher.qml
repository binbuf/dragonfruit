// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Switcher

// App-switcher overlay chrome tests (T-06.2a): one card per app in recency
// order, the selection highlight, the accessible names, and the reduced-motion
// variant. Runs headless on the offscreen platform + software scene graph; the
// compositor half (the live previews) is covered by the compositor
// conformance suite.
Item {
    id: stage
    width: 1280
    height: 720

    TestCase {
        id: testCase
        name: "AppSwitcher"
        when: windowShown

        Component { id: switcherComponent; AppSwitcher { } }

        function make(props) {
            var merged = { width: stage.width, height: stage.height };
            for (var key in (props || {}))
                merged[key] = props[key];
            var obj = createTemporaryObject(switcherComponent, stage, merged);
            waitForRendering(stage);
            return obj;
        }

        function recency() {
            return [
                { index: 0, appId: "org.example.Mail" },
                { index: 1, appId: "org.example.Editor" },
                { index: 2, appId: "org.example.Files" }
            ];
        }

        function cards(switcher) {
            var row = findChild(switcher, "appCardRow");
            var out = [];
            for (var i = 0; i < row.children.length; ++i) {
                if (row.children[i].objectName === "appCard")
                    out.push(row.children[i]);
            }
            return out;
        }

        function test_renders_one_card_per_app_in_recency_order() {
            var switcher = make({ entries: recency(), selectedIndex: 0, active: true });
            var list = cards(switcher);
            compare(list.length, 3);
            compare(findChild(list[0], "appName").text, "Mail");
            compare(findChild(list[1], "appName").text, "Editor");
            compare(findChild(list[2], "appName").text, "Files");
        }

        function test_the_selected_card_is_highlighted() {
            var switcher = make({ entries: recency(), selectedIndex: 1, active: true });
            var list = cards(switcher);
            verify(list[1].selected, "the selected card is marked");
            verify(!list[0].selected);
            verify(!list[2].selected);
            compare(findChild(switcher, "selectedName").text, "Editor");
            compare(list[1].border.width, 2);
            compare(list[0].border.width, 1);
        }

        function test_cards_carry_accessible_names() {
            var switcher = make({ entries: recency(), selectedIndex: 0, active: true });
            var list = cards(switcher);
            verify(list[0].Accessible.name.indexOf("Mail") >= 0);
            verify(list[0].Accessible.focusable);
            verify(list[0].Accessible.selected);
        }

        function test_scrim_and_cards_fade_with_reveal() {
            var switcher = make({ entries: recency(), selectedIndex: 0, active: false });
            compare(switcher.reveal, 0);
            compare(findChild(switcher, "switcherScrim").opacity, 0);
            switcher.active = true;
            tryVerify(function() {
                return switcher.reveal === 1;
            }, 2000);
            compare(findChild(switcher, "switcherScrim").opacity,
                    Theme.controls.overview.scrimOpacity);
            compare(findChild(switcher, "appCardRow").opacity, 1);
        }

        // Reduced motion appears instantly at the final state, with no
        // dependence on the animation.
        function test_reduced_motion_appears_without_animating() {
            var previous = Theme.reducedMotion;
            Theme.reducedMotion = true;
            var switcher = make({ entries: recency(), selectedIndex: 0, active: true });
            compare(switcher.reveal, 1, "reduced motion ignores the fade");
            compare(findChild(switcher, "appCardRow").opacity, 1);
            switcher.active = false;
            waitForRendering(stage);
            compare(switcher.reveal, 0, "hidden instantly too");
            Theme.reducedMotion = previous;
        }

        function test_empty_entries_render_no_cards() {
            var switcher = make({ entries: [], selectedIndex: -1, active: true });
            compare(cards(switcher).length, 0);
        }
    }
}