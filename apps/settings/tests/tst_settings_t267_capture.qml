// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T267 headless visual check (not part of ctest): the Settings window with
// the adaptive presentation keys, RTL, and a keyboard focus ring. Run through
// `scripts/capture-t267-settings-accessibility.sh`; the 2x stills come from
// the companion capture file under QT_SCALE_FACTOR=2.
Item {
    id: stage
    width: 900
    height: 900

    Component { id: shellComponent; SettingsShell { } }

    TestCase {
        id: testCase
        name: "SettingsT267Capture"
        when: windowShown

        function reset() {
            Settings.set("accessibility.reduceMotion", false);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
        }

        function capture(item, path) {
            waitForRendering(item);
            grabImage(item).save(path);
        }

        function focusedShell() {
            Settings.set("settings.lastPane", "");
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            shell.selectPane("appearance");
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                    && shell.loadedPaneId === "appearance";
            });
            waitForRendering(stage);
            var target = shell.firstFocusable(shell.paneBody.item);
            if (target !== null)
                target.forceActiveFocus();
            waitForRendering(stage);
            return shell;
        }

        function test_capture_adaptive_states() {
            reset();
            var shell = focusedShell();
            capture(shell, "docs/captures/t267-settings-a11y-baseline.png");

            Settings.set("accessibility.increaseContrast", true);
            tryVerify(function() { return Glass.increaseContrast === true; });
            waitForRendering(stage);
            capture(shell, "docs/captures/t267-settings-a11y-high-contrast.png");
            Settings.set("accessibility.increaseContrast", false);

            Settings.set("accessibility.reduceTransparency", true);
            tryVerify(function() { return Glass.reduceTransparency === true; });
            waitForRendering(stage);
            capture(shell, "docs/captures/t267-settings-a11y-reduced-transparency.png");
            Settings.set("accessibility.reduceTransparency", false);

            Settings.set("accessibility.reduceMotion", true);
            tryVerify(function() { return Theme.reducedMotion === true; });
            shell.showDebugView("appearance");
            tryVerify(function() { return shell.debugView.open; });
            waitForRendering(stage);
            capture(shell, "docs/captures/t267-settings-a11y-reduced-motion.png");
            shell.debugView.hide();
            Settings.set("accessibility.reduceMotion", false);

            shell.rtl = true;
            waitForRendering(stage);
            capture(shell, "docs/captures/t267-settings-a11y-rtl.png");
            shell.rtl = false;
            reset();
            shell.destroy();
        }
    }
}
