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
        Component { id: missionControlComponent; MissionControlPane { } }
        Component { id: dateTimeComponent; DateTimePane { } }
        Component { id: languageComponent; LanguageRegionPane { } }
        Component { id: loginComponent; LoginItemsPane { } }
        Component { id: sharingComponent; SharingPane { } }
        Component { id: backupComponent; BackupPane { } }
        Component { id: startupComponent; StartupDiskPane { } }
        Component { id: resetComponent; ResetPane { } }

        // Every shipped Wave-1 pane, with the exact control surface the
        // matrix asserts stays live while its provider is absent.
        readonly property var shippedPaneIds:
            ["appearance", "desktop-dock", "displays",
             "wallpaper", "screen-saver", "wifi", "bluetooth", "network",
             "vpn", "battery", "general", "sound",
             "search", "keyboard", "mouse", "trackpad", "notifications",
             "focus", "lock-screen", "menu-bar", "users-groups",
             "printers", "privacy", "accessibility", "game-controllers",
             "online-accounts"]

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
            compare(SettingsPanes.shippedPanes.length, 26);
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

        // -- The per-pane routing sweep (T-15.16) -----------------------------

        // Every shipped pane must mount its body and read as an interactive
        // surface when **both** providers are absent (no settingsd, no bridge
        // host). This is the headless half of the T-15 absent-daemon masking
        // matrix: it catches a pane that throws, mounts empty, or disables
        // wholesale when its subsystem is gone. A pane whose live half is an
        // adapter read owns an `absenceNote`; the settingsd/compositor-backed
        // panes stay live on the schema defaults.
        function test_every_shipped_pane_routes_and_stays_live_with_providers_absent() {
            compare(Settings.available, false,
                    "settingsd is absent under this suite");
            var shell = make();
            for (var i = 0; i < shippedPaneIds.length; ++i) {
                var id = shippedPaneIds[i];
                var pane = showPane(shell, id);
                verify(pane !== null && pane !== undefined,
                       "shipped pane " + id + " must mount with both providers absent");
                verify(pane.enabled !== false,
                       "shipped pane " + id + " must stay interactive");
                verify(pane.visible !== false,
                       "shipped pane " + id + " must be visible");
                // A routing-table row that owns an absence note must expose it
                // as a live object (its visibility is the per-pane assertion
                // above; a missing note would be a half-pane).
                if (SettingsPanes.paneById(id) !== null && pane.absenceNote !== undefined)
                    verify(pane.absenceNote !== null,
                           "shipped pane " + id + " must own its absenceNote");
            }
        }

        // -- Each General sub-pane keeps its structure ------------------------

        // One absent General host must not collapse a sub-pane to a single
        // note: the settingsd half stays live and every group title survives.
        function test_general_sub_panes_keep_their_structure_without_the_bridge_host() {
            compare(Settings.generalAvailable, false,
                    "no bridge host is the absent state under test");
            var dateTime = createTemporaryObject(dateTimeComponent, stage);
            verify(dateTime);
            compare(dateTime.available, false);
            compare(dateTime.hostGroup.visible, true);
            compare(dateTime.clockGroup.visible, true);
            compare(dateTime.absenceNote.visible, true);
            dateTime.hour24Toggle.toggle();
            compare(Settings.values["clock.use24Hour"], true);

            var language = createTemporaryObject(languageComponent, stage);
            verify(language);
            compare(language.available, false);
            compare(language.languageGroup.visible, true);
            compare(language.regionGroup.visible, true);
            compare(language.previewGroup.visible, true);
            compare(language.absenceNote.visible, true);
            compare(language.addLanguagePicker.enabled, false);
            language.calendarSelect.activateIndex(1);
            compare(Settings.values["locale.calendar"], "iso");
            verify(language.previewText.text.length > 0);

            var login = createTemporaryObject(loginComponent, stage);
            verify(login);
            compare(login.available, false);
            compare(login.startGroup.visible, true);
            compare(login.backgroundGroup.visible, true);
            compare(login.extensionsGroup.visible, true);
            compare(login.absenceNote.visible, true);
            compare(login.addPicker.enabled, false);

            var sharing = createTemporaryObject(sharingComponent, stage);
            verify(sharing);
            compare(sharing.available, false);
            compare(sharing.servicesGroup.visible, true);
            compare(sharing.absenceNote.visible, true);
            compare(sharing.hostnameButton.enabled, false);

            var backup = createTemporaryObject(backupComponent, stage);
            verify(backup);
            compare(backup.available, false);
            compare(backup.policyGroup.visible, true);
            compare(backup.absenceNote.visible, true);
            compare(backup.scheduleSelect.enabled, false);
            compare(backup.autoToggle.enabled, false);

            var startup = createTemporaryObject(startupComponent, stage);
            verify(startup);
            compare(startup.available, false);
            compare(startup.diskListGroup.visible, true);
            compare(startup.absenceNote.visible, true);
            verify(startup.readOnlyNote.visible);
            compare(startup.restartButton.enabled, false);

            var reset = createTemporaryObject(resetComponent, stage);
            verify(reset);
            compare(reset.transferGroup.visible, true);
            compare(reset.resetGroup.visible, true);
            compare(reset.eraseNote.visible, true);
            compare(reset.resetButton.enabled, true);
        }

        // -- Each pane degrades cleanly ----------------------------------------

        function test_appearance_pane_stays_live_without_a_daemon() {
            var shell = make();
            var pane = showPane(shell, "appearance");
            compare(pane.schemeTileRepeater.count, 3);
            verify(pane.schemeTileRepeater.itemAt(0).enabled);
            compare(pane.accentRepeater.count, 6);
            verify(pane.accentRepeater.itemAt(0).enabled);

            // A write lands in the in-memory store even with no daemon.
            pane.schemeTileRepeater.itemAt(1).clicked(); // dark
            compare(Settings.values["appearance.colorScheme"], "dark");
            compare(Theme.dark, true);
            pane.schemeTileRepeater.itemAt(0).clicked(); // light
            compare(Settings.values["appearance.colorScheme"], "light");
            // This headless software scene has no sampled pass; pin the
            // legacy preview so the slider's resolver is deterministic.
            pane.previewRollout = "legacy";

            // T274: the accessibility test above leaves Reduce Transparency
            // on in the shared in-memory store; the slider is correctly
            // disabled under it, so clear the override before proving the
            // no-daemon write path.
            Settings.set("accessibility.reduceTransparency", false);
            tryVerify(function() { return pane.transparencyRow.enabled; });

            // T196: the glass slider stays live on the schema defaults and
            // its preview uses the same resolver with no daemon.
            pane.transparencySlider.setValue(0.8);
            pane.transparencySlider.commit();
            fuzzyCompare(Settings.values["appearance.transparency"], 0.8, 0.0001);
            compare(pane.previewResolved.sliderApplied,
                    pane.previewRollout === "replacement"
                        && Settings.values["appearance.colorScheme"] === "light");
        }

        function test_wallpaper_pane_disables_only_the_absent_portal_row() {
            var shell = make();
            var pane = showPane(shell, "wallpaper");

            // The portal-backed control is the only disabled one.
            compare(pane.photoButton.enabled, false,
                    "Add Photo… must be disabled when the FileChooser portal is absent");
            compare(pane.photoDescription, "No file chooser is available.");
            verify(pane.showOnAllToggle.enabled);
            verify(pane.fitControl.enabled);

            // No provider: the online catalogue is empty (with the
            // available-soon note) but the shipped default keeps the Built-in
            // row populated (T-18.2 / ADR 0240).
            compare(pane.providerItems.length, 0);
            verify(pane.featuredEmpty);
            verify(pane.featuredMessageItem.visible,
                   "an absent provider shows the available-soon note, never an error");
            compare(pane.featuredTiles.length, 0);
            compare(pane.builtinTiles.length, 3);
            compare(pane.builtinTiles[0].modelData.name, "Default");
            compare(pane.builtinTiles[0].modelData.source, pane.builtinDefault);
            compare(pane.builtinTiles[1].modelData.name, "Lagoon");
            compare(pane.builtinTiles[2].modelData.name, "Midnight");
            compare(pane.colorTiles.length, pane.presets.length);

            // The settingsd-backed controls still write in memory. Applying a
            // built-in tile goes through the async apply state.
            pane.colorTiles[0].choose();
            tryVerify(function() { return pane.pending === false; }, 5000);
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
            compare(pane.resolutionRadios.count, 3);
            verify(pane.resolutionRadios.count > 0);
            verify(pane.rotationSelect.enabled);

            pane.resolutionRadios.select(0); // Large -> 1.25
            verify(Math.abs(Settings.values["display.scale"] - 1.25) < 0.001);
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
            // T271: absence keeps the section structure; only the host
            // controls disable.
            compare(pane.myDevicesGroup.visible, true,
                    "My Devices keeps its section when the host is absent");
            compare(pane.nearbyGroup.visible, true,
                    "Nearby Devices keeps its section when the host is absent");
            // T288: the discoverable note only exists with a live, discoverable
            // adapter; the section structure itself is the toggle card.
            compare(pane.discoverableRow.visible, false);
            compare(pane.myDevicesEmpty.visible, true);
            compare(pane.nearbyEmpty.visible, true);
            compare(pane.advancedRow.visible, true);

            // A write is a safe no-op with no host.
            pane.togglePower();
            compare(Settings.bluetoothAvailable, false);
        }

        // T271: the Wi-Fi pane keeps every section, the settingsd prompt rows,
        // and Advanced when NetworkManager is absent; only the radio, join,
        // forget, and hotspot controls disable with a reason.
        function test_wifi_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.wifiAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.wifi.state, undefined);

            var pane = showPane(shell, "wifi");
            compare(pane.ready, false);
            compare(pane.powerToggle.enabled, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing network service");
            verify(pane.absenceNote.text.length > 0);
            compare(pane.currentGroup.visible, true);
            compare(pane.knownGroup.visible, true);
            compare(pane.availableGroup.visible, true);
            compare(pane.hotspotGroup.visible, true);
            compare(pane.knownEmptyRow.visible, true);
            compare(pane.unavailableRow.visible, true);
            compare(pane.advancedRow.visible, true);

            // The settingsd-backed prompt policy stays live on the defaults.
            verify(pane.askToJoinSelect.enabled);
            pane.askToJoinSelect.activateIndex(0);
            compare(Settings.values["network.wifi.askToJoin"], "off");

            // A write is a safe no-op with no host.
            pane.togglePower();
            pane.scan();
            compare(Settings.wifiAvailable, false);
        }

        // With no bridge host on the private bus, the Storage pane keeps its
        // structure: the Volumes group stays with an absence note, the df
        // usage group is independent, and no write is attempted.
        function test_storage_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.storageAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.storage.present, undefined);

            var pane = showPane(shell, "storage");
            compare(pane.ready, false);
            compare(pane.volumesGroup.visible, true);
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

        // The Mouse section shares the input body: with no bridge host its
        // inventory note shows while the pointer settings stay live.
        function test_mouse_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            var pane = showPane(shell, "mouse");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true);
            verify(pane.absenceNote.text.length > 0);

            pane.pointerSpeedSlider.setValue(0.5);
            pane.pointerSpeedSlider.commit();
            verify(Math.abs(Settings.values["input.mouse.pointerSpeed"] - 0.5) < 0.001);
            pane.naturalScrollToggle.toggle();
            compare(Settings.values["input.mouse.naturalScroll"], false);
        }

        // Overview is compositor-native: the pane's rows are settingsd
        // keys, so with no daemon they stay live on the schema defaults and the
        // pane shows the absence note instead of an adapter view. T285 removed
        // the top-level pane, so this exercises the component directly; T292
        // folds it into Desktop & Dock -> Workspace Overview.
        function test_mission_control_pane_stays_live_without_a_daemon() {
            var pane = createTemporaryObject(missionControlComponent, stage);
            verify(pane);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing settings daemon");
            verify(pane.absenceNote.text.length > 0);

            // The corner rows and the gesture rows write through to the
            // in-memory store on the schema defaults.
            compare(pane.topLeft, "mission-control");
            pane.topLeftSelect.activateIndex(0); // none
            compare(Settings.values["overview.hotCornerTopLeft"], "none");
            pane.bottomRightSelect.activateIndex(1); // Overview
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

        // With no bridge host on the private bus, Notifications pane's
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

            // T229a: the schedule and shortcut are settingsd-owned, so they
            // stay live while the service is away; the people row explains
            // the Linux limitation instead of drawing a dead list.
            verify(pane.scheduleGroup.visible);
            verify(pane.shortcutGroup.visible);
            compare(pane.peopleGroup.visible, false,
                    "the people row is service policy and hides with the service");
            Settings.set("focus.schedule.enabled", true);
            compare(pane.scheduleToggle.checked, true);
            pane.shortcutDraft = "Ctrl+Alt+F";
            pane.shortcutApplyButton.clicked();
            compare(Settings.values["focus.shortcut"], "Ctrl+Alt+F");
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
            pane.requirePasswordSelect.activateIndex(1); // After 5 seconds
            compare(Settings.values["idle.lock"], 305);
            pane.messageToggle.toggle();
            compare(Settings.values["lock.showMessageWhenLocked"], true);
            pane.powerButtonsToggle.toggle();
            compare(Settings.values["lock.showPowerButtons"], false);

            // T229a: the hint editor is live and the login-window row is
            // visibly disabled with its reason (the display manager owns it).
            pane.passwordHintsToggle.toggle();
            compare(Settings.values["lock.showPasswordHints"], true);
            compare(pane.setHintButton.enabled, true);
            compare(pane.loginWindowSelect.enabled, false);
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

        // The General list never collapses. With no bridge host both halves are
        // absent, so every row stays visible and its own summary says
        // "Unavailable"; nothing errors and no write is attempted.
        function test_general_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.updatesAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.updates.state, undefined);

            var pane = showPane(shell, "general");
            compare(pane.identityReady, false);
            compare(pane.aboutRow.visible, true);
            compare(pane.updateRow.visible, true);
            compare(pane.storageRow.visible, true);
            compare(pane.passwordsRow.visible, true);
            compare(pane.aboutSummary, "Unavailable");
            compare(pane.updateSummary, "Unavailable");
            compare(pane.storageSummary, "Unavailable");
            compare(pane.passwordsSummary, "Unavailable");

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
            Settings.createAccount("kim", "Kim", "standard", "correct horse");
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

        // The Accessibility view is a host-stack adapter read, so with no
        // bridge host it is the absence state: the Vision sub-pane keeps its
        // structure but the live status rows say Unavailable, while every
        // durable preference (a settingsd key) still applies (T291a).
        function test_accessibility_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.accessibilityAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.accessibility.state, undefined);

            var pane = showPane(shell, "accessibility");
            compare(pane.subPaneId, "");
            compare(pane.listColumn.visible, true);
            compare(pane.bigList.rows.children.length, 4);
            compare(pane.visionRow.label, "Vision");
            compare(pane.motorRow.label, "Physical and Motor");
            compare(pane.hearingRow.label, "Hearing");
            compare(pane.speechRow.label, "Speech");

            pane.openSubPane("vision");
            tryVerify(function() {
                return pane.subPaneLoader.status === Loader.Ready;
            });
            waitForRendering(stage);
            var vision = pane.subPaneLoader.item;
            verify(vision);

            compare(vision.ready, false);
            compare(vision.absenceNote.visible, true,
                    "the absence note explains the missing accessibility service");
            verify(vision.absenceNote.text.length > 0);
            // The pane stays structured; only the live status rows change.
            compare(vision.screenReaderGroup.visible, true);
            compare(vision.zoomGroup.visible, true);
            compare(vision.displayGroup.visible, true);
            compare(vision.motionGroup.visible, true);
            compare(vision.readSpeakGroup.visible, true);
            compare(vision.screenReaderLabel, "Unavailable");
            compare(vision.bridgeLabel, "Unavailable");
            compare(vision.screenReaderRow.trailingActionEnabled, true,
                    "the delegate fixture is present even with no bridge host");
            // The durable preferences are settingsd keys: they stay live.
            vision.reduceMotionToggle.toggle();
            compare(Settings.values["accessibility.reduceMotion"], true);
            vision.reduceTransparencyToggle.toggle();
            vision.increaseContrastToggle.toggle();
            compare(Settings.values["accessibility.reduceTransparency"], true);
            compare(Settings.values["accessibility.increaseContrast"], true);

            // A refresh is a safe no-op with no host.
            Settings.refreshAccessibility();
            compare(Settings.accessibilityAvailable, false);
        }

        // The Network advanced (VPN) view is a host-stack adapter read, so with
        // no bridge host it is the absence state: the connection list is
        // replaced by a note and the two writes are safe no-ops. T272 keeps
        // every section visible and leaves the firewall independent. The pane
        // owns no settingsd preference, because NetworkManager is the state.
        function test_network_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            compare(Settings.vpnAvailable, false,
                    "no bridge host is the absent state under test");
            compare(Settings.vpn.state, undefined);

            var pane = showPane(shell, "network");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true,
                    "the absence note explains the missing network service");
            verify(pane.absenceNote.text.length > 0);
            compare(pane.vpnGroup.visible, true,
                    "the VPN section stays visible when its host is absent");
            compare(pane.vpnAbsenceNote.visible, true);
            compare(pane.connectionRepeater.count, 0);
            compare(pane.servicesGroup.visible, true);
            compare(pane.addServiceRow.enabled, false);
            compare(pane.firewallGroup.visible, true);
            compare(pane.firewallToggle.enabled, false);

            // A refresh and the two writes are safe no-ops with no host.
            Settings.refreshVpn();
            Settings.connectVpn("11111111-1111-1111-1111-111111111111");
            Settings.disconnectVpn("11111111-1111-1111-1111-111111111111");
            compare(Settings.vpnAvailable, false);
        }

        // T288: the always-visible VPN pane keeps its structure when
        // NetworkManager is absent: the intro, the absence note, and the
        // disabled Add footer. The empty state belongs to a live host.
        function test_vpn_pane_degrades_cleanly_without_the_bridge_host() {
            var shell = make();
            var pane = showPane(shell, "vpn");
            compare(pane.ready, false);
            compare(pane.absenceNote.visible, true);
            verify(pane.absenceNote.text.length > 0);
            compare(pane.emptyNote.visible, false);
            compare(pane.addButton.visible, true);
            compare(pane.addButton.enabled, false);
            compare(pane.wireguardGroup.visible, true);
            verify(pane.intro.text.indexOf("VPN configurations") >= 0);

            // A refresh and the writes are safe no-ops with no host.
            pane.addVpn();
            compare(pane.addVpnDialog.open, false);
            Settings.refreshVpn();
            compare(Settings.vpnAvailable, false);
        }
    }
}