// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T229a absence stills: no settingsd, no bridge host, no wallpaper provider.
// The panes state the absence and keep their durable preferences live. Run
// through `scripts/capture-t229a-settings-panes.sh`.
Item {
    id: stage
    width: 1000
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsT229aAbsenceCapture"
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
            compare(Settings.available, false);
            compare(Settings.notificationsAvailable, false);

            var wallpaper = showPane(shell, "wallpaper");
            verify(wallpaper.featuredEmpty);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229a-wallpaper-offline.png");

            var notifications = showPane(shell, "notifications");
            compare(notifications.absenceNote.visible, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229a-notifications-absence.png");

            var focus = showPane(shell, "focus");
            compare(focus.absenceNote.visible, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229a-focus-absence.png");

            var lock = showPane(shell, "lock-screen");
            compare(lock.absenceNote.visible, true);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229a-lock-screen-absence.png");
        }
    }
}
