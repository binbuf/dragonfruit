// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Biometrics & Password pane (T228).
//
// Three host owners meet here, and each degrades on its own:
//
//  * AccountsService owns the password write. The dialog collects a new
//    password and hands it straight to the host; the app never stores it.
//  * fprintd lists, enrolls, and deletes fingerprints. No reader is a normal
//    note, not an error.
//  * PAM is read from `/etc/pam.d`. The unlock/authentication rows are
//    read-only because changing PAM is system policy; they are hidden when
//    `pam_fprintd.so` is not configured, so no dead toggle is drawn.
Item {
    id: root

    readonly property var view: Settings.biometrics
    readonly property bool available: Settings.biometricsAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property var device: root.view.device !== undefined ? root.view.device : null
    readonly property var fingers: root.device !== null && root.device.enrolled !== undefined
        ? root.device.enrolled : []
    readonly property var pam: root.view.pam !== undefined ? root.view.pam : ({})
    readonly property var password: root.view.password !== undefined
        ? root.view.password : ({})
    readonly property var enrolling: root.view.enrolling !== undefined
        ? root.view.enrolling : null
    readonly property bool fprintdAvailable: root.view.fprintdAvailable === true
    readonly property bool pamAvailable: root.pam.available === true
    readonly property bool passwordAvailable: root.password.available === true

    property string selectedFinger: "right-index-finger"
    property string passwordError: ""

    // Test surface.
    property alias passwordButton: passwordButton
    property alias changePasswordDialog: changePasswordDialog
    property alias newPasswordField: newPasswordField
    property alias confirmPasswordField: confirmPasswordField
    property alias passwordErrorNote: passwordErrorNote
    property alias addFingerButton: addFingerButton
    property alias addFingerDialog: addFingerDialog
    property alias fingerSelect: fingerSelect
    property alias fingerRepeater: fingerRepeater
    property alias deleteFingerDialog: deleteFingerDialog
    property alias cancelEnrollButton: cancelEnrollButton
    property alias enrollingNote: enrollingNote
    property alias pamUnlockToggle: pamUnlockToggle
    property alias pamAuthToggle: pamAuthToggle
    property alias absenceNote: absenceNote
    property alias pamAbsenceNote: pamAbsenceNote

    // The shell reparents dialogs to its root so the detail ScrollView's clip
    // cannot cut them off (T227 note).
    property Item popupParent: null

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    readonly property var fingerModel: [
        { label: qsTr("Right index finger"), value: "right-index-finger" },
        { label: qsTr("Right middle finger"), value: "right-middle-finger" },
        { label: qsTr("Right ring finger"), value: "right-ring-finger" },
        { label: qsTr("Right little finger"), value: "right-little-finger" },
        { label: qsTr("Right thumb"), value: "right-thumb" },
        { label: qsTr("Left index finger"), value: "left-index-finger" },
        { label: qsTr("Left middle finger"), value: "left-middle-finger" },
        { label: qsTr("Left ring finger"), value: "left-ring-finger" },
        { label: qsTr("Left little finger"), value: "left-little-finger" },
        { label: qsTr("Left thumb"), value: "left-thumb" }
    ]

    Component.onCompleted: Settings.refreshBiometrics()

    // The live enrollment is driven by fprintd signals; the pane re-reads the
    // enrolled list while the host reports an enrollment in flight. A mock
    // completes synchronously, so this is a no-op there.
    Timer {
        interval: 800
        repeat: true
        running: root.enrolling !== null
        onTriggered: Settings.refreshBiometrics()
    }

    function submitPassword() {
        root.passwordError = "";
        if (newPasswordField.text.length === 0) {
            root.passwordError = qsTr("The password must not be empty.");
            return;
        }
        if (newPasswordField.text !== confirmPasswordField.text) {
            root.passwordError = qsTr("The passwords do not match.");
            return;
        }
        Settings.changePassword(newPasswordField.text);
        newPasswordField.text = "";
        confirmPasswordField.text = "";
        changePasswordDialog.accept();
    }

    function startEnrollment() {
        Settings.startFingerprintEnrollment(root.selectedFinger);
        addFingerDialog.accept();
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Password ──────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Password")

            SettingsRow {
                width: parent.width
                label: qsTr("Change password\u2026")
                description: root.passwordAvailable
                    ? String(root.password.note || "")
                    : qsTr("The account service is not running, so the password cannot be changed.")
                showSeparator: false
                controlData: Button {
                    id: passwordButton
                    objectName: "biometricsPasswordButton"
                    text: qsTr("Change\u2026")
                    enabled: root.passwordAvailable
                    onClicked: {
                        root.passwordError = "";
                        newPasswordField.text = "";
                        confirmPasswordField.text = "";
                        changePasswordDialog.show();
                    }
                }
            }
        }

        // ── Fingerprints ──────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Fingerprints")

            SettingsRow {
                width: parent.width
                label: root.device !== null && root.device.name !== undefined
                    ? String(root.device.name) : qsTr("Fingerprint reader")
                description: root.fprintdAvailable
                    ? (root.device !== null
                       ? qsTr("%1 enrolled fingerprint(s).").arg(root.fingers.length)
                       : qsTr("No fingerprint reader is connected."))
                    : qsTr("The fingerprint service is not running.")
                showSeparator: root.fingers.length > 0
                controlData: Button {
                    id: addFingerButton
                    objectName: "biometricsAddFingerButton"
                    text: qsTr("Add Fingerprint\u2026")
                    enabled: root.fprintdAvailable && root.device !== null
                        && root.enrolling === null
                    onClicked: {
                        root.selectedFinger = "right-index-finger";
                        addFingerDialog.show();
                    }
                }
            }

            Repeater {
                id: fingerRepeater
                model: root.fingers
                delegate: SettingsRow {
                    required property var modelData
                    required property int index
                    width: parent.width
                    label: modelData.label !== undefined
                        ? String(modelData.label) : String(modelData.id)
                    showSeparator: index < root.fingers.length - 1
                    controlData: Button {
                        objectName: "biometricsDeleteFingerButton"
                        text: qsTr("Delete")
                        variant: "ghost"
                        onClicked: {
                            root.selectedFinger = String(modelData.id);
                            deleteFingerDialog.show();
                        }
                    }
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.enrolling !== null
                label: root.enrolling !== null
                    ? qsTr("Enrolling %1").arg(String(root.enrolling.label))
                    : ""
                description: root.enrolling !== null
                    ? String(root.enrolling.status) : ""
                showSeparator: false
                controlData: Button {
                    id: cancelEnrollButton
                    objectName: "biometricsCancelEnrollButton"
                    text: qsTr("Cancel")
                    onClicked: Settings.cancelFingerprintEnrollment()
                }
            }

            Text {
                id: enrollingNote
                objectName: "biometricsEnrollingNote"
                visible: root.enrolling !== null
                width: parent.width
                text: root.enrolling !== null
                    ? qsTr("Touch the fingerprint sensor to finish enrolling %1.")
                          .arg(String(root.enrolling.label))
                    : ""
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── PAM fingerprint authentication ────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: root.pamAvailable
            title: qsTr("Fingerprint authentication")

            SettingsRow {
                width: parent.width
                label: qsTr("Use fingerprint to unlock")
                description: qsTr("Configured by PAM; change /etc/pam.d to alter.")
                controlData: Toggle {
                    id: pamUnlockToggle
                    objectName: "biometricsPamUnlockToggle"
                    accessibleName: qsTr("Use fingerprint to unlock")
                    checked: root.pam.unlock === true
                    enabled: false
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Use fingerprint for authentication")
                description: qsTr("Configured by PAM; change /etc/pam.d to alter.")
                showSeparator: false
                controlData: Toggle {
                    id: pamAuthToggle
                    objectName: "biometricsPamAuthToggle"
                    accessibleName: qsTr("Use fingerprint for authentication")
                    checked: root.pam.auth === true
                    enabled: false
                }
            }
        }

        Text {
            id: pamAbsenceNote
            objectName: "biometricsPamAbsenceNote"
            width: parent.width
            visible: root.ready && !root.pamAvailable
            text: qsTr("Fingerprint authentication is not configured in PAM, "
                       + "so the unlock and authentication rows are hidden.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        Text {
            id: absenceNote
            objectName: "biometricsAbsenceNote"
            width: parent.width
            visible: !root.ready
            text: qsTr("Biometrics and password are unavailable because their "
                       + "host services are not running.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }
    }

    // ── Change password ──────────────────────────────────────────────────
    Dialog {
        id: changePasswordDialog
        objectName: "biometricsChangePasswordDialog"
        title: qsTr("Change Password")
        parent: root.popupParent ? root.popupParent : root
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("The new password is handed to the account service; "
                           + "the app does not store it.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Rectangle {
                width: parent.width
                height: Theme.controls.button.height
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: newPasswordField
                    objectName: "biometricsNewPassword"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    echoMode: TextInput.Password
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: root.submitPassword()
                }
            }

            Rectangle {
                width: parent.width
                height: Theme.controls.button.height
                radius: Theme.controls.button.radius
                color: Theme.color.controlFill
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border

                TextInput {
                    id: confirmPasswordField
                    objectName: "biometricsConfirmPassword"
                    anchors.fill: parent
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    verticalAlignment: TextInput.AlignVCenter
                    echoMode: TextInput.Password
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    selectByMouse: true
                    onAccepted: root.submitPassword()
                }
            }

            Text {
                id: passwordErrorNote
                objectName: "biometricsPasswordError"
                width: parent.width
                visible: root.passwordError.length > 0
                text: root.passwordError
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Cancel")
                onClicked: changePasswordDialog.reject()
            }
            Button {
                objectName: "biometricsChangePasswordConfirm"
                text: qsTr("Change Password")
                variant: "primary"
                onClicked: root.submitPassword()
            }
        }
    }

    // ── Add fingerprint ──────────────────────────────────────────────────
    Dialog {
        id: addFingerDialog
        objectName: "biometricsAddFingerDialog"
        title: qsTr("Add Fingerprint")
        parent: root.popupParent ? root.popupParent : root
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("Choose the finger to enroll, then touch the sensor.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Select {
                id: fingerSelect
                objectName: "biometricsFingerSelect"
                width: parent.width
                accessibleName: qsTr("Finger")
                model: root.fingerModel
                onSelected: (value) => root.selectedFinger = String(value)
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Cancel")
                onClicked: addFingerDialog.reject()
            }
            Button {
                objectName: "biometricsStartEnroll"
                text: qsTr("Start")
                variant: "primary"
                onClicked: root.startEnrollment()
            }
        }
    }

    // ── Delete fingerprint confirmation ──────────────────────────────────
    Dialog {
        id: deleteFingerDialog
        objectName: "biometricsDeleteFingerDialog"
        title: qsTr("Delete Fingerprint")
        parent: root.popupParent ? root.popupParent : root
        dismissible: true

        contentData: Text {
            width: parent.width
            text: qsTr("Delete the enrolled fingerprint? You can add it again later.")
            color: Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Cancel")
                onClicked: deleteFingerDialog.reject()
            }
            Button {
                objectName: "biometricsDeleteFingerConfirm"
                text: qsTr("Delete")
                variant: "primary"
                onClicked: {
                    Settings.deleteFingerprint(root.selectedFinger);
                    deleteFingerDialog.accept();
                }
            }
        }
    }
}
