# Architecture

## Summary

Dragonfruit is a collection of cooperating processes, not one giant window
manager. The compositor is written in Rust on top of Smithay; the user-facing
shell and first-party applications are built with Qt 6 / Qt Quick (QML). System
functionality is consumed from existing Linux daemons over D-Bus rather than
reinvented.

```text
                          DRAGONFRUIT
┌────────────────────────────────────────────────────────────┐
│                       Qt Quick Shell                       │
│  Menu Bar │ Dock │ Control Center │ Notifications │ OSD   │
└──────────────────────────┬─────────────────────────────────┘
                           │ private Wayland protocol / IPC
┌──────────────────────────▼─────────────────────────────────┐
│                  Rust / Smithay Compositor                 │
│ windows │ Spaces │ Mission Control │ input │ outputs │ FX │
│                  Xwayland compatibility                   │
└───────────────┬───────────────────────────────┬────────────┘
                │                               │
        Wayland clients                   Linux graphics
                │                       DRM/KMS/Mesa/libinput
                │
      ┌─────────┴─────────┐
      │                   │
  Our apps           Third-party apps
Settings / Files    Firefox / Steam / etc.

                     OUR SERVICES
┌────────────────────────────────────────────────────────────┐
│ settingsd │ menu-broker │ app-index │ portal │ session     │
└───────────┬────────────────────────────────────────────────┘
            │ D-Bus / system APIs
            ▼
┌────────────────────────────────────────────────────────────┐
│ NetworkManager │ BlueZ │ PipeWire │ WirePlumber │ UPower  │
│ UDisks │ systemd/logind │ host security services           │
└────────────────────────────────────────────────────────────┘
```

## Ownership boundary

"Owning the desktop" means owning the compositor/shell experience. It does
**not** mean replacing Linux's networking, Bluetooth, audio, storage, input, or
graphics stacks.

| Layer | Decision |
|---|---|
| Window compositor / WM policy | **Own it**, on top of Smithay |
| Mission Control / workspaces / animations | **Own it** |
| Dock, top menu, Control Center, notification center | **Own it** |
| Window-decoration policy | **Own it** |
| System Settings UI and desktop configuration model | **Own it** |
| Finder-equivalent UI and file-manager behavior | **Own it**, reuse filesystem/mount backends |
| Global-menu broker | **Own it**, consume existing app-menu protocols where available |
| Desktop session / startup / portal integration | **Own it** |
| Wi-Fi | Reuse NetworkManager |
| Bluetooth | Reuse BlueZ |
| Audio | Reuse PipeWire + WirePlumber |
| Battery / power-device information | Reuse UPower |
| Disks / removable media | Reuse UDisks2 and GIO/GVfs |
| Authentication for privileged operations | Reuse the host's system services / policy infrastructure |
| Login screen / display manager | Reuse GDM; we ship a Wayland-session entry, not a greeter |
| Graphics drivers / KMS / input / X11 compatibility | Reuse kernel DRM/KMS, Mesa, libinput, libseat/logind, Xwayland |
| Application sandbox integration | Reuse xdg-desktop-portal, implement our DE-specific portal backend |

This is the same philosophical boundary modern desktops take. A serious
desktop is a collection of cooperating processes — separate compositor,
panel/applets, file manager, settings, settings daemon, notifications, OSD,
launcher, screenshot tool, session, idle service, workspaces, greeter, and
portal components — rather than one process.

## Process model

Every process is independently restartable except the compositor, and only
the compositor is special:

| Process | Language | Access | Crash policy |
|---|---|---|---|
| Compositor | Rust / Smithay | Seat, DRM/KMS, input (via logind) | Crash ends the session by design; the display manager returns the user to a login screen |
| Shell (menu bar, Dock, Control Center, notifications, OSD) | Qt Quick | Wayland client of the compositor | systemd restarts it; windows and the session keep running |
| `settingsd` | Rust | User session bus | Restartable; clients re-read cached state on reappearance |
| `menu-broker` | Rust | User session bus | Restartable; menu bar falls back to app-name display |
| `app-index` | Rust | User session bus | Restartable; consumers re-query on reappearance |
| Notifications / OSD / idle | Qt or Rust | User session bus | Restartable; transient state may be lost |
| Portal backend | Rust | User session bus + private compositor protocol | Restartable; portals fail soft |
| Lock screen | Compositor-enforced | Inside the compositor | Fail-secure: no code path, including a crash, unlocks a locked session |

Only the compositor touches the seat, DRM/KMS, or raw input. Everything else
is an ordinary user-session process speaking D-Bus or our private Wayland
protocols. Privileged operations are delegated to system services behind
polkit — nothing of ours runs as root.

## State ownership

Exactly one process owns each class of state; everything else observes:

| State | Owner |
|---|---|
| Windows, focus, stacking, workspaces, outputs | Compositor |
| Application identity (`app_id` → `.desktop`, icon, name) | `app-index` |
| Desktop settings and their persistence | `settingsd` |
| Which menu model belongs to the focused window | `menu-broker` |
| Notification queue, Focus/DND state | Notification service |
| Hardware state (network, audio, power, disks) | Host daemons — we observe and request, never duplicate |

