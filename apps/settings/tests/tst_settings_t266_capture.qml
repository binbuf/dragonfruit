// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T266 headless visual check (not part of ctest): the long-string row, an
// overlay Select near the window bottom, a 720x480 dialog, focus rings, and
// the three ScrollView modes. Run through
// `scripts/capture-t266-settings-components.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    WindowOverlay { id: stageOverlay }

    Component {
        id: longRowComponent
        SettingsGroup {
            width: 620
            title: "Accessibility and input"

            SettingsRow {
                width: parent.width
                label: "Barrierefreiheit und Bedienungshilfen für Menschen mit besonderen Anforderungen an die Darstellung"
                description: "Diese Beschreibung ist absichtlich sehr lang, damit die Zeile über mehrere Zeilen umbricht und das Steuerelement trotzdem vollständig sichtbar bleibt."
                disclosure: true
                controlData: Select {
                    model: ["Automatic", "Always", "When scrolling"]
                }
            }
        }
    }

    Component {
        id: hostComponent
        Item {
            width: 720
            height: 480
            property alias overlayer: overlayItem
            WindowOverlay { id: overlayItem }
        }
    }

    Component {
        id: selectComponent
        Select {
            width: 200
            model: ["Automatic", "Always", "When scrolling", "Never",
                    "Jump to the spot that is clicked", "Jump to the next page"]
        }
    }

    Component {
        id: dialogComponent
        Dialog {
            id: dialogComponentRoot
            title: "Lock Screen Message"
            message: "This message appears on the lock screen whenever it is shown. It stays fully visible while the body scrolls."
            contentData: Column {
                width: parent ? parent.width : 0
                spacing: Theme.primitive.spacing.sm
                Repeater {
                    model: 14
                    delegate: Text {
                        width: parent ? parent.width : 0
                        text: "A line of dialog content that must stay inside the 720x480 frame."
                        wrapMode: Text.WordWrap
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
            buttonsData: Row {
                Button { text: "Cancel"; onClicked: dialogComponentRoot.reject() }
                Button { text: "Set"; variant: "primary"; onClicked: dialogComponentRoot.accept() }
            }
        }
    }

    Component {
        id: focusRowComponent
        SettingsRow {
            width: 520
            label: "Pointer speed"
            description: "This row owns the keyboard focus and paints the shared focus ring."
            controlData: Slider { from: 0; to: 10; value: 6 }
        }
    }

    Component {
        id: scrollComponent
        ScrollView {
            width: 240
            height: 200
            Column {
                width: parent.width
                Repeater {
                    model: 10
                    delegate: Rectangle {
                        width: parent.width
                        height: 30
                        color: index % 2 === 0
                               ? Theme.color.surfaceElevated : Theme.color.surfaceMuted
                    }
                }
            }
        }
    }

    Component {
        id: scrollLabelsComponent
        Row {
            spacing: 20
            Text {
                width: 240
                text: "Automatic"
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: 240
                text: "When scrolling"
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
            Text {
                width: 240
                text: "Always"
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }
    }

    Component { id: shellComponent; SettingsShell { } }

    TestCase {
        id: testCase
        name: "SettingsT266Capture"
        when: windowShown

        function capture(item, path) {
            waitForRendering(stage);
            grabImage(item).save(path);
        }

        function test_capture_components() {
            // 1. Long localized label and description.
            var group = createTemporaryObject(longRowComponent, stage,
                                              { x: 40, y: 40 });
            capture(stage, "docs/captures/t266-settings-components-long-string.png");
            group.destroy();

            // 2. A Select near the bottom flips above its anchor and stays
            //    inside the 720x480 window.
            var host = createTemporaryObject(hostComponent, stage,
                                             { x: 20, y: 220 });
            var select = createTemporaryObject(selectComponent, host,
                                               { x: 40, y: 380 });
            select.openMenu();
            capture(host, "docs/captures/t266-settings-components-popup.png");
            select.menu.hide();
            select.destroy();
            host.destroy();

            // 3. A dialog at 720x480: the body scrolls and Close stays pinned.
            var dialogHost = createTemporaryObject(hostComponent, stage,
                                                   { x: 20, y: 220 });
            var dialog = createTemporaryObject(dialogComponent, dialogHost, {});
            dialog.show();
            wait(Theme.motion.popupOpen.duration + 80);
            capture(dialogHost, "docs/captures/t266-settings-components-dialog.png");
            dialog.destroy();
            dialogHost.destroy();

            // 4. Focus rings on a row and the scroll view.
            var focusRow = createTemporaryObject(focusRowComponent, stage,
                                                 { x: 40, y: 40 });
            waitForRendering(stage);
            focusRow.forceActiveFocus();
            tryVerify(function() { return focusRow.activeFocus; });
            wait(Theme.motion.focus.duration + 80);
            capture(stage, "docs/captures/t266-settings-components-focus.png");
            focusRow.destroy();

            // 5. The three scroll modes, side by side. Automatic shows an
            //    overflowing bar; when-scrolling hides it at rest; always
            //    keeps it even without overflow.
            var automatic = createTemporaryObject(scrollComponent, stage,
                                                  { x: 20, y: 240 });
            automatic.scrollBars = "automatic";
            automatic.forceActiveFocus();
            var whenScrolling = createTemporaryObject(scrollComponent, stage,
                                                      { x: 280, y: 240 });
            whenScrolling.scrollBars = "when-scrolling";
            var always = createTemporaryObject(scrollComponent, stage,
                                               { x: 540, y: 240 });
            always.scrollBars = "always";
            var labels = createTemporaryObject(scrollLabelsComponent, stage,
                                               { x: 20, y: 205 });
            waitForRendering(stage);
            capture(stage, "docs/captures/t266-settings-components-scroll-modes.png");
            labels.destroy();
            automatic.destroy();
            whenScrolling.destroy();
            always.destroy();
        }

        // The three live panes from the task's live visual check, rendered
        // headlessly through the real shell and its overlay.
        function test_capture_live_panes() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });

            shell.selectPane("general");
            tryVerify(function() { return shell.paneBody.item !== null; });
            shell.openSubPane("date-time");
            tryVerify(function() {
                return shell.paneBody.item.subPaneLoader.item !== null;
            });
            var dateTime = shell.paneBody.item.subPaneLoader.item;
            dateTime.timezoneSelect.openMenu();
            waitForRendering(stage);
            capture(shell, "docs/captures/t266-settings-components-date-time-timezone.png");
            dateTime.timezoneSelect.closeMenu();

            shell.selectPane("notifications");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            capture(shell, "docs/captures/t266-settings-components-notifications-long-name.png");

            shell.selectPane("lock-screen");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            var message = shell.paneBody.item.messageDialog;
            message.show();
            wait(Theme.motion.popupOpen.duration + 80);
            capture(shell, "docs/captures/t266-settings-components-lock-screen-dialog.png");
            message.reject();
            shell.destroy();
        }
    }
}
