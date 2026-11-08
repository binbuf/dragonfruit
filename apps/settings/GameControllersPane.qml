// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Game Controllers pane (T229).
//
// Controllers are optional: the pane lists what the kernel inventory reports
// through the shared input adapter, and every row is a real action. `Test`
// opens one diagnostic event read (the bridge host's `TestController`), and
// `Mapping` shows the SDL mapping line when the session has one. A remapper
// and a mapping editor are deliberately out of scope (T229 out-of-scope), so
// the mapping sheet is read-only and says where the mapping came from.
//
// Absence is layered: the bridge host absent, no controller attached, and a
// controller whose test node cannot be opened are three separate notes. None
// is an error.
Item {
    id: root

    readonly property var view: Settings.input
    readonly property bool available: Settings.inputAvailable
    readonly property bool present: view.state === "available"
    readonly property var controllers: view.controllers !== undefined
        ? view.controllers : []
    readonly property var test: Settings.controllerTest
    readonly property string testOutcome: test.outcome !== undefined
        ? String(test.outcome) : ""

    property string selectedName: ""
    property string selectedPath: ""
    property string selectedMapping: ""
    property bool mappingOpen: false
    property Item popupParent: null

    // Test surface (used by tst_settings_controllers.qml).
    property alias controllerRepeater: controllerRepeater
    property alias absenceNote: absenceNote
    property alias emptyNote: emptyNote
    property alias testDialog: testDialog
    property alias mappingDialog: mappingDialog
    property alias mappingText: mappingText

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    Component.onCompleted: Settings.refreshInput()

    function openTest(controller) {
        root.selectedName = controller.name;
        root.selectedPath = controller.path;
        Settings.testController(controller.path);
        root.testDialog.show();
    }

    function openMapping(controller) {
        root.selectedName = controller.name;
        root.selectedMapping = controller.mapping !== undefined
            ? String(controller.mapping) : "";
        root.mappingOpen = true;
        root.mappingDialog.show();
    }

    function outcomeLabel(outcome) {
        switch (outcome) {
        case "event": return qsTr("Event received");
        case "timeout": return qsTr("No event");
        case "denied": return qsTr("Permission denied");
        case "absent": return qsTr("Device node unavailable");
        case "failed": return qsTr("Test failed");
        default: return qsTr("Waiting…");
        }
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            visible: root.available && root.present
            title: qsTr("Controllers")
            reserveBottomMargin: false

            Text {
                id: emptyNote
                objectName: "controllersEmpty"
                width: parent.width
                visible: root.controllers.length === 0
                text: qsTr("No game controllers are connected.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                leftPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Repeater {
                id: controllerRepeater
                model: root.controllers
                delegate: Item {
                    id: controllerRow
                    required property var modelData
                    required property int index
                    width: parent ? parent.width : 0
                    height: Math.max(Theme.controls.settingsRow.height, 52)
                    Accessible.role: Accessible.Grouping
                    Accessible.name: controllerRow.modelData.name

                    Icon {
                        id: controllerIcon
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        name: "input"
                        size: 22
                        color: Theme.color.textSecondary
                    }

                    Column {
                        anchors.left: controllerIcon.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.right: controllerActions.left
                        anchors.rightMargin: Theme.controls.settingsRow.controlGap
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            objectName: "controllerName"
                            width: parent.width
                            text: controllerRow.modelData.name
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.controls.button.fontSize
                            elide: Text.ElideRight
                        }

                        Text {
                            objectName: "controllerDetail"
                            width: parent.width
                            text: {
                                var identity = controllerRow.modelData.identity !== undefined
                                    ? String(controllerRow.modelData.identity) : "";
                                var driver = controllerRow.modelData.driver !== undefined
                                    ? String(controllerRow.modelData.driver) : "";
                                if (identity.length > 0 && driver.length > 0)
                                    return identity + " \u00b7 " + driver;
                                return identity.length > 0 ? identity : driver;
                            }
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            elide: Text.ElideRight
                        }
                    }

                    Row {
                        id: controllerActions
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.controls.settingsRow.paddingH
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.primitive.spacing.xs

                        Button {
                            objectName: "controllerMapping"
                            text: qsTr("Mapping")
                            variant: "ghost"
                            onClicked: root.openMapping(controllerRow.modelData)
                        }

                        Button {
                            objectName: "controllerTest"
                            text: qsTr("Test")
                            onClicked: root.openTest(controllerRow.modelData)
                        }
                    }

                    Rectangle {
                        visible: controllerRow.index < root.controllers.length - 1
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
        }

        SettingsGroup {
            width: parent.width
            visible: !root.available || !root.present

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "controllersAbsenceNote"
                kind: "disabled-with-reason"
                reason: root.available
                    ? qsTr("No game controller inventory is available.")
                    : qsTr("The system status service is not running, so game controllers are unavailable.")
            }
        }
    }

    Dialog {
        id: testDialog
        objectName: "controllerTestDialog"
        title: root.selectedName.length > 0
            ? qsTr("Test %1").arg(root.selectedName) : qsTr("Test Controller")
        dismissible: true
        parent: root.popupParent ? root.popupParent : root

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                objectName: "controllerTestOutcome"
                width: parent.width
                text: root.outcomeLabel(root.testOutcome)
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                font.weight: Theme.primitive.font.weightMedium
            }

            Text {
                objectName: "controllerTestDetail"
                width: parent.width
                text: root.test.detail !== undefined ? String(root.test.detail) : ""
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: root.test.bytes !== undefined && String(root.test.bytes).length > 0
                text: qsTr("Bytes: %1").arg(String(root.test.bytes))
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WrapAnywhere
            }

            Text {
                width: parent.width
                text: qsTr("Press a button or move a stick while the read is in flight.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }

    Dialog {
        id: mappingDialog
        objectName: "controllerMappingDialog"
        title: root.selectedName.length > 0
            ? qsTr("Mapping — %1").arg(root.selectedName) : qsTr("Mapping")
        dismissible: true
        parent: root.popupParent ? root.popupParent : root

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                id: mappingText
                objectName: "controllerMappingText"
                width: parent.width
                text: root.selectedMapping.length > 0
                    ? root.selectedMapping
                    : qsTr("No SDL mapping is available for this controller. It still works through the kernel's evdev interface.")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WrapAnywhere
            }

            Text {
                width: parent.width
                text: qsTr("Mapping edits are delegated to the session's SDL configuration; this build does not rewrite it.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }
    }
}
