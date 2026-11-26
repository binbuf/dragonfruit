// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T265 headless visual check (not part of ctest): the grouped, capped search
// results, the no-match state, a long localized query, and the keyboard focus
// after Down enters the list. Run through
// `scripts/capture-t265-settings-search.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsT265Capture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make(width, height) {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: width, height: height,
                                                highlightHoldMs: 60000 });
            waitForRendering(stage);
            return shell;
        }

        function capture(shell, path) {
            waitForRendering(stage);
            grabImage(shell).save(path);
        }

        function search(shell, text) {
            shell.searchText = text;
            shell.commitSearch();
            waitForRendering(stage);
        }

        function test_capture_grouped_no_match_long_and_keyboard() {
            // 1. Grouped, capped results with a result count and "Show all".
            var shell = make(740, 820);
            search(shell, "network");
            verify(shell.searchActive);
            verify(shell.searchResults.length > 0);
            verify(shell.searchResultsList.visible);
            verify(SettingsSearch.groupCount(shell.searchResults) > 1);
            capture(shell, "docs/captures/t265-settings-search-grouped.png");

            // 2. No match.
            search(shell, "zzzz-no-result");
            verify(shell.searchResults.length === 0);
            verify(shell.searchResultsList.emptyText.visible);
            capture(shell, "docs/captures/t265-settings-search-no-match.png");

            // 3. A long localized query stays in the field and wraps the
            //    no-match state without clipping.
            search(shell, "Barrierefreiheit und Bedienungshilfen für Menschen "
                          + "mit besonderen Anforderungen an die Darstellung");
            verify(shell.searchResultsList.emptyText.visible);
            capture(shell, "docs/captures/t265-settings-search-long-string.png");

            // 4. Down from the field enters the list with a focused result.
            search(shell, "wifi");
            shell.searchField.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Down);
            tryVerify(function() { return shell.searchResultsList.activeFocus; });
            capture(shell, "docs/captures/t265-settings-search-keyboard.png");

            // 5. The same grouped list at the 720x480 minimum.
            shell.destroy();
            wait(100);
            var compact = make(720, 480);
            search(compact, "network");
            verify(compact.searchResultsList.visible);
            capture(compact,
                    "docs/captures/t265-settings-search-compact.png");
        }
    }
}
