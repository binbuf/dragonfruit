// SPDX-License-Identifier: MIT
// T195: the shell-side resolver for the generated glass material presets.
//
// `Theme.glass` is the generated vocabulary; this singleton is the one place
// QML turns a preset plus scheme/slider/accessibility/backend/tier into
// explicit optics and a rendering mode. It is deliberately separate from the
// existing `MaterialRole` wire values and `Material.qml`'s popup fill helper:
// presets describe the optical look, while the role/tier state remains the
// compositor transport.
//
// The compositor's `material_preset` module implements the same policy from
// the same generated values. The shared acceptance cases live in
// `tests/glass/test_material_presets.py`.
pragma Singleton
import QtQuick

QtObject {
    id: glass

    // The one internal rollout switch. The shell controller sets this from
    // `DRAGONFRUIT_GLASS_ROLLOUT` for nested/native fixtures; the default is
    // the existing appearance until T208 flips it.
    property string rollout: Theme.glass.rollout.defaultMode
    // The active scheme. Bound to the Theme unless a preview/test sets it.
    property string scheme: Theme.dark ? "dark" : "light"
    // The Appearance transparency slider, 0..1. Light `menuAttached` follows
    // the measured File curve directly; light `menuDetached` and `statusMenu`
    // follow the same curve as a relative adjustment that preserves their
    // default fits. Every other preset is unchanged.
    property real slider: Theme.glass.slider.defaultPosition
    property bool reduceTransparency: false
    property bool increaseContrast: false
    // Backend capability. `software` is headless/no-readback; otherwise
    // `sampledBlur` says whether the sampled pass exists.
    property bool sampledBlur: true
    property bool software: false
    // Degrade tier: "full" | "reduced" | "minimal".
    property string tier: "full"
    // Orthogonal to optics; accepted so callers do not fold motion policy in.
    property bool reduceMotion: Theme.reducedMotion

    readonly property var presetNames: [
        "dock", "menuBarVeil", "menuAttached", "menuDetached", "statusMenu",
        "tooltip", "controlCenterOuter", "controlCenterModule",
        "controlCenterExpanded", "notificationBanner", "notificationStack",
        "notificationLowerPanel", "windowChrome"
    ]

    function _sanitize01(value, fallback) {
        var number = Number(value);
        if (!isFinite(number))
            return fallback;
        return Math.max(0, Math.min(1, number));
    }

    function _mix(neutral, perceptual, weight) {
        var w = _sanitize01(weight, 0);
        return neutral * (1 - w) + perceptual * w;
    }

    // Piecewise-linear interpolation through the three measured light anchors.
    function _interpolate(position) {
        var anchors = Theme.glass.slider.lightAnchors;
        var points = [anchors.low, anchors.mid, anchors.high];
        var p = _sanitize01(position, anchors.mid.position);
        for (var i = 0; i + 1 < points.length; ++i) {
            var left = points[i];
            var right = points[i + 1];
            if (p <= right.position) {
                var span = right.position - left.position;
                var ratio = span <= 0 ? 0 : (p - left.position) / span;
                return {
                    alpha: left.alpha + (right.alpha - left.alpha) * ratio,
                    tint: left.tint + (right.tint - left.tint) * ratio
                };
            }
        }
        var last = points[points.length - 1];
        return { alpha: last.alpha, tint: last.tint };
    }

    // The existing per-role material token pair, used by the legacy look.
    function _legacyValues(role) {
        var material = Theme.material;
        if (role === "dock") {
            return {
                alpha: material.dockOpacity,
                blur: material.dockBlur,
                tint: 1,
                fallback: material.dockFallbackOpacity
            };
        }
        if (role === "popup" || role === "drawer") {
            return {
                alpha: material.popupOpacity,
                blur: material.popupBlur,
                tint: 1,
                fallback: material.popupFallbackOpacity
            };
        }
        return {
            alpha: material.chromeOpacity,
            blur: material.chromeBlur,
            tint: 1,
            fallback: material.chromeFallbackOpacity
        };
    }

    function _opaqueValues(scheme) {
        var entry = Theme.glass.accessibility.reduceTransparency[scheme];
        return { alpha: entry.alpha, tint: entry.tint };
    }

    function _isPersistent(role) {
        return role === "chrome" || role === "dock";
    }

    function _backend(options) {
        if (options.backend !== undefined)
            return String(options.backend);
        if (options.software !== undefined ? options.software : glass.software)
            return "software";
        var sampled = options.sampledBlur !== undefined ? options.sampledBlur : glass.sampledBlur;
        return sampled ? "gpu-sampled" : "gpu-no-sampled-blur";
    }

    // Resolve one preset. The optional `options` object overrides the current
    // property state; this is the same input shape as the Rust resolver.
    function resolve(presetName, options) {
        options = options || {};
        var scheme = options.scheme !== undefined ? String(options.scheme) : glass.scheme;
        if (scheme !== "light" && scheme !== "dark")
            scheme = "light";
        var requestedRollout = options.rollout !== undefined
            ? String(options.rollout) : glass.rollout;
        if (requestedRollout !== "legacy" && requestedRollout !== "replacement")
            requestedRollout = "legacy";
        var backend = _backend(options);
        if (backend !== "gpu-sampled" && backend !== "gpu-no-sampled-blur"
                && backend !== "software")
            backend = "software";
        var tier = options.tier !== undefined ? String(options.tier) : glass.tier;
        if (tier !== "full" && tier !== "reduced" && tier !== "minimal")
            tier = "minimal";
        var reduceTransparency = options.reduceTransparency !== undefined
            ? !!options.reduceTransparency : glass.reduceTransparency;
        var increaseContrast = options.increaseContrast !== undefined
            ? !!options.increaseContrast : glass.increaseContrast;

        var preset = Theme.glass.presets[presetName];
        if (preset === undefined) {
            var safe = _opaqueValues(scheme);
            return {
                preset: presetName,
                scheme: scheme,
                look: "replacement",
                renderMode: "solid",
                alpha: safe.alpha,
                tint: safe.tint,
                blur: 0,
                edge: 0,
                edgeWidth: 0,
                edgeTail: 0,
                sharpFraction: 0,
                cornerLensPeak: 0,
                cornerLensDistance: 0,
                cornerLensTail: 0,
                foreground: 0,
                sliderApplied: false,
                confidence: "baseline",
                fallbackReason: "unknown-preset"
            };
        }

        var role = preset.role;
        var persistent = _isPersistent(role);
        var sliderApplied = false;
        var fallbackReason = "none";
        var confidence = "baseline";
        var alpha;
        var tint;
        var blur;
        var edge;
        var edgeWidth;
        var edgeTail = 0;
        var sharpFraction = 0;
        var cornerLensPeak = 0;
        var cornerLensDistance = 0;
        var cornerLensTail = 0;
        var foreground;

        if (requestedRollout === "legacy") {
            var legacy = _legacyValues(role);
            alpha = legacy.alpha;
            tint = legacy.tint;
            blur = legacy.blur;
            edge = 0;
            edgeWidth = 0;
            foreground = 0;
            fallbackReason = "legacy";
        } else {
            var entry = preset[scheme];
            var calibration = entry.calibration;
            alpha = _mix(calibration.neutralAlpha, calibration.perceptualAlpha,
                         calibration.perceptualWeight);
            tint = _mix(calibration.neutralTint, calibration.perceptualTint,
                        calibration.perceptualWeight);
            blur = entry.blur;
            edge = entry.edge;
            edgeWidth = entry.edgeWidth;
            edgeTail = entry.edgeTail !== undefined ? entry.edgeTail : 0;
            sharpFraction = entry.sharpFraction !== undefined ? entry.sharpFraction : 0;
            cornerLensPeak = entry.cornerLensPeak !== undefined ? entry.cornerLensPeak : 0;
            cornerLensDistance = entry.cornerLensDistance !== undefined
                ? entry.cornerLensDistance : 0;
            cornerLensTail = entry.cornerLensTail !== undefined ? entry.cornerLensTail : 0;
            foreground = entry.foreground;
            confidence = entry.confidence;

            if (preset.slider === "lightAttached" && scheme === "light") {
                var position = _sanitize01(
                    options.slider !== undefined ? options.slider : glass.slider,
                    Theme.glass.slider.defaultPosition);
                var fitted = _interpolate(position);
                alpha = fitted.alpha;
                tint = fitted.tint;
                sliderApplied = true;
            } else if (preset.slider === "lightRelative" && scheme === "light") {
                // The measured curve is the attached File response. Detached
                // and status menus keep their own default fit and move
                // relative to it: alpha scales by the attached ratio and tint
                // takes the attached delta. At the default position the preset
                // is unchanged.
                var relativePosition = _sanitize01(
                    options.slider !== undefined ? options.slider : glass.slider,
                    Theme.glass.slider.defaultPosition);
                var defaultPosition = Theme.glass.slider.defaultPosition;
                var base = _interpolate(defaultPosition);
                var current = _interpolate(relativePosition);
                if (base.alpha > 0)
                    alpha = _sanitize01(alpha * current.alpha / base.alpha, alpha);
                tint = _sanitize01(tint + (current.tint - base.tint), tint);
                sliderApplied = true;
            }
        }

        // Reduce Transparency is the explicit override: it wins over the
        // slider and any sampled path, in both rollout looks.
        if (reduceTransparency) {
            var opaque = _opaqueValues(scheme);
            alpha = opaque.alpha;
            tint = opaque.tint;
            blur = 0;
            sliderApplied = false;
            fallbackReason = "reduce-transparency";
        }

        // Increase Contrast applies after the fill decision so it still
        // strengthens the edge/foreground on an opaque plate.
        if (increaseContrast) {
            var contrast = Theme.glass.accessibility.increaseContrast;
            edge = _sanitize01(edge + contrast.edgeBoost, edge);
            foreground = _sanitize01(foreground + contrast.foregroundBoost, foreground);
            edgeWidth = Math.max(0, edgeWidth + contrast.edgeWidthBoost);
        }

        var renderMode = "sampled-glass";
        if (reduceTransparency) {
            renderMode = "solid";
        } else if (requestedRollout === "legacy") {
            var legacyFallback = _legacyValues(role).fallback;
            if (backend === "software") {
                renderMode = "software-fallback";
                alpha = legacyFallback;
                blur = 0;
                fallbackReason = "software";
            } else if (backend === "gpu-no-sampled-blur") {
                renderMode = "solid";
                alpha = legacyFallback;
                blur = 0;
                fallbackReason = "no-sampled-blur";
            } else if (tier === "minimal" && !persistent) {
                renderMode = "solid";
                alpha = legacyFallback;
                blur = 0;
                fallbackReason = "minimal";
            } else {
                // The persistent floor keeps Chrome/Dock sampled even when
                // the controller visited Minimal (ADR 0203/0211).
                fallbackReason = "none";
            }
        } else {
            var intended = preset.render;
            if (intended === "veil") {
                // An intentional no-blur veil, never a missing sampled pass.
                renderMode = "no-blur-veil";
                blur = 0;
                fallbackReason = "none";
            } else if (intended === "solid") {
                renderMode = "solid";
                blur = 0;
                fallbackReason = "none";
            } else if (backend === "software") {
                var softwareFill = _opaqueValues(scheme);
                renderMode = "software-fallback";
                alpha = softwareFill.alpha;
                tint = softwareFill.tint;
                blur = 0;
                fallbackReason = "software";
            } else if (backend === "gpu-no-sampled-blur") {
                var unsupported = _opaqueValues(scheme);
                renderMode = "solid";
                alpha = unsupported.alpha;
                tint = unsupported.tint;
                blur = 0;
                fallbackReason = "no-sampled-blur";
            } else if (tier === "minimal" && !persistent) {
                var minimalFill = _opaqueValues(scheme);
                renderMode = "solid";
                alpha = minimalFill.alpha;
                tint = minimalFill.tint;
                blur = 0;
                fallbackReason = "minimal";
            } else {
                renderMode = "sampled-glass";
                fallbackReason = "none";
            }
        }

        var veil = (entry && entry.veil !== undefined) ? entry.veil : null;
        return {
            preset: presetName,
            scheme: scheme,
            look: requestedRollout,
            renderMode: renderMode,
            alpha: alpha,
            tint: tint,
            blur: blur,
            edge: edge,
            edgeWidth: edgeWidth,
            edgeTail: edgeTail,
            sharpFraction: sharpFraction,
            cornerLensPeak: cornerLensPeak,
            cornerLensDistance: cornerLensDistance,
            cornerLensTail: cornerLensTail,
            foreground: foreground,
            sliderApplied: sliderApplied,
            confidence: confidence,
            fallbackReason: fallbackReason,
            // T200: the depth LUT and extent travel with the resolved preset
            // so the shell and fixtures never re-derive them.
            veil: veil,
            stripHeight: preset.stripHeight !== undefined ? preset.stripHeight : 0,
            extent: preset.extent !== undefined ? preset.extent : 0,
            reduceMotion: options.reduceMotion !== undefined
                ? !!options.reduceMotion : glass.reduceMotion
        };
    }
}
