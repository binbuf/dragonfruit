// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings
import Dragonfruit.ControlCenter

// T293b Menu Bar Controls visual check (not part of ctest): the in-place
// sub-pane reached from Menu Bar -> Customize Controls… at the measured
// 740×820 window, in light and dark. Six surfaces per scheme at most:
//
//   * the Menu Bar list with the footer actions (the entry point);
//   * the Menu Bar Controls sub-pane opened in place (Pages + Modules);
//   * the Modules/Control sizes section;
//   * the live preview and Menu Bar Only section;
//   * the Add Controls sheet;
//   * the Reset result.
//
// Run through `scripts/capture-t293b-settings-menu-bar-controls.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT293bMenuBarControlsCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; pin every key the sub-pane
        // writes so a still never inherits the previous scheme's state.
        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.setList("quickSettings.modules", [
                "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
                "screen-mirroring", "dark", "capture-screen", "focus",
                "brightness", "volume", "airplay-audio", "edit-controls"
            ]);
            Settings.setList("quickSettings.menuBarOnly", ["bluetooth"]);
            Settings.setList("quickSettings.page2",
                             ["focus", "brightness", "volume"]);
            Settings.setList("quickSettings.page3", []);
            Settings.setList("quickSettings.pageNames", ["Home", "Work"]);
            Settings.set("quickSettings.pageCount", 2);
            Settings.set("quickSettings.activePage", 0);
            Settings.setList("quickSettings.spans", ["wifi=2x2", "focus=1x1"]);
            Settings.set("quickSettings.perDisplay", false);
            Settings.set("menu.control.controlCenter", true);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            verify(shell);
            shell.selectPane("menu-bar");
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.paneBody.item !== null;
            });
            waitForRendering(stage);
            return shell;
        }

        function openControls(shell) {
            var list = shell.paneBody.item;
            list.customizeControlsButton.clicked();
            tryVerify(function() {
                return list.subPaneLoader.status === Loader.Ready
                        && list.subPaneLoader.item !== null;
            });
            for (var f = 0; f < 4; ++f)
                waitForRendering(stage);
            return list.subPaneLoader.item;
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

        // A Dialog adopts the overlay layer and fades in; grab after a few
        // frames so a still never captures a half-faded sheet.
        function openSheet(dialog) {
            dialog.show();
            tryVerify(function() { return dialog.open; });
            tryVerify(function() {
                return dialog.width > 64 && dialog.height > 64;
            });
            for (var i = 0; i < 8; ++i)
                waitForRendering(stage);
        }

        function capture(scheme) {
            resetState(scheme);
            var shell = make();
            var list = shell.paneBody.item;

            // 1. The entry point: the Menu Bar list's footer.
            scrollTo(shell, list.footer);
            save(shell, "t293b-menu-bar-controls-entry-" + scheme);

            // 2. The sub-pane open in place (Pages + the grouped Modules).
            var body = openControls(shell);
            verify(body);
            scrollTo(shell, body.pagesGroup);
            save(shell, "t293b-menu-bar-controls-pages-" + scheme);

            // 3. The Modules section with the category filter and sizes.
            scrollTo(shell, body.modulesGroup);
            save(shell, "t293b-menu-bar-controls-modules-" + scheme);

            // 4. The live preview and Menu Bar Only section.
            scrollTo(shell, body.previewGroup);
            save(shell, "t293b-menu-bar-controls-preview-" + scheme);

            // 5. The Add Controls sheet (the registry catalogue).
            openSheet(body.addControlsDialog);
            save(shell, "t293b-menu-bar-controls-add-" + scheme);
            body.addControlsDialog.accept();
            waitForRendering(stage);

            // 6. Reset in effect: one page, default modules, no spans.
            body.addPage();
            body.renamePage(1, "Live");
            body.cycleSpan("focus");
            body.resetButton.clicked();
            tryVerify(function() { return body.pageCount === 1; });
            scrollTo(shell, body.pagesGroup);
            save(shell, "t293b-menu-bar-controls-reset-" + scheme);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
