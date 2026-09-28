// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Privacy & Security pane (T-15.13b).
//
// The pane and the Control Center tile are one functional unit: both read the
// same host-stack adapter (the xdg-desktop-portal PermissionStore) through the
// bridge host. The pane mirrors the macOS pane
// (SystemSettings_PrivacySecurity.md / System_Preferences.md) as a flat list of
// disclosure rows — one per portal permission category — without inset group
// cards (ADR 0122 names Privacy & Security as a flat list). Opening a row shows
// the category's applications and lets each one's permission be set to allowed,
// denied, or ask, or removed entirely. Those are the explicit adapter writes;
// there is no durable settingsd preference on this pane, because the store is
// the state (T-15.13a/ADR 0142).
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * The categories are the fourteen portal permission tables, not the macOS
//     rows (Calendars, Contacts, Photos, …): on Linux the desktop's honest
//     privacy record is what the portals store. A table with no entry is an
//     empty row (`None`).
//   * `Location Services`, `Full Disk Access`, `Home`, and `Media & Apple
//     Music` have no separate Linux host owner; the portal table that maps is
//     used where one exists (`location`) and no row is invented otherwise.
//   * `Passkeys Access for Web Browsers` maps to the host Secret Service in a
//     later task; it is not shown here (reuse-only, no row of its own).
//   * `Learn more...` and the trailing `?` help are omitted project-wide.
//   * The macOS `Allow`/`Deny` toggles become a four-way selector
//     (`Allowed`/`Denied`/`Ask`/`Not Set`) so the portal's tristate and its
//     removal are all reachable.
Item {
    id: root

    // The bridge host's Privacy and Security view (empty when absent).
    readonly property var view: Settings.privacy
    readonly property bool available: Settings.privacyAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var categories: root.view.categories !== undefined
        ? root.view.categories : []
    readonly property int appCount: root.view.appCount !== undefined
        ? Number(root.view.appCount) : 0

    // The category whose dialog is open, keyed by table id and derived from the
    // live view so a write's round-trip converges in the open dialog.
    property string selectedCategoryId: ""
    readonly property var selectedCategory: root.selectedCategoryId.length > 0
        ? root.findCategory(root.selectedCategoryId) : null
    // The dialog's delegate list, captured once when the category opens. It is
    // deliberately not a live binding: a permission write changes the live
    // view, and a Repeater model reset would destroy the very Select that is
    // mid-interaction. The Selects re-read the live state through their
    // Binding instead, so the rows converge without being rebuilt.
    property var dialogApps: []
    // The flattened application rows of the open category, from the live view
    // (introspection/tests; the dialog renders `dialogApps`).
    readonly property var selectedApps: root.appsFor(root.selectedCategoryId)

    // Test surface (used by tst_settings_privacy.qml).
    property alias categoryRepeater: categoryRepeater
    property alias absenceNote: absenceNote
    property alias emptyNote: emptyNote
    property alias permissionRepeater: permissionRepeater
    property alias permissionDialog: permissionDialog
    property alias noAppsNote: noAppsNote

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

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshPrivacy()

    function findCategory(table) {
        for (var i = 0; i < root.categories.length; ++i)
            if (root.categories[i].id === table)
                return root.categories[i];
        return null;
    }

    // Flatten a category's resources into one row per application.
    function appsFor(table) {
        var out = [];
        var category = root.findCategory(table);
        if (category === null)
            return out;
        var resources = category.resources !== undefined ? category.resources : [];
        for (var i = 0; i < resources.length; ++i) {
            var apps = resources[i].apps !== undefined ? resources[i].apps : [];
            for (var j = 0; j < apps.length; ++j) {
                out.push({
                    app: apps[j].app,
                    resourceId: resources[i].id,
                    state: apps[j].state !== undefined ? String(apps[j].state) : "unset",
                    stateLabel: apps[j].stateLabel !== undefined
                        ? String(apps[j].stateLabel) : ""
                });
            }
        }
        return out;
    }

    // The live tristate for one application in the open category, or `unset`
    // when its record was removed.
    function permissionStateFor(app, resourceId) {
        var category = root.selectedCategory;
        if (category === null)
            return "unset";
        var resources = category.resources !== undefined ? category.resources : [];
        for (var i = 0; i < resources.length; ++i) {
            if (String(resources[i].id) !== String(resourceId))
                continue;
            var apps = resources[i].apps !== undefined ? resources[i].apps : [];
            for (var j = 0; j < apps.length; ++j)
                if (String(apps[j].app) === String(app))
                    return apps[j].state !== undefined
                        ? String(apps[j].state) : "unset";
        }
        return "unset";
    }

    function openCategory(table) {
        root.selectedCategoryId = String(table);
        root.dialogApps = root.appsFor(root.selectedCategoryId);
        root.permissionDialog.show();
    }

    function permissionIndex(state) {
        for (var i = 0; i < root.permissionModel.length; ++i)
            if (root.permissionModel[i].value === state)
                return i;
        return 3;
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The flat category list ────────────────────────────────────────
        Column {
            width: parent.width
            visible: root.ready

            Repeater {
                id: categoryRepeater
                model: root.categories
                delegate: Item {
                    id: categoryRow
                    required property var modelData
                    required property int index
                    readonly property var category: modelData
                    width: parent ? parent.width : 0
                    height: 48
                    Accessible.role: Accessible.Grouping
                    Accessible.name: categoryRow.category.label

                    Icon {
                        id: categoryIcon
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        name: "privacy"
                        size: 20
                        color: Theme.color.textSecondary
                    }

                    Text {
                        objectName: "privacyCategoryLabel"
                        anchors.left: categoryIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: categorySummary.left
                        anchors.rightMargin: Theme.controls.settingsRow.controlGap
                        anchors.verticalCenter: parent.verticalCenter
                        text: categoryRow.category.label
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
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
                        visible: categoryRow.index < root.categories.length - 1
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
                id: noAppsNote
                objectName: "privacyNoApps"
                width: parent.width
                visible: root.dialogApps.length === 0
                text: qsTr("No app has requested this permission.")
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
                                    root.selectedCategoryId,
                                    String(permissionRow.modelData.resourceId),
                                    String(permissionRow.modelData.app));
                            else
                                Settings.setPrivacyPermission(
                                    root.selectedCategoryId,
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
}