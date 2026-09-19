# The Testing Ladder

From [../.docs/design/11-session-and-dev-workflow.md](../.docs/design/11-session-and-dev-workflow.md):
development climbs a ladder, fast iteration at the bottom, hostile
environments at the top. Never test a change higher on the ladder than
it needs; never ship a change that has only been tested lower.

```text
fast iteration
    ↓
1. nested compositor in the host session        (daily)
    ↓
2. dedicated-user real Wayland session          (weekly / risky changes)
    ↓
3. VM                                           (crash/supervision work)
    ↓
4. real primary machine                         (pre-release)
    ↓
5. multi-GPU / laptop / dock / NVIDIA machines  (release matrix)
```

## Rung 1 — nested (the daily workflow)

```bash
make dev                 # or: dragonfruit dev --nested
```

The compositor opens as a **window on the host Wayland session** with
its own private socket. The host session is never disturbed: on exit
the dev tool verifies (and fails loudly on) stray processes, stray
sockets, or leaked environment. Covers roughly 80–90% of everyday
development: layouts, menus, Dock, workspace logic, apps, animations,
most protocol work.

Extras:

- `dragonfruit dev --nested --launch CMD` — run programs against the
  nested session (they get `WAYLAND_DISPLAY` and
  `XDG_CURRENT_DESKTOP=dragonfruit` injected).
- `make e2e` — the Foundation milestone harness: one headless session
  with a shell client, a Wayland app, and an X11 app attached at once
  (`.docs/tasks/00-index.md` phase 1). It asserts the private-protocol
  handshake, chrome reserved zones, window/workspace enumeration and
  control, and a clean teardown. It needs no display, so it is the
  scripted form of the "does the whole vertical slice work together"
  check; `compositor/tests/milestone_e2e.rs` is the source.
- `make soak` — the teardown hygiene gate: 100 consecutive clean
  nested exits, zero strays (the Foundation phase exit criterion,
  scripted).
- `dragonfruit dev --soak [N]` — same gate on the headless backend
  (this is what CI runs; no display required).

## Rung 2 — dedicated-user real session (second VT)

Keep the host desktop on one VT and enter Dragonfruit from another.
**Two graphical sessions for the same Unix user collide** through
shared user-session services (portals, `XDG_RUNTIME_DIR` state,
environment). Therefore this rung uses a dedicated development user:

```bash
sudo useradd -m dfdev                       # once
sudo passwd dfdev                           # set a password
# Log into a second VT as dfdev (Ctrl+Alt+F3), then:
dbus-run-session -- target/debug/dragonfruit-compositor --backend nested
```

Until the DRM backend and session units land (T-02, T-24), run the
compositor nested inside the dedicated user's own session — the point
of this rung is **service isolation**, not DRM. Once T-02 lands,
replace the last line with the real DRM backend and, from T-24 on,
with `dragonfruit-session.target` under systemd.

## Rung 3 — VM

Intentionally crash things: compositor, lock screen, portal service,
shell. A VM is disposable; the host is not. Suspend/resume, VT
handoffs, and kill -9 supervision tests belong here (libvirt with
virtio-GPU is the baseline).

## Rung 4 — primary machine

Pre-release: install alongside (never instead of) the host desktop and
register the session with the display manager. A compositor crash ends
the session and the display manager returns — that is the failure
behavior you want while developing a display server.

## Rung 5 — hardware matrix

Multi-GPU, laptop + dock hotplug, suspend/resume cycles, NVIDIA
(soak-scripted, from T-31 onward).
