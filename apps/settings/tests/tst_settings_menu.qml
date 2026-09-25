// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings
import Dragonfruit.MenuBar

// Settings menu-model publication tests (T-09.6a).
//
// The Settings app publishes its native menu model through the
// `SettingsMenu` singleton. This suite asserts the model's shape and proves
// the headless round-trip the task's test plan names: the exact published
// model injected into the shell's `MenuBar` renders the Settings menus at the
// expected positions and routes an activated row's `action`.
Item {
    id: stage
    width: 1200
    height: 320

    TestCase {
        id: testCase
        name: "SettingsMenu"
        when: windowShown

        Component { id: barComponent; MenuBar { } }
        Component { id: menuComponent; MenuBarMenu { } }

        SignalSpy { id: triggeredSpy; signalName: "appMenuTriggered" }
        SignalSpy { id: activatedSpy; signalName: "activated" }

        function make(component, props) {
            var obj = createTemporaryObject(component, stage, props || {});
            waitForRendering(stage);
            return obj;
        }

        // -- The published model -------------------------------------------

        function test_application_menu_is_published() {
            var app = SettingsMenu.applicationMenuItems;
            compare(app.length, 8);
            compare(app[0].label, "About Settings");
            compare(app[0].action, "about");
            compare(app[2].type, "separator");
            compare(app[3].label, "Hide Settings");
            compare(app[3].shortcut, "Super+H");
            compare(app[7].label, "Quit Settings");
            compare(app[7].action, "quit");
        }

        function test_app_menus_are_published() {
            var menus = SettingsMenu.menus;
            compare(menus.length, 5);
            compare(menus[0].title, "File");
            compare(menus[1].title, "Edit");
            compare(menus[2].title, "View");
            compare(menus[3].title, "Window");
            compare(menus[4].title, "Help");

            // File carries the real window verb.
            compare(menus[0].items.length, 1);
            compare(menus[0].items[0].label, "Close Window");
            compare(menus[0].items[0].shortcut, "Super+W");
            compare(menus[0].items[0].action, "close");

            // Edit is the standard set with a separator.
            compare(menus[1].items.length, 7);
            compare(menus[1].items[2].type, "separator");
            compare(SettingsMenu.actionFor(1, 6), "edit.select-all");

            // View carries a nested Settings Pane submenu.
            var view = menus[2].items;
            compare(view[1].type, "separator");
            compare(view[2].type, "submenu");
            compare(view[2].submenu.length, 4);
            compare(view[2].submenu[0].action, "pane.appearance");
            compare(view[2].submenu[3].action, "pane.wallpaper");
        }

        function test_published_model_is_json_serializable() {
            var json = JSON.stringify(SettingsMenu.publishedModel);
            verify(json.length > 0);
            var parsed = JSON.parse(json);
            compare(parsed.appName, "Settings");
            compare(parsed.menus.length, 5);
            compare(parsed.applicationMenuItems.length, 8);
            // No QML objects leak into the published form.
            compare(typeof parsed.menus[0].items[0].label, "string");
        }

        function test_action_for_follows_the_triggered_contract() {
            compare(SettingsMenu.actionFor(0, 0), "close");
            compare(SettingsMenu.actionFor(2, 0), "fullscreen");
            compare(SettingsMenu.actionFor(9, 0), "");
            compare(SettingsMenu.actionFor(0, 9), "");
        }

        function test_activate_emits_the_action() {
            activatedSpy.target = SettingsMenu;
            activatedSpy.clear();
            compare(SettingsMenu.activate("close"), true);
            compare(activatedSpy.count, 1);
            compare(activatedSpy.signalArguments[0][0], "close");
            // An empty action is inert, not a broadcast.
            compare(SettingsMenu.activate(""), false);
            compare(activatedSpy.count, 1);
        }

        // -- Round-trip to the shell's bar ----------------------------------

        function test_published_model_round_trips_to_the_bar() {
            var bar = make(barComponent, {
                width: 1000,
                appName: SettingsMenu.appName,
                applicationMenuItems: SettingsMenu.applicationMenuItems,
                appMenuModel: SettingsMenu.menus
            });
            // Fixed system + application menus stay 0 and 1; the app's own
            // published menus follow unchanged.
            compare(bar.appMenuAt(0).showLogo, true);
            compare(bar.appMenuAt(1).emphasized, true);
            compare(bar.appMenuAt(1).title, "Settings");
            compare(bar.appMenuAt(2).title, "File");
            compare(bar.appMenuAt(3).title, "Edit");
            compare(bar.appMenuAt(4).title, "View");
            compare(bar.appMenuAt(5).title, "Window");
            compare(bar.appMenuAt(6).title, "Help");
            compare(bar.appMenuAt(7), null);
            // The row content survives the trip.
            compare(bar.appMenuAt(2).model[0].label, "Close Window");
            compare(bar.appMenuAt(2).model[0].shortcut, "Super+W");
        }

        function test_bar_routes_a_published_action() {
            var bar = make(barComponent, {
                width: 1000,
                appName: SettingsMenu.appName,
                applicationMenuItems: SettingsMenu.applicationMenuItems,
                appMenuModel: SettingsMenu.menus
            });
            triggeredSpy.target = bar;
            triggeredSpy.clear();

            // File is top-level index 2; its first row carries the action.
            bar.openMenu(2);
            waitForRendering(stage);
            bar.appMenuAt(2).activate(0);
            waitForRendering(stage);
            compare(triggeredSpy.count, 1);
            compare(triggeredSpy.signalArguments[0][0], 2);
            compare(triggeredSpy.signalArguments[0][1], 0);
            compare(triggeredSpy.signalArguments[0][2].action, "close");
        }

        function test_published_menus_normalize_through_menu_bar_menu() {
            // The published entry shape is the design system's input shape: a
            // `MenuBarMenu` built from it re-emits an equivalent `menuModel`.
            var file = make(menuComponent, {
                title: SettingsMenu.menus[0].title,
                model: SettingsMenu.menus[0].items
            });
            compare(file.menuModel.length, 1);
            compare(file.menuModel[0].label, "Close Window");
            compare(file.menuModel[0].shortcut, "Super+W");
            compare(file.menuModel[0].type, "item");
            compare(file.menuModel[0].hasSubmenu, false);

            var view = make(menuComponent, {
                title: SettingsMenu.menus[2].title,
                model: SettingsMenu.menus[2].items
            });
            compare(view.menuModel.length, 3);
            compare(view.menuModel[2].type, "submenu");
            compare(view.menuModel[2].hasSubmenu, true);
            compare(view.entries[2].submenu.length, 4);
            // JSON-serializable with no QML objects.
            verify(JSON.stringify(view.menuModel).length > 0);
        }
    }
}