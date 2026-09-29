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
| Wi-Fi, Network | NetworkManager (Wi-Fi + the VPN connections, [adr/0146](adr/0146-network-advanced-vpn-adapter.md), [adr/0147](adr/0147-network-advanced-vpn-pane-and-tile.md)) |
| Bluetooth | BlueZ |
| Battery | UPower + power-profiles-daemon where present |
| General | `settingsd` + distro provider (about, updates, defaults) |
| Appearance | `settingsd` (design-system themes, dark/light, accent) |
| Accessibility | live AT-SPI read + `settingsd` (`accessibility.reduceMotion`) ([adr/0144](adr/0144-accessibility-adapter.md), [adr/0145](adr/0145-accessibility-pane-and-tile.md)) |
| Desktop & Dock | `settingsd` → shell + compositor |
| Displays | Compositor output API (resolution, scaling, rotation, color, night light, VRR) |
| Wallpaper | `settingsd` + compositor (per-Space wallpaper); `wallpaperd` for the fetched Featured Pictures |
| Menu Bar | `settingsd` → shell |
| Search | `settingsd` → app-index (Spotlight-equivalent; roadmap "later") |
| Notifications, Focus | Notification service |
| Lock Screen | `settingsd` + compositor lock/idle policy |
| Privacy & Security | Portals (`xdg-desktop-portal` PermissionStore); no `settingsd` key — the store is the state (Secret Service/polkit later) |
| Biometrics & Password | host services (fprintd, Secret Service) where present |
| Users & Groups | accountsservice + distro provider |
| Internet Accounts | host services (GOA) — deferred, not core |
| Keyboard, Mouse, Trackpad | Compositor input API + xkbcommon |
| Sound | PipeWire / WirePlumber |
| Printers & Scanners | CUPS + SANE |
| Screen Time, AI | macOS-specific; **not planned** |

## Category icons (T-19.1b)

