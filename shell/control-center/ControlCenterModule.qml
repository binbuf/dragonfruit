// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One measured Control Center grid capsule (T235).
//
// The capsule is the live module surface: its size is the measured span
// (64×64, 140×64, 140×140, 292×64), and its body adapts to that span instead
// of stretching one full-width row. It is a pure view: the tap, toggle and
// slider gestures are raised to `ControlCenter.qml`, which owns the service
// and settingsd routing.
Rectangle {
    id: root

    // The normalized tile map from `ControlCenter.qml`.
    property var tile: ({})
    property bool sampled: false
    property bool reduceMotion: false

    // Token-resolved capsule colors (the sampled path is transparent because
    // the compositor paints the module region).
    property color fill: "transparent"
    property color hoverFill: "transparent"
    property color textPrimary: "#ffffff"
    property color textSecondary: "#cccccc"
    property color accent: "#ffffff"
    property color accentMuted: "transparent"
    property color controlFill: "transparent"
    property real rimWidth: 0
    property color rimColor: "transparent"
    property real cornerRadius: Theme.primitive.radius.md

    readonly property string moduleId: tile.id !== undefined ? String(tile.id) : ""
    readonly property string span: tile.span !== undefined ? String(tile.span) : "2x1"
    readonly property bool isToggle: tile.kind === "toggle"
    readonly property bool isSlider: tile.kind === "slider"
    readonly property bool isMedia: tile.kind === "media"
    readonly property bool isAction: tile.kind === "action"
    readonly property bool compact: span === "1x1"
    readonly property bool narrow: span === "2x1"
    readonly property bool wide: span === "4x1"
    readonly property bool tall: span === "2x2"
    readonly property bool unavailable: tile.available === false
    readonly property string title: tile.title !== undefined ? String(tile.title) : ""
    readonly property string subtitle: tile.subtitle !== undefined
        ? String(tile.subtitle) : ""
    readonly property string glyph: tile.glyph !== undefined ? String(tile.glyph) : "info"
    readonly property bool checked: tile.checked === true
    readonly property bool controlEnabled: tile.enabled !== false

    signal activated()
    signal toggleRequested(bool checked)
    signal sliderMoved(real value)
    signal mediaControlRequested(string action)

    objectName: tile.objectName !== undefined ? String(tile.objectName) : ""
    width: tile.w !== undefined ? tile.w : 140
    height: tile.h !== undefined ? tile.h : 64
    radius: root.cornerRadius
    color: capsuleHover.hovered ? root.hoverFill : root.fill
    border.width: root.rimWidth
    border.color: root.rimColor
    opacity: root.unavailable ? 0.6 : 1.0
    activeFocusOnTab: true

    Accessible.role: root.isToggle || root.isSlider ? Accessible.Grouping
                                                     : Accessible.Button
    Accessible.name: root.title
    Accessible.description: root.subtitle
    Accessible.focusable: true
    Accessible.onPressAction: root.activated()

    HoverHandler { id: capsuleHover }

    TapHandler {
        onTapped: root.activated()
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                || event.key === Qt.Key_Enter) {
            root.activated();
            event.accepted = true;
        }
    }

    FocusRing {
        target: root
        shown: root.activeFocus
    }

    // ── Icon disc ────────────────────────────────────────────────────────
    Rectangle {
        id: iconDisc
        objectName: root.moduleId + "Icon"
        width: root.compact || root.narrow ? 24 : root.tall ? 32 : 28
        height: width
        radius: width / 2
        color: (root.checked || root.isSlider) && !root.unavailable
            ? root.accentMuted : root.controlFill
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.leftMargin: root.compact ? (parent.width - width) / 2
                          : root.narrow ? 8 : 10
        anchors.topMargin: root.compact ? 8 : root.narrow ? 10 : 10

        Icon {
            anchors.centerIn: parent
            name: root.glyph
            size: parent.width * 0.55
            color: (root.checked || root.isSlider) && !root.unavailable
                ? root.accent : root.textSecondary
        }
    }

    // ── Text body ────────────────────────────────────────────────────────
    Column {
        id: textColumn
        visible: !root.compact
        anchors.left: iconDisc.right
        anchors.leftMargin: 8
        anchors.right: root.wide ? sliderControl.left
                      : (root.narrow ? parent.right
                         : (toggleControl.visible ? toggleControl.left : parent.right))
        anchors.rightMargin: root.narrow ? 8 : 8
        anchors.verticalCenter: root.tall ? undefined : parent.verticalCenter
        anchors.top: root.tall ? parent.top : undefined
        anchors.topMargin: root.tall ? 12 : 0
        spacing: 2

        Text {
            width: parent.width
            text: root.title
            color: root.textPrimary
            font.pixelSize: root.narrow ? Theme.primitive.font.sizeSm
                                        : Theme.controls.button.fontSize
            font.weight: Theme.primitive.font.weightMedium
            elide: Text.ElideRight
        }

        Text {
            objectName: root.moduleId + "Subtitle"
            visible: !(root.narrow && root.isToggle)
            width: parent.width
            text: root.subtitle
            color: root.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            elide: Text.ElideRight
            maximumLineCount: root.tall ? 3 : 1
            wrapMode: root.tall ? Text.WordWrap : Text.NoWrap
        }
    }

    // Compact 1×1 modules stack the label under the disc.
    Text {
        id: compactLabel
        visible: root.compact
        anchors.top: iconDisc.bottom
        anchors.topMargin: 4
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: 2
        anchors.rightMargin: 2
        text: root.title
        color: root.textPrimary
        font.pixelSize: 9
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideNone
        wrapMode: Text.WordWrap
        maximumLineCount: 2
    }

    // ── Toggle ───────────────────────────────────────────────────────────
    Toggle {
        id: toggleControl
        objectName: root.narrow || root.compact ? "" : root.moduleId + "Toggle"
        visible: root.isToggle && !root.compact && !root.narrow
        enabled: root.controlEnabled && !root.unavailable
        accessibleName: root.moduleId === "focus"
            ? qsTr("Do Not Disturb") : root.title
        anchors.right: parent.right
        anchors.rightMargin: root.narrow ? 8 : 10
        anchors.verticalCenter: root.narrow ? undefined : parent.verticalCenter
        anchors.bottom: root.narrow ? parent.bottom : undefined
        anchors.bottomMargin: root.narrow ? 8 : 0
        onToggled: (value) => root.toggleRequested(value)
    }

    // `Toggle` writes its own `checked` on tap, so the external state is a
    // Binding (the same seam the pre-T235 panel used).
    Binding {
        target: toggleControl
        property: "checked"
        value: root.checked
    }

    // The 2×1 modules use a compact switch so the 140 pt capsule still fits
    // the full title. It keeps the Toggle contract (`toggle()`, `toggled`,
    // Switch role) for keyboard/AT-SPI/tests.
    Rectangle {
        id: compactSwitch
        objectName: root.moduleId + "Toggle"
        visible: root.isToggle && root.narrow
        width: 30
        height: 18
        radius: 9
        color: root.checked ? root.accent : root.controlFill
        border.width: root.checked ? 0 : 1
        border.color: root.textSecondary
        opacity: root.controlEnabled && !root.unavailable ? 1.0 : 0.45
        enabled: root.controlEnabled && !root.unavailable
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 8
        activeFocusOnTab: true
        Accessible.role: Accessible.Switch
        Accessible.name: root.moduleId === "focus"
            ? qsTr("Do Not Disturb") : root.title
        Accessible.checked: root.checked
        Accessible.focusable: true
        Accessible.onPressAction: compactSwitch.toggle()
        function toggle() {
            if (root.controlEnabled && !root.unavailable)
                root.toggleRequested(!root.checked);
        }
        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                compactSwitch.toggle();
                event.accepted = true;
            }
        }
        Rectangle {
            width: 12
            height: 12
            radius: 6
            y: 3
            x: root.checked ? parent.width - width - 3 : 3
            color: Theme.color.controlKnob
        }
        TapHandler {
            onTapped: root.toggle()
        }
    }

    // A compact toggle dot keeps Dark Mode operable without a second row.
    Rectangle {
        id: compactToggle
        objectName: root.moduleId + "Toggle"
        visible: root.isToggle && root.compact
        width: 16
        height: 16
        radius: 8
        color: root.checked ? root.accent : root.controlFill
        border.width: root.checked ? 0 : 1
        border.color: root.textSecondary
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.rightMargin: 6
        anchors.bottomMargin: 6
        opacity: root.controlEnabled && !root.unavailable ? 1.0 : 0.45
        enabled: root.controlEnabled && !root.unavailable
        activeFocusOnTab: true
        Accessible.role: Accessible.Switch
        Accessible.name: root.title
        Accessible.checked: root.checked
        Accessible.focusable: true
        Accessible.onPressAction: compactToggle.toggle()
        function toggle() {
            if (root.controlEnabled && !root.unavailable)
                root.toggleRequested(!root.checked);
        }
        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                compactToggle.toggle();
                event.accepted = true;
            }
        }
        TapHandler {
            onTapped: compactToggle.toggle()
        }
    }

    // ── Slider (Display / Sound 4×1) ─────────────────────────────────────
    Slider {
        id: sliderControl
        objectName: root.moduleId + "Slider"
        visible: root.isSlider && root.wide
        width: root.moduleId === "brightness" ? 217
             : root.moduleId === "volume" ? 180
             : Math.max(96, parent.width - 190)
        from: 0.0
        to: 1.0
        enabled: root.controlEnabled && !root.unavailable
        accessibleName: root.title
        anchors.right: parent.right
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        onMoved: (value) => root.sliderMoved(value)
    }

    Binding {
        target: sliderControl
        property: "value"
        value: tile.value !== undefined ? Number(tile.value) : 0.0
    }

    // ── Now Playing controls (2×2) ───────────────────────────────────────
    Row {
        id: mediaRow
        visible: root.isMedia && root.tall
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        anchors.bottomMargin: 12
        spacing: 8

        Repeater {
            model: [
                { id: "previous", glyph: "chevron-left", label: qsTr("Previous") },
                { id: "play", glyph: "play", label: qsTr("Play") },
                { id: "next", glyph: "chevron-right", label: qsTr("Next") }
            ]
            delegate: Rectangle {
                required property var modelData
                width: 30
                height: 30
                radius: 15
                color: root.controlFill
                opacity: root.unavailable ? 0.5 : 1.0
                Accessible.role: Accessible.Button
                Accessible.name: modelData.label
                TapHandler {
                    onTapped: root.mediaControlRequested(modelData.id)
                }
                Icon {
                    anchors.centerIn: parent
                    name: modelData.glyph
                    size: 14
                    color: root.textPrimary
                }
            }
        }
    }

    // The action chevron marks a module whose tap opens its sheet/action.
    Text {
        visible: root.isAction && !root.compact && !root.tall
        text: "\u203a"
        color: root.textSecondary
        font.pixelSize: Theme.primitive.font.sizeLg
        anchors.right: parent.right
        anchors.rightMargin: 10
        anchors.verticalCenter: parent.verticalCenter
    }
}
