# Keymap Conventions and Global Shortcuts

T-03 fixes the modifier-role mapping **once** and shares it across the
compositor, the shell, and first-party applications. This file is the
in-repo record of that decision (T-03 acceptance criterion).

## Role mapping

macOS-style role names map to physical xkb modifiers as follows:

| Logical role | Physical key | xkb modifier | `df_ipc::keymap` constant |
|---|---|---|---|
| Command (Cmd) | Super / Windows | `Mod4` | `ModifierRole::Command` |
| Option | Alt | `Mod1` | `ModifierRole::Option` |
| Control | Ctrl | `Control` | `ModifierRole::Control` |
| Shift | Shift | `Shift` | `ModifierRole::Shift` |

The mapping lives in exactly one place: `protocols/df-ipc/src/lib.rs`
(`df_ipc::keymap`). The compositor's xkb keymap and its shortcut engine
both consume it; the shell and first-party apps consume the same crate
through the lockstep set. Changing it there changes it everywhere.

Layout, model, and variant are deliberately left to the user's
`XKB_DEFAULT_*` environment; what is pinned is the *role*, not a layout.

## Global shortcut engine

- The compositor owns system shortcuts (workspace switching, Mission
  Control, app switcher, screenshots, notification center, desktop reveal,
  lock screen). Bindings are matched in the seat's key filter, before a
  key reaches any client, and the release of an intercepted key is
  swallowed too so clients never see half a chord.
- Application accelerators are admitted only while their owning window is
  focused. The menu-broker (T-22) registers them; the portal's
  GlobalShortcuts interface (T-27) registers the same shape for sandboxed
  callers.
- Conflict resolution: system shortcuts win over application
  accelerators; the focused window's menu wins among application
  accelerators.
- **No client installs a raw key grab.** The shortcut-inhibit protocol is
  never advertised, and every grab request goes through the compositor's
  `GrabArbiter`, which logs and refuses unsanctioned clients.

## Default system bindings

Defaults are rebindable through the Settings Keyboard pane (T-16); they
are listed here for reference.

| Binding | Action |
|---|---|
| `Ctrl+Left` / `Ctrl+Right` | Previous / next Space |
| `Ctrl+1` … `Ctrl+9` | Activate Space by index |
| `Ctrl+Up` | Mission Control |
| `Ctrl+Down` | Desktop Reveal |
| `Cmd+Tab` | App switcher |
| `Cmd+Shift+3` / `Cmd+Shift+4` | Screenshot |
| `Cmd+Ctrl+Q` | Lock screen |
| `Ctrl+Shift+N` | Notification center |

## Gestures and hot corners

Three/four-finger horizontal swipes switch Spaces; four-finger vertical
swipes open Mission Control / reveal the desktop; pinches drive the same
Mission Control / reveal pipeline. Hot corners use a short dwell so a
pointer merely crossing one does not trigger it. Every trigger — keyboard,
gesture, hot corner, shell, portal — resolves to the same
`InputAction` and feeds the same 0→1 progress pipeline, so behavior is
identical regardless of how it started.
