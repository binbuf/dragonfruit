// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T227 absence capture (not part of ctest): run with no settingsd on the
// private bus. The panes stay live on the schema defaults; the Search
// indexing rows and the Displays capability rows show their reasons.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsPanesAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width,
                                                height: stage.height });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell.paneBody.item;
        }

        function test_capture_absence() {
            compare(Settings.available, false);
            var shell = make();

            var search = showPane(shell, "search");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-search-absence.png");

            var displays = showPane(shell, "displays");
            displays.advancedPopup.show();
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-displays-absence.png");
            displays.advancedPopup.hide();
            wait(250);

            shell.selectPane("menu-bar");
            var menuBar = showPane(shell, "menu-bar");
            shell.openSubPane("controls");
            tryVerify(function() {
                return menuBar.subPaneLoader.status === Loader.Ready
                        && menuBar.subPaneLoader.item !== null;
            });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-quick-settings-absence.png");
        }
    }
}
