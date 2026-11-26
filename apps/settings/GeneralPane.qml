// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// General (T269).
//
// The list is the macOS General page. It never collapses because one adapter
// is absent: every row stays visible and each row's summary reports its own
// half. About/Software Update read the host identity and the distribution
// update provider separately, so About stays live when the provider is gone.
// Every row opens an in-place sub-pane; Software Update is no longer a dialog.
// Storage keeps the top-level `storage` id and deep link as aliases.
Item {
    id: root

    // The bridge host's General/About/Updates view (empty when absent).
    readonly property var view: Settings.updates
    readonly property bool identityAvailable: Settings.updatesAvailable
    readonly property bool identityReady: root.identityAvailable
        && root.view.state === "available"
    readonly property string deviceName: root.view.deviceName !== undefined
        ? String(root.view.deviceName) : ""

    // The distribution update provider, when the host runs one. The host
    // identity and the provider are separate halves of the same view.
    readonly property bool providerAvailable: root.identityReady
        && root.view.updatesAvailable === true
    readonly property bool busy: root.view.busy === true
    readonly property bool rebootRequired: root.view.rebootRequired === true
    readonly property int updateCount: root.view.updateCount !== undefined
        ? root.view.updateCount : 0
    readonly property string updateLabel: root.view.label !== undefined
        ? String(root.view.label) : ""

    // The General sub-panes view (T226). Each sub-view carries its own state.
    readonly property var general: Settings.general
    readonly property bool generalAvailable: Settings.generalAvailable
    readonly property var timeView: root.general.time !== undefined
        ? root.general.time : ({})
    readonly property var localeView: root.general.locale !== undefined
        ? root.general.locale : ({})
    readonly property var loginView: root.general.login !== undefined
        ? root.general.login : ({})
    readonly property var sharingView: root.general.sharing !== undefined
        ? root.general.sharing : ({})
    readonly property var backupView: root.general.backup !== undefined
        ? root.general.backup : ({})
    readonly property var storageView: root.general.storage !== undefined
        ? root.general.storage : ({})
    readonly property var startupView: root.general.startup !== undefined
        ? root.general.startup : ({})
    readonly property var passwordsView: root.general.passwords !== undefined
        ? root.general.passwords : ({})

    // The in-place sub-pane ("" is the General list).
    property string subPaneId: ""
    readonly property string subPaneTitle: {
        switch (root.subPaneId) {
        case "about": return qsTr("About");
        case "software-update": return qsTr("Software Update");
        case "passwords": return qsTr("Passwords & Keys");
        case "login": return qsTr("Login Items & Extensions");
        case "language": return qsTr("Language & Region");
        case "date-time": return qsTr("Date & Time");
        case "sharing": return qsTr("Sharing");
        case "backup": return qsTr("Backups");
        case "startup": return qsTr("Startup Disk");
        case "reset": return qsTr("Transfer or Reset");
        case "storage": return qsTr("Storage");
        default: return "";
        }
    }

    readonly property string aboutSummary: root.identityReady
        ? root.deviceName : qsTr("Unavailable")
    readonly property string updateSummary: {
        if (!root.identityReady)
            return qsTr("Unavailable");
        if (!root.providerAvailable)
            return qsTr("No update provider");
        return root.updateLabel;
    }
    readonly property string timeSummary: root.generalAvailable
        && root.timeView.state === "available"
        ? String(root.timeView.label || qsTr("Date & Time"))
        : qsTr("Unavailable")
    readonly property string dateTimeSummary: root.timeSummary
    readonly property string localeSummary: root.generalAvailable
        && root.localeView.state === "available"
        ? String(root.localeView.label || qsTr("Language & Region"))
        : qsTr("Unavailable")
    readonly property string loginSummary: root.generalAvailable
        && root.loginView.state === "available"
        ? String(root.loginView.label || qsTr("Login Items & Extensions"))
        : qsTr("Unavailable")
    readonly property string sharingSummary: root.generalAvailable
        && root.sharingView.state === "available"
        ? String(root.sharingView.label || qsTr("Sharing"))
        : qsTr("Unavailable")
    readonly property string backupSummary: root.generalAvailable
        && root.backupView.state === "available"
        ? String(root.backupView.label || qsTr("Backups"))
        : qsTr("Unavailable")
    readonly property string storageSummary: root.generalAvailable
        && root.storageView.state === "available"
        ? String(root.storageView.label || qsTr("Storage"))
        : qsTr("Unavailable")
    readonly property string passwordsSummary: root.generalAvailable
        && root.passwordsView.state === "available"
        ? String(root.passwordsView.label || qsTr("Passwords & Keys"))
        : qsTr("Unavailable")
    readonly property string startupSummary: {
        if (!root.generalAvailable || root.startupView.state !== "available")
            return qsTr("Unavailable");
        const disk = root.startupView.disk !== undefined && String(root.startupView.disk).length > 0
            ? String(root.startupView.disk)
            : String(root.startupView.device || "");
        const mode = String(root.startupView.bootMode || "");
        return mode.length > 0 ? disk + " · " + mode : disk;
    }
    readonly property string transferSummary: qsTr("Transfer data or reset settings")

    // T287: the two big-list sub lines the mockup carries. Passwords names the
    // keyring only while the Secret Service answers; Backups names the
    // backends the host adapter can detect. Both stay honest in the absence
    // state instead of repeating a claim the host cannot support.
    readonly property string passwordsSubLine: root.generalAvailable
        && root.passwordsView.state === "available"
        && root.passwordsView.secretServiceAvailable === true
        ? qsTr("Managed by the system keyring")
        : qsTr("Unavailable")
    readonly property string backupSubLine: root.generalAvailable
        && root.backupView.state === "available"
        ? qsTr("Restic / Pika / Timeshift destinations")
        : qsTr("Unavailable")

    // Test surface (used by tst_settings_general.qml).
    property alias aboutRow: aboutRow
    property alias updateRow: updateRow
    property alias passwordsRow: passwordsRow
    property alias storageRow: storageRow
    property alias loginRow: loginRow
    property alias languageRow: languageRow
    property alias dateTimeRow: dateTimeRow
    property alias sharingRow: sharingRow
    property alias backupRow: backupRow
    property alias startupRow: startupRow
    property alias resetRow: resetRow
    property alias subPaneLoader: subPaneLoader
    property alias listColumn: listColumn
    property alias backButton: backButton
    property alias generalIntro: generalIntro
    property alias bigList: bigList

    signal navigateRequested(string paneId)
    signal aboutPageOpened()
    // Emitted when an in-place sub-pane body exists, so a deep link that was
    // waiting for it can finish landing.
    signal subPaneLoaded(string subPaneId)

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function openSubPane(id) {
        root.subPaneId = id;
        if (id === "about")
            root.aboutPageOpened();
    }

    function closeSubPane() {
        root.subPaneId = "";
    }

    // Open the surface that renders a map row before the shell scrolls to it:
    // the in-place sub-pane. Returns whether a surface was opened; the shell
    // treats false as an unlandable row.
    function openRowSurface(rowId) {
        var subPane = SettingsMap.subPaneForRow("general", rowId);
        if (subPane.length > 0) {
            root.openSubPane(subPane);
            return true;
        }
        return false;
    }

    function subPaneComponent(id) {
        switch (id) {
        case "about": return aboutComponent;
        case "software-update": return softwareUpdateComponent;
        case "passwords": return passwordsComponent;
        case "login": return loginComponent;
        case "language": return languageComponent;
        case "date-time": return dateTimeComponent;
        case "sharing": return sharingComponent;
        case "backup": return backupComponent;
        case "startup": return startupComponent;
        case "reset": return resetComponent;
        case "storage": return storageComponent;
        default: return null;
        }
    }

    // Opening the pane asks the host for a re-read (a no-op when absent).
    // T268: it must not push the update policy. Pushing the schema defaults
    // here turned automatic updates off merely by opening General; the policy
    // is forwarded only when the user changes a toggle.
    Component.onCompleted: {
        Settings.refreshUpdates();
        Settings.refreshGeneral();
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The General list ──────────────────────────────────────────────
        Column {
            id: listColumn
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId === ""

            SettingsIntro {
                id: generalIntro
                objectName: "generalIntro"
                width: parent.width
                text: qsTr("Manage your overall setup and preferences for this computer, such as updates, language, sharing, and startup.")
            }

            SettingsBigList {
                id: bigList
                objectName: "generalBigList"
                width: parent.width

                SettingsBigRow {
                    id: aboutRow
                    objectName: "generalAboutRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("about")
                    label: qsTr("About")
                    onDisclosureClicked: root.openSubPane("about")
                }

                SettingsBigRow {
                    id: updateRow
                    objectName: "generalUpdateRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("update")
                    label: qsTr("Software Update")
                    onDisclosureClicked: root.openSubPane("software-update")
                }

                SettingsBigRow {
                    id: storageRow
                    objectName: "generalStorageRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("storage")
                    label: qsTr("Storage")
                    onDisclosureClicked: root.openSubPane("storage")
                }

                SettingsBigRow {
                    id: passwordsRow
                    objectName: "generalPasswordsRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("password")
                    label: qsTr("Passwords & Keys")
                    secondaryText: root.passwordsSubLine
                    onDisclosureClicked: root.openSubPane("passwords")
                }

                SettingsBigRow {
                    id: dateTimeRow
                    objectName: "generalDateTimeRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("clock")
                    label: qsTr("Date & Time")
                    onDisclosureClicked: root.openSubPane("date-time")
                }

                SettingsBigRow {
                    id: languageRow
                    objectName: "generalLanguageRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("language")
                    label: qsTr("Language & Region")
                    onDisclosureClicked: root.openSubPane("language")
                }

                SettingsBigRow {
                    id: loginRow
                    objectName: "generalLoginRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("login")
                    label: qsTr("Login Items & Extensions")
                    onDisclosureClicked: root.openSubPane("login")
                }

                SettingsBigRow {
                    id: sharingRow
                    objectName: "generalSharingRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("sharing")
                    label: qsTr("Sharing")
                    onDisclosureClicked: root.openSubPane("sharing")
                }

                SettingsBigRow {
                    id: startupRow
                    objectName: "generalStartupRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("startup")
                    label: qsTr("Startup Disk")
                    onDisclosureClicked: root.openSubPane("startup")
                }

                SettingsBigRow {
                    id: backupRow
                    objectName: "generalBackupRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("backup")
                    label: qsTr("Backups")
                    secondaryText: root.backupSubLine
                    onDisclosureClicked: root.openSubPane("backup")
                }

                // device management is deliberately not rendered: this build
                // has no Linux MDM/profile owner. `SettingsMap` records the
                // omission and its reason.

                SettingsBigRow {
                    id: resetRow
                    objectName: "generalResetRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("transfer")
                    label: qsTr("Transfer or Reset")
                    onDisclosureClicked: root.openSubPane("reset")
                }
            }
        }

        // ── One in-place sub-pane ─────────────────────────────────────────
        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId !== ""

            Button {
                id: backButton
                objectName: "generalSubPaneBack"
                text: qsTr("‹ General")
                onClicked: root.closeSubPane()
            }

            Loader {
                id: subPaneLoader
                width: parent.width
                sourceComponent: root.subPaneComponent(root.subPaneId)
                onLoaded: root.subPaneLoaded(root.subPaneId)
            }
        }
    }

    Component {
        id: aboutComponent
        AboutPage { onNavigateRequested: (paneId) => root.navigateRequested(paneId) }
    }
    Component { id: softwareUpdateComponent; SoftwareUpdatePane { } }
    Component { id: passwordsComponent; PasswordsPane { } }
    // T263: Storage is General -> Storage. The existing Storage pane body is
    // reused in place; the top-level `storage` id and deep link remain aliases.
    Component { id: storageComponent; StoragePane { } }

    Component { id: loginComponent; LoginItemsPane { } }
    Component { id: languageComponent; LanguageRegionPane { } }
    Component { id: dateTimeComponent; DateTimePane { } }
    Component { id: sharingComponent; SharingPane { } }
    Component { id: backupComponent; BackupPane { } }
    Component { id: startupComponent; StartupDiskPane { } }
    Component { id: resetComponent; ResetPane { } }
}
