# T-28 — Screenshot and Screen Recording UI

| | |
|---|---|
| **Phase** | 5 · Desktop infrastructure |
| **Area** | `shell/screenshot/` |
| **Depends on** | [T-27](27-portal-backend.md) (capture path) · [T-03](03-input-keymaps-shortcuts.md) (keybind) · [T-08](08-design-system.md) · [T-07](07-private-shell-protocols.md) |
| **Blocks** | Daily-driver bar (screenshots/recordings) |
| **Estimate** | M |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [07-system-integration.md](../design/07-system-integration.md) |

## Summary

The capture **UI**: a shell surface for region/window/monitor selection,
shot preview with copy/save/annotate affordances, and recording controls —
always riding the portal capture path (single-frame and PipeWire streams),
never a private grab path.

## Background

The capture UI is a shell surface; the capture itself goes through the
portal capture path (single-frame and PipeWire streams —
[07-system-integration.md](../design/07-system-integration.md)); the
keybind is a compositor global shortcut; screenshot and OSD feedback
follow the same design-system motion rules as the rest of the chrome
([04-shell.md](../design/04-shell.md)).

## Scope

### In scope

1. **Invocation**: compositor global shortcut (configurable, with
   modifiers for window/full-region variants), menu-bar/Control-Center
   entry.
2. **Selection overlay**: full-screen chrome surface with per-window /
   per-monitor / free-region selection, hover highlights, cross-hair
   cursor, Esc-to-cancel, reduced-motion variant.
3. **Shot flow**: freeze-frame preview → Copy / Save / (annotate later) /
   Share-to-app actions; autosave default location (xdg-user-dirs
   Pictures/Screenshots); success follows the design-system motion rules
   (thumbnail flight to where it saved).
4. **Recording flow**: start/stop UI, OSD/indicator while recording,
   per-window/per-monitor selection (via the same portal screencast
   path), audio-source choice where WirePlumber exposes one.
5. **Portal interplay**: when a *third-party app* requests a screenshot or
   screencast through the portal (T-27), our consent UI renders here with
   per-window/per-monitor choice.
6. **Keyboard**: full keyboard-driven capture (select window by arrow
   keys, Enter to shoot).

### Out of scope

- The capture plumbing itself (T-27/T-02).
- Annotation/editing tooling (later; not roadmap-blocking).

## Requirements

- FR-1: Overlay appears within one frame of the shortcut; selection is
  progress-animated and interruptible per the design-system rules.
- FR-2: Capture rounds: region / window / monitor × screenshot /
  recording — all through the portal request path (assert: no capture
  without a portal request in the trace).
- FR-3: Copy-to-clipboard includes text/URI/image formats that work in
  Firefox and Files (DnD + paste).
- FR-4: Recording persists across monitor changes and lock policy per
  T-27's documented behavior; stop produces a valid file even if the
  shell restarts mid-recording (journaling in the portal, verified here
  end-to-end).
- FR-5: Consent UI for third-party capture requests is modal to the
  session (choice required), cancels cleanly, and honors Focus/DND
  suppression rules for *banners* only — never for consent.
- FR-6: Locked session: capture keybinds do nothing while locked
  (invariant from T-26).

## Acceptance criteria

- [ ] All capture modes work with zero dropped frames during overlay
      animations.
- [ ] End-to-end: shortcut → select → shoot → save/copy → paste into
      Firefox, insert into Files (DnD).
- [ ] Third-party consent flow demonstrated with the Flatpak browser
      (contributes to Phase-5 exit).

## Test plan

- Nested + VM: overlay timing, selection modes, keyboard flow.
- Restart drill during recording; suspend during recording (document
  outcome; soak in T-31).

## Risks / open questions

- Multi-DPI selection precision (fractional-scale coordinate mapping) —
  use compositor-provided surface geometry, not client math.
- Decide recording codec/container default (matroska/VP9 vs platform
  availability) — keep it a setting, not a fork.
