# Settings

## Summary

Settings is one of the two flagship first-party applications (with Files, see
[09-files.md](09-files.md)). It is **not** a set of wrappers that launch
existing GNOME/KDE programs. It is a single application with a stable internal
settings API, backed by a settings daemon.

## Architecture

```text
              Settings UI
                  │
                  ▼
           desktop-settingsd
                  │
     ┌───────────┼────────────┐
     │           │            │
 desktop       host         distro
 settings    services       adapter
     │           │            │
     │           ├─ NM         ├─ Fedora/dnf
     │           ├─ BlueZ      └─ Debian/apt
     │           ├─ UPower
     │           ├─ UDisks
     │           └─ PipeWire
     ▼
 compositor
```

Three provider kinds meet inside `settingsd`:

1. **Desktop settings** — our own configuration model (Dock, workspaces,
   gestures, appearance, animation policy), persisted by us.
2. **Host services** — NetworkManager, BlueZ, UPower, UDisks, PipeWire
   (see [07-system-integration.md](07-system-integration.md)).
3. **Distro adapter** — distribution-specific administration (updates,
   package management), isolated behind a provider interface.

## Routing rules

- **Displays** → our compositor API, because it owns physical outputs.
- **Keyboard, mouse, trackpad, workspace, and Dock settings** → our own
  services.
- **Wi-Fi** → NetworkManager. **Bluetooth** → BlueZ. **Sound** →
  PipeWire/WirePlumber. **Power** → UPower. **Storage** → UDisks/GIO.
- **Updates** → distro provider.

## Settings panes

The pane list mirrors the macOS information architecture (see
`../reference/System_Preferences.md`), with each pane's routing made
explicit — no pane is "a wrapper around a GNOME dialog":

| Pane | Routed to |
|---|---|
| Wi-Fi, Network | NetworkManager |
| Bluetooth | BlueZ |
| Battery | UPower + power-profiles-daemon where present |
| General | `settingsd` + distro provider (about, updates, defaults) |
| Appearance | `settingsd` (design-system themes, dark/light, accent) |
| Accessibility | `settingsd` → compositor magnification + toolkit/AT-SPI settings |
| Desktop & Dock | `settingsd` → shell + compositor |
| Displays | Compositor output API (resolution, scaling, rotation, color, night light, VRR) |
| Wallpaper | `settingsd` + compositor (per-Space wallpaper) |
| Menu Bar | `settingsd` → shell |
| Search | `settingsd` → app-index (Spotlight-equivalent; roadmap "later") |
| Notifications, Focus | Notification service |
| Lock Screen | `settingsd` + compositor lock/idle policy |
| Privacy & Security | `settingsd` + host services (Secret Service, polkit) |
| Biometrics & Password | host services (fprintd, Secret Service) where present |
| Users & Groups | accountsservice + distro provider |
| Internet Accounts | host services (GOA) — deferred, not core |
| Keyboard, Mouse, Trackpad | Compositor input API + xkbcommon |
| Sound | PipeWire / WirePlumber |
| Printers & Scanners | CUPS + SANE |
| Screen Time, AI | macOS-specific; **not planned** |

## Persistence and change notification

Desktop settings live under `$XDG_CONFIG_HOME/dragonfruit/`, written by
`settingsd` only, in a schema-documented format with named keys. Clients
observe changes over `org.dragonfruit.Settings1` signals rather than polling,
so the compositor, shell, and apps react to a single source of truth. Schema
changes are additive within a release; migrations run at `settingsd` startup.

## Menu model

The app publishes its native menu model for the global menu (T-09.6a,
[06-global-menu.md](06-global-menu.md)). `apps/settings/SettingsMenu.qml` is the
single declarative source — the fixed application menu plus the app's `File`,
`Edit`, `View`, `Window`, and `Help` menus, in the design system's normalized,
JSON-serializable entry shape; each row carries an `action` string. The same
singleton feeds an in-window local menu presentation, so global and local menus
cannot diverge. The shell's `MenuBar` consumes the published model unchanged
(ADR [0041](adr/0041-native-menu-model-publication-shape.md)); the
menu-broker (T-14.2a) owns the cross-process transport and dispatch.

## Distro provider interface

Settings should say "check for updates," not "execute a `dnf5` command." The
distro adapter is isolated behind a trait so the product architecture stays
distro-agnostic:

```rust
trait SystemProvider {
    async fn distribution_info(&self) -> DistributionInfo;
    async fn check_updates(&self) -> Result<Vec<Update>>;
    async fn install_updates(&self) -> Result<()>;
    async fn reboot(&self) -> Result<()>;
}
```

with a `FedoraProvider` now and a `DebianProvider` later. Everything above the
interface stays identical. This abstraction is what makes Debian/Ubuntu
portability mostly a packaging exercise (see [12-packaging.md](12-packaging.md)).

## UI

The Settings UI looks precisely how we want — macOS-like information
architecture — built entirely in our design system (see
[10-design-system.md](10-design-system.md)), with no dependency on GNOME
Control Center.

### Reference and style

The pane-by-pane reference is
[System_Preferences.md](../reference/System_Preferences.md): Dragonfruit
mirrors macOS System Settings **nearly verbatim** — style, information
architecture, section order, row labels, control types, and wording —
dropping only Apple/Mac-only concepts (Apple Account/iCloud, AppleCare,
Continuity/AirDrop, Siri/Apple Intelligence, Screen Time, Touch ID/Apple
Watch, App Store, Find My, FileVault). Generic terms are kept; Linux-like
substitutions are chosen deliberately per pane. The reference screenshots
themselves are local-only and never ship
([14-risks.md](14-risks.md)).

Sequencing note from the roadmap: we deliberately do **not** spend months
cloning every System Settings page before the desktop itself feels good (see
[ROADMAP.md](../ROADMAP.md)).
