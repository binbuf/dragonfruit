// SPDX-License-Identifier: LGPL-3.0-or-later
pragma Singleton
import QtQuick

// Placeholder token layer (T-08 replaces this with the real primitive /
// semantic / component token architecture). Every first-party surface
// consumes these tokens — shell chrome and apps never hardcode values.
QtObject {
    readonly property real radius: 10
    readonly property real spacing: 8
    readonly property color accent: "#a8326d"       // dragonfruit magenta
    readonly property color surface: "#211a26"
    readonly property color onSurface: "#efe9f2"
}
