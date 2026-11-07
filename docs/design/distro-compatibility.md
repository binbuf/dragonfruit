# Cross-distribution compatibility

| | |
|---|---|
| **Status** | target design; adapter contract landed (T221) |
| **Date** | 2026-11-07 |
| **Reference distro** | Fedora 44 |
| **Supported distros** | Fedora 44+, Ubuntu 24.04+/26.04+, Debian 13+, Arch Linux (rolling) |
| **Related** | [distro-adapter-contract.md](distro-adapter-contract.md), [22-total-experience.md](22-total-experience.md), [settings map](settings-business-logic-map.md), [naming/licensing](interface-naming-and-licensing.md), [ADR 0221](adr/0221-cross-distro-host-service-contract.md), [ADR 0230](adr/0230-distro-adapter-contract-and-stubs.md) |

## 1. Goal

Dragonfruit is a **desktop environment**, not a distribution. It must install
and behave coherently on the main Linux distributions, using the host's
existing system services and package manager rather than replacing them.

The compatibility goal is:

- one codebase, one shell, one compositor;
- host-service adapters where distributions differ;
- no distro-specific commands in QML or shell code;
- no hard dependency on a single init system beyond systemd/logind (the four
  supported distros all use systemd);
- a clean install and first login on each supported distro;
- a documented absence path for every optional service.

The concrete adapter contract, trait set, per-distro command matrix, dependency
sets, and probe live in
[distro-adapter-contract.md](distro-adapter-contract.md). The rest of this
document is the policy the contract implements.

## 2. Support tiers

| Tier | Meaning | Distros |
|---|---|---|
| **Tier 1** | CI install test, full feature matrix, release blocker | Fedora 44+, Ubuntu 24.04 LTS, Debian 13 |
| **Tier 2** | CI install test, core feature matrix, documented gaps | Arch Linux, Ubuntu 26.04+ |
| **Tier 3** | Best-effort; same host services; community-tested | Derivatives (Bazzite, Nobara, Pop!_OS, Mint, EndeavourOS, Manjaro) |
| **Unsupported** | No systemd/logind, no Wayland/DRM, no NetworkManager/BlueZ/PipeWire | — |

The desktop session itself requires:

- Linux with DRM/KMS and a working Mesa/GPU driver;
- Wayland (the compositor owns the session);
- systemd + logind (session, idle, suspend, user units);
- libinput + udev;
- Xwayland for X11 apps;
- NetworkManager, BlueZ, PipeWire/WirePlumber, UPower, UDisks2;
- polkit;
- xdg-desktop-portal + a desktop portal backend.

## 3. Host-service matrix

| Capability | Host service | Fedora | Ubuntu/Debian | Arch | Dragonfruit owner |
|---|---|---|---|---|---|
| Networking | NetworkManager | `NetworkManager` | `network-manager` | `networkmanager` | adapter |
| Wi-Fi | NetworkManager + wpa_supplicant/iwd | ✓ | ✓ | ✓ | adapter |
| Bluetooth | BlueZ | `bluez` | `bluez` | `bluez` | adapter |
| Audio | PipeWire + WirePlumber | ✓ | ✓ | ✓ | adapter |
| Power | UPower + power-profiles-daemon | ✓ | ✓ | ✓ | adapter |
| Storage | UDisks2 | `udisks2` | `udisks2` | `udisks2` | adapter |
| Printing | CUPS | `cups` | `cups` | `cups` | adapter |
| Scanning | SANE | `sane-backends` | `sane-utils` | `sane` | adapter |
| Accounts | accountsservice | `accountsservice` | `accountsservice` | `accountsservice` | adapter |
| Auth | polkit + PAM | ✓ | ✓ | ✓ | adapter |
| Updates | PackageKit or native | `dnf5`/PackageKit | `apt`/PackageKit | `pacman`/PackageKit | distro adapter |
| Firewall | firewalld/ufw/nftables | firewalld | ufw | nftables/iptables | distro adapter |
| Time | systemd-timesyncd/chrony | chrony | systemd-timesyncd | systemd-timesyncd | distro adapter |
| Locale | systemd-localed + locale | `localed` | `locale` | `localed` | distro adapter |
| Color | colord | `colord` | `colord` | `colord` | adapter |
| Fingerprint | fprintd | `fprintd` | `fprintd` | `fprintd` | adapter |
| Online accounts | GNOME Online Accounts | `gnome-online-accounts` | `gnome-online-accounts` | `gnome-online-accounts` | optional adapter |
| Search | Tracker/Baloo/Recoll | optional | optional | optional | search service |
| Accessibility | AT-SPI, Orca, speech-dispatcher | ✓ | ✓ | ✓ | adapter/delegate |
| IME | ibus/fcitx5 | ✓ | ✓ | ✓ | adapter |
| Portals | xdg-desktop-portal + backend | ✓ | ✓ | ✓ | portal |
| Display manager | GDM/SDDM/LightDM/greetd | GDM/SDDM | GDM/SDDM/LightDM | SDDM/greetd | session adapter |

