// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T229 headless visual check (not part of ctest): the Input, Users & Groups,
// Printers & Scanners, Game Controllers, Sound, and Battery breadth, after
// changing the pointer speed, the volume, a user/group setting, and adding a
// printer from the fixture discovery list. Run through
// `scripts/capture-t229-input-users-printers.sh`.
Item {
    id: stage
    width: 1000
    height: 900

    TestCase {
        id: testCase
        name: "SettingsT229Capture"
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
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function scrollTo(shell, item) {
            var y = item.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            waitForRendering(stage);
        }

        function test_capture_t229_breadth() {
            var shell = make();

            // Keyboard: input sources, text replacements, dictation.
            var keyboard = showPane(shell, "keyboard");
            Settings.setList("input.sources", ["us", "de:nodeadkeys"]);
            Settings.setList("input.replacements", ["omw\tOn my way"]);
            waitForRendering(stage);
            scrollTo(shell, keyboard.sourcesRow);
            grabImage(shell).save("docs/captures/t229-keyboard-input-sources.png");

            keyboard.sourcesDialog.show();
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229-keyboard-sources-dialog.png");
            keyboard.sourcesDialog.accept();
            wait(250);

            // Mouse: change the pointer speed and capture the new rows.
            var mouse = showPane(shell, "mouse");
            Settings.set("input.pointerSpeed", 0.6);
            Settings.set("input.scrollSpeed", -0.2);
            waitForRendering(stage);
            scrollTo(shell, mouse.secondaryClickSelect);
            grabImage(shell).save("docs/captures/t229-mouse-pointer-speed.png");

            // Trackpad: haptics, look-up, and the Bluetooth hand-off.
            var trackpad = showPane(shell, "trackpad");
            trackpad.tabIndex = 0;
            waitForRendering(stage);
            scrollTo(shell, trackpad.trackpadSecondarySelect);
            grabImage(shell).save("docs/captures/t229-trackpad-point-click.png");

            // Sound: change the volume and capture the test row.
            var sound = showPane(shell, "sound");
            Settings.setSoundVolume(0.35);
            waitForRendering(stage);
            scrollTo(shell, sound.outputInputGroup);
            grabImage(shell).save("docs/captures/t229-sound-volume.png");

            // Users & Groups: change a group setting and show the directory
            // and admin-group rows.
            var users = showPane(shell, "users-groups");
            users.toggleGroupMember("users", "sam", false);
            waitForRendering(stage);
            scrollTo(shell, users.adminGroupRow);
            grabImage(shell).save("docs/captures/t229-users-groups-directory.png");

            // Printers: discover and add a printer, then capture the options
            // sheet for the default queue.
            var printers = showPane(shell, "printers");
            printers.openAddPrinter();
            wait(250);
            waitForRendering(stage);
            verify(printers.addPrinterDialog.open);
            grabImage(shell).save("docs/captures/t229-printers-add.png");
            printers.chooseDevice(printers.discoveryDevices[0]);
            printers.addNameInput.text = "T229_Office";
            printers.addPrinterConfirm.clicked();
            wait(250);
            waitForRendering(stage);
            scrollTo(shell, printers.printersGroup);
            grabImage(shell).save("docs/captures/t229-printers-added.png");

            var row = findChild(printers.printerRepeater.itemAt(0), "printerRowArea");
            mouseClick(row, row.width / 2, row.height / 2);
            printers.optionsButton.clicked();
            wait(250);
            waitForRendering(stage);
            verify(printers.optionsDialog.open);
            grabImage(shell).save("docs/captures/t229-printers-options.png");
            printers.optionsDialog.accept();
            wait(250);

            // Game Controllers: the fixture controller, its test read, and the
            // mapping sheet.
            var controllers = showPane(shell, "game-controllers");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229-controllers-list.png");
            controllers.openTest(controllers.controllers[0]);
            wait(250);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t229-controllers-test.png");
            controllers.testDialog.accept();
            wait(250);

            // Battery: options and the Lock Screen hand-off.
            var battery = showPane(shell, "battery");
            Settings.set("power.showPercentage", true);
            waitForRendering(stage);
            scrollTo(shell, battery.optionsGroup);
            grabImage(shell).save("docs/captures/t229-battery-options.png");
        }
    }
}
