# 0083 — Clipboard history: a shell-side observer with a bounded pure store

## Status

Accepted (T-13.5b).

## Context

ADR 0082 froze clipboard ownership on the compositor's `wl_data_device` and
named T-13.5b's history as an observer/cache over the `wlr-data-control`
manager surface, with "clear-on-lock and size caps" living there. Legacy T-29
specified a history (recent entries, previews for text/image/file lists,
search, pin, clear, size/privacy policy, cleared on lock, no cloud). The
track made history conditional on "if the design calls for it"; the accepted
ADR 0082 and the legacy reference resolve the condition to yes.

The hard constraint: a history must not become a second clipboard owner, and
the shell has no data-control client today (its existing `wl_data_device` is
focus-gated for drops only).

## Decision

1. **The shell observes, it does not own.** `ShellProtocol` binds the vendored
   `wlr-data-control-unstable-v1` client protocol, gets a data device for the
   seat, and on each `selection` reads the supported MIME payloads
   (`text/uri-list`, `image/*`, `text/plain*`) into pipes. It only re-serves an
   entry when the user explicitly copies one from the panel, by creating a
   `zwlr_data_control_source_v1` and calling `set_selection`. That is the
   single ownership hand-off.

2. **The store is pure and headless-tested.** `ClipboardHistory`
   (`shell/src/clipboardhistory.{h,cpp}`, dockcore) names no Wayland or D-Bus.
   It classifies a selection (`text`/`image`/`files`/`other`), deduplicates by
   content hash (promoting a re-copy and preserving its pin), and enforces
   hard caps: 50 entries, 1 MiB per entry, 8 MiB total, evicting the oldest
   unpinned first.

3. **Privacy is default-on.** The common password-manager secret hints
   (`x-kde-passwordManagerHint` with a secret value, any `password`/`secret`
   MIME) are refused, not stored. A lock clears the unpinned entries by
   default; pinned entries survive.

4. **Persistence is best-effort JSON**, written under the app data directory
   and loading tolerantly (malformed or oversized payloads are dropped). The
   panel renders the store; the store never touches Qt Widgets/QML.

## Consequences

- The clipboard still works when the shell is idle: the history only reads and
  never replaces the observed source, so the original owner keeps serving
  pastes.
- A future richer UI (search, image thumbnails, dedicated popover) only reads
  `ClipboardHistory`; it needs no new protocol or ownership change.
- The compositor needs no change: it already serves `wlr-data-control` with an
  open filter, so no version bump or new interface is introduced.
- `check-no-capture-grab` is unaffected: this adds no framebuffer path.