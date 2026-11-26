#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""T353 chrome presentation-correlation parser tests.

The T238 driver is the shared trace parser. These tests feed it synthetic
shell and compositor logs so the join rules are deterministic without a
nested session:

* a frame callback with no accepted chrome content is not a presentation;
* the accepted generation, not the newest commit at vblank, must be present;
* a repeated dmabuf serial across views counts once;
* dropped commits, multiple outputs and late arrivals classify explicitly.
"""
import importlib.util
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
DRIVER = ROOT / "scripts" / "t238-dock-popover-ab-driver.py"

spec = importlib.util.spec_from_file_location("t238_driver", DRIVER)
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)


def shell_trace(
    trace_id=1,
    kind="dock-popover",
    target="dock-popup",
    output="eDP-1",
    begin=1000,
    commit=1010,
    callback=1020,
    accepted=1,
    serial=42,
    end_detail="callback",
):
    lines = [
        f"chrome trace id={trace_id} stage=begin kind={kind} t_ms={begin} "
        f"mono_us={begin} detail=fixture target={target} output={output}",
        f"chrome trace id={trace_id} stage=request dt_us=0 t_ms={begin} "
        f"mono_us={begin} detail=target={target} output={output} generation=1",
        f"chrome trace id={trace_id} stage=commit-done dt_us=10 t_ms={commit} "
        f"mono_us={commit} detail=target={target} kind="
        f"{'dock' if kind == 'dock-popover' else 'menubar'} ok=1 "
        f"accepted={accepted} serial={serial} gen=1",
        f"chrome trace id={trace_id} stage=commit-done dt_us=10 t_ms={commit} "
        f"mono_us={commit} detail=ok=1 targets=1 popup={accepted} serial={serial}",
    ]
    if callback is not None:
        lines.append(
            f"chrome trace id={trace_id} stage=frame-presented dt_us=20 "
            f"t_ms={callback} mono_us={callback} detail=target={target} "
            f"output={output}"
        )
        lines.append(
            f"chrome trace id={trace_id} stage=end dt_us=20 t_ms={callback} "
            f"mono_us={callback} detail={end_detail}"
        )
    else:
        lines.append(
            f"chrome trace id={trace_id} stage=end dt_us=20 t_ms={commit + 5} "
            f"mono_us={commit + 5} detail={end_detail}"
        )
    return lines


def compositor_commit(surface="dock-popup", generation=2, t_ms=1011,
                      buffer="dmabuf"):
    return (
        f"DFTRACE chrome-commit surface={surface} gen={generation} "
        f"buffer={buffer} bytes=0 t={t_ms} mono_us={t_ms}"
    )


def compositor_present(frame=10, output="eDP-1", chrome="dock-popup:2",
                       t_ms=1013, backend="nested", clock="host"):
    return (
        f"DFTRACE present frame={frame} output={output} backend={backend} "
        f"clock={clock} clock_us={t_ms} chrome={chrome} t={t_ms} "
        f"mono_us={t_ms}"
    )


def compositor_callback(surface="dock-popup", frame=10, t_ms=1013,
                        presented=1):
    return (
        f"DFTRACE frame-callback surface={surface} frame={frame} "
        f"output=eDP-1 presented={presented} t={t_ms} mono_us={t_ms}"
    )


def summarise(shell_lines, compositor_lines):
    traces = driver.parse_traces("\n".join(shell_lines))
    events = driver.parse_dftrace("\n".join(compositor_lines))
    return driver.summarise_presentation(traces, events)


class PresentationTraceTest(unittest.TestCase):
    def test_callback_without_accepted_content_is_not_native(self):
        shell = shell_trace()
        # The compositor presented an older generation and sent a pacing
        # callback for it; the accepted generation 2 was never presented.
        compositor = [
            compositor_commit(generation=2),
            compositor_present(chrome="dock-popup:1"),
            compositor_callback(),
        ]
        report = summarise(shell, compositor)
        self.assertEqual(report["outcomes"]["callback_without_content"], 1)
        self.assertEqual(report["native_samples"], 0)
        self.assertEqual(report["request_to_present_us"]["samples"], 0)
        self.assertEqual(report["callback_without_content"], 1)

    def test_present_with_the_accepted_generation_is_native(self):
        report = summarise(
            shell_trace(),
            [
                compositor_commit(generation=2),
                compositor_present(chrome="dock:1,dock-popup:2"),
                compositor_callback(),
            ],
        )
        self.assertEqual(report["outcomes"]["presented"], 1)
        self.assertEqual(report["native_samples"], 1)
        self.assertEqual(report["request_to_present_us"]["samples"], 1)
        # The callback is a pacing event; the present is the endpoint.
        self.assertEqual(report["request_to_present_us"]["p50"], 13000)

    def test_callback_before_present_is_a_pacing_event(self):
        # On DRM the callback is sent after queue_frame, before the vblank
        # present. That negative callback→present delta must be recorded.
        report = summarise(
            shell_trace(callback=1012),
            [
                compositor_commit(generation=2),
                compositor_callback(t_ms=1012),
                compositor_present(chrome="dock-popup:2", t_ms=1014),
            ],
        )
        self.assertEqual(report["outcomes"]["presented"], 1)
        self.assertEqual(report["callback_to_present_us"]["p50"], 2000)

    def test_repeated_serial_across_views_counts_once(self):
        shell = shell_trace()
        # A shared scene commits the same serial to the strip and the popup.
        shell.insert(
            3,
            "chrome trace id=1 stage=commit-done dt_us=10 t_ms=1010 "
            "mono_us=1010 detail=target=dock kind=dock ok=1 accepted=1 "
            "serial=42 gen=1",
        )
        report = summarise(
            shell,
            [
                compositor_commit(surface="dock", generation=3),
                compositor_commit(surface="dock-popup", generation=2),
                compositor_present(chrome="dock:3,dock-popup:2"),
            ],
        )
        self.assertEqual(report["outcomes"]["presented"], 1)
        self.assertEqual(report["native_samples"], 1)

    def test_dropped_commit_is_discarded(self):
        report = summarise(shell_trace(), [])
        self.assertEqual(report["outcomes"]["discarded"], 1)
        self.assertEqual(report["native_samples"], 0)

    def test_present_on_another_output_does_not_satisfy(self):
        report = summarise(
            shell_trace(),
            [
                compositor_commit(generation=2),
                compositor_present(output="HDMI-A-1", chrome="dock-popup:2"),
            ],
        )
        self.assertNotEqual(report["outcomes"]["presented"], 1)
        self.assertEqual(report["native_samples"], 0)

    def test_late_arrival_is_late_presented(self):
        shell = shell_trace(trace_id=1, callback=None)
        shell += shell_trace(
            trace_id=2, begin=1025, commit=1030, callback=None,
            end_detail="closed-before-callback",
        )
        report = summarise(
            shell,
            [
                compositor_commit(generation=2, t_ms=1011),
                compositor_commit(generation=4, t_ms=1031),
                # The first interaction's content presents only after the
                # second request; classify it as late, not as on time.
                compositor_present(frame=10, chrome="dock-popup:2", t_ms=1040),
            ],
        )
        self.assertGreaterEqual(report["outcomes"]["late_presented"], 1)
        self.assertGreaterEqual(report["native_samples"], 1)

    def test_geometry_only_commit_is_not_content(self):
        report = summarise(
            shell_trace(),
            [
                compositor_commit(generation=2, buffer="none"),
                compositor_present(chrome="dock-popup:2"),
            ],
        )
        self.assertEqual(report["outcomes"]["discarded"], 1)
        self.assertEqual(report["native_samples"], 0)


if __name__ == "__main__":
    unittest.main()
