// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T292a Desktop & Dock headless visual check (not part of ctest): the restyled
// pane top, the Desktop & Workspaces / Widgets / Windows sections, the live
// Workspace Overview entry with the footer, the two advanced sheets, and the
// hot-corner surface reached from the footer, at the measured 740×820 window
// in light and dark. Run through
// `scripts/capture-t292a-desktop-dock.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT292aDesktopDockCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global; pin the keys this pane shows so
        // a still never inherits the previous scheme's state.
        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.set("dock.size", 0.5);
            Settings.set("dock.magnification", 0.6);
            Settings.set("dock.position", "bottom");
            Settings.set("dock.minimizedAnimation", "scale");
            Settings.set("dock.titlebarDoubleClick", "zoom");
            Settings.set("dock.minimizeIntoTileIcon", false);
            Settings.set("dock.autohide", false);
            Settings.set("dock.animateOpening", true);
            Settings.set("dock.showIndicators", true);
            Settings.set("dock.showRecentApps", false);
            Settings.set("dock.chooserOnHover", false);
            Settings.set("dock.minimizeReaction", true);
            Settings.set("desktop.showItems", "on-desktop");
            Settings.set("defaults.browser", "");
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

        function scrollToEnd(shell) {
            tryVerify(function() { return shell.detailScroll.maxY >= 0; });
            // The detail column keeps a bottom pad so the last card clears the
            // help control; scroll it fully rather than leaving it half-visible.
            shell.detailScroll.flickable.contentY = shell.detailScroll.maxY
                    + shell.detailScroll.flickable.contentHeight;
            waitForRendering(stage);
            shell.detailScroll.flickable.contentY = shell.detailScroll.maxY;
            waitForRendering(stage);
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        // A Dialog adopts the window overlay layer, then fades its scrim and
        // scales its card in with Motion Behaviors; the hidden measures also
        // settle on the next frame. Grab a few frames after opening so a still
        // never captures a half-faded dialog.
        function openSheet(pane, dialog) {
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
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            save(shell, "t292a-desktop-dock-top-" + scheme);

            scrollTo(shell, pane.desktopGroup);
            save(shell, "t292a-desktop-dock-desktop-" + scheme);

            scrollTo(shell, pane.widgetsGroup);
            save(shell, "t292a-desktop-dock-widgets-windows-" + scheme);

            scrollToEnd(shell);
            save(shell, "t292a-desktop-dock-overview-footer-" + scheme);

            openSheet(pane, pane.advancedDockDialog);
            save(shell, "t292a-desktop-dock-advanced-dock-" + scheme);
            pane.closeAdvancedDock();
            tryVerify(function() { return !pane.advancedDockDialog.open; });
            waitForRendering(stage);

            openSheet(pane, pane.advancedWindowsDialog);
            save(shell, "t292a-desktop-dock-advanced-windows-" + scheme);
            pane.closeAdvancedWindows();
            tryVerify(function() { return !pane.advancedWindowsDialog.open; });
            waitForRendering(stage);

            // T285 removed the top-level Overview pane; the alias lands on
            // Desktop & Dock -> Workspace Overview. T292b deleted the former
            // Mission Control body, so the footer's Hot Corners… action opens
            // the Advanced Workspace Overview sheet on this pane.
            shell.deepLink("settings://pane/mission-control");
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            waitForRendering(stage);
            // Let the accent-wash fade and the next frame settle before the
            // grab, or the still can mix the two states.
            for (var f = 0; f < 8; ++f)
                waitForRendering(stage);
            save(shell, "t292a-desktop-dock-merged-overview-" + scheme);

            openSheet(pane, pane.advancedOverviewDialog);
            save(shell, "t292a-desktop-dock-hot-corners-" + scheme);
            pane.closeAdvancedOverview();
            tryVerify(function() { return !pane.advancedOverviewDialog.open; });
            waitForRendering(stage);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
