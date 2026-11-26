// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// T279 absence stills (not part of ctest): no portal PermissionStore, no
// fprintd/AccountsService, no GOA, and no security host. The four panes keep
// their headers and section titles and state the absence. Run through
// `scripts/capture-t279-accounts-security.sh`.
Item {
    id: stage
    width: 740
    height: 820

    TestCase {
        id: testCase
        name: "SettingsT279AccountsSecurityAbsenceCapture"
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

            showPane(shell, "privacy");
            save(shell, "t279-privacy-absence-" + scheme);
            showPane(shell, "biometrics");
            save(shell, "t279-biometrics-absence-" + scheme);
            showPane(shell, "users-groups");
            save(shell, "t279-users-absence-" + scheme);
            var online = showPane(shell, "online-accounts");
            save(shell, "t279-online-accounts-absence-" + scheme);
            compare(online.ready, false);
            compare(online.absenceNote.visible, true);
            shell.destroy();
        }

        function test_light() { capture("light"); }
        function test_dark() { capture("dark"); }
    }
}
