// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Settings

// Storage-pane tests (T-15.2b). Headless with `DF_SETTINGS_FIXTURE` and
// `DF_STORAGE_FIXTURE` (see CMakeLists.txt), so the `Settings` singleton
// serves a deterministic in-process adapter with no bus. The cases cover the
// volume rows and their mount/unmount round-trip, the removable-media Eject
// round-trip, and the absence note.
Item {
    id: stage
    width: 900
    height: 720

    TestCase {
        id: testCase
        name: "SettingsStorage"
        when: windowShown

        Component { id: shellComponent; SettingsShell { } }

        readonly property string volumePath:
            "/org/freedesktop/UDisks2/block_devices/sdb1"
        readonly property string drivePath:
            "/org/freedesktop/UDisks2/drives/usb"

        // The fixture store is process-global; reset the volume to unmounted
        // before every case so the pane always opens on a known state.
        function init() {
            Settings.unmountStorage(volumePath);
            Settings.refreshStorage();
        }

        function make() {
            var shell = createTemporaryObject(shellComponent, stage,
                                              { width: stage.width, height: stage.height });
            waitForRendering(stage);
            shell.selectPane("storage");
            tryVerify(function() { return shell.paneBody.status === Loader.Ready; });
            waitForRendering(stage);
            return shell;
        }

        function paneOf(shell) {
            verify(shell.paneBody.status === Loader.Ready,
                   "the Storage body must be loaded");
            var pane = shell.paneBody.item;
            verify(pane);
            return pane;
        }

        function test_pane_opens_with_the_volume_and_removable_groups() {
            var shell = make();
            compare(shell.currentPaneId, "storage");
            var pane = paneOf(shell);
            verify(pane.ready, "the storage view must be present");
            verify(pane.volumesGroup.visible, "the Volumes group is shown");
            verify(pane.removableGroup.visible, "the Removable Media group is shown");
            compare(pane.volumes.length, 1);
            compare(pane.volumes[0].name, "Photos");
            compare(pane.removableDrives.length, 1);
            compare(pane.removableDrives[0].name, "Flash Drive");
        }

        function test_volume_row_mounts_and_unmounts_live() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.volumes[0].mounted, false);

            pane.mountVolume(volumePath);
            compare(Settings.storage.volumes[0].mounted, true);
            compare(Settings.storage.volumes[0].mountPoint,
                    "/run/media/user/Photos");
            compare(pane.mountedCount, 1);

            pane.unmountVolume(volumePath);
            compare(Settings.storage.volumes[0].mounted, false);
        }

        function test_volume_action_button_is_accessible() {
            var shell = make();
            var pane = paneOf(shell);
            var action = findChild(pane, "storageVolumeAction");
            verify(action !== null, "the volume action is present");
            compare(action.Accessible.role, Accessible.Button);
            verify(action.Accessible.name.indexOf("Photos") >= 0);
        }

        function test_external_change_converges_into_the_pane() {
            var shell = make();
            var pane = paneOf(shell);
            Settings.mountStorage(volumePath);
            compare(pane.volumes[0].mounted, true);
            Settings.unmountStorage(volumePath);
            compare(pane.volumes[0].mounted, false);
        }

        // The eject case runs last (the `zz` prefix sorts after every other case):
        // the fixture cannot restore an ejected drive, so it must not precede
        // the cases that still expect the volume.
        function test_zz_eject_removes_the_removable_drive() {
            var shell = make();
            var pane = paneOf(shell);
            var eject = findChild(pane, "storageDriveEject");
            verify(eject !== null, "the drive Eject button is present");
            pane.ejectDrive(drivePath);
            compare(Settings.storage.present, false);
            compare(Settings.storage.volumes.length, 0);
        }
    }
}