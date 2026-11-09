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
| Wi-Fi, Network | NetworkManager (Wi-Fi, services, VPN connections) + the firewall adapter (firewalld/ufw/nftables) ([adr/0146](adr/0146-network-advanced-vpn-adapter.md), [adr/0147](adr/0147-network-advanced-vpn-pane-and-tile.md), [adr/0234](adr/0234-network-panes-model-order-firewall.md)) |
| Bluetooth | BlueZ |
| Battery | UPower + power-profiles-daemon where present |
| General | `settingsd` + host/distro adapters (About, native/PackageKit Updates, Storage usage, Login Items, Language & Region, Date & Time, Sharing, Backup, Startup Disk, Reset) ([adr/0235](adr/0235-general-group-host-adapters.md)) |
| Appearance | `settingsd` (design-system themes, dark/light, accent, highlight, sidebar icon size, wallpaper tint, scroll bars, attached-menu transparency, motion link) ([adr/0236](adr/0236-settings-appearance-quick-settings-search-displays.md)) |
| Accessibility | live AT-SPI read + `settingsd` (`accessibility.reduceMotion`, `reduceTransparency`, `increaseContrast`) ([adr/0144](adr/0144-accessibility-adapter.md), [adr/0145](adr/0145-accessibility-pane-and-tile.md), [adr/0215](adr/0215-glass-live-settings-and-appearance-policy.md)) |
| Desktop & Dock | `settingsd` → shell + compositor; the Desktop & Stage Manager group writes `desktop.showItems` for the Files desktop, with T242/T245b rows disabled by reason |
| Displays | Compositor output API — **shipped:** scale, rotation, and brightness; **capability-gated:** refresh, arrangement, night light, color profile, HDR, and VRR are disabled with an OPEN reason until the compositor/colord half exists ([adr/0236](adr/0236-settings-appearance-quick-settings-search-displays.md)) |
| Wallpaper | `settingsd` + compositor (per-Space wallpaper); `wallpaperd` for the fetched Featured Pictures; Screen Saver…/Clock Appearance… deep-link to the Screen Saver pane ([adr/0239](adr/0239-wallpaper-notifications-focus-lock-panes.md)) |
| Menu Bar | `settingsd` → shell |
| Search | `settingsd` durable preferences → the T246 search service; the pane ships categories, indexed locations, shortcut, web search, and menu-bar visibility, while indexing status/rebuild are disabled with the service absent |
| Screen Saver | `settingsd` + shell/LockScreen — style and clock options reuse the first-party lock scene; Start After is the session idle engine's `idle.blank` |
| Notifications, Focus | Notification service — per-app preferences (`SetAppPreference`) and the Focus mode/allow list; schedule/shortcut are `settingsd` keys the shell's `FocusSchedule` consumes ([adr/0239](adr/0239-wallpaper-notifications-focus-lock-panes.md)) |
| Lock Screen | `settingsd` + compositor lock/idle policy; the shell pushes `lock.*` onto the first-party lock scene and the power row calls logind ([adr/0239](adr/0239-wallpaper-notifications-focus-lock-panes.md)) |
| Privacy & Security | Portals (`xdg-desktop-portal` PermissionStore) mapped to the macOS categories; Security reads the distro/Flatpak sources, LUKS, SELinux/AppArmor, portal extensions, and the Advanced inventory ([adr/0237](adr/0237-security-biometrics-online-accounts.md)) |
| Biometrics & Password | fprintd + PAM + AccountsService where present; PAM and LUKS status are read-only ([adr/0237](adr/0237-security-biometrics-online-accounts.md)) |
| Users & Groups | accountsservice + distro provider |
| Online Accounts | optional GNOME Online Accounts; the pane is hidden when GOA is absent ([adr/0237](adr/0237-security-biometrics-online-accounts.md)) |
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