## 4. Application discovery and installation

### 4.1 Discovery paths

The app index must search, in order:

1. `$XDG_DATA_HOME/applications` (`~/.local/share/applications`);
2. each `$XDG_DATA_DIRS/applications` (`/usr/local/share/applications`,
   `/usr/share/applications`);
3. Flatpak exports:
   - `~/.local/share/flatpak/exports/share/applications`;
   - `/var/lib/flatpak/exports/share/applications`;
4. Snap desktop files:
   - `/var/lib/snapd/desktop/applications`;
5. AppImage/`.desktop` files installed by the user or a helper;
6. Nix profiles when `NIX_PROFILES` is set (best effort);
7. distro-specific extra paths only through a declared adapter.

Rules:

- Deduplicate by desktop id; the user-level file wins over the system file.
- Respect `NoDisplay`, `Hidden`, `OnlyShowIn`, `NotShowIn`, `TryExec`.
- Resolve `Exec` with the desktop-entry spec; never assume a shell.
- Resolve icons through the freedesktop icon theme and the app's `Icon=`.
- Record the source (system/user/flatpak/snap/appimage) for Settings and
  diagnostics.
- A missing `TryExec` marks the entry unavailable but does not delete it.

### 4.2 Installation sources

Dragonfruit does not ship an App Store. The "get apps" path is:

| Source | Integration |
|---|---|
| Distro repository | PackageKit or native package manager adapter |
| Flathub | Flatpak CLI/portal; discover and install through PackageKit/Flatpak |
| Snap | Snapd CLI/portal where installed |
| AppImage | User-managed file; optional integration helper |
| Local package | File chooser + package installer portal |

The Settings app's "Software Sources" row reports which sources are available;
it never claims a source that is not installed.

### 4.3 Launch environment

- Launch through the session's `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, and
  `DBUS_SESSION_BUS_ADDRESS`.
- Do not leak the shell's `QT_QPA_PLATFORM=offscreen` to children.
- Pass the desktop file's `Exec` arguments unchanged; expand `%f`, `%F`, `%u`,
  `%U` per spec.
- Set `XDG_ACTIVATION_TOKEN`/`DESKTOP_STARTUP_ID` where supported.
- Report launch failure with the desktop id and the resolved command.

## 5. Startup applications and login items

### 5.1 XDG autostart

- User: `$XDG_CONFIG_HOME/autostart` (`~/.config/autostart`).
- System: `/etc/xdg/autostart`.
- Respect `Hidden`, `OnlyShowIn`, `NotShowIn`, `X-GNOME-Autostart-enabled`,
  `X-KDE-autostart-after`, and `TryExec`.
- The Settings "Start at login" toggle writes a user override, not the system
  file.
- Do not start a second instance if the app is already running.

### 5.2 systemd user units

- Show units with `WantedBy=default.target` or a desktop-specific target.
- Enable/disable through `systemctl --user`; system units through polkit.
- Do not invent a second autostart format; XDG autostart and systemd user units
  are both first-class.
- The session manager starts the shell, compositor, and services as user units;
  the desktop process is a unit, not a dev-only token (G-09).

### 5.3 Per-distro differences

