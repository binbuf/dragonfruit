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

> **v0.2.0 note.** The authoritative row-level owner map is
> [settings-business-logic-map.md](settings-business-logic-map.md). It extends
> this pane list with every macOS pane/sub-pane, the disposition
> (ship/adapt/delegate/omit), the proposed key/API, and the absence path. The
> executable work is T222–T229 in
> [v0.2.0/ROADMAP.md](../v0.2.0/ROADMAP.md). This document remains the
> architecture overview.

The pane list mirrors the macOS information architecture (see
`../reference/System_Preferences.md`), with each pane's routing made
explicit — no pane is "a wrapper around a GNOME dialog":

| Pane | Routed to |
|---|---|
| Wi-Fi, Network | NetworkManager (Wi-Fi + the VPN connections, [adr/0146](adr/0146-network-advanced-vpn-adapter.md), [adr/0147](adr/0147-network-advanced-vpn-pane-and-tile.md)) |
| Bluetooth | BlueZ |
| Battery | UPower + power-profiles-daemon where present |
| General | `settingsd` + distro provider (about, updates, defaults) |
| Appearance | `settingsd` (design-system themes, dark/light, accent, attached-menu transparency) |
| Accessibility | live AT-SPI read + `settingsd` (`accessibility.reduceMotion`, `reduceTransparency`, `increaseContrast`) ([adr/0144](adr/0144-accessibility-adapter.md), [adr/0145](adr/0145-accessibility-pane-and-tile.md), [adr/0215](adr/0215-glass-live-settings-and-appearance-policy.md)) |
| Desktop & Dock | `settingsd` → shell + compositor |
| Displays | Compositor output API — **shipped:** scale and rotation; **design-only:** mode enumeration, brightness (Control Center owns the current slider), color management/HDR, night light, VRR ([STATUS.md](STATUS.md) G-13) |
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

## The executable map (T222)

`docs/design/settings-business-logic-map.md` is the row-level source of truth;
`apps/settings/SettingsMap.qml` is its runtime projection. The map owns every
pane's id, label, icon, shipped flag, owner, deep link
(`settings://pane/<id>`), absence kind, and row metadata
(`{ id, section, label, control, owner, key, absence, state }`).
`apps/settings/SettingsPanes.qml` remains the one ordered sidebar catalog and
mirrors the map one-for-one; `apps/settings/tests/tst_settings_map.qml` fails on
any drift, on a shipped pane without a body, or on a visible row without an
owner/key/absence path. Omitted panes live only in `SettingsMap.omittedPanes`,
so they cannot appear in the sidebar, and each carries its reason.

The map has 29 top-level catalog entries and the same 22 shipped panes as
before; the new `control-center` (Quick Settings), `search` (renamed from
`spotlight`), `screen-saver`, `game-controllers`, and `online-accounts`
(renamed from `internet-accounts`) entries are unshipped until T225–T229 land
their bodies. `storage`, `mission-control`, and `menu-bar` keep their existing
ids and top-level placement. The Help menu opens the in-app **Pane & Row
Owners…** debug view (also `DF_SETTINGS_DEBUG=1`) and the **Business-Logic
Map** document; the debug view shows the selected pane's owner/absence/deep
link and every row's owner/key/absence/state. See ADR
[0231](adr/0231-executable-settings-map.md).

## Search, deep links, and row anatomy (T223)

