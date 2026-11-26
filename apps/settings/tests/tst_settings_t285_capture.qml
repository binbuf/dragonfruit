// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T285 headless visual check (not part of ctest): the reduced 27-entry
// sidebar, the update status row, and one landing for each moved alias, in
// light and dark. Run through `scripts/capture-t285-settings-ia.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsT285Capture"
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

        function ready(shell) {
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
        }

        function captureSidebar(shell, scheme) {
            shell.searchText = "";
            shell.commitSearch();
            shell.selectPane("general");
            ready(shell);
            shell.sidebar.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save(
                "docs/captures/t285-sidebar-order-" + scheme + ".png");
            grabImage(shell.sidebar).save(
                "docs/captures/t285-status-row-" + scheme + ".png");
        }

        function captureAlias(shell, link, name, scheme) {
            verify(shell.deepLink(link), link + " must land");
            ready(shell);
            waitForRendering(stage);
            grabImage(shell).save(
                "docs/captures/t285-" + name + "-" + scheme + ".png");
        }

        function run(scheme) {
            setScheme(scheme);
            var shell = make();
            captureSidebar(shell, scheme);
            captureAlias(shell, "settings://pane/mission-control",
                         "alias-workspace-overview", scheme);
            captureAlias(shell, "settings://pane/control-center",
                         "alias-menu-bar-controls", scheme);
            captureAlias(shell, "settings://pane/internet-accounts",
                         "alias-internet-accounts", scheme);
            verify(!shell.deepLink("settings://pane/biometrics"),
                   "the removed Biometrics id must be refused");
        }

        function test_capture_light() {
            run("light");
        }

        function test_capture_dark() {
            run("dark");
        }
    }
}
