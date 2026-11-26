// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T292b absence capture (not part of ctest): the merged Workspace Overview
// section with no settings daemon, in light and dark. The five toggles stay
// live on the schema defaults with the section's absence note, and the
// Advanced Workspace Overview sheet still opens with every control. Run through
// `scripts/capture-t292b-workspace-overview.sh` with no settings fixture.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT292bWorkspaceOverviewAbsenceCapture"
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
            // `selectPane` is a no-op for the pane already shown; only wait
            // for the Loader when it actually swaps bodies.
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                        && shell.paneBody.item !== null;
            });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            var shell = make();
            var pane = showPane(shell, "desktop-dock");
            compare(Settings.available, false);

            // The legacy deep link lands on the merged section; the absence
            // note explains that the values come from the built-in defaults.
            shell.deepLink("settings://pane/mission-control");
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(shell.highlightTarget.sectionId, "Workspace Overview");
            compare(pane.overviewAbsenceNote.visible, true);
            for (var f = 0; f < 8; ++f)
                waitForRendering(stage);
            save(shell, "t292b-workspace-overview-absence-" + scheme);

            pane.openAdvancedOverview();
            tryVerify(function() { return pane.advancedOverviewDialog.open; });
            tryVerify(function() {
                return pane.advancedOverviewDialog.width > 64
                        && pane.advancedOverviewDialog.height > 64;
            }, 5000);
            for (var i = 0; i < 8; ++i)
                waitForRendering(stage);
            save(shell, "t292b-workspace-overview-absence-advanced-" + scheme);
            pane.closeAdvancedOverview();
            waitForRendering(stage);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
