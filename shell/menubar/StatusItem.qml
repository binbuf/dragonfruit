// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One menu-bar status-item slot (T-09 FR-5, T231 registry).
//
// Every system and third-party extra renders through this slot, so sizing,
// hover, tooltip, keyboard focus and dark/light treatment are identical
// everywhere. The registry (`StatusItemRegistry.qml`) owns order, visibility
// and overflow; this item owns only one slot's presentation.
//
// Graceful degradation (FR-4): `available: false` hides the item entirely
// when its backing daemon is absent; `enabled: false` keeps it visible but
// dimmed when the adapter is present but the feature is off. `state` carries
// the reason for introspection and the accessible description (`ok`,
// `unavailable`, `error`, `attention`).
Item {
    id: root

    property string itemId: ""
    property string icon: ""
    // A file-backed icon (a StatusNotifier tray item's themed icon); when set
    // it is rendered instead of the named `icon` glyph, so third-party items
    // get the same slot treatment as first-party ones (T-14.3).
    property url iconSource: ""
    property string label: ""
    // The registry's tooltip text; also the accessible description.
    property string tooltip: ""
    property string accessibleName: ""
    // "status" | "action" | "clock" | "overflow".
    property string kind: "status"
    // "ok" | "unavailable" | "error" | "attention".
    property string state: "ok"
    property bool available: true
    property bool selected: false
    // Keyboard selection (T-07.5b): the bar's arrow keys move this ring.
    property bool keyboardFocus: false
    property real level: 0.8
    property color tint: Theme.color.textPrimary
    property color backgroundColor: Theme.color.chrome
    // The wallpaper text treatment follows the actual lettering: white text
    // takes the dark shadow pair, black text the light one.
    readonly property color labelColor:
        root.selected ? Theme.color.accent : root.tint
    readonly property bool lightLettering: {
        var c = root.labelColor;
        return (0.299 * c.r + 0.587 * c.g + 0.114 * c.b) > 0.5;
    }
    readonly property color labelShadowColor:
        root.lightLettering ? Theme.controls.wallpaperText.shadowColorOnDark
                            : Theme.controls.wallpaperText.shadowColorOnLight
    readonly property real labelShadowOpacity:
        root.lightLettering ? Theme.controls.wallpaperText.shadowOpacityOnDark
                            : Theme.controls.wallpaperText.shadowOpacityOnLight
    // The status marks share the label's opposite-tone shadow, slightly
    // lighter because a stroked glyph covers less area than a glyph fill.
    readonly property color glyphShadowColor:
        Qt.rgba(root.labelShadowColor.r, root.labelShadowColor.g,
                root.labelShadowColor.b, root.labelShadowOpacity * 0.9)

    // T231: the clock is a first-class registry item. `StatusItem` owns its
    // slot (hover/focus/tap/accessibility) and embeds the formatting-only
    // `MenuBarClock`, so the clock sorts and overflows with every other item.
    property bool showDate: false
    property bool showSeconds: false
    property bool use24Hour: false
    property color foreground: Theme.color.textPrimary
    property color secondaryForeground: Theme.color.textSecondary

    readonly property bool isClock: root.kind === "clock"
    readonly property string clockText:
        clockLoader.item ? clockLoader.item.timeText : ""
    readonly property string clockDateText:
        clockLoader.item ? clockLoader.item.dateText : ""
    readonly property string clockAccessibleText:
        clockLoader.item ? clockLoader.item.accessibleText : ""

    signal activated(string itemId)

    visible: available
    enabled: available
    implicitWidth: available
        ? (root.isClock
           ? (clockLoader.item ? clockLoader.item.implicitWidth : 0)
           : slotContent.implicitWidth + 2 * Theme.controls.menuBar.statusItemPaddingH)
        : 0
    implicitHeight: Theme.controls.menuBar.height
    opacity: (enabled && root.state !== "error") ? 1.0 : 0.4

    Rectangle {
        id: slotBackground
        anchors.centerIn: parent
        width: root.width
        height: Theme.controls.menuBar.height - 2 * Theme.primitive.spacing.xs
        radius: Theme.controls.menuBar.hoverRadius
        color: (root.selected || root.keyboardFocus) ? Theme.color.accentMuted
                             : (slotHover.hovered ? Theme.color.controlHover : "transparent")
        antialiasing: true
    }

    FocusRing {
        target: slotBackground
        shown: root.keyboardFocus
        cornerRadius: Theme.controls.menuBar.hoverRadius
    }

    Row {
        id: slotContent
        visible: !root.isClock
        anchors.centerIn: parent
        spacing: root.label.length > 0 ? Theme.controls.menuBar.labelGap : 0

        StatusGlyph {
            id: glyph
            visible: root.icon.length > 0 && root.iconSource.toString().length === 0
            name: root.icon
            color: root.selected ? Theme.color.accent : root.tint
            shadowColor: (root.selected || root.keyboardFocus)
                         ? "transparent" : root.glyphShadowColor
            shadowOffset: Qt.point(0, Theme.controls.wallpaperText.shadowOffsetY)
            backgroundColor: root.backgroundColor
            size: Theme.controls.menuBar.iconSize
            level: root.level
            anchors.verticalCenter: parent.verticalCenter
        }

        Image {
            id: iconImage
            visible: root.iconSource.toString().length > 0
            source: root.iconSource
            sourceSize.width: Theme.controls.menuBar.iconSize
            sourceSize.height: Theme.controls.menuBar.iconSize
            fillMode: Image.PreserveAspectFit
            smooth: true
            anchors.verticalCenter: parent.verticalCenter
        }

        WallpaperLabel {
            id: labelText
            visible: root.label.length > 0
            text: root.label
            color: root.labelColor
            shadowEnabled: !root.selected && !root.keyboardFocus
            shadowColor: root.labelShadowColor
            shadowOpacity: root.labelShadowOpacity
            font.pixelSize: Theme.controls.menuBar.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    Loader {
        id: clockLoader
        anchors.centerIn: parent
        active: root.isClock
        sourceComponent: Component {
            MenuBarClock {
                objectName: "clock"
                interactive: false
                showDate: root.showDate
                showSeconds: root.showSeconds
                use24Hour: root.use24Hour
                foreground: root.foreground
                secondaryForeground: root.secondaryForeground
                shadowEnabled: !root.selected && !root.keyboardFocus
            }
        }
    }

    HoverHandler {
        id: slotHover
        onHoveredChanged: {
            if (hovered && root.tooltip.length > 0) {
                tooltipTimer.restart();
            } else {
                tooltipTimer.stop();
                tooltipItem.open = false;
            }
        }
    }

    // The owner owns the dwell timer (ADR 0093). A status tooltip is passive:
    // it never takes focus and never blocks the slot beneath it.
    Timer {
        id: tooltipTimer
        interval: Theme.controls.tooltip.dwell
        repeat: false
        onTriggered: {
            if (slotHover.hovered && root.tooltip.length > 0)
                tooltipItem.open = true;
        }
    }

    Tooltip {
        id: tooltipItem
        open: false
        text: root.tooltip
        anchorItem: root
        placement: "below"
        bounds: null
    }

    TapHandler {
        onTapped: root.activated(root.itemId)
    }

    Accessible.role: Accessible.Button
    Accessible.name: {
        if (root.isClock && root.clockAccessibleText.length > 0)
            return root.clockAccessibleText;
        return root.accessibleName.length > 0 ? root.accessibleName
                                              : (root.label.length > 0 ? root.label : root.itemId);
    }
    Accessible.description: root.tooltip
    Accessible.focusable: true
    Accessible.onPressAction: root.activated(root.itemId)
}
