// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T224 headless visual check (not part of ctest): the four states the task
// calls out — a live apply, a forced write failure with its inline revert
// message, a cancelled admin prompt, and a stopped service's absence note.
// Run through `scripts/capture-t224-settings-apply.sh`.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsApplyCapture"
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
            verify(shell.paneBody.item, "pane body for " + id + " must load");
            return shell.paneBody.item;
        }

        function init() {
            Settings.setAuthFixture("allow");
            Settings.resetAccountsFixture();
            Settings.apply("dock.autohide", false);
        }

        function test_capture_live_failure_auth_and_absence() {
            var shell = make();

            // 1. Live apply: the toggle writes through the common path and the
            //    value changes without a restart.
            var dock = showPane(shell, "desktop-dock");
            verify(shell.deepLink("settings://pane/desktop-dock/row/autohide"));
            wait(200);
            waitForRendering(stage);
            dock.autohideToggle.toggle();
            tryVerify(function() { return Settings.values["dock.autohide"] === true; });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t224-settings-live.png");

            // 2. Forced write failure: optimistic flip, then the owner rejects
            //    it; the control reverts and the row shows the message.
            Settings.failNextWrite("dock.autohide",
                                   "The settings service rejected the change.");
            dock.autohideToggle.toggle();
            tryVerify(function() {
                return Settings.values["dock.autohide"] === true
                    && Settings.stateFor("dock.autohide") === "error";
            });
            verify(Settings.errorFor("dock.autohide").indexOf("rejected") >= 0);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t224-settings-write-failure.png");

            // 3. Cancelled admin prompt: the account is untouched, the pane
            //    explains the cancellation, and there is no password field.
            var users = showPane(shell, "users-groups");
            tryVerify(function() { return users.ready === true; });
            Settings.setAuthFixture("cancel");
            Settings.setAccountType(1000, "standard");
            tryVerify(function() {
                return Settings.accountsError.indexOf("cancelled") >= 0;
            });
            verify(users.authErrorNote.visible);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t224-settings-auth-cancel.png");

            // 4. Service absence: with no bridge host on the private bus the
            //    Bluetooth pane is disabled with a reason, not a fake toggle.
            var bluetooth = showPane(shell, "bluetooth");
            tryVerify(function() { return bluetooth.ready === false; });
            compare(bluetooth.powerToggle.enabled, false);
            verify(bluetooth.absenceNote.visible);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t224-settings-service-absence.png");
        }
    }
}
