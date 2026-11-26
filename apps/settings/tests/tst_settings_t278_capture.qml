// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T278 personalization/attention headless visual check (not part of ctest):
// the Wallpaper, Screen Saver, Notifications, Focus, and Lock Screen panes at
// the measured 740×820 window in light and dark. Run through
// `scripts/capture-t278-personalization.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT278PersonalizationCapture"
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
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            wait(200);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        function prepare() {
            Settings.setWallpaperFixture("categories");
            Settings.set("wallpaper.source", "");
            Settings.set("screenSaver.style", "clock");
            Settings.set("screenSaver.clock", true);
            Settings.set("screenSaver.showDate", true);
            Settings.set("screenSaver.showSeconds", false);
            Settings.set("idle.blank", 300);
            Settings.set("lock.requirePasswordAfter", 300);
            Settings.set("lock.showUserNameAndPhoto", true);
            Settings.set("lock.showMessageWhenLocked", true);
            Settings.set("lock.message", "Back at 3.");
            Settings.set("lock.showPasswordHints", true);
            Settings.set("lock.passwordHint", "The one with the cat");
            Settings.set("lock.showPowerButtons", true);
            Settings.set("notifications.appOptions", "");
            Settings.setList("focus.customProfiles", ["Writing"]);
            Settings.set("focus.customProfile", "Writing");
            Settings.setFocusProfile("custom");
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            prepare();
            var shell = make();

            showPane(shell, "wallpaper");
            save(shell, "t278-wallpaper-" + scheme);

            showPane(shell, "screen-saver");
            save(shell, "t278-screen-saver-" + scheme);

            var notifications = showPane(shell, "notifications");
            save(shell, "t278-notifications-" + scheme);
            notifications.openAppDetail(notifications.apps[1]);
            waitForRendering(stage);
            save(shell, "t278-notifications-sheet-" + scheme);
            notifications.appDialog.reject();
            waitForRendering(stage);
            wait(200);

            showPane(shell, "focus");
            save(shell, "t278-focus-" + scheme);

            var lock = showPane(shell, "lock-screen");
            compare(lock.lockMessage, "Back at 3.");
            compare(lock.showMessageWhenLocked, true);
            compare(lock.passwordHint, "The one with the cat");
            compare(lock.showPasswordHints, true);
            save(shell, "t278-lock-screen-" + scheme);
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
