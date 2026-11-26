// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T280 absence stills (not part of ctest): no WirePlumber, no libinput/SDL
// inventory, and no CUPS/SANE. Every pane keeps its header, sections, and
// durable settingsd controls and states the absence instead of collapsing.
// Run through `scripts/capture-t280-devices.sh` without the adapter fixtures.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT280DevicesAbsenceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, paneId) {
            shell.selectPane(paneId);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            wait(200);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function save(shell, name) {
            grabImage(shell).save("docs/captures/" + name + ".png");
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            var shell = make();

            var sound = showPane(shell, "sound");
            save(shell, "t280-sound-absence-" + scheme);
            compare(sound.ready, false);
            compare(sound.absenceNote.visible, true);

            var keyboard = showPane(shell, "keyboard");
            save(shell, "t280-keyboard-absence-" + scheme);
            compare(keyboard.ready, false);
            compare(keyboard.absenceNote.visible, true);

            var mouse = showPane(shell, "mouse");
            save(shell, "t280-mouse-absence-" + scheme);
            compare(mouse.ready, false);

            var trackpad = showPane(shell, "trackpad");
            save(shell, "t280-trackpad-absence-" + scheme);
            compare(trackpad.ready, false);

            var printers = showPane(shell, "printers");
            save(shell, "t280-printers-absence-" + scheme);
            compare(printers.ready, false);
            compare(printers.absenceNote.visible, true);

            var controllers = showPane(shell, "game-controllers");
            save(shell, "t280-controllers-absence-" + scheme);
            compare(controllers.available, false);
            compare(controllers.absenceNote.visible, true);
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