| Distro | Autostart | User units | Notes |
|---|---|---|---|
| Fedora | XDG + systemd | ✓ | SELinux may restrict some paths |
| Ubuntu | XDG + systemd | ✓ | Snap autostart may add desktop files |
| Debian | XDG + systemd | ✓ | Minimal install may lack some services |
| Arch | XDG + systemd | ✓ | User must enable services; document |

## 6. Users, groups, and admin authentication

### 6.1 User/group model

- Read users/groups through accountsservice (`org.freedesktop.Accounts`).
- Write through the distro adapter:
  - `useradd`/`usermod`/`userdel`;
  - `groupadd`/`groupmod`/`groupdel`;
  - `passwd`/`chpasswd` (never log the password);
  - `gpasswd` for group membership.
- Admin group mapping:
  - Fedora/Arch: `wheel`;
  - Ubuntu/Debian: `sudo` (and `admin` on older releases);
  - detect both, never hardcode one.
- Never edit `/etc/passwd`, `/etc/shadow`, or `/etc/group` directly.
- Use polkit for privileged operations; the Settings app is not setuid.

### 6.2 Password and fingerprint

- Password change through PAM/`passwd` with the distro adapter.
- Fingerprint enrollment through fprintd.
- PAM configuration for fingerprint unlock is distro-specific; the adapter
  detects `pam_fprintd` and enables the toggle only when present.
- Fast user switching is display-manager-specific; support GDM/SDDM/LightDM
  where the API exists, otherwise hide the row.

### 6.3 Automatic login

- GDM: `AutomaticLoginEnable`/`AutomaticLogin` in `custom.conf`.
- SDDM: `[Autologin]` in `sddm.conf`.
- LightDM: `autologin-user` in `lightdm.conf`.
- greetd: `initial_session` in `config.toml`.
- The adapter writes the correct file with polkit; the UI is identical.

## 7. Software updates and package management

### 7.1 Abstraction

```rust
trait UpdateProvider {
    async fn distribution_info(&self) -> DistributionInfo;
    async fn check_updates(&self) -> Result<Vec<Update>>;
    async fn install_updates(&self, ids: &[UpdateId]) -> Result<UpdateProgress>;
    async fn reboot_required(&self) -> Result<bool>;
}
```

Backends:

| Distro | Backend | Notes |
|---|---|---|
| Fedora | `dnf5` (native) or PackageKit | Offline updates via `dnf offline` |
| Ubuntu/Debian | `apt` (native) or PackageKit | `unattended-upgrades` policy |
| Arch | `pacman` (native) or PackageKit | Rolling; no offline-update equivalent |
| Fallback | PackageKit | Use when the native backend is unavailable |

### 7.2 Rules

- Never run a package manager from QML.
- Never parse human-readable output when a machine-readable API exists.
- Show progress, cancel where the backend supports it, and a clear reboot
  prompt.
- Do not silently enable automatic updates; the default is the distro's policy.
- Record the exact command/API and result for diagnostics.
- Flatpak/Snap updates are separate sources; show them in the same UI with
  source labels.

## 8. Default applications, MIME, and portals

- Default apps: `xdg-mime`, `xdg-settings`, and `mimeapps.list` under
  `$XDG_CONFIG_HOME`; the Settings portal exposes the same state.
- Never edit `mimeapps.list` by hand when the portal API is available.
- File chooser, screenshot, screencast, settings, global shortcuts, and
  permission store come from xdg-desktop-portal.