The map has 29 top-level catalog entries and 28 shipped panes after T229; the
new `control-center` (Quick Settings), `search` (renamed from `spotlight`), and
`screen-saver` entries ship with T227, and T228 ships `biometrics` plus the
optional `online-accounts` entry. `game-controllers` remains unshipped until
T229 lands its body.
`storage`, `mission-control`, and `menu-bar` keep their existing ids and
top-level placement. The Help menu opens the in-app **Pane & Row
Owners…** debug view (also `DF_SETTINGS_DEBUG=1`) and the **Business-Logic
Map** document; the debug view shows the selected pane's owner/absence/deep
link and every row's owner/key/absence/state. See ADR
[0231](adr/0231-executable-settings-map.md).

## The General group sub-panes (T226)

The General group is one sidebar pane with an in-place sub-pane stack. About
has two surfaces after the 2026-11-08 structural review: a compact 280×503 pt
utility panel with a blank 32 pt title bar, and the canonical 740×820 pt
Settings page with 52 pt chrome and a scrolling 768 pt content area. Software
Update stays a dialog. T230c replaced the two About dialogs with an original
Dragonfruit 280×503 utility panel and the canonical in-place About page. The
system menu launches the Settings executable with `--about-system`; its blank
32 pt chrome, 194 pt vector hero at 29 pt, five host rows, and More Info action
belong to `AboutPanel.qml`. The panel follows the measured row block: the rows
start at 291 pt with a 16 pt pitch, right-aligned labels, and elided values;
More Info sits at 391 pt (94×26 pt), the regulatory link at 429 pt, and the
footer at 466 pt. More Info reveals the 740×820 Settings window and opens
`AboutPage.qml` under General. The page uses 52 pt unified chrome, a 128 pt
hero and a 768 pt scroll viewport. Both surfaces read the existing Updates
identity view; startup disk/free space use the existing General host view.
Startup disk prefers the root-volume label, then the parent disk, the root
device, and the mount point, so a btrfs/LUKS root reports a host value rather
than “Not available”. Missing fields, including serial/model/display, say
“Not available”. The utility material is a separate near-opaque vertical
gradient with a shadow, and reduced transparency makes it opaque. Warranty/
parts rows are omitted: no Linux service owns those values. The
license/regulatory actions disclose the available local information without
inventing certifications. Local captures of the live system-menu journey in
light/dark and the long/localized surfaces at 1× are under the repository's
gitignored `docs/captures/`; the same surfaces are tested at 1×/1.5×/2×.
Controlled material fit over the 21 reference backgrounds and a live localized
host remain explicit `OPEN` rows. See
[ADR 0246](adr/0246-about-two-surfaces.md).
Storage is the existing top-level pane; Login Items, Language & Region,
Date & Time, Sharing, Backup, Startup Disk, and Reset open in place with a
`‹ General` back control. Every row is a live summary of the same bridge-host
view the sub-pane renders, so the list never advertises a state the sub-pane
contradicts. See
[23-macos-structural-surface-review.md](23-macos-structural-surface-review.md).

The host half is `services/general-adapter` + `services/system-status/src/general.rs`
behind `org.dragonfruit.SystemStatus1.General`; the Settings client is
`apps/settings/GeneralClient.*` and the QML files are
`GeneralPane.qml`, `DateTimePane.qml`, `LanguageRegionPane.qml`,
`LoginItemsPane.qml`, `SharingPane.qml`, `BackupPane.qml`,
`StartupDiskPane.qml`, and `ResetPane.qml`. The native/PackageKit update
provider is attached to the `Updates` interface; the pane's automatic-update
toggles write `updates.auto`/`updates.autoInstall` and forward them to the
running provider. Both default off, so a fresh profile never silently enables
automatic updates (ADR [0235](adr/0235-general-group-host-adapters.md)).

## The T227 panes (Appearance, Quick Settings, Search, Screen Saver, Desktop & Stage, Displays advanced)

T227 completes the shell-owned settings panes. The new sidebar panes are
`control-center` (Quick Settings), `search`, and `screen-saver`; Appearance,
Desktop & Dock, and Displays gain their remaining groups. The executable map
and the sidebar catalog both mark 28 shipped panes after T229 (Online Accounts
is optional and appears only while GOA answers).

