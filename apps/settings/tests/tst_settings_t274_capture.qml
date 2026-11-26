// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T274 Appearance headless visual check (not part of ctest): the consolidated
// groups in light and dark, the long-label layout, the Reduce Transparency
// override, and the live preview at the measured 740×820 window. Run through
// `scripts/capture-t274-appearance.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT274AppearanceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: 740, height: 820 });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell) {
            shell.selectPane("appearance");
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

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("accessibility.reduceTransparency", false);
            var shell = make();
            var pane = showPane(shell);

            // The consolidated groups: Appearance, Color, and the top of the
            // icon/style card.
            shell.detailScroll.flickable.contentY = 0;
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t274-appearance-" + scheme + ".png");

            // The Glass card is the preview: the rendered attached-menu plate
            // and the tint slider share the resolver.
            scrollTo(shell, pane.glassGroup);
            grabImage(shell).save("docs/captures/t274-appearance-preview-"
                                  + scheme + ".png");

            // A long localized label must elide inside the card.
            pane.labelSuffix = " — " + qsTr("a deliberately long localized label "
                                           + "that must elide instead of moving "
                                           + "the control out of the card");
            scrollTo(shell, pane.iconStyleRow);
            grabImage(shell).save("docs/captures/t274-appearance-long-string-"
                                  + scheme + ".png");
            pane.labelSuffix = "";

            // Reduce Transparency disables the slider and shows the reason.
            Settings.set("accessibility.reduceTransparency", true);
            scrollTo(shell, pane.transparencyRow);
            grabImage(shell).save("docs/captures/t274-appearance-reduce-transparency-"
                                  + scheme + ".png");
            Settings.set("accessibility.reduceTransparency", false);

            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
