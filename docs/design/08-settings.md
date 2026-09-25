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

## The absent-provider matrix (T-09.6b)

Every pane must degrade cleanly when the service behind it is missing, and no
pane is advertised before its content works. The **no-half-panes rule** is
enforced structurally: `apps/settings/SettingsPanes.qml` carries one ordered
catalog whose entries have a `shipped` flag, the sidebar and local search list
only `SettingsPanes.shippedPanes`, and `SettingsShell.paneComponent(id)`
returns the pane body only for shipped ids. A catalog row with no body can
therefore never appear — the two halves (row and body) land together.

Wave 1's four panes talk to settingsd (all of them) and to the
xdg-desktop-portal FileChooser (Wallpaper's "Add Photo…" only); the shell is the
forwarder for the compositor-backed Wallpaper and Displays keys. The app never
touches D-Bus or the compositor directly, so its observable absent-provider
states are those two:

| Absent provider | Appearance | Wallpaper | Desktop & Dock | Displays | Session |
|---|---|---|---|---|---|
| xdg-desktop-portal FileChooser | unaffected | "Add Photo…" **disabled**, row says "No file chooser is available."; tiles/toggle/fit still live | unaffected | unaffected | unaffected |
| settingsd (`org.dragonfruit.Settings1`) | live on schema defaults; writes in memory | same; preview = default solid color | same; no shell applier present, so no visible Dock change but no error | same; shell stores the policy and applies it when an output is announced | unaffected; `Settings.available == false` |
| both | live on defaults | all settingsd controls live, portal row disabled | live on defaults | live on defaults | unaffected |

There is deliberately no error/empty state for a missing settingsd: the daemon
is our own, the schema defaults are the documented contract
([ADR 0030](adr/0030-settingsd-schema-and-dbus-surface.md)), and a pane that
showed a "settings unavailable" banner would be a half-pane. When the daemon
appears, `GetAll` makes its persisted state authoritative; a write made while it
was down is applied in memory and mirrored when it returns.

The headless half of the matrix is the CI gate:

- **`apps/settings/tests/tst_settings_absence.qml`** runs under a private
  `dbus-run-session` with **no** settingsd and **no** portal and no
  `DF_SETTINGS_FIXTURE` (so the `Settings` singleton is the live
  `DbusSettingsClient`). It asserts both providers are absent, that every
  shipped pane has a body and every unshipped id does not, and that all four
  panes stay live and write in memory. The Wallpaper case asserts the portal
  row is the only disabled control and carries the explanatory description.
- **`apps/settings/tests/tst_settings_shell.qml`** asserts the catalog subset
  (four shipped panes) and the sidebar/search list.

The live capture is
`docs/captures/t09-settings-wave-1.{png,light,dark,reduced.png,mp4}` (plus the
four per-pane stills), produced by `scripts/capture-settings-wave-1.sh`
(`make settings-wave-1-capture`): a scratch settingsd on the session bus, the
nested demo with the Settings window zoomed, and `appearance.colorScheme` /
`accessibility.reduceMotion` flipped live for the dark/light and
reduced-motion stills.

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