## Technology choices

### Compositor: Rust + Smithay

Smithay is a set of Rust building blocks for Wayland compositors rather than a
complete desktop. It handles the low-level territory — Wayland, input,
DRM/KMS/GBM, EGL, libseat, Xwayland — while leaving desktop policy to us.
COSMIC and Niri demonstrate that Smithay can underpin a complete contemporary
Linux desktop. It is MIT-licensed, which keeps our licensing options open.

The credible alternative is wlroots (composable C modules for KMS/DRM,
libinput, Wayland, X11/headless backends, Xwayland, rendering), which would be
completely defensible for a C/C++ codebase. We prefer Smithay because:

- The project decomposes into many asynchronous state machines and IPC, a
  natural fit for Rust.
- Rust is a good fit for a long-lived, privileged-ish display process.
- COSMIC has already proven the path.

**We study COSMIC heavily but do not fork it.** `cosmic-comp` is GPL-3.0, so a
distributed fork carries GPLv3 obligations for that derivative, and we would
spend increasing time undoing COSMIC-specific product decisions as our
interface diverges.

### Shell and apps: Qt 6 / Qt Quick (QML)

Qt Quick is our choice for Settings, Files, the Dock, menu bar, Control Center,
and similar interfaces because:

- Qt treats animation/transitions as first-class concepts and provides its
  own scene graph / rendering engine — essential for a highly bespoke,
  animated visual system.
- Qt has an established Linux accessibility layer through `QAccessible` /
  AT-SPI.

That combination matters more than having everything in one programming
language.

We do **not** build first-party apps with libadwaita; it would put us in a
recurring fight against another desktop's design language.

### Licensing

Smithay's MIT license keeps our options open, and we reciprocate: the
whole project — compositor, services, private protocol XMLs, design
system, shell, and apps — is MIT, so any third party may implement or
reuse it freely. The final call is a blocker for the first public
release (see [14-risks.md](14-risks.md)).

## Repository layout

```text
desktop/
├── compositor/             # Rust + Smithay
├── protocols/              # private Wayland protocols / IPC schemas
├── shell/
│   ├── menubar/
│   ├── dock/
│   ├── control-center/
│   ├── notifications/
│   └── screenshot/
├── services/
│   ├── settingsd/
│   ├── menu-broker/
│   ├── app-index/
│   ├── files-core/          # Rust browsing/ops library (not a process)
│   └── session/
├── apps/
│   ├── settings/
│   └── files/
├── portal/                 # xdg-desktop-portal-dragonfruit
├── packaging/
│   ├── fedora/
│   └── debian/
└── design-system/
```

## IPC model

Two channels connect our processes:

1. **Private, versioned Wayland protocols** — for compositor↔shell
   communication (workspace enumeration, window state, Mission Control
   control, output configuration). The shell consumes a private, versioned
   interface rather than scraping public protocols.
2. **D-Bus** — for services (settingsd, menu-broker, app-index, portal,
   session) and for talking to system daemons (NetworkManager, BlueZ, PipeWire,
   UPower, UDisks, logind).

### Versioning policy

- Private Wayland protocols are **versioned and additive-only** within a
  stable release: new events and requests may be added; existing ones never
  change meaning or disappear.
- Compositor, shell, and protocol XMLs ship as a **lockstep set** per release;
  cross-version mixing is unsupported and detected at handshake.
- D-Bus interfaces follow the same rule under `org.dragonfruit.*` names, with
  a suffix per major version (e.g. `org.dragonfruit.Settings1`).

## Session startup and shutdown

```text
logind session
  → display manager starts dragonfruit-session.target (systemd --user)
      → compositor                    (acquires seat / DRM master)
      → shell                         (once the private protocol socket exists)
      → settingsd / menu-broker / app-index / notifications   (in parallel)
      → portal backend                (registers with xdg-desktop-portal)
```

Shutdown is the reverse: services stop, the shell unmaps chrome, the
compositor is last to exit. The compositor is the only unit whose death ends
the session (see [11-session-and-dev-workflow.md](11-session-and-dev-workflow.md)).

## Compositor scope discipline

The compositor stays conceptually small: **display, input, surfaces, windows,
workspaces, effects, security boundaries, and shell protocols.**

Do not put Wi-Fi logic or file-manager behavior in it. Hardware-management
logic belongs to the existing daemons; product logic belongs to the shell,
services, and apps.

## Dependency stack

| Area | Dependencies |
|---|---|
| Compositor | Smithay, Wayland, wayland-protocols |
| Graphics | DRM/KMS, GBM, EGL/Vulkan/OpenGL, Mesa |
| Input | libinput, xkbcommon |
| Seat/session | logind / libseat / systemd |
| X11 compatibility | Xwayland |
| Shell/apps | Qt 6, Qt Quick/QML, Qt Wayland |
| IPC | D-Bus + private Wayland protocols |
| Networking | NetworkManager |
| Bluetooth | BlueZ |
| Multimedia | PipeWire, WirePlumber |
| Power | UPower |
| Storage | UDisks2, GIO/GVfs |
| Sandboxed-app integration | xdg-desktop-portal |
| Session | GDM-compatible Wayland session + systemd user units |
