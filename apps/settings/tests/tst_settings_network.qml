// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Network advanced (VPN) pane tests (T-15.15b). Headless with
// `DF_SETTINGS_FIXTURE` and `DF_VPN_FIXTURE` (see CMakeLists.txt), so the
// `Settings` singleton and the NetworkManager bridge client are deterministic
// and in-process with no bus. The cases cover the live connection list, the
// connect/disconnect round-trip, and the absence/empty gating.
Item {
    id: stage
    width: 900
    height: 1200

    TestCase {
        id: testCase
        name: "SettingsNetwork"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // The fixture is process-global; reset it before every case so each
        // starts on the known OpenVPN-up / WireGuard-idle store.
        function init() {
            Settings.resetVpnFixture();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("network");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Network pane body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_live_connections() {
            var shell = make();
            compare(shell.currentPaneId, "network");
            compare(shell.currentPane.title, "Network");
            var pane = paneOf(shell);
            compare(pane.ready, true);
            compare(pane.available, true);
            compare(pane.connectionCount, 2);
            compare(pane.connectionRepeater.count, 2);
            compare(pane.vpnGroup.visible, true);
            compare(pane.absenceNote.visible, false);
            compare(findChild(pane, "networkEmptyNote").visible, false);
            // The first connection is the connected OpenVPN link.
            var row = pane.connectionRepeater.itemAt(0);
            compare(row.label, "Work VPN");
            var toggle = findChild(row, "networkVpnToggle");
            verify(toggle !== null);
            compare(toggle.checked, true);
        }

        function test_connect_and_disconnect_round_trip() {
            var shell = make();
            var pane = paneOf(shell);
            // Work VPN starts connected; disconnect it.
            var first = pane.connectionRepeater.itemAt(0);
            var firstToggle = findChild(first, "networkVpnToggle");
            firstToggle.toggle();
            tryVerify(function() { return Settings.vpn.connectedCount === 0; });
            compare(Settings.vpn.label, "Not Connected");
            compare(Settings.vpn.glyph, "vpn-off");

            // Connect the idle WireGuard link.
            var second = pane.connectionRepeater.itemAt(1);
            compare(second.label, "Home");
            var secondToggle = findChild(second, "networkVpnToggle");
            secondToggle.toggle();
            tryVerify(function() { return Settings.vpn.connectedCount === 1; });
            compare(Settings.vpn.activeName, "Home");
        }

        function test_the_toggle_follows_the_live_state() {
            var shell = make();
            var pane = paneOf(shell);
            var second = pane.connectionRepeater.itemAt(1);
            var secondToggle = findChild(second, "networkVpnToggle");
            compare(secondToggle.checked, false);
            // A change from elsewhere (the host's pushed state) converges. The
            // Repeater rebuilds its delegates when the view changes, so re-fetch
            // the live row rather than hold the old object.
            Settings.connectVpn("22222222-2222-2222-2222-222222222222");
            tryVerify(function() {
                var row = pane.connectionRepeater.itemAt(1);
                var toggle = row ? findChild(row, "networkVpnToggle") : null;
                return toggle !== null && toggle.checked === true;
            });
        }

        function test_the_absence_and_empty_notes_are_gated() {
            var shell = make();
            var pane = paneOf(shell);
            // The host answers, so the absence note is hidden.
            compare(pane.absenceNote.visible, false);
            compare(pane.vpnGroup.visible, true);
        }
    }
}