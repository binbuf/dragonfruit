// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Accessibility (T291a).
//
// The mockup's Accessibility pane is a big list of sections that open an
// in-place sub-pane. This file owns the shared list and loader; each child
// task adds exactly one big-list row and one sub-pane body:
//
//   * T291a — Vision
//   * T291b — Physical and Motor
//   * T291c — Hearing
//   * T291d — Speech
//   * T291e — General (the Accessibility Shortcut) and the close-out audit
//
// A section whose rows all have a real Linux owner is shipped; a section with
// no owner is omitted from the list and recorded in `SettingsMap` with a
// reason. The list never grows a disabled placeholder for a later child.
//
// The intro is the shell's `SettingsIntro` from the pane catalog description
// (doc 25 §3.3); the body starts at the big list. The live AT-SPI status,
// settingsd keys, and delegates live in the sub-pane bodies, never here.
Item {
    id: root

    // The in-place sub-pane ("" is the Accessibility list).
    property string subPaneId: ""
    readonly property string subPaneTitle: {
        switch (root.subPaneId) {
        case "vision": return qsTr("Vision");
        case "motor": return qsTr("Physical and Motor");
        case "hearing": return qsTr("Hearing");
        default: return "";
        }
    }

    // Test/capture surface.
    property alias listColumn: listColumn
    property alias bigList: bigList
    property alias visionRow: visionRow
    property alias motorRow: motorRow
    property alias hearingRow: hearingRow
    property alias subPaneLoader: subPaneLoader
    property alias backButton: backButton

    signal navigateRequested(string paneId)
    // Emitted when an in-place sub-pane body exists, so a deep link that was
    // waiting for it can finish landing.
    signal subPaneLoaded(string subPaneId)

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function openSubPane(id) {
        root.subPaneId = id;
    }

    function closeSubPane() {
        root.subPaneId = "";
    }

    // Open the surface that renders a map row before the shell scrolls to it.
    // The shell treats false as an unlandable row.
    function openRowSurface(rowId) {
        var subPane = SettingsMap.subPaneForRow("accessibility", rowId);
        if (subPane.length > 0) {
            root.openSubPane(subPane);
            return true;
        }
        return false;
    }

    function subPaneComponent(id) {
        switch (id) {
        case "vision": return visionComponent;
        case "motor": return motorComponent;
        case "hearing": return hearingComponent;
        default: return null;
        }
    }

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshAccessibility()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The Accessibility list ──────────────────────────────────────
        Column {
            id: listColumn
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId === ""

            SettingsBigList {
                id: bigList
                objectName: "accessibilityBigList"
                width: parent.width

                SettingsBigRow {
                    id: visionRow
                    objectName: "accessibilityVisionRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("vision")
                    label: qsTr("Vision")
                    onDisclosureClicked: root.openSubPane("vision")
                }

                SettingsBigRow {
                    id: motorRow
                    objectName: "accessibilityMotorRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("motor")
                    label: qsTr("Physical and Motor")
                    onDisclosureClicked: root.openSubPane("motor")
                }

                SettingsBigRow {
                    id: hearingRow
                    objectName: "accessibilityHearingRow"
                    width: parent.width
                    category: SettingsPanes.rowStyle("hearing")
                    label: qsTr("Hearing")
                    onDisclosureClicked: root.openSubPane("hearing")
                }
            }
        }

        // ── One in-place sub-pane ───────────────────────────────────────
        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.lg
            visible: root.subPaneId !== ""

            Button {
                id: backButton
                objectName: "accessibilitySubPaneBack"
                text: qsTr("‹ Accessibility")
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

    Component { id: visionComponent; AccessibilityVisionPane { } }
    Component { id: motorComponent; AccessibilityMotorPane { } }
    Component { id: hearingComponent; AccessibilityHearingPane { } }
}
