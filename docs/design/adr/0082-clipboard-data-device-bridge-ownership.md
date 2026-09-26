# 0082 — Clipboard round-trips stay on the compositor data-device bridge

## Status

Accepted (T-13.5a).

## Context

T-13 needs clipboard round-trips for text, images, and file lists. The
compositor already delegates the standard `wl_data_device` bridge
(Smithay's `delegate_data_device`) and the `wlr-data-control` manager
surface (`delegate_data_control`), and the Xwayland path bridges X11
selections onto the same data-device selection (T-06). The track design's
risk is explicit: clipboard ownership must stay with that existing bridge
and must not gain a second owner.

T-13.3b left the shell's screenshot copy on `QGuiApplication`'s clipboard
and handed the real Wayland round-trip to T-13.5. T-13.5b may add a
clipboard history manager.

## Decision

1. **The compositor's `wl_data_device` is the one clipboard owner.**
   Ordinary clients set and read the clipboard through it. No portal
   interface and no shell-owned selection is introduced; a clipboard
   portal is deliberately absent because the data device already serves
   sandboxed and native clients.

2. **`wlr-data-control` is the manager surface, not a second owner.** The
   manager (the shell's future clipboard history, T-13.5b) observes and may
   set the selection without keyboard focus. A history manager must
   re-serve what it observed — it forwards the source, it does not replace
   it.

3. **The bridge is MIME-agnostic.** The round-trip matrix proven here is
   `text/plain;charset=utf-8`, `image/png`, and `text/uri-list`; payloads
   cross the offer pipe byte-for-byte with no decode or re-encode in the
   bridge. Other MIME types need no new code.

4. **Focus semantics are standard.** Smithay denies `set_selection` from a
   client without keyboard focus, and pushes the `selection` offer to the
   focused client's data device. Data-control deliberately bypasses focus
   so a manager can see the clipboard in the background.

5. **Conformance is a real headless round trip.** The proof is
   `clipboard_round_trips_text_image_and_uri_list` in
   `compositor/tests/shell_protocol_conformance.rs`: two independent
   Wayland clients, one focused source that copies all three MIME types and
   one focused target that reads every payload back, not a mock.

## Consequences

- T-13.5b's history is an observer/cache over `wlr-data-control`; it must
  keep forwarding the observed source so the clipboard still works when the
  manager is idle. Clear-on-lock and size caps live there.
- No clipboard portal exists; Flatpak applications use the standard data
  device, which is already reachable in the sandbox.
- The Xwayland clipboard bridge (T-06) keeps using the same data-device
  selection, so cross-Xwayland copy/paste needs no separate contract.