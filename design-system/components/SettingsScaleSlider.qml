// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The Settings scale slider (T286): a 246 pt slider with five tick marks and
// Slow/Fast end labels. It wraps the shared `Slider`; the pane supplies the
// value, range, and step.
Column {
    id: root

    property real value: 0
    property real from: 0
    property real to: 1
    property real stepSize: Theme.controls.slider.step
    property string minLabel: qsTr("Slow")
    property string maxLabel: qsTr("Fast")
    property string accessibleName: ""
    property alias slider: slider
    property alias tickRow: tickRow

    signal moved(real value)
    signal committed(real value)

    width: Theme.controls.settingsScale.width
    spacing: Theme.controls.settingsScale.tickGap

    Slider {
        id: slider
        width: root.width
        value: root.value
        from: root.from
        to: root.to
        stepSize: root.stepSize
        minLabel: root.minLabel
        maxLabel: root.maxLabel
        accessibleName: root.accessibleName
        onMoved: (value) => {
            root.value = value;
            root.moved(value);
        }
        onCommitted: (value) => {
            root.value = value;
            root.committed(value);
        }
    }

    Item {
        id: tickRow
        width: slider.width
        height: Theme.controls.settingsScale.tickHeight

        Repeater {
            model: 5

            delegate: Rectangle {
                required property int index

                width: Theme.controls.window.borderWidth
                height: Theme.controls.settingsScale.tickHeight
                x: Math.round((tickRow.width - width) * index / 4)
                color: Glass.increaseContrast ? Theme.color.textPrimary
                                              : Theme.color.separator
            }
        }
    }
}
