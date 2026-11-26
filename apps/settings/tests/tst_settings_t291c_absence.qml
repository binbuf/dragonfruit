// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291c Hearing absence (part of ctest): with no AT-SPI bridge host the
// Hearing sub-pane keeps its Audio group and the durable `sound.balance`
// preference stays live, because the row is settingsd-backed and not a live
// adapter read. Run with `DF_ACCESSIBILITY_FIXTURE=absent`.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsAccessibilityHearingAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.set("sound.balance", 0.5);
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function openHearing(shell) {
            var pane = shell.paneBody.item;
            verify(pane);
            pane.openSubPane("hearing");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var hearing = pane.subPaneLoader.item;
            verify(hearing);
            return hearing;
        }

        function test_the_audio_group_survives_without_the_bridge_host() {
            compare(Settings.accessibilityAvailable, false,
                    "no bridge host is the absent state under test");

            var shell = make();
            var hearing = openHearing(shell);

            // The body stays structured: the only owned group is Audio, and
            // the four ownerless sections remain map omissions.
            compare(hearing.audioGroup.visible, true);
            compare(hearing.audioGroup.title, "Audio");
            compare(SettingsMap.rowFor("accessibility",
                                       "hearing.devices").state, "omitted");
            compare(SettingsMap.rowFor("accessibility",
                                       "hearing.captions").state, "omitted");
            compare(SettingsMap.rowFor("accessibility",
                                       "hearing.spokenName").state, "omitted");
        }

        function test_the_balance_row_stays_live_without_the_bridge_host() {
            var shell = make();
            var hearing = openHearing(shell);

            compare(hearing.balance, 0.5);
            hearing.balanceSlider.setValue(0.8);
            hearing.balanceSlider.commit();
            compare(Settings.values["sound.balance"], 0.8);
            compare(hearing.balance, 0.8);

            // The row is stored-only, not a disabled live status.
            var row = SettingsMap.rowFor("accessibility", "hearing.balance");
            compare(row.state, "shipped");
            compare(row.storedOnly, true);
        }
    }
}
