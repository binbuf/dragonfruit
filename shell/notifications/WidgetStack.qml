// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Notification Center widget stack (T234).
//
// Widgets live below the notification stack, inside the Center's measured
// lower-panel material band. The live panel size is the measured 164 pt
// calendar card; the gallery's 102×102 thumbnail is never used here
// (structural review §5.4 rule 2). Widgets are optional: an empty list is the
// deliberate "No widgets" state, and a widget whose provider is absent shows
// its own unavailable text rather than invented data.
Item {
    id: root

    // Each entry is `{ id, kind, size, title, available, data }`.
    property var widgets: []
    // False while the notification/status service is absent.
    property bool available: true

    signal removeRequested(string id)

    readonly property int cardSize: 164
    readonly property int cardGap: Theme.primitive.spacing.sm

    implicitWidth: 328
    implicitHeight: cardSize

    Accessible.role: Accessible.List
    Accessible.name: qsTr("Widgets")

    function monthName(date) {
        return Qt.locale().monthName(date.getMonth(), Locale.LongFormat);
    }

    function weekdayInitials() {
        var names = [];
        for (var day = 0; day < 7; ++day) {
            // A fixed Monday-first reference week; the locale name is what the
            // user reads.
            var date = new Date(2024, 0, 1 + day); // 2024-01-01 is a Monday
            names.push(Qt.locale().dayName(date.getDay(), Locale.ShortFormat)
                           .substring(0, 2));
        }
        return names;
    }

    // The live month grid, Monday first. The shell recomputes it when the
    // widget is shown; no provider is involved.
    readonly property var calendar: {
        var now = new Date();
        var year = now.getFullYear();
        var month = now.getMonth();
        var first = new Date(year, month, 1);
        // JS Date: Sunday = 0. Convert to a Monday-first offset.
        var offset = (first.getDay() + 6) % 7;
        var days = new Date(year, month + 1, 0).getDate();
        var cells = [];
        for (var i = 0; i < offset; ++i)
            cells.push(0);
        for (var day = 1; day <= days; ++day)
            cells.push(day);
        return {
            year: year,
            month: month,
            title: monthName(now),
            today: now.getDate(),
            cells: cells,
            weekdays: weekdayInitials()
        };
    }

    Text {
        objectName: "widgetEmpty"
        visible: root.widgets.length === 0
        anchors.centerIn: parent
        width: parent.width - 2 * Theme.primitive.spacing.lg
        text: root.available ? qsTr("No widgets")
                             : qsTr("Widgets unavailable")
        color: Theme.color.textTertiary
        font.pixelSize: Theme.primitive.font.sizeSm
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        Accessible.role: Accessible.StaticText
        Accessible.name: text
    }

    Flickable {
        id: strip
        anchors.fill: parent
        visible: root.widgets.length > 0
        contentWidth: row.implicitWidth
        contentHeight: root.cardSize
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.HorizontalFlick

        Row {
            id: row
            spacing: root.cardGap

            Repeater {
                model: root.widgets
                delegate: Rectangle {
                    id: card
                    required property var modelData
                    objectName: "widget_" + modelData.id
                    readonly property bool live: modelData.available !== false
                    readonly property string kind:
                        modelData.kind !== undefined ? String(modelData.kind) : "calendar"
                    readonly property var payload:
                        modelData.data !== undefined ? modelData.data : ({})

                    // The live panel sizes: the measured Calendar is medium
                    // (164 pt); small is the gallery thumbnail size, large and
                    // extra-large take the full panel width.
                    readonly property string size:
                        modelData.size !== undefined
                            ? String(modelData.size) : "medium"
                    width: size === "small" ? 102
                         : (size === "large" || size === "extra-large")
                           ? root.width : root.cardSize
                    height: size === "small" ? 102 : root.cardSize
                    radius: Theme.controls.popup.radius
                    color: Theme.color.surfaceMuted
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border

                    Accessible.role: Accessible.Grouping
                    Accessible.name: modelData.title !== undefined
                        ? String(modelData.title) : qsTr("Widget")

                    // The remove affordance appears while the pointer is over
                    // the card. A HoverHandler on the card (not the hidden
                    // button) is what makes the button reachable: an invisible
                    // MouseArea never receives hover.
                    HoverHandler {
                        id: cardHover
                    }

                    // ── Calendar (the measured 164 pt live size) ──────────
                    Column {
                        visible: card.kind === "calendar" && card.live
                        anchors.fill: parent
                        anchors.margins: Theme.primitive.spacing.sm
                        spacing: Theme.primitive.spacing.xxs

                        Row {
                            width: parent.width

                            Text {
                                objectName: "calendarMonth"
                                width: parent.width - calendarYear.width
                                text: root.calendar.title
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                font.weight: Theme.primitive.font.weightSemibold
                                elide: Text.ElideRight
                            }
                            Text {
                                id: calendarYear
                                text: root.calendar.year
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeXs
                            }
                        }

                        Row {
                            width: parent.width
                            Repeater {
                                model: root.calendar.weekdays
                                delegate: Text {
                                    required property var modelData
                                    width: parent.width / 7
                                    text: modelData
                                    color: Theme.color.textTertiary
                                    font.pixelSize: 8
                                    horizontalAlignment: Text.AlignHCenter
                                }
                            }
                        }

                        Grid {
                            id: calendarGrid
                            objectName: "calendarGrid"
                            width: parent.width
                            columns: 7
                            rows: Math.ceil(root.calendar.cells.length / 7)
                            rowSpacing: 0

                            Repeater {
                                model: root.calendar.cells
                                delegate: Item {
                                    required property var modelData
                                    width: calendarGrid.width / 7
                                    height: 15

                                    Rectangle {
                                        anchors.centerIn: parent
                                        width: 15
                                        height: 15
                                        radius: 7.5
                                        visible: modelData === root.calendar.today
                                        color: Theme.color.accent
                                    }

                                    Text {
                                        anchors.centerIn: parent
                                        visible: modelData > 0
                                        text: modelData > 0 ? modelData : ""
                                        color: modelData === root.calendar.today
                                            ? Theme.color.accentContent
                                            : Theme.color.textSecondary
                                        font.pixelSize: 8
                                        font.weight: modelData === root.calendar.today
                                            ? Theme.primitive.font.weightSemibold
                                            : Theme.primitive.font.weightRegular
                                    }
                                }
                            }
                        }
                    }

                    // ── Weather (live data or an explicit absence) ────────
                    Column {
                        visible: card.kind === "weather"
                        anchors.fill: parent
                        anchors.margins: Theme.primitive.spacing.sm
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            width: parent.width
                            text: card.payload.location !== undefined
                                ? String(card.payload.location) : qsTr("Weather")
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeXs
                            elide: Text.ElideRight
                        }
                        Text {
                            objectName: "weatherTemperature"
                            width: parent.width
                            visible: card.live && card.payload.temperature !== undefined
                            text: card.live && card.payload.temperature !== undefined
                                ? String(card.payload.temperature) : ""
                            color: Theme.color.textPrimary
                            font.pixelSize: 34
                            font.weight: Theme.primitive.font.weightSemibold
                        }
                        Text {
                            objectName: "weatherCondition"
                            width: parent.width
                            visible: card.live && card.payload.condition !== undefined
                            text: card.live && card.payload.condition !== undefined
                                ? String(card.payload.condition) : ""
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            wrapMode: Text.WordWrap
                        }
                        Text {
                            objectName: "widgetUnavailable"
                            width: parent.width
                            visible: !card.live
                            text: qsTr("Weather unavailable")
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            wrapMode: Text.WordWrap
                        }
                    }

                    // ── Generic live/empty widget ─────────────────────────
                    Column {
                        visible: card.kind !== "calendar" && card.kind !== "weather"
                        anchors.fill: parent
                        anchors.margins: Theme.primitive.spacing.sm
                        spacing: Theme.primitive.spacing.xxs

                        Text {
                            width: parent.width
                            text: modelData.title !== undefined
                                ? String(modelData.title) : qsTr("Widget")
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.weight: Theme.primitive.font.weightSemibold
                            elide: Text.ElideRight
                        }
                        Text {
                            objectName: "widgetValue"
                            width: parent.width
                            visible: card.live && card.payload.value !== undefined
                            text: card.live && card.payload.value !== undefined
                                ? String(card.payload.value) : ""
                            color: Theme.color.textSecondary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            wrapMode: Text.WordWrap
                        }
                        Text {
                            objectName: "widgetEmptyBody"
                            width: parent.width
                            visible: !card.live
                            text: qsTr("No data")
                            color: Theme.color.textTertiary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            wrapMode: Text.WordWrap
                        }
                    }

                    // Remove affordance: visible on hover, reachable through
                    // the gallery as well.
                    Rectangle {
                        objectName: "widgetRemove"
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: Theme.primitive.spacing.xs
                        width: 20
                        height: 20
                        radius: 10
                        visible: cardHover.hovered || removeArea.containsMouse
                        color: Theme.color.surfaceElevated
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Remove widget")

                        Icon {
                            anchors.centerIn: parent
                            name: "close"
                            size: 12
                            color: Theme.color.textSecondary
                        }

                        MouseArea {
                            id: removeArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: root.removeRequested(String(card.modelData.id))
                        }
                    }
                }
            }
        }
    }
}
