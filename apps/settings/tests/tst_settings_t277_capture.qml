// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T277 Quick Settings and Menu Bar headless visual check (not part of ctest):
// the categorized modules, pages, live preview, Add Controls sheet, Reset,
// menu-bar controls, per-control options, third-party StatusNotifier list, and
// the Linux-only rows at the measured 740×820 window. Run through
// `scripts/capture-t277-quick-settings-menu-bar.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT277QuickSettingsMenuBarCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, paneId) {
            shell.selectPane(paneId);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function scrollTo(shell, item) {
            var y = item.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            waitForRendering(stage);
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        function prepare() {
            Settings.setList("quickSettings.modules", [
                "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
                "screen-mirroring", "dark", "capture-screen", "focus",
                "brightness", "volume", "airplay-audio", "edit-controls"
            ]);
            Settings.setList("quickSettings.menuBarOnly", ["bluetooth"]);
            Settings.setList("quickSettings.page2",
                             ["storage", "keyboard", "mission-control"]);
            Settings.setList("quickSettings.page3", []);
            Settings.setList("quickSettings.pageNames", ["Home", "Work"]);
            Settings.set("quickSettings.pageCount", 2);
            Settings.set("quickSettings.activePage", 0);
            Settings.setList("quickSettings.spans", ["wifi=2x2", "focus=1x1"]);
            Settings.set("quickSettings.perDisplay", false);
            Settings.set("menu.overflow", "automatic");
            Settings.set("menu.control.workspaces", true);
            Settings.set("menu.control.focusAlways", false);
            Settings.set("menu.control.volumeWhenActive", false);
            Settings.set("power.showPercentage", true);
            Settings.setList("menu.tray.hidden", []);
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            prepare();
            var shell = make();

            var menuBar = showPane(shell, "menu-bar");
            shell.openSubPane("controls");
            tryVerify(function() {
                return menuBar.subPaneLoader.status === Loader.Ready
                        && menuBar.subPaneLoader.item !== null;
            });
            var quick = menuBar.subPaneLoader.item;
            verify(quick);
            scrollTo(shell, quick.modulesGroup);
            save(shell, "t277-quick-settings-modules-" + scheme);

            scrollTo(shell, quick.pagesGroup);
            save(shell, "t277-quick-settings-pages-" + scheme);

            scrollTo(shell, quick.previewGroup);
            save(shell, "t277-quick-settings-preview-" + scheme);

            quick.addControlsDialog.show();
            waitForRendering(stage);
            verify(quick.addControlsDialog.open);
            save(shell, "t277-quick-settings-add-controls-" + scheme);
            quick.addControlsDialog.accept();
            waitForRendering(stage);

            scrollTo(shell, quick.resetGroup);
            save(shell, "t277-quick-settings-reset-" + scheme);
            quick.resetButton.clicked();
            scrollTo(shell, quick.modulesGroup);
            save(shell, "t277-quick-settings-reset-restored-" + scheme);

            // Menu Bar: the controls, the third-party list, the options sheet,
            // and the Linux-only rows/reset.
            var menu = showPane(shell, "menu-bar");
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            save(shell, "t277-menu-bar-controls-" + scheme);

            menu.controlOptionsDialog.optionKind = "battery";
            menu.controlOptionsDialog.show();
            waitForRendering(stage);
            verify(menu.controlOptionsDialog.open);
            save(shell, "t277-menu-bar-options-" + scheme);
            menu.controlOptionsDialog.accept();
            waitForRendering(stage);

            scrollTo(shell, menu.thirdPartyGroup);
            save(shell, "t277-menu-bar-third-party-" + scheme);

            scrollTo(shell, menu.resetButton);
            save(shell, "t277-menu-bar-reset-" + scheme);
            menu.resetButton.clicked();
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            save(shell, "t277-menu-bar-reset-restored-" + scheme);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
