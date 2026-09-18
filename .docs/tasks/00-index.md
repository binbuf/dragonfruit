# Dragonfruit — Task Index

High-level implementation tickets derived from the design documents in
[../design/](../design/). Each ticket is a unit of deliverable work with scope,
requirements, acceptance criteria, and dependencies. Tickets reference the
design doc that governs them; when ticket and design doc disagree, the design
doc wins and the ticket gets fixed.

Estimates assume **one strong full-time Linux/Wayland engineer** (see
[../design/13-roadmap.md](../design/13-roadmap.md)): convincing prototype
1–3 months, daily-drivable 6–12 months, arbitrary users beyond a year.

- **S** < 1 week · **M** 1–2 weeks · **L** 2–6 weeks · **XL** > 6 weeks

## Phase map

The phases follow the implementation sequence in
[../design/13-roadmap.md](../design/13-roadmap.md). Phases do not fade into
each other; each has hard exit criteria.

| Phase | Theme | Tickets |
|---|---|---|
| 1 · Foundation | Compositor skeleton, backends, input, windows, Spaces, Xwayland, private protocols | [T-01](01-repo-scaffolding-ci-licensing.md) · [T-02](02-compositor-core.md) · [T-03](03-input-keymaps-shortcuts.md) · [T-04](04-window-model.md) · [T-05](05-spaces-model.md) · [T-06](06-xwayland.md) · [T-07](07-private-shell-protocols.md) |
| 2 · Experience | Design system, menu bar, Dock, Mission Control, app switcher, decorations | [T-08](08-design-system.md) · [T-09](09-menu-bar.md) · [T-10](10-dock.md) · [T-11](11-mission-control-workspace-ux.md) · [T-12](12-app-switcher.md) · [T-13](13-window-decorations-ssd.md) · [T-14](14-hot-corners-desktop-background.md) |
| 3 · Flagship apps | settingsd, Settings, files-core, Files, desktop icons (later) | [T-15](15-settingsd-settings-model.md) · [T-16](16-settings-app.md) · [T-17](17-files-core.md) · [T-18](18-files-app.md) · [T-19](19-desktop-icons.md) |
| 4 · System integration | Service adapters, Control Center, menu broker, app index | [T-20](20-system-service-adapters.md) · [T-21](21-control-center.md) · [T-22](22-global-menu-broker.md) · [T-23](23-app-index.md) |
| 5 · Desktop infrastructure | Session, notifications/OSD, lock/idle, portals, capture, clipboard/auth | [T-24](24-session-lifecycle.md) · [T-25](25-notifications-and-osd.md) · [T-26](26-lock-screen-idle.md) · [T-27](27-portal-backend.md) · [T-28](28-screenshot-recording-ui.md) · [T-29](29-clipboard-auth-agent.md) |
| 6 · Compatibility | DBusMenu bridge, StatusNotifier, decoration themes, Xwayland zoo | [T-30](30-compatibility-bridges.md) |
| 7 · Polish | Multi-monitor, scaling, suspend, drivers, a11y, i18n, perf, crashes | [T-31](31-polish-hardening.md) |
| Cross-cutting | Packaging and distribution | [T-32](32-packaging-distribution.md) |

## The first vertical slice (Phase 1–2 target)

The demo MVP — from [../design/13-roadmap.md](../design/13-roadmap.md):

```text
Compositor: single-monitor output · mouse+keyboard · floating windows ·
focus/move/resize/zoom/fullscreen · three workspaces · basic animations ·
Xwayland · private shell protocol
Shell: menu bar · Dock · clock · Wi-Fi menu · volume · battery ·
Mission Control button/gesture
Apps: Settings · Files
```

The core interaction loop that must be "absurdly polished" — it matters more
than a complete printer page:

```text
launch app → Dock animation → window appears → traffic lights →
workspace switching → Mission Control → minimize → restore from Dock →
app switch → close
```

## Dependency graph (coarse)

