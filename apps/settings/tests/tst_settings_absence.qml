// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Settings absent-provider matrix (T-09.6b).
//
// The headless half of the matrix: with no settingsd on the bus and no
// xdg-desktop-portal, every shipped Wave-1 pane still renders, every control
// stays functional, and writes are served from the schema defaults in memory.
// No pane is advertised that has no body (the no-half-panes rule), and the
// wallpaper "Add Photo…" row is the one control that reflects a missing
// provider (the portal) by disabling with an explanatory description.
//
// Run under a private `dbus-run-session` with **no** `DF_SETTINGS_FIXTURE`, so
// the `Settings` singleton is the live `DbusSettingsClient` and both providers
// are honestly absent (see CMakeLists.txt).
Item {
    id: stage
    width: 1000
    height: 720

    TestCase {
        id: testCase
        name: "SettingsAbsence"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        // Every shipped Wave-1 pane, with the exact control surface the
        // matrix asserts stays live while its provider is absent.
        readonly property var shippedPaneIds:
            ["appearance", "desktop-dock", "mission-control", "displays",
             "wallpaper", "bluetooth", "battery", "storage", "general", "sound",
             "keyboard", "mouse", "trackpad", "notifications", "focus",
             "lock-screen", "menu-bar", "users-groups", "printers", "privacy"]

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            return shell;
        }

        function showPane(shell, id) {
            shell.selectPane(id);
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            verify(shell.paneBody.item, "pane body for " + id + " must load");
            return shell.paneBody.item;
        }

        // -- Both providers are absent -----------------------------------------

        function test_no_daemon_and_no_portal_are_the_absent_state() {
            compare(Settings.available, false,
                    "no settingsd on the bus is the absent-provider state under test");
            compare(Settings.wallpaperChooserAvailable, false,
                    "no portal on the bus is the absent-provider state under test");
        }

        // -- The wallpaper provider is absent too (T-18.1b) --------------------

        // No `org.dragonfruit.Wallpaper1` on the private bus: the Featured
        // catalogue and the fetched default are empty (never an error), while
        // the shipped default still resolves so the Built-in row is never
        // empty.
        function test_wallpaper_provider_absence_never_surfaces_an_error() {
            compare(Settings.providerItems.length, 0,
                    "no provider: the Featured catalogue is empty, not an error");
            compare(Settings.providerStatus, "",
                    "no provider: the status is empty, not an error");
            compare(Settings.providerDefault, "",
                    "no provider: the fetched default is empty");
            verify(Settings.wallpaperBuiltinDefault.length > 0,
                   "the shipped default still resolves with the provider absent");
            verify(Settings.wallpaperBuiltinDefault.indexOf("Default.jpg") >= 0,
                   "the shipped default resolves to Default.jpg");

            // An eager load request is a safe no-op with no provider.
            Settings.preloadWallpapers();
            compare(Settings.providerItems.length, 0);
        }

        // -- The no-half-panes catalog rule ------------------------------------

        function test_every_shipped_pane_has_a_body_and_no_other_does() {
            var shell = make();
            compare(SettingsPanes.shippedPanes.length, 20);
            for (var i = 0; i < SettingsPanes.catalog.length; ++i) {
                var pane = SettingsPanes.catalog[i];
                var body = shell.paneComponent(pane.id);
                if (pane.shipped)
                    verify(body !== null, "shipped pane " + pane.id + " must have a body");
                else
                    verify(body === null, "unshipped pane " + pane.id
                           + " must not advertise a body");
            }
            // The sidebar/search list is exactly the shipped subset.
            var listed = shell.visiblePanes.map(function(pane) { return pane.id; });
            compare(listed.sort().join(","), shippedPaneIds.slice().sort().join(","));
        }

        // -- Each pane degrades cleanly ----------------------------------------

        function test_appearance_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "appearance");
            verify(pane.schemeControl.enabled);
            compare(pane.accentRepeater.count, 6);
            verify(pane.accentRepeater.itemAt(0).enabled);

            // A write lands in the in-memory store even with no daemon.
            pane.schemeControl.activateIndex(1); // dark
            compare(Settings.values["appearance.colorScheme"], "dark");
            compare(Theme.dark, true);
            pane.schemeControl.activateIndex(0); // light
            compare(Settings.values["appearance.colorScheme"], "light");
        }

        function test_wallpaper_pane_disables_only_the_absent_portal_row() {
            var shell = make();
            var pane = showPane(shell, "wallpaper");

            // The portal-backed control is the only disabled one.
            compare(pane.photoButton.enabled, false,
                    "Add Photo… must be disabled when the FileChooser portal is absent");
            compare(pane.photoRow.description, "No file chooser is available.");
            verify(pane.showOnAllToggle.enabled);
            verify(pane.fitControl.enabled);

            // No provider: Featured is empty (with the available-soon note) but
            // the shipped default keeps the Built-in row populated (T-18.2).
            compare(pane.providerItems.length, 0);
            verify(pane.featuredEmpty);
            verify(pane.featuredMessageItem.visible,
                   "an absent provider shows the available-soon note, never an error");
            compare(pane.featuredTiles.length, 0);
            compare(pane.builtinTiles.length, 1 + pane.presets.length);
            compare(pane.builtinTiles[0].modelData.name, "Default");
            compare(pane.builtinTiles[0].modelData.source, pane.builtinDefault);

            // The settingsd-backed controls still write in memory.
            pane.builtinTiles[1].choose();
            compare(Settings.values["wallpaper.source"], pane.presets[0].source);
            pane.showOnAllToggle.toggle();
            compare(Settings.values["wallpaper.showOnAllSpaces"], false);
            pane.fitControl.activateIndex(2); // stretch
            compare(Settings.values["wallpaper.fit"], "stretch");
        }

        function test_desktop_dock_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "desktop-dock");
            compare(pane.dockGroup.rows.children.length, 12);
            var rows = pane.dockGroup.rows.children;
            for (var i = 0; i < rows.length; ++i)
                verify(rows[i].control.enabled, "Dock row " + i + " control must stay enabled");

            pane.autohideToggle.toggle();
            compare(Settings.values["dock.autohide"], true);
            pane.positionSelect.activateIndex(1); // left
            compare(Settings.values["dock.position"], "left");
            pane.sizeSlider.setValue(0.8);
            pane.sizeSlider.commit();
            verify(Math.abs(Settings.values["dock.size"] - 0.8) < 0.001);
        }

        function test_displays_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "displays");
            compare(pane.tileItems.length, 5);
            for (var i = 0; i < pane.tileItems.length; ++i)
                verify(pane.tileItems[i].enabled, "resolution tile " + i + " must stay enabled");
            verify(pane.rotationSelect.enabled);

            pane.tileItems[0].choose(); // Larger Text
            verify(Math.abs(Settings.values["display.scale"] - 1.5) < 0.001);
            pane.rotationSelect.activateIndex(2); // 180°
            compare(Settings.values["display.rotation"], "180");
        }

        // With no bridge host on the private bus, the Bluetooth pane is the absence