The Settings sidebar rows and the detail-pane header card draw their icons as
`SettingsCategoryIcon` gradient tiles — the macOS System Settings language, and
the **only** surface that gets the container (the menu bar, Dock, and
Applications drawer use bare Phosphor glyphs). `apps/settings/SettingsPanes.qml`
holds the single category → (glyph, gradient) table (`categoryStyles`, keyed by
each pane's existing `icon` identity); `PaneHeader.qml` and `SettingsShell.qml`
resolve it through `SettingsPanes.categoryStyle(pane)` and pass the values to
the component. The literal category hues live in that one table and are never
baked into an SVG; the container geometry is tokenized
(`component.settingsCategory.*`, `component.sidebar.categoryIconSize`). A pane
whose icon has no Phosphor mapping (Trackpad) keeps the original `Icon.qml`
glyph — the documented fallback. See ADR
[0164](adr/0164-settings-category-tile.md).

## Wallpaper sources (T-18)

The Wallpaper pane presents three source rows: **Featured** (Wikimedia
Commons Featured Pictures, fetched by `services/wallpaperd`), **Built-in**
(the shipped original default `assets/graphics/wallpapers/Default.jpg` plus the
original gradients), and **Custom** (the portal chooser). The out-of-box
background is the shipped original default, so first run and a cold cache never
depend on the network. The provider downloads Featured at first run and
re-checks roughly weekly, caching the images and their attribution metadata
under `$XDG_CACHE_HOME/dragonfruit/`; it warms lazily on launch and eagerly when
the pane is opened. The effective source is `wallpaper.source` (a user choice)
when non-empty, otherwise the shipped `wallpaper.builtinDefault`, otherwise the
fetched `wallpaper.providerSource`. While the first catalogue is still
downloading, Featured renders as reduced-motion-aware skeleton tiles. The
provider, key ownership, effective-source precedence, cache policy, offline
behavior, and licensing are fixed by ADRs
[0055](adr/0055-online-wallpaper-content-provider.md) /
[0094](adr/0094-bundled-default-wallpaper-and-lazy-cache.md); the pane UI,
states, and attribution are T-18.2.

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

## The absent-daemon masking matrix (T-15.16)

T-15 landed the Wave-2/3 panes and their Control Center tiles, taking the
catalog to **22 shipped panes** (`SettingsPanes.shippedPanes`). Each pane routes
to exactly one subsystem; masking that subsystem is a supported session state,
not an incident. The matrix below is the contract every shipped pane satisfies
and the wave-2/3 companion to the T-09.6b table above.

Two absence families appear, and a pane can mix them:

- **settingsd-backed controls** (Appearance, Desktop & Dock, Mission Control,
  Displays, Lock Screen, Menu Bar, and the durable halves of Wallpaper, Sound,
  Keyboard/Mouse/Trackpad, Notifications/Focus, Accessibility) stay **live on
  the schema defaults** with no settingsd. A write lands in memory and is
  mirrored when the daemon returns ([ADR 0030](adr/0030-settingsd-schema-and-dbus-surface.md));
  there is no "settings unavailable" banner.
- **adapter-read halves** read a host daemon through the `system-status` bridge
  host ([ADR 0029](adr/0029-system-status-bridge-host.md)). With no host, an
  unowned bus name, or a daemon answering `present: false`, the pane replaces
  that half with a one-line note and attempts no write, and the matching tile
  hides by the same rule. A present-but-unreadable daemon is `Error` — visible
  and inert, never leaked into a neighbour.

| Shipped pane | Routed to | Pane with the provider absent | Control Center tile |
|---|---|---|---|
| Appearance | settingsd | live on schema defaults | — (none) |
| Desktop & Dock | settingsd → shell/compositor | live on defaults; no shell applier, but no error | — (none) |
| Mission Control | settingsd + compositor | settingsd keys live on defaults; the note says the daemon is gone | **hides** (no compositor bridge) |
| Displays | compositor output API + settingsd | live on defaults; shell applies the policy when an output appears | — (none) |
| Wallpaper | settingsd + `wallpaperd` + FileChooser portal | built-in/toggle/fit live; Featured empty (never an error); the "Add Photo…" portal row **disabled** with its explanation | — (none) |
| Bluetooth | BlueZ (bridge host) | note; toggle disabled; no write | **hides** |
| Network (VPN) | NetworkManager (bridge host) | note; connection list empty; `Connect`/`Deactivate` are no-ops | **hides** |
| Battery | UPower + power-profiles-daemon (bridge host) | note; no profile write | **hides** |
| Storage | UDisks2/GIO (bridge host) | note; no mount/eject | **hides** |
| General | settingsd + update adapter (bridge host) | About reads locally; the update half shows its note | **hides** |
| Sound | PipeWire/WirePlumber (bridge host) + settingsd | routing half note; Sound Effects/Balance stay live on defaults | volume tile `enabled: audioAvailable` |
| Keyboard | libinput (bridge host) + settingsd | inventory note; every preference row live on defaults | **hides** |
| Mouse | libinput (bridge host) + settingsd | inventory note; pointer settings live on defaults | — (shared Keyboard tile) |
| Trackpad | libinput (bridge host) + settingsd | inventory note; tap-to-click etc. live on defaults | — (shared Keyboard tile) |
| Notifications | notification service (bridge host) + settingsd | per-app inventory note; presentation prefs stay live | — (none) |
| Focus | notification service (bridge host) + settingsd | inventory note; policy rows stay live | **hides** |
| Lock Screen | settingsd + compositor lock/idle | controls live on defaults; the note names the daemon | **hides** |
| Menu Bar | settingsd → shell | controls live on defaults; the note names the daemon | **hides** |
| Users & Groups | accountsservice/distro (bridge host) | note; read-only tile | **hides** |
| Printers & Scanners | CUPS/SANE (bridge host) | note; no add/queue write | **hides** |
| Privacy & Security | portal `PermissionStore` (bridge host) | note; no permission write | **hides** |
| Accessibility | AT-SPI (bridge host) + settingsd | status note; `accessibility.reduceMotion` stays live | **hides** |

The headless half is the CI gate:

- **`apps/settings/tests/tst_settings_absence.qml`** runs under a private
  `dbus-run-session` with no settingsd, no portal, and no bridge host. It
  asserts both providers are absent, that all 22 shipped panes have a body and
  no unshipped id does, that **every shipped pane mounts and stays interactive**
  (the routing sweep, `test_every_shipped_pane_routes_and_stays_live_with_providers_absent`),
  and carries one case per pane (including the shared Mouse section) that
  asserts the absence note and that the settingsd-backed controls still write.
- The per-pane suites (`tst_settings_<pane>.qml`) assert the adapter's
  `absent` value through the mock or fixture seam; the Rust adapters assert the
  three-state projection in `services/*/tests/`.
- **`scripts/t15-absence-matrix.sh`** (`make t15-absence-matrix`) reproduces the
  headless half into `docs/captures/t15-absence-matrix.txt`; the reviewed matrix
  with the live halves is `docs/captures/t15-absence-matrix.md`.

The real-daemon half — stopping BlueZ/UDisks/CUPS/etc. on a VM and watching the
pane and tile — is the design's "real masking in a VM when available"
([14-risks.md](14-risks.md)); no VM is in CI, so it is recorded in the reviewed
matrix rather than automated. "No bridge host" is the same
hidden-everything state on a live session, because the host is a separate
process.

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
