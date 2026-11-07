# Cross-distro adapter contract (T221)

| | |
|---|---|
| **Status** | contract landed; concrete backends T254–T259 |
| **Date** | 2026-11-07 |
| **Crate** | `services/distro-adapters` (`dragonfruit-distro-adapters`) |
| **Related** | [distro-compatibility.md](distro-compatibility.md), [07-system-integration.md](07-system-integration.md), [12-packaging.md](12-packaging.md), [ADR 0221](adr/0221-cross-distro-host-service-contract.md), [ADR 0230](adr/0230-distro-adapter-contract-and-stubs.md) |

## 1. Scope

Dragonfruit targets Fedora, Ubuntu, Debian, and Arch. They share the runtime
host stack but differ in the administration tools behind Settings: the package
manager, firewall, users/groups tooling, time sync, locale tooling, autologin
file, default-app registration, and extra app-discovery paths.

This document is the contract between a Settings pane and those tools. The
pane never shells out to a distribution command and never names a
distribution. It holds one adapter per capability and asks
`available()` / `status()`; the adapter owns the distro difference.

T221 lands the contract, the per-distro command/API matrix, the four stubs, the
mock, and the contract tests. T254–T259 implement the concrete backends behind
the same traits.

## 2. The crate and the status contract

`services/distro-adapters` is **dependency-free** (std only), like
`services/system-adapters`. The Settings host and every concrete backend
depend on it; it never drags a package manager, D-Bus stack, or distro tool
into the contract.

One adapter answers with a `CapabilityStatus`:

| State | Meaning | Pane behaviour |
|---|---|---|
| `Available` | The host service/tool is present and the backend can serve it. | Render the live controls. |
| `Absent` | The host service/tool is not installed. A normal state. | One-line note, no fake toggle; only the affected feature disables. |
| `Unsupported` | The selected backend is not implemented yet (the T221 stubs), or the distribution is outside the matrix. | Disable the pane with the reason; never an error dialog. |

`CapabilityAdapter` is the common trait:

```rust
pub trait CapabilityAdapter {
    fn capability(&self) -> Capability;
    fn backend(&self) -> BackendId;
    fn available(&self) -> bool;      // status().is_available()
    fn status(&self) -> CapabilityStatus;
}
```

`CapabilityStatus::view()` is the flat wire projection a bridge host or QML
pane can serialize: `{capability, state, available, backend, command, reason}`.
The `available` boolean is redundant with `state` on purpose so a pane that
only needs the boolean does not match on the enum.

A trait method returns `Result<T, DistroError>`:

- `NotSupported { capability, backend, command }` — the T221 stub and the
  unknown-distribution answer. Its `Display` starts with `not supported:` and
  names the command/API the eventual backend will use.
- `Absent { capability, reason }` — the host tool is not installed.
- `Failed { capability, message }` — the tool is present but the request
  failed (including a polkit refusal).

## 3. The eight traits

| Trait | Read | Explicit writes | Settings pane |
|---|---|---|---|
| `UpdateProvider` | `distribution_info`, `check_updates`, `reboot_required` | `install_updates` | Software Update |
| `FirewallProvider` | `firewall_status` | `set_enabled`, `open_port`, `close_port` | Network (Firewall) |
| `UserGroupProvider` | `admin_groups`, `list_groups` | `create_user`, `delete_user`, `set_password`, `create_group`, `delete_group`, `set_group_members` | Users & Groups |
| `TimeProvider` | `time_status` | `set_timezone`, `set_ntp` | Date & Time |
| `LocaleProvider` | `locale_status`, `available_locales` | `set_locale` | Language & Region |
| `AutologinProvider` | `autologin_status` | `set_autologin` | Users & Groups |
| `DefaultAppsProvider` | `roles`, `default_app`, `mime_handler` | `set_default_app` | Default Apps |
| `AppDiscoveryProvider` | `discovery_paths`, `install_sources` | — | Applications |

