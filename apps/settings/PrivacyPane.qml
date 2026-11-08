// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Privacy & Security pane (T-15.13b; T228 completion).
//
// The pane has two halves, both read from the bridge host:
//
//  * Privacy — the macOS reference categories mapped to the closest Linux
//    portal permission. A row states the Linux difference, and opening it
//    shows the per-app tristate the portal PermissionStore holds. Categories
//    with no portal table are explicitly delegated (address book, AT-SPI,
//    polkit, …); they never draw a fake TCC-style toggle. The remaining portal
//    tables (Notifications, USB, Remote Desktop, …) are listed under "Other
//    portal permissions" because they are real host state even though macOS
//    has no matching row.
//  * Security — software sources, read-only disk encryption and hardening,
//    installed portal extensions, and an Advanced host-stack sheet.
//
// No durable settingsd preference lives on this pane: the portal store and the
// host tools are the state. Every write is one explicit adapter call and the
// host re-reads, so the pane never invents a result.
Item {
    id: root

    // The bridge host's Privacy and Security views (empty when absent).
    readonly property var view: Settings.privacy
    readonly property bool available: Settings.privacyAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var categories: root.view.categories !== undefined
        ? root.view.categories : []
    readonly property var mappedCategories: root.view.macCategories !== undefined
        ? root.view.macCategories : []
    readonly property var otherCategories: root.view.otherCategories !== undefined
        ? root.view.otherCategories : []
    readonly property int appCount: root.view.appCount !== undefined
        ? Number(root.view.appCount) : 0

    readonly property var security: Settings.security
    readonly property bool securityAvailable: Settings.securityAvailable
    readonly property bool securityReady: root.securityAvailable
        && root.security.state === "available"
    readonly property var softwareSources: root.security.softwareSources !== undefined
        ? root.security.softwareSources : ({})
    readonly property var securitySources: root.softwareSources.sources !== undefined
        ? root.softwareSources.sources : []
    readonly property var encryption: root.security.encryption !== undefined
        ? root.security.encryption : ({})
    readonly property var hardening: root.security.hardening !== undefined
        ? root.security.hardening : ({})
    readonly property var extensions: root.security.extensions !== undefined
        ? root.security.extensions : []
    readonly property var advancedRows: root.security.advanced !== undefined
        ? root.security.advanced : []
    readonly property int enabledSourceCount:
        root.security.enabledSourceCount !== undefined
        ? Number(root.security.enabledSourceCount) : 0

    // The category whose dialog is open, keyed by id and derived from the live
    // view so a write's round-trip converges in the open dialog.
    property string selectedCategoryId: ""
    readonly property var selectedCategory: root.selectedCategoryId.length > 0
        ? root.findCategory(root.selectedCategoryId) : null
    // The dialog's delegate list, captured once when the category opens. A
    // permission write changes the live view; the Selects re-read the live
    // state through their Binding instead of being rebuilt mid-interaction.
    property var dialogApps: []
    readonly property var selectedApps: root.appsFor(root.selectedCategoryId)

    // Test surface.
    property alias categoryRepeater: categoryRepeater
    property alias otherRepeater: otherRepeater
    property alias absenceNote: absenceNote
    property alias emptyNote: emptyNote
    property alias permissionRepeater: permissionRepeater
    property alias permissionDialog: permissionDialog
    property alias noAppsNote: noAppsNote
    property alias securityGroup: securityGroup
    property alias securityAbsenceNote: securityAbsenceNote
    property alias sourcesButton: sourcesButton
    property alias sourcesDialog: sourcesDialog
    property alias sourceRepeater: sourceRepeater
    property alias extensionsButton: extensionsButton
    property alias extensionsDialog: extensionsDialog
    property alias advancedButton: advancedButton
    property alias advancedDialog: advancedDialog

    // The shell reparents dialogs to its root so the detail ScrollView's clip
    // cannot cut them off (T227 note).
    property Item popupParent: null

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    // The tristate + removal choices (the stable ids the adapter understands).
    readonly property var permissionModel: [
        { label: qsTr("Allowed"), value: "allowed" },
        { label: qsTr("Denied"), value: "denied" },
        { label: qsTr("Ask"), value: "ask" },
        { label: qsTr("Not Set"), value: "unset" }
    ]

    // Opening the pane asks both hosts for a re-read (a no-op when absent).
    Component.onCompleted: {
        Settings.refreshPrivacy();
        Settings.refreshSecurity();
    }

    function findCategory(id) {
        for (var i = 0; i < root.mappedCategories.length; ++i)
            if (root.mappedCategories[i].id === id)
                return root.mappedCategories[i];
        for (var j = 0; j < root.otherCategories.length; ++j)
            if (root.otherCategories[j].id === id)
                return root.otherCategories[j];
        return null;
    }

    // Flatten a category's resources into one row per application.
    function appsFor(id) {
        var out = [];
        var category = root.findCategory(id);
        if (category === null)
            return out;
        if (category.apps !== undefined) {
            for (var i = 0; i < category.apps.length; ++i)
                out.push(category.apps[i]);
            return out;
        }
        var resources = category.resources !== undefined ? category.resources : [];
        for (var j = 0; j < resources.length; ++j) {
            var apps = resources[j].apps !== undefined ? resources[j].apps : [];
            for (var k = 0; k < apps.length; ++k) {
                out.push({
                    app: apps[k].app,
                    table: category.id,
                    resourceId: resources[j].id,
                    state: apps[k].state !== undefined ? String(apps[k].state) : "unset",
                    stateLabel: apps[k].stateLabel !== undefined
                        ? String(apps[k].stateLabel) : ""
                });
            }
        }
        return out;
    }

    // The live tristate for one application in the open category.
    function permissionStateFor(app, table, resourceId) {
        var category = root.findCategory(root.selectedCategoryId);
        if (category === null)
            return "unset";
        if (category.apps !== undefined) {
            for (var i = 0; i < category.apps.length; ++i)
                if (String(category.apps[i].app) === String(app)
                        && String(category.apps[i].table) === String(table)
                        && String(category.apps[i].resourceId) === String(resourceId))
                    return String(category.apps[i].state);
            return "unset";
        }
        var resources = category.resources !== undefined ? category.resources : [];
        for (var j = 0; j < resources.length; ++j) {
            if (String(resources[j].id) !== String(resourceId))
                continue;
            var apps = resources[j].apps !== undefined ? resources[j].apps : [];
            for (var k = 0; k < apps.length; ++k)
                if (String(apps[k].app) === String(app))
                    return apps[k].state !== undefined ? String(apps[k].state) : "unset";
        }
        return "unset";
    }

    function openCategory(id) {
        root.selectedCategoryId = String(id);
        root.dialogApps = root.appsFor(root.selectedCategoryId);
        root.permissionDialog.show();
    }

    function permissionIndex(state) {
        for (var i = 0; i < root.permissionModel.length; ++i)
            if (root.permissionModel[i].value === state)
                return i;
        return 3;
    }

    function sourceSummary() {
        if (!root.securityReady || root.softwareSources.available !== true)
            return qsTr("Unavailable");
        var names = [];
        for (var i = 0; i < root.securitySources.length; ++i)
            if (root.securitySources[i].enabled === true)
                names.push(String(root.securitySources[i].name));
        return names.length === 0 ? qsTr("None enabled") : names.join(", ");
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The macOS privacy rows mapped to Linux ────────────────────────
        Column {
            width: parent.width
            visible: root.ready

            Repeater {
                id: categoryRepeater
                model: root.mappedCategories
                delegate: Item {
                    id: categoryRow
                    required property var modelData
                    required property int index
                    readonly property var category: modelData
                    width: parent ? parent.width : 0
                    height: 64
                    Accessible.role: Accessible.Grouping
                    Accessible.name: categoryRow.category.label
                    Accessible.description: categoryRow.category.difference

                    Icon {
                        id: categoryIcon
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        name: "privacy"
                        size: 20
                        color: Theme.color.textSecondary
                    }

                    Column {
                        anchors.left: categoryIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: categorySummary.left
                        anchors.rightMargin: Theme.controls.settingsRow.controlGap
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "privacyCategoryLabel"
                            width: parent.width
                            text: categoryRow.category.label
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Text {
                            objectName: "privacyCategoryDifference"
                            width: parent.width
                            text: categoryRow.category.difference
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        id: categorySummary
                        objectName: "privacyCategorySummary"
                        anchors.right: categoryChevron.left
                        anchors.rightMargin: Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                        text: categoryRow.category.summary !== undefined
                            ? String(categoryRow.category.summary) : ""
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                    }

                    Icon {
                        id: categoryChevron
                        objectName: "privacyCategoryChevron"
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        name: "chevron-right"
                        size: 14
                        color: Theme.color.textTertiary
                    }

                    MouseArea {
                        objectName: "privacyCategoryArea"
                        anchors.fill: parent
                        onClicked: root.openCategory(categoryRow.category.id)
                    }

                    Rectangle {
                        visible: categoryRow.index < root.mappedCategories.length - 1
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.bottom: parent.bottom
                        height: Theme.controls.window.borderWidth
                        color: Theme.color.separator
                    }
                }
            }

            Text {
                id: emptyNote
                objectName: "privacyEmptyNote"
                width: parent.width
                visible: root.ready && root.appCount === 0
                text: qsTr("No application has requested a permission yet.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.sm
            }
        }

        // ── The remaining real portal tables ──────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.ready && root.otherCategories.length > 0
            title: qsTr("Other portal permissions")

            Repeater {
                id: otherRepeater
                model: root.otherCategories
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.label
                    description: qsTr("Portal permission table: %1").arg(modelData.id)
                    disclosure: true
                    showSeparator: index < root.otherCategories.length - 1
                    onDisclosureClicked: root.openCategory(modelData.id)
                }
            }
        }

        // ── Host absent ───────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "privacyAbsenceNote"
                width: parent.width
                text: qsTr("The permission service is not running, so app "
                           + "permissions are unavailable.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Security ──────────────────────────────────────────────────────
        SettingsGroup {
            id: securityGroup
            objectName: "privacySecurityGroup"
            width: parent.width
            title: qsTr("Security")

            SettingsRow {
                width: parent.width
                label: qsTr("Software sources")
                description: root.securityReady
                    ? qsTr("Applications can be installed from %1.").arg(root.sourceSummary())
                    : qsTr("The security host stack is unavailable.")
                controlData: Button {
                    id: sourcesButton
                    objectName: "privacySourcesButton"
                    text: qsTr("Manage\u2026")
                    enabled: root.securityReady
                        && root.softwareSources.available === true
                    onClicked: root.sourcesDialog.show()
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Disk encryption")
                description: root.encryption.note !== undefined
                    ? String(root.encryption.note) : ""
                controlData: Text {
                    objectName: "privacyEncryptionLabel"
                    text: root.encryption.label !== undefined
                        ? String(root.encryption.label) : qsTr("Unknown")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Hardening mode")
                description: root.hardening.detail !== undefined
                    ? String(root.hardening.detail) : ""
                controlData: Text {
                    objectName: "privacyHardeningLabel"
                    text: root.hardening.label !== undefined
                        ? String(root.hardening.label) : qsTr("Unknown")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Extensions")
                description: root.extensions.length === 0
                    ? qsTr("No portal extensions are installed.")
                    : qsTr("%1 portal extension(s) installed.").arg(root.extensions.length)
                controlData: Button {
                    id: extensionsButton
                    objectName: "privacyExtensionsButton"
                    text: qsTr("View\u2026")
                    enabled: root.securityReady && root.extensions.length > 0
                    onClicked: root.extensionsDialog.show()
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Advanced\u2026")
                description: qsTr("Authorization, firewall, Secure Boot, and the host stack.")
                showSeparator: false
                controlData: Button {
                    id: advancedButton
                    objectName: "privacyAdvancedButton"
                    text: qsTr("Open\u2026")
                    enabled: root.securityReady
                    onClicked: root.advancedDialog.show()
                }
            }
        }

        Text {
            id: securityAbsenceNote
            objectName: "privacySecurityAbsenceNote"
            width: parent.width
            visible: !root.securityReady
            text: qsTr("Security status is unavailable because the host "
                       + "tools did not answer.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
    }

    // ── The per-category permission dialog ────────────────────────────────
    Dialog {
        id: permissionDialog
        objectName: "privacyPermissionDialog"
        title: root.selectedCategory !== null
            ? String(root.selectedCategory.label) : qsTr("Permissions")
        parent: root.popupParent ? root.popupParent : root
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: root.selectedCategory !== null
                    && root.selectedCategory.difference !== undefined
                    ? String(root.selectedCategory.difference) : ""
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: root.selectedCategory !== null
                    && root.selectedCategory.delegated === true
                text: root.selectedCategory !== null
                    && root.selectedCategory.linuxOwner !== undefined
                    ? qsTr("Linux owner: %1").arg(root.selectedCategory.linuxOwner)
                    : ""
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                id: noAppsNote
                objectName: "privacyNoApps"
                width: parent.width
                visible: root.dialogApps.length === 0
                text: root.selectedCategory !== null
                    && root.selectedCategory.delegated === true
                    ? qsTr("This category is delegated; the portal store keeps no per-app record.")
                    : qsTr("No app has requested this permission.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: permissionRepeater
                model: root.dialogApps
                delegate: SettingsRow {
                    id: permissionRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    showSeparator: permissionRow.index < root.dialogApps.length - 1
                    label: permissionRow.modelData.app
                    description: permissionRow.modelData.resourceId !== undefined
                        ? String(permissionRow.modelData.resourceId) : ""
                    controlData: Select {
                        id: permissionSelect
                        objectName: "privacyPermissionSelect"
                        accessibleName: qsTr("Permission for %1").arg(permissionRow.modelData.app)
                        model: root.permissionModel
                        onSelected: (value) => {
                            if (String(value) === "unset")
                                Settings.deletePrivacyPermission(
                                    String(permissionRow.modelData.table),
                                    String(permissionRow.modelData.resourceId),
                                    String(permissionRow.modelData.app));
                            else
                                Settings.setPrivacyPermission(
                                    String(permissionRow.modelData.table),
                                    String(permissionRow.modelData.resourceId),
                                    String(permissionRow.modelData.app),
                                    String(value));
                        }

                        Binding {
                            target: permissionSelect
                            property: "currentIndex"
                            value: root.permissionIndex(
                                root.permissionStateFor(
                                    permissionRow.modelData.app,
                                    permissionRow.modelData.table,
                                    permissionRow.modelData.resourceId))
                        }
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: permissionDialog.accept()
            }
        }
    }

    // ── Software sources ─────────────────────────────────────────────────
    Dialog {
        id: sourcesDialog
        objectName: "privacySourcesDialog"
        title: qsTr("Software sources")
        parent: root.popupParent ? root.popupParent : root
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("Distribution repositories are system policy. "
                           + "Flatpak remotes can be enabled or disabled here.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: sourceRepeater
                model: root.securitySources
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    showSeparator: index < root.securitySources.length - 1
                    label: modelData.name
                    description: modelData.note
                    controlData: Toggle {
                        objectName: "privacySourceToggle"
                        accessibleName: qsTr("Enable %1").arg(modelData.name)
                        checked: modelData.enabled === true
                        enabled: modelData.writable === true
                        onToggled: (value) => Settings.setSecuritySourceEnabled(
                            String(modelData.id), value)
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: sourcesDialog.accept()
            }
        }
    }

    // ── Installed portal extensions ──────────────────────────────────────
    Dialog {
        id: extensionsDialog
        objectName: "privacyExtensionsDialog"
        title: qsTr("Extensions")
        parent: root.popupParent ? root.popupParent : root
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Repeater {
                model: root.extensions
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    showSeparator: index < root.extensions.length - 1
                    label: modelData.name
                    description: (modelData.dbusName !== undefined
                                  ? String(modelData.dbusName) : "")
                                 + (modelData.interfaces !== undefined
                                    ? " \u00b7 " + modelData.interfaces.length
                                      + qsTr(" interface(s)") : "")
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: extensionsDialog.accept()
            }
        }
    }

    // ── Advanced host-stack inventory ────────────────────────────────────
    Dialog {
        id: advancedDialog
        objectName: "privacyAdvancedDialog"
        title: qsTr("Advanced")
        parent: root.popupParent ? root.popupParent : root
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Repeater {
                model: root.advancedRows
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    showSeparator: index < root.advancedRows.length - 1
                    label: modelData.label
                    description: modelData.detail
                    controlData: Text {
                        text: modelData.value
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: advancedDialog.accept()
            }
        }
    }
}
