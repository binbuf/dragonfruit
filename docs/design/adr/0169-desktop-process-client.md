# 0169 — The Files desktop process is a self-contained offscreen private-protocol client

## Status

accepted

## Context

T-19.3 lands `dragonfruit-files --desktop`: the file manager owning `~/Desktop`
on the compositor's `background` layer (ADR
[0168](0168-desktop-layer-and-role-scoped-layers.md)). The compositor half and
the shared view-selection logic were already in place; the process itself
needed a decision, because `apps/files` had no Wayland dependency and the shell
already owns a large `df_core`/`df_shell` client
(`shell/src/shellprotocol.*`, ~3.5k lines) tied to every chrome surface.

Four things had to be decided once:

1. How the desktop process speaks the private protocol.
2. How it renders the QML scene (icons, labels, rubber band) to a layer buffer.
3. How compositor pointer/keyboard input reaches the QML scene.
4. Who provisions the `desktop:` launch token outside the conformance tests.

## Decision

- **A trimmed, self-contained client, not an extracted shell client.**
  `apps/files/DesktopProtocol.{h,cpp}` connects, presents the desktop token,
  creates exactly one full-output `background` layer surface, and commits
  shared-memory frames. It does not link `dragonfruit-shell-dockcore`, whose
  `ShellProtocol` drags in the Dock/DND/lock victims the desktop never uses.
  The lockstep-version define and wayland-scanner generation mirror the shell's
  `shell/src/CMakeLists.txt`; the browser still builds without libwayland, and
  `--desktop` then reports the missing client instead of linking.
- **Render offscreen, commit the grep.** The process forces
  `QT_QPA_PLATFORM=offscreen` before `QGuiApplication`, loads the
  `Dragonfruit.Files` `DesktopSurface` QML (same `FilesDirectoryModel` +
  `FilesIconView` as a window), `grabWindow()`s the configured output size, and
  commits that `QImage` through the protocol. The scene-graph frame gate
  (self-re-entrancy of `grabWindow`) matches the shell chrome commit path.
- **Reuse the shared pointer-injection helper.** `libs/chromepointer` is
  promoted out of `shell/src` into a small static library linked by both the
  shell and Files, and `ChromePointer::send` grows an optional modifiers
  argument. The desktop's Cmd-click/Shift-click selection and its rubber band
  therefore go through the one constructor that owns the monotonic timestamp
  (the T-16.12 invariant), instead of introducing a second injection site.
  `DesktopInput.h` holds the pure evdev/xkb→Qt mapping and is unit-tested
  without a compositor.
- **Input fidelity is the client's job.** The private protocol delivers raw
  `wl_pointer` press/release pairs, so `FilesDesktop` pairs a second left press
  within the platform double-click interval/distance and emits Qt's
  `MouseButtonPress` + `MouseButtonDblClick` — the sequence a `MouseArea`'s
  `onDoubleClicked` needs. Pairing is the pure, unit-tested
  `desktopIsDoubleClick` (`DesktopInput.h`). A directory open spawns the browser
  with `QT_QPA_PLATFORM` removed from its environment: the desktop forces
  `offscreen` for its own window, and the browser must map as a normal Wayland
  client.
- **Independent crash domain.** The desktop is its own process on the
  `background` layer; its death leaves the compositor, shell, and browser
  running, and vice versa. The protocol half is asserted by
  `shell_protocol_conformance::desktop_client_death_leaves_the_shell_and_session_alive`.
- **Dev-tool provisioning now; session manager later.** The compositor only
  writes `<socket>.desktop-launch-token` when it is handed a desktop token, so
  `dragonfruit` (`make dev`/`make demo`) starts the compositor with a random
  `DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN`, then launches
  `dragonfruit-files --desktop` with the hand-off file's token. The systemd
  session entry does not yet mint the token; that is the remaining follow-up.

## Consequences

- The shell protocol client stays shell-shaped; the desktop's client is small
  and independently testable. If the two clients later grow a common core, this
  ADR is the place to revisit the split.
- `libs/chromepointer` is now the single injected-pointer constructor; a new
  chrome surface must link it rather than build its own `QMouseEvent`.
- The desktop lists `~/Desktop` resolved through `QStandardPaths`
  (`FilesArguments.filesDesktopDirectory`), falling back to `$HOME/Desktop`; a
  missing directory is the normal empty desktop.
- Keyboard input is limited to navigation/activation/Delete/select-all from the
  trimmed map in `DesktopInput.h`; the inline rename editor's text entry is a
  later slice, as are the background menu's richer actions.