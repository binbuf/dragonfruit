# Dragonfruit portable bundle

This is a plain install tree, not a distro package. It installs the
compositor, shell, session units, first-party apps, portal backend, icons,
translations, and the shipped wallpaper under `/usr`.

## Install

```bash
tar -xzf dragonfruit-<version>-linux-<arch>.tar.gz
cd dragonfruit-<version>-linux-<arch>
sudo ./install.sh
```

Log out, then choose **Dragonfruit** at the login screen. The bundle does not
preselect a session, arm autologin, change default applications, or remove
your current desktop.

To upgrade, run the new bundle's `install.sh` over the old one. The bundle is a
plain file tree, so files removed between versions are not pruned
automatically; use the RPM/DEB for managed upgrades.

To stage without touching the host:

```bash
DESTDIR=/tmp/dragonfruit-root ./install.sh
```

## Uninstall

```bash
sudo ./uninstall.sh
```

The uninstaller uses the manifest written during staging, so it only removes
files this bundle installed. It keeps user settings under
`~/.config/dragonfruit/` and state under `~/.local/state/dragonfruit/`.

## Runtime requirements

The bundle is dynamically linked. It needs:

- Qt 6.8 or newer (Core, Gui, Quick, DBus, Network, the Wayland platform
  plugin, and the QtQuick / Shapes / VectorImage QML modules)
- Wayland, libxkbcommon, DRM/GBM, libinput, libseat, udev/systemd, D-Bus
- a systemd user manager and a Wayland-capable display manager
- Xwayland for X11 applications (optional, but expected on a desktop)

Use the Fedora RPM or Debian package when one is available: the package
manager installs the matching runtime dependencies and tracks upgrades.
