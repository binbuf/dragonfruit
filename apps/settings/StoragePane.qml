// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Storage pane (T-15.2b).
//
// The pane and the Control Center tile are one functional unit: both read the
// same UDisks2 adapter through the bridge host, and the three explicit writes
// (mount, unmount, eject) go through the `Settings` singleton to the adapter —
// never to D-Bus from QML. The pane lists the mountable volumes, mounts or
// unmounts each in place, and offers Eject for removable media. Opening the
// pane asks the host for a re-read.
//
// Absence is a normal state: UDisks2 gone disables everything and shows a
// one-line note; a running daemon with no mountable volume shows the same
// note. Neither errors.
Item {
    id: root

    // The bridge host's storage view (empty when absent).
    readonly property var view: Settings.storage
    readonly property bool available: Settings.storageAvailable
    readonly property bool present: view.present === true
    readonly property var volumes: view.volumes !== undefined ? view.volumes : []
    readonly property var drives: view.drives !== undefined ? view.drives : []
    readonly property int mountedCount: view.mountedCount !== undefined
        ? view.mountedCount : 0
    // T226: the General host's filesystem usage categories. They are a
    // separate read from UDisks2, so a missing `df` shows a note without
    // hiding the volume rows.
    readonly property var usage: Settings.general.storage !== undefined
        ? Settings.general.storage : ({})
    readonly property var usageCategories: usage.categories !== undefined
        ? usage.categories : []
    readonly property bool usageAvailable: Settings.generalAvailable
        && usage.state === "available"

    // The second hide rule (ADR 0119): UDisks2 absent (not available) or a
    // running daemon with no mountable volume (`present: false`).
    readonly property bool ready: root.available && root.present

    readonly property var removableDrives: {
        var out = [];
        for (var i = 0; i < root.drives.length; ++i) {
            if (root.drives[i].removable === true
                    && root.drives[i].mediaAvailable !== false)
                out.push(root.drives[i]);
        }
        return out;
    }

    // Test surface (used by tst_settings_storage.qml).
    property alias volumesGroup: volumesGroup
    property alias volumesRepeater: volumesRepeater
    property alias noVolumesNote: noVolumesNote
    property alias removableGroup: removableGroup
    property alias removableRepeater: removableRepeater
    property alias absenceNote: absenceNote
    property alias usageGroup: usageGroup
    property alias usageRepeater: usageRepeater
    property alias usageAbsenceNote: usageAbsenceNote
    property alias manageRow: manageRow

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function mountVolume(path) {
        if (!root.ready || !path)
            return;
        Settings.mountStorage(path);
    }

    function unmountVolume(path) {
        if (!root.ready || !path)
            return;
        Settings.unmountStorage(path);
    }

    function ejectDrive(drivePath) {
        if (!root.ready || !drivePath)
            return;
        Settings.ejectStorage(drivePath);
    }

    function formatBytes(bytes) {
        const value = Number(bytes || 0);
        if (value <= 0)
            return "0 GB";
        const gib = value / (1024 * 1024 * 1024);
        return (gib >= 10 ? Math.round(gib) : Math.round(gib * 10) / 10) + " GB";
    }

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: {
        Settings.refreshStorage();
        Settings.refreshGeneral();
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Usage categories (T226) ───────────────────────────────────────
        SettingsGroup {
            id: usageGroup
            width: parent.width
            title: qsTr("Usage")
            visible: root.usageAvailable

            Text {
                width: parent.width
                visible: root.usageCategories.length === 0
                text: qsTr("No filesystem usage is reported.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: usageRepeater
                model: root.usageCategories
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.label
                    description: qsTr("%1 of %2")
                        .arg(root.formatBytes(modelData.bytes))
                        .arg(root.formatBytes(root.usage.usedBytes))
                    showSeparator: index < root.usageCategories.length - 1
                    controlData: Rectangle {
                        width: 120
                        height: 6
                        radius: 3
                        color: Theme.color.controlFill
                        Rectangle {
                            width: parent.width * Math.min(1,
                                root.usage.usedBytes > 0
                                    ? modelData.bytes / root.usage.usedBytes : 0)
                            height: parent.height
                            radius: parent.radius
                            color: Theme.color.accent
                        }
                    }
                }
            }

            SettingsRow {
                id: manageRow
                objectName: "storageManageRow"
                width: parent.width
                label: qsTr("Manage storage…")
                description: qsTr("Formatting and partitioning are delegated to the Files app and the disk utility.")
                showSeparator: false
                controlData: Text {
                    text: qsTr("Delegated")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Volumes ───────────────────────────────────────────────────────
        SettingsGroup {
            id: volumesGroup
            width: parent.width
            title: qsTr("Volumes")
            visible: root.ready

            Text {
                id: noVolumesNote
                objectName: "storageNoVolumes"
                width: parent.width
                visible: root.volumes.length === 0
                text: qsTr("No mountable volumes.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: volumesRepeater
                model: root.volumes
                delegate: Item {
                    id: volumeRow
                    required property var modelData
                    width: parent.width
                    height: 48

                    Column {
                        anchors.left: parent.left
                        anchors.right: volumeAction.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "storageVolumeName"
                            width: parent.width
                            text: volumeRow.modelData.name
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Text {
                            objectName: "storageVolumeDetail"
                            width: parent.width
                            text: volumeRow.modelData.mounted
                                ? volumeRow.modelData.mountPoint
                                : (volumeRow.modelData.filesystem + " "
                                   + qsTr("(not mounted)"))
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        id: volumeAction
                        objectName: "storageVolumeAction"
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: volumeRow.modelData.mounted
                            ? qsTr("Unmount") : qsTr("Mount")
                        color: Theme.color.accent
                        font.pixelSize: Theme.controls.button.fontSize
                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("%1 %2")
                            .arg(text).arg(volumeRow.modelData.name)
                        TapHandler {
                            onTapped: {
                                if (volumeRow.modelData.mounted)
                                    root.unmountVolume(volumeRow.modelData.path);
                                else
                                    root.mountVolume(volumeRow.modelData.path);
                            }
                        }
                    }
                }
            }
        }

        // ── Removable media ───────────────────────────────────────────────
        SettingsGroup {
            id: removableGroup
            width: parent.width
            title: qsTr("Removable Media")
            visible: root.ready && root.removableDrives.length > 0

            Repeater {
                id: removableRepeater
                model: root.removableDrives
                delegate: Item {
                    id: driveRow
                    required property var modelData
                    width: parent.width
                    height: 44

                    Text {
                        objectName: "storageDriveName"
                        anchors.left: parent.left
                        anchors.right: ejectButton.left
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: driveRow.modelData.name
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                    }

                    Text {
                        id: ejectButton
                        objectName: "storageDriveEject"
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Eject")
                        color: driveRow.modelData.ejectable === true
                            ? Theme.color.accent : Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                        enabled: driveRow.modelData.ejectable === true
                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Eject %1")
                            .arg(driveRow.modelData.name)
                        TapHandler {
                            onTapped: root.ejectDrive(driveRow.modelData.path)
                        }
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            visible: !root.usageAvailable && root.ready

            SettingsAbsenceNotice {
                id: usageAbsenceNote
                objectName: "storageUsageAbsenceNote"
                kind: "disabled-with-reason"
                reason: qsTr("Filesystem usage is unavailable because df did not answer.")
            }
        }

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "storageAbsenceNote"
                kind: "disabled-with-reason"
                reason: qsTr("No storage devices are available on this system.")
            }
        }
    }
}