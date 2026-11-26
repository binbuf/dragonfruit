// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291e General absence (part of ctest): with no AT-SPI bridge host and no
// speech delegate, the General sub-pane keeps its Accessibility Shortcut group
// live, because the recorded chord is a settingsd preference the compositor
// reads; the recorder has no external program to be missing. The five-row list
// stays complete. Run with `DF_ACCESSIBILITY_FIXTURE=absent` and
// `DF_ACCESSIBILITY_DELEGATE_FIXTURE=absent`.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsAccessibilityGeneralAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function init() {
            Settings.set("accessibility.shortcut", "");
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

        function openGeneral(shell) {
            var pane = shell.paneBody.item;
            verify(pane);
            pane.openSubPane("general");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var general = pane.subPaneLoader.item;
            verify(general);
            return general;
        }

        function test_the_shortcut_row_stays_live_without_the_bridge_host() {
            compare(Settings.accessibilityAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.accessibilitySpeechAvailable, false,
                    "no speech delegate is the absent state under test");

            var shell = make();
            var pane = shell.paneBody.item;
            compare(pane.bigList.rows.children.length, 5,
                    "every mockup section is listed, absent host or not");
            compare(pane.generalRow.visible, true);
            compare(pane.generalRow.label, "General");

            var general = openGeneral(shell);
            compare(general.shortcutGroup.visible, true);
            compare(general.shortcutGroup.title, "Accessibility Shortcut");
            compare(general.shortcut, "");

            // The recorder is a settingsd write, not a delegate: it records
            // with no host at all.
            general.recorder.startRecording();
            keyClick(Qt.Key_K, Qt.MetaModifier | Qt.ShiftModifier);
            compare(Settings.values["accessibility.shortcut"], "Cmd+Shift+K");
            compare(general.recorder.chord, "Cmd+Shift+K");

            // The row keeps its real owner and stays searchable.
            compare(SettingsMap.rowFor("accessibility", "shortcut.recorder").state,
                    "shipped");
            var results = SettingsSearch.search("accessibility shortcut");
            var found = false;
            for (var i = 0; i < results.length; ++i) {
                if (results[i].kind === "row"
                        && results[i].targetRow === "shortcut.recorder")
                    found = true;
            }
            verify(found, "the shortcut row stays searchable when the host is absent");
        }
    }
}