- The portal backend is selected by the session (`xdg-desktop-portal-gtk`,
  `-gnome`, `-kde`, or Dragonfruit's own backend). Dragonfruit ships its own
  backend for the portals it owns; the rest fall back to the host backend.
- A missing portal shows a disabled row with a reason; it never silently
  falls back to an unsafe path.

## 9. Network, firewall, and VPN

- NetworkManager is the only network backend. Do not support
  `systemd-networkd` as a second model in the first release.
- Wi-Fi, Ethernet, VPN, bridges, bonds, VLANs, and connection profiles come
  from NM.
- Firewall:
  - Fedora: firewalld D-Bus;
  - Ubuntu: ufw (CLI/`ufw` D-Bus if available);
  - Debian: nftables/ufw;
  - Arch: nftables/iptables.
  The adapter exposes a common `FirewallStatus`/`set_enabled`/`open_port` API.
- If no supported firewall is present, the pane shows `No firewall detected`
  and does not offer a fake toggle.
- VPN: NetworkManager VPN plugins; the UI lists installed plugins and hides
  unavailable providers.

## 10. Power, audio, Bluetooth, printing, storage

### 10.1 Power

- UPower for battery/AC/health.
- power-profiles-daemon for profiles where present; fall back to
  `/sys/firmware/acpi/platform_profile` or hide the row.
- logind for suspend/hibernate/poweroff/reboot.
- TLP is optional; do not fight it. If TLP is active, show its state and
  disable conflicting Dragonfruit power controls.

### 10.2 Audio

- PipeWire + WirePlumber are the target.
- PulseAudio is not a supported second backend in the first release; if
  PipeWire is absent, the Sound pane shows an explicit note.
- Device routing, volume, balance, mute, and profiles come from PipeWire.
- UI sounds use the same output; volume feedback respects the setting.

### 10.3 Bluetooth

- BlueZ is the only backend.
- Adapter power, discoverability, pairing, connected devices, and battery
  where exposed.
- LE Audio depends on BlueZ/PipeWire support; hide unsupported codecs/features.

### 10.4 Printing

- CUPS for printers/queues; SANE for scanners.
- Add/printer discovery through CUPS; scanner discovery through SANE.
- A missing CUPS/SANE shows a note and disables only that half.

### 10.5 Storage

- UDisks2 for disks, partitions, mounts, and removable media.
- Formatting/partitioning is delegated to GNOME Disks or a future first-party
  tool; the Settings pane does not reimplement it in the first release.
- LUKS status is read-only; unlocking is handled by the distro's cryptsetup
  integration.

## 11. Online accounts, search, backup, accessibility, IME

### 11.1 Online accounts

- GNOME Online Accounts (`org.gnome.OnlineAccounts`) if installed.
- The Settings pane lists providers, add/remove, and sync toggles.
- If GOA is absent, the pane is hidden (not a dead entry).
- No first-party OAuth implementation in the first release.

### 11.2 Search

- The search service owns the index.
- Optional backends:
  - Tracker/Tracker Miners (GPL, separate process);
  - Baloo (LGPL, separate process);
  - Recoll (GPL, separate process);
  - built-in filename index (default, no external dependency).
- The Settings pane selects the backend and indexed locations; it never
  claims content indexing that the backend does not provide.
- Privacy: excluded paths, no network indexing by default.

### 11.3 Backup

- Optional backends: restic (BSD-2-Clause), Pika Backup (GPL, separate app),
  Timeshift (GPL, separate app).
- Dragonfruit's first-party Backup pane is a thin front end; if no backend is
  installed, it shows a note and offers installation instructions.
- Do not ship a custom backup format or a privileged backup daemon in the first
  release.

### 11.4 Accessibility

- AT-SPI is the accessibility bus.
- Orca is the screen reader; Dragonfruit provides the toggle and settings, not
  a new screen reader.
- speech-dispatcher + a synthesizer (espeak-ng/Piper) for read-and-speak.
- The compositor magnifier is first-party.
- The shell exports its chrome to AT-SPI (G-19); this is a release gate for the
  accessibility task.
- If a delegate is absent, the row is disabled with an explanation.

### 11.5 Input methods

- Support ibus and fcitx5 as separate processes.
- Detect the running framework; do not start both.
- The compositor advertises `text-input`/`input-method`; the IME candidate
  window is the framework's or a first-party minimal one (decide in T253).
- Settings exposes input sources and the framework choice; it does not embed
  the IME engine.

## 12. Display manager and session

Supported display managers:

| DM | Integration | Notes |
|---|---|---|
| GDM | Wayland session entry, autologin, user list | GNOME dependency; common on Fedora/Ubuntu |
| SDDM | Wayland session entry, autologin | Common on KDE/Arch |
| LightDM | Session entry, autologin | Debian/Ubuntu minimal |
| greetd | Session entry, autologin | Lightweight/Arch |

Rules:

- Ship a `.desktop` session file (`dragonfruit.desktop`) with
  `Exec=dragonfruit-session`, `Type=Application`, `DesktopNames=Dragonfruit`.
- The session starts the compositor; the compositor starts the shell/services
  through systemd user units.
- Logout returns to the display manager; no live compositor handoff.
- The session must be selectable without editing config files.
- `make native` / `native-return` is a dev harness, not the product installer.

## 13. Packaging and dependencies

### 13.1 Packages

| Distro | Format | Build |
|---|---|---|
| Fedora | RPM (COPR) | `packaging/fedora/` |
| Ubuntu/Debian | DEB | `packaging/debian/` |
| Arch | PKGBUILD (AUR/official) | `packaging/arch/` |

### 13.2 Dependency policy

The canonical `Recommends`/`Suggests` package markers are
[`packaging/optional-services.json`](../../packaging/optional-services.json),
shared with Debian and mirrored in `NOTICE`. The runtime contract encodes the
same sets in `services/distro-adapters/src/dependency.rs` and references the
marker table by id instead of duplicating package names or licenses.

| Set | Members | Absence behaviour |
|---|---|---|
| **Required** | Wayland/DRM/Mesa, libinput + udev, systemd/logind, NetworkManager, BlueZ, PipeWire/WirePlumber, UPower, UDisks2, polkit, xdg-desktop-portal + backend, Qt 6, Xwayland, Rust toolchain at build time | A missing session service shows a shell banner and disables the affected feature; the rest of the desktop works. Xwayland absence disables X11 apps only. The Rust toolchain is build-time only. |
| **Recommended** | CUPS, SANE, colord, fprintd, PackageKit, accountsservice, GNOME Online Accounts, ibus/fcitx5, Orca, speech-dispatcher, Flatpak | Feature-level absence: one line, no fake toggle. PackageKit falls back to the native backend; GOA/Software Center panes hide; fingerprint/IME rows hide; printing/scanning/color/a11y disable only their half. |
| **Optional** | Tracker/Baloo/Recoll, restic/Pika/Timeshift, LocalSend, snapd, firewalld/ufw/nftables/iptables, espeak-ng, GNOME Software/Discover | Built-in fallback (search), feature disabled (backup, Flatpak/Snap, firewall), or pane hidden (Nearby Transfer, Software Center). No first-party feature depends on an optional service. |

Rules:

- Do not bundle GPL services into the MIT codebase. Use separate processes or
  dynamic linking with the license notices.
- Record every optional dependency's license in `NOTICE` and the packaging
  metadata.
- The desktop must boot and be usable with the required set only; every
  optional feature has an absence path.
- A missing optional service disables only the affected feature; it never
  blocks session startup and never produces a repeated error dialog.

### 13.3 CI matrix and clean-VM smoke plan

| Job | Distro | What it proves |
|---|---|---|
| Build | Fedora 44 | Rust + Qt + CMake; `make gates` |
| Build | Ubuntu 24.04 | Rust + Qt + CMake; `make gates`; the probe records the Ubuntu host |
| Build | Debian 13 | Rust + Qt + CMake; `make gates` |
| Build | Arch | Rust + Qt + CMake; `make gates` |
| Install | Fedora 44 VM | package install, session entry, first login |
| Install | Ubuntu 24.04 VM | package install, session entry, first login |
| Install | Debian 13 VM | package install, session entry, first login |
| Install | Arch VM | package install, session entry, first login |
| Feature smoke | each | shell, Dock, menu bar, Settings, Files, network, audio |
| Absence | each | stop optional services; panes degrade |
| Adapter contract | all four | `cargo test -p dragonfruit-distro-adapters`; probe output per host |

The clean-VM smoke plan is a seven-step pass per distro: fresh image, install
the required set, select the session entry and log in, run the feature smoke,
run `dragonfruit-distro-probe`, stop each optional service and verify the
documented absence, then commit the transcript and package versions. The full
checklist and the adapter contract are in
[distro-adapter-contract.md §9](distro-adapter-contract.md#9-ci-matrix-and-clean-vm-smoke-plan).
A package is not done when it builds; it is done when a clean VM reaches a
usable desktop.

## 14. Distro differences table

| Area | Fedora | Ubuntu/Debian | Arch | Adapter rule |
|---|---|---|---|---|
| Admin group | `wheel` | `sudo`/`admin` | `wheel` | detect both |
| Package manager | `dnf5` | `apt` | `pacman` | native or PackageKit |
| Firewall | firewalld | ufw | nftables/iptables | common firewall API |
| Time sync | chrony | systemd-timesyncd | systemd-timesyncd | `timedatectl` first |
| Locale | `localectl` | `locale`/`localectl` | `localectl` | `localectl` first |
| Autologin | GDM/SDDM | GDM/SDDM/LightDM | SDDM/greetd | DM adapter |
| SELinux/AppArmor | SELinux | AppArmor | none by default | report status, do not configure |
| Snap | optional | default on Ubuntu | optional | detect `snapd` |
| Flatpak | optional | optional | optional | detect `flatpak` |
| NVIDIA | RPM Fusion | PPA | repo | driver matrix, not Settings |
| Bootloader | GRUB/BLS | GRUB | GRUB/systemd-boot | read-only Startup Disk |

## 15. Failure and absence rules

- A missing required service is a **session-level** error: the shell shows a
  banner, the affected pane/feature is disabled, and the rest of the desktop
  works.
- A missing optional service is a **feature-level** absence: a one-line note,
  no fake toggle, no repeated error dialogs.
- A missing package-manager backend is a **pane-level** absence: Software
  Update says "No update backend detected" and offers documentation.
- A missing portal is a **capability-level** absence: the file chooser,
  screenshot, or screencast feature is disabled with a reason.
- Never silently fall back to an unsafe or unsupported path.
- Every absence path has a test (headless mock or VM service stop).

## 16. Adapter contract (T221)

The contract lives in the dependency-free `services/distro-adapters` crate
(`dragonfruit-distro-adapters`) and is documented in
[distro-adapter-contract.md](distro-adapter-contract.md). Its shape:

- **One trait per capability** — `UpdateProvider`, `FirewallProvider`,
  `UserGroupProvider`, `TimeProvider`, `LocaleProvider`, `AutologinProvider`,
  `DefaultAppsProvider`, `AppDiscoveryProvider`. Every trait extends
  `CapabilityAdapter`, so a Settings pane can ask `available()` / `status()`
  without knowing the distribution.
- **Three statuses** — `Available`, `Absent`, `Unsupported`. Absence is a
  normal state, not an error dialog; `Unsupported` is the T221 stub or an
  unknown distribution and carries the eventual command/API.
- **One typed error** — `DistroError::NotSupported` / `Absent` / `Failed`. The
  `NotSupported` display starts with `not supported:`.
- **Detection from `/etc/os-release`** — `ID` first, then `ID_LIKE`, with
  Ubuntu checked before Debian. An unrecognized host is `Unknown`, never a
  guessed backend.
- **Four stubs** — Fedora, Ubuntu, Debian, Arch. Every method returns
  `NotSupported` today and every status is `Unsupported`, but each carries the
  correct command/API from the matrix. T254–T259 implement the concrete
  backends behind the same traits.
- **A mock backend** — `MockBackend` drives all three states, every write, and
  the absence paths with no daemon, package manager, or distro tool on the CI
  host.
- **A live probe** — `make distro-contract-check` runs the contract tests and
  prints the detected distro, host capability probe, and selected backend
  status. The probe never executes a distro command.

## 17. Implementation tasks

| Task | Scope |
|---|---|
| T221 | Contract, trait stubs, matrix, dependency sets, mock, probe (landed) |
| T254 | App discovery and install locations |
| T255 | Startup apps and login items |
| T256 | Users/groups, admin auth, polkit |
| T257 | Software updates and package management |
| T258 | Default apps, MIME, portals |
| T259 | Cross-distro validation matrix and packaging |

## 18. References

- [Cross-distro adapter contract](distro-adapter-contract.md)
- [Existing system integration design](07-system-integration.md)
- [Existing packaging design](12-packaging.md)
- [Existing settings design](08-settings.md)
- [Total experience design](22-total-experience.md)
- [Settings business-logic map](settings-business-logic-map.md)
- [Naming/licensing](interface-naming-and-licensing.md)
- [ADR 0221](adr/0221-cross-distro-host-service-contract.md)
- [ADR 0230](adr/0230-distro-adapter-contract-and-stubs.md)