```text
T-01 scaffolding
 └─ T-02 compositor core ──┬─ T-03 input ─┬─ T-04 window model ── T-05 Spaces
                           │              └─ T-07 private protocols
                           ├─ T-06 Xwayland
                           └─ T-13 SSD decorations
T-08 design system ─┬─ T-09 menu bar ─┬─ T-10 Dock
                    ├─ T-16 Settings   ├─ T-18 Files
                    └─ T-21 Control Center
T-07 private protocols ─┬─ T-11 Mission Control UX ─ T-12 app switcher
                        ├─ T-09/T-10 (shell chrome)
                        └─ T-28 screenshot UI
T-15 settingsd ── T-16 Settings
T-17 files-core ─┬─ T-18 Files ─┬─ T-19 desktop icons (later)
                 └─ T-27 portal (FileChooser)
T-20 adapters ── T-21 Control Center, T-16 Settings panes
T-22 menu broker ── T-09 menu bar (app menu), T-30 DBusMenu bridge
T-23 app-index ── T-10 Dock, T-12 app switcher
T-24 session ── everything runs under it in real sessions
T-26 lock/idle ── T-24 session, T-02 compositor
T-27 portal ─┬─ T-28 capture UI
             └─ Flatpak/browsers work out of the box
```

## Phase exit criteria

From [../design/13-roadmap.md](../design/13-roadmap.md); tickets within a phase
are not "done" until the phase criterion passes:

1. **Foundation:** nested and DRM sessions both run the vertical slice;
   Xwayland windows map; session teardown is clean (no leaked VT master, no
   orphaned clients).
2. **Experience:** the 30-second interaction loop runs with zero dropped
   frames on baseline Intel/AMD hardware; reduced-motion variants pass.
3. **Flagship apps:** Settings fully configures every pane it ships; Files
   covers the MVP scope; both publish menu models and render identical
   traffic lights.
4. **System integration:** every adapter degrades gracefully when its daemon
   is missing — verified by masking the systemd unit in a VM.
5. **Desktop infrastructure:** the lock screen survives kill tests; portals
   work for a Flatpak browser (file chooser, screenshot, screen share).
6. **Compatibility:** a DBusMenu-exporting Qt app shows a global menu; a
   StatusNotifier tray item renders in the menu bar.
7. **Polish:** multi-monitor hotplug, 100 suspend/resume cycles, and
   fractional scaling pass scripted soak tests in a VM matrix.

## Performance budgets (apply across tickets)

Targets on baseline Intel/AMD hardware, enforced inside the dev loop — never
"discovered" in the polish phase:

| Path | Budget | Verified in |
|---|---|---|
| Workspace switch / Mission Control | 60 Hz, no dropped frames for the full gesture | T-11, T-31 |
| Input-to-photon latency | Under one frame, nested and DRM | T-02, T-03 |
| Idle desktop | Zero damage, zero client wakeups from our shell | T-02, T-09, T-10 |
| Animation system | One frame of compositor work per frame of animation | T-02, T-08 |
| Folder open (warm, 1k items) | < 50 ms to first frame, streaming listing | T-17 |
| List-view scroll (100k items) | 60 Hz windowed rendering, flat memory | T-18 |
| Rename/trash/new folder | Visible within one frame (optimistic) | T-18 |

## Cross-cutting rules

These recur in nearly every ticket; each ticket links back to its source:

- **No duplicated state machines.** Compositor owns windows/workspaces/outputs;
  settingsd owns settings; the shell observes and renders
  ([01-architecture.md](../design/01-architecture.md)).
- **Everything but the compositor is restartable**; compositor death ends the
  session by design ([01-architecture.md](../design/01-architecture.md)).
- **Gesture-driven transitions are progress-based and interruptible.** A
  discrete "instant" code path is a bug
  ([10-design-system.md](../design/10-design-system.md)).
- **Reduced-motion variant required** for every animation
  ([10-design-system.md](../design/10-design-system.md)).
- **Degrade gracefully when a host daemon is absent** — absence is a normal
  state, not an error ([07-system-integration.md](../design/07-system-integration.md)).
- **Reuse the plumbing.** We never write an SMB client, indexer, thumbnail
  daemon, file-watching daemon, credential store, or privilege-escalation
  scheme ([09-files.md](../design/09-files.md),
  [07-system-integration.md](../design/07-system-integration.md)).
- **Original assets only.** Reproduce the interaction model, never Apple's
  bitmap output ([14-risks.md](../design/14-risks.md)).
