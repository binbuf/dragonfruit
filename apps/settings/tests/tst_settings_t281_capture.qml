// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T281 Linux-specific additions headless visual check (not part of ctest).
// Visits every pane that owns a Linux-only row, scrolls to that row, and
// writes it at the measured 740×820 window in light and dark. Run through
// `scripts/capture-t281-linux-specific.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT281LinuxSpecificCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // One pane per Linux-only row family. The row id is the executable
        // map id, so the capture also proves the row is rendered.
        readonly property var targets: [
            { pane: "wallpaper", row: "slideshow", name: "wallpaper-slideshow" },
            { pane: "desktop-dock", row: "windowRules", name: "desktop-window-rules" },
            { pane: "keyboard", row: "inputMethod", name: "keyboard-input-method" },
            { pane: "lock-screen", row: "idleDim", name: "lock-idle-stages" },
            { pane: "search", row: "clipboard", name: "search-clipboard" },
            { pane: "menu-bar", row: "globalMenu", name: "menubar-global-menu" },
            // T292b: the merged Desktop & Dock -> Advanced Workspace Overview
            // sheet (the former Overview pane's rows).
            { pane: "desktop-dock", row: "workspaceCount", name: "overview-workspaces" },
            { pane: "appearance", row: "windowControls", name: "appearance-window-controls" },
            { pane: "displays", row: "nightLight", name: "displays-night-light" },
            { pane: "sound", row: "perApp", name: "sound-per-app" },
            { pane: "trackpad", row: "gestures", name: "trackpad-gestures", tab: 2 },
            { pane: "battery", row: "adapter.chargeLimit", name: "battery-charge-limit" },
            { pane: "network", row: "firewallOptions", name: "network-firewall" },
            { pane: "vpn", row: "vpn.wireguard", name: "vpn-wireguard" },
            { pane: "privacy", row: "accessories", name: "privacy-usb-accessories" }
        ]

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
            wait(150);
            verify(shell.paneBody.item);
            return shell.paneBody.item;
        }

        function findRow(item, settingsId) {
            if (item.settingsId !== undefined
                    && String(item.settingsId) === settingsId)
                return item;
            var kids = item.children || [];
            for (var i = 0; i < kids.length; ++i) {
                var found = findRow(kids[i], settingsId);
                if (found)
                    return found;
            }
            return null;
        }

        function scrollTo(shell, pane, item) {
            // Map into the scroll content, the same coordinate space the
            // shell's reveal path uses.
            var content = shell.detailScroll.flickable.contentItem;
            var y = item.mapToItem(content, 0, 0).y;
            // The first set can run before the freshly loaded pane reports its
            // content height, so set it again after layout settles.
            shell.detailScroll.flickable.contentY = Math.max(0, y - 120);
            waitForRendering(stage);
            wait(80);
            shell.detailScroll.flickable.contentY = Math.max(0, y - 120);
            waitForRendering(stage);
            wait(120);
        }

        function save(shell, name, scheme) {
            grabImage(shell).save("docs/captures/t281-" + name + "-" + scheme + ".png");
        }

        function prepare() {
            Settings.set("wallpaper.slideshow", 30);
            Settings.set("search.clipboardHistory", true);
            Settings.set("idle.dim", 150);
            Settings.set("idle.blank", 300);
            Settings.set("idle.suspend", 0);
            Settings.set("idle.lock", 600);
            Settings.set("accessibility.reduceTransparency", false);
            Settings.set("accessibility.increaseContrast", false);
        }

        function capture(scheme) {
            Settings.set("appearance.colorScheme", scheme);
            prepare();
            var shell = make();
            for (var i = 0; i < targets.length; ++i) {
                var target = targets[i];
                var pane = showPane(shell, target.pane);
                if (target.tab !== undefined) {
                    pane.tabIndex = target.tab;
                    waitForRendering(stage);
                    wait(120);
                }
                var row = findRow(pane, target.row);
                verify(row !== null,
                       "row " + target.pane + "/" + target.row + " must render");
                scrollTo(shell, pane, row);
                save(shell, target.name, scheme);
            }
        }

        function test_capture_light_and_dark() {
            capture("light");
            capture("dark");
        }
    }
}
