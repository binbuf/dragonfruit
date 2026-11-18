// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Login Items & Extensions sub-pane (T226; T270 completion).
//
// XDG autostart entries (`~/.config/autostart`, `/etc/xdg/autostart`) and
// systemd user units are the host's. A system entry is disabled with a user
// override, never edited in place. Extensions are a delegated portal surface,
// grouped by the backend that serves them instead of fake toggles.
//
// T270: macOS labels, add/remove login items, a broken-item warning, and a
// grouped Extensions list. Every group stays visible with its own absence note
// when the adapter is gone.
Item {
    id: root

    readonly property var view: Settings.general.login !== undefined
        ? Settings.general.login : ({})
    readonly property bool available: Settings.generalAvailable
        && root.view.state === "available"
    readonly property var autostart: root.view.autostart !== undefined
        ? root.view.autostart : []
    readonly property var userUnits: root.view.userUnits !== undefined
        ? root.view.userUnits : []
    readonly property var availableApps: root.view.availableApps !== undefined
        ? root.view.availableApps : []
    readonly property var extensions: root.view.extensions !== undefined
        ? root.view.extensions : []
    readonly property string extensionsNote: root.view.extensionsNote !== undefined
        ? String(root.view.extensionsNote) : ""

    readonly property var brokenItems: root.autostart.filter(function(item) {
        return item.broken === true;
    })
    readonly property string addModel: ""
    readonly property var addAppModel: root.availableApps.map(function(app) {
        return { label: String(app.name), value: String(app.id) };
    })
    // Group the extension providers by the surface they serve. A group with no
    // entries is not drawn.
    readonly property var extensionGroups: {
        var groups = [];
        for (var i = 0; i < root.extensions.length; ++i) {
            var extension = root.extensions[i];
            var category = String(extension.category || qsTr("Extensions"));
            var found = null;
            for (var g = 0; g < groups.length; ++g) {
                if (groups[g].category === category) {
                    found = groups[g];
                    break;
                }
            }
            if (found === null) {
                found = { category: category, items: [] };
                groups.push(found);
            }
            found.items.push(extension);
        }
        return groups;
    }
    readonly property string loginError: Settings.generalWriteOperation === "login"
        && (Settings.generalWriteState === "denied"
            || Settings.generalWriteState === "failed")
        ? Settings.generalWriteError : ""
    // Extensions can be read as one flat "By App" list or grouped by the
    // category the backend reports. Both views are the same managed data.
    property int extensionMode: 0

    property alias autostartRepeater: autostartRepeater
    property alias userUnitRepeater: userUnitRepeater
    property alias addPicker: addPicker
    property alias extensionsRepeater: extensionsRepeater
    property alias extensionsFlatRepeater: extensionsFlatRepeater
    property alias extensionModeControl: extensionModeControl
    property alias brokenRow: brokenRow
    property alias startGroup: startGroup
    property alias backgroundGroup: backgroundGroup
    property alias extensionsGroup: extensionsGroup
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsIntro {
            width: parent.width
            text: qsTr("Choose which apps open at login and which background services run with your session.")
        }

        SettingsGroup {
            id: startGroup
            width: parent.width
            title: qsTr("Open at Login")

            SettingsNote {
                width: parent.width
                text: qsTr("These items will open automatically when you log in.")
            }

            // A broken item is visible, named, and explained. The row stays in
            // the list so the user can remove it.
            SettingsRow {
                id: brokenRow
                objectName: "loginBrokenRow"
                width: parent.width
                visible: root.available && root.brokenItems.length > 0
                label: qsTr("%1 login item needs attention").arg(root.brokenItems.length)
                description: root.brokenItems.map(function(item) {
                    return item.name + ": " + String(item.brokenReason || "");
                }).join(" ")
                controlData: Text {
                    text: "!" // df-allow-untranslated: warning glyph
                    color: Theme.color.danger
                    font.pixelSize: Theme.primitive.font.sizeLg
                    font.weight: Theme.primitive.font.weightSemibold
                }
            }

            Text {
                width: parent.width
                visible: root.available && root.autostart.length === 0
                text: qsTr("No apps open at login.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: autostartRepeater
                model: root.autostart
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: (modelData.source === "system" ? qsTr("System") : qsTr("You"))
                        + (modelData.broken === true
                           ? " · " + qsTr("Cannot start: %1").arg(modelData.brokenReason)
                           : " · " + modelData.command)
                    showSeparator: index < root.autostart.length - 1
                    controlData: Row {
                        spacing: Theme.primitive.spacing.xs

                        Toggle {
                            objectName: "loginToggle" + index
                            checked: modelData.enabled === true
                            enabled: root.available && modelData.canToggle !== false
                            accessibleName: qsTr("Open %1 at login").arg(modelData.name)
                            onToggled: (checked) => Settings.setAutostartEnabled(
                                modelData.id, checked)
                        }

                        Button {
                            objectName: "loginRemove" + index
                            text: qsTr("Remove")
                            variant: "ghost"
                            enabled: root.available
                            accessibleName: qsTr("Remove %1 from login items").arg(
                                modelData.name)
                            onClicked: Settings.removeAutostart(modelData.id)
                        }
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Add Login Item…")
                description: root.available
                    ? (root.availableApps.length > 0
                       ? qsTr("Choose an installed application.")
                       : qsTr("No other installed applications were found."))
                    : qsTr("Unavailable")
                errorText: root.loginError
                showSeparator: false
                controlData: SearchablePicker {
                    id: addPicker
                    objectName: "loginAddPicker"
                    model: root.addAppModel
                    currentIndex: -1
                    placeholderLabel: qsTr("Add…")
                    placeholderText: qsTr("Search applications")
                    emptyText: qsTr("No application matches.")
                    enabled: root.available && root.addAppModel.length > 0
                    accessibleName: qsTr("Add login item")
                    onActivated: (index) => {
                        if (index >= 0 && index < root.addAppModel.length)
                            Settings.addAutostart(String(root.addAppModel[index].value));
                    }
                }
            }

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "loginAbsenceNote"
                kind: "disabled-with-reason"
                show: !root.available
                reason: root.view.error !== undefined && root.view.error !== null
                    ? String(root.view.error)
                    : qsTr("Login items are unavailable because the session services are not running.")
            }
        }

        SettingsGroup {
            id: backgroundGroup
            width: parent.width
            title: qsTr("Background App Activity")

            SettingsNote {
                width: parent.width
                text: qsTr("These services can run in the background to sync data and check for updates.")
            }

            Text {
                width: parent.width
                visible: root.available && root.userUnits.length === 0
                text: qsTr("No systemd user services are installed.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Repeater {
                id: userUnitRepeater
                model: root.userUnits
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: modelData.id + " · " + modelData.active
                    showSeparator: index < root.userUnits.length - 1
                    controlData: Toggle {
                        objectName: "backgroundToggle" + index
                        checked: modelData.enabled === "enabled"
                        enabled: root.available && modelData.canToggle !== false
                        accessibleName: qsTr("Allow %1 in the background").arg(modelData.name)
                        onToggled: (checked) => Settings.setUserUnitEnabled(
                            modelData.id, checked)
                    }
                }
            }

            SettingsAbsenceNotice {
                kind: "disabled-with-reason"
                show: !root.available
                reason: qsTr("Background services are unavailable because the systemd user session is not running.")
            }
        }

        SettingsGroup {
            id: extensionsGroup
            width: parent.width
            title: qsTr("Extensions")

            SettingsNote {
                width: parent.width
                text: qsTr("Extensions add extra functionality to the system and apps, and some may run in the background.")
            }

            SegmentedControl {
                id: extensionModeControl
                objectName: "loginExtensionMode"
                model: [qsTr("By App"), qsTr("By Category")]
                currentIndex: root.extensionMode
                accessibleName: qsTr("Group extensions")
                onActivated: (index) => root.extensionMode = index
            }

            Repeater {
                id: extensionsFlatRepeater
                visible: root.extensionMode === 0
                model: root.extensions
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: modelData.detail
                    showSeparator: index < root.extensions.length - 1
                    controlData: Text {
                        text: qsTr("Managed")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }

            Repeater {
                id: extensionsRepeater
                visible: root.extensionMode === 1
                model: root.extensionGroups
                delegate: Column {
                    required property var modelData
                    width: parent.width
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        width: parent.width
                        text: modelData.category
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.weight: Theme.primitive.font.weightSemibold
                    }

                    Repeater {
                        model: modelData.items
                        delegate: SettingsRow {
                            required property var modelData
                            width: parent.width
                            label: modelData.name
                            description: modelData.detail
                            showSeparator: false
                            controlData: Text {
                                text: qsTr("Managed")
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                        }
                    }
                }
            }

            Text {
                id: extensionsText
                objectName: "loginExtensionsNote"
                width: parent.width
                visible: root.extensions.length === 0
                text: root.extensionsNote.length > 0
                    ? root.extensionsNote
                    : qsTr("Extensions are managed by the desktop portal and installed apps.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                text: qsTr("Extensions are enabled and configured by the application that provides them.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }
}
