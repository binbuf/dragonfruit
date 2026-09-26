# 0080 — The ScreenCast portal and the shell source picker

## Status

Accepted (T-13.4a).

## Context

Sandboxed applications ask the desktop to share a monitor or a window through
`org.freedesktop.impl.portal.ScreenCast`. The interface is a three-step session
(`CreateSession` → `SelectSources` → `Start`), and the dialog that lets the
user choose is arguably the most security-sensitive surface in the portal
track. The streaming half (PipeWire) is a separate, harder slice (T-13.4b), so
this decision must leave a seam that does not change when the stream arrives.

The existing pattern is the FileChooser (ADR 0076/0077) and Screenshot (ADR
0078/0079): a pure model with a one-shot completion registry, a diagnostic
presenter bridge on `org.dragonfruit.Portal1`, and a shell overlay surface fed
by a `*Bridge` in `dockcore`.

## Decision

1. **The backend serves `org.freedesktop.impl.portal.ScreenCast` version 3.**
   Version 3 is the first with the `source_type` stream property, which the
   result already needs; persistence (`persist_mode`/`restore_data`, v4) and
   virtual monitors (`types` bit `4`) are not advertised because they are not
   implemented. `AvailableSourceTypes = MONITOR|WINDOW` and
   `AvailableCursorModes = HIDDEN|EMBEDDED` are honest bitmasks.

2. **The picker opens on `SelectSources`, and the result is a source handle.**
   `SelectSources` registers a request in `portal::screencast` and awaits a
   one-shot completion; the diagnostic `ScreenCastOpened` signal (carrying the
   request's `types`, `multiple`, and raw options) is how the shell picks it up.
   `CompleteScreenCast(handle, a(su))` supplies the chosen `(id, source-type)`
   pairs and `CancelScreenCast(handle)` declines. The session stores the
   selection; `Start` returns it as `streams` (`a(ua{sv})`).

3. **The stream's node id is a placeholder until T-13.4b.** Each stream carries
   `node_id = 0` and a Dragonfruit `id` property with the chosen source handle
   (plus `source_type`). T-13.4b replaces the node id with the real PipeWire
   node and fills the geometry; the D-Bus session, the picker, and the shell
   bridge do not change.

4. **The available sources are the compositor's projection.** The portal does
   not enumerate monitors or windows. The shell's `ShellProtocol` exposes
   `screencastSources()` (monitors from announced outputs, windows from the
   toplevel projection), and `ShellController` filters it by the request's
   `types` before handing it to `ScreenCastBridge`. The shell is the one source
   of truth for what can be shared.

5. **The picker is a centred `screencast` overlay surface.**
   `ScreenCastPicker.qml` (in `Dragonfruit.Screenshot`) is a pure view;
   `ScreenCastBridge` (dockcore) owns the request and the one D-Bus call. A
   missing portal is a normal, silent state. `DF_SCREENCAST_FIXTURE` presents
   the picker with a deterministic source list for the live visual check.

## Consequences

- T-13.4b owns the PipeWire node, stream geometry, cursor handling, and the
  stills fallback; it only fills in `ScreenCastStream` and need not touch the
  picker, the session object, or the diagnostic seam.
- Persistence and virtual monitors require a version bump and a restore-data
  format; they are deliberately not promised by this version.
- The shell never fabricates a source: a session with no compositor projection
  shows an empty picker, and the accept stays disabled.