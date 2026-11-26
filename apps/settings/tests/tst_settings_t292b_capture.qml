// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T292b Desktop & Dock headless visual check (not part of ctest): the merged
// Workspace Overview section and its Advanced Workspace Overview sheet (the
// real workspace count, the gestures, and the four corner assignments), at the
// measured 740×820 window in light and dark. The absence pass writes the same
// two surfaces with no settings daemon. Run through
// `scripts/capture-t292b-workspace-overview.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT292bWorkspaceOverviewCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; pin the keys this section shows
        // so a still never inherits the previous scheme's state.
        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("overview.switchToAppSpace", true);
            Settings.set("overview.hotCornerTopLeft", "mission-control");
            Settings.set("overview.hotCornerTopRight", "notification-center");
            Settings.set("overview.hotCornerBottomLeft", "desktop-reveal");
            Settings.set("overview.hotCornerBottomRight", "lock-screen");
            Settings.set("overview.hotCornerTopLeftModifier", "none");
            Settings.set("overview.hotCornerTopRightModifier", "command");
            Settings.set("overview.hotCornerBottomLeftModifier", "none");
            Settings.set("overview.hotCornerBottomRightModifier", "shift");
            Settings.set("gestures.missionControl", true);
            Settings.set("gestures.spaceSwitch", true);
            Settings.set("workspaces.count", 3);
        }

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

        function scrollTo(shell, item) {
            if (!item) {
                shell.detailScroll.flickable.contentY = 0;
                waitForRendering(stage);
                return;
            }
            // Rewind first so mapToItem sees the settled position.
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            tryVerify(function() { return shell.detailScroll.maxY >= 0; });
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
            var pane = showPane(shell, "desktop-dock");

            // The merged section, landed through its legacy deep link so the
            // still shows exactly what a `settings://pane/mission-control`
            // visit opens.
            shell.deepLink("settings://pane/mission-control");
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(shell.highlightTarget.sectionId, "Workspace Overview");
            for (var f = 0; f < 8; ++f)
                waitForRendering(stage);
            save(shell, "t292b-workspace-overview-merged-" + scheme);

            // After the accent wash releases, the footer's Hot Corners…
            // action opens the Advanced Workspace Overview sheet.
            scrollTo(shell, pane.missionControlGroup);
            save(shell, "t292b-workspace-overview-footer-" + scheme);

            openSheet(pane.advancedOverviewDialog);
            save(shell, "t292b-workspace-overview-advanced-" + scheme);
            pane.closeAdvancedOverview();
            tryVerify(function() { return !pane.advancedOverviewDialog.open; });
            waitForRendering(stage);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
