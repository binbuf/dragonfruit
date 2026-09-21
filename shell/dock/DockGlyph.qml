// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// Dock entry artwork (T-10). Two shapes:
//   * an app tile — a rounded, deterministically coloured square carrying the
//     application's initial. Real themed icons arrive with app-index (T-23);
//     until then this is an original placeholder, never a bitmap asset.
//   * the Trash — our own geometry (lid, handle, bin) with an empty/full
//     state. No Apple artwork is copied (14-risks.md).
Item {
    id: root

    // "app" | "trash"
    property string kind: "app"
    property string name: ""
    property string appId: ""
    property bool trashFull: false
    property real size: Theme.controls.dock.iconSize

    implicitWidth: size
    implicitHeight: size

    readonly property color tileColor: {
        var palette = [
            Theme.primitive.color.magenta500,
            Theme.primitive.color.violet500,
            Theme.primitive.color.jade500,
            Theme.primitive.color.gold500,
            Theme.primitive.color.coral500,
            Theme.primitive.color.sky500
        ];
        var key = appId.length > 0 ? appId : name;
        var hash = 0;
        for (var i = 0; i < key.length; ++i)
            hash = (hash * 31 + key.charCodeAt(i)) & 0x7fffffff;
        return palette[hash % palette.length];
    }

    readonly property string initial:
        name.length > 0 ? name.charAt(0).toUpperCase() : "?"

    // -- App tile --------------------------------------------------------
    Rectangle {
        visible: root.kind === "app"
        anchors.fill: parent
        radius: root.size * 0.24
        color: root.tileColor
        border.width: 1
        border.color: Qt.rgba(0, 0, 0, 0.18)

        Text {
            anchors.centerIn: parent
            text: root.initial
            color: Theme.color.accentContent
            font.pixelSize: Math.round(root.size * 0.44)
            font.weight: Theme.primitive.font.weightSemibold
        }
    }

    // -- Downloads stack -------------------------------------------------
    // A folder glyph for the Downloads stack (T-10 section 17): a tab plus a
    // body, drawn from our own geometry.
    Item {
        id: stack
        visible: root.kind === "stack"
        anchors.fill: parent

        readonly property real s: root.size

        Rectangle {
            width: stack.s * 0.42
            height: stack.s * 0.16
            radius: stack.s * 0.04
            color: Theme.primitive.color.sky500
            x: stack.s * 0.14
            y: stack.s * 0.22
        }
        Rectangle {
            width: stack.s * 0.72
            height: stack.s * 0.46
            radius: stack.s * 0.09
            color: Theme.primitive.color.sky500
            border.width: 1
            border.color: Qt.rgba(0, 0, 0, 0.18)
            x: stack.s * 0.14
            y: stack.s * 0.32
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            y: stack.s * 0.66
            text: root.name.length > 0 ? root.name : qsTr("Downloads")
            color: Theme.color.textSecondary
            font.pixelSize: Math.round(stack.s * 0.16)
            elide: Text.ElideRight
            width: stack.s * 0.8
            horizontalAlignment: Text.AlignHCenter
        }
    }

    // -- Trash -----------------------------------------------------------
    Item {
        id: trash
        visible: root.kind === "trash"
        anchors.fill: parent
        readonly property real s: root.size

        // Handle.
        Rectangle {
            width: trash.s * 0.16
            height: trash.s * 0.06
            radius: height / 2
            color: Theme.color.textSecondary
            anchors.horizontalCenter: parent.horizontalCenter
            y: trash.s * 0.16
        }

        // Lid.
        Rectangle {
            width: trash.s * 0.62
            height: trash.s * 0.07
            radius: height / 2
            color: Theme.color.textSecondary
            anchors.horizontalCenter: parent.horizontalCenter
            y: trash.s * 0.24
        }

        // Bin body.
        Rectangle {
            id: body
            width: trash.s * 0.52
            height: trash.s * 0.52
            radius: trash.s * 0.08
            color: "transparent"
            border.width: Math.max(1.5, trash.s * 0.05)
            border.color: root.trashFull ? Theme.color.accent : Theme.color.textSecondary
            anchors.horizontalCenter: parent.horizontalCenter
            y: trash.s * 0.34

            // Two vertical ribs.
            Repeater {
                model: 2
                delegate: Rectangle {
                    required property int index
                    width: Math.max(1, body.width * 0.07)
                    height: body.height * 0.5
                    radius: width / 2
                    color: root.trashFull ? Theme.color.accent : Theme.color.textSecondary
                    x: body.width * (index === 0 ? 0.32 : 0.61)
                    y: body.height * 0.25
                }
            }
        }
    }
}
