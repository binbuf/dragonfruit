# ADR 0238 — Input breadth, controller inventory, directory projection, and printer options

- **Status:** Accepted
- **Date:** 2026-11-08
- **Task:** T229

## Context

T229 closes the remaining Settings breadth: Sound's Test action, Keyboard
input sources/text replacements/dictation, the Mouse and Trackpad rows the
earlier wave omitted, Game Controllers, Users & Groups' admin group and
directory account, Printers & Scanners add/options, and Battery's options and
power-settings link. The existing adapters are the only host-stack owners; the
Settings app must never shell out or write xkb/xinput files.

## Decision

1. **Pointer settings reach the compositor live.** The private
   `df_toplevel_manager` interface grows two additive v10 requests:
   `set_pointer_policy` (pointer speed, natural scroll, tap-to-click,
   left-handed, scroll method, secondary click, scroll speed, double-click
   interval, haptics) and `set_keyboard_layout` (layout/variant). The shell
   mirrors the settingsd keys; the compositor stores them in the existing
   `PointerSettings`/keymap model. Speeds travel as millionths and are clamped
   to the schema ranges. Older compositors ignore both requests.
2. **Input sources are one durable ordered list.** `input.sources` is a
   `TextList`; the first entry is the active layout and is forwarded as the
   compositor keymap. The pane never writes an xkb file. Text replacements are
   the `input.replacements` `from\tto` list consumed by the session input
   method (T253). Dictation is enabled only when a speech-to-text command is
   installed; otherwise the row is present and disabled with its reason.
3. **Controllers reuse the input adapter.** `services/input` reads
   `/proc/bus/input/devices` for joystick/gamepad records and the SDL mapping
   table, and exposes a diagnostic `TestController` read (one event with a
   two-second timeout). The pane is optional and shipped; mapping edits and a
   remapper are out of scope.
4. **Users & Groups reports the directory; it does not join it.** The account
   adapter reads `/etc/sssd/sssd.conf` domains/provider and `realm list`, and
   derives the `wheel`/`sudo` admin group from the group provider. Joining or
   leaving a domain stays delegated to the distribution's `realm`/SSSD
   tooling.
5. **Printers grow discovery, add, and per-queue options.** `lpinfo -v`
   populates the Add dialog; `lpadmin -p … -E -v … -m …` adds the queue;
   `lpoptions -l`/`-o` read and write one option. A queue with no options is an
   honest empty sheet. A full job manager and driver installation remain out
   of scope.
6. **Battery options are settingsd keys.** `power.showPercentage` is consumed
   by the menu-bar battery item; `power.historyRange` persists the existing
   range switch. The Power Settings rows read `idle.blank`/`idle.lock` and
   navigate to Lock Screen. Schema revision 22 carries the ten new keys.

## Consequences

- The compositor is the only live pointer/keymap applier; the Settings app
  stays a settingsd writer.
- Per-device pointer overrides remain the DRM backend's when it configures
  libinput devices; the default policy is now live in the compositor state.
- The controller test is a diagnostic read, not a remapper; SDL mapping is
  shown read-only.
- Directory join/leave and printer driver installation stay delegated, with
  explicit notes instead of dead controls.
- The T229 capture (`make t229-input-users-printers-capture`) is the repeatable
  visual evidence; the host Wayland pass remains the manual rail.
