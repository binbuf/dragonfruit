// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T286 headless visual check (not part of ctest): the measured shell at
// 740x820 and 720x480, the sidebar selection and spacer states, and the flat
// search-results list, in light and dark. Run through
// `scripts/capture-t286-settings-shell.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsT286Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make(width, height) {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: width,
                                                height: height });
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

        function captureShell(size, scheme) {
            var shell = make(size[0], size[1]);
            shell.selectPane("general");
            ready(shell);
            shell.sidebar.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save(
                "docs/captures/t286-shell-" + size[0] + "x" + size[1]
                + "-" + scheme + ".png");
            shell.destroy();
        }

        function captureSidebar(scheme) {
            var shell = make(740, 820);
            shell.selectPane("general");
            ready(shell);
            waitForRendering(stage);
            grabImage(shell.sidebar).save(
                "docs/captures/t286-sidebar-selection-" + scheme + ".png");

            // A second selection proves the neutral fill follows the row, and
            // the group boundary shows the 13 pt spacer.
            shell.selectPane("displays");
            ready(shell);
            waitForRendering(stage);
            grabImage(shell.sidebar).save(
                "docs/captures/t286-sidebar-spacers-" + scheme + ".png");
            shell.destroy();
        }

        function captureSearch(scheme) {
            var shell = make(740, 820);
            shell.searchText = "displays";
            shell.commitSearch();
            waitForRendering(stage);
            grabImage(shell).save(
                "docs/captures/t286-search-results-" + scheme + ".png");
            shell.destroy();
        }

        function run(scheme) {
            setScheme(scheme);
            captureShell([740, 820], scheme);
            captureShell([720, 480], scheme);
            captureSidebar(scheme);
            captureSearch(scheme);
        }

        function test_capture_light() {
            run("light");
        }

        function test_capture_dark() {
            run("dark");
        }
    }
}
