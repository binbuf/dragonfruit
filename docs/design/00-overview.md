# Design Overview

Dragonfruit is a macOS-like desktop environment for Linux. Fedora 44 is the
reference platform; Ubuntu, Debian, and Arch are first-class targets. The
cross-distro contract is in [distro-compatibility.md](distro-compatibility.md).

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
| [22-total-experience.md](22-total-experience.md) | **The whole-desktop target**: shell surfaces, behaviour, motion, HUDs, Settings, ownership |
| [settings-business-logic-map.md](settings-business-logic-map.md) | Every Settings pane/row mapped to its owner and absence path |
| [distro-compatibility.md](distro-compatibility.md) | Fedora/Ubuntu/Debian/Arch host-service and packaging contract |
| [interface-naming-and-licensing.md](interface-naming-and-licensing.md) | Generic names, original assets, third-party substitutions |
| [motion-and-feedback.md](motion-and-feedback.md) | Motion language, HUDs, sound, cursor |
| [evidence-and-acceptance.md](evidence-and-acceptance.md) | Capture, measurement, and acceptance process |
| [STATUS.md](STATUS.md) | **What is implemented, partial, or design-only**, plus the gap register and recommended next sequence |
| [ROADMAP.md](../ROADMAP.md) | Task sequence, definitions of done, and the hardware rail |
| [v0.2.0 roadmap](../v0.2.0/ROADMAP.md) | The total-experience task set |
| [14-risks.md](14-risks.md) | Technical risks, hard limits, and IP constraints |

## Product thesis

**Reproduce the interaction quality and mental model of macOS, not Apple's
bitmap output.**

We own everything that makes the desktop feel like *our* product — the
compositor, workspaces, Overview/Mission Control, Dock, menu bar, Quick
Settings, window decoration policy, Settings, and Files — while reusing the
mature Linux subsystems that have nothing to do with that differentiation
(networking, Bluetooth, audio, power, storage, graphics drivers, portals).

The v0.2.0 target is the **total desktop experience**, not a material alone:
every always-visible shell surface, navigation/window-management mode,
Search/launching path, Settings pane, animation, HUD, and transient feedback.
The target and ownership model are in
[22-total-experience.md](22-total-experience.md); the Settings row map is in
[settings-business-logic-map.md](settings-business-logic-map.md).

Concretely, the goals are:

- macOS-like workflow and spatial organization
- macOS-like Dock behavior and window lifecycle semantics
- macOS-like Overview/Mission Control and workspace interaction
- macOS-like app switcher, Search, Applications drawer, and Quick Settings
- macOS-like global-menu *concept* and Settings information architecture
- Measured motion, HUDs, sound, cursor, and focus/accessibility language
- Every Settings row mapped to a real owner, with an absence path
- Cross-distro compatibility (Fedora, Ubuntu, Debian, Arch) through host
  services and adapters
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
3. **AirDrop/Continuity/Handoff are omitted by default** — there is no
   permissively licensed drop-in with the same UX. Optional LocalSend/KDE
   Connect integration can be evaluated separately; it is not a core promise.
4. **Apple account services are omitted** — iCloud, Siri, Apple Intelligence,
   Screen Time, Wallet, Game Center, Find My, and App Store are not Linux
   services. Their settings rows are removed or replaced by a distro/package
   equivalent.

Everything else on the feature list — Dock behavior, Control Center, Spaces,
Mission Control, Settings, Files, animation language, gestures, and overall
curation — is within our control.

## Non-goals

Boundaries declared up front, so scope debates later have an answer:

- **Not a GNOME theme or extension.** The experience requires owning the
  compositor and shell; theming someone else's desktop cannot deliver it.
- **Not a distribution.** Dragonfruit is a desktop environment installed on
  Fedora, Ubuntu, Debian, Arch, and compatible derivatives. It does not fork
  the kernel, init, package manager, network stack, audio stack, or graphics
  drivers.
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
| Global menu in compatible Linux apps | **Partial today** — native tier shipped; DBusMenu shell consumption/clicks still open | DBusMenu/AppMenu bridge (G-05) |
| Global menu in *every* Linux app | Impossible to guarantee | Application cooperation required |
| Wi-Fi/Bluetooth/audio/power UI | Excellent | Existing system daemons + our UI |
| X11 applications | Good | Xwayland |
| Exact macOS visual copy | Technically easy; legally unwise | Use original visual assets instead |