- **Appearance** adds highlight color (`appearance.highlight`), sidebar icon
  size (`appearance.sidebarIconSize`), wallpaper tinting
  (`appearance.wallpaperTint`), scroll-bar policy (`appearance.scrollBars`),
  and a Motion deep link. `shell/src/themebinding.*` maps the first three onto
  `Theme.highlightOverride`, the design-system `Sidebar` slots, and
  `Glass.wallpaperTint`; `ScrollView` consumes `Theme.scrollBars`. The Settings
  app mirrors the same bindings in `SettingsShell.qml`, so the pane and the
  live shell restyle on the same `Changed` signal.
- **Quick Settings** is a customization surface, not a second panel. It writes
  the ordered `quickSettings.modules` list and the `quickSettings.menuBarOnly`
  set. `ControlCenter.qml` filters and orders its live tile model from the same
  keys; removing a module hides it immediately, and a menu-bar-only item leaves
  the panel while its existing `menu.control.*` mark stays live. Per-display
  panels are disabled with a T235 reason (one Control Center surface today).
- **Search** writes the durable T246 contract (`search.categories`,
  `search.indexLocations`, `search.shortcut`, `search.web`, and
  `menu.control.search`). The indexing status and Rebuild rows are disabled
  with the T246 reason; the pane never shows a fake index.
- **Screen Saver** reuses the first-party lock scene. `screenSaver.style`
  selects the clock or none, `idle.blank` is Start After, and
  `screenSaver.clock`/`showDate`/`showSeconds` are pushed to `LockScreen` while
  locked. Animated savers are not advertised.
- **Desktop & Stage Manager** ships `desktop.showItems`, consumed by the Files
  desktop process. Click-wallpaper, Stage Manager, recent apps, and widgets are
  disabled with their owning task or an explicit OPEN reason.
- **Displays advanced** is the shared capability table in
  `libs/settings-client/displaycapabilities.*`. Brightness and rotation are
  live; refresh, arrangement, night light, color profile, HDR, and VRR are
  disabled with the exact missing compositor/colord half. The shell's
  `tst_displayspolicy` asserts the table.

The live evidence is `make t227-settings-panes-capture`:
`docs/captures/t227-{appearance,quick-settings,quick-settings-live,search,screen-saver,desktop-stage,displays-advanced,search-absence,displays-absence,quick-settings-absence}.png`.
The capture script runs the real shell offscreen with the deterministic
settings fixture, then the live `ControlCenter` panel with the customized
module list, then a second no-settingsd pass for the absence stills. See ADR
[0236](adr/0236-settings-appearance-quick-settings-search-displays.md).

## The T228 panes (Privacy & Security, Biometrics & Password, Online Accounts)

T228 completes the three host-service panes. The catalog then had **27 shipped
panes**; `online-accounts` is shipped with `optional: true`, so it appears only
while `org.gnome.OnlineAccounts` answers. The executable map carries the same
optional flag.

- **Privacy & Security** renders the fifteen macOS privacy rows mapped to the
  closest Linux portal permission (`Location`, `Camera`, `Microphone`,
  `Files and folders`, `Screen recording`, …). Each row states its Linux
  difference; delegated rows (Contacts, Calendars, Reminders, Speech,
  Accessibility, Full disk access, Automation, App management, Developer
  tools) name the host owner and never draw a fake TCC toggle. The remaining
  real portal tables (Notifications, USB, Remote Desktop, …) are listed under
  **Other portal permissions**. The Security group adds Software sources
  (Flatpak remotes writable, distribution repositories read-only), read-only
  Disk encryption (LUKS via `lsblk`), read-only Hardening (SELinux/AppArmor and
  Secure Boot), installed portal Extensions, and an Advanced sheet.
- **Biometrics & Password** shows the fprintd reader and enrolled fingers,
  enrolls/deletes through fprintd, and changes the password through
  AccountsService after polkit authorization. The PAM unlock/authentication
  rows are read-only status rows and are hidden when `pam_fprintd.so` is not
  configured.
