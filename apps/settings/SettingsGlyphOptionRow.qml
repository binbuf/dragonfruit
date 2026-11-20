// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// One OS menu glyph choice (T293a): the mockup's radio row with an
// actual-size menu-bar strip preview on the right.
//
// "Actual size" is literal. The strip is `Theme.controls.menuBar.height` tall
// and carries a real `MenuBarMenu` logo slot (`Theme.controls.menuBarMenu`
// bar height, bar padding, and `Theme.controls.menuBar.iconSize` glyph). The
// glyph keeps the same wallpaper-contrast shadow the live bar draws, and the
// slot's plate is the bar's one hover/open ramp, so the preview reads as the
// bar's true left edge rather than a scaled-up icon tile.
//
// Clicking anywhere on the row selects it. Space/Return selects it from the
// keyboard; the row is one AT-SPI radio button, and the group owner
// (`SettingsRadioGroup`) announces the group. The row is not a scroll-aware
// widget: the shell's detail view owns scrolling, exactly like `SettingsRow`.
Item {
    id: root

    property string label: ""
    property string description: ""
    // The `SystemMenuGlyph` artwork name ("dragonfruit" or "tux").
    property string glyph: "dragonfruit"
    // Whether this option is the group's current value.
    property bool selected: false
    // The keyboard shows the bar's open-menu plate on this row. Hover always
    // does; this only adds the keyboard's own affordance.
    property bool openSample: false
    // The optional "File Edit View" sample beside the glyph. It shows the
    // scale; it never implies a second menu bar. The pane keeps it off: the
    // strip's width is the bar's scale, and sample words would read as a
    // second menu bar at 184 pt.
    property bool sampleMenus: false
    // The settingsd key the shared write-error row reports.
    property string settingsKey: ""
    // The group's keyboard adapter. A SettingsRow (and so this row) is not a
    // FocusScope, so a `Keys` handler on it does not see the key events; the
    // adapter is what actually owns them (the same RolesLayout the shared
    // pane vocabulary uses).
    property Item keyTarget: null
    property bool showSeparator: true
    property bool highlighted: false

    signal chosen()

    // The mark takes the lettering's opposite-tone shadow, exactly like the
    // live bar: the preview stands in for the bar over a wallpaper.
    readonly property bool lightLettering: {
        var c = Theme.color.textPrimary;
        return (0.299 * c.r + 0.587 * c.g + 0.114 * c.b) > 0.5;
    }
    readonly property color glyphShadowColor: {
        var base = root.lightLettering
                ? Theme.controls.wallpaperText.shadowColorOnDark
                : Theme.controls.wallpaperText.shadowColorOnLight;
        var opacity = root.lightLettering
                ? Theme.controls.wallpaperText.shadowOpacityOnDark
                : Theme.controls.wallpaperText.shadowOpacityOnLight;
        return Qt.rgba(base.r, base.g, base.b, opacity * 0.9);
    }
    // At least 180 pt so the glyph reads at its true size beside the sample.
    readonly property real stripWidth: 184

    implicitHeight: Math.max(Theme.controls.settingsRow.height,
                             previewStrip.height + 2 * Theme.primitive.spacing.sm)
    implicitWidth: 480
    width: parent ? parent.width : implicitWidth
    activeFocusOnTab: true

    function choose() {
        root.chosen();
    }

    HoverHandler { id: rowHover }

    // The deep-link landing wash, the same treatment `SettingsRow` paints.
    Rectangle {
        id: highlightRect
        anchors.fill: parent
        radius: Theme.controls.settingsGroup.radius
        color: Theme.color.accentMuted
        opacity: root.highlighted ? 1.0 : 0.0
        visible: opacity > 0
        antialiasing: true
    }

    Rectangle {
        id: radio
        x: Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter
        width: Theme.controls.settingsRadio.size
        height: width
        radius: width / 2
        color: "transparent"
        border.width: Theme.controls.window.borderWidth
        border.color: root.selected ? Theme.color.accent
                                    : Theme.color.textTertiary
        antialiasing: true

        Rectangle {
            anchors.centerIn: parent
            width: Theme.controls.settingsRadio.dot
            height: width
            radius: width / 2
            color: Theme.color.accent
            visible: root.selected
            antialiasing: true
        }
    }

    Column {
        id: labelColumn
        anchors.left: radio.right
        anchors.leftMargin: Theme.primitive.spacing.md
        anchors.right: previewStrip.left
        anchors.rightMargin: Theme.primitive.spacing.md
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xxs

        Text {
            width: parent.width
            text: root.label
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            elide: Text.ElideRight
        }

        Text {
            width: parent.width
            visible: root.description.length > 0
            text: root.description
            color: Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
        }
    }

    // The actual-size strip: the bar's height, the bar's edge radius, and a
    // real logo slot. The plate is the bar's hover ramp; the open sample is
    // the bar's open ramp, so the strip documents both states.
    Rectangle {
        id: previewStrip
        objectName: "settingsGlyphPreview-" + root.glyph
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter
        width: root.stripWidth
        height: Theme.controls.menuBar.height
        radius: Math.min(Theme.controls.menuBar.radius, height / 2)
        // The bar's own fallback fill: an opaque plate, so the glyph's
        // wallpaper-contrast shadow reads the way it does on the bar.
        color: Theme.color.chromeFallbackFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        clip: true
        antialiasing: true

        readonly property bool showPlate: rowHover.hovered || root.openSample
                                          || root.selected

        // The preview is actual size. A narrow pane may still be narrower than
        // the strip; the slot and glyph keep their real size and the strip is
        // clipped at its right edge, never scaled.
        readonly property real slotWidth: Theme.controls.menuBar.iconSize
                                          + 2 * Theme.controls.menuBarMenu.barPaddingH
        readonly property real markX: Theme.controls.menuBar.paddingH
                                      + Theme.controls.menuBarMenu.barPaddingH

        Rectangle {
            id: logoSlot
            objectName: "settingsGlyphPlate-" + root.glyph
            anchors.left: parent.left
            anchors.leftMargin: Theme.controls.menuBar.paddingH
            anchors.verticalCenter: parent.verticalCenter
            width: previewStrip.slotWidth
            height: Theme.controls.menuBarMenu.barHeight
            radius: Math.min(Theme.controls.focusRing.radius, height / 2)
            color: Qt.rgba(Theme.color.textPrimary.r, Theme.color.textPrimary.g,
                           Theme.color.textPrimary.b,
                           root.openSample ? Theme.controls.menuBar.openOpacity
                                           : Theme.controls.menuBar.hoverOpacity)
            visible: previewStrip.showPlate
            antialiasing: true
        }

        SystemMenuGlyph {
            id: previewGlyph
            objectName: "settingsGlyphMark-" + root.glyph
            // The real logo slot: left padding, then the bar item's own padding.
            x: previewStrip.markX
            anchors.verticalCenter: parent.verticalCenter
            glyph: root.glyph
            size: Theme.controls.menuBar.iconSize
            color: Theme.color.textPrimary
            shadowColor: root.glyphShadowColor
            shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)
        }

        Row {
            id: sampleRow
            visible: root.sampleMenus
            anchors.left: logoSlot.right
            anchors.leftMargin: Theme.controls.menuBar.labelGap
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.controls.menuBar.clockGap

            Repeater {
                model: [qsTr("File"), qsTr("Edit"), qsTr("View")]

                Text {
                    required property string modelData
                    text: modelData
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.menuBar.fontSize
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }

    Rectangle {
        visible: root.showSeparator
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.settingsRow.paddingH
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.bottom: parent.bottom
        height: Theme.controls.window.borderWidth
        color: Theme.color.separator
    }

    // The shared write-error line, when the pane gives this row a key.
    Text {
        id: errorText
        objectName: "settingsGlyphError-" + root.glyph
        anchors.left: labelColumn.left
        anchors.right: labelColumn.right
        anchors.top: labelColumn.bottom
        anchors.topMargin: Theme.primitive.spacing.xxs
        visible: root.settingsKey.length > 0
                 && String(Settings.writeErrors[root.settingsKey] || "").length > 0
        text: visible ? String(Settings.writeErrors[root.settingsKey]) : ""
        color: Theme.color.danger
        font.pixelSize: Theme.primitive.font.sizeSm
        wrapMode: Text.WordWrap
    }

    // "Clicking anywhere on the row selects." A tap (not a drag) is the
    // gesture, so the row stays inside the shell's detail ScrollView and the
    // focus follows the pointer pick.
    TapHandler {
        id: press
        onTapped: root.choose()
    }

    // The shell's detail view is one ScrollView, so a stationary press can be
    // claimed by the scroll owner. The row therefore also listens for the
    // pointer button on its own: a press that does not start a scroll selects
    // on release, and the focus always follows the press.
    MouseArea {
        id: rowMouse
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        onPressed: (mouse) => root.forceActiveFocus()
        onClicked: root.choose()
    }

    FocusRing {
        target: root
        cornerRadius: Theme.primitive.radius.sm
        shown: root.activeFocus
    }

    // Forward a key to the group adapter, which owns the semantics.
    function handleKey(key) {
        if (root.keyTarget === null)
            return false;
        switch (key) {
        case Qt.Key_Up:
        case Qt.Key_Down:
        case Qt.Key_Left:
        case Qt.Key_Right:
        case Qt.Key_Space:
        case Qt.Key_Return:
        case Qt.Key_Enter:
            return root.keyTarget.handleKey(key);
        }
        return false;
    }

    // The radio keys (Space/Return select the focused row; the arrows move the
    // selection) belong to the group adapter. This row is the focused item,
    // so it forwards them: a `Keys` handler on the item that holds focus is
    // what receives the event (the row is not a FocusScope).
    Accessible.role: Accessible.RadioButton
    Accessible.name: root.label
    Accessible.description: root.description
    Accessible.checkable: true
    Accessible.checked: root.selected
    Accessible.focusable: true
    Accessible.onPressAction: root.choose()
}
