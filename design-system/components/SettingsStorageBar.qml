// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The storage summary bar and legend (T286). `segments` is a list of
// `{ label, value, color }`; the bar divides the track by the summed values.
// The pane supplies real usage from the storage sampler.
Column {
    id: root

    property var segments: []
    property string summaryText: ""
    property alias bar: bar
    property alias legend: legend

    width: parent ? parent.width : implicitWidth
    spacing: Theme.primitive.spacing.sm

    Text {
        id: summaryLabel
        width: parent.width
        visible: root.summaryText.length > 0
        text: root.summaryText
        color: Theme.color.textPrimary
        font.pixelSize: Theme.controls.button.fontSize
        elide: Text.ElideRight
        horizontalAlignment: root.LayoutMirroring.enabled
                             ? Text.AlignRight : Text.AlignLeft
    }

    Item {
        id: bar
        width: parent.width
        height: Theme.primitive.spacing.sm

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Theme.color.controlFill
            antialiasing: true
        }

        Row {
            anchors.fill: parent
            spacing: 0

            Repeater {
                model: root.segments

                delegate: Rectangle {
                    required property var modelData

                    readonly property real total: {
                        var sum = 0;
                        for (var i = 0; i < root.segments.length; ++i)
                            sum += Math.max(0, Number(root.segments[i].value) || 0);
                        return sum;
                    }
                    readonly property real value:
                        Math.max(0, Number(modelData.value) || 0)

                    width: total > 0 ? bar.width * value / total : 0
                    height: bar.height
                    radius: height / 2
                    color: modelData.color !== undefined
                           ? modelData.color : Theme.color.accent
                    antialiasing: true
                }
            }
        }
    }

    Flow {
        id: legend
        width: parent.width
        spacing: Theme.primitive.spacing.md

        Repeater {
            model: root.segments

            delegate: Row {
                required property var modelData
                spacing: Theme.primitive.spacing.xs

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.primitive.spacing.sm
                    height: width
                    radius: width / 2
                    color: modelData.color !== undefined
                           ? modelData.color : Theme.color.accent
                    antialiasing: true
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData.label !== undefined
                          ? String(modelData.label) : ""
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.primitive.font.sizeSm
                }
            }
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.summaryText
}
