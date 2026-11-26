// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Sharing sub-pane (T226; T270 completion).
//
// The host only reports services that are installed: an absent SMB, SSH, CUPS,
// or remote-desktop unit is not a row. Each toggle is one explicit
// `systemctl enable/disable --now` through the bridge host, which may ask for
// administrator authentication.
//
// T270: a local-hostname row with Edit, a per-service Options sheet, no
// duplicated on/off or "requires authentication" text in the description, and
// a structured service list when the host is absent.
Item {
    id: root

    readonly property var view: Settings.general.sharing !== undefined
        ? Settings.general.sharing : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var services: root.view.services !== undefined
        ? root.view.services : []
    readonly property string hostname: root.view.hostname !== undefined
        ? String(root.view.hostname) : ""
    readonly property bool hostnameEditable: root.view.hostnameEditable !== false
    readonly property string hostnameReason: root.view.hostnameReason !== undefined
        ? String(root.view.hostnameReason) : ""

    readonly property string hostnameDraft: hostnameField.text
    readonly property string hostnameValidation: root.hostnameDraft.trim() !== root.hostname
        ? root.validateHostname(root.hostnameDraft) : ""
    property var optionsService: null

    readonly property string hostnameError: root.hostnameValidation.length > 0
        ? root.hostnameValidation
        : (Settings.generalWriteOperation === "hostname"
           && (Settings.generalWriteState === "denied"
               || Settings.generalWriteState === "failed")
           ? (Settings.generalWriteState === "denied"
              ? qsTr("Authentication was denied: %1").arg(Settings.generalWriteError)
              : Settings.generalWriteError)
           : "")
    readonly property string sharingError: Settings.generalWriteOperation === "sharing"
        && (Settings.generalWriteState === "denied"
            || Settings.generalWriteState === "failed")
        ? Settings.generalWriteError : ""
    readonly property bool hostnameBusy: Settings.generalWriteOperation === "hostname"
        && Settings.generalWriteState === "busy"

    // The mockup's three sections. Only services the host reports are drawn;
    // a section with no installed service is not a fake row.
    readonly property var serviceSections: {
        var content = [];
        var accessories = [];
        var advanced = [];
        for (var i = 0; i < root.services.length; ++i) {
            var service = root.services[i];
            var id = String(service.id || "");
            if (id === "file" || id === "media" || id === "screen"
                    || id === "cache")
                content.push(service);
            else if (id === "bluetooth" || id === "printer"
                     || id === "internet")
                accessories.push(service);
            else
                advanced.push(service);
        }
        return [
            { id: "content", title: qsTr("Content & Media"), items: content },
            { id: "accessories", title: qsTr("Accessories & Internet"), items: accessories },
            { id: "advanced", title: qsTr("Advanced"), items: advanced }
        ];
    }

    property alias contentRepeater: contentRepeater
    property alias accessoriesRepeater: accessoriesRepeater
    property alias advancedRepeater: advancedRepeater
    property alias contentGroup: contentGroup
    property alias accessoriesGroup: accessoriesGroup
    property alias advancedGroup: advancedGroup
    property alias emptyNote: emptyNote
    property alias absenceNote: absenceNote
    property alias hostnameField: hostnameField
    property alias hostnameButton: hostnameButton
    property alias hostnameRow: hostnameRow
    property alias optionsDialog: optionsDialog
    property alias servicesGroup: servicesGroup

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function validateHostname(value) {
        const text = String(value || "").trim();
        if (text.length === 0)
            return qsTr("The hostname cannot be empty.");
        if (text.length > 63)
            return qsTr("The hostname cannot be longer than 63 characters.");
        if (text.startsWith("-") || text.endsWith("-"))
            return qsTr("The hostname cannot start or end with a hyphen.");
        if (!/^[A-Za-z0-9-]+$/.test(text))
            return qsTr("The hostname can only contain letters, digits, and hyphens.");
        return "";
    }

    function applyHostname() {
        const value = root.hostnameField.text.trim();
        if (root.validateHostname(value).length > 0)
            return;
        if (value === root.hostname)
            return;
        Settings.setHostname(value);
    }

    function showOptions(service) {
        root.optionsService = service;
        optionsDialog.show();
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        Text {
            width: parent.width
            text: qsTr("Share this computer's files, screen, printers, and remote login with other devices.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Computer")

            SettingsRow {
                id: hostnameRow
                objectName: "sharingHostnameRow"
                width: parent.width
                label: qsTr("Local hostname")
                description: root.available
                    ? (root.hostnameEditable
                       ? qsTr("Other computers use this name to reach this one.")
                       : root.hostnameReason)
                    : qsTr("Unavailable")
                errorText: root.hostnameError
                showSeparator: false
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Rectangle {
                        width: 140
                        height: Theme.controls.select.height
                        radius: Theme.controls.select.radius
                        color: Theme.color.controlFill
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        TextInput {
                            id: hostnameField
                            objectName: "sharingHostnameField"
                            anchors.fill: parent
                            anchors.leftMargin: Theme.controls.select.paddingH
                            anchors.rightMargin: Theme.controls.select.paddingH
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            enabled: root.available && root.hostnameEditable
                            selectByMouse: true
                            text: root.hostname
                            Accessible.name: qsTr("Local hostname")
                        }
                    }

                    Button {
                        id: hostnameButton
                        objectName: "sharingHostnameEdit"
                        text: root.hostnameBusy ? qsTr("Saving…") : qsTr("Edit")
                        enabled: root.available && root.hostnameEditable
                            && !root.hostnameBusy
                        onClicked: root.applyHostname()
                    }
                }
            }
        }

        Column {
            id: servicesGroup
            width: parent.width
            spacing: Theme.primitive.spacing.lg

            Text {
                id: emptyNote
                objectName: "sharingEmptyNote"
                width: parent.width
                visible: root.available && root.services.length === 0
                text: qsTr("No sharing services are installed on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsGroup {
                id: contentGroup
                width: parent.width
                title: qsTr("Content & Media")
                visible: root.serviceSections[0].items.length > 0

                Repeater {
                    id: contentRepeater
                    model: root.serviceSections[0].items
                    delegate: serviceRow
                }
            }

            SettingsGroup {
                id: accessoriesGroup
                width: parent.width
                title: qsTr("Accessories & Internet")
                visible: root.serviceSections[1].items.length > 0

                Repeater {
                    id: accessoriesRepeater
                    model: root.serviceSections[1].items
                    delegate: serviceRow
                }
            }

            SettingsGroup {
                id: advancedGroup
                width: parent.width
                title: qsTr("Advanced")
                visible: root.serviceSections[2].items.length > 0

                Repeater {
                    id: advancedRepeater
                    model: root.serviceSections[2].items
                    delegate: serviceRow
                }
            }

            Text {
                width: parent.width
                visible: root.sharingError.length > 0
                text: root.sharingError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "sharingAbsenceNote"
                kind: "disabled-with-reason"
                show: !root.available
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Sharing is unavailable because systemd is not reachable.")
            }
        }
    }

    Component {
        id: serviceRow
        SettingsRow {
            required property var modelData
            required property int index
            width: parent.width
            label: modelData.name
            description: modelData.description
            controlData: Row {
                spacing: Theme.primitive.spacing.xs

                Toggle {
                    objectName: "sharingToggle-" + modelData.id
                    checked: modelData.active === true
                    enabled: root.available && modelData.canToggle !== false
                    accessibleName: modelData.name
                    onToggled: (checked) => Settings.setSharingEnabled(
                        modelData.id, checked)
                }

                Button {
                    objectName: "sharingOptions-" + modelData.id
                    text: qsTr("Options…")
                    variant: "ghost"
                    enabled: root.available
                    accessibleName: qsTr("%1 options").arg(modelData.name)
                    onClicked: root.showOptions(modelData)
                }
            }
        }
    }

    Dialog {
        id: optionsDialog
        objectName: "sharingOptionsDialog"
        title: root.optionsService !== null
            ? qsTr("%1 Options").arg(root.optionsService.name) : qsTr("Sharing Options")
        message: root.optionsService !== null
            ? qsTr("Changes require administrator authentication.")
            : ""
        dismissible: true

        contentData: Column {
            width: parent ? parent.width : 0
            spacing: Theme.primitive.spacing.xs

            Repeater {
                model: root.optionsService !== null
                    && root.optionsService.details !== undefined
                    ? root.optionsService.details : []
                delegate: SettingsRow {
                    required property var modelData
                    width: parent.width
                    label: String(modelData.label)
                    showSeparator: false
                    controlData: Text {
                        text: String(modelData.value)
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
            }

            Text {
                width: parent.width
                visible: root.optionsService !== null
                    && (root.optionsService.details === undefined
                        || root.optionsService.details.length === 0)
                text: qsTr("This service reports no additional options.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Button {
            text: qsTr("Done")
            variant: "primary"
            onClicked: optionsDialog.accept()
        }
    }
}
