// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Menu-bar clock (T-09 FR-5). Formats through the active locale's time
// format (12- or 24-hour, as the locale dictates). It never polls: a single
// re-armed one-shot timer fires on the next minute (or second) boundary, so
// an idle menu bar contributes one scheduled wakeup per minute, not a
// repeating sample loop (FR-6).
Item {
    id: root

    property date now: new Date()
    property bool showSeconds: false
    property bool showDate: false
    // T226: force 24-hour time instead of the locale's own format. The
    // default follows the locale, so an untouched profile is unchanged.
    property bool use24Hour: false
    // T200: wallpaper-adaptive menu-bar foreground, injected by MenuBar.
    property color foreground: Theme.color.textPrimary
    property color secondaryForeground: Theme.color.textSecondary
    // The owning slot disables the wallpaper shadow when its accent capsule
    // already supplies the contrast (selected / keyboard focus).
    property bool shadowEnabled: true
    // The wallpaper text treatment follows the actual lettering: white text
    // takes the dark shadow pair, black text the light one.
    readonly property bool lightLettering: {
        var c = root.foreground;
        return (0.299 * c.r + 0.587 * c.g + 0.114 * c.b) > 0.5;
    }
    readonly property color labelShadowColor:
        root.lightLettering ? Theme.controls.wallpaperText.shadowColorOnDark
                            : Theme.controls.wallpaperText.shadowColorOnLight
    readonly property real labelShadowOpacity:
        root.lightLettering ? Theme.controls.wallpaperText.shadowOpacityOnDark
                            : Theme.controls.wallpaperText.shadowOpacityOnLight
    // T231: the clock is a registry item rendered inside `StatusItem`. The
    // slot owns hover, the focus ring, the tap and the accessibility node, so
    // the embedded clock is presentation only. Standalone uses (the QML
    // tests, the lock scene) keep the interactive default.
    property bool interactive: true

    signal activated()

    readonly property string timeText: Qt.formatTime(root.now,
        root.use24Hour
            ? (root.showSeconds ? "HH:mm:ss" : "HH:mm")
            : Qt.locale().timeFormat(root.showSeconds ? Locale.LongFormat : Locale.ShortFormat))
    readonly property string dateText: Qt.formatDate(root.now, Locale.ShortFormat)
    readonly property string accessibleText: root.showDate ? root.dateText + " " + root.timeText
                                                           : root.timeText

    implicitWidth: clockContent.implicitWidth + 2 * Theme.controls.menuBar.statusItemPaddingH
    implicitHeight: Theme.controls.menuBar.height

    function arm() {
        var d = new Date();
        var untilBoundary = root.showSeconds
            ? 1000 - d.getMilliseconds()
            : (60 - d.getSeconds()) * 1000 - d.getMilliseconds();
        tick.interval = Math.max(50, untilBoundary);
        tick.restart();
    }

    Timer {
        id: tick
        repeat: false
        onTriggered: {
            root.now = new Date();
            root.arm();
        }
    }

    Component.onCompleted: arm()

    Rectangle {
        id: clockBackground
        anchors.centerIn: parent
        width: root.width
        height: Theme.controls.menuBar.height - 2 * Theme.primitive.spacing.xs
        radius: Theme.controls.menuBar.hoverRadius
        readonly property color hoverFill: Qt.rgba(
            root.foreground.r, root.foreground.g, root.foreground.b,
            Theme.controls.menuBar.hoverOpacity)
        color: clockHover.hovered ? hoverFill : "transparent"
        antialiasing: true

        Behavior on color {
            enabled: !Theme.reducedMotion
            ColorAnimation {
                duration: Theme.controls.menuBar.hoverDuration
                easing.type: Easing.OutCubic
            }
        }
    }

    Row {
        id: clockContent
        anchors.centerIn: parent
        spacing: Theme.controls.menuBar.clockGap

        WallpaperLabel {
            visible: root.showDate
            text: root.dateText
            color: root.secondaryForeground
            shadowEnabled: root.shadowEnabled
            shadowColor: root.labelShadowColor
            shadowOpacity: root.labelShadowOpacity
            font.pixelSize: Theme.controls.menuBar.fontSize
            font.features: ({ "tnum": 1 })
            anchors.verticalCenter: parent.verticalCenter
        }

        WallpaperLabel {
            text: root.timeText
            color: root.foreground
            shadowEnabled: root.shadowEnabled
            shadowColor: root.labelShadowColor
            shadowOpacity: root.labelShadowOpacity
            font.pixelSize: Theme.controls.menuBar.fontSize
            font.features: ({ "tnum": 1 })
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    HoverHandler { id: clockHover; enabled: root.interactive }

    TapHandler {
        enabled: root.interactive
        onTapped: root.activated()
    }

    Accessible.ignored: !root.interactive
    Accessible.role: Accessible.StaticText
    Accessible.name: root.accessibleText
    Accessible.focusable: root.interactive
    Accessible.onPressAction: {
        if (root.interactive)
            root.activated();
    }
}
