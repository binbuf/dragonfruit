// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Privacy & Security pane (T-15.13b; T279 completion).
//
// Two owners meet here, and each degrades on its own:
//
//  * Privacy — the macOS categories mapped to the closest Linux portal
//    permission, grouped the way the reference groups them: Location Services
//    (a master gate plus the per-app list), app access to other apps and
//    hardware, Analytics, and the remaining real portal tables. A category
//    with no portal table is explicitly delegated; it never draws a fake
//    TCC-style toggle.
//  * Security — software sources, read-only disk encryption with a delegate
//    sheet, USB accessories (USBGuard), hardening, portal extensions, and the
//    Advanced host-stack sheet.
//
// No per-app permission is cached: the portal store is the state and every
// write is one explicit adapter call followed by a host re-read. The one
// durable preference is the Location Services master (`privacy.locationServices`),
// which gates the pane's per-app location writes because the portal store has
// no global switch. Analytics is opt-in, off by default, and stored only until
// a collector exists.
Item {
    id: root

    // The bridge host's Privacy and Security views (empty when absent).
    readonly property var view: Settings.privacy
    readonly property bool available: Settings.privacyAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var mappedCategories: root.view.macCategories !== undefined
        ? root.view.macCategories : []
    readonly property var otherCategories: root.view.otherCategories !== undefined
        ? root.view.otherCategories : []
    readonly property int appCount: root.view.appCount !== undefined
        ? Number(root.view.appCount) : 0

    // Location Services is its own group: the master gate plus the one
    // disclosure row that opens the per-app list.
    readonly property var locationCategory: root.findMapped("location")
    // Every other macOS category, in the reference order.
    readonly property var appCategories: {
        var out = [];
        for (var i = 0; i < root.mappedCategories.length; ++i)
            if (String(root.mappedCategories[i].id) !== "location")
                out.push(root.mappedCategories[i]);
        return out;
    }
    readonly property bool locationEnabled:
        Settings.values["privacy.locationServices"] !== false
    readonly property bool analyticsEnabled:
        Settings.values["privacy.analytics"] === true

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
    readonly property var encryptionDevices: root.encryption.devices !== undefined
        ? root.encryption.devices : []
    readonly property var hardening: root.security.hardening !== undefined
        ? root.security.hardening : ({})
    readonly property var extensions: root.security.extensions !== undefined
        ? root.security.extensions : []
    readonly property var accessories: root.security.accessories !== undefined
        ? root.security.accessories : ({})
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
    readonly property bool selectedIsLocation:
        root.selectedCategoryId === "location"

    // Test surface.
    property alias locationGroup: locationGroup
    property alias locationMasterToggle: locationMasterToggle
    property alias locationRow: locationRow
    property alias appGroup: appGroup
    property alias categoryRepeater: categoryRepeater
    property alias otherRepeater: otherRepeater
    property alias absenceNote: absenceNote
    property alias emptyNote: emptyNote
    property alias permissionRepeater: permissionRepeater
    property alias permissionDialog: permissionDialog
    property alias noAppsNote: noAppsNote
    property alias locationMasterNote: locationMasterNote
    property alias analyticsGroup: analyticsGroup
    property alias analyticsToggle: analyticsToggle
    property alias securityGroup: securityGroup
    property alias securityAbsenceNote: securityAbsenceNote
    property alias sourcesButton: sourcesButton
    property alias sourcesDialog: sourcesDialog
    property alias sourceRepeater: sourceRepeater
    property alias encryptionButton: encryptionButton
    property alias encryptionDialog: encryptionDialog
    property alias encryptionDeviceRepeater: encryptionDeviceRepeater
    property alias accessoriesButton: accessoriesButton
    property alias accessoriesDialog: accessoriesDialog
    property alias accessoriesNote: accessoriesNote
    property alias extensionsButton: extensionsButton
    property alias extensionsDialog: extensionsDialog
    property alias advancedButton: advancedButton
    property alias advancedDialog: advancedDialog

    // The shell reparents dialogs to its root so the detail ScrollView's clip
    // cannot cut them off (T227 note).

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

    // The category glyph vocabulary. A category without an entry falls back to
    // the pane glyph so a future portal table is still readable.
    readonly property var categoryIcons: ({
        "location": "privacy-location",
        "contacts": "privacy-contacts",
        "calendars": "privacy-calendars",
        "reminders": "privacy-reminders",
        "photos": "privacy-photos",
        "camera": "privacy-camera",
        "microphone": "privacy-microphone",
        "speech": "privacy-speech",
        "accessibility": "privacy-accessibility",
        "fullDisk": "privacy-full-disk",
        "files": "privacy-files",
        "screenRecording": "privacy-screen-recording",
        "automation": "privacy-automation",
        "appManagement": "privacy-app-management",
        "developerTools": "privacy-developer-tools"
    })

    // Opening the pane asks both hosts for a re-read (a no-op when absent).
    Component.onCompleted: {
        Settings.refreshPrivacy();
        Settings.refreshSecurity();
    }

    function iconFor(id) {
        return root.categoryIcons[id] !== undefined
            ? String(root.categoryIcons[id]) : "privacy";
    }

    function findMapped(id) {
        for (var i = 0; i < root.mappedCategories.length; ++i)
            if (String(root.mappedCategories[i].id) === id)
                return root.mappedCategories[i];
        return null;
    }

    function findCategory(id) {
        var mapped = root.findMapped(id);
        if (mapped !== null)
            return mapped;
        for (var j = 0; j < root.otherCategories.length; ++j)
            if (String(root.otherCategories[j].id) === id)
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

    function accessoriesLabel() {
        if (root.accessories.label !== undefined)
            return String(root.accessories.label);
        return root.accessories.available == true
            ? qsTr("Managed") : qsTr("Not installed");
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Location Services ─────────────────────────────────────────────
        SettingsGroup {
            id: locationGroup
            objectName: "privacyLocationGroup"
            width: parent.width
            visible: root.ready
            title: qsTr("Location Services")

            SettingsRow {
                objectName: "privacyLocationMasterRow"
                width: parent.width
                label: qsTr("Location Services")
                description: qsTr("The portal has no global switch; turning this off "
                                  + "disables the per-app location controls below.")
                settingsId: "locationMaster"
                settingsKey: "privacy.locationServices"
                controlData: Toggle {
                    id: locationMasterToggle
                    objectName: "privacyLocationMasterToggle"
                    accessibleName: qsTr("Location Services")
                    checked: root.locationEnabled
                    onToggled: (value) => Settings.set("privacy.locationServices", value)
                }
            }

            SettingsRow {
                id: locationRow
                objectName: "privacyLocationRow"
                width: parent.width
                leadingIcon: root.iconFor("location")
                label: root.locationCategory !== null
                    ? String(root.locationCategory.label) : qsTr("Location")
                description: root.locationCategory !== null
                    ? String(root.locationCategory.difference) : ""
                settingsId: "location"
                disclosure: true
                showSeparator: false
                onDisclosureClicked: root.openCategory("location")
                controlData: Text {
                    objectName: "privacyLocationSummary"
                    text: root.locationCategory !== null
                        ? String(root.locationCategory.summary) : ""
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            Text {
                id: locationMasterNote
                objectName: "privacyLocationMasterNote"
                width: parent.width
                visible: !root.locationEnabled
                text: qsTr("Location Services is off. Per-app location writes are disabled.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── App access to other apps and hardware ─────────────────────────
        SettingsGroup {
            id: appGroup
            objectName: "privacyAppGroup"
            width: parent.width
            visible: root.ready
            title: qsTr("App access")

            Repeater {
                id: categoryRepeater
                model: root.appCategories
                delegate: SettingsRow {
                    id: categoryRow
                    required property var modelData
                    required property int index
                    width: parent.width
                    leadingIcon: root.iconFor(String(modelData.id))
                    label: modelData.label
                    description: modelData.difference
                    settingsId: String(modelData.id)
                    disclosure: true
                    showSeparator: categoryRow.index < root.appCategories.length - 1
                    onDisclosureClicked: root.openCategory(modelData.id)
                    controlData: Text {
                        objectName: "privacyCategorySummary"
                        text: categoryRow.modelData.summary !== undefined
                            ? String(categoryRow.modelData.summary) : ""
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Accessible.role: Accessible.Grouping
                    Accessible.name: categoryRow.label
                    Accessible.description: categoryRow.description
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

        // ── Analytics ─────────────────────────────────────────────────────
        SettingsGroup {
            id: analyticsGroup
            objectName: "privacyAnalyticsGroup"
            width: parent.width
            visible: root.ready
            title: qsTr("Analytics & Improvements")

            SettingsRow {
                objectName: "privacyAnalyticsRow"
                width: parent.width
                leadingIcon: "privacy-analytics"
                label: qsTr("Share analytics with the distribution")
                description: qsTr("Off by default. This build has no analytics collector, "
                                  + "so the choice is stored only.")
                settingsId: "analytics"
                settingsKey: "privacy.analytics"
                showSeparator: false
                controlData: Toggle {
                    id: analyticsToggle
                    objectName: "privacyAnalyticsToggle"
                    accessibleName: qsTr("Share analytics with the distribution")
                    checked: root.analyticsEnabled
                    onToggled: (value) => Settings.set("privacy.analytics", value)
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
                leadingIcon: "software-update"
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
                leadingIcon: "privacy-encryption"
                label: qsTr("Disk encryption")
                description: root.encryption.note !== undefined
                    ? String(root.encryption.note) : ""
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        objectName: "privacyEncryptionLabel"
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.encryption.label !== undefined
                            ? String(root.encryption.label) : qsTr("Unknown")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Button {
                        id: encryptionButton
                        objectName: "privacyEncryptionButton"
                        text: qsTr("Open\u2026")
                        enabled: root.securityReady
                            && root.encryption.available == true
                        onClicked: root.encryptionDialog.show()
                    }
                }
            }

            SettingsRow {
                width: parent.width
                leadingIcon: "privacy-accessories"
                label: qsTr("USB accessories")
                description: root.securityReady
                    ? (root.accessories.note !== undefined
                       ? String(root.accessories.note)
                       : qsTr("USBGuard owns the device policy."))
                    : qsTr("The security host stack is unavailable.")
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        objectName: "privacyAccessoriesLabel"
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.accessoriesLabel()
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Button {
                        id: accessoriesButton
                        objectName: "privacyAccessoriesButton"
                        text: qsTr("Details\u2026")
                        enabled: root.securityReady
                            && root.accessories.available == true
                        onClicked: root.accessoriesDialog.show()
                    }
                }
            }

            SettingsRow {
                width: parent.width
                leadingIcon: "privacy-hardening"
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

        SettingsAbsenceNotice {
            id: securityAbsenceNote
            objectName: "privacySecurityAbsenceNote"
            kind: "read-only"
            show: !root.securityReady
            reason: qsTr("Security status is unavailable because the host "
                         + "tools did not answer.")
        }
    }

    // ── The per-category permission dialog ────────────────────────────────
    Dialog {
        id: permissionDialog
        objectName: "privacyPermissionDialog"
        title: root.selectedCategory !== null
            ? String(root.selectedCategory.label) : qsTr("Permissions")
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
                width: parent.width
                visible: root.selectedIsLocation && !root.locationEnabled
                text: qsTr("Location Services is off; these per-app choices are "
                           + "read-only until it is turned back on.")
                color: Theme.color.textSecondary
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
                        enabled: !root.selectedIsLocation || root.locationEnabled
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

    // ── Disk encryption (read-only + delegate) ───────────────────────────
    Dialog {
        id: encryptionDialog
        objectName: "privacyEncryptionDialog"
        title: qsTr("Disk encryption")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: root.encryption.note !== undefined
                    ? String(root.encryption.note) : ""
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Repeater {
                id: encryptionDeviceRepeater
                model: root.encryptionDevices
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.name
                    description: (modelData.kind !== undefined
                                  ? String(modelData.kind) : "")
                                 + (modelData.fstype !== undefined
                                    && String(modelData.fstype).length > 0
                                    ? " \u00b7 " + String(modelData.fstype) : "")
                    showSeparator: index < root.encryptionDevices.length - 1
                    controlData: Text {
                        text: modelData.mountpoints !== undefined
                            ? modelData.mountpoints.join(", ") : ""
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }
            }

            Text {
                width: parent.width
                text: qsTr("Unlocking and recovery stay with the distribution's "
                           + "cryptsetup tools; Settings reports the state only.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: encryptionDialog.accept()
            }
        }
    }

    // ── USB accessories (USBGuard) ───────────────────────────────────────
    Dialog {
        id: accessoriesDialog
        objectName: "privacyAccessoriesDialog"
        title: qsTr("USB accessories")
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                id: accessoriesNote
                objectName: "privacyAccessoriesNote"
                width: parent.width
                text: root.accessories.note !== undefined
                    ? String(root.accessories.note) : ""
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Policy")
                description: qsTr("Reported by USBGuard; Settings never edits a rule.")
                showSeparator: false
                controlData: Text {
                    objectName: "privacyAccessoriesPolicy"
                    text: root.accessoriesLabel()
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            Text {
                width: parent.width
                text: qsTr("Manage devices with the distribution's usbguard command.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Done")
                variant: "primary"
                onClicked: accessoriesDialog.accept()
            }
        }
    }

    // ── Installed portal extensions ──────────────────────────────────────
    Dialog {
        id: extensionsDialog
        objectName: "privacyExtensionsDialog"
        title: qsTr("Extensions")
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
