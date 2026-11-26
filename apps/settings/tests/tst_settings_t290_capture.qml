// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T290g Appearance headless visual check (not part of ctest): the landed
// mockup IA at the measured 740×820 window, in light and dark. It writes the
// pane top, the three mode tiles, the Glass preview and tint slider, the
// accent and highlight popups, the Advanced Appearance sheet, and the
// reduced-transparency and increase-contrast states. Run through
// `scripts/capture-t290-appearance.sh`.
Item {
    id: stage
    width: 740
    height: 820
    property string currentScheme: "light"

    TestCase {
        id: testCase
        name: "SettingsT290AppearanceCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture store is process-global. Reset every key this pane
        // touches so a still never inherits the previous scheme's state.
        function resetState(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            Settings.set("appearance.accent", "");
            Settings.set("appearance.transparency", 0.5255);
            Settings.set("appearance.highlight", "");
            Settings.set("appearance.folderColor", "");
            Settings.set("appearance.iconStyle", "default");
            Settings.set("appearance.iconTheme", "");
            Settings.set("appearance.cursorTheme", "");
            Settings.set("appearance.cursorSize", 24);
            Settings.set("appearance.sidebarIconSize", "medium");
            Settings.set("appearance.wallpaperTint", true);
            Settings.set("appearance.scrollBars", "automatic");
            Settings.set("appearance.scrollBarClick", "jump-to-next-page");
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
            Settings.setIconThemeFixture("present");
            Settings.setCursorThemeFixture("present");
        }

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
            if (!item) {
                shell.detailScroll.flickable.contentY = 0;
                return;
            }
            var y = item.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            waitForRendering(stage);
        }

        function capture(shell, name, item) {
            scrollTo(shell, item);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t290-appearance-" + name + "-"
                                  + stage.currentScheme + ".png");
        }

        // A popup/dialog animates in; grab after the open motion has settled
        // so the still is not a mid-flight ghost.
        function openPopup(popup) {
            popup.show();
            tryCompare(popup, "open", true);
            wait(Theme.motion.popupOpen.duration + 80);
            waitForRendering(stage);
        }

        function closePopup(popup) {
            popup.hide();
            tryCompare(popup, "open", false);
            wait(Theme.motion.popupClose.duration + 80);
            waitForRendering(stage);
        }

        function run(scheme) {
            stage.currentScheme = scheme;
            resetState(scheme);
            var shell = make();
            var pane = showPane(shell);

            // The pane top: Appearance mode tiles, then the Glass card.
            capture(shell, "pane", null);

            // The three mode tiles over `appearance.colorScheme`.
            capture(shell, "modes", pane.schemeRow);

            // The rendered attached-menu plate and the one tint slider.
            capture(shell, "glass", pane.glassGroup);

            // Accent popup: the one custom-hex entry point.
            scrollTo(shell, pane.accentRow);
            openPopup(pane.accentPopup);
            grabImage(shell).save("docs/captures/t290-appearance-accent-popup-"
                                  + scheme + ".png");
            closePopup(pane.accentPopup);

            // Highlight popup: a design-system palette swatch list.
            scrollTo(shell, pane.highlightRow);
            openPopup(pane.highlightSelectPopup);
            grabImage(shell).save("docs/captures/t290-appearance-highlight-popup-"
                                  + scheme + ".png");
            closePopup(pane.highlightSelectPopup);

            // The Advanced Appearance sheet owns the six Linux-only rows.
            scrollTo(shell, pane.advancedFooter);
            openPopup(pane.advancedDialog);
            grabImage(shell).save("docs/captures/t290-appearance-advanced-"
                                  + scheme + ".png");
            closePopup(pane.advancedDialog);

            // Reduce Transparency disables the slider and shows the reason.
            Settings.set("accessibility.reduceTransparency", true);
            waitForRendering(stage);
            capture(shell, "reduce-transparency", pane.transparencyRow);
            Settings.set("accessibility.reduceTransparency", false);

            // Increase Contrast is the resolver's other accessibility input.
            Settings.set("accessibility.increaseContrast", true);
            waitForRendering(stage);
            capture(shell, "increase-contrast", pane.glassGroup);
            Settings.set("accessibility.increaseContrast", false);

            shell.destroy();
        }

        function test_capture_light() { run("light"); }
        function test_capture_dark() { run("dark"); }
    }
}
