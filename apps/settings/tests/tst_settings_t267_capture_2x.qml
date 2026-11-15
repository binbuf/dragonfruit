// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T267 2x visual check. Run through
// `scripts/capture-t267-settings-accessibility.sh` with QT_SCALE_FACTOR=2 so
// the same QML renders the real 2x scene instead of a scaled screenshot.
Item {
    id: stage
    width: 900
    height: 900

    Component { id: shellComponent; SettingsShell { } }

    TestCase {
        id: testCase
        name: "SettingsT267Capture2x"
        when: windowShown

        function capture(item, path) {
            waitForRendering(item);
            grabImage(item).save(path);
        }

        function makeShell(width, height) {
            Settings.set("settings.lastPane", "");
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: width, height: height });
            waitForRendering(stage);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            shell.selectPane("appearance");
            tryVerify(function() {
                return shell.paneBody.status === Loader.Ready
                    && shell.loadedPaneId === "appearance";
            });
            waitForRendering(stage);
            return shell;
        }

        function test_capture_2x() {
            compare(Screen.devicePixelRatio, 2.0);
            var wide = makeShell(740, 820);
            capture(wide, "docs/captures/t267-settings-a11y-2x-740x820.png");
            wide.destroy();

            var minimum = makeShell(720, 480);
            capture(minimum, "docs/captures/t267-settings-a11y-2x-720x480.png");
            minimum.destroy();
        }
    }
}
