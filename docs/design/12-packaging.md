# Packaging and Distribution

## Summary

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
| `dragonfruit-settings`, `dragonfruit-files` | First-party applications, design system assets |
| `dragonfruit-portal` | `xdg-desktop-portal-dragonfruit` + `portals.conf` for our desktop name |
| `dragonfruit-session` | `dragonfruit-session` binary, `share/wayland-sessions/dragonfruit.desktop`, `bin/dragonfruit-session-entry`, systemd user units and target (installed with `dragonfruit-session --install-session "$RPM_BUILD_ROOT/usr"`) |
| `dragonfruit-desktop` | Meta-package requiring the above |

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
- **Default handlers:** the packages register `dragonfruit-files.desktop` as
  the default `inode/directory` handler, so folder-opening from any app or
  portal routes to Files (see [09-files.md](09-files.md)).

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
