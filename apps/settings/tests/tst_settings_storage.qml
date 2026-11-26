// SPDX-License-Identifier: GPL-3.0-only
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
    width: 740
    height: 820

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
            Settings.refreshStorage();
            Settings.setStorageFixture("live");
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

        function test_usage_chart_summary_and_recommendations() {
            var shell = make();
            var pane = paneOf(shell);
            compare(pane.usageAvailable, true);
            compare(pane.usageChart.visible, true);
            compare(pane.usageChartRepeater.count, 3);
            verify(pane.usageSummaryText.text.indexOf("used of") >= 0);
            compare(pane.recommendationsGroup.visible, true);
            verify(pane.recommendationsRepeater.count > 0);
            compare(pane.manageRow.visible, true);
            compare(pane.manageButton.enabled, true);
            // The stacked chart has one segment per category plus free, and
            // the legend names the same set.
            compare(pane.usageChartRepeater.count, 3);
            compare(pane.usageLegendRepeater.count, 4);
            verify(pane.usageLegendRepeater.itemAt(3).modelData.label === "Free");
            verify(pane.usageSummaryText.text.indexOf("GiB") >= 0);
        }

        function test_empty_storage_keeps_the_chart_and_manage_action() {
            Settings.setStorageFixture("empty");
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.storage.present, true);
            verify(pane.ready);
            compare(pane.volumes.length, 0);
            verify(pane.noVolumesNote.visible);
            verify(pane.usageGroup.visible);
            verify(pane.usageChart.visible);
            verify(pane.manageRow.visible);
            compare(pane.manageButton.enabled, true);
        }

        function test_udisks_absence_keeps_the_pane_structure() {
            Settings.setStorageFixture("absent");
            var shell = make();
            var pane = paneOf(shell);
            compare(Settings.storageAvailable, false);
            compare(pane.ready, false);
            verify(pane.usageGroup.visible);
            verify(pane.volumesGroup.visible);
            verify(pane.absenceNote.visible);
            verify(pane.usageChart.visible, "df usage stays live without UDisks2");
            verify(pane.manageRow.visible);
            compare(pane.manageButton.enabled, true);
            Settings.setStorageFixture("live");
        }

        function test_volume_row_shows_capacity_from_the_usage_read() {
            var shell = make();
            var pane = paneOf(shell);
            var detail = findChild(pane, "storageVolumeDetail");
            verify(detail !== null);
            verify(detail.text.length > 0);
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