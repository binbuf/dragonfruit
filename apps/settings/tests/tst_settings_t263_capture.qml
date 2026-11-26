// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T263 headless visual check (not part of ctest): the measured 740x820
// System Settings window and the 720x480 minimum, in light and dark. Run
// through `scripts/capture-t263-settings-window.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsT263Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function setScheme(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            waitForRendering(stage);
        }

        function capture(shell, width, height, path) {
            shell.width = width;
            shell.height = height;
            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            grabImage(shell).save(path);
        }

        function test_capture_light() {
            setScheme("light");
            var shell = make();
            capture(shell, 740, 820,
                    "docs/captures/t263-settings-740x820-light.png");
            capture(shell, 720, 480,
                    "docs/captures/t263-settings-720x480-light.png");
        }

        function test_capture_dark() {
            setScheme("dark");
            var shell = make();
            capture(shell, 740, 820,
                    "docs/captures/t263-settings-740x820-dark.png");
            capture(shell, 720, 480,
                    "docs/captures/t263-settings-720x480-dark.png");
        }
    }
}
