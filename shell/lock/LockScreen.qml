// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The first-party lock screen (T-12.3a): a pure view the shell controller
// renders into the compositor's `ext-session-lock-v1` surfaces, one per
// output. The layout follows the reference lock screen: the live wallpaper
// fills the output, the date and large time sit at the top center, and the
// account portrait, password field, and optional power actions sit at the
// bottom center. Authentication (PAM) and input capture land in
// T-12.3b/T-12.3c; this slice presents the fail-secure lock and its identity.
Item {
    id: root

    // The wallpaper behind the locked session. The shell pushes the active
    // Space's source (or the configured fallback), the Space's solid colour,
    // and the fit mode. An empty source with no colour falls back to the
    // neutral gradient below.
    property url wallpaperSource: ""
    property color wallpaperColor: "transparent"
    property string wallpaperFit: "fill"

    // The signed-in account shown on the lock scene. The session owner is
    // fixed for T-12.3a; PAM authentication (T-12.3b) binds the real user.
    property string userName: qsTr("Dragonfruit")
    // Live clock, driven by the shell controller each render.
    property string timeText: ""
    property string dateText: ""
    // T227: the Screen Saver pane's clock options. The shell reads the
    // `screenSaver.*` keys and pushes these; the lock scene stays a pure view.
    property bool showClock: true
    property bool showDate: true
    property bool showSeconds: false
    // T229a: the Lock Screen pane's display policy. The shell reads the
    // `lock.*` keys and pushes these; the scene never enforces the lock.
    property bool showUserNameAndPhoto: true
    property bool showPasswordHints: false
    property string passwordHint: ""
    property bool showLockMessage: false
    property string lockMessageText: ""
    property bool showPowerButtons: true

    signal powerActionRequested(string action)
    // The circular enter control inside the password field. The shell owns
    // the captured password buffer, so the scene only reports the intent.
    signal submitPasswordRequested()

    // Authentication state (T-12.3b). Until then the field is present but
    // inert, so the layout is final.
    property bool authEnabled: false
    property bool authBusy: false
    // A short status line under the field (wrong password, PAM unavailable).
    property string message: ""
    // How many characters the captured password has (T-12.3c). The buffer
    // itself stays in the shell controller; the scene only draws the mask.
    property int passwordLength: 0
    // True while the lock scene is the live session lock. The shell clears it
    // on teardown so the caret blink timer cannot keep a hidden scene awake.
    property bool active: true
    // The caret blink phase. Reset to visible whenever the field changes.
    property bool caretVisible: true

    // The reference layout scales with the output instead of a fixed card.
    readonly property int timePixelSize: Math.round(Math.max(56,
        Math.min(root.height * 0.14, root.width * 0.16)))
    readonly property int datePixelSize: Math.round(Math.max(
        Theme.primitive.font.sizeLg,
        Math.min(root.height * 0.024, root.width * 0.03)))
    readonly property int avatarSize: Math.round(Math.max(64,
        Math.min(88, root.height * 0.12)))
    readonly property int fieldWidth: Math.max(0, Math.min(320,
        root.width - 2 * Theme.primitive.spacing.xl))
    // Text starts a few pixels right of the caret so the empty-field caret is
    // not drawn on top of the placeholder's first glyph.
    readonly property int fieldCaretInset: Theme.primitive.spacing.lg
    readonly property int fieldTextInset: root.fieldCaretInset + 6
    readonly property string userInitial: root.userName.length > 0
        ? root.userName.charAt(0).toUpperCase() : "?"
    // `df_workspace.wallpaper_fit` spells the four modes; map them onto the
    // QML image fill modes without letting the lock scene re-implement the
    // compositor's sampling geometry.
    readonly property int wallpaperFillMode: {
        switch (root.wallpaperFit) {
        case "fit":
            return Image.PreserveAspectFit;
        case "stretch":
            return Image.Stretch;
        case "center":
            return Image.Pad;
        default:
            return Image.PreserveAspectCrop;
        }
    }

    function passwordMask() {
        var mask = "";
        for (var i = 0; i < root.passwordLength; ++i)
            mask += "\u2022";
        return mask;
    }

    function restartCaret() {
        root.caretVisible = true;
        if (caretTimer.running)
            caretTimer.restart();
    }

    onPasswordLengthChanged: root.restartCaret()
    onAuthBusyChanged: root.restartCaret()
    onActiveChanged: root.restartCaret()

    // The field is always the lock scene's input target, so the caret blinks
    // whenever the lock is live and PAM is not verifying a submitted answer.
    Timer {
        id: caretTimer
        interval: 530
        repeat: true
        running: root.active && root.authEnabled && !root.authBusy
        onTriggered: root.caretVisible = !root.caretVisible
    }

    function submitPassword() {
        if (!root.authEnabled || root.authBusy || root.passwordLength <= 0)
            return;
        root.submitPasswordRequested();
    }

    clip: true
    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Lock screen")
    Accessible.description: root.message.length > 0 ? root.message
                                                     : qsTr("Session locked.")

    // The neutral fallback when no wallpaper source or solid colour exists.
    Rectangle {
        id: wallpaperFallback
        anchors.fill: parent
        gradient: Gradient {
            GradientStop {
                position: 0.0
                color: Theme.primitive.color.neutral900
            }
            GradientStop {
                position: 1.0
                color: Theme.primitive.color.neutral950
            }
        }
    }

    // A Space with no image keeps its solid colour; draw it under the image so
    // transparent or letterboxed wallpapers still read as the session's own.
    Rectangle {
        id: wallpaperColorFill
        objectName: "lockWallpaperColor"
        anchors.fill: parent
        color: root.wallpaperColor
        visible: root.wallpaperColor.a > 0
    }

    Image {
        id: wallpaperImage
        objectName: "lockWallpaper"
        // `center` keeps the image at its native size and centres it; every
        // other mode fills the output and lets the fill mode crop or letterbox.
        anchors.centerIn: parent
        width: root.wallpaperFit === "center" ? implicitWidth : root.width
        height: root.wallpaperFit === "center" ? implicitHeight : root.height
        source: root.wallpaperSource
        fillMode: root.wallpaperFillMode
        asynchronous: false
        cache: true
        smooth: true
        mipmap: true
        visible: status === Image.Ready
    }

    // The reference lock screen keeps the wallpaper visible but tames its top
    // and bottom edges so the white clock and controls stay legible on any
    // image.
    Rectangle {
        id: wallpaperScrim
        objectName: "lockWallpaperScrim"
        anchors.fill: parent
        gradient: Gradient {
            GradientStop {
                position: 0.0
                color: Qt.rgba(0, 0, 0, 0.72)
            }
            GradientStop {
                position: 0.40
                color: Qt.rgba(0, 0, 0, 0.32)
            }
            GradientStop {
                position: 0.60
                color: Qt.rgba(0, 0, 0, 0.12)
            }
            GradientStop {
                position: 1.0
                color: Qt.rgba(0, 0, 0, 0.60)
            }
        }
    }

    // Top center: date first, then the large time, as in the reference lock
    // screen. The clock is the primary anchor; the date is the smaller line.
    Column {
        id: clock
        visible: root.showClock
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: Math.max(Theme.primitive.spacing.xxxl,
                                    Math.round(parent.height * 0.09))
        spacing: Theme.primitive.spacing.xs

        WallpaperLabel {
            objectName: "lockDate"
            visible: root.showDate
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.dateText
            color: Qt.rgba(1, 1, 1, 0.88)
            font.pixelSize: root.datePixelSize
            font.weight: Theme.primitive.font.weightMedium
            shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
            shadowOpacity: Theme.controls.wallpaperText.increaseContrastShadowOpacity
            outlineOpacity: Theme.controls.wallpaperText.increaseContrastOutlineOpacity
        }
        WallpaperLabel {
            objectName: "lockClock"
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.timeText
            color: "#ffffff"
            font.pixelSize: root.timePixelSize
            font.weight: Theme.primitive.font.weightMedium
            font.features: ({ "tnum": 1 })
            font.letterSpacing: -Math.round(root.timePixelSize * 0.02)
            shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
            shadowOpacity: Theme.controls.wallpaperText.increaseContrastShadowOpacity
            outlineOpacity: Theme.controls.wallpaperText.increaseContrastOutlineOpacity
        }
    }

    // Bottom center: portrait, name, password field, status lines, and the
    // optional power actions.
    Column {
        id: bottomContent
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Math.max(Theme.primitive.spacing.xl,
                                       Math.round(parent.height * 0.105))
        spacing: Theme.primitive.spacing.md

        // The portrait disc with the account's initial.
        Rectangle {
            id: avatar
            objectName: "lockAvatar"
            visible: root.showUserNameAndPhoto
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.avatarSize
            height: width
            radius: width / 2
            color: Theme.color.accent
            border.width: 2
            border.color: Qt.rgba(1, 1, 1, 0.42)
            antialiasing: true

            Text {
                anchors.centerIn: parent
                text: root.userInitial
                color: Theme.color.accentContent
                font.pixelSize: Math.round(parent.width * 0.42)
                font.weight: Theme.primitive.font.weightSemibold
            }
        }

        WallpaperLabel {
            objectName: "lockUserName"
            visible: root.showUserNameAndPhoto
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.userName
            color: "#ffffff"
            font.pixelSize: Theme.primitive.font.sizeXl
            font.weight: Theme.primitive.font.weightSemibold
            width: Math.min(implicitWidth,
                            root.width - 2 * Theme.primitive.spacing.xl)
            elide: Text.ElideRight
            shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
            shadowOpacity: Theme.controls.wallpaperText.increaseContrastShadowOpacity
            outlineOpacity: Theme.controls.wallpaperText.increaseContrastOutlineOpacity
        }

        // The password field: a grey, semi-transparent pill with the circular
        // enter control fitted inside its right edge. T-12.3b wires it to PAM;
        // here it is the visual contract the compositor's input capture feeds.
        Rectangle {
            id: field
            objectName: "lockPasswordField"
            anchors.horizontalCenter: parent.horizontalCenter
            width: root.fieldWidth
            height: 44
            radius: height / 2
            color: Qt.rgba(0.18, 0.18, 0.20, 0.58)
            border.width: 1
            border.color: Qt.rgba(1, 1, 1, root.authEnabled ? 0.30 : 0.18)
            antialiasing: true

            Text {
                id: placeholderText
                objectName: "lockPasswordPlaceholder"
                anchors.left: parent.left
                anchors.leftMargin: root.fieldTextInset
                anchors.right: enterButton.left
                anchors.rightMargin: Theme.primitive.spacing.sm
                anchors.verticalCenter: parent.verticalCenter
                visible: root.passwordLength === 0
                text: root.authBusy ? qsTr("Authenticating…")
                                    : (root.authEnabled ? qsTr("Enter Password")
                                                        : qsTr("Press Enter to unlock"))
                color: Qt.rgba(1, 1, 1, 0.78)
                font.pixelSize: Theme.primitive.font.sizeLg
                elide: Text.ElideRight
            }

            // The masked password (T-12.3c): captured keys are shown as
            // bullets, never as text and never echoed.
            Text {
                id: maskText
                objectName: "lockPasswordMask"
                anchors.left: parent.left
                anchors.leftMargin: root.fieldTextInset
                anchors.right: enterButton.left
                anchors.rightMargin: Theme.primitive.spacing.sm
                anchors.verticalCenter: parent.verticalCenter
                visible: root.passwordLength > 0
                text: root.passwordMask()
                color: "#ffffff"
                font.pixelSize: Theme.primitive.font.sizeLg
                font.letterSpacing: 2
                elide: Text.ElideRight
            }

            // The text caret. The shell owns the password buffer, so this is
            // the only cursor the field can show: a blinking bar at the end of
            // the mask (or at the insertion point while the field is empty).
            Rectangle {
                id: caret
                objectName: "lockCaret"
                visible: root.active && root.caretVisible
                         && root.authEnabled && !root.authBusy
                width: 2
                height: Math.round(Theme.primitive.font.sizeLg * 1.15)
                radius: 1
                color: "#ffffff"
                anchors.verticalCenter: parent.verticalCenter
                x: {
                    if (root.passwordLength === 0)
                        return root.fieldCaretInset;
                    return Math.min(maskText.x + maskText.contentWidth + 2,
                                    enterButton.x - 4);
                }
            }

            Rectangle {
                id: enterButton
                objectName: "lockEnterButton"
                signal clicked()

                readonly property real inset: Math.max(4, Math.round(parent.height * 0.1))
                anchors.right: parent.right
                anchors.rightMargin: inset
                anchors.verticalCenter: parent.verticalCenter
                width: parent.height - 2 * inset
                height: width
                radius: width / 2
                color: enterTap.pressed ? Qt.rgba(1, 1, 1, 0.98)
                       : enterHover.hovered ? Qt.rgba(1, 1, 1, 0.94)
                       : Qt.rgba(1, 1, 1, 0.86)
                border.width: 1
                border.color: Qt.rgba(0, 0, 0, 0.10)
                opacity: root.authBusy ? 0.55
                         : (root.passwordLength > 0 ? 1.0 : 0.82)
                activeFocusOnTab: true
                antialiasing: true

                Text {
                    objectName: "lockEnterGlyph"
                    anchors.centerIn: parent
                    text: "\u2192"
                    color: Theme.primitive.color.neutral900
                    font.pixelSize: Math.round(parent.height * 0.52)
                    font.weight: Theme.primitive.font.weightSemibold
                }

                HoverHandler { id: enterHover }
                TapHandler {
                    id: enterTap
                    onTapped: enterButton.clicked()
                }
                Accessible.role: Accessible.Button
                Accessible.name: qsTr("Unlock")
                Accessible.onPressAction: enterButton.clicked()

                onClicked: root.submitPassword()
            }
        }

        WallpaperLabel {
            objectName: "lockPasswordHint"
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.showPasswordHints && root.passwordHint.length > 0
            text: root.passwordHint
            color: Qt.rgba(1, 1, 1, 0.82)
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            width: Math.min(root.fieldWidth, root.width - 2 * Theme.primitive.spacing.xl)
            shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
            shadowOpacity: Theme.controls.wallpaperText.increaseContrastShadowOpacity
            outlineOpacity: Theme.controls.wallpaperText.increaseContrastOutlineOpacity
        }

        WallpaperLabel {
            objectName: "lockMessage"
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.message.length > 0
            text: root.message
            color: "#ffb8c0"
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            width: Math.min(root.fieldWidth, root.width - 2 * Theme.primitive.spacing.xl)
            shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
            shadowOpacity: Theme.controls.wallpaperText.increaseContrastShadowOpacity
            outlineOpacity: Theme.controls.wallpaperText.increaseContrastOutlineOpacity
        }

        WallpaperLabel {
            objectName: "lockScreenMessage"
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.showLockMessage && root.lockMessageText.length > 0
            text: root.lockMessageText
            color: Qt.rgba(1, 1, 1, 0.82)
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            width: Math.min(root.fieldWidth, root.width - 2 * Theme.primitive.spacing.xl)
            shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
            shadowOpacity: Theme.controls.wallpaperText.increaseContrastShadowOpacity
            outlineOpacity: Theme.controls.wallpaperText.increaseContrastOutlineOpacity
        }

        // T229a: the reference power row. The shell performs the action
        // through logind; the scene only reports intent.
        Row {
            id: powerRow
            objectName: "lockPowerRow"
            visible: root.showPowerButtons
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.primitive.spacing.sm

            LockPowerButton {
                objectName: "lockSleepButton"
                text: qsTr("Sleep")
                onClicked: root.powerActionRequested("sleep")
            }
            LockPowerButton {
                objectName: "lockRestartButton"
                text: qsTr("Restart")
                onClicked: root.powerActionRequested("restart")
            }
            LockPowerButton {
                objectName: "lockShutDownButton"
                text: qsTr("Shut Down")
                onClicked: root.powerActionRequested("shut-down")
            }
        }
    }

    // The lock screen's own action button: a translucent capsule with the
    // wallpaper shadow treatment, so the power row reads on any background.
    component LockPowerButton: Item {
        id: action

        property string text: ""
        signal clicked()

        width: actionLabel.implicitWidth + 2 * Theme.primitive.spacing.md
        height: 28
        activeFocusOnTab: true
        Accessible.role: Accessible.Button
        Accessible.name: action.text
        Accessible.onPressAction: action.clicked()

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: actionTap.pressed ? Qt.rgba(1, 1, 1, 0.24)
                   : actionHover.hovered ? Qt.rgba(1, 1, 1, 0.16)
                   : Qt.rgba(0, 0, 0, 0.30)
            border.width: action.activeFocus ? Theme.controls.focusRing.width : 1
            border.color: action.activeFocus ? Theme.color.focusRing
                                             : Qt.rgba(1, 1, 1, 0.20)
            antialiasing: true
        }

        WallpaperLabel {
            id: actionLabel
            anchors.centerIn: parent
            text: action.text
            color: "#ffffff"
            font.pixelSize: Theme.primitive.font.sizeSm
            font.weight: Theme.primitive.font.weightMedium
            shadowColor: Theme.controls.wallpaperText.shadowColorOnDark
            shadowOpacity: Theme.controls.wallpaperText.increaseContrastShadowOpacity
            outlineOpacity: Theme.controls.wallpaperText.increaseContrastOutlineOpacity
        }

        HoverHandler { id: actionHover }
        TapHandler {
            id: actionTap
            onTapped: action.clicked()
        }
    }
}
