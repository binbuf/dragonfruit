# T-17.1c Flatpak/browser end-to-end — reviewed flows

This is the reviewed companion to the scripted capture in
`docs/captures/t17-flatpak-browser.*` (`make t17-flatpak-browser-capture`). The
harness and its evidence boundary are
[ADR 0161](../design/adr/0161-t17-flatpak-browser-capture.md); the protocol
compatibility this sits over is [ADR 0085](../design/adr/0085-real-frontend-portal-compatibility.md).

## The Flatpak browser walkthrough

A real Flatpak browser (`org.mozilla.firefox`) is executed *inside its sandbox*
against the **live nested Dragonfruit session**: a private session bus runs the
real `xdg-desktop-portal` frontend with the Dragonfruit backend, `make demo`
runs nested with the synthetic-input harness, and the **live shell** presents
and answers each request. No portal call is simulated and no host stand-in
answers; the client's portal response is the assertion. The raw transcript is
`t17-flatpak-browser.txt`.

| Flow | Browser call | Live shell presenter | Observed response | Still |
|---|---|---|---|---|
| File-choose | `FileChooser.OpenFile` (with `current_folder`) | centred 640×440 picker (`FileChooser.qml`); first row clicked, Return | `response=0`, `uris=['file:///run/user/1000/doc/…/picked.txt']` (the document-portal URI of the picked file) | `t17-flatpak-browser-file-choose.png` |
| Screenshot | `Screenshot.Screenshot` (`interactive=true`) | full-output `SelectionOverlay.qml` in region mode; a region dragged | `response=0`, `uri='file:///home/user/Pictures/Screenshots/Screenshot_….png'` (the shell captured, saved, and returned it) | `t17-flatpak-browser-screenshot.png` |
| Screen-share | `ScreenCast.CreateSession` / `SelectSources` / `Start` | centred 560×460 `ScreenCastPicker.qml`; the monitor row clicked, Return | `SelectSources response=0`; `Start` returns `streams=[(0, {df_stream_mode: stills, id: monitor:NESTED-1, source_type: 1})]` | `t17-flatpak-browser-screen-share.png` |

Each flow's client log ends `FLOW: RESULT: PASS <flow>`; the capture driver
fails the run if any is not `PASS`. The initial still is
`t17-flatpak-browser.png` (the nested desktop) and the clip is
`t17-flatpak-browser.mp4`.

## Live visual check (run for this task)

The nested session was launched (`make demo`) and the whole walkthrough was
driven and captured. The desktop renders in every step — menu bar and Dock
present, the wallpaper and the demo clients composited:

- **File-choose still**: the centred picker card titled *T17 Flatpak
  FileChooser*, the path bar over the host scratch folder, the single file row
  `picked.txt` (32 B), and the `Cancel`/`Open` footer; the `Open` button is
  enabled only after the row is selected. No blank/torn/ghosted regions.
- **Screenshot still**: the full-output dimmed selection overlay with the
  *Screenshot · Region* badge and *Drag to select · Esc to cancel* hint, with
  the menu bar and Dock still visible through the scrim. No artifacts.
- **Screen-share still**: the centred *Share your screen* card, the subtitle
  naming `org.mozilla.firefox`, the stills-fallback note, the **Screens**
  section with the `NESTED-1` 1920 × 1200 monitor row, a **Windows** section,
  and the `Cancel`/`Share` footer. No artifacts in the shell chrome.

The expected `xmessage` X11 demo window carries its own raw-X11 rendering (a
slightly clipped edge), unrelated to the shell chrome. The reviewed captures
are `docs/captures/t17-flatpak-browser*.png`.

## Deviations and gotchas

- **The verification surfaced and fixed a presenter bug.** Qt's D-Bus `ay`
  decoding hands the shell a NUL-terminated `current_folder`; `suggestedUri`
  passed that NUL into `QUrl::fromLocalFile`, so the real Flatpak file-choose
  failed with `Cannot open file:%00.` before listing anything. The minimal fix
  (trim trailing NULs, matching the portal backend's `decode_path_bytes`) is in
  [chooserbridge.cpp](../../shell/src/chooserbridge.cpp), pinned by
  `tst_chooser`'s now NUL-terminated `current_folder`. This is the one product
  change in an otherwise verification-only unit.
- **Clicks are diff-calibrated.** The picker and source-picker surface rects
  are recovered by diffing a pre-flow baseline still against the mapped-picker
  still (the surfaces are the only change), so the row clicks land regardless
  of host scale; the geometry matched the documented centred cards
  (`640×440` at `(640,378)`, `560×460` at `(680,368)`).
- **KWin and Spectacle must use the host session bus.** The demo runs on the
  private portal bus; the active-window raise and `spectacle -a` are pinned to
  the host bus captured before the private one is exported, or the capture
  fails silently.
- **The screen-share stream is the documented stills fallback.** This host has
  no PipeWire producer, so `Start` returns `df_fallback =
  pipewire-producer-unavailable` and `df_stream_mode = stills`; the picker
  names the fallback before the user chooses (T-13.4b). The live PipeWire half
  is a host-capability matter, not a portal-flow failure.
- **The screenshot flow writes a real file.** The shell saves to
  `~/Pictures/Screenshots` and copies to the clipboard (T-13.3b); the run
  leaves one PNG there.
- The `F: Can't find a11y bus` line in the client log is the sandbox's
  AT-SPI-bus probe failing (its session has no accessibility bus); it is
  unrelated to the portal flows.

## Reproduction

```bash
# host Wayland session, flatpak + org.mozilla.firefox, /usr/libexec/xdg-desktop-portal,
# spectacle, python3 + PyGObject/Pillow, ffmpeg (optional clip), built tree
make t17-flatpak-browser-capture     # or: bash scripts/capture-t17-flatpak-browser.sh
```

The protocol compatibility this capture sits over:

```bash
cargo test -p xdg-desktop-portal-dragonfruit
ctest --test-dir build -R "tst_chooser$|tst_screenshot$|tst_screencast$"
```

## Follow-ups

- The live PipeWire screencast producer (the stills fallback is named but the
  live stream is not exercised here); T-17.2 repeats the flows on DRM.