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
    # T200: the measured background-luminance response. Without this curve
    # the veil stays at the black-background alpha over light wallpapers.
    checks.check(light_veil["veil"]["adaptLuma"] == [0.0, 0.25, 0.5, 0.78, 1.0],
                 "Light adaptation spans the measured luminance anchors")
    for expected, actual in zip((1.0, 1.06, 0.91, 0.48, 0.0),
                                light_veil["veil"]["adaptScale"]):
        checks.near(actual, expected, "Light adaptation scale")
    dark_veil = presets.resolve(data, "menuBarVeil", scheme="dark",
                                rollout="replacement")
    checks.near(dark_veil["tint"], 0.0, "Dark veil is a black wash")
    checks.check(dark_veil["veil"]["alpha95"] == 0.0,
                 "Dark LUT ends at zero by 95 pt")
    # The coverage captures give a much subtler Dark wash than the original
    # 0.48 baseline; the curve must keep the mid-grey fit and disappear over
    # white. It must also peak at mid-grey rather than at 25% grey: a black
    # wash is most visible over a dark window, so the near-black response
    # stays low instead of scaling the Light curve directly.
    for expected, actual in zip((0.06, 0.14, 0.26026, 0.13728, 0.0),
                                dark_veil["veil"]["adaptScale"]):
        checks.near(actual, expected, "Dark adaptation scale")
    checks.check(dark_veil["veil"]["adaptScale"][1]
                 < dark_veil["veil"]["adaptScale"][2],
                 "Dark adaptation peaks at mid-grey, not 25% grey")
    checks.check(dark_veil["veil"]["alpha5"]
                 * dark_veil["veil"]["adaptScale"][0] < 0.08,
                 "Dark veil stays subtle over a dark window")

    # T206: first-party app chrome. The measured Light Settings/Files fits and
    # the measured Dark Files fits are exact; Settings Dark is an adaptation
    # from Finder. The presets are near-opaque solid, never sampled passes.
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
        result = presets.resolve(data, name, scheme=scheme,
                                 rollout="replacement")
        checks.near(result["alpha"], alpha, f"{name} {scheme} alpha")
        checks.near(result["tint"], tint, f"{name} {scheme} tint")
        checks.check(result["renderMode"] == "solid",
                     f"{name} {scheme} is an app-local solid preset")
        checks.check(result["blur"] == 0.0,
                     f"{name} {scheme} does not sample")
        checks.check(result["confidence"] == confidence,
                     f"{name} {scheme} confidence is {confidence}")
        checks.check(result["alpha"] >= 0.9,
                     f"{name} {scheme} stays near-opaque")
        checks.check(result["fallbackReason"] == "none",
                     f"{name} {scheme} names no fallback")
        default = presets.resolve(data, name, scheme=scheme)
        checks.check(default["look"] == "replacement",
                     f"{name} {scheme} uses the replacement look by default")
        legacy = presets.resolve(data, name, scheme=scheme, rollout="legacy")
        checks.check(legacy["look"] == "legacy",
                     f"{name} {scheme} keeps the legacy look selectable")
        reduced = presets.resolve(data, name, scheme=scheme,
                                  reduce_transparency=True,
                                  rollout="replacement")
        checks.check(reduced["renderMode"] == "solid" and reduced["alpha"] >= 0.9,
                     f"{name} {scheme} Reduce Transparency uses the opaque plate")
    for scheme in presets.SCHEMES:
        sidebar = presets.resolve(data, "filesSidebar", scheme=scheme,
                                  rollout="replacement")
        toolbar = presets.resolve(data, "filesToolbar", scheme=scheme,
                                  rollout="replacement")
        checks.check((sidebar["alpha"], sidebar["tint"])
                     != (toolbar["alpha"], toolbar["tint"]),
                     f"Files {scheme} sidebar/toolbar are independent")
        for name in ("settingsSidebar", "settingsToolbar"):
            entry = presets.resolve(data, name, scheme=scheme,
                                    rollout="replacement")
            checks.check(entry["alpha"] >= 0.9,
                         f"{name} {scheme} is near-opaque")
    # The app presets do not follow the light attached-menu slider.
    for name in presets.APP_CHROME_PRESETS:
        low = presets.resolve(data, name, scheme="light", slider=0.0,
                              rollout="replacement")
        high = presets.resolve(data, name, scheme="light", slider=1.0,
                               rollout="replacement")
        checks.check(not low["sliderApplied"] and not high["sliderApplied"],
                     f"{name} ignores the menu slider")
        checks.near(low["alpha"], high["alpha"],
                    f"{name} slider does not move alpha")
        checks.near(low["tint"], high["tint"],
                    f"{name} slider does not move tint")

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

    # The rollout switch defaults to the replacement appearance and parses the
    # shared fixture/native value; the legacy rail stays selectable.
    replacement = presets.resolve(data, "dock", scheme="light")
    checks.check(replacement["look"] == "replacement",
                 "default rollout is replacement")
    checks.check(replacement["renderMode"] == "sampled-glass",
                 "replacement default keeps sampled glass")
    legacy = presets.resolve(data, "dock", scheme="light", rollout="legacy")
    checks.check(legacy["look"] == "legacy", "legacy rollout is selectable")
    checks.check(legacy["renderMode"] == "sampled-glass",
                 "legacy rollout keeps sampled glass")
    checks.check(presets.rollout_from_env({}) == "replacement",
                 "unset rollout defaults to replacement")
    checks.check(presets.rollout_from_env(
        {"DRAGONFRUIT_GLASS_ROLLOUT": "legacy"}) == "legacy",
        "legacy rollout is selectable")
    checks.check(presets.rollout_from_env(
        {"DRAGONFRUIT_GLASS_ROLLOUT": "bogus"}) == "replacement",
        "unknown rollout falls back to the default")

    # The Applications drawer is a translucent sampled glass card, not the
    # earlier near-opaque plate. The light fit follows the reference panel
    # gray; dark is the same transmission with the measured dark menu tint.
    for scheme, alpha, tint in (
        ("light", 0.72, 0.96),
        ("dark", 0.82, 0.26),
    ):
        drawer = presets.resolve(data, "appsDrawer", scheme=scheme,
                                 rollout="replacement")
        checks.near(drawer["alpha"], alpha, f"appsDrawer.{scheme} alpha")
        checks.near(drawer["tint"], tint, f"appsDrawer.{scheme} tint")
        checks.check(drawer["renderMode"] == "sampled-glass",
                     f"appsDrawer.{scheme} keeps sampled glass")
        checks.check(drawer["alpha"] < 0.9,
                     f"appsDrawer.{scheme} stays translucent")

    # T205: the banner, stack and lower panel are three measured/adapted
    # materials with independent resolution, not one Popup plate.
    notification_targets = {
        ("notificationBanner", "light"): (0.549, 0.976),
        ("notificationStack", "light"): (0.551, 0.706),
        ("notificationStack", "dark"): (0.507, 0.550),
        ("notificationLowerPanel", "light"): (0.407, 0.852),
    }
    for (name, scheme), (alpha, tint) in notification_targets.items():
        result = presets.resolve(data, name, scheme=scheme,
                                 rollout="replacement")
        checks.near(result["alpha"], alpha, f"{name}.{scheme} alpha")
        checks.near(result["tint"], tint, f"{name}.{scheme} tint")
        checks.check(result["renderMode"] == "sampled-glass",
                     f"{name}.{scheme} keeps sampled glass")
    for name in ("notificationBanner", "notificationStack",
                 "notificationLowerPanel"):
        for scheme in ("light", "dark"):
            reduced = presets.resolve(data, name, scheme=scheme,
                                      rollout="replacement",
                                      reduce_transparency=True)
            checks.check(reduced["renderMode"] == "solid"
                         and reduced["alpha"] >= 0.95,
                         f"{name}.{scheme} Reduce Transparency is the opaque plate")
            software = presets.resolve(data, name, scheme=scheme,
                                       rollout="replacement", backend="software")
            checks.check(software["renderMode"] == "software-fallback",
                         f"{name}.{scheme} software fallback is explicit")
    for scheme in ("light", "dark"):
        banner = presets.resolve(data, "notificationBanner", scheme=scheme,
                                 rollout="replacement")
        stack = presets.resolve(data, "notificationStack", scheme=scheme,
                                rollout="replacement")
        lower = presets.resolve(data, "notificationLowerPanel", scheme=scheme,
                                rollout="replacement")
        outputs = {
            "banner": 0.502 * (1 - banner["alpha"]) + banner["tint"] * banner["alpha"],
            "stack": 0.502 * (1 - stack["alpha"]) + stack["tint"] * stack["alpha"],
            "lower": 0.502 * (1 - lower["alpha"]) + lower["tint"] * lower["alpha"],
        }
        checks.check(max(outputs.values()) - min(outputs.values()) > 0.02,
                     f"{scheme} notification materials are distinct")
    checks.check(
        data["glass"]["presets"]["notificationBanner"]["dark"]["confidence"]
        == "adapted",
        "the uncovered Dark banner is labelled adapted")
    checks.check(
        data["glass"]["presets"]["notificationLowerPanel"]["dark"]["confidence"]
        == "adapted",
        "the uncovered Dark lower panel is labelled adapted")

    # The token file stays JSON-reproducible.
    serialized = json.dumps(data, sort_keys=True)
    checks.check(serialized == json.dumps(presets.load(), sort_keys=True),
                 "token data round-trips reproducibly")

    return checks.report()


if __name__ == "__main__":
    raise SystemExit(main())
