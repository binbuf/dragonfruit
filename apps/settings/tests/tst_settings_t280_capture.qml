// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T280 device/input headless visual check (not part of ctest): Sound,
// Keyboard, Mouse, Trackpad, Printers & Scanners, and Game Controllers at the
// measured 740×820 window in light and dark, including the Sound input tab,
// the keyboard shortcuts/dictation sheets, the trackpad tabs, the add-printer
// dialog, and the controller test dialog. Run through
// `scripts/capture-t280-devices.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT280DevicesCapture"
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

        function prepare() {
            Settings.setSoundVolume(0.6);
            Settings.setSoundMute(false);
            Settings.setSoundSourceVolume(0.5);
            Settings.setSoundSourceMute(false);
            Settings.setSoundDefaultSink(7);
            Settings.setSoundDefaultSource(20);
            Settings.set("input.mouse.pointerSpeed", 0.1);
            Settings.set("input.mouse.naturalScroll", true);
            Settings.set("input.mouse.smartZoom", true);
            Settings.set("input.mouse.pointerAcceleration", true);
            Settings.set("input.trackpad.pointerSpeed", 0.2);
            Settings.set("input.trackpad.clickPressure", 0.5);
            Settings.set("input.trackpad.pinchZoom", true);
            Settings.set("input.trackpad.smartZoom", true);
            Settings.set("input.trackpad.rotate", true);
            Settings.set("gestures.pages", "two-finger");
            Settings.set("gestures.fullscreen", "three-finger");
            Settings.set("gestures.notifications", false);
            Settings.set("gestures.showDesktop", true);
            Settings.set("gestures.appWindows", true);
            Settings.setList("input.sources", ["us", "de"]);
            Settings.setList("input.replacements", ["omw\tOn my way"]);
            Settings.resetPrintersFixture();
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            prepare();
            var shell = make();

            var sound = showPane(shell, "sound");
            save(shell, "t280-sound-" + scheme);
            sound.tabIndex = 1;
            waitForRendering(stage);
            wait(150);
            save(shell, "t280-sound-input-" + scheme);
            sound.tabIndex = 0;

            var keyboard = showPane(shell, "keyboard");
            save(shell, "t280-keyboard-" + scheme);
            keyboard.dictationOptionsDialog.show();
            waitForRendering(stage);
            wait(150);
            save(shell, "t280-keyboard-dictation-" + scheme);
            keyboard.dictationOptionsDialog.accept();
            keyboard.shortcutsDialog.show();
            waitForRendering(stage);
            wait(150);
            save(shell, "t280-keyboard-shortcuts-" + scheme);
            keyboard.shortcutsDialog.accept();
            waitForRendering(stage);
            wait(150);

            showPane(shell, "mouse");
            save(shell, "t280-mouse-" + scheme);

            var trackpad = showPane(shell, "trackpad");
            save(shell, "t280-trackpad-point-" + scheme);
            trackpad.tabIndex = 1;
            waitForRendering(stage);
            wait(150);
            save(shell, "t280-trackpad-scroll-zoom-" + scheme);
            trackpad.tabIndex = 2;
            waitForRendering(stage);
            wait(150);
            save(shell, "t280-trackpad-gestures-" + scheme);

            var printers = showPane(shell, "printers");
            save(shell, "t280-printers-" + scheme);
            printers.addPrinterButton.clicked();
            waitForRendering(stage);
            wait(150);
            save(shell, "t280-printers-add-" + scheme);
            printers.addPrinterDialog.reject();
            waitForRendering(stage);
            wait(150);

            var controllers = showPane(shell, "game-controllers");
            save(shell, "t280-controllers-" + scheme);
            controllers.openTest(controllers.controllers[0]);
            waitForRendering(stage);
            wait(150);
            save(shell, "t280-controllers-test-" + scheme);
            controllers.testDialog.accept();
            waitForRendering(stage);
            wait(150);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
