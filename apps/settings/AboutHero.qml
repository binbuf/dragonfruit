// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Window

// Original vector artwork shared by the compact and full About surfaces.
Image {
    property real artworkSize: 194
    width: artworkSize
    height: artworkSize
    source: Qt.resolvedUrl("about-hero.svg")
    sourceSize.width: Math.ceil(width * Screen.devicePixelRatio)
    sourceSize.height: Math.ceil(height * Screen.devicePixelRatio)
    fillMode: Image.PreserveAspectFit
    smooth: true
}
