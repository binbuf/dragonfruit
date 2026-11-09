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
    }
}
