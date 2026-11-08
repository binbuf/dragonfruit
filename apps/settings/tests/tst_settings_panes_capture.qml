// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings
import Dragonfruit.ControlCenter

// T227 headless visual check (not part of ctest): the Appearance completion,
// Quick Settings customization, Search, Screen Saver, Desktop & Stage, the
// Displays advanced sheet, and the live Control Center panel with the
// customized module list. Run through
// `scripts/capture-t227-settings-panes.sh`.
Item {
    id: stage
    width: 1000
    height: 760

    TestCase {
        id: testCase
        name: "SettingsPanesCapture"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }
        Component { id: desktopPaneComponent; DesktopDockPane { } }
        Component { id: panelComponent; ControlCenter { } }

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

        function test_capture_panes() {
            // Appearance: the new highlight/sidebar/tint/scroll/motion rows.
            var shell = make();
            showPane(shell, "appearance");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-appearance.png");

            // Quick Settings: the ordered module list and menu-bar-only rows.
            var quick = showPane(shell, "control-center");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-quick-settings.png");

            // The live panel with the customized list: dark + Wi-Fi + Sound in
            // the panel, Bluetooth in the menu bar only. Hide the Settings
            // shell so the translucent panel is captured over the neutral
            // window, not the pane behind it.
            Settings.setList("quickSettings.modules", ["dark", "wifi", "volume"]);
            Settings.setList("quickSettings.menuBarOnly", ["bluetooth"]);
            shell.visible = false;
            waitForRendering(stage);
            var panel = createTemporaryObject(panelComponent, stage, {
                width: 360,
                height: 520,
                quickSettingsModules: ["dark", "wifi", "volume"],
                quickSettingsMenuBarOnly: ["bluetooth"],
                wifi: { kind: "wifi", state: "available", visible: true,
                        enabled: true, radioEnabled: true, label: "Home" },
                audio: { kind: "audio", state: "available", visible: true,
                         enabled: true, volume: 0.6, muted: false,
                         defaultSink: "Built-in Speakers" },
                dark: true
            });
            waitForRendering(stage);
            verify(panel.tiles.length === 3);
            grabImage(panel).save("docs/captures/t227-quick-settings-live.png");
            shell.visible = true;
            waitForRendering(stage);

            // Search: categories, locations, shortcut, and the disabled
            // indexing status rows.
            showPane(shell, "search");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-search.png");

            // Screen Saver: style, start-after, and the clock options.
            showPane(shell, "screen-saver");
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-screen-saver.png");

            // Displays: the advanced sheet with the supported brightness /
            // rotation rows and the OPEN night-light/color/HDR/VRR reasons.
            var displays = showPane(shell, "displays");
            displays.advancedPopup.show();
            wait(250);
            waitForRendering(stage);
            verify(displays.advancedPopup.width > 200);
            grabImage(shell).save("docs/captures/t227-displays-advanced.png");
            displays.advancedPopup.hide();
            wait(250);

            // Desktop & Stage: the pane's bottom group (the shell scrolls to
            // it so the capture shows the rows, not the Dock group above).
            var desktop = showPane(shell, "desktop-dock");
            var y = desktop.desktopGroup.mapToItem(shell.detailContent, 0, 0).y;
            shell.detailScroll.flickable.contentY = Math.max(0, y - 12);
            waitForRendering(stage);
            grabImage(shell).save("docs/captures/t227-desktop-stage.png");

            // Restore the fixture list so a rerun is deterministic.
            Settings.setList("quickSettings.modules", [
                "wifi", "bluetooth", "storage", "focus", "volume", "brightness",
                "dark", "keyboard", "mission-control", "battery", "lock-screen",
                "menu-bar", "software-update", "users", "printers", "privacy",
                "accessibility", "vpn"
            ]);
            Settings.setList("quickSettings.menuBarOnly", []);
        }
    }
}
