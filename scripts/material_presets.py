#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""The glass material-preset vocabulary and its resolution policy (T195).

``design-system/tokens/tokens.json`` carries the data; this module is the
single reference implementation used by the token gate and the preset
self-test. The QML (`design-system/Glass.qml`) and Rust
(`compositor/src/material_preset.rs`) resolvers implement the same policy for
their runtimes; ``tests/glass/test_material_presets.py`` and the two runtime
test suites pin the shared outcomes.

The policy deliberately keeps three things apart:

* **Optical presets** (this module) are per-surface calibration data. They are
  not the existing ``MaterialRole`` wire values or the ``DegradeTier`` ladder.
* **Neutral and perceptual fits** are separate calibration records. Resolution
  mixes them by ``perceptualWeight`` instead of painting both at full strength.
* **Rendering mode** is explicit: ``sampled-glass``, ``no-blur-veil``,
  ``solid`` or ``software-fallback``. A veil is an intentional no-blur surface,
  never the result of a missing sampled path.

Usage::

    python3 scripts/material_presets.py --check
    python3 scripts/material_presets.py resolve --preset dock --scheme light \
        --rollout replacement --backend gpu-sampled
"""
from __future__ import annotations

import argparse
import json
import math
import os
import sys
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parent.parent
SOURCE = REPO / "design-system" / "tokens" / "tokens.json"

PRESET_NAMES = (
    "dock",
    "menuBarVeil",
    "menuAttached",
    "menuDetached",
    "statusMenu",
    "tooltip",
    "controlCenterOuter",
    "controlCenterModule",
    "controlCenterExpanded",
    "controlCenterExpandedOuter",
    "notificationBanner",
    "notificationStack",
    "notificationLowerPanel",
    "windowChrome",
    # T206: first-party app chrome. These are client-alpha presets consumed by
    # QML only; the compositor never resolves them because it never draws the
    # app's sidebar/toolbar.
    "settingsSidebar",
    "settingsToolbar",
    "filesSidebar",
    "filesToolbar",
)
APP_CHROME_PRESETS = (
    "settingsSidebar",
    "settingsToolbar",
    "filesSidebar",
    "filesToolbar",
)
SCHEMES = ("light", "dark")
ROLES = ("chrome", "dock", "popup", "drawer")
RENDER_MODES = ("sampled", "veil", "solid")
SLIDER_MODES = ("none", "lightAttached", "lightRelative")
CONFIDENCES = ("measured", "measured-effective", "observed-luma", "adapted", "baseline")
ROLLOUTS = ("legacy", "replacement")
BACKENDS = ("gpu-sampled", "gpu-no-sampled-blur", "software")
TIERS = ("full", "reduced", "minimal")
RENDER_MODE_NAMES = ("sampled-glass", "no-blur-veil", "solid", "software-fallback")
SLIDER_ANCHOR_POSITIONS = (0.0, 0.5255, 1.0)

# The legacy appearance is the existing per-role material token pair. Keeping
# the mapping here means the replacement look can be resolved next to the old
# one without teaching QML/Rust a second role vocabulary.
LEGACY_ROLE_TOKENS = {
    "chrome": ("chromeOpacity", "chromeBlur", "chromeFallbackOpacity"),
    "dock": ("dockOpacity", "dockBlur", "dockFallbackOpacity"),
    "popup": ("popupOpacity", "popupBlur", "popupFallbackOpacity"),
    "drawer": ("popupOpacity", "popupBlur", "popupFallbackOpacity"),
}

# Roles that keep the persistent material floor (ADR 0203/0211).
PERSISTENT_ROLES = ("chrome", "dock")

# The preset whose optical response follows the measured light File slider
# anchors directly. The detached/status menu presets follow the same measured
# curve as a relative adjustment (their default fits are preserved).
SLIDER_PRESET = "menuAttached"
RELATIVE_SLIDER_PRESETS = ("menuDetached", "statusMenu")


class PresetError(ValueError):
    """A schema or resolution input error with a path-like message."""


def _finite(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _number(node: dict, key: str, path: str, errors: list[str],
            *, low: float | None = None, high: float | None = None) -> float | None:
    value = node.get(key)
    if not _finite(value):
        errors.append(f"{path}.{key}: expected a finite number, got {value!r}")
        return None
    if low is not None and value < low:
        errors.append(f"{path}.{key}: {value} is below {low}")
    if high is not None and value > high:
        errors.append(f"{path}.{key}: {value} is above {high}")
    return float(value)


def _number_list(node: dict, key: str, path: str, errors: list[str],
                 *, length: int, low: float, high: float) -> list[float] | None:
    values = node.get(key)
    if not isinstance(values, list) or len(values) != length:
        errors.append(f"{path}.{key}: expected a {length}-element number list, got {values!r}")
        return None
    result: list[float] = []
    for index, value in enumerate(values):
        if not _finite(value):
            errors.append(f"{path}.{key}[{index}]: expected a finite number, got {value!r}")
            return None
        if value < low or value > high:
            errors.append(f"{path}.{key}[{index}]: {value} is outside [{low}, {high}]")
        result.append(float(value))
    return result


def _string(node: dict, key: str, path: str, errors: list[str], *,
            allowed: tuple[str, ...] | None = None) -> str | None:
    value = node.get(key)
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{path}.{key}: expected a non-empty string, got {value!r}")
        return None
    if allowed is not None and value not in allowed:
        errors.append(f"{path}.{key}: {value!r} is not one of {allowed}")
    return value


def _expect_object(node: Any, path: str, errors: list[str]) -> dict | None:
    if not isinstance(node, dict):
        errors.append(f"{path}: expected an object, got {type(node).__name__}")
        return None
    return node


def _clamp01(value: Any, fallback: float) -> float:
    """Clamp a runtime input; non-finite/absent values take the named fallback."""
    if not _finite(value):
        return fallback
    return max(0.0, min(1.0, float(value)))


def _mix(neutral: float, perceptual: float, weight: float) -> float:
    return neutral * (1.0 - weight) + perceptual * weight


def _interpolate(anchors: dict, position: float) -> tuple[float, float]:
    """Piecewise-linear interpolation through the three measured anchors."""
    points = [anchors[name] for name in ("low", "mid", "high")]
    position = _clamp01(position, points[1]["position"])
    for left, right in zip(points, points[1:]):
        if position <= right["position"]:
            span = right["position"] - left["position"]
            ratio = 0.0 if span <= 0.0 else (position - left["position"]) / span
            alpha = left["alpha"] + (right["alpha"] - left["alpha"]) * ratio
            tint = left["tint"] + (right["tint"] - left["tint"]) * ratio
            return alpha, tint
    last = points[-1]
    return last["alpha"], last["tint"]


def validate(data: dict) -> list[str]:
    """Return every schema/range/finite-value error in the ``glass`` group."""
    errors: list[str] = []
    glass = _expect_object(data.get("glass"), "glass", errors)
    if glass is None:
        return errors

    _string(glass, "version", "glass", errors)
    rollout = _expect_object(glass.get("rollout"), "glass.rollout", errors)
    if rollout is not None:
        _string(rollout, "defaultMode", "glass.rollout", errors, allowed=ROLLOUTS)
        _string(rollout, "legacyMode", "glass.rollout", errors, allowed=ROLLOUTS)
        _string(rollout, "replacementMode", "glass.rollout", errors, allowed=ROLLOUTS)
        _string(rollout, "envVar", "glass.rollout", errors)
        declared = (rollout.get("legacyMode"), rollout.get("replacementMode"))
        if rollout.get("defaultMode") not in declared:
            errors.append(
                "glass.rollout.defaultMode: must match the legacy or replacement mode"
            )

    slider = _expect_object(glass.get("slider"), "glass.slider", errors)
    if slider is not None:
        _number(slider, "defaultPosition", "glass.slider", errors, low=0.0, high=1.0)
        anchors = _expect_object(slider.get("lightAnchors"), "glass.slider.lightAnchors", errors)
        if anchors is not None:
            names = ("low", "mid", "high")
            for name, expected_position in zip(names, SLIDER_ANCHOR_POSITIONS):
                path = f"glass.slider.lightAnchors.{name}"
                anchor = _expect_object(anchors.get(name), path, errors)
                if anchor is None:
                    continue
                _number(anchor, "position", path, errors, low=0.0, high=1.0)
                _number(anchor, "alpha", path, errors, low=0.0, high=1.0)
                _number(anchor, "tint", path, errors, low=0.0, high=1.0)
                if _finite(anchor.get("position")) and abs(anchor["position"] - expected_position) > 1e-9:
                    errors.append(
                        f"{path}.position: {anchor['position']} is not the measured "
                        f"anchor {expected_position}"
                    )

    accessibility = _expect_object(glass.get("accessibility"), "glass.accessibility", errors)
    if accessibility is not None:
        opaque = _expect_object(accessibility.get("reduceTransparency"),
                                "glass.accessibility.reduceTransparency", errors)
        if opaque is not None:
            for scheme in SCHEMES:
                path = f"glass.accessibility.reduceTransparency.{scheme}"
                entry = _expect_object(opaque.get(scheme), path, errors)
                if entry is None:
                    continue
                _number(entry, "alpha", path, errors, low=0.9, high=1.0)
                _number(entry, "tint", path, errors, low=0.0, high=1.0)
                _string(entry, "colourSpace", path, errors, allowed=("srgb",))
        contrast = _expect_object(accessibility.get("increaseContrast"),
                                  "glass.accessibility.increaseContrast", errors)
        if contrast is not None:
            _number(contrast, "edgeBoost", "glass.accessibility.increaseContrast",
                    errors, low=0.0, high=1.0)
            _number(contrast, "foregroundBoost", "glass.accessibility.increaseContrast",
                    errors, low=0.0, high=1.0)
            _number(contrast, "edgeWidthBoost", "glass.accessibility.increaseContrast",
                    errors, low=0.0, high=8.0)

    presets = _expect_object(glass.get("presets"), "glass.presets", errors)
    if presets is None:
        return errors
    missing = [name for name in PRESET_NAMES if name not in presets]
    for name in missing:
        errors.append(f"glass.presets.{name}: missing required preset")

    for name, preset in presets.items():
        path = f"glass.presets.{name}"
        preset = _expect_object(preset, path, errors)
        if preset is None:
            continue
        _string(preset, "role", path, errors, allowed=ROLES)
        _string(preset, "render", path, errors, allowed=RENDER_MODES)
        slider_mode = _string(preset, "slider", path, errors, allowed=SLIDER_MODES)
        if slider_mode not in (None, "none") and name != SLIDER_PRESET \
                and name not in RELATIVE_SLIDER_PRESETS:
            errors.append(
                f"{path}.slider: only {SLIDER_PRESET} and "
                f"{', '.join(RELATIVE_SLIDER_PRESETS)} follow the slider"
            )
        if name == "menuBarVeil":
            strip = _number(preset, "stripHeight", path, errors, low=1.0, high=128.0)
            extent = _number(preset, "extent", path, errors, low=1.0, high=512.0)
            if strip is not None and extent is not None and extent <= strip:
                errors.append(f"{path}.extent: must extend past the {strip} pt strip")
        for scheme in SCHEMES:
            scheme_path = f"{path}.{scheme}"
            entry = _expect_object(preset.get(scheme), scheme_path, errors)
            if entry is None:
                continue
            _string(entry, "confidence", scheme_path, errors, allowed=CONFIDENCES)
            _string(entry, "provenance", scheme_path, errors)
            _string(entry, "colourSpace", scheme_path, errors, allowed=("srgb",))
            calibration = _expect_object(entry.get("calibration"),
                                         f"{scheme_path}.calibration", errors)
            if calibration is not None:
                _number(calibration, "neutralAlpha", f"{scheme_path}.calibration",
                        errors, low=0.0, high=1.0)
                _number(calibration, "neutralTint", f"{scheme_path}.calibration",
                        errors, low=0.0, high=1.0)
                _number(calibration, "perceptualWeight", f"{scheme_path}.calibration",
                        errors, low=0.0, high=1.0)
                _number(calibration, "perceptualAlpha", f"{scheme_path}.calibration",
                        errors, low=0.0, high=1.0)
                _number(calibration, "perceptualTint", f"{scheme_path}.calibration",
                        errors, low=0.0, high=1.0)
            _number(entry, "blur", scheme_path, errors, low=0.0, high=64.0)
            _number(entry, "edge", scheme_path, errors, low=0.0, high=1.0)
            _number(entry, "edgeWidth", scheme_path, errors, low=0.0, high=8.0)
            _number(entry, "foreground", scheme_path, errors, low=0.0, high=1.0)
            # T200: the menu-bar veil is a depth LUT, not a flat alpha. The
            # samples must stay ordered, end at zero, and keep the scheme's
            # veil colour (white in Light, black in Dark).
            if name == "menuBarVeil":
                veil = _expect_object(entry.get("veil"), f"{scheme_path}.veil", errors)
                if veil is not None:
                    _number(veil, "probe", f"{scheme_path}.veil", errors, low=0.0, high=64.0)
                    # T200: the measured background-luminance response. The
                    # Light curve is 1.0/1.06/0.91/0.48/0.0 at
                    # L=0/0.25/0.5/0.78/1.0; Dark is the identity baseline.
                    adapt_luma = _number_list(
                        veil, "adaptLuma", f"{scheme_path}.veil", errors,
                        length=5, low=0.0, high=1.0)
                    adapt_scale = _number_list(
                        veil, "adaptScale", f"{scheme_path}.veil", errors,
                        length=5, low=0.0, high=1.5)
                    if adapt_luma is not None:
                        if abs(adapt_luma[0]) > 1e-9 or abs(adapt_luma[-1] - 1.0) > 1e-9:
                            errors.append(
                                f"{scheme_path}.veil.adaptLuma: must span 0..1")
                        for left, right in zip(adapt_luma, adapt_luma[1:]):
                            if right <= left:
                                errors.append(
                                    f"{scheme_path}.veil.adaptLuma: must be strictly "
                                    f"increasing, got {adapt_luma}")
                                break
                    if adapt_scale is not None:
                        if scheme == "light":
                            if abs(adapt_scale[0] - 1.0) > 1e-9 \
                                    or abs(adapt_scale[-1]) > 1e-9:
                                errors.append(
                                    f"{scheme_path}.veil.adaptScale: must start at "
                                    f"1.0 and end at 0.0")
                        elif adapt_scale[0] <= 0.0 or abs(adapt_scale[-1]) > 1e-9:
                            errors.append(
                                f"{scheme_path}.veil.adaptScale: Dark adaptation "
                                f"must start above zero and end at zero")
                        elif abs(adapt_scale[2] - 0.26026) > 0.01:
                            errors.append(
                                f"{scheme_path}.veil.adaptScale: Dark mid-grey "
                                f"scale must match the coverage capture (0.26026)")
                    previous = 1.0
                    for depth in (5, 15, 25, 35, 50, 75, 90, 95):
                        key = f"alpha{depth}"
                        alpha = _number(veil, key, f"{scheme_path}.veil", errors,
                                        low=0.0, high=1.0)
                        if alpha is None:
                            continue
                        if alpha > previous + 1e-9:
                            errors.append(
                                f"{scheme_path}.veil.{key}: {alpha} rises above the "
                                f"previous sample {previous}"
                            )
                        previous = alpha
                    if _finite(veil.get("alpha95")) and veil["alpha95"] > 1e-6:
                        errors.append(f"{scheme_path}.veil.alpha95: must end at zero")
                calibration = entry.get("calibration") or {}
                tint = calibration.get("neutralTint")
                if _finite(tint):
                    if scheme == "light" and tint < 0.9:
                        errors.append(f"{scheme_path}.calibration.neutralTint: "
                                      "the Light veil must be a white lift")
                    if scheme == "dark" and tint > 0.1:
                        errors.append(f"{scheme_path}.calibration.neutralTint: "
                                      "the Dark veil must be a black wash")
            # T198/T203: the measured flat-band + rim + corner-lens fits. The
            # fields are optional for the other presets (they keep the legacy
            # adaptive tint/rim), but these entries would silently lose the
            # measured checker/rim/capsule-end calibration without them.
            if name in ("dock", "controlCenterOuter", "controlCenterModule",
                        "controlCenterExpanded"):
                _number(entry, "edgeTail", scheme_path, errors, low=0.0, high=16.0)
                _number(entry, "sharpFraction", scheme_path, errors, low=0.0, high=1.0)
                _number(entry, "cornerLensPeak", scheme_path, errors, low=0.0, high=64.0)
                _number(entry, "cornerLensDistance", scheme_path, errors, low=0.1, high=64.0)
                _number(entry, "cornerLensTail", scheme_path, errors, low=1.0, high=128.0)

    # The measured File anchors are the light attached-menu calibration. A
    # mismatch here would silently move the slider midpoint off the measured
    # 0.552/0.993 sample.
    if slider is not None and "menuAttached" in presets:
        anchors = slider.get("lightAnchors") or {}
        mid = anchors.get("mid") or {}
        entry = (presets.get("menuAttached") or {}).get("light") or {}
        calibration = entry.get("calibration") or {}
        for anchor_key, calibration_key in (("alpha", "neutralAlpha"), ("tint", "neutralTint")):
            if _finite(mid.get(anchor_key)) and _finite(calibration.get(calibration_key)):
                if abs(mid[anchor_key] - calibration[calibration_key]) > 1e-9:
                    errors.append(
                        f"glass.presets.menuAttached.light.calibration.{calibration_key}: "
                        f"{calibration[calibration_key]} does not match the measured "
                        f"slider midpoint {mid[anchor_key]}"
                    )

    # Distinct dark attached/detached baselines are a required product
    # decision, not an accident of two equal numbers.
    attached = (presets.get("menuAttached") or {}).get("dark") or {}
    detached = (presets.get("menuDetached") or {}).get("dark") or {}
    a_cal = attached.get("calibration") or {}
    d_cal = detached.get("calibration") or {}
    if all(_finite(value) for value in (
        a_cal.get("neutralAlpha"), a_cal.get("neutralTint"),
        d_cal.get("neutralAlpha"), d_cal.get("neutralTint"),
    )):
        if (abs(a_cal["neutralAlpha"] - d_cal["neutralAlpha"]) < 1e-6
                and abs(a_cal["neutralTint"] - d_cal["neutralTint"]) < 1e-6):
            errors.append(
                "glass.presets: dark menuAttached and menuDetached must have "
                "distinct presets"
            )

    return errors


def load(path: Path = SOURCE) -> dict:
    with path.open() as handle:
        return json.load(handle)


def _legacy_values(data: dict, role: str, scheme: str) -> tuple[float, float, float, float]:
    material = data["semantic"][scheme]["material"]
    opacity_key, blur_key, fallback_key = LEGACY_ROLE_TOKENS[role]
    return (
        float(material[opacity_key]),
        float(material[blur_key]),
        1.0,  # the legacy plate is a neutral surface tone, not a fitted tint
        float(material[fallback_key]),
    )


def _opaque_values(data: dict, scheme: str) -> tuple[float, float]:
    entry = data["glass"]["accessibility"]["reduceTransparency"][scheme]
    return float(entry["alpha"]), float(entry["tint"])


def resolve(
    data: dict,
    preset_name: str,
    *,
    scheme: str = "light",
    slider: Any = None,
    reduce_transparency: bool = False,
    increase_contrast: bool = False,
    backend: str = "gpu-sampled",
    tier: str = "full",
    rollout: str | None = None,
    reduce_motion: bool = False,
) -> dict:
    """Resolve one preset to explicit optics and a rendering mode.

    Precedence, highest first:

    1. Reduce Transparency: opaque scheme plate, sampling/lensing off, and the
       slider is not consulted.
    2. Rollout: ``legacy`` keeps the existing per-role material; ``replacement``
       uses the optical preset.
    3. The supported light slider anchors (``menuAttached`` only); every other
       preset is unchanged by the slider.
    4. Increase Contrast: adds edge/foreground contrast after the fill decision,
       so it still applies with Reduce Transparency.
    5. Backend/tier: selects ``sampled-glass``, ``no-blur-veil``, ``solid`` or
       ``software-fallback``. A persistent role keeps its sampled floor while
       the GPU path exists.
    """
    if scheme not in SCHEMES:
        raise PresetError(f"unknown scheme {scheme!r}")
    if backend not in BACKENDS:
        backend = "software"
    if tier not in TIERS:
        tier = "minimal"
    default_rollout = (
        data.get("glass", {}).get("rollout", {}).get("defaultMode", "replacement")
    )
    if default_rollout not in ROLLOUTS:
        default_rollout = "replacement"
    if rollout is None:
        rollout = default_rollout
    if rollout not in ROLLOUTS:
        rollout = default_rollout

    glass = data.get("glass", {})
    presets = glass.get("presets", {})
    preset = presets.get(preset_name)
    reduce_motion = bool(reduce_motion)  # deliberately no effect on optics

    if preset is None:
        alpha, tint = _opaque_values(data, scheme)
        return {
            "preset": preset_name,
            "scheme": scheme,
            "look": "replacement",
            "renderMode": "solid",
            "alpha": alpha,
            "tint": tint,
            "blur": 0.0,
            "edge": 0.0,
            "edgeWidth": 0.0,
            "edgeTail": 0.0,
            "sharpFraction": 0.0,
            "cornerLensPeak": 0.0,
            "cornerLensDistance": 0.0,
            "cornerLensTail": 0.0,
            "foreground": 0.0,
            "sliderApplied": False,
            "confidence": "baseline",
            "fallbackReason": "unknown-preset",
        }

    role = preset.get("role", "popup")
    persistent = role in PERSISTENT_ROLES
    confidence = "baseline"
    fallback_reason = "none"
    slider_applied = False

    if rollout == "legacy":
        alpha, blur, tint, fallback_alpha = _legacy_values(data, role, scheme)
        edge = 0.0
        edge_width = 0.0
        foreground = 0.0
        edge_tail = 0.0
        sharp_fraction = 0.0
        corner_lens_peak = 0.0
        corner_lens_distance = 0.0
        corner_lens_tail = 0.0
        confidence = "baseline"
        fallback_reason = "legacy"
    else:
        entry = preset[scheme]
        calibration = entry["calibration"]
        alpha = _mix(calibration["neutralAlpha"], calibration["perceptualAlpha"],
                     calibration["perceptualWeight"])
        tint = _mix(calibration["neutralTint"], calibration["perceptualTint"],
                    calibration["perceptualWeight"])
        blur = float(entry["blur"])
        edge = float(entry["edge"])
        edge_width = float(entry["edgeWidth"])
        foreground = float(entry["foreground"])
        edge_tail = float(entry.get("edgeTail", 0.0))
        sharp_fraction = float(entry.get("sharpFraction", 0.0))
        corner_lens_peak = float(entry.get("cornerLensPeak", 0.0))
        corner_lens_distance = float(entry.get("cornerLensDistance", 0.0))
        corner_lens_tail = float(entry.get("cornerLensTail", 0.0))
        confidence = entry["confidence"]
        fallback_alpha = _opaque_values(data, scheme)[0]

        if preset.get("slider") == "lightAttached" and scheme == "light":
            position = _clamp01(slider, float(glass["slider"]["defaultPosition"]))
            alpha, tint = _interpolate(glass["slider"]["lightAnchors"], position)
            slider_applied = True
        elif preset.get("slider") == "lightRelative" and scheme == "light":
            # The measured curve is the attached File response. Other light
            # menu presets keep their own default fit and move relative to it:
            # alpha scales by the attached alpha ratio, tint takes the attached
            # tint delta. At the default position the preset is unchanged.
            anchors = glass["slider"]["lightAnchors"]
            default_position = float(glass["slider"]["defaultPosition"])
            position = _clamp01(slider, default_position)
            base_alpha, base_tint = _interpolate(anchors, default_position)
            current_alpha, current_tint = _interpolate(anchors, position)
            if base_alpha > 0.0:
                alpha = _clamp01(alpha * current_alpha / base_alpha, alpha)
            tint = _clamp01(tint + (current_tint - base_tint), tint)
            slider_applied = True

    # Reduce Transparency is the explicit override: it wins over the slider and
    # any sampled path, in both rollout looks, and uses the scheme's opaque
    # plate rather than a translucent preset alpha.
    if reduce_transparency:
        alpha, tint = _opaque_values(data, scheme)
        blur = 0.0
        slider_applied = False
        fallback_reason = "reduce-transparency"

    # Increase Contrast is applied after the fill decision so it strengthens
    # the edge/foreground even when the plate is already opaque.
    if increase_contrast:
        contrast = glass["accessibility"]["increaseContrast"]
        edge = _clamp01(edge + float(contrast["edgeBoost"]), edge)
        foreground = _clamp01(foreground + float(contrast["foregroundBoost"]), foreground)
        edge_width = max(0.0, edge_width + float(contrast["edgeWidthBoost"]))

    render_mode = "sampled-glass"
    if reduce_transparency:
        render_mode = "solid"
    elif rollout == "legacy":
        if backend == "software":
            render_mode = "software-fallback"
            alpha = fallback_alpha
            blur = 0.0
            fallback_reason = "software"
        elif backend == "gpu-no-sampled-blur":
            render_mode = "solid"
            alpha = fallback_alpha
            blur = 0.0
            fallback_reason = "no-sampled-blur"
        elif tier == "minimal" and not persistent:
            render_mode = "solid"
            alpha = fallback_alpha
            blur = 0.0
            fallback_reason = "minimal"
        else:
            # The persistent floor keeps Chrome/Dock sampled even when the
            # controller visited Minimal (ADR 0203/0211).
            fallback_reason = "none"
    else:
        intended = preset.get("render", "sampled")
        if intended == "veil":
            # A veil is an intentional no-blur surface. It must never be
            # reported as a missing sampled pass.
            render_mode = "no-blur-veil"
            blur = 0.0
            fallback_reason = "none"
        elif intended == "solid":
            render_mode = "solid"
            blur = 0.0
            fallback_reason = "none"
        elif backend == "software":
            render_mode = "software-fallback"
            alpha, tint = _opaque_values(data, scheme)
            blur = 0.0
            fallback_reason = "software"
        elif backend == "gpu-no-sampled-blur":
            render_mode = "solid"
            alpha, tint = _opaque_values(data, scheme)
            blur = 0.0
            fallback_reason = "no-sampled-blur"
        elif tier == "minimal" and not persistent:
            render_mode = "solid"
            alpha, tint = _opaque_values(data, scheme)
            blur = 0.0
            fallback_reason = "minimal"
        else:
            render_mode = "sampled-glass"
            fallback_reason = "none"

    return {
        "preset": preset_name,
        "scheme": scheme,
        "look": "legacy" if rollout == "legacy" else "replacement",
        "renderMode": render_mode,
        "alpha": alpha,
        "tint": tint,
        "blur": blur,
        "edge": edge,
        "edgeWidth": edge_width,
        "edgeTail": edge_tail,
        "sharpFraction": sharp_fraction,
        "cornerLensPeak": corner_lens_peak,
        "cornerLensDistance": corner_lens_distance,
        "cornerLensTail": corner_lens_tail,
        "foreground": foreground,
        "sliderApplied": slider_applied,
        "confidence": confidence,
        "fallbackReason": fallback_reason,
        # T200: the veil depth LUT travels with the resolution so the shell,
        # compositor diagnostic and fixtures read the same samples.
        "veil": (preset.get(scheme, {}).get("veil")
                 if isinstance(preset, dict) else None),
        "stripHeight": preset.get("stripHeight") if isinstance(preset, dict) else None,
        "extent": preset.get("extent") if isinstance(preset, dict) else None,
        # Orthogonal by construction: accepted so a caller cannot accidentally
        # fold motion policy into the optical resolution.
        "reduceMotion": bool(reduce_motion),
    }


def rollout_from_env(env: dict[str, str] | None = None) -> str:
    """The one internal rollout switch, shared by nested and native fixtures."""
    values = os.environ if env is None else env
    default = load().get("glass", {}).get("rollout", {}).get("defaultMode", "replacement")
    if default not in ROLLOUTS:
        default = "replacement"
    requested = str(values.get("DRAGONFRUIT_GLASS_ROLLOUT", default)).strip().lower()
    return requested if requested in ROLLOUTS else default


def self_test() -> list[str]:
    """Behavioural checks over the checked-in token data."""
    failures: list[str] = []
    data = load()
    errors = validate(data)
    failures.extend(errors)

    def check(condition: bool, message: str) -> None:
        if not condition:
            failures.append(message)

    def near(actual: float, expected: float, message: str, tolerance: float = 1e-9) -> None:
        check(abs(actual - expected) <= tolerance, f"{message}: {actual!r} != {expected!r}")

    if errors:
        return failures

    # Light slider anchors: exact endpoints and the measured midpoint.
    for position, expected_alpha, expected_tint in (
        (0.0, 0.732, 0.953),
        (0.5255, 0.552, 0.993),
        (1.0, 0.415, 0.997),
    ):
        result = resolve(data, "menuAttached", scheme="light", slider=position,
                         rollout="replacement")
        near(result["alpha"], expected_alpha, f"light slider alpha at {position}")
        near(result["tint"], expected_tint, f"light slider tint at {position}")
        check(result["sliderApplied"], f"slider applied at {position}")

    # Piecewise interpolation is monotone through the midpoint.
    quarter = resolve(data, "menuAttached", scheme="light", slider=0.25,
                      rollout="replacement")
    check(0.552 < quarter["alpha"] < 0.732, "light slider interpolates below the midpoint")

    # Unmeasured slider roles are unchanged by the slider. The detached and
    # status menus follow the measured curve by the documented relative
    # adjustment and preserve their default fits at the default position.
    for name in PRESET_NAMES:
        if name == SLIDER_PRESET or name in RELATIVE_SLIDER_PRESETS:
            continue
        low = resolve(data, name, scheme="light", slider=0.0, rollout="replacement")
        high = resolve(data, name, scheme="light", slider=1.0, rollout="replacement")
        check(not low["sliderApplied"] and not high["sliderApplied"],
              f"{name}: slider must not apply")
        near(low["alpha"], high["alpha"], f"{name}: slider changed alpha")
        near(low["tint"], high["tint"], f"{name}: slider changed tint")

    for name in RELATIVE_SLIDER_PRESETS:
        default = resolve(data, name, scheme="light", rollout="replacement")
        default_position = float(data["glass"]["slider"]["defaultPosition"])
        explicit = resolve(data, name, scheme="light", slider=default_position,
                           rollout="replacement")
        near(default["alpha"], explicit["alpha"],
             f"{name}: relative default alpha must be unchanged")
        near(default["tint"], explicit["tint"],
             f"{name}: relative default tint must be unchanged")
        low = resolve(data, name, scheme="light", slider=0.0, rollout="replacement")
        high = resolve(data, name, scheme="light", slider=1.0, rollout="replacement")
        check(low["sliderApplied"] and high["sliderApplied"],
              f"{name}: relative slider must be recorded")
        check(low["alpha"] > default["alpha"] > high["alpha"],
              f"{name}: relative alpha must follow the attached curve")
        check(low["tint"] < default["tint"] < high["tint"],
              f"{name}: relative tint must follow the attached curve")
        dark = resolve(data, name, scheme="dark", slider=0.0, rollout="replacement")
        dark_high = resolve(data, name, scheme="dark", slider=1.0, rollout="replacement")
        check(not dark["sliderApplied"] and not dark_high["sliderApplied"],
              f"{name}: dark must not follow the light slider")
        near(dark["alpha"], dark_high["alpha"], f"{name}: dark slider alpha")

    # Dark is a baseline, not a light-slider response.
    dark_low = resolve(data, "menuAttached", scheme="dark", slider=0.0, rollout="replacement")
    dark_high = resolve(data, "menuAttached", scheme="dark", slider=1.0, rollout="replacement")
    check(not dark_low["sliderApplied"] and not dark_high["sliderApplied"],
          "dark menuAttached must not follow the light slider")
    near(dark_low["alpha"], dark_high["alpha"], "dark menuAttached slider alpha")

    # Dark attached and detached are distinct presets.
    attached = resolve(data, "menuAttached", scheme="dark", rollout="replacement")
    detached = resolve(data, "menuDetached", scheme="dark", rollout="replacement")
    check((attached["alpha"], attached["tint"]) != (detached["alpha"], detached["tint"]),
          "dark attached and detached presets must be distinct")

    # Reduce Transparency wins over the slider and disables sampling/lensing.
    reduced = resolve(data, "menuAttached", scheme="light", slider=0.0,
                      reduce_transparency=True, rollout="replacement")
    check(reduced["renderMode"] == "solid", "Reduce Transparency must be solid")
    check(not reduced["sliderApplied"], "Reduce Transparency must skip the slider")
    check(reduced["alpha"] >= 0.9, "Reduce Transparency must be opaque")
    check(reduced["blur"] == 0.0, "Reduce Transparency must disable sampling")

    # Increase Contrast still applies with Reduce Transparency.
    contrasted = resolve(data, "menuAttached", scheme="light", slider=0.0,
                         reduce_transparency=True, increase_contrast=True,
                         rollout="replacement")
    check(contrasted["edge"] > reduced["edge"],
          "Increase Contrast must strengthen the edge with Reduce Transparency")
    check(contrasted["foreground"] > reduced["foreground"],
          "Increase Contrast must strengthen the foreground with Reduce Transparency")

    # The veil is intentional no-blur, not an unsupported sampled path.
    veil = resolve(data, "menuBarVeil", scheme="light", rollout="replacement",
                   backend="gpu-sampled")
    check(veil["renderMode"] == "no-blur-veil", "the menu-bar veil must be no-blur-veil")
    check(veil["fallbackReason"] == "none", "the veil must not report a fallback")
    check(veil["blur"] == 0.0, "the veil must not request sampling")
    unsupported = resolve(data, "menuBarVeil", scheme="light", rollout="replacement",
                          backend="gpu-no-sampled-blur")
    check(unsupported["renderMode"] == "no-blur-veil",
          "an unsupported backend must not turn the veil into a sampled fallback")

    # Unsupported sampled paths are safe and explicit.
    for backend, expected in (("gpu-no-sampled-blur", "solid"),
                              ("software", "software-fallback")):
        result = resolve(data, "menuDetached", scheme="dark", rollout="replacement",
                         backend=backend)
        check(result["renderMode"] == expected,
              f"{backend} must resolve to {expected}")
        check(result["alpha"] >= 0.9, f"{backend} must use an opaque plate")
        check(result["blur"] == 0.0, f"{backend} must not request sampling")

    # Unknown presets take the safe solid path rather than raising.
    unknown = resolve(data, "not-a-preset", scheme="light", rollout="replacement")
    check(unknown["renderMode"] == "solid", "unknown preset must be solid")
    check(unknown["fallbackReason"] == "unknown-preset",
          "unknown preset must name its reason")

    # T206: first-party app chrome is near-opaque client alpha, not a sampled
    # pass. The measured Light Settings/Files fits and the measured Dark Files
    # fits are pinned; Settings Dark is an explicit adaptation from Finder.
    app_targets = {
        ("settingsSidebar", "light"): (0.986, 0.857, "measured"),
        ("settingsToolbar", "light"): (0.961, 0.965, "measured"),
        ("settingsSidebar", "dark"): (0.935, 0.218, "adapted"),
        ("settingsToolbar", "dark"): (0.900, 0.247, "adapted"),
        ("filesSidebar", "light"): (0.985, 0.858, "measured"),
        ("filesToolbar", "light"): (0.994, 0.922, "measured"),
        ("filesSidebar", "dark"): (0.935, 0.218, "measured"),
        ("filesToolbar", "dark"): (0.900, 0.247, "measured"),
    }
    for (name, scheme), (alpha, tint, confidence) in app_targets.items():
        result = resolve(data, name, scheme=scheme, rollout="replacement")
        near(result["alpha"], alpha, f"{name} {scheme} alpha")
        near(result["tint"], tint, f"{name} {scheme} tint")
        check(result["renderMode"] == "solid",
              f"{name} {scheme} must be an app-local solid preset")
        check(result["blur"] == 0.0, f"{name} {scheme} must not request sampling")
        check(result["confidence"] == confidence,
              f"{name} {scheme} confidence must be {confidence}")
        check(result["alpha"] >= 0.9,
              f"{name} {scheme} must stay near-opaque")
        default = resolve(data, name, scheme=scheme)
        check(default["look"] == "replacement",
              f"{name} {scheme} must use the replacement look by default")
        legacy = resolve(data, name, scheme=scheme, rollout="legacy")
        check(legacy["look"] == "legacy",
              f"{name} {scheme} must keep the legacy look selectable")
        reduced = resolve(data, name, scheme=scheme, reduce_transparency=True,
                          rollout="replacement")
        check(reduced["renderMode"] == "solid" and reduced["alpha"] >= 0.9,
              f"{name} {scheme} Reduce Transparency must use the opaque plate")
    # The two Files surfaces stay independent in both schemes.
    for scheme in ("light", "dark"):
        sidebar = resolve(data, "filesSidebar", scheme=scheme, rollout="replacement")
        toolbar = resolve(data, "filesToolbar", scheme=scheme, rollout="replacement")
        check((sidebar["alpha"], sidebar["tint"])
              != (toolbar["alpha"], toolbar["tint"]),
              f"Files {scheme} sidebar and toolbar must be independent presets")

    # The persistent Dock floor keeps sampling when the controller visits
    # Minimal; transient popups may become solid.
    dock = resolve(data, "dock", scheme="light", rollout="replacement", tier="minimal")
    check(dock["renderMode"] == "sampled-glass", "the Dock must keep its sampled floor")
    popup = resolve(data, "menuDetached", scheme="light", rollout="replacement",
                    tier="minimal")
    check(popup["renderMode"] == "solid", "a transient popup may reach solid")

    # Reduce Motion is orthogonal to optics.
    moving = resolve(data, "dock", scheme="light", rollout="replacement",
                     reduce_motion=False)
    still = resolve(data, "dock", scheme="light", rollout="replacement",
                    reduce_motion=True)
    check((moving["alpha"], moving["renderMode"]) == (still["alpha"], still["renderMode"]),
          "Reduce Motion must not change the optics")

    # The rollout switch defaults to the replacement appearance and keeps the
    # legacy rail selectable for A/B and rollback.
    replacement = resolve(data, "dock", scheme="light")
    check(replacement["look"] == "replacement",
          "the default rollout must be replacement")
    check(replacement["renderMode"] == "sampled-glass",
          "replacement default keeps sampled glass")
    legacy = resolve(data, "dock", scheme="light", rollout="legacy")
    check(legacy["look"] == "legacy", "the legacy rollout stays selectable")
    check(legacy["renderMode"] == "sampled-glass", "legacy sampled appearance")
    check(rollout_from_env({}) == "replacement",
          "unset rollout defaults to replacement")
    check(rollout_from_env({"DRAGONFRUIT_GLASS_ROLLOUT": "legacy"}) == "legacy",
          "legacy rollout is selectable")
    check(rollout_from_env({"DRAGONFRUIT_GLASS_ROLLOUT": "bogus"}) == "replacement",
          "unknown rollout falls back to the default")

    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="validate the token schema and run the preset policy checks")
    sub = parser.add_subparsers(dest="command")
    resolve_parser = sub.add_parser("resolve", help="print one resolved preset")
    resolve_parser.add_argument("--preset", required=True, choices=PRESET_NAMES)
    resolve_parser.add_argument("--scheme", default="light", choices=SCHEMES)
    resolve_parser.add_argument("--slider", type=float)
    resolve_parser.add_argument("--reduce-transparency", action="store_true")
    resolve_parser.add_argument("--increase-contrast", action="store_true")
    resolve_parser.add_argument("--backend", default="gpu-sampled", choices=BACKENDS)
    resolve_parser.add_argument("--tier", default="full", choices=TIERS)
    resolve_parser.add_argument("--rollout", choices=ROLLOUTS)
    args = parser.parse_args(argv)

    data = load()
    if args.check:
        failures = self_test()
        if failures:
            for failure in failures:
                print(f"FAIL  {failure}", file=sys.stderr)
            print(f"{len(failures)} material-preset checks failed", file=sys.stderr)
            return 1
        print("material presets: schema and resolution checks pass")
        return 0
    if args.command == "resolve":
        result = resolve(
            data, args.preset, scheme=args.scheme, slider=args.slider,
            reduce_transparency=args.reduce_transparency,
            increase_contrast=args.increase_contrast,
            backend=args.backend, tier=args.tier, rollout=args.rollout,
        )
        print(json.dumps(result, sort_keys=True))
        return 0
    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
