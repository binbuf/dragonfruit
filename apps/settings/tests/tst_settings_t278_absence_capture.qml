// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T278 absence capture (not part of ctest): the five panes with no settings
// daemon, no provider, and no notification service. Run through
// `scripts/capture-t278-personalization.sh` with no fixtures.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT278PersonalizationAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            var ids = ["wallpaper", "screen-saver", "notifications", "focus",
                       "lock-screen"];
            for (var i = 0; i < ids.length; ++i) {
                shell.selectPane(ids[i]);
                tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
                waitForRendering(stage);
                grabImage(shell).save("docs/captures/t278-" + ids[i]
                                      + "-absence-" + scheme + ".png");
            }
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