- **Online Accounts** lists the GOA providers and accounts, writes Add/Remove
  and the per-capability sync toggles, and is hidden when GOA is absent. It
  never implements OAuth.

The host half is `services/privacy-adapter` (`SecuritySource`) and
`services/account-adapter` (`BiometricsSource`, `OnlineAccountsSource`),
projected through the existing `Privacy` and `Accounts` bridge interfaces; the
Settings clients are `PrivacyClient` and `AccountsClient`, and the QML files
are `PrivacyPane.qml`, `BiometricsPane.qml`, and `OnlineAccountsPane.qml`. The
live evidence is `make t228-privacy-accounts-capture`
(`docs/captures/t228-privacy-list.png`, `t228-privacy-detail.png`,
`t228-security.png`, `t228-security-sources.png`, `t228-biometrics.png`,
`t228-online-accounts.png`, and the three absence stills). See ADR
[0237](adr/0237-security-biometrics-online-accounts.md).

## The T229a panes (Wallpaper, Notifications, Focus, Lock Screen)

The last four panes close the reference information architecture
([adr/0239](adr/0239-wallpaper-notifications-focus-lock-panes.md)):

- **Wallpaper** keeps the current preview, Show on all Workspaces, Fit, the
  Built-in/Featured/Custom sources, and the provider's fetching/offline/empty
  states. **Screen Saver…** and **Clock Appearance…** navigate to the Screen
  Saver pane instead of duplicating its controls.
- **Notifications** keeps the four settingsd presentation preferences, adds
  one live per-app sheet (Allow, Style, Sound, Badge) over the notification
  service's new `SetAppPreference`, and links to Focus. The service enforces
  the mute and `style = none` in admission; the app inventory is the observed
  list, not a guessed installed-app set.
- **Focus** renders the three modes with their admission rule and active
  state, the per-app allow list, an explicit Allowed People note (senders are
  apps, not people), the durable `focus.schedule.*` schedule, and the
  `focus.shortcut` recorder. `shell/src/focusschedule.cpp` is the one consumer:
  it applies Focus at the opening boundary and Off at the closing boundary,
  and a manual mode change wins until the next window.
- **Lock Screen** writes the `lock.*` display keys and the shell now pushes
  them onto the first-party lock scene (identity, password hint, message,
  power buttons). The power row calls logind `Suspend`/`Reboot`/`PowerOff`
  through the same shell path as the system menu. The Login window row is
  disabled with the display-manager reason; Accessibility Options navigates
  to the Accessibility pane.

Schema revision 23 adds `focus.schedule.enabled/start/end/days`,
`focus.shortcut`, `lock.passwordHint`, and `login.windowStyle`.

## The T229 panes (Sound, Keyboard/Input, Mouse/Trackpad, Controllers, Users & Groups, Printers & Scanners, Battery/Storage)

T229 closes the remaining Settings breadth with real host adapters and one
absence path per row. The catalog now has **28 shipped panes** (Online Accounts
remains optional).

- **Sound** adds the Test action: `Audio.PlayTestSound` runs `pw-play`
  (`paplay` fallback) on a system sound file; no player/file disables only the
  button. The existing device table, volume, balance, mute, startup/UI/feedback
  rows are unchanged.
- **Keyboard** adds the ordered `input.sources` list (the first entry is the
  active layout, forwarded to the compositor keymap in protocol v10),
  `input.replacements` (`from\tto` entries for the T253 input method), and a
  Dictation row enabled only when a speech-to-text command is installed.
  Repeat, backlight, navigation, and the emoji-key action stay live.
- **Mouse/Trackpad** add Secondary click, Scroll speed, Double-click speed,
  Look up, and Haptics. They travel with the existing pointer keys in
  `df_toplevel_manager.set_pointer_policy` (v10); the compositor stores them in
  `PointerSettings` and the backend applies the default when it configures a
  libinput device. Haptics is enabled only when a touchpad is present; the
  Bluetooth-trackpad hand-off navigates to Bluetooth and disables when BlueZ is
  absent.
