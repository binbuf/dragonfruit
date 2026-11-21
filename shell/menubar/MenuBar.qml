// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The top menu bar (T-09): anchored chrome surface with reserved zones. From
// left to right it hosts the always-present system menu (the dragonfruit mark,
// T-09), the always-present application menu (the focused app's name and its
// About/Settings/Hide/Quit items; Files on the empty desktop), the focused
// application's own menus (menu-broker, T-22), system status items (Wi-Fi,
// Bluetooth, volume, battery, clock, Focus/DND, accessibility), the Control
// Center entry point, and an Overview button/gesture target.
//
// The shell process owns the Wayland surface and reserved zone; this
// component is the render/interaction core. Data is injected (the shell
// controller maps compositor broadcasts and T-20 adapter events onto these
// properties), so the whole bar is testable headless.
Rectangle {
    id: menuBar

    // --- Data (injected by the shell controller) --------------------------
    property string appName: ""
    property string appId: ""
    // The fixed system menu (brand mark): About/Settings/App Store/Sleep/…
    // [{ label, shortcut, type, enabled, action }]
    property var systemMenuItems: []
    // The fixed application menu: About <App>/Settings/Hide/Quit, synthesized
    // from the focused app (or Files on the desktop) until T-22 owns it.
    property var applicationMenuItems: []
    // The focused application's exported top-level menus.
    // [{ title: "File", items: [ {label, shortcut, type, enabled, action} ] }]
    property var appMenuModel: []
    // T-14.2b: the global application-menu toggle (`menu.global`). When off,
    // the focused app's exported menus are not shown globally so first-party
    // apps keep their own local menu presentation; the fixed system and
    // application menus stay.
    property bool globalMenuEnabled: true
    // [{ id, icon, iconSource, label, tooltip, menu, priority, visible,
    //    available, enabled, selected, state, accessibleName, kind,
    //    thirdParty }]; every system extra and every third-party
    //    StatusNotifier item joins this one registry (T231). The shell
    //    controller supplies live entries; `StatusItemRegistry` owns order,
    //    visibility and overflow.
    property var statusItems: []
    // The row's slot budget. -1 derives the budget from the measured bar
    // width (T233); a finite value is an explicit override used by tests and
    // captures. The registry moves third-party items into the overflow
    // affordance first, then optional system items, and finally the
    // lowest-priority essential items on a very narrow display.
    property int statusOverflowCapacity: -1
    // T233: a notch/safe-area inset. It is not a hit target; it only reduces
    // the width the status row may use, so items move to overflow rather than
    // being clipped around a camera housing.
    property int notchWidth: 0
    // T233 multi-display policy. One all-output surface is composited per
    // display; the active display owns the application menu and may be the
    // only one that shows status items. `MenuBarDisplayPolicy` is the pure
    // resolver the shell can use when it gains per-display surfaces.
    property bool activeDisplay: true
    property bool showStatusItems: true
    // T233: the compositor-owned reveal fraction, mirrored here so the scene
    // and its captures can show the measured slide. Production motion is
    // applied by the compositor to the surface; this property is the QML
    // introspection/test seam, like `visibleIn()`.
    property real revealProgress: 1.0
    readonly property real revealOffset:
        (1.0 - Math.max(0.0, Math.min(1.0, revealProgress)))
        * Theme.controls.menuBar.height
    readonly property bool revealMotionActive:
        revealProgress > 0.0 && revealProgress < 1.0
    // The built-in fallbacks keep the bar usable in QML tests and before the
    // controller's first status snapshot. The controller's entries win by id.
    property bool showClock: true
    // The open tray item's DBusMenu rows (T-14.3), a design-system menu model.
    property var trayMenu: []
    property string trayMenuService: ""
    property bool trayMenuOpen: false
    // The bridge host's decoded status views (T-07.5a/T-07.5b); empty until
    // the shell applies them, which hides the corresponding item.
    property var wifiMenu: ({})
    property var volumeMenu: ({})
    property var batteryMenu: ({})
    property var bluetoothMenu: ({})
    // Which status popover is open: "" | "wifi" | "volume" | "battery" |
    // "bluetooth" | "clock".
    property string openStatusItem: ""
    // The overflow affordance's menu (T231): third-party items that do not
    // fit in `statusOverflowCapacity`, then optional system items.
    property bool overflowMenuOpen: false
    property var overflowMenu: []
    property var overflowMenuEntries: []
    // Keyboard navigation (T-07.5b): the status slot the arrow keys select.
    // -1 means no slot is selected.
    property int keyboardStatusIndex: -1
    readonly property bool statusMenuOpen: openStatusItem !== ""
    property bool showDate: false
    property bool showSeconds: false
    property bool use24Hour: false
    // T-15.9b follow-up: the OS menu glyph the user picked in Settings
    // (`menu.osGlyph`). "dragonfruit" is the brand mark, "tux" the alternate
    // penguin mark. The system menu's slot stays square for either.
    property string osGlyph: "dragonfruit"
    // T-15.9b: `menu.showBackground`. When off the bar keeps its items and its
    // reserved zone but draws no chrome material, so the desktop shows through.
    property bool showBackground: true
    // T201: `menu.autoHide`. The compositor owns the actual per-output reveal;
    // this property is the bar's copy of the policy so the scene and its tests
    // share the same semantics. `visibleIn()` is the pure rule.
    property string autoHideMode: "full-screen"
    // T277: `menu.overflow`. `automatic` lets the measured width decide;
    // `always` keeps the overflow affordance visible and moves every
    // third-party item into it first.
    property string overflowPolicy: "automatic"
    // Test/introspection helpers for the policy rule.
    readonly property bool autoHideAlways: autoHideMode === "always"
    readonly property bool autoHideInFullscreen: autoHideMode === "full-screen"

    // Whether the bar is visible for one output. `revealed` is the compositor's
    // transient reveal and `active` is an open menu or menu-bar focus.
    function visibleIn(fullscreen, revealed, active) {
        if (autoHideMode === "never")
            return true;
        if (autoHideMode === "always")
            return revealed || active;
        return !fullscreen || revealed || active;
    }
    // Effective chrome material from the compositor (P1). When the sampled
    // backdrop pass is not actually running (the current DRM rail, headless,
    // or a blur-off tier), the bar uses the near-opaque milky fallback rather
    // than a translucent fill over an unfrosted wallpaper.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    // T200: the accessibility overrides the compositor applies to the veil.
    // They are pushed by the shell alongside the role material state so QML
    // can choose the scheme-appropriate opaque fallback when the veil is off.
    property bool reduceTransparency: false
    property bool increaseContrast: false
    // T-21.2c: the transient Popup role state for every dropdown/popover in
    // the bar scene. The shell applies it through the same MaterialPolicy as
    // the persistent Chrome role above; QML never re-derives the tier.
    property bool popupMaterialBlurActive: true
    property int popupMaterialTier: 0
    property int popupMaterialFallback: 0
    readonly property string popupMaterialFallbackReason:
        Material.fallbackReason(popupMaterialFallback, popupMaterialBlurActive,
                                popupMaterialTier)
    // The compositor reports when the shell/menu surface holds focus; when
    // it is lost the open menu dismisses (FR-3).
    property bool shellFocused: true
    // T-09 keyboard entry: Ctrl+F2 (compositor `FocusMenuBar`) hands the bar
    // the keyboard. `keyboardMenuIndex` is the title ring, `keyboardStatusIndex`
    // the status ring; -1 means that family is not focused.
    property bool keyboardFocused: false
    property int keyboardMenuIndex: -1
    // The output's logical size, injected by the shell controller. It is the
    // bound a dropdown clamps to, independent of the temporary scene growth
    // used to render an open popup.
    property int outputWidth: 0
    property int outputHeight: 0
    readonly property var hostBounds: ({
        x: 0, y: 0,
        w: menuBar.outputWidth > 0 ? menuBar.outputWidth
          : (Window.window ? Window.window.width : menuBar.width),
        h: menuBar.outputHeight > 0 ? menuBar.outputHeight
          : (Window.window ? Window.window.height : menuBar.height)
    })

    signal keyboardFocusReleaseRequested()

    // The registry source: the controller's entries plus the built-in
    // clock/Quick Settings/Overview fallbacks. Deduplicating by id lets the
    // controller own visibility (a hidden clock is not re-added here) while
    // the QML tests and the first frame still get a complete bar.
    readonly property var _statusSource: {
        var source = [];
        var seen = ({});
        for (var i = 0; i < statusItems.length; ++i) {
            var raw = statusItems[i] || {};
            var id = raw.id !== undefined ? String(raw.id) : "";
            if (id.length === 0)
                continue;
            source.push(raw);
            seen[id] = true;
        }
        var builtins = [
            { id: "clock", kind: "clock", menu: "clock", priority: 70,
              tooltip: qsTr("Date & Time"), accessibleName: qsTr("Date & Time"),
              visible: showClock, essential: true },
            { id: "control-center", icon: "control-center", kind: "action",
              priority: 80, tooltip: qsTr("Quick Settings"),
              accessibleName: qsTr("Quick Settings"), visible: true,
              essential: true },
            { id: "mission-control", icon: "mission-control", kind: "action",
              priority: 90, tooltip: qsTr("Overview"),
              accessibleName: qsTr("Overview"), visible: true,
              essential: true }
        ];
        for (var j = 0; j < builtins.length; ++j) {
            if (!seen[builtins[j].id])
                source.push(builtins[j]);
        }
        return source;
    }

    StatusItemRegistry {
        id: statusRegistry
        items: menuBar._statusSource
        maxVisible: menuBar.effectiveStatusCapacity
        forceOverflow: menuBar.overflowPolicy === "always"
    }

    // T233: the status-row budget is measured, not guessed. The passive
    // measurement repeater reports one `StatusItemMeasure` width per registry
    // entry, so a localized clock, a tray label, or a themed icon changes the
    // capacity exactly as it changes the row. The registry's ordered list does
    // not depend on the budget, so there is no binding cycle.
    readonly property var _statusSlotWidths: {
        var widths = [];
        for (var i = 0; i < statusMeasureRepeater.count; ++i) {
            var item = statusMeasureRepeater.itemAt(i);
            widths.push(item ? item.measuredWidth : 0);
        }
        return widths;
    }
    // The full width of every visible status slot, in registry order. This is
    // the un-budgeted row width the bar's own implicit width uses, so an unset
    // width resolves to "every item fits" without feeding the overflow
    // capacity back into the bar's width.
    readonly property real _statusMeasureWidth: {
        var widths = _statusSlotWidths;
        var gap = Theme.controls.menuBar.statusItemGap;
        var total = 0;
        var count = 0;
        for (var i = 0; i < widths.length; ++i) {
            if (widths[i] <= 0)
                continue;
            total += widths[i] + (count > 0 ? gap : 0);
            count++;
        }
        return total;
    }
    readonly property real _statusAvailableWidth:
        Math.max(0, width - appMenuRow.implicitWidth
                 - 2 * Theme.controls.menuBar.paddingH - notchWidth)
    readonly property int measuredStatusCapacity: {
        var widths = _statusSlotWidths;
        var gap = Theme.controls.menuBar.statusItemGap;
        var visibleCount = 0;
        var total = 0;
        for (var i = 0; i < widths.length; ++i) {
            if (widths[i] <= 0)
                continue;
            total += widths[i] + (visibleCount > 0 ? gap : 0);
            visibleCount++;
        }
        if (visibleCount === 0)
            return 0;
        if (total <= _statusAvailableWidth)
            return visibleCount;
        // Overflow needs one slot of its own. Reserve it, then count the
        // leading items that still fit. The registry decides which items
        // leave (third-party first); this is only the slot count.
        var overflowWidth = Theme.controls.menuBar.iconSize
            + 2 * Theme.controls.menuBar.statusItemPaddingH;
        var budget = Math.max(0, _statusAvailableWidth - overflowWidth - gap);
        var fit = 0;
        var used = 0;
        for (var j = 0; j < widths.length; ++j) {
            if (widths[j] <= 0)
                continue;
            var need = widths[j] + (fit > 0 ? gap : 0);
            if (used + need > budget)
                break;
            used += need;
            fit++;
        }
        return fit;
    }
    readonly property int effectiveStatusCapacity:
        statusOverflowCapacity >= 0 ? statusOverflowCapacity : measuredStatusCapacity
    readonly property var statusRowEntries: statusRegistry.statusRowItems
    readonly property var statusOverflowEntries: statusRegistry.overflowItems
    readonly property bool statusHasOverflow: statusRegistry.hasOverflow
    readonly property bool statusRowVisible: showStatusItems

    // The bar's top-level menus, left to right: the fixed system menu, the
    // fixed application menu, then whatever the focused app exported. Keeping
    // them in one model is what lets drag-through and the open-menu-tracks-
    // focus rules span all three groups. `kind` selects the rendering
    // treatment ("system" -> brand mark, "app" -> bold title).
    readonly property var topLevelMenus: {
        var out = [{ kind: "system", title: qsTr("System"), items: systemMenuItems }];
        // T233 multi-display: only the active display owns the application
        // menu and the focused app's exported menus. A secondary display
        // keeps the shell-owned system menu and its status items.
        if (!activeDisplay)
            return out;
        out.push({ kind: "app", title: appName, items: applicationMenuItems });
        // The global-menu toggle (T-14.2b) suppresses the app's exported menus
        // only; the fixed system and application menus are always present.
        if (!globalMenuEnabled)
            return out;
        for (var i = 0; i < appMenuModel.length; ++i) {
            var m = appMenuModel[i] || {};
            out.push({ kind: "menu", title: m.title || "", items: m.items || [] });
        }
        return out;
    }

    // --- Interaction state ------------------------------------------------
    property int openMenuIndex: -1
    property int hoverMenuIndex: -1
    readonly property var activeStatusPopup: openStatusItem === "wifi" ? wifiMenuPopup
        : openStatusItem === "volume" ? volumeMenuPopup
        : openStatusItem === "battery" ? batteryMenuPopup
        : openStatusItem === "bluetooth" ? bluetoothMenuPopup
        : openStatusItem === "clock" ? dateTimePanel : null

    // Geometry of the open dropdown in window coordinates. The shell process
    // renders the popup into a separate `overlay` chrome surface placed at
    // this rectangle, so the transient menu composites above fullscreen
    // windows while the bar keeps its own `top` surface and reserved zone.
    readonly property var _dropdownRect: {
        if (activeStatusPopup) {
            var statusPopup = activeStatusPopup.popup;
            if (statusPopup.open) {
                var statusOrigin =
                    activeStatusPopup.mapToItem(null, statusPopup.x, statusPopup.y);
                return { x: statusOrigin.x, y: statusOrigin.y,
                         w: statusPopup.width, h: statusPopup.height };
            }
        }
        if (overflowMenuOpen && overflowContextMenu.open) {
            var overflowRect = overflowContextMenu.contentRect;
            var overflowOrigin =
                overflowContextMenu.mapToItem(null, overflowRect.x, overflowRect.y);
            return { x: overflowOrigin.x, y: overflowOrigin.y,
                     w: overflowRect.w, h: overflowRect.h };
        }
        if (trayMenuOpen && trayContextMenu.open) {
            var trayRect = trayContextMenu.contentRect;
            var trayOrigin = trayContextMenu.mapToItem(null, trayRect.x, trayRect.y);
            return { x: trayOrigin.x, y: trayOrigin.y, w: trayRect.w, h: trayRect.h };
        }
        if (openMenuIndex < 0)
            return { x: 0, y: 0, w: 0, h: 0 };
        var item = menuRepeater.itemAt(openMenuIndex);
        if (!item || !item.popup)
            return { x: 0, y: 0, w: 0, h: 0 };
        // `contentRect` is the dropdown unioned with any open submenu, so the
        // overlay surface grows to contain the nested panel instead of
        // clipping it.
        var rect = item.contentRect;
        if (rect && rect.w > 0 && rect.h > 0) {
            var origin = item.mapToItem(null, rect.x, rect.y);
            return { x: origin.x, y: origin.y, w: rect.w, h: rect.h };
        }
        var popup = item.popup;
        var topLeft = popup.mapToItem(null, 0, 0);
        return { x: topLeft.x, y: topLeft.y, w: popup.width, h: popup.height };
    }
    readonly property real dropdownX: _dropdownRect.x
    readonly property real dropdownY: _dropdownRect.y
    readonly property real dropdownWidth: _dropdownRect.w
    readonly property real dropdownHeight: _dropdownRect.h
    // T202: the preset for the open dropdown and the independently keyed
    // material regions the shell declares on the popup overlay. Attached
    // app menus keep square top corners; status menus and the tray context
    // menu are detached/status cards.
    readonly property string dropdownPreset: {
        if (openStatusItem !== "")
            return "statusMenu";
        if (overflowMenuOpen && overflowContextMenu.open)
            return "menuDetached";
        if (trayMenuOpen && trayContextMenu.open)
            return "menuDetached";
        return "menuAttached";
    }
    readonly property var dropdownRegions: {
        var out = [];
        if (activeStatusPopup) {
            var popup = activeStatusPopup.popup;
            if (popup.open) {
                var p = activeStatusPopup.mapToItem(null, popup.x, popup.y);
                var r = Theme.controls.contextMenu.radius;
                out.push({ key: "menu", preset: "statusMenu",
                           x: p.x, y: p.y, w: popup.width, h: popup.height,
                           tl: r, tr: r, br: r, bl: r, overscan: 20 });
            }
            return out;
        }
        if (overflowMenuOpen && overflowContextMenu.open) {
            var overflowParent = overflowContextMenu.parent;
            var overflowOrigin = overflowParent
                ? overflowParent.mapToItem(null, overflowContextMenu.x,
                                           overflowContextMenu.y)
                : Qt.point(overflowContextMenu.x, overflowContextMenu.y);
            var overflowRegions = overflowContextMenu.regions;
            for (var oi = 0; oi < overflowRegions.length; ++oi) {
                var overflowRegion = overflowRegions[oi];
                out.push({ key: overflowRegion.key, preset: overflowRegion.preset,
                           x: overflowOrigin.x + overflowRegion.x,
                           y: overflowOrigin.y + overflowRegion.y,
                           w: overflowRegion.w, h: overflowRegion.h,
                           tl: overflowRegion.tl, tr: overflowRegion.tr,
                           br: overflowRegion.br, bl: overflowRegion.bl,
                           overscan: overflowRegion.overscan });
            }
            return out;
        }
        if (trayMenuOpen && trayContextMenu.open) {
            var trayParent = trayContextMenu.parent;
            var origin = trayParent
                ? trayParent.mapToItem(null, trayContextMenu.x, trayContextMenu.y)
                : Qt.point(trayContextMenu.x, trayContextMenu.y);
            var regions = trayContextMenu.regions;
            for (var i = 0; i < regions.length; ++i) {
                var region = regions[i];
                out.push({ key: region.key, preset: region.preset,
                           x: origin.x + region.x, y: origin.y + region.y,
                           w: region.w, h: region.h,
                           tl: region.tl, tr: region.tr,
                           br: region.br, bl: region.bl,
                           overscan: region.overscan });
            }
            return out;
        }
        if (openMenuIndex >= 0) {
            var item = menuRepeater.itemAt(openMenuIndex);
            if (item) {
                var itemParent = item.parent;
                var itemOrigin = itemParent
                    ? itemParent.mapToItem(null, item.x, item.y)
                    : Qt.point(item.x, item.y);
                var itemRegions = item.regions;
                for (var j = 0; j < itemRegions.length; ++j) {
                    var itemRegion = itemRegions[j];
                    out.push({ key: itemRegion.key, preset: itemRegion.preset,
                               x: itemOrigin.x + itemRegion.x,
                               y: itemOrigin.y + itemRegion.y,
                               w: itemRegion.w, h: itemRegion.h,
                               tl: itemRegion.tl, tr: itemRegion.tr,
                               br: itemRegion.br, bl: itemRegion.bl,
                               overscan: itemRegion.overscan });
                }
            }
        }
        return out;
    }

    // The compositor adds the popup surface origin. Publish coordinates
    // relative to the same integer crop used for its buffer, not the bar.
    readonly property var dropdownSurfaceRegions: {
        var dx = Math.max(0, Math.floor(dropdownX));
        var dy = Math.max(0, Math.floor(dropdownY));
        return dropdownRegions.map(function(region) {
            return { key: region.key, preset: region.preset,
                     x: region.x - dx, y: region.y - dy,
                     w: region.w, h: region.h,
                     tl: region.tl, tr: region.tr,
                     br: region.br, bl: region.bl,
                     overscan: region.overscan };
        });
    }

    signal appMenuTriggered(int menuIndex, int itemIndex, var item)
    signal appMenuOpened(int menuIndex)
    signal appMenuClosed()
    signal popupOpened()
    signal popupClosed()
    signal statusItemActivated(string itemId)
    // A row of an open StatusNotifier tray menu was chosen; the shell forwards
    // it to the owning item's DBusMenu (T-14.3).
    signal trayMenuTriggered(string service, int id)
    signal trayMenuClosed()
    // Wi-Fi and volume popovers (T-07.5a). The bar raises the user's action;
    // the shell controller forwards it to the bridge host.
    signal wifiJoinRequested(string ssid, string secret)
    signal wifiToggleRequested(bool enabled)
    signal wifiSettingsRequested()
    signal volumeSetRequested(double volume)
    signal muteToggleRequested()
    signal volumeSettingsRequested()
    signal sinkSelected(int id)
    signal batterySettingsRequested()
    signal powerProfileSelected(string profile)
    // Bluetooth status menu (T202): the same action policy as the Control
    // Center tile, routed through the existing bridge-host slots.
    signal bluetoothPowerToggleRequested(bool enabled)
    signal bluetoothDeviceToggled(string address, bool connected)
    signal bluetoothSettingsRequested()
    signal bluetoothDiscoveringToggled(bool discovering)
    signal bluetoothPairRequested(string address)
    signal statusMenuRefreshRequested(string itemId)
    signal statusMenuOpened()
    signal statusMenuClosed()
    signal controlCenterRequested()
    signal missionControlRequested()

    implicitHeight: Theme.controls.menuBar.height
    // The bar root holds the keyboard during keyboard entry; a focused title
    // or status item then owns the arrow keys.
    focus: menuBar.keyboardFocused
    activeFocusOnTab: false
    // Use the measurement row, not the budget-dependent visible row, so an
    // unset width resolves to "every item fits" instead of feeding the
    // overflow capacity back into the bar's own width.
    implicitWidth: appMenuRow.implicitWidth
                   + (showStatusItems ? menuBar._statusMeasureWidth : 0)
                   + 2 * Theme.controls.menuBar.paddingH
    // T233: the measured slide, mirrored from the compositor's reveal
    // progress. At 1.0 this is a no-op.
    transform: Translate { y: -menuBar.revealOffset }

    // T-20.1: the bar's fill is the translucent chrome material (the same
    // `material.chromeOpacity` the compositor reads), so its sampled backdrop
    // blur shows through; the items/text stay fully opaque because the alpha
    // is in the fill, not on the item. When the sampled pass is absent (P1),
    // the milky fallback keeps the same chrome geometry and text contrast.
    readonly property color chromeFill: Theme.color.chrome
    readonly property color chromeFallbackFill: Theme.color.chromeFallbackFill
    // T200: the replacement rail moves the menu-bar backdrop into the
    // compositor. In that mode the QML surface paints no fill at all; the
    // compositor's sharp veil sits below the foreground and the popups. When
    // the veil cannot run (legacy rollout, Reduce Transparency, or no sampled
    // scene texture) QML keeps the scheme-appropriate plate, so the bar never
    // shows an unfiltered wallpaper strip.
    readonly property int veilExtent: Theme.glass.presets.menuBarVeil.extent
    readonly property int referenceStripHeight:
        Theme.glass.presets.menuBarVeil.stripHeight
    readonly property string materialBackend:
        menuBar.materialBlurActive ? "gpu-sampled"
        : (menuBar.materialFallback === 2 ? "software" : "gpu-no-sampled-blur")
    readonly property string materialTierName:
        menuBar.materialTier <= 0 ? "full"
        : (menuBar.materialTier === 1 ? "reduced" : "minimal")
    readonly property var veilResolution:
        Glass.resolve("menuBarVeil", {
            scheme: menuBar.veilScheme,
            backend: menuBar.materialBackend,
            tier: menuBar.materialTierName,
            reduceTransparency: menuBar.reduceTransparency,
            increaseContrast: menuBar.increaseContrast
        })
    readonly property bool veilActive:
        menuBar.showBackground && menuBar.materialBlurActive
        && menuBar.veilResolution.renderMode === "no-blur-veil"
    // T200 text treatment: while the compositor veil runs it is the bar's
    // contrast mechanism, so the lettering and marks are flat. The reference
    // measures plain near-black text on a white veil and records no outline
    // or three-layer halo. The wallpaper contrast treatment returns only when
    // the veil is not the background (`showBackground` off, or a no-veil
    // fallback/first frame), where the items can be drawn over the wallpaper.
    readonly property bool textTreatmentEnabled:
        !menuBar.veilActive
    // The region the shell commits to the compositor. The legacy sampled rail
    // keeps the 28 pt bar panel; the veil declares its full 95 pt visual
    // extent while the input region and reservation stay the strip.
    readonly property bool compositorPanelActive:
        menuBar.showBackground && menuBar.materialBlurActive
        && (menuBar.veilResolution.renderMode === "no-blur-veil"
            || menuBar.veilResolution.renderMode === "sampled-glass")
    readonly property int compositorPanelHeight:
        menuBar.veilActive ? menuBar.veilExtent : Theme.controls.menuBar.height
    readonly property int compositorPanelRadius:
        menuBar.veilActive ? 0 : Theme.controls.menuBar.radius

    // Cached wallpaper top-band sRGB luminance selects one foreground/veil
    // pair. The compositor receives the same polarity with the content frame.
    property real backgroundLuma: 0.5
    // Wallpaper polarity, not the app appearance, owns the menu-bar pair.
    // Prefer white lettering with a dark wash except on very bright scenes.
    readonly property string veilScheme:
        backgroundLuma >= Theme.controls.menuBar.foregroundFlipThresholdLight
            ? "light" : "dark"
    // The legacy sampled rail paints a scheme-coloured glass plate, so it
    // keeps the scheme pair. The no-blur veil and the background-off mode are
    // wallpaper-driven.
    readonly property bool wallpaperPolarity:
        !menuBar.showBackground || menuBar.veilActive
    // The luminance the lettering actually sits on: the veil-composited
    // backdrop when the veil runs, otherwise the wallpaper band. Using the
    // pre-veil wallpaper luma was the old bug: it left white text on the
    // 0.55–0.78 band's mid-grey composite.
    readonly property real foregroundLuma:
        menuBar.veilActive ? menuBar.renderedLuma : menuBar.backgroundLuma

    function veilAlphaAt(depth) {
        if (!menuBar.veilActive)
            return 0;
        var v = menuBar.veilResolution.veil;
        if (!v)
            return 0;
        var depths = [5, 15, 25, 35, 50, 75, 90, 95];
        var alphas = [v.alpha5, v.alpha15, v.alpha25, v.alpha35,
                      v.alpha50, v.alpha75, v.alpha90, v.alpha95];
        var base = 0;
        if (depth <= depths[0]) {
            base = alphas[0];
        } else {
            for (var i = 1; i < depths.length; ++i) {
                if (depth <= depths[i]) {
                    var span = depths[i] - depths[i - 1];
                    var t = span <= 0 ? 0 : (depth - depths[i - 1]) / span;
                    base = alphas[i - 1] + (alphas[i] - alphas[i - 1]) * t;
                    break;
                }
            }
        }
        var luma = Math.max(0, Math.min(1, menuBar.backgroundLuma));
        var adaptLuma = v.adaptLuma;
        var adaptScale = v.adaptScale;
        var scale = adaptScale[adaptScale.length - 1];
        if (luma <= adaptLuma[0]) {
            scale = adaptScale[0];
        } else {
            for (var j = 1; j < adaptLuma.length; ++j) {
                if (luma <= adaptLuma[j]) {
                    var adaptSpan = adaptLuma[j] - adaptLuma[j - 1];
                    var u = adaptSpan <= 0 ? 0 : (luma - adaptLuma[j - 1]) / adaptSpan;
                    scale = adaptScale[j - 1] + (adaptScale[j] - adaptScale[j - 1]) * u;
                    break;
                }
            }
        }
        return base * scale;
    }

    readonly property real barVeilAlpha:
        menuBar.veilAlphaAt(Theme.controls.menuBar.height * 0.5)
    readonly property real renderedLuma: {
        var alpha = menuBar.barVeilAlpha;
        var veilLuma = menuBar.veilResolution.tint;
        return menuBar.backgroundLuma * (1 - alpha) + veilLuma * alpha;
    }
    // Hysteresis keeps a wallpaper that sits on the threshold from flickering
    // the lettering. The latch is updated by the luminance handler and seeded
    // on completion, so a test that sets `backgroundLuma` at creation is
    // correct on the first frame.
    property bool _darkForegroundLatch: false
    function updateForegroundLatch() {
        var threshold = Theme.controls.wallpaperText.flipThreshold;
        var band = Theme.controls.wallpaperText.flipHysteresis;
        if (menuBar.foregroundLuma >= threshold + band)
            menuBar._darkForegroundLatch = true;
        else if (menuBar.foregroundLuma <= threshold - band)
            menuBar._darkForegroundLatch = false;
    }
    onForegroundLumaChanged: updateForegroundLatch()
    Component.onCompleted: updateForegroundLatch()
    readonly property bool darkForeground:
        menuBar.wallpaperPolarity ? menuBar._darkForegroundLatch
                                  : !Theme.dark
    readonly property color foreground:
        menuBar.darkForeground ? Theme.controls.menuBar.foregroundOnLight
                               : Theme.controls.menuBar.foregroundOnDark
    readonly property color secondaryForeground:
        menuBar.darkForeground ? Theme.controls.menuBar.foregroundSecondaryOnLight
                               : Theme.controls.menuBar.foregroundSecondaryOnDark

    readonly property color glassFill: Qt.rgba(
        menuBar.chromeFill.r, menuBar.chromeFill.g, menuBar.chromeFill.b,
        Theme.material.chromeOpacity)
    readonly property color fallbackFill: Qt.rgba(
        menuBar.chromeFallbackFill.r, menuBar.chromeFallbackFill.g,
        menuBar.chromeFallbackFill.b, Theme.material.chromeFallbackOpacity)
    color: !menuBar.showBackground || menuBar.veilActive ? "transparent"
        : (menuBar.materialBlurActive
           && menuBar.veilResolution.renderMode === "sampled-glass")
          ? menuBar.glassFill
          : menuBar.fallbackFill

    function openMenu(index) {
        if (index < 0 || index >= menuRepeater.count || index === openMenuIndex)
            return;
        var item = menuRepeater.itemAt(index);
        if (item)
            item.openMenu();
    }

    function closeMenus(immediate) {
        closeTrayMenu(immediate);
        closeOverflowMenu(immediate);
        var index = openMenuIndex;
        openMenuIndex = -1;
        hoverMenuIndex = -1;
        switchTimer.stop();
        // Only announce a close when a menu was actually open. A menu item's
        // own handler may already have closed it, and the bar's click-away
        // handler runs afterwards; emitting twice made the dismissal janky.
        if (index >= 0) {
            var item = menuRepeater.itemAt(index);
            if (item)
                item.closeMenu(immediate === true);
            appMenuClosed();
            popupClosed();
        }
    }

    function toggleMenu(index) {
        if (openMenuIndex === index)
            closeMenus();
        else
            openMenu(index);
    }

    // Left/Right from an open dropdown's top level moves to the next sibling
    // menu with rows, wrapping across the system/application/app menus (T230).
    function moveMenuSelection(index, delta) {
        var count = topLevelMenus.length;
        if (count <= 0)
            return;
        var i = index;
        for (var step = 0; step < count; ++step) {
            i = ((i + delta) % count + count) % count;
            var menu = topLevelMenus[i] || {};
            if (menu.items && menu.items.length > 0) {
                openMenu(i);
                return;
            }
        }
    }

    // Test/introspection hooks: the shell controller uses these to drive
    // the bar; the QML tests use them to assert without poking at the scene.
    function appMenuAt(index) {
        return menuRepeater.itemAt(index);
    }

    // T230 capture seam: activate one row through the same path a click uses.
    // The shell's `DF_MENUBAR_ACTIVATE_FIXTURE` calls this after opening a menu.
    function activateMenuRow(menuIndex, itemIndex) {
        var item = menuRepeater.itemAt(menuIndex);
        if (item)
            item.activate(itemIndex);
    }

    function statusItemAt(index) {
        return statusRepeater.itemAt(index);
    }

    // The live status-item delegate for `id` ("wifi"/"volume"), or null before
    // the Repeater has produced it. Used to anchor the popovers.
    function statusItemFor(id) {
        for (var i = 0; i < statusRepeater.count; ++i) {
            var item = statusRepeater.itemAt(i);
            if (item && item.itemId === id)
                return item;
        }
        return null;
    }

    // The live tray StatusItem for a registered name ("tray:<name>"), or null.
    function trayItemFor(service) {
        return statusItemFor("tray:" + service);
    }

    // Open a tray item's DBusMenu beneath its slot, right-aligned so it stays
    // inside the bar when the slot is near the right edge. The shell sets
    // `trayMenu` before calling this.
    function openTrayMenu(service) {
        if (!service || service.length === 0)
            return;
        closeMenus(true);
        closeStatusMenu(true);
        trayMenuService = service;
        trayMenuOpen = true;
        var slot = trayItemFor(service);
        var menuWidth = Math.max(trayContextMenu.width, Theme.controls.contextMenu.minWidth);
        var anchorX = slot ? slot.mapToItem(menuBar, slot.width, 0).x : menuBar.width;
        var left = Math.max(0, Math.min(menuBar.width - menuWidth, anchorX - menuWidth));
        trayContextMenu.showAt(left, menuBar.height);
        popupOpened();
    }

    function closeTrayMenu(immediate) {
        if (!trayMenuOpen)
            return;
        trayMenuOpen = false;
        trayContextMenu.hide(immediate === true);
        trayMenuClosed();
        popupClosed();
    }

    // Open the popover for a status item. A hidden item (daemon absent, or a
    // machine with no battery) has nothing to open, matching the graceful-
    // degradation rule.
    function openStatusMenu(id) {
        if (id !== "wifi" && id !== "volume" && id !== "battery"
                && id !== "bluetooth" && id !== "clock")
            return;
        if (id !== "clock") {
            var data = id === "wifi" ? wifiMenu
                     : (id === "volume" ? volumeMenu
                     : (id === "battery" ? batteryMenu : bluetoothMenu));
            if (!data || data.state === "unavailable")
                return;
            if (id === "bluetooth" && data.present === false)
                return;
            // A desktop can have no battery but still select a power profile;
            // only hide the battery popover when neither half has content.
            if (id === "battery" && data.present === false
                    && data.profilesAvailable !== true)
                return;
        }
        closeMenus(true);
        closeStatusMenu(true);
        openStatusItem = id;
        activeStatusPopup.open = true;
        statusMenuRefreshRequested(id);
        statusMenuOpened();
        popupOpened();
    }

    function closeStatusMenu(immediate) {
        // `openStatusItem` is the single owner of "a status popover is open".
        // Guarding on it (not on the popups) makes the re-entrant call from a
        // popup's own `closed` signal a no-op instead of double-emitting.
        if (openStatusItem === "")
            return;
        var popup = activeStatusPopup.popup;
        openStatusItem = "";
        popup.hide(immediate === true);
        statusMenuClosed();
        popupClosed();
        // Keep the status ring on the item that opened the popover when the
        // bar owns keyboard entry; a pointer dismissal clears it.
        if (!keyboardFocused)
            keyboardStatusIndex = -1;
        // Return keyboard focus to the bar so the arrow keys keep driving the
        // status row after a popover closes.
        if (openMenuIndex < 0)
            forceActiveFocus();
    }

    // One activation path for every registry entry (T231). The item's
    // `menu` names the first-party popover; tray/action items fall through to
    // the shell's `statusItemActivated`, and the two overlay entries raise
    // their existing signals.
    function activateStatusItem(id) {
        var item = statusRegistry.itemById(id);
        if (item && item.kind === "overflow") {
            toggleOverflowMenu();
            return;
        }
        if (item && item.kind === "clock") {
            openStatusMenu("clock");
            return;
        }
        // First-party popovers keep the id fallback so a raw entry from an
        // older producer (or a QML test) still opens its menu.
        if (id === "wifi" || id === "volume" || id === "battery"
                || id === "bluetooth") {
            openStatusMenu(id);
            return;
        }
        if (item && item.menu.length > 0 && item.menu !== "overflow") {
            openStatusMenu(item.menu);
            return;
        }
        closeMenus();
        closeStatusMenu();
        if (id === "control-center") {
            controlCenterRequested();
            return;
        }
        if (id === "mission-control") {
            missionControlRequested();
            return;
        }
        statusItemActivated(id);
    }

    function _overflowRows() {
        var rows = [];
        for (var i = 0; i < overflowMenuEntries.length; ++i) {
            var item = overflowMenuEntries[i];
            rows.push({ label: item.accessibleName,
                        enabled: item.enabled !== false,
                        checked: item.selected === true,
                        checkable: false,
                        action: item.id });
        }
        return rows;
    }

    function openOverflowMenu() {
        if (!statusRegistry.hasOverflow)
            return;
        closeMenus(true);
        closeStatusMenu(true);
        overflowMenuEntries = statusRegistry.overflowItems;
        overflowMenu = _overflowRows();
        overflowMenuOpen = true;
        var slot = statusItemFor("overflow");
        var menuWidth = Math.max(overflowContextMenu.width,
                                 Theme.controls.contextMenu.minWidth);
        var anchorX = slot ? slot.mapToItem(menuBar, slot.width, 0).x : menuBar.width;
        var left = Math.max(0, Math.min(menuBar.width - menuWidth,
                                      anchorX - menuWidth));
        overflowContextMenu.showAt(left, menuBar.height);
        popupOpened();
    }

    function closeOverflowMenu(immediate) {
        if (!overflowMenuOpen)
            return;
        overflowMenuOpen = false;
        overflowContextMenu.hide(immediate === true);
        popupClosed();
    }

    function toggleOverflowMenu() {
        if (overflowMenuOpen)
            closeOverflowMenu();
        else
            openOverflowMenu();
    }

    // --- Keyboard navigation (T-07.5b) ------------------------------------
    // The bar keeps focus; Left/Right move a ring across the focusable status
    // slots and Return/Space opens the selected slot's popover (or activates
    // it). Escape closes. This is the keyboard path to every status menu.
    function moveStatusFocus(delta) {
        var n = statusRepeater.count;
        if (n <= 0) {
            keyboardStatusIndex = -1;
            return;
        }
        var i = keyboardStatusIndex < 0 ? (delta > 0 ? -1 : 0) : keyboardStatusIndex;
        for (var step = 0; step < n; ++step) {
            i = ((i + delta) % n + n) % n;
            var item = statusRepeater.itemAt(i);
            if (item && item.available) {
                keyboardStatusIndex = i;
                return;
            }
        }
        keyboardStatusIndex = -1;
    }

    function titleHasItems(index) {
        var menu = topLevelMenus[index] || {};
        return menu.items !== undefined && menu.items.length > 0;
    }

    function firstTitleIndex() {
        for (var i = 0; i < topLevelMenus.length; ++i) {
            if (titleHasItems(i))
                return i;
        }
        return -1;
    }

    function lastTitleIndex() {
        for (var i = topLevelMenus.length - 1; i >= 0; --i) {
            if (titleHasItems(i))
                return i;
        }
        return -1;
    }

    function nextStatusIndex(from, delta) {
        var n = statusRepeater.count;
        var i = from + delta;
        while (i >= 0 && i < n) {
            var item = statusRepeater.itemAt(i);
            if (item && item.available)
                return i;
            i += delta;
        }
        return -1;
    }

    function focusTitle(index) {
        var item = menuRepeater.itemAt(index);
        if (item) {
            keyboardMenuIndex = index;
            item.forceActiveFocus();
        }
    }

    // Predictable title/status traversal: Left/Right moves within the title
    // row, crosses into the status row at the edge, and comes back from the
    // status row's other edge.
    function moveKeyboardFocus(delta) {
        if (keyboardMenuIndex >= 0) {
            var i = keyboardMenuIndex;
            for (var step = 0; step < topLevelMenus.length; ++step) {
                i += delta;
                if (i < 0 || i >= topLevelMenus.length) {
                    keyboardMenuIndex = -1;
                    keyboardStatusIndex = -1;
                    var statusIndex = nextStatusIndex(delta > 0
                                                      ? -1 : statusRepeater.count,
                                                      delta);
                    if (statusIndex >= 0) {
                        keyboardStatusIndex = statusIndex;
                        forceActiveFocus();
                    }
                    return;
                }
                if (titleHasItems(i)) {
                    keyboardStatusIndex = -1;
                    focusTitle(i);
                    return;
                }
            }
            return;
        }
        if (keyboardStatusIndex >= 0) {
            var next = nextStatusIndex(keyboardStatusIndex, delta);
            if (next >= 0) {
                keyboardStatusIndex = next;
                forceActiveFocus();
                return;
            }
            keyboardStatusIndex = -1;
            var title = delta > 0 ? firstTitleIndex() : lastTitleIndex();
            if (title >= 0)
                focusTitle(title);
            return;
        }
        var first = delta > 0 ? firstTitleIndex() : lastTitleIndex();
        if (first >= 0) {
            focusTitle(first);
            return;
        }
        moveStatusFocus(delta);
    }

    function activateKeyboardFocus() {
        if (keyboardMenuIndex >= 0) {
            toggleMenu(keyboardMenuIndex);
            return;
        }
        if (keyboardStatusIndex >= 0) {
            activateKeyboardStatus();
            return;
        }
        var first = firstTitleIndex();
        if (first >= 0)
            focusTitle(first);
    }

    // Ctrl+F2 hands the bar the keyboard (the compositor's FocusMenuBar).
    // Start on the first title; a bar with no populated menu falls back to the
    // status row so the keyboard entry is never a dead end.
    function beginKeyboardNavigation() {
        keyboardFocused = true;
        keyboardStatusIndex = -1;
        var first = firstTitleIndex();
        if (first >= 0) {
            focusTitle(first);
        } else {
            keyboardMenuIndex = -1;
            moveStatusFocus(1);
            forceActiveFocus();
        }
    }

    function endKeyboardNavigation() {
        keyboardMenuIndex = -1;
        keyboardStatusIndex = -1;
        // Move focus off a title before its `activeFocusOnTab` binding follows
        // `keyboardFocused` to false; Qt refuses that change while the item is
        // the active focus item.
        for (var i = 0; i < menuRepeater.count; ++i) {
            var item = menuRepeater.itemAt(i);
            if (item && item.activeFocus) {
                menuBar.forceActiveFocus();
                break;
            }
        }
        keyboardFocused = false;
    }

    onKeyboardFocusedChanged: {
        if (!keyboardFocused) {
            keyboardMenuIndex = -1;
            keyboardStatusIndex = -1;
        }
    }

    function activateKeyboardStatus() {
        if (keyboardStatusIndex < 0)
            return;
        var item = statusRepeater.itemAt(keyboardStatusIndex);
        if (!item)
            return;
        activateStatusItem(item.itemId);
    }

    // An open menu session tracks sibling titles on the next event-loop turn.
    // Only submenus need dwell; delaying titles makes horizontal sweeps lag.
    function scheduleMenuSwitch(index) {
        if (openMenuIndex >= 0 && openMenuIndex !== index) {
            hoverMenuIndex = index;
            switchTimer.restart();
        } else {
            switchTimer.stop();
        }
    }

    // Live focus-switch tracking: the open menu follows the newly focused
    // application, and closes if that application exports no menu (FR-3).
    function setFocusedApp(name, id, menuModel) {
        var wasOpen = openMenuIndex;
        var previous = appMenuAt(wasOpen);
        openMenuIndex = -1;
        if (previous)
            previous.closeMenu(true);
        appName = name;
        appId = id;
        appMenuModel = menuModel || [];
        if (wasOpen >= 0) {
            if (wasOpen < topLevelMenus.length)
                openMenu(wasOpen);
            else {
                appMenuClosed();
                popupClosed();
            }
        }
    }

    onShellFocusedChanged: {
        if (!shellFocused) {
            closeMenus();
            closeStatusMenu();
            closeTrayMenu();
            closeOverflowMenu();
            endKeyboardNavigation();
        }
    }

    Timer {
        id: switchTimer
        interval: 0
        repeat: false
        onTriggered: {
            if (menuBar.hoverMenuIndex >= 0 && menuBar.openMenuIndex >= 0
                    && menuBar.hoverMenuIndex !== menuBar.openMenuIndex)
                menuBar.openMenu(menuBar.hoverMenuIndex);
        }
    }

    // Tapping empty bar space dismisses an open menu. Taps that land on an
    // app-menu title are ignored here: that title's own handler toggles the
    // menu, and without this guard the click-away would close it immediately.
    TapHandler {
        onTapped: (eventPoint) => {
            const p = eventPoint.position;
            // Ignore taps on the app-menu row and the status row: those items
            // own their tap (open/toggle), and without this guard the click-
            // away handler would close the surface it just opened.
            if (p.x >= appMenuRow.x && p.x <= appMenuRow.x + appMenuRow.width
                    && p.y >= appMenuRow.y && p.y <= appMenuRow.y + appMenuRow.height)
                return;
            if (p.x >= statusRow.x && p.x <= statusRow.x + statusRow.width
                    && p.y >= statusRow.y && p.y <= statusRow.y + statusRow.height)
                return;
            menuBar.closeMenus();
            menuBar.closeStatusMenu();
            menuBar.closeTrayMenu();
            menuBar.closeOverflowMenu();
        }
    }

    Keys.onEscapePressed: (event) => {
        if (menuBar.keyboardFocused) {
            // Leave keyboard entry and hand the keyboard back to the invoking
            // window (the compositor restores the active window's focus).
            menuBar.keyboardFocusReleaseRequested();
            menuBar.endKeyboardNavigation();
        } else {
            menuBar.closeMenus();
            menuBar.closeStatusMenu();
            menuBar.closeTrayMenu();
            menuBar.closeOverflowMenu();
        }
        event.accepted = true;
    }

    // Keyboard navigation across the title row and the status row (T-09,
    // T-07.5b). Left/Right moves within the focused family and crosses at the
    // edge; Return/Space opens the selected title or status item. A popup that
    // holds focus handles its own keys.
    Keys.onLeftPressed: (event) => {
        if (menuBar.keyboardFocused) {
            menuBar.moveKeyboardFocus(-1);
            event.accepted = true;
        } else if (!menuBar.statusMenuOpen) {
            menuBar.moveStatusFocus(-1);
            event.accepted = true;
        }
    }

    Keys.onRightPressed: (event) => {
        if (menuBar.keyboardFocused) {
            menuBar.moveKeyboardFocus(1);
            event.accepted = true;
        } else if (!menuBar.statusMenuOpen) {
            menuBar.moveStatusFocus(1);
            event.accepted = true;
        }
    }

    Keys.onReturnPressed: (event) => {
        if (menuBar.keyboardFocused) {
            menuBar.activateKeyboardFocus();
            event.accepted = true;
        } else if (!menuBar.statusMenuOpen) {
            menuBar.activateKeyboardStatus();
            event.accepted = true;
        }
    }

    Keys.onSpacePressed: (event) => {
        if (menuBar.keyboardFocused) {
            menuBar.activateKeyboardFocus();
            event.accepted = true;
        } else if (!menuBar.statusMenuOpen) {
            menuBar.activateKeyboardStatus();
            event.accepted = true;
        }
    }

    Keys.onDownPressed: (event) => {
        if (menuBar.keyboardFocused) {
            menuBar.activateKeyboardFocus();
            event.accepted = true;
        }
    }

    Row {
        id: appMenuRow
        objectName: "appMenuRow"
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.menuBar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        height: Theme.controls.menuBarMenu.barHeight

        Repeater {
            id: menuRepeater
            objectName: "menuRepeater"
            // Keep the title and popup alive when its rows refresh. An array
            // model resets every delegate even for a single changed row.
            model: menuBar.topLevelMenus.length
            onItemRemoved: (index, item) => {
                if (menuBar.openMenuIndex === index)
                    menuBar.closeMenus();
            }

            delegate: MenuBarMenu {
                id: menuTitle
                required property int index
                readonly property var modelData: menuBar.topLevelMenus[index] || {}

                // Tab reaches the titles while the bar owns keyboard entry.
                // Outside that mode the offscreen window must not auto-focus
                // the first title and draw its ring on show.
                activeFocusOnTab: menuBar.keyboardFocused
                showFocusRing: menuBar.keyboardFocused

                showLogo: modelData.kind === "system"
                glyph: modelData.kind === "system" ? menuBar.osGlyph : "dragonfruit"
                emphasized: modelData.kind === "app"
                foreground: menuBar.foreground
                textTreatmentEnabled: menuBar.textTreatmentEnabled
                title: modelData.title !== undefined ? modelData.title : ""
                model: modelData.items !== undefined ? modelData.items : []
                hostBounds: menuBar.hostBounds
                materialBlurActive: menuBar.popupMaterialBlurActive
                materialTier: menuBar.popupMaterialTier
                materialFallback: menuBar.popupMaterialFallback

                onTriggered: (itemIndex, item) => menuBar.appMenuTriggered(index, itemIndex, item)
                onMenuSwitchRequested: (delta) => menuBar.moveMenuSelection(index, delta)
                onOpened: {
                    var previous = menuBar.appMenuAt(menuBar.openMenuIndex);
                    menuBar.openMenuIndex = index;
                    if (previous && previous !== menuTitle)
                        previous.closeMenu(true);
                    menuBar.closeStatusMenu(true);
                    menuBar.closeTrayMenu(true);
                    menuBar.closeOverflowMenu(true);
                    popup.forceActiveFocus();
                    menuBar.keyboardMenuIndex = index;
                    menuBar.appMenuOpened(index);
                    menuBar.popupOpened();
                }
                onClosed: {
                    if (menuBar.openMenuIndex === index) {
                        menuBar.openMenuIndex = -1;
                        menuBar.appMenuClosed();
                        menuBar.popupClosed();
                        if (menuBar.keyboardFocused)
                            menuBar.focusTitle(index);
                    }
                }

                HoverHandler {
                    onHoveredChanged: {
                        if (hovered) {
                            menuBar.hoverMenuIndex = index;
                            menuBar.scheduleMenuSwitch(index);
                        } else if (menuBar.hoverMenuIndex === index) {
                            menuBar.hoverMenuIndex = -1;
                            switchTimer.stop();
                        }
                    }
                }
            }
        }
    }

    // One clock source for the whole bar. The visible clock slot and the
    // passive width measurement both read its formatted text; neither owns a
    // timer, so an idle bar has one boundary wakeup instead of two.
    MenuBarClock {
        id: menuClock
        objectName: "menuBarClockSource"
        visible: false
        interactive: false
        showDate: menuBar.showDate
        showSeconds: menuBar.showSeconds
        use24Hour: menuBar.use24Hour
    }

    Row {
        id: statusRow
        objectName: "statusRow"
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.menuBar.paddingH
        anchors.verticalCenter: parent.verticalCenter
        height: Theme.controls.menuBar.height
        spacing: Theme.controls.menuBar.statusItemGap
        visible: menuBar.showStatusItems

        Repeater {
            id: statusRepeater
            objectName: "statusRepeater"
            model: statusRegistry.statusRowItems

            delegate: StatusItem {
                required property var modelData
                required property int index

                keyboardFocus: menuBar.keyboardStatusIndex === index
                itemId: modelData.id !== undefined ? modelData.id : ""
                icon: modelData.icon !== undefined ? modelData.icon : ""
                iconSource: modelData.iconSource !== undefined ? modelData.iconSource : ""
                label: modelData.label !== undefined ? modelData.label : ""
                tooltip: modelData.tooltip !== undefined ? modelData.tooltip : ""
                accessibleName: modelData.accessibleName !== undefined ? modelData.accessibleName : ""
                kind: modelData.kind !== undefined ? modelData.kind : "status"
                state: modelData.state !== undefined ? modelData.state : "ok"
                available: modelData.available !== false
                enabled: modelData.enabled !== false
                selected: modelData.selected === true
                level: modelData.level !== undefined ? modelData.level : 0.8
                showDate: menuBar.showDate
                tint: modelData.tint !== undefined ? modelData.tint : menuBar.foreground
                foreground: menuBar.foreground
                secondaryForeground: menuBar.secondaryForeground
                backgroundColor: menuBar.color
                textTreatmentEnabled: menuBar.textTreatmentEnabled
                clockSource: menuClock

                onActivated: (id) => menuBar.activateStatusItem(id)
            }
        }
    }

    // T233: the passive measurement path. One `StatusItemMeasure` per registry
    // entry reports the same width the visible `StatusItem` will take, without
    // instantiating a second status tree (or a second clock timer). The
    // repeater is invisible and never hit-tested.
    Repeater {
        id: statusMeasureRepeater
        objectName: "statusMeasureRepeater"
        model: statusRegistry.orderedItems

        delegate: StatusItemMeasure {
            required property var modelData

            entry: modelData
            showDate: menuBar.showDate
            clockSource: menuClock
        }
    }

    // The Wi-Fi and volume popovers (T-07.5a). They live in the bar scene,
    // anchored to their status item; the shell commits the union rectangle to
    // the `overlay` popup surface (see `_dropdownRect`).
    WifiMenu {
        id: wifiMenuPopup
        objectName: "wifiMenu"
        model: menuBar.wifiMenu
        anchorItem: menuBar.statusItemFor("wifi")
        materialPreset: "statusMenu"
        materialBackdrop: "trusted"
        materialBlurActive: menuBar.popupMaterialBlurActive
        materialTier: menuBar.popupMaterialTier
        materialFallback: menuBar.popupMaterialFallback
        onJoinRequested: (ssid, secret) => menuBar.wifiJoinRequested(ssid, secret)
        onToggleRequested: (enabled) => menuBar.wifiToggleRequested(enabled)
        onSettingsRequested: menuBar.wifiSettingsRequested()
        onRefreshRequested: menuBar.statusMenuRefreshRequested("wifi")
        onClosed: menuBar.closeStatusMenu()
    }

    VolumeMenu {
        id: volumeMenuPopup
        objectName: "volumeMenu"
        model: menuBar.volumeMenu
        anchorItem: menuBar.statusItemFor("volume")
        materialPreset: "statusMenu"
        materialBackdrop: "trusted"
        materialBlurActive: menuBar.popupMaterialBlurActive
        materialTier: menuBar.popupMaterialTier
        materialFallback: menuBar.popupMaterialFallback
        onVolumeSetRequested: (volume) => menuBar.volumeSetRequested(volume)
        onMuteToggleRequested: menuBar.muteToggleRequested()
        onSinkSelected: (id) => menuBar.sinkSelected(id)
        onSettingsRequested: menuBar.volumeSettingsRequested()
        onRefreshRequested: menuBar.statusMenuRefreshRequested("volume")
        onClosed: menuBar.closeStatusMenu()
    }

    BatteryMenu {
        id: batteryMenuPopup
        objectName: "batteryMenu"
        model: menuBar.batteryMenu
        anchorItem: menuBar.statusItemFor("battery")
        materialPreset: "statusMenu"
        materialBackdrop: "trusted"
        materialBlurActive: menuBar.popupMaterialBlurActive
        materialTier: menuBar.popupMaterialTier
        materialFallback: menuBar.popupMaterialFallback
        onRefreshRequested: menuBar.statusMenuRefreshRequested("battery")
        onPowerProfileSelected: (profile) => menuBar.powerProfileSelected(profile)
        onSettingsRequested: menuBar.batterySettingsRequested()
        onClosed: menuBar.closeStatusMenu()
    }

    BluetoothMenu {
        id: bluetoothMenuPopup
        objectName: "bluetoothMenu"
        model: menuBar.bluetoothMenu
        anchorItem: menuBar.statusItemFor("bluetooth")
        materialPreset: "statusMenu"
        materialBackdrop: "trusted"
        materialBlurActive: menuBar.popupMaterialBlurActive
        materialTier: menuBar.popupMaterialTier
        materialFallback: menuBar.popupMaterialFallback
        onPowerToggleRequested: (enabled) => menuBar.bluetoothPowerToggleRequested(enabled)
        onDeviceToggled: (address, connected) => menuBar.bluetoothDeviceToggled(address, connected)
        onSettingsRequested: menuBar.bluetoothSettingsRequested()
        onDiscoveringToggleRequested: (discovering) => menuBar.bluetoothDiscoveringToggled(discovering)
        onPairRequested: (address) => menuBar.bluetoothPairRequested(address)
        onRefreshRequested: menuBar.statusMenuRefreshRequested("bluetooth")
        onClosed: menuBar.closeStatusMenu()
    }

    // The Date & Time panel (T231): a clock popover, not a menu, using the
    // same status-popover material and Escape dismissal as the other extras.
    DateTimePanel {
        id: dateTimePanel
        objectName: "dateTimePanel"
        anchorItem: menuBar.statusItemFor("clock")
        showSeconds: menuBar.showSeconds
        use24Hour: menuBar.use24Hour
        materialBlurActive: menuBar.popupMaterialBlurActive
        materialTier: menuBar.popupMaterialTier
        materialFallback: menuBar.popupMaterialFallback
        onClosed: menuBar.closeStatusMenu()
    }

    // The overflow affordance (T231). It is a design-system ContextMenu so it
    // shares the tray menu's keyboard, material and dismissal contract.
    ContextMenu {
        id: overflowContextMenu
        objectName: "overflowContextMenu"
        model: menuBar.overflowMenu
        availableRect: menuBar.hostBounds
        accessibleName: qsTr("More menu bar items")
        materialPreset: "menuDetached"
        materialBackdrop: "trusted"
        materialBlurActive: menuBar.popupMaterialBlurActive
        materialTier: menuBar.popupMaterialTier
        materialFallback: menuBar.popupMaterialFallback
        onTriggered: (index, item) => {
            var action = (item && item.action !== undefined) ? item.action : "";
            menuBar.closeOverflowMenu();
            if (action.length > 0)
                menuBar.activateStatusItem(action);
        }
        onClosed: menuBar.closeOverflowMenu()
    }

    // A StatusNotifier tray item's DBusMenu (T-14.3). It reuses the
    // design-system ContextMenu, so tray menus and the Dock's context menus
    // cannot diverge; the shell commits its `contentRect` as the overlay
    // popup rectangle (see `_dropdownRect`).
    ContextMenu {
        id: trayContextMenu
        objectName: "trayContextMenu"
        model: menuBar.trayMenu
        availableRect: menuBar.hostBounds
        accessibleName: qsTr("Tray menu")
        materialPreset: "menuDetached"
        materialBackdrop: "trusted"
        materialBlurActive: menuBar.popupMaterialBlurActive
        materialTier: menuBar.popupMaterialTier
        materialFallback: menuBar.popupMaterialFallback
        onTriggered: (index, item) => {
            var id = (item && item.id !== undefined) ? item.id : -1;
            menuBar.trayMenuTriggered(menuBar.trayMenuService, id);
        }
        onClosed: menuBar.closeTrayMenu()
    }
}
