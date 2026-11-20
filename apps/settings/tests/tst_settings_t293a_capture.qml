// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T293a Menu Bar visual check (not part of ctest): the restyled pane at the
// measured 740×820 window in light and dark —
//
//   * the main pane in the mockup order (Behaviour, OS Menu Glyph, Menu Bar
//     Controls, Allow in the Menu Bar, footer);
//   * the OS Menu Glyph radio group with both actual-size previews;
//   * the controls list with the options rows and the disabled Display row;
//   * the third-party StatusNotifier list;
//   * the footer with Add Controls…, Customize Controls…, Advanced…, and
//     Reset Menu Bar…;
//   * the Advanced Menu Bar sheet;
//   * the Add Controls sheet.
//
// Six interactive surfaces per scheme at most. Run through
// `scripts/capture-t293a-settings-menu-bar.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT293aMenuBarCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; pin every key this pane writes
        // so a still never inherits the previous scheme's state.
        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("menu.autoHide", "full-screen");
            Settings.set("menu.showBackground", true);
            Settings.set("menu.recentItems", 10);
            Settings.set("menu.osGlyph", "dragonfruit");
            Settings.set("menu.clock.showDate", true);
            Settings.set("menu.clock.showSeconds", false);
            Settings.set("menu.control.clock", true);
            Settings.set("menu.control.search", true);
            Settings.set("menu.control.wifi", true);
            Settings.set("menu.control.bluetooth", true);
            Settings.set("menu.control.battery", true);
            Settings.set("menu.control.focus", true);
            Settings.set("menu.control.volume", true);
            Settings.set("menu.control.accessibility", true);
            Settings.set("menu.control.controlCenter", true);
            Settings.set("menu.control.workspaces", true);
            Settings.set("menu.control.focusAlways", false);
            Settings.set("menu.control.volumeWhenActive", false);
            Settings.set("power.showPercentage", true);
            Settings.set("menu.overflow", "automatic");
            Settings.set("menu.global", true);
            Settings.set("quickSettings.perDisplay", false);
            Settings.setList("menu.tray.hidden", []);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, paneId) {
            shell.selectPane(paneId);
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.paneBody.item !== null;
            });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function scrollTo(shell, item) {
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            tryVerify(function() { return shell.detailScroll.maxY >= 0; });
            if (!item) {
                waitForRendering(stage);
                return;
            }
            var y = item.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, Math.min(
                    shell.detailScroll.maxY, y - 12));
            waitForRendering(stage);
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        // A Dialog adopts the window overlay layer, then fades its scrim and
        // scales its card in with Motion Behaviors; grab a few frames after
        // opening so a still never captures a half-faded dialog.
        function openSheet(dialog) {
            dialog.show();
            tryVerify(function() { return dialog.open; });
            tryVerify(function() {
                return dialog.width > 64 && dialog.height > 64;
            }, 5000);
            for (var i = 0; i < 8; ++i)
                waitForRendering(stage);
        }

        function capture(scheme) {
            resetState(scheme);
            var shell = make();
            var pane = showPane(shell, "menu-bar");
            for (var f = 0; f < 4; ++f)
                waitForRendering(stage);

            // 1. The main pane: Behaviour, OS Menu Glyph, Controls.
            scrollTo(shell, pane.behaviorGroup);
            save(shell, "t293a-menu-bar-main-" + scheme);

            // 2. The OS glyph radio group with both real-size previews. The
            // second row is landed on so the group and its selected state read
            // together. A hovered sample shows the open plate.
            scrollTo(shell, pane.glyphGroup);
            for (var g = 0; g < 4; ++g)
                waitForRendering(stage);
            save(shell, "t293a-menu-bar-os-glyph-" + scheme);

            // 3. The controls list (Clock + Search … Workspaces, Display).
            scrollTo(shell, pane.controlsGroup);
            save(shell, "t293a-menu-bar-controls-" + scheme);

            // 4. The third-party list and the footer actions.
            scrollTo(shell, pane.footer);
            save(shell, "t293a-menu-bar-footer-" + scheme);

            // 5. Reset in effect: Tux chosen, then the footer's Reset Menu
            // Bar… restores Dragonfruit and every default.
            Settings.set("menu.osGlyph", "tux");
            Settings.set("menu.control.wifi", false);
            Settings.set("menu.autoHide", "never");
            waitForRendering(stage);
            pane.resetButton.clicked();
            compare(Settings.values["menu.osGlyph"], "dragonfruit");
            scrollTo(shell, pane.glyphGroup);
            save(shell, "t293a-menu-bar-reset-" + scheme);

            // 6. The Advanced Menu Bar sheet: the three Linux-only rows.
            pane.openAdvancedMenuBar();
            for (var a = 0; a < 8; ++a)
                waitForRendering(stage);
            save(shell, "t293a-menu-bar-advanced-" + scheme);
            pane.closeAdvancedMenuBar();
            waitForRendering(stage);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
