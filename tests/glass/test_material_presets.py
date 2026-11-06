#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T195 acceptance checks for the glass preset schema and resolution policy.

Run with ``make glass-presets-check`` (also part of ``make gates``). The same
cases are pinned by ``compositor/tests/material_presets.rs`` and the QML
``test_glass_*`` functions, so the three runtimes cannot drift silently.
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO / "scripts"))

import material_presets as presets  # noqa: E402


class Checks:
    def __init__(self) -> None:
        self.passed = 0
        self.failed = 0

    def check(self, condition: bool, message: str) -> None:
        if condition:
            self.passed += 1
        else:
            self.failed += 1
            print(f"FAIL  {message}", file=sys.stderr)

    def near(self, actual: float, expected: float, message: str,
             tolerance: float = 1e-9) -> None:
        self.check(abs(actual - expected) <= tolerance,
                   f"{message}: {actual!r} != {expected!r}")

    def report(self) -> int:
        print(f"{self.passed} passed, {self.failed} failed")
        return 0 if self.failed == 0 else 1


def main() -> int:
    checks = Checks()
    data = presets.load()

    # Schema: required surfaces, finite numbers, ranges, provenance and
    # confidence. A broken value fails here before it can be generated.
    errors = presets.validate(data)
    checks.check(not errors, "schema validates" + (f": {errors[:3]}" if errors else ""))
    for name in presets.PRESET_NAMES:
        checks.check(name in data["glass"]["presets"], f"{name} preset exists")
        for scheme in presets.SCHEMES:
            entry = data["glass"]["presets"][name][scheme]
            checks.check(bool(entry["provenance"]), f"{name}.{scheme} provenance")
            checks.check(entry["confidence"] in presets.CONFIDENCES,
                         f"{name}.{scheme} confidence")

    # Range/finite validation catches a bad value rather than generating it.
    broken = copy.deepcopy(data)
    broken["glass"]["presets"]["dock"]["light"]["calibration"]["neutralAlpha"] = float("nan")
    checks.check(bool(presets.validate(broken)), "non-finite value is rejected")
    broken = copy.deepcopy(data)
    broken["glass"]["presets"]["dock"]["light"]["blur"] = -1.0
    checks.check(bool(presets.validate(broken)), "negative blur is rejected")
    broken = copy.deepcopy(data)
    broken["glass"]["slider"]["lightAnchors"]["mid"]["alpha"] = 0.9
    checks.check(bool(presets.validate(broken)),
                 "a moved measured anchor is rejected")
    broken = copy.deepcopy(data)
    del broken["glass"]["presets"]["tooltip"]
    checks.check(bool(presets.validate(broken)), "a missing required preset is rejected")

    # The three measured light File anchors, with piecewise interpolation.
    for position, alpha, tint in (
        (0.0, 0.732, 0.953),
        (0.5255, 0.552, 0.993),
        (1.0, 0.415, 0.997),
    ):
        result = presets.resolve(data, "menuAttached", scheme="light",
                                 slider=position, rollout="replacement")
        checks.near(result["alpha"], alpha, f"light slider alpha at {position}")
        checks.near(result["tint"], tint, f"light slider tint at {position}")
        checks.check(result["sliderApplied"], f"slider applied at {position}")
    quarter = presets.resolve(data, "menuAttached", scheme="light", slider=0.25,
                              rollout="replacement")
    checks.check(0.552 < quarter["alpha"] < 0.732,
                 "light slider interpolates between the first two anchors")

    # Unmeasured roles keep their named baseline; the detached and status
    # menus follow the measured curve by the documented relative adjustment.
    for name in presets.PRESET_NAMES:
        if name == presets.SLIDER_PRESET or name in presets.RELATIVE_SLIDER_PRESETS:
            continue
        low = presets.resolve(data, name, scheme="light", slider=0.0,
                              rollout="replacement")
        high = presets.resolve(data, name, scheme="light", slider=1.0,
                               rollout="replacement")
        checks.check(not low["sliderApplied"], f"{name}: slider not applied")
        checks.near(low["alpha"], high["alpha"], f"{name}: slider alpha unchanged")
        checks.near(low["tint"], high["tint"], f"{name}: slider tint unchanged")
    default_position = float(data["glass"]["slider"]["defaultPosition"])
    for name in presets.RELATIVE_SLIDER_PRESETS:
        default = presets.resolve(data, name, scheme="light",
                                  rollout="replacement")
        explicit = presets.resolve(data, name, scheme="light",
                                   slider=default_position,
                                   rollout="replacement")
        checks.near(default["alpha"], explicit["alpha"],
                    f"{name}: relative default alpha preserved")
        checks.near(default["tint"], explicit["tint"],
                    f"{name}: relative default tint preserved")
        low = presets.resolve(data, name, scheme="light", slider=0.0,
                              rollout="replacement")
        high = presets.resolve(data, name, scheme="light", slider=1.0,
                               rollout="replacement")
        checks.check(low["sliderApplied"] and high["sliderApplied"],
                     f"{name}: relative slider applied")
        checks.check(low["alpha"] > default["alpha"] > high["alpha"],
                     f"{name}: relative alpha follows the attached curve")
        checks.check(low["tint"] < default["tint"] < high["tint"],
                     f"{name}: relative tint follows the attached curve")
        dark = presets.resolve(data, name, scheme="dark", slider=0.0,
                               rollout="replacement")
        dark_high = presets.resolve(data, name, scheme="dark", slider=1.0,
                                    rollout="replacement")
        checks.check(not dark["sliderApplied"] and not dark_high["sliderApplied"],
                     f"{name}: dark ignores the light slider")
        checks.near(dark["alpha"], dark_high["alpha"],
                    f"{name}: dark slider alpha unchanged")
    dark_low = presets.resolve(data, "menuAttached", scheme="dark", slider=0.0,
                               rollout="replacement")
    dark_high = presets.resolve(data, "menuAttached", scheme="dark", slider=1.0,
                                rollout="replacement")
    checks.check(not dark_low["sliderApplied"], "dark attached menu ignores the slider")
    checks.near(dark_low["alpha"], dark_high["alpha"], "dark attached alpha unchanged")

    # Dark attached and detached are separate presets.
    attached = presets.resolve(data, "menuAttached", scheme="dark", rollout="replacement")
    detached = presets.resolve(data, "menuDetached", scheme="dark", rollout="replacement")
    checks.check((attached["alpha"], attached["tint"])
                 != (detached["alpha"], detached["tint"]),
                 "dark attached and detached presets are distinct")

    # Reduce Transparency wins over the slider and sampling; Increase
    # Contrast still strengthens the result.
    reduced = presets.resolve(data, "menuAttached", scheme="light", slider=0.0,
                              reduce_transparency=True, rollout="replacement")
    checks.check(reduced["renderMode"] == "solid",
                 "Reduce Transparency is solid")
    checks.check(not reduced["sliderApplied"],
                 "Reduce Transparency skips the slider")
    checks.check(reduced["alpha"] >= 0.9,
                 "Reduce Transparency is opaque")
    checks.check(reduced["blur"] == 0.0,
                 "Reduce Transparency disables sampling")
    contrasted = presets.resolve(data, "menuAttached", scheme="light", slider=0.0,
                                 reduce_transparency=True, increase_contrast=True,
                                 rollout="replacement")
    checks.check(contrasted["edge"] > reduced["edge"],
                 "Increase Contrast strengthens the edge")
    checks.check(contrasted["foreground"] > reduced["foreground"],
                 "Increase Contrast strengthens the foreground")

    # A veil is intentional no-blur, not an unsupported sampled path.
    for backend in presets.BACKENDS:
        veil = presets.resolve(data, "menuBarVeil", scheme="light",
                               rollout="replacement", backend=backend)
        checks.check(veil["renderMode"] == "no-blur-veil",
                     f"veil is explicit on {backend}")
        checks.check(veil["fallbackReason"] == "none",
                     f"veil has no fallback reason on {backend}")
        checks.check(veil["blur"] == 0.0, f"veil requests no blur on {backend}")

    # T200: the measured Light depth LUT is exact at the reference samples and
    # ends by 95 pt; the veil colour is the scheme's lift/wash, not the legacy
    # neutral plate.
    light_veil = presets.resolve(data, "menuBarVeil", scheme="light",
                                 rollout="replacement")
    checks.near(light_veil["tint"], 1.0, "Light veil is a white lift")
    checks.near(light_veil["stripHeight"], 39, "reference strip is 39 pt")
    checks.near(light_veil["extent"], 95, "veil extent is 95 pt")
    measured = {
        5: 0.45098037, 15: 0.43921566, 25: 0.40784317, 35: 0.35686272,
        50: 0.24313729, 75: 0.05882353, 90: 0.00784314, 95: 0.0,
    }
    for depth, expected in measured.items():
        actual = light_veil["veil"][f"alpha{depth}"]
        checks.check(abs(actual - expected) <= 0.03,
                     f"Light LUT alpha at {depth} pt is within 0.03")
    checks.check(light_veil["veil"]["alpha95"] == 0.0,
                 "Light LUT ends at zero by 95 pt")
    dark_veil = presets.resolve(data, "menuBarVeil", scheme="dark",
                                rollout="replacement")
    checks.near(dark_veil["tint"], 0.0, "Dark veil is a black wash")
    checks.check(dark_veil["veil"]["alpha95"] == 0.0,
                 "Dark LUT ends at zero by 95 pt")

    # Unsupported sampled paths are safe and explicit.
    for backend, mode in (("gpu-no-sampled-blur", "solid"),
                          ("software", "software-fallback")):
        result = presets.resolve(data, "menuDetached", scheme="dark",
                                 rollout="replacement", backend=backend)
        checks.check(result["renderMode"] == mode,
                     f"{backend} resolves to {mode}")
        checks.check(result["alpha"] >= 0.9,
                     f"{backend} uses an opaque plate")
        checks.check(result["blur"] == 0.0,
                     f"{backend} requests no sampling")

    # Unknown names take the safe solid path.
    unknown = presets.resolve(data, "not-a-preset", scheme="light",
                              rollout="replacement")
    checks.check(unknown["renderMode"] == "solid", "unknown preset is solid")
    checks.check(unknown["fallbackReason"] == "unknown-preset",
                 "unknown preset names its reason")

    # The persistent Dock floor keeps sampling at Minimal; transient popups
    # may become solid.
    dock = presets.resolve(data, "dock", scheme="light", rollout="replacement",
                           tier="minimal")
    checks.check(dock["renderMode"] == "sampled-glass",
                 "Dock keeps the sampled floor")
    popup = presets.resolve(data, "menuDetached", scheme="light",
                            rollout="replacement", tier="minimal")
    checks.check(popup["renderMode"] == "solid",
                 "transient popup may become solid")

    # Reduce Motion is orthogonal.
    moving = presets.resolve(data, "dock", scheme="light", rollout="replacement",
                             reduce_motion=False)
    still = presets.resolve(data, "dock", scheme="light", rollout="replacement",
                            reduce_motion=True)
    checks.check((moving["alpha"], moving["renderMode"])
                 == (still["alpha"], still["renderMode"]),
                 "Reduce Motion does not change optics")

    # The rollout switch defaults to the existing appearance and parses the
    # shared fixture/native value.
    legacy = presets.resolve(data, "dock", scheme="light")
    checks.check(legacy["look"] == "legacy", "default rollout is legacy")
    checks.check(legacy["renderMode"] == "sampled-glass",
                 "legacy default keeps sampled glass")
    checks.check(presets.rollout_from_env({}) == "legacy",
                 "unset rollout defaults to legacy")
    checks.check(presets.rollout_from_env(
        {"DRAGONFRUIT_GLASS_ROLLOUT": "replacement"}) == "replacement",
        "replacement rollout is selectable")
    checks.check(presets.rollout_from_env(
        {"DRAGONFRUIT_GLASS_ROLLOUT": "bogus"}) == "legacy",
        "unknown rollout is safe")

    # The token file stays JSON-reproducible.
    serialized = json.dumps(data, sort_keys=True)
    checks.check(serialized == json.dumps(presets.load(), sort_keys=True),
                 "token data round-trips reproducibly")

    return checks.report()


if __name__ == "__main__":
    raise SystemExit(main())
