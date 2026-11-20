// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T291d Speech absence (part of ctest): with no AT-SPI bridge host and no
// speech-dispatcher/espeak-ng delegate installed, the Speech sub-pane keeps its
// Live Speech group and disables only the sample action with the
// missing-program reason. The row is never hidden. Run with
// `DF_ACCESSIBILITY_FIXTURE=absent` and `DF_ACCESSIBILITY_DELEGATE_FIXTURE=absent`.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsAccessibilitySpeechAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            shell.selectPane("accessibility");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function openSpeech(shell) {
            var pane = shell.paneBody.item;
            verify(pane);
            pane.openSubPane("speech");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var speech = pane.subPaneLoader.item;
            verify(speech);
            return speech;
        }

        function test_the_live_speech_row_survives_without_the_engine() {
            compare(Settings.accessibilityAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.accessibilitySpeechAvailable, false,
                    "no speech delegate is the absent state under test");

            var shell = make();
            var speech = openSpeech(shell);

            // The body stays structured: Live Speech is present, and the two
            // ownerless sections remain map omissions.
            compare(speech.liveSpeechGroup.visible, true);
            compare(speech.liveSpeechGroup.title, "Live Speech");
            compare(SettingsMap.rowFor("accessibility",
                                       "speech.customVoice").state, "omitted");
            compare(SettingsMap.rowFor("accessibility",
                                       "speech.vocalShortcuts").state, "omitted");

            // A present-but-unavailable delegate is a disabled row with a
            // reason, never a hidden row or a fake action.
            compare(speech.speechAvailable, false);
            compare(speech.speakSampleRow.visible, true);
            compare(speech.speakSampleRow.trailingAction, "Unavailable");
            compare(speech.speakSampleRow.trailingActionEnabled, false);
            verify(speech.speakSampleRow.description.indexOf("Not available") === 0,
                   "the disabled row must name the missing program");

            speech.speakSampleRow.trailingActionClicked();
            compare(Settings.accessibilityLastDelegate, "",
                    "the absent action must not launch anything");
        }

        function test_the_speech_row_is_still_listed_and_searchable() {
            var shell = make();
            var pane = shell.paneBody.item;
            compare(pane.speechRow.visible, true);
            compare(pane.speechRow.label, "Speech");

            var results = SettingsSearch.search("live speech");
            var found = false;
            for (var i = 0; i < results.length; ++i) {
                if (results[i].kind === "row"
                        && results[i].targetRow === "speech.liveSpeech")
                    found = true;
            }
            verify(found, "the Live Speech row stays searchable when absent");
        }
    }
}