// state: the toggle is disabled, the discoverable caption and both device
// groups are hidden, and a one-line note explains the missing daemon. Nothing
// errors and no write is attempted.
        function test_bluetooth_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.bluetoothAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.bluetooth.present, undefined);

            var pane = showPane(shell, "bluetooth");
            compare(pane.ready, false);
            verify(pane.powerToggle);
            compare(pane.powerToggle.enabled, false,
                    "the Bluetooth toggle is disabled when the host is absent");
            compare(pane.caption.visible, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing bridge host");
            verify(pane.absenceNote.text.length > 0);
            compare(pane.myDevicesGroup.visible, false);
            compare(pane.nearbyGroup.visible, false);

            // A write is a safe no-op with no host.
            pane.togglePower();
            compare(Settings.bluetoothAvailable, false);
        }

        // With no bridge host on the private bus, the Storage pane is the
        // absence state: both groups are hidden and a one-line note explains
        // the missing daemon. Nothing errors and no write is attempted.
        function test_storage_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.storageAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.storage.present, undefined);

            var pane = showPane(shell, "storage");
            compare(pane.ready, false);
            compare(pane.volumesGroup.visible, false);
            compare(pane.removableGroup.visible, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing bridge host");
            verify(pane.absenceNote.text.length > 0);

            // A write is a safe no-op with no host.
            pane.mountVolume("/org/freedesktop/UDisks2/block_devices/sdb1");
            compare(Settings.storageAvailable, false);
        }

        // With no bridge host on the private bus, the Sound pane's routing half is
        // the absence state: the Output & Input group is hidden and a one-line
        // note explains the missing daemon. The Sound Effects/Balance controls
        // are settingsd-backed and stay live on the schema defaults, so the
        // pane is never a dead surface.
        function test_sound_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.soundAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.sound.state, undefined);

            var pane = showPane(shell, "sound");
            compare(pane.ready, false);
            compare(pane.outputInputGroup.visible, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing bridge host");
            verify(pane.absenceNote.text.length > 0);

            // The settingsd-backed controls still work with no daemon.
            verify(pane.effectsGroup.visible);
            pane.playOnStartupToggle.toggle();
            compare(Settings.values["sound.playOnStartup"], false);
            pane.alertVolumeSlider.setValue(0.4);
            pane.alertVolumeSlider.commit();
            verify(Math.abs(Settings.values["sound.alertVolume"] - 0.4) < 0.001);

            // A routing write is a safe no-op with no host.
            pane.selectDevice(9);
            compare(Settings.soundAvailable, false);
        }

        // With no bridge host on the private bus, the Keyboard/Mouse/Trackpad
        // panes' inventory half is the absence state: a one-line note explains
        // the missing libinput view. Every preference row is settingsd-backed
        // and stays live on the schema defaults, so the panes are never dead
        // surfaces.
        function test_keyboard_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.inputAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.input.state, undefined);

            var pane = showPane(shell, "keyboard");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing input inventory");
            verify(pane.absenceNote.text.length > 0);

            // The settingsd-backed controls still work with no daemon.
            pane.keyboardNavigationToggle.toggle();
            compare(Settings.values["input.keyboardNavigation"], true);
            pane.repeatRateSlider.setValue(0.5);
            pane.repeatRateSlider.commit();
            verify(Math.abs(Settings.values["input.repeatRate"] - 100) < 0.001);

            // A refresh is a safe no-op with no host.
            Settings.refreshInput();
            compare(Settings.inputAvailable, false);
        }

        function test_trackpad_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            var pane = showPane(shell, "trackpad");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true);
            compare(pane.tapToClickToggle.enabled, true);
            pane.tapToClickToggle.toggle();
            compare(Settings.values["input.tapToClick"], false);
        }

        // Mission Control is compositor-native: the pane's rows are settingsd
        // keys, so with no daemon they stay live on the schema defaults and the
        // pane shows the absence note instead of an adapter view. The Control
        // Center tile's absent case is the compositor bridge, owned by the
        // shell (T-15.5b).
        function test_mission_control_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "mission-control");
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing settings daemon");
            verify(pane.absenceNote.text.length > 0);

            // The corner rows and the gesture rows write through to the
            // in-memory store on the schema defaults.
            compare(pane.topLeft, "mission-control");
            pane.topLeftSelect.activateIndex(0); // none
            compare(Settings.values["overview.hotCornerTopLeft"], "none");
            pane.bottomRightSelect.activateIndex(1); // Mission Control
            compare(Settings.values["overview.hotCornerBottomRight"], "mission-control");
            pane.gestureMissionToggle.toggle();
            compare(Settings.values["gestures.missionControl"], false);
        }

        // With no bridge host on the private bus, the Battery pane is the absence
        // state: the power/battery/history groups are hidden and a one-line note
        // explains the missing service. Nothing errors and no write is attempted.
        function test_battery_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.batteryAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.battery.state, undefined);

            var pane = showPane(shell, "battery");
            compare(pane.present, false);
            compare(pane.profilesAvailable, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing status service");
            verify(pane.absenceNote.text.length > 0);

            // A profile write is a safe no-op with no host.
            Settings.setPowerProfile("performance");
            compare(Settings.batteryAvailable, false);
        }

        // A control changed while the daemon is absent still converges into the
        // pane on the next event-loop turn (the in-memory store is the source
        // of truth for the session).
        function test_absent_writes_converge_into_the_controls() {
            var shell = make();
            var pane = showPane(shell, "desktop-dock");
            Settings.set("dock.showIndicators", false);
            compare(pane.showIndicatorsToggle.checked, false);
            Settings.set("dock.showIndicators", true);
            compare(pane.showIndicatorsToggle.checked, true);
        }

        // With no bridge host on the private bus, the Notifications pane's
        // per-app inventory is the absence state while the four settingsd
        // presentation preferences stay live on the schema defaults, so the
        // pane is never a dead surface.
        function test_notifications_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.notificationsAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.notifications.state, undefined);

            var pane = showPane(shell, "notifications");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing notification service");
            verify(pane.absenceNote.text.length > 0);
            compare(pane.appGroup.visible, false);

            // The settingsd-backed preferences still write in memory.
            pane.previewSelect.activateIndex(0); // Always
            compare(Settings.values["notifications.showPreviews"], "always");
            pane.lockedToggle.toggle();
            compare(Settings.values["notifications.showWhenLocked"], false);

            // A refresh is a safe no-op with no host.
            Settings.refreshNotifications();
            compare(Settings.notificationsAvailable, false);
        }

        // The Focus pane's mode and allow-list are the notification adapter's
        // state, so with no bridge host they are the absence state; nothing
        // errors and no write is attempted.
        function test_focus_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.notificationsAvailable, false,
                    "no bridge host is the absent state under test");

            var pane = showPane(shell, "focus");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing notification service");
            verify(pane.absenceNote.text.length > 0);

            // A mode write is a safe no-op with no host.
            Settings.setFocusMode("dnd");
            compare(Settings.notificationsAvailable, false);
        }

        // Lock policy is session/compositor-native: the pane's rows are
        // settingsd keys, so with no daemon they stay live on the schema
        // defaults and the pane shows the absence note instead of an adapter
        // view. The lock itself is enforced by the compositor (T-15.8b).
        function test_lock_screen_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "lock-screen");
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing settings daemon");
            verify(pane.absenceNote.text.length > 0);

            // The timing rows reuse the session `idle.*` keys and the display
            // rows write the revision-15 `lock.*` keys through to the
            // in-memory store on the schema defaults.
            compare(pane.requirePasswordSeconds, 600);
            pane.requirePasswordSelect.activateIndex(0); // After 5 seconds
            compare(Settings.values["idle.lock"], 5);
            pane.messageToggle.toggle();
            compare(Settings.values["lock.showMessageWhenLocked"], true);
            pane.powerButtonsToggle.toggle();
            compare(Settings.values["lock.showPowerButtons"], false);
        }

        // The menu bar is shell-native: the pane's rows are settingsd keys, so
        // with no daemon they stay live on the schema defaults and the pane
        // shows the absence note instead of an adapter view. The bar itself is
        // drawn by the shell (T-15.9b).
        function test_menu_bar_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "menu-bar");
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing settings daemon");
            verify(pane.absenceNote.text.length > 0);

            // The behavior rows and the per-control toggles write through to
            // the in-memory store on the schema defaults.
            compare(pane.autoHide, "full-screen");
            pane.autoHideSelect.activateIndex(0); // Never
            compare(Settings.values["menu.autoHide"], "never");
            pane.backgroundToggle.toggle();
            compare(Settings.values["menu.showBackground"], false);
            pane.recentSelect.activateIndex(0); // None
            compare(Settings.values["menu.recentItems"], 0);
            pane.clockSecondsToggle.toggle();
            compare(Settings.values["menu.clock.showSeconds"], true);
            var toggle = findChild(pane.controlsRepeater.itemAt(0),
                                   "menuBarControlToggle");
            toggle.toggle();
            compare(Settings.values["menu.control.wifi"], false);
        }

        // The General/About/Updates view is a host-stack adapter read, so with
        // no bridge host it is the absence state: the disclosure rows and the
        // dialogs' controls are replaced by a one-line note. Nothing errors and
        // no write is attempted.
        function test_general_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.updatesAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.updates.state, undefined);

            var pane = showPane(shell, "general");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing host-stack service");
            verify(pane.absenceNote.text.length > 0);

            // A refresh and the update writes are safe no-ops with no host.
            Settings.refreshUpdates();
            Settings.checkUpdates();
            Settings.installUpdates();
            Settings.rebootUpdates();
            compare(Settings.updatesAvailable, false);
        }

        // The Users & Groups view is a host-stack adapter read, so with no
        // bridge host it is the absence state: the user/group lists and the
        // add/edit dialogs are replaced by a one-line note. Nothing errors and
        // no write is attempted (T-15.11b).
        function test_users_groups_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.accountsAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.accounts.state, undefined);

            var pane = showPane(shell, "users-groups");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing account service");
            verify(pane.absenceNote.text.length > 0);

            // A refresh and the account/group writes are safe no-ops.
            Settings.refreshAccounts();
            Settings.createAccount("kim", "Kim", "standard");
            Settings.setAccountLocked(1000, true);
            Settings.createAccountGroup("devs");
            compare(Settings.accountsAvailable, false);
        }

        // The Printers & Scanners view is a host-stack adapter read, so with no
        // bridge host it is the absence state: the queue/scanner lists and the
        // per-printer dialog are replaced by a one-line note. The one settingsd
        // key the pane owns (`printers.defaultPaperSize`) stays live on its
        // schema default, so the pane is never a dead surface (T-15.12b).
        function test_printers_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.printersAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.printers.state, undefined);

            var pane = showPane(shell, "printers");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing printing service");
            verify(pane.absenceNote.text.length > 0);
            compare(pane.printersGroup.visible, false);
            compare(pane.scannersGroup.visible, false);

            // The settingsd-backed paper size still writes in memory.
            pane.paperSizeSelect.activateIndex(3); // A4
            compare(Settings.values["printers.defaultPaperSize"], "a4");

            // The queue writes are safe no-ops with no host.
            Settings.refreshPrinters();
            Settings.setDefaultPrinter("Canon_MF230");
            Settings.setPrinterAcceptingJobs("Canon_MF230", false);
            Settings.cancelPrinterJob(1);
            compare(Settings.printersAvailable, false);
        }

        // The Privacy & Security view is a host-stack adapter read, so with no
        // bridge host it is the absence state: the category list is replaced by
        // a one-line note and the permission writes are safe no-ops. The pane
        // owns no settingsd preference, because the portal store is the state
        // (T-15.13b).
        function test_privacy_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.privacyAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.privacy.state, undefined);

            var pane = showPane(shell, "privacy");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing permission service");
            verify(pane.absenceNote.text.length > 0);
            compare(pane.categoryRepeater.count, 0);

            // A refresh and the permission writes are safe no-ops with no host.
            Settings.refreshPrivacy();
            Settings.setPrivacyPermission("devices", "camera", "org.mozilla.firefox", "denied");
            Settings.deletePrivacyPermission("devices", "camera", "org.mozilla.firefox");
            compare(Settings.privacyAvailable, false);
        }
    }
}