- **Game Controllers** is a shipped, optional pane. `services/input` reads
  `/proc/bus/input/devices` for joystick/gamepad records and the SDL mapping
  table; `Test` reads one event from the `js`/`event` node with a two-second
  timeout and reports event/timeout/permission/absent; `Mapping` is read-only.
  A remapper and mapping editor are out of scope.
- **Users & Groups** adds the `wheel`/`sudo` admin-group row and a read-only
  Directory account projection (`/etc/sssd/sssd.conf` domains/provider plus
  `realm list`). Joining a domain stays delegated to `realm`/SSSD.
- **Printers & Scanners** adds `lpinfo -v` discovery, `lpadmin` add, and a
  per-queue `lpoptions -l`/`-o` Options & Supplies sheet. A queue with no
  options is an honest empty sheet; a host with no CUPS disables Add/Options.
- **Battery** adds the Options group (`power.showPercentage`, persisted
  `power.historyRange`) and a Power Settings group that reads `idle.blank`/
  `idle.lock` and navigates to Lock Screen. Storage's usage categories and
  Manage row are now shipped rows in the map.

Schema revision 22 adds `input.secondaryClick`, `input.scrollSpeed`,
`input.doubleClickSpeed`, `input.haptics`, `input.lookUp`, `input.sources`,
`input.replacements`, `input.dictation`, `power.showPercentage`, and
`power.historyRange`. The input adapter grows the controller inventory and the
diagnostic test read; the audio adapter grows one playback action; the printer
adapter grows discovery/add/options; the account adapter grows the directory
projection and admin-group helper. See ADR
[0238](adr/0238-input-breadth-users-printers-battery.md). Live evidence is
`make t229-input-users-printers-capture` (`docs/captures/t229-*.png`).

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

## Wallpaper sources (T-18 / ADR 0240)

The Wallpaper pane is a flat gallery, not a card stack: a 160×100 hero with the
current name, Fit, Show on all Workspaces, and the Screen Saver/Clock
Appearance links, then collection rows with four tiles per row and a
**Show All (N)** / **Show Less** toggle. The online rows are **Abstract**
(Wikimedia Commons), **Nature**, **Water**, **Cityscapes**, **Earth**,
**Space** (NASA Image and Video Library), **Scenery**, and **Underwater**;
**Built-in** holds the shipped original default
`assets/graphics/wallpapers/Default.jpg` plus the original gradients, and
**Your Photos** starts with the portal **Add Photo…** tile.

The out-of-box background is the shipped original default, so first run and a
cold cache never depend on the network. The provider warms lazily on launch and
eagerly when the pane is opened, caching images and attribution under
`$XDG_CACHE_HOME/dragonfruit/`. Wikimedia images are downloaded in full during
the catalogue fetch; NASA Space thumbnails are cached first and the full
original is downloaded on demand through `FetchItem`. The effective source is
`wallpaper.source` (a user choice) when non-empty, otherwise the shipped
`wallpaper.builtinDefault`, otherwise the fetched `wallpaper.providerSource`.

Clicking a tile enters an explicit apply state: the old preview stays, the tile
and hero show a progress ring, a deferred full image downloads if needed, and
`wallpaper.source` is written only when the new image is ready. A failure keeps
the previous wallpaper. While the catalogue is fetching, empty rows show
reduced-motion-aware skeletons; a cold/offline/error catalogue shows the
available-soon note plus **Try Again**, never an error. The provider, key
ownership, effective-source precedence, cache policy, offline behavior, and
licensing are fixed by ADRs
[0055](adr/0055-online-wallpaper-content-provider.md) /
[0094](adr/0094-bundled-default-wallpaper-and-lazy-cache.md) /
[0240](adr/0240-wallpaper-pane-collections-and-additional-sources.md).

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