Every write is one explicit request. A successful write invents no snapshot;
the host re-reads the provider and the snapshot stays the single source of
truth. `Secret` wraps a password and has a redacted `Debug`; a backend calls
`Secret::expose()` only immediately before handing the value to PAM/passwd.

`UserGroupProvider` is the **write** path plus the group list. The user read
stays with AccountsService (`services/account-adapter`), exactly as
[07-system-integration.md](07-system-integration.md) splits them.

## 4. Detection

The adapter is selected at startup from `/etc/os-release`, not from a
hardcoded list in the UI:

- `ID=fedora` (or `rhel`, `centos`, `rocky`, `almalinux`) → Fedora;
- `ID=ubuntu` → Ubuntu;
- `ID=debian` → Debian;
- `ID=arch`, `archlinux`, `manjaro`, `endeavouros` → Arch;
- otherwise `ID_LIKE` tokens inherit the upstream family, with Ubuntu checked
  before Debian so `ID_LIKE="ubuntu debian"` stays Ubuntu;
- anything else is `Unknown`, best-effort, never a guessed backend.

The tier is Fedora/Ubuntu 24.04/Debian 13 = Tier 1, Ubuntu 26.04+ and Arch =
Tier 2, derivatives = Tier 3. Admin groups are detected per family:
`wheel` on Fedora/Arch, `sudo` and `admin` on Ubuntu/Debian.

`SystemProbe` checks executable bits on `PATH` and reads
`root/etc/os-release`; it never executes a distro command. A fixture root makes
the probe testable without the host.

## 5. The distro matrix

The Rust table is `MATRIX` in `services/distro-adapters/src/matrix.rs`; every
row carries the backend, command, exact invocation/API, detection probes,
package names, and an implementation note. The T221 stub exposes the same row
in its `Unsupported` status and in the `NotSupported` error.

### Updates

| Distro | Backend | Command/API | Probe | Packages |
|---|---|---|---|---|
| Fedora | native dnf5, PackageKit fallback | `dnf5 check-update --json`; `dnf5 upgrade --refresh`; offline via `dnf5 offline` | `dnf5`, `dnf`, `pkcon` | `dnf5`, `PackageKit` |
| Ubuntu | native apt, PackageKit fallback | `apt-get update`; `apt-get --simulate upgrade`; `apt-get install` | `apt-get`, `pkcon` | `apt`, `PackageKit` |
| Debian | native apt, PackageKit fallback | same as Ubuntu; `unattended-upgrades` owns the automatic policy | `apt-get`, `pkcon` | `apt`, `PackageKit` |
| Arch | native pacman, PackageKit fallback | `pacman -Sy`; `pacman -Qu`; `pacman -S`; no offline-update equivalent | `pacman`, `pkcon` | `pacman`, `PackageKit` |

### Firewall

| Distro | Backend | Command/API | Probe | Packages |
|---|---|---|---|---|
| Fedora | firewalld | `org.fedoraproject.FirewallD1` over D-Bus; `firewall-cmd --state` | `firewall-cmd` | `firewalld` |
| Ubuntu | ufw | `ufw status`; `ufw enable`; `ufw allow <port>/<proto>` | `ufw` | `ufw` |
| Debian | nftables (ufw when present) | `nft list ruleset`; `nft add rule ...` | `nft`, `ufw` | `nftables`, `ufw` |
| Arch | nftables/iptables | `nft list ruleset`; `nft add rule ...`; iptables fallback | `nft`, `iptables` | `nftables`, `iptables` |

No supported manager is `No firewall detected`; the pane offers no fake
toggle.

### Users and groups

| Distro | Command/API | Admin group |
|---|---|---|
| Fedora | `useradd`/`usermod`/`userdel`; `groupadd`/`groupmod`/`groupdel`; `passwd`/`chpasswd`; `gpasswd` | `wheel` |
| Ubuntu | same, with `adduser` where installed | `sudo`, `admin` |
| Debian | same, with `adduser` where installed | `sudo`, `admin` |
| Arch | same | `wheel` |

AccountsService owns the user read; the distro provider owns group and
password writes. Never edit `/etc/passwd`, `/etc/shadow`, or `/etc/group`
directly.

