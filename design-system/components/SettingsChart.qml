// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// The Settings chart frame (T286): the battery pane's axes and bars. `values`
// is a list of 0..1 samples in display order; the frame draws the baseline,
// a mid grid line, and one bar per sample. It never invents data.
Item {
    id: root

    property var values: []
    property color barColor: Theme.color.accent
    property alias bars: bars
    property alias baselineRect: baseline

    implicitWidth: 320
    implicitHeight: Theme.controls.settingsChart.height

    Rectangle {
        id: baseline
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.controls.window.borderWidth
        color: Glass.increaseContrast ? Theme.color.textPrimary
                                      : Theme.color.separator
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
        opacity: 0.6
    }

    Row {
        id: bars
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: baseline.top
        spacing: Theme.controls.settingsChart.barGap

        Repeater {
            model: root.values

            delegate: Item {
                required property var modelData

                readonly property real fraction:
                    Math.max(0, Math.min(1, Number(modelData) || 0))

                width: bars.width / Math.max(1, root.values.length)
                       - Theme.controls.settingsChart.barGap
                height: bars.height

                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: Math.max(Theme.controls.window.borderWidth,
                                     parent.height * parent.fraction)
                    radius: Theme.primitive.radius.xs
                    color: root.barColor
                    opacity: 0.85
                    antialiasing: true
                }
            }
        }
    }

    Accessible.role: Accessible.Graphic
    Accessible.name: qsTr("Battery level over time")
}
