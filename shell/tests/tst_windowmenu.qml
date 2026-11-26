// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.WindowMenu

// T329: the titlebar window menu is readable. The compositor owns the model
// and input; this view test pins the labels, the Tabler icons, the highlight,
// and the Move to Space submenu card.
Item {
    id: stage
    width: 480
    height: 320

    TestCase {
        id: testCase
        name: "WindowMenu"
        when: windowShown

        Component { id: menuComponent; WindowMenu { } }

        readonly property var rows: [
            { index: 0, label: "Move to Space", icon: "layout-grid" },
            { index: 1, label: "Minimize", icon: "minus" },
            { index: 2, label: "Zoom", icon: "maximize" },
            { index: 3, label: "Close", icon: "x" }
        ]
        readonly property var spaces: [
            { index: 0, label: "Space 1", icon: "layout-grid" },
            { index: 1, label: "Space 2", icon: "layout-grid" }
        ]

        function make(props) {
            var menu = createTemporaryObject(menuComponent, stage, props || {});
            waitForRendering(stage);
            return menu;
        }

        function rowLabel(menu, index) {
            return findChild(findChild(menu, "windowMenuRow" + index), "menuRowLabel").text;
        }

        function rowIcon(menu, index) {
            return findChild(findChild(menu, "windowMenuRow" + index), "menuRowIcon").name;
        }

        function test_every_row_has_a_readable_label_and_icon() {
            var menu = make({
                rows: testCase.rows,
                spaces: testCase.spaces,
                panelWidth: 180,
                panelHeight: 2 * 6 + 4 * 26
            });
            var labels = [];
            var icons = [];
            for (var i = 0; i < 4; ++i) {
                labels.push(rowLabel(menu, i));
                icons.push(rowIcon(menu, i));
            }
            compare(labels, ["Move to Space", "Minimize", "Zoom", "Close"]);
            compare(icons, ["layout-grid", "minus", "maximize", "x"]);
            compare(findChild(menu, "windowMenuRow0").entry.type, "submenu",
                    "Move to Space is the submenu opener");
        }

        function test_highlight_is_the_compositor_state() {
            var menu = make({
                rows: testCase.rows,
                panelWidth: 180,
                panelHeight: 2 * 6 + 4 * 26,
                highlightedIndex: 2
            });
            verify(findChild(menu, "windowMenuRow2").highlighted,
                   "the compositor's highlighted row is the one drawn");
            verify(!findChild(menu, "windowMenuRow0").highlighted);
        }

        function test_the_move_to_space_submenu_shows_the_space_names() {
            var menu = make({
                rows: testCase.rows,
                spaces: testCase.spaces,
                panelWidth: 180,
                panelHeight: 2 * 6 + 4 * 26,
                submenuOpen: true,
                submenuX: 186,
                submenuY: 6,
                submenuWidth: 180,
                submenuHeight: 2 * 6 + 2 * 26,
                submenuHighlightedIndex: 1
            });
            verify(findChild(menu, "windowMenuSubmenu").visible);
            compare(findChild(findChild(menu, "windowMenuSpace0"), "menuRowLabel").text,
                    "Space 1");
            compare(findChild(findChild(menu, "windowMenuSpace1"), "menuRowLabel").text,
                    "Space 2");
            compare(findChild(findChild(menu, "windowMenuSpace1"), "menuRowIcon").name,
                    "layout-grid");
            verify(findChild(menu, "windowMenuSpace1").highlighted);
        }

        function test_material_regions_are_the_two_detached_cards() {
            var menu = make({
                rows: testCase.rows,
                spaces: testCase.spaces,
                panelX: 10,
                panelY: 20,
                panelWidth: 180,
                panelHeight: 116,
                submenuOpen: true,
                submenuX: 196,
                submenuY: 26,
                submenuWidth: 180,
                submenuHeight: 64
            });
            compare(menu.regions.length, 2);
            compare(menu.regions[0].key, "panel");
            compare(menu.regions[1].key, "submenu");
            compare(menu.regions[1].x, 196);
            verify(menu.regions[0].w > 0 && menu.regions[0].h > 0);
        }

        function test_scheme_and_accessible_name() {
            var menu = make({
                rows: testCase.rows,
                panelWidth: 180,
                panelHeight: 116,
                scheme: 0
            });
            compare(menu.scheme, 0);
            compare(menu.Accessible.name, "Window menu");
        }
    }
}