`apps/settings/SettingsSearch.qml` is the one local Settings index. It is built
once from the executable map and the pane catalog: every shipped pane title,
every map section, and every shipped row, plus synonyms and help text (the row's
owner, key/API, absence kind, and the pane's absence note). Unshipped panes and
`planned`/`omitted` rows are not indexed, so search can never advertise a
half-pane. Search is a pure synchronous scan — no bus, daemon, or separate
search service.

Ranking is field-weighted: exact label, label prefix, word start, substring,
then synonyms, section, pane title, and help text. A pane whose own name matches
sorts first, so `display` opens Displays before an Accessibility row called
Display, and `battery` opens Battery before the menu-bar Battery checkbox.
Technical key/control terms are weighted below the user's words.

The result list (`SettingsSearchResults.qml`) replaces the sidebar pane list
while the search field has text. It shows pane, section, and row results, is a
real AT-SPI list, and supports Up/Down/Home/End/Return/Escape; Down from the
search field hands focus to the list. An empty query restores the sidebar and a
no-match query shows an explicit "No results" state.

A result carries a `settings://pane/<id>` or
`settings://pane/<id>/row/<rowId>` deep link. The shell accepts a row link only
when the map knows the row and marks it shipped, swaps the pane, then finds the
rendered `SettingsRow` by `settingsId` first and label/synonym second. It
scrolls the detail view to the row, paints an accent wash, and moves keyboard
focus into the row's control. A row that is custom pane chrome rather than a
`SettingsRow` lands on its `SettingsGroup` section, and a section-less pane
lands at its top; a result is never a dead link. Back/forward remain pane
history, not row history.

`SettingsRow` owns the one row anatomy: label, description, control, disclosure
chevron, trailing action, and help affordance. `settingsId`, `owner`, and
`settingsKey` link a rendered row back to the map, and `highlighted` is the
deep-link landing state. Long labels elide and long descriptions wrap; the
control slot stays inside the row. `SettingsGroup` carries `sectionId` for
section-level landings.

The bottom-right `?` help control opens
`Settings.helpUrl(paneId)` — the business-logic map with a `#pane-<id>` anchor
— and disables when the document is not installed, rather than opening a dead
link. The four reference header panes (General, Accessibility, Notifications,
Privacy & Security) keep their header card; settings-list panes stay headerless.
See ADR [0232](adr/0232-settings-search-and-deep-links.md). The deterministic
visual check is `make t223-settings-search-capture`
(`docs/captures/t223-settings-{search,deep-link,long-strings}.png`).

## Category icons (T-19.1b)

The Settings sidebar rows and the detail-pane header card draw their icons as
`SettingsCategoryIcon` gradient tiles — the macOS System Settings language, and
the **only** surface that gets the container (the menu bar, Dock, and
Applications drawer use bare Tabler line glyphs). `apps/settings/SettingsPanes.qml`
holds the single category → (glyph, gradient) table (`categoryStyles`, keyed by
each pane's existing `icon` identity); `PaneHeader.qml` and `SettingsShell.qml`
resolve it through `SettingsPanes.categoryStyle(pane)` and pass the values to
the component. The literal category hues live in that one table and are never
baked into an SVG; the container geometry is tokenized
(`component.settingsCategory.*`, `component.sidebar.categoryIconSize`). The
tile draws at the size of the slot it is given (a 20 px sidebar row, a 48 px
hero) rather than its own implicit size, so it cannot overflow the row into the
label or the next icon. A list that mixes a category tile with a plain fallback
glyph (Trackpad) reserves the same 20 px glyph column for both, keeping every
label in one column. The tile glyph is a Phosphor `fill` mark because the tile
needs a solid near-white symbol; a pane whose icon has no category-tile mapping
falls back to the Tabler-backed `Icon.qml` vocabulary. See ADR
[0164](adr/0164-settings-category-tile.md) and ADR
[0185](adr/0185-tabler-line-icons.md).

## Window chrome material (T206)

The Settings window is a first-party CSD surface: `SettingsWindow.qml` draws
the design-system `TitleBar` and `SettingsShell` owns the sidebar, toolbar and
detail sheet. T206 gives the sidebar and toolbar their observed near-opaque
appearance without changing the opaque content sheet:

- Light sidebar `settingsSidebar` 0.986/0.857 and toolbar
  `settingsToolbar` 0.961/0.965 (measured).
- Dark uses the Finder-derived 0.935/0.218 sidebar and 0.900/0.247 toolbar as
  an explicit **adapted** baseline; System Settings Dark has no independent
  fit.
- The window frame/content backing is transparent only behind those two
  surfaces (`AppWindow.chromeTransparent`); the titlebar keeps its opaque
  backing and the scrolling detail sheet stays `Theme.color.surface`.
- `Material.appChromeFill()` resolves the preset through the shared `Glass`
  resolver. Reduce Transparency uses the shared opaque plate; a
  non-composited/unsupported client keeps the existing opaque
  `surfaceMuted`/`chrome` fills; Increase Contrast raises the separator ink.
  Focus is a controls/text state: the titlebar/controls dim, the material does
  not change.
- `SettingsShell` mirrors `appearance.colorScheme`,
  `appearance.accent`, `accessibility.reduceMotion`,
  `accessibility.reduceTransparency` and `accessibility.increaseContrast` onto
  its own `Theme`/`Glass` singletons, the same app-local pattern ADR 0037
  established. `Settings.appChromeSupported` is the client-alpha capability
  seam (`DF_APP_CHROME_SOLID=1` forces the opaque capture path).

The preset stores scalar alpha/tint only, so the reference's small sidebar
chroma pickup is not reproduced; T206 captures record the rendered channel
spread separately. ADR [0225](adr/0225-first-party-app-chrome-presets.md)
records the decision and the unresolved chroma transport.

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
was down survives in memory for the session only and is reconciled away by that
resync (ADR [0032](adr/0032-shell-settings-client.md)).

The headless half of the matrix is the CI gate:

- **`apps/settings/tests/tst_settings_absence.qml`** runs under a private
  `dbus-run-session` with **no** settingsd and **no** portal and no
  `DF_SETTINGS_FIXTURE` (so the `Settings` singleton is the live
  `DbusSettingsClient`). It asserts both providers are absent, that every
  shipped pane has a body and every unshipped id does not, and that all four
  panes stay live and write in memory. The Wallpaper case asserts the portal
  row is the only disabled control and carries the explanatory description.
- **`apps/settings/tests/tst_settings_shell.qml`** asserts the shipped catalog
  subset and the sidebar/search list. **`tst_settings_map.qml`** (T222) asserts
  the catalog/map agreement, every visible row's owner, the omitted-pane set,
  and the debug view.

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
  the schema defaults** with no settingsd. A write lands in memory for the
  session and is reported as `deferred`; when the daemon returns, `GetAll`
  reconciles it away ([ADR 0030](adr/0030-settingsd-schema-and-dbus-surface.md),
  [ADR 0032](adr/0032-shell-settings-client.md)). There is no "settings
  unavailable" banner.
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
| Accessibility | AT-SPI (bridge host) + settingsd | status note; `accessibility.reduceMotion`/`reduceTransparency`/`increaseContrast` stay live | **hides** |

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

## Live apply, validation, admin auth, and absence (T224)

Every settingsd-backed control goes through one write path in the shared
client. `Settings.apply(key, value)` (and its `Settings.set` compatibility
alias) first runs `SettingsClient::validate` against the same schema mirror the
owner uses: unknown keys, wrong types, out-of-range numbers, and values outside
an enum are rejected before anything is written. A valid write is applied
optimistically on the same event-loop turn, marked `pending`, and sent to
settingsd. Success clears the state; a failure reverts to the previous value
and records the owner's message. A monotonic generation per key makes a stale
reply from an older write harmless, so a dragged slider cannot be clobbered by
an earlier answer.

The message is part of the row, not a global banner: `SettingsRow.errorText`
draws it under the label/description, and `SettingsKeyRow` binds that property
to `Settings.writeErrors[key]`. `Settings.schemaFor(key)` exposes the declared
type, default, range, and allowed values for a row that wants to size a bounded
control from the owner's contract. `Settings.stateFor(key)` is
`idle`/`pending`/`deferred`/`error`.

Privileged account and group actions are authorized by the host polkit
authority (`CheckAuthorization` with `AllowUserInteraction`). The shell's
registered polkit agent renders the system prompt and owns the PAM
conversation; the Settings app never renders a password field. A denial,
cancellation, wrong password, or unavailable authority leaves the view
untouched and publishes `Settings.accountsError`, which the Users & Groups pane
shows next to the affected section. The deterministic fixture seam is
`Settings.setAuthFixture(...)`; the live path talks to the authority on the
system bus.

Absence has one policy table, `SettingsAbsence`, with the five design kinds:

| Kind | Controls | Note | Example |
|---|---|---|---|
| `live-on-defaults` | visible, enabled | optional | settingsd-backed panes with no daemon |
| `disabled-with-reason` | visible, inert | required | Bluetooth with no BlueZ/bridge host |
| `hidden` | not drawn | none | a row that does not exist on this host |
| `read-only` | visible, inert | required | observable state with no writable owner |
| `delegated` | not drawn; hand-off action | required | a third-party service's own UI |

`SettingsAbsenceNotice` renders the note/action and exposes the policy
(`controlsVisible`, `controlsEnabled`, `writable`, `delegated`). A pane keeps
its own enabling because it can mix a live settingsd half with an absent
adapter half. The executable map assigns all five kinds to panes or rows;
`tst_settings_apply_ui` fails if one is unused.

The settingsd restart resync keeps the T-08.3/ADR 0032 contract: a write made
while the daemon is away is in-memory for the session only and is reported as
`deferred`; when the daemon returns, `GetAll` is authoritative, the while-down
value is reconciled away, and the deferred state clears. Other controls keep
their values, so the pane never loses its live half.

The headless gate is three suites:

- **`tst_settings_apply`** (private `dbus-run-session`, fake owner) covers
  range/enum/type/unknown-key validation, repeated rejection replacing the
  message, an owner failure with optimistic apply then revert, service
  disappearance with deferred writes, and restart resync.
- **`tst_settings_auth`** (private bus, fake polkit authority) covers allow,
  deny, cancel, dismissed, authority error, and the wrong-password fixture; it
  asserts the `AllowUserInteraction` flag and the unix-session subject.
- **`tst_settings_apply_ui`** (settings + accounts fixtures) covers the inline
  error, the five absence kinds, every map kind being used, every common row
  naming a real schema key/control, and cancelled/denied/wrong-password/allow
  account actions with no password field.

The deterministic visual check is `make t224-settings-apply-capture`
(`scripts/capture-t224-settings-apply.sh`), writing
`docs/captures/t224-settings-{live,write-failure,auth-cancel,service-absence}.png`
through the real Settings shell offscreen. A host session can repeat the same
four states with the same fixtures.

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
