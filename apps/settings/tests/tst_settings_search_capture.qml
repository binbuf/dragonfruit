// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T223 headless visual check (not part of ctest): render the ranked search
// results, a deep-linked row with its highlight, and a long-string pane.
// Run through `scripts/capture-t223-settings-search.sh`.
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsSearchCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        Component {
            id: longPaneComponent
            Rectangle {
                color: Theme.color.surface

                Column {
                    x: Theme.primitive.spacing.xl
                    width: Math.max(0, parent.width - 2 * Theme.primitive.spacing.xl)
                    spacing: Theme.primitive.spacing.lg

                    Item { width: 1; height: Theme.primitive.spacing.sm }

                    PaneHeader {
                        width: parent.width
                        pane: ({
                            id: "long-strings",
                            title: qsTr("Barrierefreiheit und Bedienungshilfen für Menschen mit besonderen Anforderungen"),
                            description: qsTr("Diese Beschreibung ist absichtlich sehr lang, damit der Kopfbereich über mehrere Zeilen umbricht, ohne ein Steuerelement zu verdecken."),
                            icon: "general"
                        })
                    }

                    SettingsGroup {
                        width: parent.width
                        title: qsTr("Darstellung und Bewegung für sehr lange lokalisierte Beschriftungen")

                        SettingsRow {
                            width: parent.width
                            label: qsTr("Bildschirmleser und Sprachausgabe für Menschen mit besonderen Anforderungen")
                            description: qsTr("Die Beschriftung und die Beschreibung sind absichtlich sehr lang. Das Steuerelement bleibt trotzdem vollständig sichtbar und wird nicht abgeschnitten.")
                            settingsId: "long-strings"
                            owner: "settingsd"
                            settingsKey: "test.long"
                            disclosure: true
                            trailingAction: qsTr("Details...")
                            help: qsTr("Help for the long-string row")
                            controlData: Toggle { checked: true }
                        }

                        SettingsRow {
                            width: parent.width
                            label: qsTr("Kontrast und Transparenz")
                            description: qsTr("Ein zweiter Eintrag zeigt, dass die Zeile ihr eigenes Steuerelement und ihre eigene Beschreibung behält.")
                            showSeparator: false
                            controlData: Toggle { checked: true }
                        }
                    }
                }
            }
        }

        function test_capture_search_deep_link_and_long_strings() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height,
                                                highlightHoldMs: 60000 });
            waitForRendering(stage);

            // 1. Ranked search results.
            shell.searchText = "previews";
            waitForRendering(stage);
            verify(shell.searchActive);
            verify(shell.searchResults.length > 0);
            verify(shell.searchResultsList.visible);
            grabImage(shell).save("docs/captures/t223-settings-search.png");

            // 2. A deep-linked row with its accent wash.
            shell.searchText = "";
            verify(shell.deepLink(
                "settings://pane/desktop-dock/row/minimizeReaction"));
            wait(200);
            waitForRendering(stage);
            verify(shell.highlightTarget !== null);
            compare(shell.highlightTarget.highlighted, true);
            grabImage(shell).save("docs/captures/t223-settings-deep-link.png");

            shell.destroy();
            wait(100);

            // 3. A long-string pane: the row anatomy keeps the control inside
            // the card and elides the label instead of clipping it.
            var pane = createTemporaryObject(longPaneComponent, stage,
                                             { width: 620, height: 640 });
            waitForRendering(stage);
            grabImage(pane).save("docs/captures/t223-settings-long-strings.png");
        }
    }
}
