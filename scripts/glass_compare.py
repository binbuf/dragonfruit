#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Hardware-independent comparison for Dragonfruit glass captures.

This is the T194 comparison core.  It is deliberately small and dependency
light: Pillow is the only third-party import, so the self-test runs anywhere a
developer can run the capture scripts.  It is used by

* ``scripts/t194-glass-compare.py`` (the command-line wrapper),
* ``tests/glass/run_selftest.py`` (the deterministic fixture self-test), and
* later T207 comparison runs.

The comparison has four independent metric families:

* **colour** — mean / p95 absolute sRGB error, CIE76 ΔE and SSIM over an
  explicit material band.  Text, icons and unrelated windows are excluded by
  the case's masks.
* **geometry** — foreground bbox, bbox delta, mask IoU and area ratio.  A
  small bounded registration removes capture-alignment drift *before* scoring,
  but a shift that lands on the search bound is reported as a failure so a
  crop or a real geometry change cannot be hidden by registration.
* **MTF** — the 1-D contrast amplitude at named spatial periods, with the
  candidate/reference ratio.  This catches blur/contrast regressions on the
  checker and line-grid fixtures.
* **motion** — object displacement and direction between two frames.  A
  stationary candidate where the reference moves fails.

Every metric reports ``coverage`` (the number of source pixels considered) and
``status`` (``pass``, ``fail``, ``missing`` or ``not_requested``).  A missing
band is never silently treated as a pass.

Source images are converted to sRGB before scoring.  An embedded ICC profile is
honoured (Display P3 in the reference checkout); a file without a profile is
treated as sRGB.  Geometry and case rectangles are logical coordinates; the
caller supplies the capture scale (``2`` for the reference workstation, ``1``
for the nested fixtures).
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from dataclasses import dataclass
from io import BytesIO
from pathlib import Path
from typing import Iterable, Sequence

try:
    from PIL import Image, ImageChops, ImageCms, ImageStat
except ImportError as exc:  # pragma: no cover - the error is the useful part
    raise SystemExit(
        "glass_compare: Pillow is required (python3 -c 'import PIL'); "
        "install python3-pil/Pillow"
    ) from exc


SCHEMA_VERSION = 1
SRGB_PROFILE = ImageCms.createProfile("sRGB")
LAB_PROFILE = ImageCms.createProfile("LAB")
DEFAULT_MAX_SHIFT_LOGICAL = 2.0
DEFAULT_COLOUR_THRESHOLDS = {
    "mean_abs_srgb": 0.02,
    "p95_abs_srgb": 0.05,
    "ssim": 0.90,
    "delta_e76_mean": 3.0,
}
DEFAULT_GEOMETRY_THRESHOLDS = {
    "bbox_delta_px": 1.0,
    "iou": 0.98,
    "area_ratio": 0.02,
}
DEFAULT_MTF_THRESHOLDS = {
    "ratio": 0.85,
}
DEFAULT_MOTION_THRESHOLDS = {
    "displacement_ratio": 0.50,
    "direction_error_deg": 30.0,
}


def _pixels(image: Image.Image) -> list:
    """Pillow 12 deprecates ``getdata``; use the replacement when present."""
    getter = getattr(image, "get_flattened_data", None)
    if getter is not None:
        return list(getter())
    return list(image.getdata())  # pragma: no cover - older Pillow


def load_srgb(path: str | Path) -> Image.Image:
    """Load *path* as 8-bit RGB in sRGB, honouring an embedded ICC profile."""
    image = Image.open(path)
    icc = image.info.get("icc_profile")
    image = image.convert("RGB")
    if icc:
        source = ImageCms.getOpenProfile(BytesIO(icc))
        image = ImageCms.profileToProfile(
            image, source, SRGB_PROFILE, outputMode="RGB", renderingIntent=0
        )
    return image


def icc_info(path: str | Path) -> dict:
    """Describe the embedded profile without converting the image."""
    image = Image.open(path)
    icc = image.info.get("icc_profile")
    if not icc:
        return {"present": False, "assumed": "sRGB"}
    try:
        profile = ImageCms.getOpenProfile(BytesIO(icc))
        description = ImageCms.getProfileDescription(profile).strip()
    except Exception:
        description = "unreadable"
    return {
        "present": True,
        "description": description,
        "sha256": __import__("hashlib").sha256(icc).hexdigest(),
        "assumed": "embedded profile converted to sRGB",
    }


