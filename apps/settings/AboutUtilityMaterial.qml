// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// Utility-window vibrancy role. The gradient and shadow are separate from
// popup glass; accessibility overrides make the backing fully opaque.
Item {
    id: root
    objectName: "aboutUtilityMaterial"
    readonly property bool opaque: Settings.values["accessibility.reduceTransparency"] === true
    readonly property bool contrast: Settings.values["accessibility.increaseContrast"] === true
    readonly property color topColor: Theme.dark ? (opaque ? "#30343d" : "#f030343d")
                                                  : (opaque ? "#f8f9fb" : "#f8f8f9fb")
    readonly property color bottomColor: Theme.dark ? (opaque ? "#20242c" : "#f020242c")
                                                     : (opaque ? "#eef0f4" : "#f8eef0f4")
    Shadow {
        anchors.fill: parent
        level: "high"
        radius: 13
    }
    Rectangle {
        anchors.fill: parent
        radius: 13
        gradient: Gradient {
            GradientStop { position: 0; color: root.topColor }
            GradientStop { position: 1; color: root.bottomColor }
        }
        border.width: root.contrast ? 2 : 1
        border.color: Theme.color.border
    }
}
