// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T276 absence capture (not part of ctest): the Desktop & Dock and Overview
// panes with no settings daemon. Run through
// `scripts/capture-t276-desktop-dock.sh` with no settings fixture.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT276DesktopDockAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("desktop-dock");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var pane = shell.paneBody.item;
            verify(pane);
            compare(Settings.available, false);
            grabImage(shell).save("docs/captures/t276-desktop-dock-absence-"
                                  + scheme + ".png");

            // T292b: the former Overview pane is merged into Desktop & Dock ->
            // Workspace Overview; the legacy deep link lands on the section,
            // which shows its own absence note on the built-in defaults.
            shell.deepLink("settings://pane/mission-control");
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            waitForRendering(stage);
            var overview = shell.paneBody.item;
            verify(overview);
            compare(overview.overviewAbsenceNote.visible, true);
            for (var f = 0; f < 8; ++f)
                waitForRendering(stage);
            grabImage(shell).save("docs/captures/t276-desktop-dock-overview-absence-"
                                  + scheme + ".png");
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
