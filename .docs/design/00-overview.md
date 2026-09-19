# Design Overview

Dragonfruit is a macOS-like desktop environment for Linux, targeting Fedora 44
as the reference platform, with Debian/Ubuntu portability designed in from the
start.

It is a **desktop environment** — a compositor, a shell, services, and
first-party applications cooperating as separate processes — not a window
manager, a GNOME theme, or a set of extensions. The feature list (Mission
Control, Spaces, Dock semantics, window decorations, global menu, animation
policy) is compositor/shell territory, and that is the territory we own.

This directory contains the architecture and core design documents. They are
organized by topic:

| Doc | Topic |
|---|---|
| [01-architecture.md](01-architecture.md) | System architecture, ownership boundary, technology choices, repository layout |
| [02-compositor.md](02-compositor.md) | Compositor design: Smithay, backends, windows, input, Xwayland |
| [03-workspaces.md](03-workspaces.md) | Spaces, Mission Control, and overview interactions |
| [04-shell.md](04-shell.md) | Menu bar, Dock, Control Center, notifications, OSD |
| [05-window-decorations.md](05-window-decorations.md) | Traffic lights and the SSD/CSD decoration policy |
| [06-global-menu.md](06-global-menu.md) | The global menu broker and compatibility tiers |
| [07-system-integration.md](07-system-integration.md) | NetworkManager, BlueZ, PipeWire, UPower, UDisks, portals |
| [08-settings.md](08-settings.md) | Settings app, `settingsd`, and the distro provider model |
| [09-files.md](09-files.md) | Files (Finder-equivalent) application |
| [10-design-system.md](10-design-system.md) | The Qt Quick design system and first-party component library |
| [11-session-and-dev-workflow.md](11-session-and-dev-workflow.md) | Session model, nested development, and the testing ladder |
| [12-packaging.md](12-packaging.md) | Fedora packaging, COPR, Spins, and portability |
| [ROADMAP.md](../ROADMAP.md) | MVP definition, implementation sequence, and timeline |
| [14-risks.md](14-risks.md) | Technical risks, hard limits, and IP constraints |

## Product thesis

**Reproduce the interaction quality and mental model of macOS, not Apple's
bitmap output.**

We own everything that makes the desktop feel like *our* product — the
compositor, workspaces, Mission Control, Dock, menu bar, Control Center,
window decoration policy, Settings, and Files — while reusing the mature Linux
subsystems that have nothing to do with that differentiation (networking,
Bluetooth, audio, power, storage, graphics drivers, portals).

Concretely, the goals are:

- macOS-like workflow and spatial organization
- macOS-like Dock behavior and window lifecycle semantics
- macOS-like Mission Control and Spaces interaction
- macOS-like global-menu *concept* and Settings information architecture
- Left-side three-button window controls wherever technically compatible

## Accepted compromises

Two features are bounded by what third-party applications permit, and we accept
these limits from the outset:

1. **Traffic lights wherever the application permits native desktop
   decoration** — not "all windows." Client-side-decorated applications draw
   their own titlebars; we do not fight them (see
   [05-window-decorations.md](05-window-decorations.md)).
2. **Global menu wherever the application exports one** — applications that do
   not export a menu model keep their in-app menus (see
   [06-global-menu.md](06-global-menu.md)).

Everything else on the feature list — Dock behavior, Control Center, Spaces,
Mission Control, Settings, Files, animation language, gestures, and overall
curation — is within our control.

## Non-goals

Boundaries declared up front, so scope debates later have an answer:

- **Not a GNOME theme or extension.** The experience requires owning the
  compositor and shell; theming someone else's desktop cannot deliver it.
- **Not a replacement for Linux plumbing.** Networking, Bluetooth, audio,
  power, storage, graphics, and sandbox integration stay with NetworkManager,
  BlueZ, PipeWire, UPower, UDisks, DRM/KMS, and xdg-desktop-portal (see
  [01-architecture.md](01-architecture.md)).
- **Not a pixel copy of macOS.** The interaction model is what we reproduce;
  the design assets are ours (see [14-risks.md](14-risks.md)).
- **No live compositor handoff.** Sessions start and end as a unit; there is
  no in-place DE swap (see
  [11-session-and-dev-workflow.md](11-session-and-dev-workflow.md)).
- **No forced universal traffic lights or universal global menu.** Both stop
  where third-party application cooperation ends — the two accepted
  compromises below.

## Feature feasibility summary

| Feature | Feasibility | Where it belongs |
|---|---|---|
| macOS-like menu bar | Excellent | Shell |
| Control Center | Excellent | Shell + system-service adapters |
| macOS-style Dock | Excellent | Shell + compositor IPC |
| Virtual desktops / Spaces | Excellent | Compositor |
| Mission Control | Excellent | Compositor + shell |
| Trackpad Mission Control gestures | Excellent | Compositor / libinput |
| macOS-style Settings | Excellent | Settings app |
| Finder-equivalent | Excellent, but substantial | Files app |
| Left-side traffic lights in our apps | Excellent | Design system / apps |
| Traffic lights in *every* Linux app | Impossible to guarantee | Toolkit/app dependent |
| Global menu in our apps | Excellent | Our app API / menu broker |
| Global menu in compatible Linux apps | Good | DBusMenu/AppMenu bridge |
| Global menu in *every* Linux app | Impossible to guarantee | Application cooperation required |
| Wi-Fi/Bluetooth/audio/power UI | Excellent | Existing system daemons + our UI |
| X11 applications | Good | Xwayland |
| Exact macOS visual copy | Technically easy; legally unwise | Use original visual assets instead |
