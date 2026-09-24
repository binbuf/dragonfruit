# Private Shell Protocols (T-07)

Compositor↔shell communication runs over **private, versioned Wayland
protocols** — never by scraping public protocols
([01-architecture.md](../docs/design/01-architecture.md)). The XMLs live in
[`protocols/`](../protocols/) and ship as a lockstep set with the compositor
and shell; the versioning rules are in
[ipc-versioning.md](ipc-versioning.md).

```text
protocols/
├── dragonfruit-core.xml       df_core          handshake + trust model
├── dragonfruit-shell.xml      df_shell         chrome surfaces (layer-shell-style)
│                              df_layer_surface
└── dragonfruit-toplevel.xml   df_toplevel_manager  windows / workspaces / outputs
                               df_toplevel
                               df_workspace
                               df_output
```

## The lockstep handshake (`df_core`)

`df_core` is the only private global advertised unconditionally. Binding it
grants nothing; a client must present the **one-time launch token** it was
provisioned with out-of-band at startup (session environment, T-24) and the
lockstep version it speaks:

```text
authenticate(lockstep_version, token)
  → authenticated(lockstep_version)   on success
  → refused(code, message) + disconnect on failure
```

Refusal codes (additive-only):

| Code | Name | Meaning |
|---|---|---|
| 0 | `invalid-token` | unknown token, or one minted by a previous boot |
| 1 | `token-consumed` | the one-time token was already redeemed |
| 2 | `version-mismatch` | lockstep sets disagree |
| 3 | `not-provisioned` | no token was presented |
| 4 | `already-authenticated` | this client already authenticated |

Every other private global is advertised but its `bind` is refused with a
protocol error unless the client authenticated. Refused binds and handshakes
are logged.

### Trust model

The model lives in
[`compositor/src/shell/trust.rs`](../compositor/src/shell/trust.rs) and is
deliberately pure so the refusal matrix is unit-tested without a live
compositor:

- **One-time.** A token is consumed by the first successful handshake;
  replaying it is refused (FR-5).
- **Per-boot.** Tokens are bound to the compositor boot that minted them; a
  token captured in a previous session is refused (FR-5).
- **Versioned.** The handshake compares lockstep versions before the token,
  so a misconfigured shell does not burn its token (FR-6).
- **Role-scoped.** A token admits the role it was minted for (`shell`,
  `desktop-icons`).

**Provisioning.** At startup the compositor mints a shell token and writes it
0600 to `$XDG_RUNTIME_DIR/<socket>.launch-token`, then removes the file at
teardown. `DRAGONFRUIT_LAUNCH_TOKENS` (comma-separated hex) provisions
additional tokens — used by the session manager and the conformance tests;
`DRAGONFRUIT_LAUNCH_TOKEN` is the single-token hand-off the session manager
exports to the shell process (T-24). Tokens are never logged (`Debug`
redacts the value).

## Chrome surfaces (`df_shell`, `df_layer_surface`)

`df_shell.get_layer_surface` turns an existing `wl_surface` into a chrome
surface and places it on a layer. `df_layer_surface` requests set the anchor
edges, size, margins, exclusive (reserved) zone, and keyboard-interaction
mode; the compositor replies with `configure(serial, width, height)`, which
the client must `ack_configure`.

- **Anchors** are a bitfield (`top=1, bottom=2, left=4, right=8`). A zero
  size stretches between both anchors on an axis (or the whole output when
  unanchored); an unanchored axis is centred.
- **Reserved zones** (`set_exclusive_zone`): a positive value on a single
  vertical edge reserves that edge. The compositor folds all chrome reserves
  into the usable (Zoom) area and reports them to every `df_output` as
  `reserved_zone(edge, thickness)`; a negative value opts a surface out
  (overlays).
- **Keyboard interaction**: `none`, `exclusive` (modal, e.g. OSD),
  `on_demand` (focus on click). The compositor tracks the mode; the shell's
  rendering and the seat grab are T-09/T-10.

Layers match `wlr-layer-shell`'s ordering: `background`, `bottom`, `top`,
`overlay`. The menu bar and Dock are `top`; menus and the OSD are `overlay`.

> **Out of scope for T-07:** rendering the chrome. The compositor owns
> placement, reserved zones, and configuration; the shell draws it
> (T-09/T-10).

## Windows, workspaces, and outputs (`df_toplevel_manager`)

The manager is per-client. On bind the compositor replays the current scene:
it announces every `df_output`, `df_workspace`, and `df_toplevel`, sends each
object's initial properties, and emits `done`. Every state-changing request
is acked with `done` after the scene applies it (FR-3).

- **`df_toplevel`** is a compositor-stable window handle. The compositor
  never makes the shell invent ids: the handle maps to a `WindowId`.
  Requests: `activate`, `close`, `minimize`, `unminimize`, `zoom`,
  `unzoom`, `fullscreen`, `unfullscreen`, `move_to_workspace`. Events:
  `title`, `app_id`, `state` (bitfield `minimized|zoomed|fullscreen|focused`),
  `workspace_entered`/`left`, `output_entered`/`left`, `closed`, `done`.
- **`df_workspace`** is one Space on one output. Spaces are per-output and
  ordered; activation is lockstep across outputs
  ([03-workspaces.md](../docs/design/03-workspaces.md)). A fullscreen window
  owns a dedicated, transient Space (`fullscreen` true). Requests: `activate`,
  `set_wallpaper`; events: `name`, `index`, `activated`, `fullscreen`,
  `wallpaper`, `removed`, `done`.
