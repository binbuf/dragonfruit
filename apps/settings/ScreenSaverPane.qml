// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Screen Saver pane (T227; completed by T278).
//
// The shipped screen saver is the first-party lock scene: the style selects
// its clock presentation, and the clock/date/seconds options are the same
// `screenSaver.*` keys the shell's lock scene consumes. The gallery previews
// are rendered from those keys, so the tile the user picks is the scene they
// get.
//
// Start After is the session idle engine's `idle.blank` stage: the display
// turns off after this much inactivity, and the lock scene is the shipped
// saver. `None` disables the saver, so Start After and the clock options are
// disabled with a reason instead of silently changing the display policy.
// Animated savers are not advertised until a renderer exists.
Item {
    id: root

    readonly property var styleOptions: [
        { value: "clock", label: qsTr("Clock") },
        { value: "none", label: qsTr("None") }
    ]

    // T278: the macOS set plus 2 h/5 h/never. `0` disables the stage in the
    // idle engine's spelling.
    readonly property var startAfterOptions: [
        { value: 0, label: qsTr("Never") },
        { value: 60, label: qsTr("After 1 minute") },
        { value: 300, label: qsTr("After 5 minutes") },
        { value: 600, label: qsTr("After 10 minutes") },
        { value: 1200, label: qsTr("After 20 minutes") },
        { value: 1800, label: qsTr("After 30 minutes") },
        { value: 3600, label: qsTr("After 1 hour") },
        { value: 7200, label: qsTr("After 2 hours") },
        { value: 18000, label: qsTr("After 5 hours") }
    ]

    readonly property string style: Settings.values["screenSaver.style"] || "clock"
    readonly property bool saverEnabled: root.style !== "none"
    readonly property int startAfter: {
        var value = Number(Settings.values["idle.blank"]);
        return isFinite(value) ? Math.max(0, Math.round(value)) : 300;
    }
    readonly property bool clockVisible:
        Settings.values["screenSaver.clock"] !== false
    readonly property bool dateVisible:
        Settings.values["screenSaver.showDate"] !== false
    readonly property bool secondsVisible:
        Settings.values["screenSaver.showSeconds"] === true

    // A custom idle value is shown as its real duration, never as "Never".
    readonly property var startAfterModel: {
        var list = root.startAfterOptions.slice();
        for (var i = 0; i < list.length; ++i) {
            if (list[i].value === root.startAfter)
                return list;
        }
        list.splice(list.length - 1, 0, {
            value: root.startAfter,
            label: qsTr("After %1").arg(root.durationLabel(root.startAfter))
        });
        return list;
    }

    signal navigationRequested(string paneId)

    property alias styleGallery: styleRepeater
    property alias startAfterSelect: startAfterSelect
    property alias clockToggle: clockToggle
    property alias showDateToggle: showDateToggle
    property alias showSecondsToggle: showSecondsToggle
    property alias hotCornersButton: hotCornersButton
    property alias styleGroup: styleGroup
    property alias optionsGroup: optionsGroup
    property alias startAfterGroup: startAfterGroup
    property alias disabledReason: disabledReason
    property var styleTiles: []

    function styleIndex() {
        for (var i = 0; i < root.styleOptions.length; ++i) {
            if (root.styleOptions[i].value === root.style)
                return i;
        }
        return 0;
    }

    function startAfterIndex() {
        for (var i = 0; i < root.startAfterModel.length; ++i) {
            if (root.startAfterModel[i].value === root.startAfter)
                return i;
        }
        return 0;
    }

    function durationLabel(seconds) {
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

    function chooseStyle(value) {
        Settings.set("screenSaver.style", value);
    }

    implicitWidth: 480
    implicitHeight: content.implicitHeight
    width: parent ? parent.width : implicitWidth

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Style gallery ─────────────────────────────────────────────────
        SettingsGroup {
            id: styleGroup
            width: parent.width
            title: qsTr("Screen Saver")

            Text {
                objectName: "screenSaverGalleryDescription"
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                text: qsTr("The first-party lock scene is the shipped screen "
                           + "saver. Choose the clock style or turn it off.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                topPadding: Theme.primitive.spacing.xs
            }

            Row {
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                spacing: Theme.primitive.spacing.md
                topPadding: Theme.primitive.spacing.sm

                Repeater {
                    id: styleRepeater
                    model: root.styleOptions

                    delegate: StyleTile {
                        required property var modelData
                        required property int index
                        width: Math.max(0, (parent.width
                                            - (root.styleOptions.length - 1)
                                              * parent.spacing)
                                           / root.styleOptions.length)
                        value: modelData.value
                        label: modelData.label
                    }
                }
            }
        }

        // ── Start After ───────────────────────────────────────────────────
        SettingsGroup {
            id: startAfterGroup
            width: parent.width
            title: qsTr("Start After")

            SettingsRow {
                width: parent.width
                label: qsTr("Start after")
                description: root.saverEnabled
                    ? qsTr("The display turns off after this much inactivity. "
                           + "The lock scene is the shipped screen saver.")
                    : qsTr("The screen saver is off, so there is no start time.")
                settingsId: "startAfter"
                owner: "settingsd -> compositor idle"
                settingsKey: "idle.blank"
                showSeparator: false

                controlData: Select {
                    id: startAfterSelect
                    accessibleName: qsTr("Start the screen saver after")
                    enabled: root.saverEnabled
                    model: root.startAfterModel
                    onActivated: (index) => Settings.set(
                                    "idle.blank",
                                    root.startAfterModel[index].value)
                }
            }

            Text {
                id: disabledReason
                objectName: "screenSaverDisabledReason"
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                visible: !root.saverEnabled
                text: qsTr("Choose the Clock style to enable the screen saver.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                bottomPadding: Theme.primitive.spacing.xs
            }
        }

        // ── Clock options ─────────────────────────────────────────────────
        SettingsGroup {
            id: optionsGroup
            width: parent.width
            title: qsTr("Clock Options")

            SettingsRow {
                width: parent.width
                label: qsTr("Show with clock")
                description: root.saverEnabled
                    ? "" : qsTr("The screen saver is off.")
                settingsId: "clock"
                owner: "settingsd -> shell/LockScreen"
                settingsKey: "screenSaver.clock"
                controlData: Toggle {
                    id: clockToggle
                    text: ""
                    enabled: root.saverEnabled
                    onToggled: (checked) => Settings.set("screenSaver.clock", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show date")
                settingsId: "showDate"
                owner: "settingsd -> shell/LockScreen"
                settingsKey: "screenSaver.showDate"
                controlData: Toggle {
                    id: showDateToggle
                    text: ""
                    enabled: root.saverEnabled && root.clockVisible
                    onToggled: (checked) => Settings.set("screenSaver.showDate", checked)
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Show seconds")
                settingsId: "showSeconds"
                owner: "settingsd -> shell/LockScreen"
                settingsKey: "screenSaver.showSeconds"
                showSeparator: false
                controlData: Toggle {
                    id: showSecondsToggle
                    text: ""
                    enabled: root.saverEnabled && root.clockVisible
                    onToggled: (checked) => Settings.set("screenSaver.showSeconds", checked)
                }
            }
        }

        // ── Hot Corners ───────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            title: qsTr("Hot Corners")

            SettingsRow {
                width: parent.width
                label: qsTr("Hot corners")
                description: qsTr("The four corner assignments are shared with "
                                  + "Workspace Overview in Desktop & Dock.")
                settingsId: "hotCorners"
                owner: "settingsd -> compositor/input"
                settingsKey: "overview.hotCorner*"
                showSeparator: false

                controlData: Button {
                    id: hotCornersButton
                    text: qsTr("Open Hot Corners\u2026")
                    accessibleName: qsTr("Open the Hot Corners controls")
                    // T292b: the assignments live in Desktop & Dock's merged
                    // Workspace Overview, in its Advanced Workspace Overview
                    // sheet; the pane is the landed home (the row is gone).
                    onClicked: root.navigationRequested("desktop-dock")
                }
            }
        }
    }

    // One real preview tile. The clock plate renders the live
    // clock/date/seconds choices, so it is a preview of the actual lock scene,
    // not a generic icon.
    component StyleTile: Item {
        id: tile

        property string value: ""
        property string label: ""
        readonly property bool selected: root.style === tile.value
        readonly property bool hovered: hoverHandler.hovered
        readonly property string timeText: root.secondsVisible ? "9:41:05" : "9:41"
        readonly property string dateText: "Monday 1 January"

        width: 0
        height: plate.height + Theme.primitive.spacing.xs + caption.height
        activeFocusOnTab: true

        function choose() {
            root.chooseStyle(tile.value);
        }

        Component.onCompleted: root.styleTiles = root.styleTiles.concat([tile])
        Component.onDestruction: root.styleTiles = root.styleTiles.filter(
            function(item) { return item !== tile; })

        Rectangle {
            id: plate
            width: parent.width
            height: Math.round(width / 1.6)
            radius: Theme.controls.button.radius
            color: "#141419"
            border.width: tile.selected ? Theme.controls.focusRing.width
                                        : Theme.controls.window.borderWidth
            border.color: tile.selected ? Theme.color.accent : Theme.color.border
            clip: true

            // The clock style: a real mini lock scene.
            Column {
                visible: tile.value !== "none"
                anchors.centerIn: parent
                spacing: Theme.primitive.spacing.xxs

                Text {
                    objectName: "screenSaverPreviewTime"
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: tile.timeText
                    color: "#ffffff"
                    font.pixelSize: Theme.primitive.font.sizeXl
                    font.weight: Theme.primitive.font.weightRegular
                }

                Text {
                    objectName: "screenSaverPreviewDate"
                    anchors.horizontalCenter: parent.horizontalCenter
                    visible: root.dateVisible
                    text: tile.dateText
                    color: "#d0d0d6"
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    visible: root.clockVisible
                    width: Theme.primitive.spacing.xxl
                    height: 3
                    radius: 2
                    color: "#7a7a84"
                }
            }

            Column {
                visible: tile.value === "none"
                anchors.centerIn: parent
                spacing: Theme.primitive.spacing.xs

                Icon {
                    anchors.horizontalCenter: parent.horizontalCenter
                    name: "displays"
                    size: Theme.primitive.spacing.xl
                    color: "#8a8a92"
                }

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: qsTr("Off")
                    color: "#d0d0d6"
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }

            Rectangle {
                anchors.fill: parent
                radius: plate.radius
                color: Theme.color.controlHover
                opacity: tile.hovered ? 0.14 : 0
            }

            Rectangle {
                visible: tile.selected
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: Theme.primitive.spacing.xs
                width: Theme.primitive.spacing.md
                height: width
                radius: width / 2
                color: Theme.color.accent

                Icon {
                    anchors.centerIn: parent
                    name: "check"
                    size: Theme.primitive.font.sizeSm
                    color: Theme.color.accentContent
                }
            }
        }

        Text {
            id: caption
            anchors.top: plate.bottom
            anchors.topMargin: Theme.primitive.spacing.xs
            width: parent.width
            height: Theme.primitive.font.sizeSm + 2 * Theme.primitive.spacing.xxs
            text: tile.label
            color: Theme.color.textPrimary
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        HoverHandler {
            id: hoverHandler
            cursorShape: Qt.PointingHandCursor
        }

        TapHandler {
            onTapped: tile.choose()
        }

        FocusRing {
            target: tile
            cornerRadius: Theme.controls.button.radius
            shown: tile.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                tile.choose();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: tile.label
        Accessible.description: tile.value === "none"
            ? qsTr("No screen saver")
            : qsTr("The first-party lock scene clock")
        Accessible.checkable: true
        Accessible.checked: tile.selected
        Accessible.focusable: true
        Accessible.onPressAction: tile.choose()
    }

    Binding {
        target: startAfterSelect
        property: "currentIndex"
        value: root.startAfterIndex()
    }
    Binding {
        target: clockToggle
        property: "checked"
        value: root.clockVisible
    }
    Binding {
        target: showDateToggle
        property: "checked"
        value: root.dateVisible
    }
    Binding {
        target: showSecondsToggle
        property: "checked"
        value: root.secondsVisible
    }
}
