// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T229 absence stills: no settingsd, no bridge host, no CUPS/SANE, no
// controller inventory. The panes state the absence and keep their durable
// preferences live. Run through
// `scripts/capture-t229-input-users-printers.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsT229AbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function test_capture_absence() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width,
                                                height: stage.height,
                                                highlightHoldMs: 50 });
            waitForRendering(stage);
            compare(Settings.printersAvailable, false);
            compare(Settings.inputAvailable, false);

            showPane(shell, "game-controllers");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229-controllers-absence.png");

            showPane(shell, "printers");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229-printers-absence.png");

            showPane(shell, "sound");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229-sound-absence.png");
        }
    }
}
