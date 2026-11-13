// SPDX-License-Identifier: MIT
// The popup-family material view (T-21.2c). The compositor owns the degrade
// policy and the shell's `MaterialPolicy` turns its role state into the
// `materialBlurActive` / `materialTier` / `materialFallback` properties on
// each surface. This singleton is only the visual complement: the sampled-glass
// scrim when blur runs, and the token-driven milky plate when it does not.
// No surface re-derives the tier or invents a second neutral.
pragma Singleton
import QtQuick

QtObject {
    id: material

    // The wire values from `df_output.material_role_state.fallback`.
    readonly property var fallbackNames: [
        "none", "no-sampled-blur", "software", "multi-gpu", "floor-missed"
    ]

    // The recorded reason a surface is not on the sampled pass. An explicit
    // fallback wins; otherwise a blur-off tier is recorded as `minimal` (the
    // transient role's bottom rung) or `blur-off` for a malformed state.
    function fallbackReason(fallback, blurActive, tier) {
        if (fallback > 0 && fallback < material.fallbackNames.length)
            return material.fallbackNames[fallback];
        if (blurActive)
            return "none";
        return tier >= 2 ? "minimal" : "blur-off";
    }

    // Menus, Dock popovers, Control Center, notifications, and the OSD share
    // one popup material. The fallback fill is the existing milky chrome
    // neutral (R4.4's source of truth) at the popup fallback alpha, so the
    // surface stays a deliberate plate rather than a translucent slab.
    function popupFill(blurActive) {
        return blurActive
            ? Qt.rgba(Theme.color.surfaceElevated.r, Theme.color.surfaceElevated.g,
                      Theme.color.surfaceElevated.b, Theme.material.popupOpacity)
            : Qt.rgba(Theme.color.chromeFallbackFill.r, Theme.color.chromeFallbackFill.g,
                      Theme.color.chromeFallbackFill.b, Theme.material.popupFallbackOpacity);
    }


    // T202: the menu family uses the shared optical presets. `preset` is one
    // of `menuAttached`, `menuDetached` or `statusMenu`; `trusted` says the
    // surface has the compositor's sampled backdrop behind it. App-owned
    // surfaces (Files content and Select) pass false and get the
    // deterministic opaque plate, never a translucent slab over unfrosted
    // content. The legacy rollout keeps the existing `popupFill` values.
    function _menuTierName(tier) {
        return tier >= 2 ? "minimal" : (tier === 1 ? "reduced" : "full");
    }

    function _menuBackend(blurActive, fallback, trusted) {
        if (!trusted || fallback === 2)
            return "software";
        return blurActive ? "gpu-sampled" : "gpu-no-sampled-blur";
    }

    function menuResolution(preset, blurActive, tier, fallback, trusted) {
        if (Glass.rollout !== "replacement")
            return null;
        return Glass.resolve(preset, {
            scheme: Theme.dark ? "dark" : "light",
            rollout: "replacement",
            slider: Glass.slider,
            reduceTransparency: Glass.reduceTransparency,
            increaseContrast: Glass.increaseContrast,
            backend: _menuBackend(blurActive, fallback, trusted),
            tier: _menuTierName(tier)
        });
    }

    function menuFill(preset, blurActive, tier, fallback, trusted) {
        var resolved = menuResolution(preset, blurActive, tier, fallback, trusted);
        if (resolved === null)
            return popupFill(blurActive);
        // The compositor supplies only the frosted backdrop for menus. Keep
        // tint with the content so opening/closing cannot flash a bare plate.
        return Qt.rgba(resolved.tint, resolved.tint, resolved.tint,
                       resolved.alpha);
    }

    function menuRadius(preset) {
        if (Glass.rollout === "replacement" && preset === "menuAttached")
            return Theme.controls.contextMenu.attachedRadius;
        return Theme.controls.contextMenu.radius;
    }

    // The desktop is an alpha-composited surface, but cannot sample the
    // compositor's scene. Preserve some wallpaper transmission without
    // claiming it is blurred glass. App content menus retain the solid plate.
    function desktopMenuFill(blurActive, tier, fallback) {
        if (!Glass.appChromeSupported || Glass.software || !blurActive)
            return menuFill("menuDetached", false, tier, fallback, false);
        var resolved = menuResolution("menuDetached", true, tier, fallback, true);
        if (resolved === null)
            return popupFill(true);
        return Qt.rgba(resolved.tint, resolved.tint, resolved.tint, resolved.alpha);
    }

    function menuCloseScale(preset) {
        return Theme.reducedMotion ? 1.0 : Theme.controls.contextMenu.closeScale;
    }

    // T206: first-party app chrome (the Settings/Files sidebar and toolbar).
    // These surfaces are client-drawn on a transparent window: the app's own
    // alpha is composited by the compositor over the normal lower scene, so the
    // measured near-opaque transmission needs no sampled compositor pass. When
    // the client surface has no alpha/composition path, the app keeps its
    // existing opaque scheme plate instead. Reduce Transparency resolves to
    // the shared opaque accessibility plate through the one resolver.
    function appChromeResolution(preset) {
        if (Glass.rollout !== "replacement")
            return null;
        return Glass.resolve(preset, {
            scheme: Theme.dark ? "dark" : "light",
            rollout: "replacement",
            slider: Glass.slider,
            reduceTransparency: Glass.reduceTransparency,
            increaseContrast: Glass.increaseContrast,
            backend: Glass.software ? "software" : "gpu-sampled",
            tier: Glass.tier
        });
    }

    function appChromeFill(preset, fallback) {
        if (!Glass.appChromeSupported)
            return fallback;
        var resolved = appChromeResolution(preset);
        if (resolved === null)
            return fallback;
        return Qt.rgba(resolved.tint, resolved.tint, resolved.tint,
                       resolved.alpha);
    }

    // Increase Contrast raises the separator/edge ink on the app chrome. The
    // normal path keeps the design-system border token; focus itself remains a
    // controls/text state and never changes the material.
    function appChromeBorder(preset, fallback) {
        if (!Glass.increaseContrast)
            return fallback;
        var resolved = appChromeResolution(preset);
        if (resolved === null || resolved.edge <= 0)
            return fallback;
        var ink = resolved.tint > 0.5 ? 0.0 : 1.0;
        return Qt.rgba(ink, ink, ink, Math.min(1, resolved.edge));
    }

    // The Applications drawer's card keeps its own translucent surface tone on
    // the legacy path; without a sampled pass it takes the same milky plate as
    // the popup family. The light card is a large browsing surface and reads a
    // little clearer than a menu; the dark card needs a little more tint to
    // keep 13 pt labels legible over a bright wallpaper.
    function drawerOpacity() {
        return Theme.dark ? Theme.controls.appsDrawer.panelOpacityDark
                          : Theme.controls.appsDrawer.panelOpacity;
    }

    function drawerResolution(blurActive, tier, fallback) {
        return menuResolution("appsDrawer", blurActive, tier, fallback, true);
    }

    // True when the compositor's sampled drawer glass is drawing the card.
    // The card fill then stays transparent: the compositor owns the blur,
    // edge lens, specular rim and adaptive tint in one place, and QML owns
    // only the content and the fallback plate.
    function drawerUsesCompositor(blurActive, tier, fallback) {
        var resolved = drawerResolution(blurActive, tier, fallback);
        return resolved !== null && resolved.renderMode === "sampled-glass";
    }

    function drawerFill(blurActive, tier, fallback) {
        if (Glass.rollout !== "replacement") {
            return blurActive
                ? Qt.rgba(Theme.color.surface.r, Theme.color.surface.g,
                          Theme.color.surface.b, drawerOpacity())
                : Qt.rgba(Theme.color.chromeFallbackFill.r, Theme.color.chromeFallbackFill.g,
                          Theme.color.chromeFallbackFill.b,
                          Theme.controls.appsDrawer.panelFallbackOpacity);
        }
        var resolved = drawerResolution(blurActive, tier, fallback);
        if (resolved === null) {
            return Qt.rgba(Theme.color.chromeFallbackFill.r, Theme.color.chromeFallbackFill.g,
                           Theme.color.chromeFallbackFill.b,
                           Theme.controls.appsDrawer.panelFallbackOpacity);
        }
        // The sampled material is compositor-owned; an opaque QML fill would
        // hide the blur/rim/lens behind it.
        if (resolved.renderMode === "sampled-glass")
            return Qt.rgba(0, 0, 0, 0);
        // Reduce Transparency, no sampled blur, software and Minimal keep the
        // readable resolved plate.
        return Qt.rgba(resolved.tint, resolved.tint, resolved.tint, resolved.alpha);
    }
}
