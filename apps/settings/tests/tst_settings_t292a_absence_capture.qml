// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T292a absence capture (not part of ctest): Desktop & Dock with no settings
// daemon, in light and dark. Every row stays live on the schema defaults and
// the advanced sheets still open; the former Mission Control surface keeps its
// own absence note. Run through `scripts/capture-t292a-desktop-dock.sh` with no
// settings fixture.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT292aDesktopDockAbsenceCapture"
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

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            var shell = make();

            var pane = showPane(shell, "desktop-dock");
            compare(Settings.available, false);
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            save(shell, "t292a-desktop-dock-absence-" + scheme);

            pane.advancedDockDialog.show();
            tryVerify(function() { return pane.advancedDockDialog.open; });
            tryVerify(function() {
                return pane.advancedDockDialog.width > 64
                        && pane.advancedDockDialog.height > 64;
            }, 5000);
            for (var i = 0; i < 8; ++i)
                waitForRendering(stage);
            save(shell, "t292a-desktop-dock-absence-advanced-dock-" + scheme);
            pane.closeAdvancedDock();
            waitForRendering(stage);

            // T285 removed the top-level Overview pane, so the alias lands on
            // Desktop & Dock's merged Workspace Overview section (T292b). The
            // section lands with the accent wash and stays live on the
            // defaults with its own absence note.
            shell.deepLink("settings://pane/mission-control");
            tryVerify(function() { return shell.currentPaneId === "desktop-dock"; });
            tryVerify(function() { return shell.highlightTarget !== null; });
            compare(Settings.available, false);
            waitForRendering(stage);
            compare(shell.paneBody.item.overviewAbsenceNote.visible, true);
            save(shell, "t292a-desktop-dock-absence-overview-" + scheme);

            shell.destroy();
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
