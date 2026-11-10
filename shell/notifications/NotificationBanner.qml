// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One notification banner card (T-11.1a; actions and activation T-11.1b).
//
// The shell controller renders this into the top-right `notification` overlay
// surface; the properties are the newest active notification from the
// service's `Banners()` view. The card grows to fit an inline action row when
// the app supplied actions, and the whole body clicks (the app's `default`
// action if it registered one, otherwise a dismiss). `actionInvoked` and
// `activated` are the shell controller's hooks into the notification service.
Item {
    id: root

    property string appName: ""
    property string summary: ""
    property string body: ""
    property string urgency: "normal"
    // The service's `actions` view: a list of { key, label } maps.
    property var actions: []
    // T234: enter/exit motion and the stacked look. The shell sets `present`
    // false for the exit, keeps the surface mapped for the short motion, then
    // unmaps. `stackCount` is the number of active banners; the card draws up
    // to two peeking edges behind the newest one.
    property bool present: true
    property bool reducedMotion: false
    property int stackCount: 1
    property real enterProgress: present ? 1 : 0
    // T-21.2c: the shell applies the compositor's Popup role state. Without
    // the sampled pass the card takes the token-driven milky plate.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    // T205: the banner is its own optical preset, not the shared Popup plate.
    // The compositor paints the measured flat band on the sampled path; the
    // QML complement is transparent there and paints the same resolved
    // tint/alpha on solid, software and Reduce Transparency. Legacy keeps the
    // pre-T205 popup fill.
    readonly property var bannerGlass: Glass.resolve("notificationBanner", {
        scheme: Theme.dark ? "dark" : "light",
        rollout: Glass.rollout,
        reduceTransparency: Glass.reduceTransparency,
        increaseContrast: Glass.increaseContrast,
        backend: materialFallback === 2 ? "software"
               : materialBlurActive ? "gpu-sampled" : "gpu-no-sampled-blur",
        tier: materialTier === 2 ? "minimal" : materialTier === 1 ? "reduced" : "full"
    })
    readonly property bool replacementLook: Glass.rollout === "replacement"
    readonly property bool sampledGlass: replacementLook && materialBlurActive
        && bannerGlass.renderMode === "sampled-glass"
    readonly property color cardFill: sampledGlass ? "transparent"
        : replacementLook
          ? Qt.rgba(bannerGlass.tint, bannerGlass.tint, bannerGlass.tint,
                    bannerGlass.alpha)
          : Material.popupFill(materialBlurActive)

    signal actionInvoked(string key)
    signal activated()

    readonly property bool hasActions: actions.length > 0
    readonly property int cardHeight: hasActions ? 132 : 96

    readonly property int padding: Theme.primitive.spacing.md
    readonly property color accent: root.urgency === "critical" ? Theme.color.danger
                                    : root.urgency === "low" ? Theme.color.textTertiary
                                    : Theme.color.accent

    // T205: the banner's one material region is the visible card, not the
    // surface's transparent strip below a no-action card. The compositor
    // commit-latches this list with the banner image; `banner` is the stable
    // key and `notificationBanner` the measured preset.
    // T234: the measured enter/exit motion. A banner must not steal focus;
    // this is a short, non-interactive slide/fade on the shell's own surface.
    readonly property real slideOffset: reducedMotion ? 0 : (1 - enterProgress) * -10
    Behavior on enterProgress {
        NumberAnimation {
            duration: root.reducedMotion ? 0 : 180
            easing.type: Easing.OutCubic
        }
    }
    opacity: root.enterProgress

    readonly property var materialRegions: [{
        key: "banner",
        preset: "notificationBanner",
        x: 0, y: slideOffset, w: width, h: cardHeight,
        tl: Theme.controls.popup.radius, tr: Theme.controls.popup.radius,
        br: Theme.controls.popup.radius, bl: Theme.controls.popup.radius,
        overscan: 8
    }]

    // The scene is exactly the surface; the transparent area below the card
    // passes clicks through (the shell only makes the card clickable).
    width: 380
    // The card is 132 pt with actions; the extra strip shows the stack lip.
    height: 144

    // T234: up to two peeking edges behind the newest card show that more
    // than one banner is queued. The service still exposes one card at a time;
    // the stack is a visual count, not a second banner model.
    Rectangle {
        id: peek2
        objectName: "bannerStackPeek2"
        visible: root.stackCount > 2
        x: 8
        y: root.slideOffset
        width: parent.width - 16
        height: root.cardHeight + 12
        radius: Theme.controls.popup.radius
        color: Theme.color.surfaceMuted
        opacity: 0.92
        z: -2
    }

    Rectangle {
        id: peek1
        objectName: "bannerStackPeek1"
        visible: root.stackCount > 1
        x: 4
        y: root.slideOffset
        width: parent.width - 8
        height: root.cardHeight + 6
        radius: Theme.controls.popup.radius
        color: Theme.color.surfaceElevated
        opacity: 0.94
        z: -1
    }

    Shadow {
        anchors.fill: card
        radius: card.radius
        level: "overlay"
        // T205: the compositor's flat band is opaque and already sits behind
        // the card. A QML shadow under a transparent sampled fill would show
        // through and darken the measured material, so it is only painted on
        // the solid/software paths where the card carries its own fill.
        visible: !root.sampledGlass
    }

    Rectangle {
        id: card
        objectName: "bannerCard"
        y: root.slideOffset
        width: parent.width
        height: root.cardHeight
        radius: Theme.controls.popup.radius
        // T205: the sampled compositor pass paints the banner material; the
        // QML card is transparent then and carries the same resolved fill on
        // the solid/software/accessibility paths. The rim stays opaque so the
        // card edge and urgency stripe remain legible at every tier.
        color: root.cardFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        // The urgency stripe, inset with the card's rounded corner left alone.
        Rectangle {
            id: stripe
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: Theme.primitive.spacing.sm
            width: 4
            radius: width / 2
            color: root.accent
        }

        Column {
            id: content
            anchors.left: stripe.right
            anchors.right: parent.right
            anchors.top: parent.top
            // Leave room for the action row when the card carries one.
            anchors.bottom: root.hasActions ? actionsRow.top : parent.bottom
            anchors.leftMargin: Theme.primitive.spacing.sm
            anchors.rightMargin: root.padding
            anchors.topMargin: root.padding
            anchors.bottomMargin: root.hasActions ? Theme.primitive.spacing.xs : root.padding
            spacing: Theme.primitive.spacing.xxs

            Text {
                width: parent.width
                visible: root.appName.length > 0
                objectName: "appName"
                text: root.appName
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                visible: root.summary.length > 0
                objectName: "summary"
                text: root.summary
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                visible: root.body.length > 0
                objectName: "body"
                text: root.body
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeMd
                wrapMode: Text.WordWrap
                maximumLineCount: root.hasActions ? 1 : 2
                elide: Text.ElideRight
            }
        }

        // Clicking the body fires the app's default action (or dismisses).
        MouseArea {
            id: bodyArea
            anchors.left: content.left
            anchors.right: content.right
            anchors.top: content.top
            anchors.bottom: root.hasActions ? actionsRow.top : content.bottom
            onClicked: root.activated()
        }

        Row {
            id: actionsRow
            objectName: "actionsRow"
            visible: root.hasActions
            height: 28
            anchors.left: content.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: root.padding
            anchors.bottomMargin: Theme.primitive.spacing.sm
            spacing: Theme.primitive.spacing.sm

            Repeater {
                model: root.actions
                delegate: Rectangle {
                    required property var modelData
                    objectName: "action_" + modelData.key
                    height: actionsRow.height
                    width: actionLabel.implicitWidth + 2 * Theme.primitive.spacing.md
                    radius: Theme.controls.sidebar.rowRadius
                    color: actionArea.containsMouse ? Theme.color.surfaceMuted
                                                    : "transparent"
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    Text {
                        id: actionLabel
                        anchors.centerIn: parent
                        text: modelData.label
                        color: Theme.color.accent
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.weight: Theme.primitive.font.weightMedium
                    }

                    MouseArea {
                        id: actionArea
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: root.actionInvoked(modelData.key)
                    }
                }
            }
        }
    }

    Accessible.role: Accessible.Alert
    Accessible.name: root.summary
    Accessible.description: root.body
}