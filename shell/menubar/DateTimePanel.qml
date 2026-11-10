// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Date & Time panel (T231): a clock popover, not a menu. It shows the
// current time/date using the same `menu.clock.*` and `clock.use24Hour`
// preferences as the bar clock, and a month calendar with keyboard traversal.
//
// The surface reuses the status-popover material preset (`statusMenu`) and the
// design-system Popup, so the liquid-glass rollout, fallback tiers, reduced
// motion and Escape dismissal are shared with every other status popover. The
// panel deliberately exposes no application actions: a calendar day is a
// selection (and raises `dayActivated`), never a hidden shell command.
Item {
    id: root

    property date now: new Date()
    property date selectedDate: new Date()
    property date visibleMonth: new Date()
    property bool showSeconds: false
    property bool use24Hour: false
    property Item anchorItem: null

    property alias popup: popup
    property alias open: popup.open
    property alias dayGrid: dayGrid
    property alias dayRepeater: dayRepeater
    property alias monthTitleText: monthTitle
    property alias todayButton: todayButton
    property alias previousButton: previousButton
    property alias nextButton: nextButton

    // T-21.2c: the menu bar pushes the compositor's Popup role state here.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0

    // The panel is a sibling overlay in the bar scene. Matching the bar's
    // box makes the shared Popup's edge clamp use the bar width, so a clock
    // near the right edge opens a right-aligned panel instead of x=0.
    width: parent ? parent.width : 0
    height: parent ? parent.height : 0

    signal dayActivated(date date)
    signal closed()

    onShowSecondsChanged: {
        if (popup.open)
            arm();
    }

    readonly property int firstDayOfWeek: Qt.locale().firstDayOfWeek
    readonly property int columns: 7
    readonly property int cellSpacing: Theme.primitive.spacing.xxs
    readonly property real cellWidth:
        Math.max(24, Math.floor((dayGrid.width
                                 - (columns - 1) * cellSpacing) / columns))
    readonly property real cellHeight: 26

    readonly property string timeText: Qt.formatTime(root.now,
        root.use24Hour
            ? (root.showSeconds ? "HH:mm:ss" : "HH:mm")
            : Qt.locale().timeFormat(root.showSeconds ? Locale.LongFormat
                                                      : Locale.ShortFormat))
    readonly property string dateText: Qt.formatDate(root.now, Locale.LongFormat)
    readonly property string monthTitleTextValue:
        Qt.formatDate(root.visibleMonth, "MMMM yyyy")

    readonly property var weekdayLabels: {
        var labels = [];
        // 2024-01-07 is a Sunday; walk from the locale's first day.
        var base = new Date(2024, 0, 7);
        for (var i = 0; i < 7; ++i) {
            var day = new Date(2024, 0, 7 + ((root.firstDayOfWeek + i) % 7));
            labels.push(Qt.formatDate(day, "ddd"));
        }
        return labels;
    }

    function sameDay(a, b) {
        return a.getFullYear() === b.getFullYear()
            && a.getMonth() === b.getMonth()
            && a.getDate() === b.getDate();
    }

    function startOfMonth(date) {
        return new Date(date.getFullYear(), date.getMonth(), 1);
    }

    function daysInMonth(year, month) {
        return new Date(year, month + 1, 0).getDate();
    }

    function addDays(date, days) {
        return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
    }

    function selectDate(date) {
        root.selectedDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());
        root.visibleMonth = root.startOfMonth(root.selectedDate);
    }

    function moveDay(delta) {
        root.selectDate(root.addDays(root.selectedDate, delta));
    }

    function shiftMonth(delta) {
        var target = new Date(root.visibleMonth.getFullYear(),
                              root.visibleMonth.getMonth() + delta, 1);
        var day = Math.min(root.selectedDate.getDate(),
                           root.daysInMonth(target.getFullYear(), target.getMonth()));
        root.selectedDate = new Date(target.getFullYear(), target.getMonth(), day);
        root.visibleMonth = target;
    }

    function goToday() {
        root.now = new Date();
        root.selectedDate = root.now;
        root.visibleMonth = root.startOfMonth(root.now);
    }

    function activateSelectedDay() {
        root.dayActivated(new Date(root.selectedDate.getFullYear(),
                                   root.selectedDate.getMonth(),
                                   root.selectedDate.getDate()));
    }

    function arm() {
        var d = new Date();
        var untilBoundary = root.showSeconds
            ? 1000 - d.getMilliseconds()
            : (60 - d.getSeconds()) * 1000 - d.getMilliseconds();
        tick.interval = Math.max(50, untilBoundary);
        tick.restart();
    }

    readonly property var calendarCells: {
        var year = root.visibleMonth.getFullYear();
        var month = root.visibleMonth.getMonth();
        var first = new Date(year, month, 1);
        var offset = (first.getDay() - root.firstDayOfWeek + 7) % 7;
        var start = new Date(year, month, 1 - offset);
        var cells = [];
        for (var i = 0; i < 42; ++i) {
            var d = new Date(start.getFullYear(), start.getMonth(), start.getDate() + i);
            var inMonth = d.getMonth() === month && d.getFullYear() === year;
            var today = root.sameDay(d, root.now);
            var selected = root.sameDay(d, root.selectedDate);
            var name = Qt.formatDate(d, Locale.LongFormat);
            if (today)
                name = qsTr("%1, today").arg(name);
            cells.push({ date: d, day: d.getDate(), inMonth: inMonth,
                         today: today, selected: selected, accessibleName: name });
        }
        return cells;
    }

    Timer {
        id: tick
        repeat: false
        onTriggered: {
            var wasToday = root.sameDay(root.selectedDate, root.now);
            root.now = new Date();
            if (wasToday)
                root.selectedDate = root.now;
            root.arm();
        }
    }

    Popup {
        id: popup
        objectName: "dateTimePopup"
        anchorItem: root.anchorItem
        preferredWidth: 318
        accessibleRole: Accessible.Pane
        accessibleName: qsTr("Date & Time")
        escapeCloses: true
        motionPreset: "popup"
        materialPreset: "statusMenu"
        materialBackdrop: "trusted"
        materialBlurActive: root.materialBlurActive
        materialTier: root.materialTier
        materialFallback: root.materialFallback

        onOpened: {
            root.now = new Date();
            root.selectedDate = root.now;
            root.visibleMonth = root.startOfMonth(root.now);
            root.arm();
        }
        onClosed: {
            tick.stop();
            root.closed();
        }

        // Keyboard traversal: arrows move the selection by a day/week,
        // PageUp/PageDown move by month, Home returns to today, Return raises
        // the selected day. Escape is the shared Popup dismissal.
        Keys.onLeftPressed: (event) => { root.moveDay(-1); event.accepted = true; }
        Keys.onRightPressed: (event) => { root.moveDay(1); event.accepted = true; }
        Keys.onUpPressed: (event) => { root.moveDay(-7); event.accepted = true; }
        Keys.onDownPressed: (event) => { root.moveDay(7); event.accepted = true; }
        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_PageUp) {
                root.shiftMonth(-1);
                event.accepted = true;
            } else if (event.key === Qt.Key_PageDown) {
                root.shiftMonth(1);
                event.accepted = true;
            } else if (event.key === Qt.Key_Home) {
                root.goToday();
                event.accepted = true;
            } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                       || event.key === Qt.Key_Space) {
                root.activateSelectedDay();
                event.accepted = true;
            }
        }

        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            // ── The current date/time ──────────────────────────────────
            Column {
                width: parent.width
                spacing: Theme.primitive.spacing.xxs

                Text {
                    objectName: "dateTimePanelTime"
                    width: parent.width
                    text: root.timeText
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeXxl
                    font.weight: Theme.primitive.font.weightSemibold
                }

                Text {
                    objectName: "dateTimePanelDate"
                    width: parent.width
                    text: root.dateText
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    elide: Text.ElideRight
                }
            }

            // ── Month navigation ──────────────────────────────────────
            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.xs

                Item {
                    id: previousButton
                    objectName: "dateTimePreviousButton"
                    width: 24
                    height: 24
                    Accessible.role: Accessible.Button
                    Accessible.name: qsTr("Previous month")
                    Accessible.focusable: true
                    Accessible.onPressAction: root.shiftMonth(-1)

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.primitive.radius.sm
                        color: previousHover.hovered ? Theme.color.controlHover
                                                     : "transparent"
                    }
                    TablerIcon {
                        anchors.centerIn: parent
                        name: "chevron-left"
                        size: 14
                        color: Theme.color.textSecondary
                    }
                    HoverHandler { id: previousHover }
                    TapHandler { onTapped: root.shiftMonth(-1) }
                }

                Text {
                    id: monthTitle
                    objectName: "dateTimeMonthTitle"
                    width: parent.width - 2 * 24 - 2 * Theme.primitive.spacing.xs
                    height: 24
                    text: root.monthTitleTextValue
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeLg
                    font.weight: Theme.primitive.font.weightMedium
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }

                Item {
                    id: nextButton
                    objectName: "dateTimeNextButton"
                    width: 24
                    height: 24
                    Accessible.role: Accessible.Button
                    Accessible.name: qsTr("Next month")
                    Accessible.focusable: true
                    Accessible.onPressAction: root.shiftMonth(1)

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.primitive.radius.sm
                        color: nextHover.hovered ? Theme.color.controlHover
                                                 : "transparent"
                    }
                    TablerIcon {
                        anchors.centerIn: parent
                        name: "chevron-right"
                        size: 14
                        color: Theme.color.textSecondary
                    }
                    HoverHandler { id: nextHover }
                    TapHandler { onTapped: root.shiftMonth(1) }
                }
            }

            // ── Weekday header ────────────────────────────────────────
            Row {
                width: parent.width
                spacing: root.cellSpacing

                Repeater {
                    model: root.weekdayLabels

                    delegate: Text {
                        required property string modelData
                        width: root.cellWidth
                        text: modelData
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeXs
                        horizontalAlignment: Text.AlignHCenter
                    }
                }
            }

            // ── Day grid ──────────────────────────────────────────────
            Grid {
                id: dayGrid
                width: parent.width
                columns: root.columns
                rowSpacing: root.cellSpacing
                columnSpacing: root.cellSpacing

                Repeater {
                    id: dayRepeater
                    model: root.calendarCells

                    delegate: Rectangle {
                        required property var modelData
                        required property int index

                        width: root.cellWidth
                        height: root.cellHeight
                        radius: Theme.primitive.radius.sm
                        color: modelData.selected ? Theme.color.accent
                             : (modelData.today ? Theme.color.accentMuted : "transparent")
                        antialiasing: true

                        HoverHandler { id: dayHover }

                        Rectangle {
                            anchors.fill: parent
                            radius: parent.radius
                            visible: dayHover.hovered && !modelData.selected
                            color: Theme.color.controlHover
                        }

                        Text {
                            anchors.centerIn: parent
                            text: modelData.day
                            color: modelData.selected ? Theme.color.accentContent
                                 : (modelData.inMonth ? Theme.color.textPrimary
                                                      : Theme.color.textTertiary)
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.weight: modelData.today
                                         ? Theme.primitive.font.weightSemibold
                                         : Theme.primitive.font.weightRegular
                        }

                        TapHandler {
                            onTapped: root.selectDate(modelData.date)
                        }

                        Accessible.role: Accessible.Button
                        Accessible.name: modelData.accessibleName
                        Accessible.description: modelData.today
                                                ? qsTr("Today") : ""
                        Accessible.focusable: true
                        Accessible.selected: modelData.selected
                        Accessible.onPressAction: root.selectDate(modelData.date)
                    }
                }
            }

            // ── Footer ────────────────────────────────────────────────
            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                Item {
                    id: todayButton
                    objectName: "dateTimeTodayButton"
                    width: todayLabel.implicitWidth + 2 * Theme.primitive.spacing.sm
                    height: 26
                    Accessible.role: Accessible.Button
                    Accessible.name: qsTr("Show today")
                    Accessible.focusable: true
                    Accessible.onPressAction: root.goToday()

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.primitive.radius.sm
                        color: todayHover.hovered ? Theme.color.controlHover
                                                  : "transparent"
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border
                    }
                    Text {
                        id: todayLabel
                        anchors.centerIn: parent
                        text: qsTr("Today")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                    HoverHandler { id: todayHover }
                    TapHandler { onTapped: root.goToday() }
                }
            }
        }
    }
}
