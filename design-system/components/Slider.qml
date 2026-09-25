// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A continuous single-value slider (T-09.4). Drag the knob, tap the track, or
// use the arrow keys / Home / End; the value is a real in `[from, to]`.
//
// The owner stays in control: the slider only reports intent through
// `moved(value)` while the pointer or a key changes it and `committed(value)`
// when the gesture ends, and it never writes its own bound state back to the
// owner. Settings panes bind `value` to the settings key and write on `moved`
// (the T-09.1b live-apply pattern), so a user drag and an external daemon
// change converge.
//
// Optional `minLabel`/`midLabel`/`maxLabel` caption the ends and midpoint (`Small`
// … `Large`, `Off` … `Large`). AT-SPI exposes a Slider role with its range and
// current value.
Item {
    id: root

    property real value: 0
    property real from: 0
    property real to: 1
    property real stepSize: Theme.controls.slider.step
    property string minLabel: ""
    property string midLabel: ""
    property string maxLabel: ""
    property string accessibleName: ""
    property alias hovered: sliderHover.hovered

    signal moved(real value)
    signal committed(real value)

    readonly property real range: root.to - root.from
    readonly property real fraction: root.range > 0
        ? Math.max(0, Math.min(1, (root.value - root.from) / root.range))
        : 0
    readonly property bool hasCaptions: root.minLabel.length > 0
        || root.midLabel.length > 0 || root.maxLabel.length > 0

    implicitWidth: Theme.controls.slider.minWidth
    implicitHeight: Theme.controls.slider.height
                    + (root.hasCaptions ? captionRow.implicitHeight + Theme.controls.slider.captionGap : 0)
    activeFocusOnTab: true
    opacity: enabled ? 1.0 : 0.45

    function clamp(v) {
        return Math.max(root.from, Math.min(root.to, v));
    }

    function setValue(v) {
        var next = root.clamp(v);
        if (next === root.value)
            return;
        root.value = next;
        root.moved(next);
    }

    function commit() {
        root.committed(root.value);
    }

    function nudge(delta) {
        root.setValue(root.value + delta);
        root.commit();
    }

    Item {
        id: sliderBody
        width: parent.width
        height: Theme.controls.slider.height
        anchors.top: parent.top

        Rectangle {
            id: track
            width: parent.width
            height: Theme.controls.slider.trackHeight
            radius: height / 2
            color: Theme.color.controlFill
            anchors.verticalCenter: parent.verticalCenter
        }

        Rectangle {
            width: (track.width - Theme.controls.slider.knob) * root.fraction
                  + Theme.controls.slider.knob / 2
            height: track.height
            radius: track.radius
            color: Theme.color.accent
            anchors.left: track.left
            anchors.verticalCenter: track.verticalCenter
        }

        Rectangle {
            id: knob
            width: Theme.controls.slider.knob
            height: width
            radius: width / 2
            color: Theme.color.controlKnob
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            x: (track.width - width) * root.fraction
            anchors.verticalCenter: track.verticalCenter
            antialiasing: true
        }

        HoverHandler { id: sliderHover }

        TapHandler {
            onTapped: (eventPoint) => {
                root.setValue(root.from + (eventPoint.position.x / sliderBody.width) * root.range);
                root.commit();
            }
        }

        DragHandler {
            id: drag
            target: null
            property real base: 0

            onActiveChanged: {
                if (active)
                    drag.base = root.value;
                else
                    root.commit();
            }
            onTranslationChanged: {
                root.setValue(drag.base + (activeTranslation.x / sliderBody.width) * root.range);
            }
        }

        }

    FocusRing {
        target: sliderBody
        cornerRadius: Theme.controls.slider.height / 2
        shown: root.activeFocus
    }

    Keys.onPressed: (event) => {
        switch (event.key) {
        case Qt.Key_Left:
        case Qt.Key_Down:
            root.nudge(-root.stepSize);
            event.accepted = true;
            break;
        case Qt.Key_Right:
        case Qt.Key_Up:
            root.nudge(root.stepSize);
            event.accepted = true;
            break;
        case Qt.Key_Home:
            root.setValue(root.from);
            root.commit();
            event.accepted = true;
            break;
        case Qt.Key_End:
            root.setValue(root.to);
            root.commit();
            event.accepted = true;
            break;
        }
    }

    Item {
        id: captionRow
        visible: root.hasCaptions
        width: parent.width
        implicitHeight: captionFont.implicitHeight
        anchors.top: sliderBody.bottom
        anchors.topMargin: Theme.controls.slider.captionGap

        Text {
            id: captionFont
            visible: false
            text: " "
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        Text {
            anchors.left: parent.left
            text: root.minLabel
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.midLabel.length > 0
            text: root.midLabel
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
        Text {
            anchors.right: parent.right
            text: root.maxLabel
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
        }
    }

    Accessible.role: Accessible.Slider
    Accessible.name: root.accessibleName
    Accessible.focusable: true
    Accessible.onIncreaseAction: root.nudge(root.stepSize)
    Accessible.onDecreaseAction: root.nudge(-root.stepSize)
}