### Time and locale

| Distro | Time | Locale |
|---|---|---|
| Fedora | `timedatectl`; `chronyc tracking` | `localectl` |
| Ubuntu | `timedatectl`; `timedatectl timesync-status` | `localectl` first; `locale-gen`/`update-locale` fallback |
| Debian | `timedatectl`; `timedatectl timesync-status` | `localectl` first; `locale-gen`/`update-locale` fallback |
| Arch | `timedatectl`; `timedatectl timesync-status` | `localectl` first; `locale-gen` fallback |

`timedatectl` and `localectl` are the first path on every distro; the
distro-specific tools are only the fallback.

### Autologin

| Distro | Detected display manager and file |
|---|---|
| Fedora | GDM `/etc/gdm/custom.conf` (`AutomaticLoginEnable`, `AutomaticLogin`); SDDM `/etc/sddm.conf` (`[Autologin]`) |
| Ubuntu | GDM `/etc/gdm3/custom.conf`; LightDM `/etc/lightdm/lightdm.conf`; SDDM `/etc/sddm.conf` |
| Debian | LightDM `/etc/lightdm/lightdm.conf`; GDM `/etc/gdm3/custom.conf`; SDDM `/etc/sddm.conf` |
| Arch | SDDM `/etc/sddm.conf`; greetd `/etc/greetd/config.toml` (`initial_session`); GDM |

The adapter detects the running display manager and writes its own file; there
is no shared config format.

### Default apps and app discovery

All four distros use `xdg-mime`, `xdg-settings`, and `mimeapps.list` under
`$XDG_CONFIG_HOME`; the Settings portal is preferred when available.

App discovery is owned by `app-index`, not this adapter. The adapter declares
the XDG paths plus Flatpak/Snap/AppImage/Nix sources and the install sources
(distribution repository, Flatpak, Snap, AppImage, local package). It never
scans desktop files itself.

## 6. Dependency sets and absence behaviour

The package markers are canonical in
[`packaging/optional-services.json`](../../packaging/optional-services.json);
the Rust table in `services/distro-adapters/src/dependency.rs` references that
table by id instead of duplicating package names or licenses. The table exists
so the adapter layer can answer "what happens when this is absent?" at
runtime.

| Set | Members | Absence behaviour |
|---|---|---|
| **Required** | Wayland/DRM/Mesa, libinput + udev, systemd/logind, NetworkManager, BlueZ, PipeWire/WirePlumber, UPower, UDisks2, polkit, xdg-desktop-portal + backend, Qt 6, Xwayland, Rust toolchain (build) | A missing session service shows a shell banner and disables the affected feature; the rest of the desktop works. Xwayland is a compatibility layer: its absence disables X11 apps only. The Rust toolchain is build-time only. |
| **Recommended** | CUPS, SANE, colord, fprintd, PackageKit, accountsservice, GNOME Online Accounts, ibus, fcitx5, Orca, speech-dispatcher, Flatpak | Feature-level absence: one line, no fake toggle. PackageKit falls back to the native backend; GOA and Software Center panes hide; fingerprint/IME rows hide; printing/scanning/color/a11y disable only their half. |
| **Optional** | Tracker, Baloo, Recoll, restic, Pika Backup, Timeshift, LocalSend, snapd, firewalld, ufw, nftables, iptables, espeak-ng, GNOME Software, KDE Discover | Built-in fallback (search), feature disabled (backup, Flatpak/Snap, firewall), or pane hidden (Nearby Transfer, Software Center). No first-party feature depends on an optional service. |

The required desktop boots and is usable with the required set only. A missing
optional service never blocks the session and never produces a repeated error
dialog. Every absence path has a test: the mock backend in CI and a service
stop on the clean VM.

## 7. Mock and contract tests

`MockBackend` is the CI path. It holds every capability's status and data
behind one shared lock, so a test can drive several adapters from the same
simulated host. `available()`, `absent()`, and `unsupported()` are the three
states; writes mutate the simulated host and are counted so a test can prove
one explicit call, never a loop.

