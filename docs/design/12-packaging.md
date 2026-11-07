# Packaging and Distribution

## Summary

> **Status (2026-11): design-only.** `packaging/fedora/` and
> `packaging/debian/` still contain only README placeholders. The package
> names, contents, and commands below are the target; Fedora packaging/CI
> (T164), Debian packaging/CI (T165), and the packaged perf re-measure (T166)
> are OPEN. See [STATUS.md](STATUS.md) G-11.

Fedora 44 is the reference platform, but the desktop is designed so very
little knows it is running on Fedora. Fedora already treats alternative
desktops as first-class products (official Sway, COSMIC, Budgie, Xfce spins),
which proves both the distribution model and the parallel-install model.

## Fedora packaging

Initially shipped as multiple RPMs:

```text
dragonfruit-compositor
dragonfruit-shell
dragonfruit-settingsd
dragonfruit-settings
dragonfruit-files
dragonfruit-portal
dragonfruit-session
dragonfruit-desktop        # meta-package
```

Install everything without removing GNOME:

```bash
sudo dnf install dragonfruit-desktop
```

### Package contents

| Package | Contents |
|---|---|
| `dragonfruit-compositor` | Compositor binary, private protocol XMLs + generated bindings |
| `dragonfruit-shell` | Shell process: menu bar, Dock, Control Center, notifications, OSD |
| `dragonfruit-settingsd` | `settingsd`, `menu-broker`, `app-index` (split later if warranted) |
| `dragonfruit-settings`, `dragonfruit-files` | First-party applications, design system assets, and their app icon under `share/icons/hicolor/scalable/apps/` (T-19.1d) |
| `dragonfruit-portal` | `xdg-desktop-portal-dragonfruit` + `portals.conf` for our desktop name |
| `dragonfruit-session` | `dragonfruit-session` binary, `share/wayland-sessions/dragonfruit.desktop`, `bin/dragonfruit-session-entry`, systemd user units and target (installed with `dragonfruit-session --install-session "$RPM_BUILD_ROOT/usr"`) |
| `dragonfruit-desktop` | Meta-package requiring the above |

### Optional dependency markers

The canonical `Recommends`/`Suggests` table is
[`packaging/optional-services.json`](../../packaging/optional-services.json),
shared with Debian and recorded in `NOTICE`. Rules:

- The core desktop must install, start, and work with the required host stack
  only. An optional service is never a hard `Requires`/`Depends`.
- `Recommends` for the preferred backend or strongly expected service
  (restic, GNOME Online Accounts, Orca, speech-dispatcher, ibus/fcitx5,
  PackageKit, Flatpak, CUPS, SANE, colord, fprintd, accountsservice, and the
  distro's firewall adapter).
- `Suggests` for a useful alternate or future delegate (Tracker/Baloo/Recoll,
  Pika Backup/Timeshift, espeak-ng, LocalSend, GNOME Software/Discover, snapd,
  and the non-native firewall adapter).
- Do not require a display manager. Ship the session `.desktop` entry and
  support GDM, SDDM, LightDM, and greetd through the per-DM adapter (ADR
  [0173](adr/0173-display-manager-session-selection-seam.md)).
- The eight naming/licensing decisions are recorded in ADR
  [0229](adr/0229-naming-and-optional-service-decisions.md).

### Parallel-install rules

- Everything we ship lives under the `dragonfruit` namespace: session file
  `dragonfruit.desktop`, D-Bus names `org.dragonfruit.*`, icon theme and
  settings paths likewise — zero file conflicts with GNOME or any other DE.
- `portals.conf` restricts portal backends to
  `xdg-desktop-portal-dragonfruit` plus the generic/GTK backends, so Flatpak
  apps work out of the box.
- Compositor, shell, and protocol XMLs ship as a **lockstep set** per
  release (see [01-architecture.md](01-architecture.md)); packages carry the
  same `%version` and the compositor refuses mismatched protocol versions at
  handshake.
- No package requires, recommends, or obsoletes anything GNOME.
- **Default handlers:** the packages register `org.dragonfruit.Files.desktop`
  as the default `inode/directory` handler, so folder-opening from any app or
  portal routes to Files (see [09-files.md](09-files.md)). The current
  in-tree desktop file already carries that id.
- **First-party app icons (T-19.1d):** each app installs its Tabler-derived
  SVG (`org.dragonfruit.Files.svg`, `org.dragonfruit.Settings.svg`, generated
  by `scripts/gen-app-icons.py`) into `share/icons/hicolor/scalable/apps/`, and
  its `.desktop` `Icon=` carries the same id. A launcher, the global menu, and
  the portal file chooser therefore resolve the same artwork the shell bundles
  at `qrc:/icons/apps/` for nested dev. The design-system resource payload and
  the installed file are generated from one source and guarded by
  `make check-tabler` (ADR [0166](adr/0166-first-party-app-icons.md), ADR
  [0185](adr/0185-tabler-line-icons.md)).

## Hardware baseline

- **Recommended GPU (T-20.4):** a GLES 3.x / Vulkan-capable GPU — any modern
  Intel/AMD/NVIDIA discrete or integrated device, or a virtio-GPU VM with
  `virgl`/`venus`. This is the baseline for the `Full`/`Reduced` liquid-glass
  material (ADR [0182](adr/0182-tahoe-liquid-glass-material-pass.md)); the
  compositor packages do not hard-require it. A renderer that refuses the
  custom texture shaders falls back to a bilinear downsample, and the
  software/headless session runs the deterministic `Minimal` feather stack.
- The packages do not declare a `Recommends` on a GPU driver: the desktop
  starts and is legible on any renderer, and the material degrades honestly (the
  same tier the trace reports).

## Distribution path

1. **COPR** for early testers — Fedora describes COPR as its easy-to-use
   community build system.
2. **Official repositories** once stable.
3. **A Fedora Spin** once genuinely stable — the existence of official Sway
   and COSMIC Spins demonstrates the model.

## Debian / Ubuntu later

Most of the software architecture is unchanged on Debian-derived
distributions because NetworkManager, BlueZ, PipeWire, UPower, D-Bus, Wayland,
and similar APIs provide the abstraction boundary. The large work becomes
packaging, dependency versions, defaults, and distro-specific administration
(COSMIC's upstream documentation lists equivalent dependency families across
Fedora and Debian-derived distributions — a useful precedent).

Distro-specific functionality is isolated behind the `SystemProvider` trait in
`settingsd` (see [08-settings.md](08-settings.md)); everything above that
interface stays identical.

```text
packaging/
├── fedora/
└── debian/
```
