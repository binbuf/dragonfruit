# 0161 — The T-17.1c Flatpak/browser capture runs the live shell as the portal presenter

## Status

accepted

## Context

T-17.1c verifies the Flatpak half of the T-17 premium gate on the nested
session and commits a capture: a real Flatpak browser
(`org.mozilla.firefox`) file-chooses, screenshots, and screen-shares
([17-premium-gate.md](../tracks/17-premium-gate.md)). T-13.7 already exercised
the same three portal calls through the real `xdg-desktop-portal` frontend and
the Dragonfruit backend, but its automated round-trips used a **host Python
stand-in** for the presenter; only the FileChooser picker was raised live, as a
still, with the request left pending. The premium gate needs the **live shell**
to answer each request, so the browser's portal response is the proof.

The three shell presenters are [0077](0077-filechooser-picker-seat.md),
[0078](0078-screenshot-portal-and-selection-overlay.md), and
[0080](0080-screencast-portal-and-source-picker.md); all route their chrome
input through the shell protocol (the T-17.1a/T-151a capture pattern).

## Decision

- **The live shell is the presenter; the browser's response is the assertion.**
  `scripts/capture-t17-flatpak-browser.sh` starts a private session bus with
  the real frontend, the Dragonfruit backend, and `make demo` nested with the
  synthetic-input harness. `scripts/t17-flatpak-flow.py` runs *inside*
  `flatpak run org.mozilla.firefox` and issues one real portal call per flow;
  the host driver
  (`scripts/t17-flatpak-browser-driver.py`) completes each in the live shell by
  synthetic input and the capture fails unless the client prints
  `FLOW: RESULT: PASS` with a non-empty `uris` / `uri` / `streams` result.
- **Clicks are calibrated against the live layout, not hard-coded.** The
  picker and source-picker surfaces are the only change between a pre-flow
  baseline still and the mapped-picker still, so the driver diff-images them to
  recover the surface rect (the centred `640x440` and `560x460` cards) and then
  clicks the first row relative to that rect. The screenshot overlay is
  full-output and is driven by a region drag.
- **KWin and Spectacle use the host session bus.** The demo runs on the private
  portal bus, so the active-window raise (`gdbus … org.kde.KWin`) and
  `spectacle -a` are pinned to the host bus captured before the private bus is
  exported. Without this the active-window capture fails silently.
- **The `current_folder` `ay` is trimmed of its trailing NUL.** The presenter
  bug the flow surfaced (see Consequences) is fixed in
  `ChooserBridge::suggestedUri`, with the real NUL-terminated option pinned in
  `tst_chooser`.

## Consequences

- The capture is reproducible non-interactively on a host Wayland session with
  Flatpak + Firefox, `/usr/libexec/xdg-desktop-portal`, `spectacle`, and
  python3 (host and sandbox) with PyGObject/Pillow; it is not part of `make
  e2e` (CI has no host session or Flatpak).
- The verification **surfaced a real presenter bug**: Qt's D-Bus `ay` decoding
  hands the shell's `ChooserBridge` a NUL-terminated `current_folder`, and
  `suggestedUri` fed the NUL into `QUrl::fromLocalFile`, so the real Flatpak
  file-choose failed with `Cannot open file:%00.` before any listing. The
  minimal fix (chop trailing NULs from `current_file`/`current_folder`, the
  same trim the portal backend's `decode_path_bytes` already does) is in this
  unit because the acceptance cannot be met otherwise; `tst_chooser`'s
  round-trip now sends the real NUL-terminated option so the regression cannot
  return.
- The screen-share stream is the documented stills fallback
  (`df_fallback = pipewire-producer-unavailable`); the picker names it, and
  `Start` returns `monitor:NESTED-1`. T-17.2 repeats this on DRM.
- T-17.2 reuses this private-bus + live-presenter pattern for the DRM loop.