- **`df_output`** follows `wlr-output-management`: `name`, `geometry`,
  `mode`, `scale`, `transform`, `vrr`, `night_light`, `reserved_zone`,
  `done`, `removed`. Requests set mode/scale/transform (applied through
  Smithay's `Output::change_current_state`); VRR and night light are accepted
  and acked but their backend plumbing is a T-16 displays-pane item (see
  [PROGRESS.md](tasks/legacy/PROGRESS.md), T-02 (legacy)).
- **Manager requests** include the cross-cutting controls: workspace
  create/remove/reorder/activate, `enter`/`exit_mission_control`,
  `select_overview_toplevel`, `activate_app`, `cycle_app_switcher`,
  `release_keyboard_focus` (v2), `set_reduced_motion` (v3), and
  `set_launch_origin` (v4). The last hands the Dock entry's tile rectangle to
  the compositor so a launching app's window appears from it (T-02.1b); the
  compositor falls back to a centered origin when it is never sent.
- **Manager events** carry the cross-cutting broadcasts: `workspace_activated`,
  `focused`, `attention` (xdg-activation / demands-attention → Dock bounce),
  `hot_corner` (the same event whether triggered by pointer, gesture, or
  keyboard), `overview_changed` (Mission Control), `app_switcher`,
  `input_action`, `progress`, and `app_accelerator`.

### Scene-consistent ordering (FR-2)

The manager never describes a state the scene has not reached. Workspace
events are recorded in the same call that mutates the model, before any scene
change or render pass; the broadcaster drains the T-03/T-04/T-05 outboxes in
one loop iteration, in order, before the client flush. A structural
workspace/output change re-syncs the object list from the model rather than
trusting a lossy event id, so the shell can never observe a frame where its
state and the compositor's disagree.

## Deviations from the wlr precedents

We studied the wlr protocols and own our own; the meaningful deviations:

| Area | wlr precedent | Dragonfruit | Why |
|---|---|---|---|
| Access | public extension surface | token-gated, trusted processes only | chrome is a privilege, not a public API ([02-compositor.md](../docs/design/02-compositor.md)) |
| Versioning | per-interface `version` | lockstep set + handshake refusal | cross-version mixing is unsupported ([01-architecture.md](../docs/design/01-architecture.md)) |
| Window identity | `wlr-foreign-toplevel` handle | compositor-stable `WindowId` in the handle | survives shell restarts; the shell never invents ids |
| Workspaces | none | first-class `df_workspace` + lockstep activation | Spaces are a compositor primitive (T-05) |
| Mission Control / app switcher | none | `overview_changed`, `app_switcher`, recency order | compositor-driven, shell-rendered ([04-shell.md](../docs/design/04-shell.md)) |
| Launch feedback | `xdg-activation` is public | `attention` broadcast to the shell | Dock bounce (T-10) without polling |
| Output extras | mode/scale/transform | + VRR, night light, reserved zones | Displays pane (T-16) and chrome reserves |
| Capture | `wlr-screencopy` exists | never advertised; portal-only | "if a capture is not a portal request, the answer is no" |
| Failure model | protocol error | `refused` + disconnect, logged | handshake is an auditable event |

We deliberately do **not** design a public extension surface. Third-party
requests to bind the chrome protocols are answered no by design. If an
upstream `ext-` staging standard lands (e.g. `ext-workspace`), we prefer
consuming it and shrinking ours — but only inside a major-version boundary.

## Client bindings

Both bindings are generated from the same XMLs:

- **Rust** (services, the compositor, tests): `wayland-scanner`'s
  `generate_client_code!` / `generate_server_code!`. The compositor's server
  bindings are in
  [`compositor/src/shell_protocol/mod.rs`](../compositor/src/shell_protocol/mod.rs);
  the conformance suite generates client bindings from the same files.
- **Qt/C++** (shell, T-08): generate with Qt's `qtwaylandscanner` (or
  `wayland-scanner client-header` + `private-code`) against the same XMLs.
  `scripts/gen-shell-protocol-bindings.sh` records the exact commands; the
  script is a no-op when the toolchain is absent.

## Compliance

`compositor/tests/shell_protocol_conformance.rs` is the compliance client.
It spawns the headless compositor, provisions tokens, and exercises:

- the handshake and the full refusal matrix (untrusted bind, invalid token,
  wrong version, replayed token) — FR-4/5/6;
- chrome placement/configure and reserved zones — FR-1;
- output/workspace/toplevel enumeration and request round-trips with `done`
  acks — FR-2/FR-3;
- per-window requests (`zoom`/`unzoom`, `minimize`/`unminimize`,
  `fullscreen`/`unfullscreen`, `move_to_workspace`, `close`).

## Open items

- **Qt shell bindings** are documented but not yet consumed; the shell lands
  in T-08.
- **VRR / night light / per-output color** are accepted and acked but not
  plumbed to the backend (T-16).
- **Chrome rendering, keyboard-mode seat grabs, and layer stacking** are
  T-09/T-10; T-07 owns placement and reserved zones.
- **Reserved zones are a single global union**, not per-output; multi-monitor
  per-output zones are T-11/T-16.
- **Toplevel thumbnails** for the Dock window chooser are not part of the
  protocol yet; if added they must be token-gated like every private
  interface (T-10 decision).