def _rect(value: Sequence[float], scale: float = 1.0) -> tuple[int, int, int, int]:
    if len(value) != 4:
        raise ValueError(f"rectangle needs four numbers, got {value!r}")
    x0, y0, x1, y1 = (int(round(float(v) * scale)) for v in value)
    if x1 <= x0 or y1 <= y0:
        raise ValueError(f"empty rectangle {value!r} at scale {scale}")
    return x0, y0, x1, y1


def _crop(image: Image.Image, rect: tuple[int, int, int, int], shift=(0, 0)):
    x0, y0, x1, y1 = rect
    dx, dy = shift
    return image.crop((x0 + dx, y0 + dy, x1 + dx, y1 + dy))


def _inside(image: Image.Image, rect: tuple[int, int, int, int], shift=(0, 0)) -> bool:
    x0, y0, x1, y1 = rect
    dx, dy = shift
    return x0 + dx >= 0 and y0 + dy >= 0 and x1 + dx <= image.width and y1 + dy <= image.height


def _translate(image: Image.Image, dx: int, dy: int, fill="#000000") -> Image.Image:
    """Move image content by (dx, dy) on a same-size canvas."""
    out = Image.new("RGB", image.size, fill)
    out.paste(image, (dx, dy))
    return out


def _histogram_stats(diff: Image.Image) -> dict:
    """Mean / p95 / max of an RGB difference image, normalised to 0..1."""
    hist = diff.histogram()
    total = 0
    count = 0
    maximum = 0
    channel_hist = [hist[i * 256 : (i + 1) * 256] for i in range(3)]
    for channel in channel_hist:
        for value, n in enumerate(channel):
            total += value * n
            count += n
            if n and value > maximum:
                maximum = value
    mean = total / count if count else 0.0
    # p95 across all three channels, weighted by the same pixel count.
    threshold = 0.95 * count
    running = 0
    p95 = 0
    for value in range(256):
        running += sum(channel[value] for channel in channel_hist)
        if running >= threshold:
            p95 = value
            break
    return {
        "mean_abs_srgb": mean / 255.0,
        "p95_abs_srgb": p95 / 255.0,
        "max_abs_srgb": maximum / 255.0,
        "samples": count // 3,
    }


def _ssim(reference: Image.Image, candidate: Image.Image) -> float:
    """Global SSIM on luminance, downsampled to keep large captures cheap."""
    a = reference.convert("L")
    b = candidate.convert("L")
    limit = 512
    if max(a.size) > limit:
        ratio = limit / max(a.size)
        size = (max(1, round(a.width * ratio)), max(1, round(a.height * ratio)))
        a = a.resize(size, Image.Resampling.BILINEAR)
        b = b.resize(size, Image.Resampling.BILINEAR)
    pa = _pixels(a)
    pb = _pixels(b)
    n = len(pa)
    if not n:
        return 1.0
    sum_a = sum(pa)
    sum_b = sum(pb)
    sum_aa = sum(v * v for v in pa)
    sum_bb = sum(v * v for v in pb)
    sum_ab = sum(x * y for x, y in zip(pa, pb))
    mean_a = sum_a / n
    mean_b = sum_b / n
    var_a = max(0.0, sum_aa / n - mean_a * mean_a)
    var_b = max(0.0, sum_bb / n - mean_b * mean_b)
    cov = sum_ab / n - mean_a * mean_b
    c1 = (0.01 * 255) ** 2
    c2 = (0.03 * 255) ** 2
    numerator = (2 * mean_a * mean_b + c1) * (2 * cov + c2)
    denominator = (mean_a * mean_a + mean_b * mean_b + c1) * (var_a + var_b + c2)
    return float(numerator / denominator) if denominator else 1.0


