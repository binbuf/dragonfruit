// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A floating surface used for menus, popovers, and control-center panels.
// Open/close is progress-based and interruptible: the animation can be
// retargeted mid-flight and the reduced-motion variant removes the scale
// while keeping the show/hide legible.
Item {
    id: root

    property bool open: false
    property Item anchorItem: null
    property int preferredWidth: Theme.controls.popup.minWidth
    property int horizontalPadding: Theme.controls.popup.padding
    property int verticalPadding: Theme.controls.popup.padding
    property real anchorOffset: Theme.controls.popup.offset
    property int accessibleRole: Accessible.Pane
    property string accessibleName: ""
    // T202: the shared menu-family preset and whether the compositor has a
    // trusted sampled backdrop behind this surface. App-owned surfaces keep
    // `local` and paint the deterministic opaque plate.
    property string materialPreset: "menuDetached"
    property string materialBackdrop: "local"
    // Attached menus have square top corners and rounded bottom corners;
    // detached/status menus are all-rounded. `cornerRadius` is the bottom
    // radius, `cornerTopRadius` the top pair.
    readonly property real cornerRadius: Material.menuRadius(materialPreset)
    readonly property real cornerTopRadius:
        materialPreset === "menuAttached" ? 0 : cornerRadius
    readonly property bool trustedBackdrop: materialBackdrop === "trusted"
    readonly property var glassResolution:
        Material.menuResolution(materialPreset, materialBlurActive, materialTier,
                                materialFallback, trustedBackdrop)
    readonly property color surfaceFill:
        Material.menuFill(materialPreset, materialBlurActive, materialTier,
                          materialFallback, trustedBackdrop)
    // `menu` uses the measured menu motion; `popup` keeps the generic popup
    // timing for non-menu surfaces (e.g. the Appearance color picker).
    property string motionPreset: "popup"
    readonly property var openMotion:
        motionPreset === "menu" ? Theme.motion.menuOpen : Theme.motion.popupOpen
    readonly property var closeMotion:
        motionPreset === "menu" ? Theme.motion.menuClose : Theme.motion.popupClose
    // When false the owner handles Escape itself (e.g. a menu with an open
    // submenu closes the submenu first) and the popup stays open.
    property bool escapeCloses: true
    // T-21.2c: the shell applies the compositor's Popup role state. When the
    // sampled pass is absent the surface uses the token-driven milky plate;
    // `materialFallback` records why (0=none, 1=no-sampled-blur, 2=software,
    // 3=multi-gpu, 4=floor-missed). QML never re-derives the degrade policy.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    default property alias contentData: contentColumn.data
    property alias content: contentColumn

    signal opened()
    signal closed()
    signal escapePressed()

    function show() { root.open = true; }
    function hide() { root.open = false; }
    function toggle() { root.open = !root.open; }

    focus: open
    activeFocusOnTab: open
    visible: opacity > 0
    z: 1000

    readonly property point anchorPoint: {
        if (!anchorItem || !root.parent)
            return Qt.point((root.parent ? root.parent.width : 0) / 2 - width / 2,
                            (root.parent ? root.parent.height : 0) / 2 - height / 2);
        var p = anchorItem.mapToItem(root.parent, 0, anchorItem.height);
        return Qt.point(p.x, p.y + root.anchorOffset);
    }
    // The anchor point in popup-local coordinates: the surface shrinks toward
    // this point on close (T202).
    readonly property point anchorLocal: {
        if (!anchorItem)
            return Qt.point(width / 2, 0);
        var p = anchorItem.mapToItem(root, anchorItem.width / 2, anchorItem.height);
        return Qt.point(Math.max(0, Math.min(width, p.x)),
                        Math.max(0, Math.min(height, p.y)));
    }
    readonly property real closeScale:
        motionPreset === "menu" ? Material.menuCloseScale(materialPreset) : 0.97
    readonly property real exitTranslateX: (1 - scale) * anchorLocal.x
    readonly property real exitTranslateY: (1 - scale) * anchorLocal.y

    width: Math.max(preferredWidth, contentColumn.implicitWidth + 2 * horizontalPadding)
    height: contentColumn.implicitHeight + 2 * verticalPadding
    x: anchorPoint.x
    y: anchorPoint.y
    scale: open ? 1.0 : closeScale
    transformOrigin: Item.TopLeft
    transform: Translate {
        x: root.open ? 0 : root.exitTranslateX
        y: root.open ? 0 : root.exitTranslateY
    }
    opacity: open ? 1.0 : 0.0

    onOpenChanged: {
        if (open) {
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    Behavior on opacity {
        enabled: !Theme.reducedMotion
        NumberAnimation {
            duration: root.open ? root.openMotion.duration : root.closeMotion.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? root.openMotion.curve : root.closeMotion.curve
        }
    }
    Behavior on scale {
        enabled: !Theme.reducedMotion
        NumberAnimation {
            duration: root.open ? root.openMotion.duration : root.closeMotion.duration
            easing.type: Easing.Bezier
            easing.bezierCurve: root.open ? root.openMotion.curve : root.closeMotion.curve
        }
    }

    Shadow {
        width: root.width
        height: root.height
        radius: root.cornerRadius
        blur: Theme.controls.popup.shadowBlur
    }

    Rectangle {
        objectName: "popupSurface"
        anchors.fill: parent
        radius: root.cornerRadius
        // The panel is the compositor material's scrim: the fill carries the
        // resolved preset alpha/tint so the sampled backdrop blur and
        // liquid-glass pass show through; the rim/hairline stays opaque.
        // Without the sampled pass it switches to the milky complement.
        color: root.surfaceFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border
        antialiasing: true
    }

    // Square the attached menu's top corners while keeping the bottom pair
    // rounded. The compositor region carries the same per-corner geometry.
    Rectangle {
        visible: root.cornerTopRadius < root.cornerRadius
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.cornerRadius
        color: root.surfaceFill
        antialiasing: true
    }
    Rectangle {
        visible: root.cornerTopRadius < root.cornerRadius
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.controls.window.borderWidth
        color: Theme.color.border
    }

    Column {
        id: contentColumn
        x: root.horizontalPadding
        y: root.verticalPadding
        width: root.width - 2 * root.horizontalPadding
    }

    Keys.onEscapePressed: (event) => {
        root.escapePressed();
        if (root.escapeCloses)
            root.hide();
        event.accepted = true;
    }

    Accessible.role: root.accessibleRole
    Accessible.name: root.accessibleName
}
