# ADR 0351 — Terminal resolution and a visible missing-terminal failure

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T340
- **Supersedes:** none

## Context

The shell launched `Terminal=true` desktop entries by honoring `$TERMINAL` or
`x-terminal-emulator` and otherwise running the app command unwrapped. On a
minimal install with neither, a terminal app therefore started with no terminal
(usually failing silently) or a GUI app ran in the background with no window.
The Dock also seeded its Terminal pin from any `.desktop` record with the
`TerminalEmulator` category, even when no emulator binary existed.

A terminal is an optional host service: the session must start and work
without one, but a terminal entry must never silently pretend to work.

## Decision

1. `resolveTerminal(environment, searchPaths)` is one pure resolver in
   `shell/src/desktopentry.*`. It tries, in order: `$TERMINAL`,
   `xdg-terminal-exec`, `x-terminal-emulator`, then ptyxis / gnome-terminal /
   konsole / alacritty / kitty / foot / xterm.
2. `$TERMINAL` may carry arguments (`alacritty --class Terminal`). It is used
   only when its first token is executable; a stale value falls through to the
   next candidate rather than becoming a broken launch.
3. Each source carries the arguments that precede the app command:
   `xdg-terminal-exec` receives the command after `--` (it owns the terminal
   choice), the common terminals and `x-terminal-emulator` use their own `-e`
   or `--` separator, and `$TERMINAL` keeps its configured arguments plus `-e`.
4. `buildLaunchCommand` wraps a `Terminal=true` entry only when the resolver
   found a terminal. With none it returns an empty argv; it never returns the
   unwrapped command. The shell turns that empty result into the existing Dock
   launch-failure notification with the resolver's explanatory reason.
5. The default Dock pins include the `TerminalEmulator` tile only when a
   terminal resolves. The packaging metadata recommends a terminal: Debian's
   `x-terminal-emulator` virtual package, Fedora's `ptyxis`, and an Arch
   `optdepend` on `ptyxis`.

## Consequences

- A missing terminal is a visible, explained failure, not a silent background
  process or a half-started app.
- The resolver is testable with a temporary PATH and fixture executables;
  `tst_desktopentry` covers every source, the ordering, the absent path, a real
  fixture launch, the notification, and the conditional Dock pin.
- `xdg-terminal-exec` is preferred over a hard-coded emulator, so a user's
  configured default terminal wins without the shell learning per-distro
  desktop ids.
- A user can still force a specific terminal with `$TERMINAL`; if that program
  disappears, the shell falls back instead of failing every launch.
- The tar bundle is unmanaged and cannot express a dependency; its README names
  a terminal as recommended.