def _delta_e76(reference: Image.Image, candidate: Image.Image) -> dict:
    """Mean CIE76 ΔE in Lab, sampled on very large bands."""
    lab_a = ImageCms.profileToProfile(
        reference, SRGB_PROFILE, LAB_PROFILE, outputMode="LAB", renderingIntent=0
    )
    lab_b = ImageCms.profileToProfile(
        candidate, SRGB_PROFILE, LAB_PROFILE, outputMode="LAB", renderingIntent=0
    )
    pa = _pixels(lab_a)
    pb = _pixels(lab_b)
    step = max(1, len(pa) // 250_000)
    values = []
    for i in range(0, len(pa), step):
        a = pa[i]
        b = pb[i]
        values.append(math.sqrt((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2))
    if not values:
        return {"mean": 0.0, "p95": 0.0, "samples": 0}
    values.sort()
    return {
        "mean": sum(values) / len(values),
        "p95": values[min(len(values) - 1, int(0.95 * len(values)))],
        "samples": len(values),
    }


def colour_metrics(
    reference: Image.Image,
    candidate: Image.Image,
    rect: tuple[int, int, int, int],
    shift=(0, 0),
    thresholds: dict | None = None,
) -> dict:
    """Score one material band independently."""
    thresholds = {**DEFAULT_COLOUR_THRESHOLDS, **(thresholds or {})}
    # Registration is expressed as the candidate's offset: the reference is
    # sampled at the case rectangle, the candidate at the registered offset.
    if not _inside(reference, rect) or not _inside(candidate, rect, shift):
        return {"status": "missing", "coverage": 0, "thresholds": thresholds}
    ref = _crop(reference, rect)
    cand = _crop(candidate, rect, shift)
    coverage = ref.width * ref.height
    if coverage == 0:
        return {"status": "missing", "coverage": 0, "thresholds": thresholds}
    diff = ImageChops.difference(ref, cand)
    stats = _histogram_stats(diff)
    ssim = _ssim(ref, cand)
    delta = _delta_e76(ref, cand)
    failures = []
    if stats["mean_abs_srgb"] > thresholds["mean_abs_srgb"]:
        failures.append("mean_abs_srgb")
    if stats["p95_abs_srgb"] > thresholds["p95_abs_srgb"]:
        failures.append("p95_abs_srgb")
    if ssim < thresholds["ssim"]:
        failures.append("ssim")
    if delta["mean"] > thresholds["delta_e76_mean"]:
        failures.append("delta_e76_mean")
    return {
        "status": "fail" if failures else "pass",
        "coverage": coverage,
        "thresholds": thresholds,
        "failures": failures,
        **stats,
        "ssim": ssim,
        "delta_e76_mean": delta["mean"],
        "delta_e76_p95": delta["p95"],
        "delta_e76_samples": delta["samples"],
    }


def _foreground_mask(
    image: Image.Image, rect: tuple[int, int, int, int], background, tolerance: float
) -> Image.Image:
    crop = _crop(image, rect)
    bg = Image.new("RGB", crop.size, background)
    diff = ImageChops.difference(crop, bg)
    r, g, b = diff.split()
    max_channel = ImageChops.lighter(ImageChops.lighter(r, g), b)
    threshold = max(1, int(round(float(tolerance) * 255)))
    return max_channel.point(lambda value: 255 if value > threshold else 0, mode="1")


def _mask_count(mask: Image.Image) -> int:
    return mask.histogram()[255]


def _mask_iou(a: Image.Image, b: Image.Image) -> float:
    if a.size != b.size:
        return 0.0
    both = ImageChops.logical_and(a, b)
    either = ImageChops.logical_or(a, b)
    union = _mask_count(either)
    return float(_mask_count(both) / union) if union else 1.0


def _mask_centroid(mask: Image.Image) -> tuple[float, float] | None:
    bbox = mask.getbbox()
    if bbox is None:
        return None
    total = 0
    sx = 0
    sy = 0
    for y in range(bbox[1], bbox[3]):
        row = mask.crop((bbox[0], y, bbox[2], y + 1))
        values = _pixels(row)
        for x, value in enumerate(values, start=bbox[0]):
            if value:
                total += 1
                sx += x
                sy += y
    if not total:
        return None
    return sx / total, sy / total


def _bbox_logical(mask: Image.Image, rect: tuple[int, int, int, int], scale: float):
    bbox = mask.getbbox()
    if bbox is None:
        return None
    x0, y0, x1, y1 = bbox
    rx0, ry0, _, _ = rect
    return [
        (rx0 + x0) / scale,
        (ry0 + y0) / scale,
        (rx0 + x1) / scale,
        (ry0 + y1) / scale,
    ]


def geometry_metrics(
    reference: Image.Image,
    candidate: Image.Image,
    rect: tuple[int, int, int, int],
    background,
    tolerance: float,
    scale: float,
    shift=(0, 0),
    thresholds: dict | None = None,
) -> dict:
    """Compare the detected foreground geometry, after bounded registration."""
    thresholds = {**DEFAULT_GEOMETRY_THRESHOLDS, **(thresholds or {})}
    # The reference is sampled at the case rectangle; the candidate at the
    # registered offset.  Both bboxes are reported in the same logical frame.
    dx, dy = shift
    cand_rect = (rect[0] + dx, rect[1] + dy, rect[2] + dx, rect[3] + dy)
    if not _inside(reference, rect) or not _inside(candidate, cand_rect):
        return {
            "status": "missing",
            "coverage": 0,
            "reference_bbox": None,
            "candidate_bbox": None,
            "thresholds": thresholds,
        }
    ref_mask = _foreground_mask(reference, rect, background, tolerance)
    cand_mask = _foreground_mask(candidate, cand_rect, background, tolerance)
    coverage = (rect[2] - rect[0]) * (rect[3] - rect[1])
    ref_bbox = _bbox_logical(ref_mask, rect, scale)
    cand_bbox = _bbox_logical(cand_mask, cand_rect, scale)
    if cand_bbox is not None:
        cand_bbox = [
            cand_bbox[0] - dx / scale,
            cand_bbox[1] - dy / scale,
            cand_bbox[2] - dx / scale,
            cand_bbox[3] - dy / scale,
        ]
    if ref_bbox is None or cand_bbox is None:
        return {
            "status": "missing",
            "coverage": coverage,
            "reference_bbox": ref_bbox,
            "candidate_bbox": cand_bbox,
            "thresholds": thresholds,
        }
    bbox_delta = max(abs(a - b) for a, b in zip(ref_bbox, cand_bbox))
    ref_area = _mask_count(ref_mask)
    cand_area = _mask_count(cand_mask)
    area_ratio = abs(cand_area - ref_area) / ref_area if ref_area else 1.0
    iou = _mask_iou(ref_mask, cand_mask)
    failures = []
    if bbox_delta > thresholds["bbox_delta_px"]:
        failures.append("bbox_delta_px")
    if iou < thresholds["iou"]:
        failures.append("iou")
    if area_ratio > thresholds["area_ratio"]:
        failures.append("area_ratio")
    return {
        "status": "fail" if failures else "pass",
        "coverage": coverage,
        "thresholds": thresholds,
        "failures": failures,
        "reference_bbox": ref_bbox,
        "candidate_bbox": cand_bbox,
        "bbox_delta_px": bbox_delta,
        "iou": iou,
        "reference_area": ref_area,
        "candidate_area": cand_area,
        "area_ratio": area_ratio,
    }


def _profile(
    image: Image.Image, rect: tuple[int, int, int, int], axis: str, shift=(0, 0)
) -> list[float]:
    crop = _crop(image, rect, shift).convert("L")
    values = _pixels(crop)
    width, height = crop.size
    if axis == "x":
        profile = [0.0] * width
        for y in range(height):
            base = y * width
            for x in range(width):
                profile[x] += values[base + x]
        return [value / height for value in profile]
    profile = [0.0] * height
    for y in range(height):
        base = y * width
        row = values[base : base + width]
        profile[y] = sum(row) / width
    return profile


def _amplitude(profile: Sequence[float], period: float) -> float:
    values = [float(v) for v in profile]
    if not values:
        return 0.0
    mean = sum(values) / len(values)
    values = [v - mean for v in values]
    n = len(values)
    window = [0.5 - 0.5 * math.cos(2 * math.pi * i / (n - 1)) if n > 1 else 1.0 for i in range(n)]
    total_window = sum(window)
    if total_window <= 0:
        return 0.0
    frequency = 1.0 / float(period)
    real = 0.0
    imag = 0.0
    for i, value in enumerate(values):
        angle = 2 * math.pi * frequency * i
        real += value * window[i] * math.cos(angle)
        imag -= value * window[i] * math.sin(angle)
    return 2 * math.hypot(real, imag) / total_window


def mtf_metrics(
    reference: Image.Image,
    candidate: Image.Image,
    rect: tuple[int, int, int, int],
    axis: str,
    periods: Iterable[float],
    shift=(0, 0),
    thresholds: dict | None = None,
) -> dict:
    """Compare contrast amplitude at named periods (the MTF proxy)."""
    thresholds = {**DEFAULT_MTF_THRESHOLDS, **(thresholds or {})}
    periods = [float(p) for p in periods]
    coverage = (rect[2] - rect[0]) * (rect[3] - rect[1])
    if not periods:
        return {"status": "not_requested", "coverage": 0, "thresholds": thresholds}
    if not _inside(reference, rect) or not _inside(candidate, rect, shift):
        return {
            "status": "missing",
            "coverage": 0,
            "axis": axis,
            "thresholds": thresholds,
            "periods": {},
            "failures": ["band outside the capture"],
        }
    ref_profile = _profile(reference, rect, axis)
    cand_profile = _profile(candidate, rect, axis, shift)
    rows = {}
    failures = []
    for period in periods:
        ref_amp = _amplitude(ref_profile, period)
        cand_amp = _amplitude(cand_profile, period)
        ratio = (cand_amp / ref_amp) if ref_amp > 1e-9 else None
        rows[f"{period:g}"] = {
            "period_px": period,
            "reference_amplitude": ref_amp,
            "candidate_amplitude": cand_amp,
            "ratio": ratio,
        }
        if ratio is None:
            failures.append(f"period={period:g}:reference-flat")
        elif ratio < thresholds["ratio"]:
            failures.append(f"period={period:g}")
    return {
        "status": "fail" if failures else "pass",
        "coverage": coverage,
        "axis": axis,
        "thresholds": thresholds,
        "periods": rows,
        "failures": failures,
    }


def register(
    reference: Image.Image,
    candidate: Image.Image,
    rect: tuple[int, int, int, int],
    max_shift_px: int,
) -> dict:
    """Search a bounded integer translation that best aligns *candidate*.

    The score is the mean absolute luminance difference over the registration
    band.  A best shift on the search bound is reported as not accepted: if the
    real offset is larger than the bound, the caller must fail rather than let
    registration hide a geometry regression.
    """
    if max_shift_px <= 0:
        return {
            "shift_px": [0, 0],
            "shift_logical": [0.0, 0.0],
            "score": None,
            "bounded": True,
            "ok": True,
            "at_bound": False,
        }
    ref = _crop(reference, rect).convert("L")
    # Keep registration cheap on full-output captures.
    limit = 320
    ratio = 1.0
    if max(ref.size) > limit:
        ratio = limit / max(ref.size)
        size = (max(1, round(ref.width * ratio)), max(1, round(ref.height * ratio)))
        ref_small = ref.resize(size, Image.Resampling.BILINEAR)
        x0, y0, x1, y1 = rect
        scaled_rect = (
            x0,
            y0,
            x0 + max(1, round((x1 - x0) * ratio)),
            y0 + max(1, round((y1 - y0) * ratio)),
        )
    else:
        ref_small = ref
        scaled_rect = rect
    best = None
    for dy in range(-max_shift_px, max_shift_px + 1):
        for dx in range(-max_shift_px, max_shift_px + 1):
            scaled_shift = (int(round(dx * ratio)), int(round(dy * ratio)))
            cand = _crop(candidate, scaled_rect, scaled_shift).convert("L")
            if cand.size != ref_small.size:
                continue
            diff = ImageChops.difference(ref_small, cand)
            stat = ImageStat.Stat(diff)
            score = float(stat.mean[0])
            if best is None or score < best[0]:
                best = (score, dx, dy)
    if best is None:
        return {
            "shift_px": [0, 0],
            "shift_logical": [0.0, 0.0],
            "score": None,
            "bounded": True,
            "ok": False,
            "at_bound": True,
            "reason": "no candidate translation fit inside the image",
        }
    score, dx, dy = best
    at_bound = abs(dx) == max_shift_px or abs(dy) == max_shift_px
    return {
        "shift_px": [dx, dy],
        "shift_logical": [dx, dy],
        "score": score,
        "bounded": True,
        "ok": not at_bound,
        "at_bound": at_bound,
        "reason": "best shift is on the search bound; a larger geometry change is not accepted"
        if at_bound
        else None,
    }


def _case_regions(case: dict) -> dict:
    return case.get("regions", {})


def compare_case(
    reference_path: str | Path,
    candidate_path: str | Path,
    case: dict,
    *,
    scale: float | None = None,
    max_shift_logical: float = DEFAULT_MAX_SHIFT_LOGICAL,
) -> dict:
    """Compare one candidate capture against a reference using *case* masks."""
    reference = load_srgb(reference_path)
    candidate = load_srgb(candidate_path)
    scale = float(case.get("scale", scale if scale is not None else 1.0))
    report = {
        "schema": SCHEMA_VERSION,
        "case": case.get("name", Path(reference_path).stem),
        "reference": str(reference_path),
        "candidate": str(candidate_path),
        "scale": scale,
        "reference_size": list(reference.size),
        "candidate_size": list(candidate.size),
        "metrics": {},
    }
    if reference.size != candidate.size:
        report["verdict"] = "fail"
        report["error"] = (
            f"candidate size {candidate.size} does not match reference "
            f"{reference.size}; a crop/scale change is a geometry regression"
        )
        return report

    regions = _case_regions(case)
    registration_rect = _rect(
        case.get("registration_region", regions.get("geometry", regions.get("interior", [0, 0, 1, 1]))),
        scale,
    )
    max_shift_px = int(round(float(max_shift_logical) * scale))
    registration = register(reference, candidate, registration_rect, max_shift_px)
    registration["max_shift_px"] = max_shift_px
    registration["max_shift_logical"] = max_shift_logical
    report["registration"] = registration
    shift = tuple(registration["shift_px"])
    if not registration["ok"]:
        report["verdict"] = "fail"
        report["error"] = registration.get("reason", "registration failed")
        return report
    # Apply the correction once so every metric sees the same aligned frame.
    # Registration reports how the candidate's content is offset; translating
    # it by the negative offset puts it back in the reference frame.
    background = case.get("geometry", {}).get("background", "#000000")
    if shift != (0, 0):
        candidate = _translate(candidate, -shift[0], -shift[1], background)
    shift = (0, 0)

    # --- colour: every named band is scored separately ---------------------
    colour_regions = case.get("colour_regions")
    if colour_regions is None:
        colour_regions = [
            {"name": name, "rect": regions[name]}
            for name in ("interior", "rim", "shadow")
            if name in regions
        ]
    colour = {}
    for entry in colour_regions:
        name = entry.get("name", "band")
        rect = _rect(entry["rect"], scale)
        colour[name] = colour_metrics(
            reference, candidate, rect, shift, entry.get("thresholds")
        )
    report["metrics"]["colour"] = _family(colour)

    # --- geometry ----------------------------------------------------------
    geometry_case = case.get("geometry")
    if geometry_case:
        rect = _rect(geometry_case.get("rect", regions.get("geometry", [0, 0, 1, 1])), scale)
        report["metrics"]["geometry"] = geometry_metrics(
            reference,
            candidate,
            rect,
            geometry_case.get("background", "#000000"),
            float(geometry_case.get("tolerance", 0.05)),
            scale,
            shift,
            geometry_case.get("thresholds"),
        )
    else:
        report["metrics"]["geometry"] = {
            "status": "not_requested",
            "coverage": 0,
        }

    # --- MTF ---------------------------------------------------------------
    mtf_case = case.get("mtf")
    if mtf_case:
        rect = _rect(mtf_case.get("rect", regions.get("mtf", regions.get("interior", [0, 0, 1, 1]))), scale)
        report["metrics"]["mtf"] = mtf_metrics(
            reference,
            candidate,
            rect,
            mtf_case.get("axis", "x"),
            mtf_case.get("periods_px", []),
            shift,
            mtf_case.get("thresholds"),
        )
    else:
        report["metrics"]["mtf"] = {"status": "not_requested", "coverage": 0}

    report["verdict"] = _verdict(report["metrics"])
    return report


def _family(entries: dict) -> dict:
    statuses = [entry.get("status") for entry in entries.values()]
    if not entries:
        return {"status": "not_requested", "coverage": 0, "bands": {}}
    if any(status == "fail" for status in statuses):
        status = "fail"
    elif any(status == "missing" for status in statuses):
        status = "missing"
    else:
        status = "pass"
    return {
        "status": status,
        "coverage": sum(entry.get("coverage", 0) for entry in entries.values()),
        "bands": entries,
    }


def _verdict(metrics: dict) -> str:
    statuses = [metric.get("status") for metric in metrics.values()]
    if any(status == "fail" for status in statuses):
        return "fail"
    if any(status == "missing" for status in statuses):
        return "missing"
    return "pass"


def _angle_error_deg(a: tuple[float, float], b: tuple[float, float]) -> float:
    if a == (0.0, 0.0) or b == (0.0, 0.0):
        return 0.0 if a == b else 180.0
    dot = a[0] * b[0] + a[1] * b[1]
    norm = math.hypot(*a) * math.hypot(*b)
    if norm == 0:
        return 0.0
    return math.degrees(math.acos(max(-1.0, min(1.0, dot / norm))))


def compare_motion(
    reference_before: str | Path,
    reference_after: str | Path,
    candidate_before: str | Path,
    candidate_after: str | Path,
    case: dict,
    *,
    scale: float | None = None,
) -> dict:
    """Compare the displacement of a moving test object between two frames."""
    scale = float(case.get("scale", scale if scale is not None else 1.0))
    rect = _rect(case.get("region", case.get("regions", {}).get("motion", [0, 0, 1, 1])), scale)
    background = case.get("background", "#000000")
    tolerance = float(case.get("tolerance", 0.05))
    thresholds = {**DEFAULT_MOTION_THRESHOLDS, **case.get("thresholds", {})}
    images = {
        "reference_before": load_srgb(reference_before),
        "reference_after": load_srgb(reference_after),
        "candidate_before": load_srgb(candidate_before),
        "candidate_after": load_srgb(candidate_after),
    }
    for name, image in images.items():
        if image.size != next(iter(images.values())).size:
            return {
                "schema": SCHEMA_VERSION,
                "case": case.get("name", "motion"),
                "verdict": "fail",
                "error": f"{name} size {image.size} differs from the first frame",
                "metrics": {},
            }
    centroids = {}
    for name, image in images.items():
        mask = _foreground_mask(image, rect, background, tolerance)
        centroids[name] = _mask_centroid(mask)
    coverage = (rect[2] - rect[0]) * (rect[3] - rect[1])
    if any(value is None for value in centroids.values()):
        return {
            "schema": SCHEMA_VERSION,
            "case": case.get("name", "motion"),
            "verdict": "missing",
            "error": "one or more frames have no foreground object",
            "coverage": coverage,
            "centroids": centroids,
            "metrics": {"motion": {"status": "missing", "coverage": coverage}},
        }
    ref_dx = centroids["reference_after"][0] - centroids["reference_before"][0]
    ref_dy = centroids["reference_after"][1] - centroids["reference_before"][1]
    cand_dx = centroids["candidate_after"][0] - centroids["candidate_before"][0]
    cand_dy = centroids["candidate_after"][1] - centroids["candidate_before"][1]
    ref_vec = (ref_dx / scale, ref_dy / scale)
    cand_vec = (cand_dx / scale, cand_dy / scale)
    ref_len = math.hypot(*ref_vec)
    cand_len = math.hypot(*cand_vec)
    ratio = cand_len / ref_len if ref_len > 1e-9 else None
    direction_error = _angle_error_deg(ref_vec, cand_vec)
    min_displacement = float(case.get("min_displacement_px", 0.0))
    failures = []
    if ref_len < min_displacement:
        failures.append("reference displacement below min_displacement_px")
    if ratio is None:
        failures.append("reference has no displacement")
    elif ratio < thresholds["displacement_ratio"]:
        failures.append("displacement_ratio")
    if direction_error > thresholds["direction_error_deg"]:
        failures.append("direction_error_deg")
    if cand_len < min_displacement:
        failures.append("candidate displacement below min_displacement_px")
    return {
        "schema": SCHEMA_VERSION,
        "case": case.get("name", "motion"),
        "coverage": coverage,
        "scale": scale,
        "thresholds": thresholds,
        "centroids": centroids,
        "reference_displacement": [ref_vec[0], ref_vec[1]],
        "candidate_displacement": [cand_vec[0], cand_vec[1]],
        "reference_distance": ref_len,
        "candidate_distance": cand_len,
        "displacement_ratio": ratio,
        "direction_error_deg": direction_error,
        "failures": failures,
        "metrics": {
            "motion": {
                "status": "fail" if failures else "pass",
                "coverage": coverage,
                "failures": failures,
            }
        },
        "verdict": "fail" if failures else "pass",
    }


def compare_timing(reference: dict, candidate: dict, *, rel: float = 0.10, absolute: float = 2.0) -> dict:
    """Compare scalar counters with relative and absolute tolerances."""
    rows = {}
    failures = []
    for key in sorted(set(reference) & set(candidate)):
        a = reference[key]
        b = candidate[key]
        if isinstance(a, bool) or isinstance(b, bool):
            continue
        if not isinstance(a, (int, float)) or not isinstance(b, (int, float)):
            continue
        delta = b - a
        tolerance = max(absolute, rel * abs(a))
        ok = abs(delta) <= tolerance
        rows[key] = {
            "reference": a,
            "candidate": b,
            "delta": delta,
            "tolerance": tolerance,
            "status": "pass" if ok else "fail",
        }
        if not ok:
            failures.append(key)
    return {
        "schema": SCHEMA_VERSION,
        "thresholds": {"relative": rel, "absolute": absolute},
        "coverage": len(rows),
        "counters": rows,
        "failures": failures,
        "status": "fail" if failures else ("pass" if rows else "missing"),
    }


def load_case(path: str | Path) -> dict:
    return json.loads(Path(path).read_text())


def _print_report(report: dict) -> None:
    print(json.dumps(report, indent=2, sort_keys=True))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    compare = sub.add_parser("compare", help="compare one capture against a reference")
    compare.add_argument("--reference", required=True)
    compare.add_argument("--candidate", required=True)
    compare.add_argument("--case", required=True)
    compare.add_argument("--scale", type=float)
    compare.add_argument("--max-shift", type=float, default=DEFAULT_MAX_SHIFT_LOGICAL)
    compare.add_argument("--out")
    compare.add_argument("--json", action="store_true", help="print the report as JSON")

    motion = sub.add_parser("motion", help="compare a two-frame motion sequence")
    motion.add_argument("--reference-before", required=True)
    motion.add_argument("--reference-after", required=True)
    motion.add_argument("--candidate-before", required=True)
    motion.add_argument("--candidate-after", required=True)
    motion.add_argument("--case", required=True)
    motion.add_argument("--scale", type=float)
    motion.add_argument("--out")
    motion.add_argument("--json", action="store_true")

    timing = sub.add_parser("timing", help="compare scalar counter JSON files")
    timing.add_argument("--reference", required=True)
    timing.add_argument("--candidate", required=True)
    timing.add_argument("--relative", type=float, default=0.10)
    timing.add_argument("--absolute", type=float, default=2.0)
    timing.add_argument("--out")
    timing.add_argument("--json", action="store_true")

    args = parser.parse_args(argv)
    if args.command == "compare":
        report = compare_case(
            args.reference, args.candidate, load_case(args.case),
            scale=args.scale, max_shift_logical=args.max_shift,
        )
    elif args.command == "motion":
        report = compare_motion(
            args.reference_before, args.reference_after,
            args.candidate_before, args.candidate_after,
            load_case(args.case), scale=args.scale,
        )
    else:
        report = compare_timing(
            json.loads(Path(args.reference).read_text()),
            json.loads(Path(args.candidate).read_text()),
            rel=args.relative, absolute=args.absolute,
        )
    if args.out:
        Path(args.out).parent.mkdir(parents=True, exist_ok=True)
        Path(args.out).write_text(json.dumps(report, indent=2, sort_keys=True))
    if args.json or not args.out:
        _print_report(report)
    else:
        print(f"wrote {args.out}: {report.get('verdict', report.get('status'))}")
    return 0 if report.get("verdict", report.get("status")) == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
