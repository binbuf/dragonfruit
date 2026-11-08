// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T229a headless visual check (not part of ctest): Wallpaper, Notifications,
// Focus, and Lock Screen in light and dark, with a live setting changed, the
// per-app detail sheet, and the provider/service absence states. Run through
// `scripts/capture-t229a-settings-panes.sh`.
Item {
    id: stage
    width: 1000
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsT229aCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width,
                                                height: stage.height,
                                                highlightHoldMs: 50 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function setScheme(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            waitForRendering(stage);
        }

        function capturePane(shell, id, path) {
            showPane(shell, id);
            waitForRendering(stage);
            grabImage(shell).save(path);
        }

        function test_capture_light() {
            Settings.setWallpaperFixture("ready");
            Settings.set("wallpaper.fit", "center");
            Settings.set("notifications.showPreviews", "always");
            Settings.set("notifications.showWhenSleeping", true);
            Settings.set("focus.schedule.enabled", true);
            Settings.set("focus.schedule.start", "21:30");
            Settings.set("focus.schedule.end", "06:15");
            Settings.set("focus.shortcut", "Ctrl+Alt+F");
            Settings.set("lock.showPasswordHints", true);
            Settings.set("lock.passwordHint", "The one with the cat");
            Settings.set("lock.showMessageWhenLocked", true);
            Settings.set("lock.message", "Back at 3.");
            setScheme("light");

            var shell = make();
            capturePane(shell, "wallpaper",
                        "docs/captures/t229a-wallpaper-light.png");
            capturePane(shell, "notifications",
                        "docs/captures/t229a-notifications-light.png");
            var notifications = showPane(shell, "notifications");
            notifications.openAppDetail(notifications.appByName("Mail"));
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229a-notifications-app-light.png");
            notifications.appDialog.reject();
            waitForRendering(stage);

            capturePane(shell, "focus", "docs/captures/t229a-focus-light.png");
            capturePane(shell, "lock-screen",
                        "docs/captures/t229a-lock-screen-light.png");
        }

        function test_capture_dark() {
            setScheme("dark");
            var shell = make();
            capturePane(shell, "wallpaper",
                        "docs/captures/t229a-wallpaper-dark.png");
            capturePane(shell, "notifications",
                        "docs/captures/t229a-notifications-dark.png");
            var notifications = showPane(shell, "notifications");
            notifications.openAppDetail(notifications.appByName("Pager"));
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229a-notifications-app-dark.png");
            notifications.appDialog.reject();
            waitForRendering(stage);

            capturePane(shell, "focus", "docs/captures/t229a-focus-dark.png");
            capturePane(shell, "lock-screen",
                        "docs/captures/t229a-lock-screen-dark.png");
        }
    }
}
