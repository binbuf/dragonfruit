// SPDX-License-Identifier: GPL-3.0-or-later
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

    signal activated()

    readonly property string timeText: Qt.formatTime(root.now,
        Qt.locale().timeFormat(root.showSeconds ? Locale.LongFormat : Locale.ShortFormat))
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
            color: Theme.color.textSecondary
            font.pixelSize: Theme.controls.menuBar.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            text: root.timeText
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.menuBar.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    HoverHandler { id: clockHover }

    TapHandler {
        onTapped: root.activated()
    }

    Accessible.role: Accessible.StaticText
    Accessible.name: root.accessibleText
    Accessible.focusable: true
    Accessible.onPressAction: root.activated()
}
