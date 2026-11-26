// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T276 Desktop & Dock / Overview headless visual check (not part of ctest):
// the Dock, Desktop & Stage, Widgets, Windows, default-browser, Overview, and
// Hot Corners groups at the measured 740×820 window. Run through
// `scripts/capture-t276-desktop-dock.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT276DesktopDockCapture"
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

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            var shell = make();

            var pane = showPane(shell, "desktop-dock");
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            save(shell, "t276-desktop-dock-dock-" + scheme);

            scrollTo(shell, pane.desktopGroup);
            save(shell, "t276-desktop-dock-desktop-stage-" + scheme);

            scrollTo(shell, pane.widgetsGroup);
            save(shell, "t276-desktop-dock-widgets-" + scheme);

            scrollTo(shell, pane.windowsGroup);
            save(shell, "t276-desktop-dock-windows-" + scheme);

            scrollTo(shell, pane.missionControlGroup);
            save(shell, "t276-desktop-dock-default-browser-" + scheme);

            // T292b: the former Overview pane is merged into Desktop & Dock ->
            // Workspace Overview; reaching it lands on the merged section, whose
            // corner controls live in the Advanced Workspace Overview sheet.
            shell.deepLink("settings://pane/mission-control");
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            waitForRendering(stage);
            save(shell, "t276-desktop-dock-overview-" + scheme);

            var overview = shell.paneBody.item;
            overview.openAdvancedOverview();
            tryVerify(function() { return overview.advancedOverviewDialog.open; });
            for (var f = 0; f < 8; ++f)
                waitForRendering(stage);
            save(shell, "t276-desktop-dock-hot-corners-" + scheme);
            overview.closeAdvancedOverview();
            waitForRendering(stage);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
