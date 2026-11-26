// SPDX-License-Identifier: GPL-3.0-only
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Biometrics & Password pane (T228; T279 completion).
//
// Three host owners meet here, and each degrades on its own:
//
//  * AccountsService owns the password write. The dialog collects a new
//    password, hands it straight to the host, and stays open until the host's
//    own report arrives; the app never stores it.
//  * fprintd lists, enrolls, and deletes fingerprints. The enrollment wizard
//    reports fprintd's own `EnrollStatus` stage text — never an invented
//    percentage — and a denied write or an unsupported reader is shown at the
//    row. No reader is a normal note, not an error.
//  * PAM is read from `/etc/pam.d`. The unlock/authentication rows are
//    read-only because changing PAM is system policy; they are hidden when
//    `pam_fprintd.so` is not configured, so no dead toggle is drawn.
//
// A fast-user-switching row is deliberately absent: no display manager exposes
// a fingerprint-triggered switch, so a control for it would have no consumer.
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
    readonly property string enrollmentError: root.view.enrollmentError !== undefined
        && root.view.enrollmentError !== null
        ? String(root.view.enrollmentError) : ""
    readonly property bool fprintdAvailable: root.view.fprintdAvailable === true
    readonly property bool pamAvailable: root.pam.available === true
    readonly property bool passwordAvailable: root.password.available === true

    // T279: the host's write report. `idle` until the first write; a pending
    // password keeps its dialog open, a denial leaves the view unchanged.
    readonly property string writeOperation: Settings.biometricsWriteOperation
    readonly property string writeState: Settings.biometricsWriteState
    readonly property string writeError: Settings.biometricsWriteError
    readonly property bool passwordBusy: root.writeOperation === "ChangePassword"
        && root.writeState === "pending"
    readonly property bool passwordFailed: root.writeOperation === "ChangePassword"
        && (root.writeState === "denied" || root.writeState === "failed"
            || root.writeState === "absent")
    readonly property string fingerprintError: {
        if (root.enrollmentError.length > 0)
            return qsTr("The reader reported: %1").arg(root.enrollmentError);
        if (root.writeState === "denied" || root.writeState === "failed"
                || root.writeState === "absent")
            return root.writeError;
        return "";
    }

    property string selectedFinger: "right-index-finger"
    property string passwordError: ""

    // Test surface.
    property alias passwordRow: passwordRow
    property alias passwordButton: passwordButton
    property alias changePasswordDialog: changePasswordDialog
    property alias newPasswordField: newPasswordField
    property alias confirmPasswordField: confirmPasswordField
    property alias passwordErrorNote: passwordErrorNote
    property alias passwordPendingNote: passwordPendingNote
    property alias addFingerButton: addFingerButton
    property alias addFingerDialog: addFingerDialog
    property alias fingerSelect: fingerSelect
    property alias fingerRepeater: fingerRepeater
    property alias fingerprintRow: fingerprintRow
    property alias deleteFingerDialog: deleteFingerDialog
    property alias cancelEnrollButton: cancelEnrollButton
    property alias enrollingNote: enrollingNote
    property alias enrollStageText: enrollStageText
    property alias enrollRawStageText: enrollRawStageText
    property alias enrollDialogError: enrollDialogError
    property alias pamUnlockToggle: pamUnlockToggle
    property alias pamAuthToggle: pamAuthToggle
    property alias absenceNote: absenceNote
    property alias pamAbsenceNote: pamAbsenceNote

    // The shell reparents dialogs to its root so the detail ScrollView's clip
    // cannot cut them off (T227 note).

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

    function fingerEnrolled(finger) {
        for (var i = 0; i < root.fingers.length; ++i)
            if (String(root.fingers[i].id) === String(finger))
                return true;
        return false;
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
        // The dialog waits: the host report closes it on success and leaves it
        // open with the denial on failure.
        Settings.changePassword(newPasswordField.text);
    }

    function startEnrollment() {
        root.selectedFinger = fingerSelect.currentValue !== undefined
            ? String(fingerSelect.currentValue) : root.selectedFinger;
        addFingerDialog.started = true;
        addFingerDialog.errorText = "";
        Settings.startFingerprintEnrollment(root.selectedFinger);
    }

    // The wizard closes when the host reports the finger enrolled. A failed
    // stage stays in the dialog (and at the row) until the user cancels.
    onEnrollingChanged: {
        if (!addFingerDialog.started || root.enrolling !== null)
            return;
        if (root.fingerEnrolled(root.selectedFinger))
            addFingerDialog.finish();
    }

    Connections {
        target: Settings

        function onBiometricsWriteChanged() {
            if (root.writeOperation === "ChangePassword") {
                if (root.writeState === "applied") {
                    newPasswordField.text = "";
                    confirmPasswordField.text = "";
                    root.passwordError = "";
                    changePasswordDialog.accept();
                } else if (root.passwordFailed) {
                    root.passwordError = root.writeError.length > 0
                        ? root.writeError
                        : qsTr("The account service refused the password change.");
                }
            }
            if (addFingerDialog.started && root.writeOperation === "StartFingerprintEnrollment"
                    && (root.writeState === "denied" || root.writeState === "failed"
                        || root.writeState === "absent")) {
                addFingerDialog.errorText = root.writeError;
            }
        }
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
                id: passwordRow
                objectName: "biometricsPasswordRow"
                width: parent.width
                leadingIcon: "lock"
                label: qsTr("Change password\u2026")
                description: root.passwordAvailable
                    ? String(root.password.note || "")
                    : qsTr("The account service is not running, so the password cannot be changed.")
                errorText: root.passwordFailed ? root.passwordError : ""
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
                id: fingerprintRow
                objectName: "biometricsFingerprintRow"
                width: parent.width
                leadingIcon: "fingerprint"
                label: root.device !== null && root.device.name !== undefined
                    ? String(root.device.name) : qsTr("Fingerprint reader")
                description: root.fprintdAvailable
                    ? (root.device !== null
                       ? qsTr("%1 enrolled fingerprint(s).").arg(root.fingers.length)
                       : qsTr("No fingerprint reader is connected."))
                    : qsTr("The fingerprint service is not running.")
                errorText: root.fingerprintError
                showSeparator: root.fingers.length > 0
                controlData: Button {
                    id: addFingerButton
                    objectName: "biometricsAddFingerButton"
                    text: qsTr("Add Fingerprint\u2026")
                    enabled: root.fprintdAvailable && root.device !== null
                        && root.enrolling === null
                    onClicked: {
                        root.selectedFinger = "right-index-finger";
                        addFingerDialog.started = false;
                        addFingerDialog.errorText = "";
                        fingerSelect.currentIndex = 0;
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
        dismissible: true

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                text: qsTr("The new password is handed to the account service; "
                           + "the app does not store it. The dialog waits for the "
                           + "host's answer.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                text: qsTr("New Password")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
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

            Text {
                width: parent.width
                text: qsTr("Confirm Password")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
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
                id: passwordPendingNote
                objectName: "biometricsPasswordPending"
                width: parent.width
                visible: root.passwordBusy
                text: qsTr("Waiting for the account service\u2026")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
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
                text: root.passwordBusy ? qsTr("Changing\u2026") : qsTr("Change Password")
                variant: "primary"
                enabled: !root.passwordBusy
                onClicked: root.submitPassword()
            }
        }
    }

    // ── Add fingerprint (the enrollment wizard) ──────────────────────────
    Dialog {
        id: addFingerDialog
        objectName: "biometricsAddFingerDialog"
        title: qsTr("Add Fingerprint")
        dismissible: true

        // Step 1 chooses the finger; step 2 reports fprintd's own stages.
        property bool started: false
        property string errorText: ""

        function finish() {
            addFingerDialog.accept();
        }

        contentData: Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Text {
                width: parent.width
                visible: !addFingerDialog.started
                text: qsTr("Choose the finger to enroll, then touch the sensor.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Select {
                id: fingerSelect
                objectName: "biometricsFingerSelect"
                width: parent.width
                visible: !addFingerDialog.started
                accessibleName: qsTr("Finger")
                model: root.fingerModel
                onSelected: (value) => root.selectedFinger = String(value)
            }

            Text {
                id: enrollStageText
                objectName: "biometricsEnrollStage"
                width: parent.width
                visible: addFingerDialog.started
                text: root.enrolling !== null
                    ? String(root.enrolling.status)
                    : qsTr("Waiting for the fingerprint sensor\u2026")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                wrapMode: Text.WordWrap
            }

            Text {
                id: enrollRawStageText
                objectName: "biometricsEnrollRawStage"
                width: parent.width
                visible: addFingerDialog.started && root.enrolling !== null
                         && String(root.enrolling.stage || "").length > 0
                text: root.enrolling !== null
                    ? qsTr("fprintd stage: %1").arg(String(root.enrolling.stage))
                    : ""
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                id: enrollDialogError
                objectName: "biometricsEnrollError"
                width: parent.width
                visible: addFingerDialog.errorText.length > 0
                text: addFingerDialog.errorText
                color: Theme.color.danger
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Text {
                width: parent.width
                visible: addFingerDialog.started
                text: qsTr("Lift and place the same finger until the reader "
                           + "reports the fingerprint enrolled.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap
            Button {
                text: qsTr("Cancel")
                onClicked: {
                    if (addFingerDialog.started && root.enrolling !== null)
                        Settings.cancelFingerprintEnrollment();
                    addFingerDialog.reject();
                }
            }
            Button {
                objectName: "biometricsStartEnroll"
                visible: !addFingerDialog.started
                text: qsTr("Start")
                variant: "primary"
                onClicked: root.startEnrollment()
            }
        }

        onClosed: addFingerDialog.started = false
    }

    // ── Delete fingerprint confirmation ──────────────────────────────────
    Dialog {
        id: deleteFingerDialog
        objectName: "biometricsDeleteFingerDialog"
        title: qsTr("Delete Fingerprint")
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
