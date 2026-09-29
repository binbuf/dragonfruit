# Runbook — the Dragonfruit session on real DRM hardware

The exact commands for the **T-03** hardware rail: bring the compositor up on a
real DRM/KMS seat, validate input and multi-GPU import/fallback, soak the
teardown, and record the artifacts. Written so a second person can reproduce
the run without help ([03-real-session-bringup-perf.md](design/tracks/03-real-session-bringup-perf.md),
[testing-ladder.md](testing-ladder.md) rungs 2–3).

On a workstation already running a desktop the host compositor owns DRM master
(`seat0`), so none of this runs there: every step below records an explicit
**OPEN** artifact instead, and the unit is *marked open, not skipped*. The two
ways to get a free seat are the dedicated-user second VT and a VM.

Everything is run from the project root:

```bash
cd /path/to/dragonfruit
make build            # Rust workspace + Qt/CMake shell
```

## 1. Dedicated-user second VT (physical machine)

Two graphical sessions for the same Unix user collide through shared
user-session state, so use a dedicated development user:

```bash
sudo useradd -m dfdev        # once
sudo passwd dfdev            # set a password
```

Switch to a free VT from the host desktop (`Ctrl+Alt+F3`), log in as `dfdev`,
and build the tree there (or use a tree owned by `dfdev`). Then bring the
session up:

```bash
export XDG_RUNTIME_DIR=/run/user/$(id -u)
make build
# the full nested-equivalent session on the real backend, with the shell:
target/debug/dragonfruit dev --drm --shell 2>&1 | tee /tmp/df-drm.log
```

`dev --drm` starts `dragonfruit-compositor --backend drm` (logind/libseat takes
the VT) and, with `--shell`, the built shell against that private socket.
Return to the host with `Ctrl+Alt+F1`; the host session is never closed.

If the session package is installed, the display-manager entry
(`share/wayland-sessions/dragonfruit.desktop` +
`dragonfruit-session-entry`) does the same thing: `sudo dragonfruit-session
--install-session /usr`, then pick **Dragonfruit** at the login screen
([testing-ladder.md](testing-ladder.md) rung 2).

## 2. VM (disposable, easiest free seat)

A VM has its own seat by construction, so it is the fastest path:

```bash
# Boot any recent Fedora/Arch image with a virtio-GPU. In the domain XML:
#   <video><model type='virtio'/></video>
# (or `virt-install --video virtio ...`), then log in as a normal user.
```

Build and run the same commands as §1 inside the VM. The VM is also where the
T-16 crash/suspend suites belong; the host is never at risk.

## 3. The T-03 acceptance runs

Run these on the free seat (physical second VT or VM). Each writes a validated
artifact when the hardware is available, and an explicit `.open.txt` when it is
not — either way the command exits 0.

```bash
make drm-bringup        # DRM backend bring-up + frame trace
make input-validation   # real libinput devices + a non-US layout
make multi-gpu-validation  # import/fallback classification (needs 2 cards)
make drm-soak           # 100-cycle teardown soak + one DRM session cycle
```

| Command | Validated artifact | Open artifact | What it proves |
|---|---|---|---|
| `make drm-bringup` | `docs/captures/t03-drm-loop-trace.txt` | `…-loop.open.txt` | session + GPU + ≥1 output; `SIGUSR1` frame trace |
| `make input-validation` | `docs/captures/t03-input-matrix.txt` | `…-matrix.open.txt` | mouse/keyboard/touchpad gestures/hot corners, a pen, a non-US layout |
| `make multi-gpu-validation` | `docs/captures/t03-multigpu.txt` | `…multigpu.open.txt` | every secondary GPU imports, or falls back with linear-only import |
| `make drm-soak` | `docs/captures/t03-drm-soak.txt` | `…drm-soak.open.txt` | 100 clean teardown cycles + one clean DRM cycle |

### The demo (manual visual walkthrough)

On the free seat, launch the shell with a client and walk the loop the T-03
demo describes — menu bar + Dock on the real display, launch a Dock app,
titlebar/drag/zoom/minimize/restore/close, a workspace switch, then exit:

```bash
target/debug/dragonfruit dev --drm --shell --launch <app>
```

Capture a still from the DRM session with any Wayland screenshot client against
the session socket, e.g.

```bash
# <socket> is the name printed by `dev --drm` (default dragonfruit-dev-<pid>).
WAYLAND_DISPLAY=<socket> grim - > docs/captures/t03-drm-loop-still.png
```

Inspect the still: full desktop, no stray artifacts.

### The DRM session cycle

`make drm-soak` runs the automated teardown soak (100 headless cycles, the same
gate as `make soak`) and then **one DRM cycle** through the same check:

```bash
target/debug/dragonfruit dev --soak 1 --drm
```

For each cycle the tool asserts that the exit left **no leaked launch token**
(`.launch-token`, `.desktop-launch-token`), **no leaked socket or lock**, and
**no orphaned `dragonfruit*` process**. The script then re-probes the card with
`scripts/drm-seat-probe.py`: a card that accepts master again proves the
compositor released the **VT/DRM master**. Run the tool by hand to read the raw
failure line when a cycle is dirty.

Longer soak: `SOAK_CYCLES=N make drm-soak`, or
`dragonfruit dev --soak N --drm` for a single DRM cycle count.

## 4. Teardown expectations

A clean exit removes, for its socket `<name>` in `$XDG_RUNTIME_DIR`:

- `<name>` (the Wayland socket) and `<name>.lock`;
- `<name>.launch-token` and `<name>.desktop-launch-token`;
- every child process the session started (shell, launched apps, services).

On DRM it additionally drops DRM master — `scripts/drm-seat-probe.py` reports
`free_card` for the card it just used. A leftover token is as serious as a
leftover socket: it hands a later client a trusted role.

## 5. Troubleshooting

- **`DRM bring-up: OPEN (no seat: …)`** — the current user cannot take the VT.
  Check `loginctl seat-status seat0`; on a shared host use a dedicated user on a
  second VT or a VM.
- **`DRM bring-up: OPEN (no connected output: master busy …)`** — the card
  opened but the host compositor still holds master. Free the VT or use a VM.
- **`make input-validation` reports `tablet-pen` as a gap** — no pen is present;
  attach one and re-run.
- **`make multi-gpu-validation` writes `.open.txt`** — the seat has a single
  GPU; attach a second card (or a VM with two virtio-GPUs) and re-run.
- **Cargo cannot find the DRM stack** — set `PKG_CONFIG_PATH` /
  `RUSTFLAGS` / `LD_LIBRARY_PATH` at `$HOME/.local/df-devroot/lib64` as the
  Makefile does; `libxkbcommon.so` also needs `$HOME/.local/lib` at runtime.

## See also

- [testing-ladder.md](testing-ladder.md) — rungs 2–3, the seat/session model.
- [design/02-compositor.md](design/02-compositor.md) — the DRM backend and the
  bring-up/input/multi-GPU markers.
- ADRs [0170](design/adr/0170-drm-first-bringup-open-marker-and-no-output-guard.md),
  [0171](design/adr/0171-hardware-input-validation-matrix.md),
  [0172](design/adr/0172-drm-soak-teardown-and-runbook.md).