// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Lock Screen pane (T-15.8b).
//
// The pane and the Control Center Lock Screen tile are one functional unit.
// Lock policy has no external daemon: the session idle/lock engine
// (`services/session/src/idle.rs`, ADR 0070) owns the stage timing and the
// compositor owns the one fail-secure lock state (ADR 0067); the shell mirrors
// both over the private `df_toplevel_manager` bridge. Like the Mission Control
// pane (T-15.5b), this pane is the *configuration* half and writes only
// settingsd keys, so every control applies live and persists:
//
//   * the two timing rows reuse the revision-5 `idle.blank` / `idle.lock`
//     keys the session idle engine already reads (`IdlePolicy::from_keys`);
//   * the four display rows write the revision-15 `lock.*` keys
//     (`dragonfruit-lock-adapter`'s `LockDisplayOption::id()` suffixes).
//
// The compositor lock hot path is never touched; the durable preference is
// the single source of truth the shell summarizes in the Control Center tile.
// Wiring the `lock.*` keys into the lock-screen renderer is a follow-up, as
// the Mission Control corner-apply request was (ADR 0127/0133).
//
// Deviations from the macOS capture reference (ADR 0122), recorded so later
// tasks do not re-litigate them:
//   * the battery/adapter pair collapses to one `Turn display off when
//     inactive` row, the Linux adaptation ADR 0132 fixed (one blank stage);
//   * the warning row is shown for any long display-off delay, not only on
//     battery;
//   * `Login window shows` is omitted (no login-window provider yet) and the
//     `Accessibility Options...` button is omitted (the Accessibility pane is
//     not shipped; linking it would be a dead control);
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

    // The require-password popup choices; `Never` (0) disables the lock stage.
    readonly property var requirePasswordOptions: [
        { value: 5, label: qsTr("After 5 seconds") },
        { value: 30, label: qsTr("After 30 seconds") },
        { value: 60, label: qsTr("After 1 minute") },
        { value: 300, label: qsTr("After 5 minutes") },
        { value: 600, label: qsTr("After 10 minutes") },
        { value: 900, label: qsTr("After 15 minutes") },
        { value: 0, label: qsTr("Never") }
    ]

    // The capture's energy note appears once the display-off delay is long.
    readonly property bool longDisplayOffDelay: root.displayOffSeconds >= 1200
        || root.displayOffSeconds === 0

    // Test surface (used by tst_settings_lock_screen.qml).
    property alias displayOffSelect: displayOffSelect
    property alias requirePasswordSelect: requirePasswordSelect
    property alias energyWarning: energyWarning
    property alias userNameToggle: userNameToggle
    property alias passwordHintsToggle: passwordHintsToggle
    property alias messageToggle: messageToggle
    property alias setMessageButton: setMessageButton
    property alias messageDialog: messageDialog
    property alias messageInput: messageInput
    property alias powerButtonsToggle: powerButtonsToggle
    property alias absenceNote: absenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function indexFor(options, value) {
        for (var i = 0; i < options.length; ++i) {
            if (Number(options[i].value) === Number(value))
                return i;
        }
        return options.length - 1;
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !Settings.available

            Text {
                id: absenceNote
                objectName: "lockScreenAbsenceNote"
                width: parent.width
                text: qsTr("The settings daemon is not running, so these "
                           + "preferences apply from the built-in defaults. "
                           + "The lock itself is enforced by the compositor.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
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
                    model: root.displayOffOptions
                    onActivated: (index) => Settings.set(
                        "idle.blank", root.displayOffOptions[index].value)
                }
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
                label: qsTr("Require password after the screen turns off")
                description: qsTr("The session locks after this much "
                                  + "inactivity. Choose Never to leave the "
                                  + "session unlocked.")
                showSeparator: false
                controlData: Select {
                    id: requirePasswordSelect
                    accessibleName: qsTr("Require password after the screen turns off")
                    model: root.requirePasswordOptions
                    onActivated: (index) => Settings.set(
                        "idle.lock", root.requirePasswordOptions[index].value)
                }
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
                controlData: Toggle {
                    id: passwordHintsToggle
                    accessibleName: qsTr("Show password hints")
                    onToggled: (checked) => Settings.set(
                        "lock.showPasswordHints", checked)
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
                        text: qsTr("Set\u2026")
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

    // The message editor the trailing `Set...` button opens (the capture's
    // sheet). `accept()` writes the text; `reject()` leaves it unchanged.
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

        onAccepted: Settings.set("lock.message", messageInput.text)
        onOpened: messageInput.forceActiveFocus()
    }

    // Keep the controls in step with external changes.
    Binding {
        target: displayOffSelect
        property: "currentIndex"
        value: root.indexFor(root.displayOffOptions, root.displayOffSeconds)
    }
    Binding {
        target: requirePasswordSelect
        property: "currentIndex"
        value: root.indexFor(root.requirePasswordOptions,
                             root.requirePasswordSeconds)
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
        target: messageInput
        property: "text"
        value: root.lockMessage
    }
}