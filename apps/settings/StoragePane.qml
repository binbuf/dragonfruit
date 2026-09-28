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

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshStorage()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

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

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "storageAbsenceNote"
                width: parent.width
                text: qsTr("No storage devices are available on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }
}