T-15 landed the Wave-2/3 panes and their Control Center tiles; T225–T228 added
the later panes, taking the catalog to **28 shipped panes**
(`SettingsPanes.shippedPanes`; Online Accounts is optional). Each pane routes
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
| Quick Settings | settingsd → shell/ControlCenter | live on defaults; the panel keeps the built-in module order and hides only absent modules | — (none) |
| Search | settingsd (the T246 service is a separate provider) | live on defaults; categories/locations/shortcut/web stay live, indexing status/rebuild disabled with the T246 reason | — (none) |
| Screen Saver | settingsd + shell/LockScreen | live on defaults; style/clock options stay live, the lock scene renders the schema defaults | — (none) |
| Desktop & Dock | settingsd → shell/compositor | live on defaults; no shell applier, but no error | — (none) |
| Mission Control | settingsd + compositor | settingsd keys live on defaults; the note says the daemon is gone | **hides** (no compositor bridge) |
| Displays | compositor output API + settingsd | live on defaults; shell applies the policy when an output appears | — (none) |
| Wallpaper | settingsd + `wallpaperd` + FileChooser portal | built-in/toggle/fit live; Featured empty (never an error); the "Add Photo…" portal row **disabled** with its explanation | — (none) |
| Wi-Fi | NetworkManager (bridge host) + settingsd prompt policy | note; radio/join/forget disabled; the two `network.wifi.*` policy rows stay live on defaults | **hides** |
| Network | NetworkManager + firewall adapter (bridge host) | note; service/VPN writes disabled; firewall "No firewall detected" or Unavailable (never a fake Off); settingsd policy rows stay live | **hides** |
| Bluetooth | BlueZ (bridge host) | note; toggle disabled; no write | **hides** |
| Network (VPN) | NetworkManager (bridge host) | note; connection list empty; `Connect`/`Deactivate` are no-ops; no plugin hides Add | **hides** |
| Battery | UPower + power-profiles-daemon (bridge host) | note; no profile write; Options stay live on defaults | **hides** |
| Storage | UDisks2/GIO (bridge host) | note; no mount/eject | **hides** |
| General | settingsd + General/Updates host (bridge host) | About/Update rows stay live when only the General host is absent; each sub-pane shows its own absence note | **hides** (the update tile hides only when the host is absent) |
| Sound | PipeWire/WirePlumber (bridge host) + settingsd | routing half note; Test disabled; Sound Effects/Balance stay live on defaults | volume tile `enabled: audioAvailable` |
| Keyboard | libinput (bridge host) + settingsd | inventory note; every preference row live on defaults; input sources/replacements live; Dictation disabled with its reason | **hides** |
| Mouse | libinput (bridge host) + settingsd | inventory note; pointer settings live on defaults | — (shared Keyboard tile) |
| Trackpad | libinput (bridge host) + settingsd | inventory note; tap-to-click/haptics/look-up live on defaults; haptics disabled with no touchpad | — (shared Keyboard tile) |
| Game Controllers | udev/SDL inventory (bridge host) | note; list empty; Test/Mapping disabled | **hides** |
| Notifications | notification service (bridge host) + settingsd | per-app inventory and its Allow/Style/Sound/Badge sheet hide behind the note; the four presentation prefs stay live | — (none) |
| Focus | notification service (bridge host) + settingsd | mode/allow-list and the people note hide; schedule and shortcut stay live on defaults | **hides** |
| Lock Screen | settingsd + compositor lock/idle | controls live on defaults; the note names the daemon; the lock scene renders the schema defaults | **hides** |
| Menu Bar | settingsd → shell | controls live on defaults; the note names the daemon | **hides** |
| Users & Groups | accountsservice/distro (bridge host) | note; read-only tile; Directory account is a read-only projection | **hides** |
| Printers & Scanners | CUPS/SANE (bridge host) | note; Add/Options/queue writes disabled | **hides** |
| Privacy & Security | portal `PermissionStore` (bridge host) | note; no permission write | **hides** |
| Accessibility | AT-SPI (bridge host) + settingsd | status note; `accessibility.reduceMotion`/`reduceTransparency`/`increaseContrast` stay live | **hides** |

The headless half is the CI gate:

- **`apps/settings/tests/tst_settings_absence.qml`** runs under a private
  `dbus-run-session` with no settingsd, no portal, and no bridge host. It
  asserts both providers are absent, that all 28 shipped panes have a body and
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
