# Dragonfruit

A modern beautiful Linux desktop powered by **Rust, Smithay, and Qt Quick**, inspired by macOS.

[![CI](https://github.com/binbuf/dragonfruit/actions/workflows/ci.yml/badge.svg)](https://github.com/binbuf/dragonfruit/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md) [![Status: Early development](https://img.shields.io/badge/Status-Early%20development-orange.svg)](docs/design/STATUS.md)

<p align="center">
  <img src="docs/images/desktop.jpg" alt="Dragonfruit desktop screenshot" width="100%">
</p>

## Overview

Dragonfruit is a Linux desktop environment, not a theme for another DE, and not a distro.

Our goal is to bring the visual polish, consistency, and intuitive flow of macOS to the Linux desktop experience using your favorite distro.


## Features

- **Supports your favorite distro** Arch, Debian, Fedora, and Ubuntu ( or their derivatives).
- **A complete experience** Dock, menu bar, workspaces, App Drawer, Search, System Settings, and Files.
- **Thoughtful design** modern translucent materials, fluid motion, and consistent interactions.
- **Native foundations** Rust/Smithay Wayland compositor, Qt Quick shell, and integration with your distro's native Linux services.

> [!NOTE]
> Dragonfruit is under active development and is not yet intended as a stable daily-driver desktop.

## Quick start

**Fedora** has been the inital development and testing platform. From an existing Wayland session:

```bash
sudo dnf install rustc cargo qt6-qtdeclarative-devel qt6-qtwayland cmake ninja-build \
  libxkbcommon-devel wayland-devel libdrm-devel mesa-libgbm-devel \
  libinput-devel libseat-devel systemd-devel

git clone https://github.com/binbuf/dragonfruit.git
cd dragonfruit
make dev
```

The Qt 6 Wayland platform plugin (`qt6-qtwayland` on Fedora) is a separate
package on several distributions; `scripts/check-native-deps.sh` checks for it
without root and prints the install command for your distro.

`make dev` builds and runs Dragonfruit **in a window**, without replacing your current desktop. For native-session installation and recovery, run `make help` and review the scripts before proceeding.

## Development

```bash
make build   # Build the desktop
make test    # Run tests
make lint    # Run lint checks

make dev-full # Run DE inside of current DE

make make native # Switch your current DE for this one
make native-rebuild # Build and update this DE on logout
make native-reboot # Build and update this DE on reboot
make native-recover # Switch DE back to your distro default
```

Explore the [design goals](docs/design/22-total-experience.md), [architecture and design docs](docs/design/), [status and roadmap](docs/design/STATUS.md), and [packaging guide](packaging/README.md).

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for more information

## Credit

### Built on Open Source

If there's a high quality open source, license permissible asset or project available for a given feature, no need to re-invent the wheel. We'll review if we can use that instead of building something custom.

| Project | Dragonfruit Implementation |
|---|---|
| **Inter** (SIL OFL 1.1) | Typography |
| **Tabler Icons** (MIT) | Interface line icons |
| **Phosphor Icons** (MIT) | Filled interface and Settings glyphs |
| **WhiteSur icon theme** (GPL-3.0-only) | Folder, Trash, place, Files, and App launcher artwork |
| **Capitaine Cursors** (LGPL) | Cursor theme |
| **Wikimedia Commons** (asset-specific licenses) | Wallpaper discovery |


## License

Dragonfruit is [MIT licensed](LICENSE.md). Third-party components and assets retain their own licenses; see [NOTICE](NOTICE) and `LICENSES/`.

Dragonfruit is an independent open-source project, not affiliated with or endorsed by Apple Inc. macOS is a trademark of Apple Inc.