// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

Item {
    id: stage
    width: 1000
    height: 900
    TestCase {
        name: "SettingsAboutCapture"
        when: windowShown
        Component { id: shellComponent; SettingsShell { } }
        Component { id: panelComponent; AboutPanel { } }
        function capture(scheme) {
            Settings.resetUpdatesFixture();
            Settings.resetGeneralFixture();
            Settings.set("appearance.colorScheme", scheme);
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            shell.visible = false;
            var panel = createTemporaryObject(panelComponent, stage,
                                              { x: 0, y: 0 });
            waitForRendering(stage);
            grabImage(panel).save("docs/captures/t230c-about-panel-" + scheme + ".png");
            panel.visible = false;
            shell.visible = true;
            shell.openAboutPage();
            tryVerify(function() { return shell.paneBody.item
                                            && shell.paneBody.item.subPaneId === "about"; });
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t230c-about-page-" + scheme + ".png");
            shell.destroy();
            panel.destroy();
        }
        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
        function test_long_localized_panel() {
            Settings.resetUpdatesFixture();
            Settings.resetGeneralFixture();
            Settings.set("appearance.colorScheme", "light");
            var panel = createTemporaryObject(panelComponent, stage,
                                              { x: 0, y: 0 });
            panel.identity = {
                deviceName: "A very long localized Dragonfruit machine name that exceeds the panel",
                hasSerial: false
            };
            panel.productName = "Système Dragonfruit très long et localisé";
            waitForRendering(stage);
            grabImage(panel).save("docs/captures/t230c-about-panel-long.png");
            panel.destroy();

            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            shell.visible = false;
            shell.openAboutPage();
            tryVerify(function() { return shell.paneBody.item
                                            && shell.paneBody.item.subPaneId === "about"; });
            var page = shell.paneBody.item.subPaneLoader.item;
            page.identity = {
                deviceName: "A very long localized Dragonfruit machine name that exceeds the page",
                hasSerial: false
            };
            page.productName = "Système Dragonfruit très long et localisé";
            waitForRendering(stage);
            shell.visible = true;
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t230c-about-page-long.png");
            shell.destroy();
        }
    }
}
