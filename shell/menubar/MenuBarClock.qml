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
        color: clockHover.hovered ? Theme.color.controlHover : "transparent"
        antialiasing: true
    }

    Row {
        id: clockContent
        anchors.centerIn: parent
        spacing: Theme.controls.menuBar.clockGap

        Text {
            visible: root.showDate
            text: root.dateText
            color: root.secondaryForeground
            font.pixelSize: Theme.controls.menuBar.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.timeText
            color: root.foreground
            font.pixelSize: Theme.controls.menuBar.fontSize
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