`services/distro-adapters/tests/contract.rs` covers:

- every matrix row for every supported family (command, API, probe, packages);
- every stub method for every family returns `NotSupported` with the right
  capability/backend/command;
- `/etc/os-release` detection for the four distros, derivatives, Ubuntu 26.04,
  and unknown hosts;
- the fixture-root probe (no command execution);
- the mock's three states and every write round-trip;
- the dependency sets and their references to the canonical optional-service
  table;
- the live probe's report shape.

## 8. The live probe

```sh
make distro-contract-check
# or
cargo run -q -p dragonfruit-distro-adapters --bin dragonfruit-distro-probe
```

The probe prints the detected distribution, the selected stub backend, the
host capability probe, and each adapter's status. It never executes a distro
command. Example from the T221 Fedora 44 host:

```text
dragonfruit distro probe (T221)
distro: Fedora Linux 44 (fedora, Tier 1)
backend: fedora (stub; T254-T259 implement the concrete backend)
host capabilities: 7/8
capability       host         adapter        command/API
updates          available    unsupported    dnf5 check-update --json; dnf5 upgrade --refresh
...
autologin        absent       unsupported    GDM /etc/gdm/custom.conf ...
                                             none of gdm, sddm is installed
```

`host` is what the host probe found; `adapter` is the selected backend's
implementation status. The T221 stubs are always `unsupported`; T254–T259
flip each row to `available` when the concrete backend lands.

## 9. CI matrix and clean-VM smoke plan

### CI matrix

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

The build jobs are the existing CI shape. The install/smoke/absence jobs are
T259's deliverable; T221 documents them so the matrix has one owner and the
contract tests are already in `make gates`.

### Clean-VM smoke plan

For each Tier 1 distro (Fedora 44, Ubuntu 24.04, Debian 13) and Tier 2 Arch:

1. **Fresh image.** Use the distro's current cloud/VM image; record the image
   name and package versions. No developer packages, no prior Dragonfruit
   install.
2. **Build or install.** Install the package set from the distro's packaging
   (RPM/DEB/PKGBUILD) or the CI artifact. The required set only; optional
   services are not installed on the first pass.
3. **Session entry.** Select `dragonfruit.desktop` in the display manager,
   log in, and verify the compositor starts the shell, Dock, menu bar, and
   Settings. No live compositor handoff.
4. **Feature smoke.** Network list, audio output, battery/power row, storage
   mount, Files browse, Settings panes, app launch, lock/unlock, logout.
5. **Adapter probe.** Run `dragonfruit-distro-probe` and record the detected
   distro, host capabilities, and selected backend. The stub rows are expected
   `unsupported` until T254–T259 land; the host rows prove the matrix is
   correct for that distro.
6. **Absence pass.** Stop each optional service (CUPS, SANE, colord, fprintd,
   PackageKit, GOA, ibus/fcitx5, Orca, Flatpak, Snap) and verify the affected
   pane shows the documented one-line absence while the rest of the desktop
   keeps working. Stop a required service and verify the shell banner and the
   feature disable, not a session abort.
7. **Record.** Commit the probe transcript, the package versions, and the
   pass/fail per row. A package is not done when it builds; it is done when a
   clean VM reaches a usable desktop.

## 10. Follow-ups

- **T254** app discovery and install locations: implement
  `AppDiscoveryProvider` and the app-index source paths.
- **T255** startup apps: XDG autostart and systemd user units (not in this
  contract; startup apps are a separate Settings surface).
- **T256** users/groups, admin auth, polkit: implement `UserGroupProvider` and
  the autologin per-DM files.
- **T257** updates: implement `UpdateProvider` (native + PackageKit).
- **T258** default apps, MIME, portals: implement `DefaultAppsProvider`.
- **T259** cross-distro validation and packaging: run the CI matrix and the
  clean-VM smoke plan, then flip the stub rows to `available`.
- **Settings host wiring:** a bridge host should expose
  `CapabilityStatus::view()` per capability so the QML panes can consume it
  without linking Rust. T224/T225/T226/T229 own the panes.
