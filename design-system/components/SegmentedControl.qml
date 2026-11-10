// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A single-choice segmented selector. Each segment is a radio button to
// AT-SPI; arrow keys move the selection (roving focus), Home/End jump to the
// ends. The selection indicator slides with the segmented motion token and
// collapses under reduced motion.
Item {
    id: root

    property var model: []
    property int currentIndex: 0
    property alias hoveredIndex: hoverTracker.index
    // Optional measured geometry. Zero keeps the generic control's intrinsic
    // size; a consumer with a reference measurement (Files' 164×52 pt group
    // with 40×36 pt segments) sets these without a second component.
    property real groupWidth: 0
    property real groupHeight: 0
    property real segmentWidth: 0
    property real segmentHeight: 0
    property real paddingX: Theme.controls.segmentedControl.padding
    property real paddingY: Theme.controls.segmentedControl.padding

    signal activated(int index)
    // A disabled segment is a real state, not a dead glyph: consumers show
    // the reason (hover label or inline notice).
    signal rejected(int index, string reason)

    readonly property var entries: {
        var out = [];
        for (var i = 0; i < root.model.length; ++i) {
            var m = root.model[i];
            if (typeof m === "string")
                out.push({ label: m, icon: "", enabled: true, reason: "" });
            else
                out.push({ label: m.label || "", icon: m.icon || "",
                           enabled: m.enabled !== false,
                           reason: m.reason || "" });
        }
        return out;
    }
    readonly property Item currentSegment: segmentRepeater.count >= 0
                                           ? segmentRepeater.itemAt(root.currentIndex) : null

    implicitWidth: root.groupWidth > 0
                   ? root.groupWidth
                   : segmentRow.implicitWidth + 2 * root.paddingX
    implicitHeight: root.groupHeight > 0
                    ? root.groupHeight
                    : Theme.controls.segmentedControl.height

    function activateIndex(index) {
        var e = root.entries[index];
        if (!e)
            return;
        if (!e.enabled) {
            root.rejected(index, e.reason);
            return;
        }
        root.currentIndex = index;
        root.activated(index);
    }

    function move(delta, from) {
        var count = root.entries.length;
        if (count === 0)
            return;
        var i = from;
        for (var step = 0; step < count; ++step) {
            i = (i + delta + count) % count;
            if (root.entries[i].enabled) {
                root.currentIndex = i;
                var item = segmentRepeater.itemAt(i);
                if (item)
                    item.forceActiveFocus();
                return;
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.segmentedControl.radius
        color: Theme.color.controlFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    Rectangle {
        id: indicator
        y: root.paddingY
        height: parent.height - 2 * root.paddingY
        x: root.currentSegment ? root.currentSegment.x : root.paddingX
        width: root.currentSegment ? root.currentSegment.width : 0
        radius: Math.max(0, Theme.controls.segmentedControl.radius
                            - root.paddingY)
        color: Theme.color.surfaceElevated
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true

        Behavior on x {
            NumberAnimation {
                duration: Theme.motion.segmented.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.segmented.curve
            }
        }
        Behavior on width {
            NumberAnimation {
                duration: Theme.motion.segmented.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.segmented.curve
            }
        }
    }

    Row {
        id: segmentRow
        x: root.paddingX
        y: root.paddingY
        spacing: 0

        Repeater {
            id: segmentRepeater
            model: root.entries

            delegate: Segment { }
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: qsTr("Segmented control")

    component Segment: Item {
        id: segment

        required property var modelData
        required property int index

        readonly property bool current: root.currentIndex === segment.index
        readonly property bool hovered: segmentHover.hovered
        readonly property bool hasIcon: segment.modelData.icon.length > 0

        width: root.segmentWidth > 0
               ? root.segmentWidth
               : Math.max(Theme.controls.segmentedControl.segmentMinWidth,
                          (segment.hasIcon
                           ? Theme.controls.button.iconSize
                           : label.implicitWidth) + 2 * Theme.primitive.spacing.md)
        height: root.segmentHeight > 0
                ? root.segmentHeight
                : root.height - 2 * root.paddingY
        activeFocusOnTab: true
        opacity: segment.modelData.enabled ? 1.0 : 0.4

        function activate() {
            root.activateIndex(segment.index);
        }

        Text {
            id: label
            visible: !segment.hasIcon
            anchors.centerIn: parent
            text: segment.modelData.label
            color: segment.current ? Theme.color.textPrimary : Theme.color.textSecondary
            font.pixelSize: Theme.controls.segmentedControl.fontSize
            font.weight: segment.current ? Theme.controls.button.fontWeight
                                         : Theme.primitive.font.weightRegular
        }

        Icon {
            id: glyph
            visible: segment.hasIcon
            anchors.centerIn: parent
            name: segment.modelData.icon
            size: Theme.controls.button.iconSize
            color: segment.current ? Theme.color.textPrimary : Theme.color.textSecondary
        }

        HoverHandler {
            id: segmentHover
            onHoveredChanged: {
                if (hovered)
                    root.hoveredIndex = segment.index;
            }
        }

        TapHandler {
            onTapped: segment.activate()
        }

        FocusRing {
            target: segment
            cornerRadius: Theme.controls.segmentedControl.radius
            shown: segment.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Left || event.key === Qt.Key_Up) {
                root.move(-1, segment.index);
                event.accepted = true;
            } else if (event.key === Qt.Key_Right || event.key === Qt.Key_Down) {
                root.move(1, segment.index);
                event.accepted = true;
            } else if (event.key === Qt.Key_Home) {
                root.move(1, root.entries.length - 1);
                event.accepted = true;
            } else if (event.key === Qt.Key_End) {
                root.move(-1, 0);
                event.accepted = true;
            } else if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                       || event.key === Qt.Key_Enter) {
                segment.activate();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: segment.modelData.label
        Accessible.description: segment.modelData.reason
        Accessible.checkable: true
        Accessible.checked: segment.current
        Accessible.focusable: true
        Accessible.onPressAction: segment.activate()
    }

    // Tracks which segment is hovered for consumers that want a tooltip.
    QtObject {
        id: hoverTracker
        property int index: -1
    }
}
