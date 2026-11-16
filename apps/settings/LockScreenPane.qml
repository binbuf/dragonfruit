// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Lock Screen pane (T-15.8b; completed by T278).
//
// The pane and the Control Center Lock Screen tile are one functional unit.
// Lock policy has no external daemon: the session idle/lock engine
// (`services/session/src/idle.rs`, ADR 0070) owns the stage timing and the
// compositor owns the one fail-secure lock state (ADR 0067); the shell mirrors
// both over the private `df_toplevel_manager` bridge. Like the Mission Control
// pane (T-15.5b), this pane is the *configuration* half and writes only
// settingsd keys, so every control applies live and persists:
//
//   * `idle.blank` is the one display-off stage (ADR 0279 explains why the
//     Linux adaptation keeps one stage instead of a battery/AC pair);
//   * `lock.requirePasswordAfter` is the delay *after* display-off, and the
//     pane derives `idle.lock` from it so the lock never fires from raw
//     inactivity;
//   * the four `lock.*` display rows (`dragonfruit-lock-adapter`'s
//     `LockDisplayOption::id()` suffixes) feed the shell's lock scene.
//
// Deviations from the macOS capture reference (ADR 0122/0279):
//   * the battery/adapter pair collapses to one `Turn display off when
//     inactive` row with an explicit one-stage reason;
//   * `Require password after` means after display-off/lock, not inactivity;
//     `Immediately` is the zero-second option and `Never` disables the stage;
//   * `Login window shows` is present but disabled with the reason: the
//     display manager does not expose a style setting on the host;
//   * `Accessibility Options...` opens the Accessibility pane instead of
//     duplicating its controls;
//   * the Apple Account/sidebar row and the trailing `?` help are omitted
//     project-wide.
Item {
    id: root

    // ── The session idle timing keys (revision 5, ADR 0070) ──────────────
    // 0 disables a stage; the engine reads whole seconds.
    readonly property int displayOffSeconds:
        Settings.values["idle.blank"] !== undefined
            ? Number(Settings.values["idle.blank"]) : 300
    readonly property int requirePasswordSeconds:
        Settings.values["idle.lock"] !== undefined
            ? Number(Settings.values["idle.lock"]) : 600
    // T278: the durable "after display-off" preference. -1 never, 0
    // immediately, otherwise seconds after the display turns off.
    readonly property int requireAfter:
        Settings.values["lock.requirePasswordAfter"] !== undefined
            ? Number(Settings.values["lock.requirePasswordAfter"]) : 300

    // ── The lock-screen display keys (revision 15, ADR 0133) ─────────────
    readonly property bool showUserNameAndPhoto:
        Settings.values["lock.showUserNameAndPhoto"] !== false
    readonly property bool showPasswordHints:
        Settings.values["lock.showPasswordHints"] === true
    readonly property bool showMessageWhenLocked:
        Settings.values["lock.showMessageWhenLocked"] === true
    readonly property string lockMessage:
        Settings.values["lock.message"] !== undefined
            ? String(Settings.values["lock.message"]) : ""
    readonly property bool showPowerButtons:
        Settings.values["lock.showPowerButtons"] !== false
    readonly property string passwordHint:
        Settings.values["lock.passwordHint"] !== undefined
            ? String(Settings.values["lock.passwordHint"]) : ""
    readonly property string loginWindowStyle:
        Settings.values["login.windowStyle"] !== undefined
            ? String(Settings.values["login.windowStyle"]) : "list"

    // The display-off popup choices, in ascending order with `Never` last.
    readonly property var displayOffOptions: [
        { value: 60, label: qsTr("For 1 minute") },
        { value: 120, label: qsTr("For 2 minutes") },
        { value: 300, label: qsTr("For 5 minutes") },
        { value: 600, label: qsTr("For 10 minutes") },
        { value: 900, label: qsTr("For 15 minutes") },
        { value: 1200, label: qsTr("For 20 minutes") },
        { value: 1800, label: qsTr("For 30 minutes") },
        { value: 3600, label: qsTr("For 1 hour") },
        { value: 0, label: qsTr("Never") }
    ]

    // The require-password choices. The value is the delay *after* display-off.
    readonly property var requireAfterOptions: [
        { value: 0, label: qsTr("Immediately") },
        { value: 5, label: qsTr("After 5 seconds") },
        { value: 30, label: qsTr("After 30 seconds") },
        { value: 60, label: qsTr("After 1 minute") },
        { value: 300, label: qsTr("After 5 minutes") },
        { value: 600, label: qsTr("After 10 minutes") },
        { value: -1, label: qsTr("Never") }
    ]

    // A custom idle value shows its real duration, never `Never` (T278).
    readonly property var displayOffModel:
        root.withCustomOption(root.displayOffOptions, root.displayOffSeconds,
                              qsTr("For %1"))
    readonly property var requireAfterModel:
        root.withCustomOption(root.requireAfterOptions, root.requireAfter,
                              qsTr("After %1"))

    // The capture's energy note appears once the display-off delay is long.
    readonly property bool longDisplayOffDelay: root.displayOffSeconds >= 1200
        || root.displayOffSeconds === 0

    // Test surface (used by tst_settings_lock_screen.qml).
    property alias displayOffSelect: displayOffSelect
    property alias requirePasswordSelect: requirePasswordSelect
    property alias energyWarning: energyWarning
    property alias powerStageNote: powerStageNote
    property alias userNameToggle: userNameToggle
    property alias passwordHintsToggle: passwordHintsToggle
    property alias messageToggle: messageToggle
    property alias setMessageButton: setMessageButton
    property alias messageDialog: messageDialog
    property alias messageInput: messageInput
    property alias powerButtonsToggle: powerButtonsToggle
    property alias absenceNote: absenceNote
    property alias setHintButton: setHintButton
    property alias hintDialog: hintDialog
    property alias hintInput: hintInput
    property alias loginWindowSelect: loginWindowSelect
    property alias accessibilityButton: accessibilityButton
    property alias preview: preview
    property alias previewName: previewName
    property alias previewMessage: previewMessage
    property alias previewHint: previewHint
    property alias previewPowerButtons: previewPowerButtons
    property alias previewTime: previewTime
    property alias lockSummary: lockSummary

    signal navigateRequested(string paneId)

    readonly property var loginWindowOptions: [
        { value: "list", label: qsTr("List of users") },
        { value: "name-and-password", label: qsTr("Name and password") }
    ]

    // The editors' drafts (T278): Cancel restores the stored value instead of
    // leaving an unsaved edit in the field.
    property string messageDraft: root.lockMessage
    property string hintDraft: root.passwordHint

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    function formatDuration(seconds) {
        if (seconds <= 0)
            return qsTr("Never");
        if (seconds < 60) {
            return seconds === 1 ? qsTr("1 second")
                                 : qsTr("%1 seconds").arg(seconds);
        }
        if (seconds % 3600 === 0) {
            var hours = seconds / 3600;
            return hours === 1 ? qsTr("1 hour") : qsTr("%1 hours").arg(hours);
        }
        var minutes = Math.round(seconds / 60);
        return minutes === 1 ? qsTr("1 minute") : qsTr("%1 minutes").arg(minutes);
    }

    function withCustomOption(options, value, pattern) {
        for (var i = 0; i < options.length; ++i) {
            if (Number(options[i].value) === Number(value))
                return options;
        }
        var list = options.slice();
        list.splice(list.length - 1, 0, {
            value: value,
            label: String(pattern).arg(root.formatDuration(value))
        });
        return list;
    }

    function loginWindowIndex(value) {
        for (var i = 0; i < root.loginWindowOptions.length; ++i) {
            if (root.loginWindowOptions[i].value === value)
                return i;
        }
        return 0;
    }

    function indexFor(options, value) {
        for (var i = 0; i < options.length; ++i) {
            if (Number(options[i].value) === Number(value))
                return i;
        }
        return options.length - 1;
    }

    // The lock stage is display-off + the after delay. `Never` (-1) and a
    // display that never turns off both disable the lock stage.
    function syncLockStage(after, blank) {
        if (after < 0 || blank <= 0) {
            Settings.set("idle.lock", 0);
            return;
        }
        Settings.set("idle.lock", blank + after);
    }

    function applyDisplayOff(value) {
        Settings.set("idle.blank", value);
        root.syncLockStage(root.requireAfter, value);
    }

    function applyRequireAfter(value) {
        Settings.set("lock.requirePasswordAfter", value);
        root.syncLockStage(value, root.displayOffSeconds);
    }

    readonly property string lockSummaryText: {
        if (root.requireAfter < 0)
            return qsTr("The session stays unlocked when the display turns off.");
        if (root.displayOffSeconds <= 0)
            return qsTr("The display never turns off, so the password is "
                        + "never required automatically.");
        if (root.requireAfter === 0)
            return qsTr("The session locks immediately when the display turns "
                        + "off.");
        return qsTr("The session locks %1 after the display turns off.")
            .arg(root.formatDuration(root.requireAfter));
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !Settings.available

            SettingsAbsenceNotice {
                id: absenceNote
                objectName: "lockScreenAbsenceNote"
                kind: "live-on-defaults"
                showWhenLive: true
                reason: qsTr("The settings daemon is not running, so these "
                             + "preferences apply from the built-in defaults. "
                             + "The lock itself is enforced by the compositor.")
            }
        }

        // ── Preview ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Preview")

            Rectangle {
                id: preview
                objectName: "lockScreenPreview"
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                height: 188
                radius: Theme.controls.button.radius
                color: "#141419"
                clip: true

                Column {
                    anchors.centerIn: parent
                    spacing: Theme.primitive.spacing.xs

                    Rectangle {
                        id: previewAvatar
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: root.showUserNameAndPhoto
                        width: Theme.primitive.spacing.xxl
                        height: width
                        radius: width / 2
                        color: "#3a3a44"

                        Icon {
                            anchors.centerIn: parent
                            name: "home"
                            size: Theme.primitive.spacing.lg
                            color: "#d0d0d6"
                        }
                    }

                    Text {
                        id: previewName
                        objectName: "lockScreenPreviewName"
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: root.showUserNameAndPhoto
                        text: qsTr("User")
                        color: "#ffffff"
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }

                    Text {
                        id: previewTime
                        objectName: "lockScreenPreviewTime"
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "9:41"
                        color: "#ffffff"
                        font.pixelSize: Theme.primitive.font.sizeXl
                        font.weight: Theme.primitive.font.weightRegular
                    }

                    Text {
                        id: previewMessage
                        objectName: "lockScreenPreviewMessage"
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: root.showMessageWhenLocked
                                 && root.lockMessage.length > 0
                        text: root.lockMessage
                        color: "#d0d0d6"
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                        width: preview.width - 24
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Text {
                        id: previewHint
                        objectName: "lockScreenPreviewHint"
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: root.showPasswordHints
                                 && root.passwordHint.length > 0
                        text: root.passwordHint
                        color: "#a0a0a8"
                        font.pixelSize: Theme.primitive.font.sizeSm
                        elide: Text.ElideRight
                        width: preview.width - 24
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Row {
                        id: previewPowerButtons
                        objectName: "lockScreenPreviewPowerButtons"
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: root.showPowerButtons
                        spacing: Theme.primitive.spacing.xs

                        Repeater {
                            model: [qsTr("Sleep"), qsTr("Restart"),
                                    qsTr("Shut Down")]

                            delegate: Rectangle {
                                required property string modelData
                                width: powerLabel.implicitWidth
                                       + 2 * Theme.primitive.spacing.sm
                                height: 24
                                radius: 12
                                color: "#2a2a32"

                                Text {
                                    id: powerLabel
                                    anchors.centerIn: parent
                                    text: parent.modelData
                                    color: "#e0e0e6"
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Display and password timing ───────────────────────────────────
        SettingsGroup {
            id: timingGroup
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Turn display off when inactive")
                description: qsTr("The session blanks the display after this "
                                  + "much inactivity.")
                controlData: Select {
                    id: displayOffSelect
                    accessibleName: qsTr("Turn display off when inactive")
                    model: root.displayOffModel
                    onActivated: (index) => root.applyDisplayOff(
                        root.displayOffModel[index].value)
                }
            }

            // ADR 0279: Linux has one display-off stage, not a battery/AC
            // pair. The row is the whole story; this note says so.
            Text {
                id: powerStageNote
                objectName: "lockScreenPowerStageNote"
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                text: qsTr("One display-off stage: this session does not vary "
                           + "the timeout by power source.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                bottomPadding: Theme.primitive.spacing.xs
            }

            // The capture's yellow-triangle warning, adapted to Linux (one
            // blank stage, so it triggers on the delay rather than on battery).
            Row {
                id: energyWarning
                objectName: "lockScreenEnergyWarning"
                width: parent.width
                visible: root.longDisplayOffDelay
                spacing: Theme.primitive.spacing.sm
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs

                Canvas {
                    width: 16
                    height: 16
                    anchors.verticalCenter: parent.verticalCenter
                    renderStrategy: Canvas.Immediate
                    onPaint: {
                        var ctx = getContext("2d");
                        ctx.reset();
                        ctx.fillStyle = Theme.color.warning;
                        ctx.beginPath();
                        ctx.moveTo(8, 1);
                        ctx.lineTo(15.5, 14.5);
                        ctx.lineTo(0.5, 14.5);
                        ctx.closePath();
                        ctx.fill();
                        ctx.fillStyle = Theme.color.accentContent;
                        ctx.fillRect(7, 6, 2, 5);
                        ctx.fillRect(7, 12, 2, 1.6);
                    }
                    Component.onCompleted: requestPaint()
                }

                Text {
                    objectName: "lockScreenEnergyWarningText"
                    width: parent.width - 16 - Theme.primitive.spacing.sm
                    text: qsTr("Energy usage may be higher when this computer "
                               + "is inactive for longer periods of time before "
                               + "the display turns off.")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    wrapMode: Text.WordWrap
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Require password after display off or lock")
                description: root.lockSummaryText
                showSeparator: false
                controlData: Select {
                    id: requirePasswordSelect
                    accessibleName: qsTr("Require password after display off or lock")
                    model: root.requireAfterModel
                    onActivated: (index) => root.applyRequireAfter(
                        root.requireAfterModel[index].value)
                }
            }

            Text {
                id: lockSummary
                objectName: "lockScreenLockSummary"
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                text: root.lockSummaryText
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                bottomPadding: Theme.primitive.spacing.xs
            }
        }

        // ── Lock-screen display options ───────────────────────────────────
        SettingsGroup {
            id: displayGroup
            width: parent.width

            SettingsRow {
                width: parent.width
                label: qsTr("Show user name and photo")
                controlData: Toggle {
                    id: userNameToggle
                    accessibleName: qsTr("Show user name and photo")
                    onToggled: (checked) => Settings.set(
                        "lock.showUserNameAndPhoto", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show password hints")
                settingsId: "passwordHints"
                owner: "settingsd"
                settingsKey: "lock.showPasswordHints/lock.passwordHint"
                controlData: Row {
                    spacing: Theme.primitive.spacing.md

                    Toggle {
                        id: passwordHintsToggle
                        accessibleName: qsTr("Show password hints")
                        anchors.verticalCenter: parent.verticalCenter
                        onToggled: (checked) => Settings.set(
                            "lock.showPasswordHints", checked)
                    }

                    Button {
                        id: setHintButton
                        objectName: "lockScreenSetHintButton"
                        text: qsTr("Set…")
                        enabled: root.showPasswordHints
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: hintDialog.show()
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show message when locked")
                controlData: Row {
                    spacing: Theme.primitive.spacing.md

                    Toggle {
                        id: messageToggle
                        accessibleName: qsTr("Show message when locked")
                        anchors.verticalCenter: parent.verticalCenter
                        onToggled: (checked) => Settings.set(
                            "lock.showMessageWhenLocked", checked)
                    }

                    Button {
                        id: setMessageButton
                        objectName: "lockScreenSetMessageButton"
                        text: qsTr("Set…")
                        enabled: root.showMessageWhenLocked
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: messageDialog.show()
                    }
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show the Sleep, Restart, and Shut Down buttons")
                showSeparator: false
                controlData: Toggle {
                    id: powerButtonsToggle
                    accessibleName: qsTr("Show the Sleep, Restart, and Shut Down buttons")
                    onToggled: (checked) => Settings.set(
                        "lock.showPowerButtons", checked)
                }
            }
        }

        // ── Login window ──────────────────────────────────────────────────
        SettingsGroup {
            id: loginGroup
            width: parent.width
            title: qsTr("Login Window")

            SettingsRow {
                width: parent.width
                label: qsTr("Login window shows")
                description: qsTr("The display manager owns the login window; "
                                  + "it does not expose a style setting on this "
                                  + "host, so the choice is disabled.")
                settingsId: "loginWindow"
                owner: "settingsd"
                settingsKey: "login.windowStyle"
                controlData: Select {
                    id: loginWindowSelect
                    accessibleName: qsTr("Login window shows")
                    enabled: false
                    model: root.loginWindowOptions
                    onActivated: (index) => Settings.set(
                        "login.windowStyle",
                        root.loginWindowOptions[index].value)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Accessibility options…")
                description: qsTr("Open the Accessibility pane for the lock "
                                  + "screen's keyboard and visual options.")
                settingsId: "accessibility"
                owner: "settingsd"
                settingsKey: "a11y.lockOptions"
                showSeparator: false
                controlData: Button {
                    id: accessibilityButton
                    text: qsTr("Open…")
                    accessibleName: qsTr("Open Accessibility settings")
                    onClicked: root.navigateRequested("accessibility")
                }
            }
        }

        // ── When Switching User ───────────────────────────────────────────
        SettingsGroup {
            id: switchingGroup
            width: parent.width
            title: qsTr("When Switching User")

            Text {
                objectName: "lockScreenSwitchingNote"
                width: parent.width
                text: qsTr("The list of users on the login screen is provided "
                           + "by the display manager; configuring it arrives "
                           + "with the Users & Groups pane.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }
        }
    }

    // The message editor the trailing `Set...` button opens. `accept()` writes
    // the draft; `reject()` restores the stored value (T278).
    Dialog {
        id: messageDialog
        objectName: "lockScreenMessageDialog"
        title: qsTr("Lock Screen Message")
        message: qsTr("This message appears on the lock screen whenever it is "
                      + "shown.")
        dismissible: true

        contentData: Rectangle {
            width: parent.width
            height: 34
            radius: Theme.primitive.radius.md
            color: Theme.color.controlFill
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border

            TextInput {
                id: messageInput
                objectName: "lockScreenMessageInput"
                anchors.fill: parent
                anchors.leftMargin: Theme.primitive.spacing.md
                anchors.rightMargin: Theme.primitive.spacing.md
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                selectionColor: Theme.color.selection
                selectedTextColor: Theme.color.textPrimary
                selectByMouse: true
                clip: true
                font.pixelSize: Theme.controls.button.fontSize
                text: root.messageDraft
                onTextEdited: root.messageDraft = text
                onAccepted: messageDialog.accept()
            }

            Text {
                anchors.fill: messageInput
                visible: messageInput.text.length === 0
                text: qsTr("Message")
                color: Theme.color.textTertiary
                verticalAlignment: Text.AlignVCenter
                font.pixelSize: Theme.controls.button.fontSize
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: messageDialog.reject()
            }
            Button {
                text: qsTr("Set")
                variant: "primary"
                onClicked: messageDialog.accept()
            }
        }

        onOpened: {
            root.messageDraft = root.lockMessage;
            messageInput.forceActiveFocus();
        }
        onAccepted: Settings.set("lock.message", root.messageDraft)
        onRejected: root.messageDraft = root.lockMessage
    }

    // The password-hint editor (T229a). The hint is shown under the lock
    // screen's password field when `lock.showPasswordHints` is on.
    Dialog {
        id: hintDialog
        objectName: "lockScreenHintDialog"
        title: qsTr("Password Hint")
        message: qsTr("This hint appears under the password field when "
                      + "password hints are on.")
        dismissible: true

        contentData: Rectangle {
            width: parent.width
            height: 34
            radius: Theme.primitive.radius.md
            color: Theme.color.controlFill
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border

            TextInput {
                id: hintInput
                objectName: "lockScreenHintInput"
                anchors.fill: parent
                anchors.leftMargin: Theme.primitive.spacing.md
                anchors.rightMargin: Theme.primitive.spacing.md
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.color.textPrimary
                selectionColor: Theme.color.selection
                selectedTextColor: Theme.color.textPrimary
                selectByMouse: true
                clip: true
                font.pixelSize: Theme.controls.button.fontSize
                text: root.hintDraft
                onTextEdited: root.hintDraft = text
                onAccepted: hintDialog.accept()
            }

            Text {
                anchors.fill: hintInput
                visible: hintInput.text.length === 0
                text: qsTr("Hint")
                color: Theme.color.textTertiary
                verticalAlignment: Text.AlignVCenter
                font.pixelSize: Theme.controls.button.fontSize
            }
        }

        buttonsData: Row {
            spacing: Theme.controls.dialog.buttonGap

            Button {
                text: qsTr("Cancel")
                onClicked: hintDialog.reject()
            }
            Button {
                text: qsTr("Set")
                variant: "primary"
                onClicked: hintDialog.accept()
            }
        }

        onOpened: {
            root.hintDraft = root.passwordHint;
            hintInput.forceActiveFocus();
        }
        onAccepted: Settings.set("lock.passwordHint", root.hintDraft)
        onRejected: root.hintDraft = root.passwordHint
    }

    // Keep the controls in step with external changes.
    Binding {
        target: displayOffSelect
        property: "currentIndex"
        value: root.indexFor(root.displayOffModel, root.displayOffSeconds)
    }
    Binding {
        target: requirePasswordSelect
        property: "currentIndex"
        value: root.indexFor(root.requireAfterModel, root.requireAfter)
    }
    Binding {
        target: userNameToggle
        property: "checked"
        value: root.showUserNameAndPhoto
    }
    Binding {
        target: passwordHintsToggle
        property: "checked"
        value: root.showPasswordHints
    }
    Binding {
        target: messageToggle
        property: "checked"
        value: root.showMessageWhenLocked
    }
    Binding {
        target: powerButtonsToggle
        property: "checked"
        value: root.showPowerButtons
    }
    Binding {
        target: loginWindowSelect
        property: "currentIndex"
        value: root.loginWindowIndex(root.loginWindowStyle)
    }
    Connections {
        target: Settings
        function onValuesChanged() {
            // An external change converges into the drafts unless the user is
            // mid-edit with the dialog open.
            if (!messageDialog.open)
                root.messageDraft = root.lockMessage;
            if (!hintDialog.open)
                root.hintDraft = root.passwordHint;
        }
    }
}
