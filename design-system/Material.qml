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
    // surfaces (Files, the desktop, Select) pass false and get the
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
        return Qt.rgba(resolved.tint, resolved.tint, resolved.tint,
                       resolved.alpha);
    }

    function menuRadius(preset) {
        if (Glass.rollout === "replacement" && preset === "menuAttached")
            return Theme.controls.contextMenu.attachedRadius;
        return Theme.controls.contextMenu.radius;
    }

    function menuCloseScale(preset) {
        return Theme.reducedMotion ? 1.0 : Theme.controls.contextMenu.closeScale;
    }

    // The Applications drawer's card keeps its own translucent surface tone on
    // the sampled path; without the pass it takes the same milky plate as the
    // popup family.
    function drawerFill(blurActive) {
        return blurActive
            ? Qt.rgba(Theme.color.surface.r, Theme.color.surface.g,
                      Theme.color.surface.b, Theme.controls.appsDrawer.panelOpacity)
            : Qt.rgba(Theme.color.chromeFallbackFill.r, Theme.color.chromeFallbackFill.g,
                      Theme.color.chromeFallbackFill.b, Theme.material.popupFallbackOpacity);
    }
}
