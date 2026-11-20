# System Settings visual parity

| | |
|---|---|
| **Status** | accepted target for the `system-settings-2` set |
| **Date** | 2026-11-17 |
| **Reference** | sibling `macos-design-review` checkout, `mockups/settings`, commit `b60d97c` |
| **Supersedes** | [`24-system-settings.md`](24-system-settings.md) §3.1–3.2 and the pane inventory in §4 where this document says so; the row ownership and absence rules in §3.4–3.7 still apply |
| **Task set** | [`docs/tasks/system-settings-2/`](../tasks/system-settings-2/README.md) |
| **ADR** | [0287](adr/0287-system-settings-visual-parity.md) |

## 1. Why this document exists

The first System Settings set (T263–T284) made the app truthful: every pane
had an owner, an absence path, a keyboard path, and a tested consumer. It did
not make the app look like one product. The 2026-11-17 mockup is a high-fidelity
UI/UX study of the same window with the Linux-only decisions already applied.
This document is the contract for the visual/IA pass that follows it.

The mockup is a browser document, not a widget toolkit. Use its layout,
information architecture, labels, and interaction model. Do not copy its CSS
values or SVG paths into production where the design system already has a
measured token or glyph. The mockup's own README says the same thing: the
material and tokens in `docs/material-schema.json` and
`design-system/tokens/tokens.json` remain the source of truth.

## 2. Reference checkout

Use the sibling checkout at `../macos-design-review`, relative to the
repository root, at commit `b60d97c4f6e437ce9fd90b5adb456d916239ba71`. The
files that matter:

| Path | Use |
|---|---|
| `macos-design-review/mockups/settings/README.md` | Geometry, tokens, adaptations, known gaps |
| `macos-design-review/mockups/settings/index.html` | Window shell, sidebar, toolbar, search, help |
| `macos-design-review/mockups/settings/styles.css` | Row/control geometry and state language |
| `macos-design-review/mockups/settings/js/panes.js` | Pane catalogue, sections, rows, labels |
| `macos-design-review/mockups/settings/js/app.js` | Sidebar/search rendering, popup/radio/slider behavior |
| `mockups/settings/js/icons.js` | Semantic icon names and tile treatment |

The checkout is a design reference only. The production build and ordinary
gates must work without it. Do not vendor the mockup or its assets into this
repository. Do not copy reference captures, names, or artwork.

The mockup's sheet buttons are inert, its lists are fixed examples, and its
material is a CSS approximation. The task set must use the real adapters,
sheets, dialogs, and design-system material. Do not copy the mockup's
placeholder data or treat an inert sheet button as a finished surface.

## 3. Global decisions

### 3.1 Window and chrome

| Element | Target |
|---|---|
| Window | 740 × 820 pt, 10 pt radius, 720 × 480 minimum |
| Chrome | 52 pt unified toolbar |
| Traffic lights | 12 pt circles, 11 pt gap, 18 pt from the left, 22 pt from the top |
| Sidebar | 232 pt, full height, translucent material under the toolbar |
| Detail content | 508 pt wide, 468 pt column, 20 pt side insets |
| Content top inset | 24 pt below the toolbar (76 pt from the window top) |
| Content bottom padding | 64 pt so the last row clears the help control |
| Toolbar | Back/forward pill 78 × 36 pt, 1 × 18 pt divider, title 15 pt semibold left-aligned 12 pt after the pill |
| Pane title | The current pane or open sub-pane; the compositor/taskbar title follows it |
| Sidebar search | 212 × 28 pt, 10 pt side margins, 64 pt from the window top |
| Sidebar rows | 32 pt high, 10 pt side margins, 8 pt radius, neutral selected fill, semibold selected label |
| Plain rows | 33 pt high |
| Icon rows | 53 pt high, 29 pt tile |
| Description rows | 52 pt high |
| Slider-scale rows | 42 pt high |
| Cards | 10 pt radius, no inner card padding; rows own 12 pt horizontal padding |
| Section headings | 13 pt semibold, 26 pt above, 8 pt below |
| Body / secondary | 13 pt / 11 pt |
| Help button | 24 pt circle, 20 pt from the content right/bottom |

The mockup's title is left-aligned beside the history control. The 2026-11-14
review's “centred title” note described an older build; this document replaces
it for System Settings. The shared `TitleBar` keeps its centred default for
other apps and gains a left-aligned Settings variant.

The detail toolbar uses the content surface, not a second chrome sheet. The
sidebar keeps the `settingsSidebar` material; Reduce Transparency and Increase
Contrast still follow the shared Glass resolver.

### 3.2 Sidebar

The sidebar is one ordered catalog with three spacer groups and no visible
group headings. The groups are logical, not titled:

| Group | Entries |
|---|---|
| Top | Software Update status row, Wi-Fi, Bluetooth, Network, VPN, Energy |
| Middle | General, Accessibility, Appearance, Desktop & Dock, Displays, Menu Bar, Search, Wallpaper, Screen Saver |
| Bottom | Notifications, Sound, Focus, Lock Screen, Privacy & Security, Users & Groups, Online Accounts, Keyboard, Mouse, Trackpad, Game Controllers, Printers & Scanners |

The spacer is 13 pt. The status row is a real row: it shows the current update
state and badge from the update provider, and opens General → Software Update.
It is not a fake notification.

T285 makes this executable: `SettingsPanes.catalog` is the 26-pane order and
`SettingsPanes.sidebarStatus` is the status row the shell prepends to the first
group. The sidebar is 27 entries including that row. The label and badge are
seeded from the provider contract; T286 renders the live row.

General remains the default pane. The last-pane and window-geometry persistence
from T263 stays; a deep link wins over the restored pane.

The logical group names stay available to AT-SPI even though they are not
painted. A screen reader must hear “Connectivity”, “Desktop”, or “Input” rather
than a flat list with no structure.

### 3.3 Pane content

The mockup opens most panes with intro text, not a large header card. The
General, Accessibility, Notifications, and Privacy header cards are removed.
Their descriptions become `SettingsIntro` text. T287 moves General's intro
into the pane body above its big list; the other three stay shell intros. The
toolbar title is the only pane title.

The mockup uses three content shapes:

- **Form pane**: section heading, card of rows. Most panes use this.
- **Big-list pane**: a card of 52 pt icon rows that opens a sub-pane. General,
  Accessibility, Privacy & Security, and the top-level Focus list use this.
- **Custom pane**: a wall-top preview, a chart, a storage bar, a hero, or a
  device grid. Wallpaper, Energy, Storage, About, and Lock Screen use this.

Every pane ends with an optional footer of right-aligned buttons (or left-
aligned Add/Remove actions) and an optional note. The mockup's footer spacing
is 18 pt above the buttons.

### 3.4 Row anatomy

Extend the existing `SettingsRow`/`SettingsGroup`; do not fork a second row
component. The shared row keeps its label, description, control slot,
disclosure, trailing action, help, error, focus ring, and AT-SPI contract.
Add these variants:

- `SettingsRow` with a 29 pt leading category tile (icon row).
- `SettingsRow` with a 33 pt plain height and a 52 pt description height.
- `SettingsBigRow` inside `SettingsBigList`: 52 pt, 29 pt tile, optional
  second line, chevron.
- `SettingsFooter`: right/left button row, 18 pt top gap.
- `SettingsIntro`: body text plus an optional `Learn More…` link.
- `SettingsNote`: 12 pt muted note, 16 pt line height.
- `SettingsBanner`: a titled delegate banner for a service-owned surface.
- `SettingsServiceRow`: icon, label, status text, and a switch or status dot.
- `SettingsScanRow`: a “Scanning…” / “Searching…” row with a spinner.
- `SettingsRadioGroup`: stacked and inline radios, 16 pt circle, accent fill.
- `SettingsScaleSlider`: 246 pt slider with tick marks and Slow/Fast labels.
- `SettingsModeTile`: the three appearance thumbnails.
- `SettingsNavPill`: the 78 × 36 back/forward control.
- `SettingsHelpButton`: the 24 pt circle and its live document popover.
- `SettingsSidebarStatusRow`: the update status row and badge.
- `SettingsCollectionHead`: heading plus a `Show All` link.
- `SettingsWallTile`: a wallpaper thumbnail with selected ring and name.
- `SettingsChart`: the battery chart frame, axes, and bars.
- `SettingsHero`: the About hero.
- `SettingsStorageBar`: the storage summary and legend.
- `SettingsAvatar`: the user/account avatar.
- `SettingsDeviceRow`, `SettingsDiskRow`: the Sound/Startup Disk rows.

The shared components own the mockup's geometry. Pane tasks may add a new
component only when the category needs it; the component lands in
`design-system/components/`, not in a pane-local copy.

### 3.5 Controls

The mockup uses the existing control families with these sizes:

| Control | Target |
|---|---|
| Switch | 38 × 22 pt |
| Segmented | Compact; wide/prominent only where the mockup uses it |
| Popup/select | Sized to the longest option, chevron on the right |
| Radio | 16 pt circle, 8 pt inner dot |
| Slider | 160 pt default; 246 pt with tick scale |
| Button | Compact text button; primary only for the default action |
| Text field | 28 pt pill for search; standard row field otherwise |

The controls keep the existing focus ring, keyboard activation, AT-SPI name,
current value, range/allowed values, and reduced-motion path. The mockup's
controls are visual targets, not a new widget toolkit.

### 3.6 Icons

The mockup's icon names are semantic. Map them to the design system's
Phosphor/Tabler glyphs and the `SettingsPanes.categoryStyles` table. Do not copy
the mockup's SVG paths into production.

Add category styles for the names the new sidebar and big lists need:
`mouse`, `gamepad`, `controller`, `about`, `update`, `storage`, `clock`,
`language`, `login`, `sharing`, `startup`, `backup`, `device`, `transfer`,
`password`, `camera`, `mic`, `location`, `files`, `photos`, `eye`, `ear`,
`speech`, `cloud`, `hand`. If the vendored glyph set has no exact name, use the
closest existing glyph and record the mapping in
`docs/design/10-design-system.md` or the category table. A pane's sidebar tile
and its big-list tile must be the same identity.

The mockup's colored tile is the existing `SettingsCategoryIcon` gradient
container, not a bitmap. No new icon artwork is required unless a semantic name
has no reasonable existing glyph.

### 3.7 Row ownership policy

Every rendered row has one of:

- a settingsd key with a real consumer;
- a host adapter value with a real consumer;
- a delegate action to a named service;
- a read-only status with a named owner;
- an absence row for a present-but-unavailable service.

A mockup row with no owner, consumer, delegate, or read-only owner is omitted.
The omission is recorded in `SettingsMap` with `state: "omitted"`,
`absence: "omitted"`, and a one-line reason. It is not search-indexed and not
rendered. “Show it disabled” is not the default for an ownerless feature; a
disabled row is only for a real service that is present but unavailable.

Do not add a settingsd key for a mockup row unless the key has a consumer in
the same change. If a new key is needed, add the schema revision, the client
mirror, `docs/design/settings-keys.md`, the consumer test, and the map row together.

### 3.8 Removed, renamed, and merged panes

| Current | Decision |
|---|---|
| Biometrics & Password | **Removed.** No fprintd/Fingerprint feature is offered. Delete the pane, map rows, tests, and docs. |
| Overview (Overview) | **Removed as a top-level pane.** Its rows move to Desktop & Dock → Workspace Overview. Keep `settings://pane/mission-control` and `settings://pane/overview` as aliases to that section. |
| Quick Settings | **Removed as a top-level pane.** Rename it **Menu Bar Controls** and move it under Menu Bar. Keep `settings://pane/control-center` and `settings://pane/quick-settings` as aliases to Menu Bar → Menu Bar Controls. |
| Screen Saver | **Kept as its own pane** after Wallpaper. The mockup puts it under Wallpaper; this is the one deliberate deviation. Wallpaper keeps a `Screen Saver…` footer link. |
| Online Accounts | The mockup's **Online Accounts** label is the macOS term; the naming gate maps it to the generic **Online Accounts**. Keep the stable `online-accounts` id, keep the title Online Accounts, and add `settings://pane/internet-accounts` as an alias plus the search synonym. |
| Autofill & Passwords | Renamed **Passwords & Keys**. Keep the `passwords` id. |
| Backup | Renamed **Backups**. Keep the `backup` id. |
| Overview | Renamed **Workspace Overview** in the Desktop & Dock section. |
| Stage | Renamed **Workspace Focus**. |
| Storage | Stays a hidden General sub-pane; `settings://pane/storage` remains an alias. |
| VPN | Becomes always visible, with the mockup's empty state. The optional-profile gate is removed. |
| Mouse, Game Controllers | Kept as top-level panes. They show their absent-hardware state when no device is present. |

### 3.8a T286 implementation notes

T286 lands §3.1–3.4 as tokens and shared components (see
[08-settings.md](08-settings.md) and [10-design-system.md](10-design-system.md)).
The target values are unchanged; these are the implementation choices worth
recording:

- The traffic lights draw the mockup's 12 pt circle inside the shared 16 pt
  hit frame. The 11 pt edge gap is the 23 pt centre spacing the shared
  `TrafficLights` already uses, so the visible circles match the mockup while
  the hit target stays accessible.
- The sidebar status row stays present when the provider reports no update
  (label "Up to Date"), rather than disappearing. It is the honest current
  state and keeps the General → Software Update entry reachable.
- The help control opens the live business-logic map anchor directly. The
  mockup's "Help content is not included" placeholder is not shipped; a
  minimal install with no map document disables the control and explains why.
- The sidebar search keeps its query after a result opens. The pane becomes
  visible; the next keystroke re-opens the ranked list.
- The neutral selected fill is a Settings opt-in on `Sidebar`
  (`selectionStyle: "neutral"`). It is not a global accent change; Files and
  the Applications drawer keep the accent selection.
- The detail toolbar is the content surface. The `settingsToolbar` glass
  preset remains in the token set for other consumers and for the increase
  contrast edge, but the shell no longer paints a second chrome sheet.

### 3.9 Deep links

| Link | Target |
|---|---|
| `settings://pane/storage` | General → Storage |
| `settings://pane/mission-control` | Desktop & Dock → Workspace Overview |
| `settings://pane/overview` | Desktop & Dock → Workspace Overview |
| `settings://pane/control-center` | Menu Bar → Menu Bar Controls |
| `settings://pane/quick-settings` | Menu Bar → Menu Bar Controls |
| `settings://pane/internet-accounts` | Online Accounts |
| `settings://pane/biometrics` | Refused; the pane no longer exists |

A refused link returns `false` from the shell resolver and is surfaced by the
extras host as a rejection, never as a silent no-op.

The table is executable as `SettingsPanes.aliases` and mirrored one-for-one in
`SettingsMap.aliases` (T285). `SettingsPanes.removed`/`SettingsMap.removed`
carry the reason the three ids left the top-level catalog; the debug view shows
it. Biometrics has no alias, so `resolve()` returns null and the shell sets
`lastDeepLinkError` before returning `false`.

## 4. Pane inventory

| Id | Sidebar title | Disposition | Owner |
|---|---|---|---|
| — | Software Update Available | status row | update provider |
| `wifi` | Wi-Fi | ship | NetworkManager |
| `bluetooth` | Bluetooth | ship | BlueZ |
| `network` | Network | ship | NetworkManager + firewall adapter |
| `vpn` | VPN | ship (always visible) | NetworkManager VPN |
| `battery` | Energy | ship | UPower + power-profiles-daemon |
| `general` | General | ship | settingsd + host/distro adapters |
| `storage` | — (hidden) | General → Storage alias | UDisks2 + filesystem usage |
| `accessibility` | Accessibility | ship | settingsd + compositor + AT-SPI |
| `appearance` | Appearance | ship | settingsd + compositor |
| `desktop-dock` | Desktop & Dock | ship | settingsd → compositor/shell |
| `mission-control` | — (hidden) | Desktop & Dock → Workspace Overview alias | settingsd + compositor |
| `displays` | Displays | ship | compositor + display adapter |
| `menu-bar` | Menu Bar | ship | settingsd → shell |
| `search` | Search | ship | search service + app-index + settingsd |
| `wallpaper` | Wallpaper | ship | settingsd + wallpaperd + compositor |
| `screen-saver` | Screen Saver | ship (kept) | shell/compositor + settingsd |
| `notifications` | Notifications | ship | notification service + settingsd |
| `sound` | Sound | ship | PipeWire/WirePlumber + settingsd |
| `focus` | Focus | ship | notification service + settingsd |
| `lock-screen` | Lock Screen | ship | settingsd + compositor/lock + logind |
| `privacy` | Privacy & Security | ship | portals + polkit + LUKS + firewall |
| `users-groups` | Users & Groups | ship | accountsservice + distro adapter + polkit |
| `online-accounts` | Online Accounts | ship | Online Accounts service |
| `keyboard` | Keyboard | ship | settingsd + compositor input + ibus/fcitx5 |
| `mouse` | Mouse | ship | settingsd + compositor input |
| `trackpad` | Trackpad | ship | settingsd + compositor input (libinput) |
| `game-controllers` | Game Controllers | ship | udev/SDL |
| `printers` | Printers & Scanners | ship | CUPS + SANE |

Removed: `biometrics`, `control-center` (as a top-level pane). Hidden aliases:
`storage`, `mission-control`. There is no top-level Overview or Quick Settings
row.

## 5. Per-category target

The mockup's pane data is the exact section/row inventory. This section records
the dispositions that differ from the current app and the rows that have no
owner. A task may not add a row that is not in this table or the mockup.

### 5.1 General

Mockup: intro + big list of 12 rows (About, Software Update, Storage,
Passwords & Keys, Date & Time, Language & Region, Login Items & Extensions,
Sharing, Startup Disk, Backups, device management, Transfer or Reset).

| Mockup row | Disposition | Owner / key |
|---|---|---|
| About | ship; hero + host rows | host identity, `updates.*` |
| Software Update | ship; provider card | `updates.auto`, `updates.autoInstall`, provider |
| Storage | ship; General → Storage | UDisks2 + filesystem usage |
| Passwords & Keys | ship; rename | `defaults.passwordManager` |
| Date & Time | ship | `clock.use24Hour`, timedatectl |
| Language & Region | ship | `locale.*` |
| Login Items & Extensions | ship | XDG autostart + systemd user units |
| Sharing | ship with live service rows | host sharing services |
| Startup Disk | ship; read-only disk + restart | firmware/boot adapter |
| Backups | ship; rename | `backup.*` |
| device management | **omit** | No Linux MDM/profile owner in scope; map reason |
| Transfer or Reset | ship | reset/transfer actions |

Sub-pane dispositions:

- **About**: keep the real host identity, add the mockup hero, group rows, and
  about-foot links. Energy Compliance is omitted unless a host document exists.
- **Software Update**: keep the provider card and automatic-update keys. Omit
  “Update Tonight” and “Beta Updates” unless a schedule/beta provider exists.
  “Also Available” is shown only when the provider reports additional items.
- **Storage**: add the storage bar and legend from real usage. Keep
  Recommendations only where a consumer exists. Keep Volumes and Removable
  Media as the Linux data that exists; omit the macOS category rows that have
  no sampler.
- **Passwords & Keys**: keep the provider selection and keyring status. The
  AutoFill toggle and “Delete After Use” are omitted until a consumer exists.
- **Date & Time**: keep NTP/time-zone/24-hour rows. “Time zone city” uses the
  host's timedatectl list.
- **Language & Region**: keep preferred languages, previews, and the live
  format keys. Omit “Text in Images”, per-app language settings, and
  “Translation Languages” until an owner exists.
- **Login Items & Extensions**: keep XDG autostart and systemd user units.
  “Background App Activity” is the existing Allow in Background list. The
  extension list stays a delegate/managed surface.
- **Sharing**: keep the mockup's three sections, but render only services the
  host reports. The fixed macOS rows with no Linux service are omitted with map
  reasons; the empty state names the missing services.
- **Startup Disk**: keep the read-only disk and restart action; firmware/secure
  boot details stay in an advanced section or sheet.
- **Backups**: keep the empty state and Add Backup Disk action. When a backend
  exists, show backend/destination/schedule rows in the same section.
- **Transfer or Reset**: keep transfer-to/from, reset settings, and the
  explicit “Erase All Content and Settings is not offered on Linux” note.

**Landed (T287).** The General list is the mockup intro plus an 11-row
`SettingsBigList`: About, Software Update, Storage, Passwords & Keys, Date &
Time, Language & Region, Login Items & Extensions, Sharing, Startup Disk,
Backups, Transfer or Reset. Every row is a 52 pt `SettingsBigRow` with a 29 pt
category tile; Passwords & Keys and Backups carry the mockup sub lines, and
the other rows stay single-line. device management is not rendered: it is a
`state: "omitted"` row in `SettingsMap` with the reason “no Linux MDM/profile
owner in scope”. The row tiles map to the closest vendored Phosphor glyphs
(`info`, `download-simple`, `hard-drives`, `lock-simple`, `clock-counter-clockwise`,
`globe-simple`, `power`, `users`, `hard-drive`, `arrow-clockwise`,
`arrows-clockwise`); `SettingsPanes.rowStyle(name)` resolves them.

The sub-pane dispositions landed as written: About uses the hero and the five
host rows plus Dragonfruit OS/Displays/Storage and the about-foot links;
Software Update is a provider card with Also Available only when the provider
lists items and Beta Updates only when a beta channel is advertised; Storage
keeps the real sampler's categories and shows the summary bar/legend; Passwords
& Keys keeps the keyring banner, provider selection and keyring status;
Date & Time adds Source and the derived Time zone city; Language & Region adds
the date/time/currency/number previews; Login Items adds Background App
Activity and the By App/By Category extension view; Sharing renders Content &
Media, Accessories & Internet, and Advanced from the live service list;
Startup Disk adds the intro, selected-disk note and an Advanced group for
firmware/Secure Boot; Backups keeps the empty/backend states and Add Backup
Disk; Transfer or Reset names the transfer assistant and the Linux erase
limitation. The ownerless rows the mockup still lists are map omissions, not
disabled controls: Energy Compliance, Beta Updates, AutoFill Passwords and
Keys, Delete After Use, Text in Images, Applications, Translation Languages,
Content Caching, Bluetooth Sharing, and Remote Application Scripting.

### 5.2 Connectivity

| Pane | Target |
|---|---|
| Wi-Fi | Intro, toggle, current network icon row, Known Network, Other Networks with scan/connect rows, Other…/Advanced… footers, Ask to join networks/hotspots. Keep the real NetworkManager data and join dialogs. Personal Hotspot moves to the Advanced sheet. |
| Bluetooth | Intro, toggle, discoverable note, My Devices, Nearby Devices with searching. Keep the real BlueZ data and device/advanced sheets. |
| Network | Service rows (Wi-Fi, Firewall, other services), Other Services, More popup, VPN section, details, Add Service. Keep real NetworkManager/firewall data. |
| VPN | Always visible. Intro, empty state, Add VPN Configuration…, connection rows, details. Keep WireGuard/NetworkManager VPN. |

The Wi-Fi “Ask to join” keys are stored-only today; the pane must say so
rather than implying the shell prompts. The Bluetooth discoverable note is
only shown while the pane is open and the adapter is discoverable.

**Landed (T288).** Wi-Fi is the mockup intro, the toggle card, the current
network icon row, Known Networks, Other Networks, the Other… footer, the
stored-only Ask to Join rows, and the Advanced… footer. Current/known/other
rows are `SettingsIconRow`s; the scan progress state is `SettingsScanRow`, and
a real "Scan for networks" action row sits beside it when no scan is running.
The row action is Connect (the mockup's word). The details sheet owns
Connect/Disconnect/Forget; Personal Hotspot moved into the Advanced sheet with
the Linux-only TCP/IP/hardware rows. Bluetooth is the intro, the toggle row,
the discoverable note (only while the radio is on and the adapter reports
discoverable), My Devices, Nearby Devices with the searching row, and the
existing device info/Advanced sheets. Network is one service card (Wi-Fi,
Firewall) built from `SettingsServiceRow`, an Other Services card of the real
NetworkManager services, the existing VPN section, and a More footer whose
popup offers Set Service Order… (a real `autoconnect-priority` sheet) and Add
Service…. VPN is always visible with its intro, the "No VPN configurations"
empty state, connection rows when configured, the WireGuard capability row,
and the Add VPN Configuration… footer. The optional-profile gate is removed
from `SettingsPanes`/`SettingsMap`. Every rendered row has a map row and a
search synonym; the four panes keep their structural absence.
`make t288-settings-connectivity-capture` writes connected/empty/absent stills
in light and dark.

### 5.3 Energy and Displays

| Pane | Target |
|---|---|
| Energy | Low Power Mode popup, Battery Health, Last 24 Hours / Last 10 Days, status pill, Battery Level chart, Options…. Keep Power Adapter/Power Settings in an advanced section or the existing Options sheet. |
| Displays | Display row, Resolution radios, scaling note, Brightness, auto-brightness, Ambient color (Adaptive color), Preset (color profile), Refresh rate, projector row, Advanced…/Night Light…. Move Arrangement, Rotation, and Color Profile into the Advanced sheet. |

“Preset” maps to the existing color-profile adapter; “Ambient color” maps to
adaptive color. “Projector mode” uses the existing “When connected to TV” row;
if the host has no TV/projector signal, the row is an absence row with a reason.

**Landed (T289).** Energy (the mockup label is Battery) is the mockup order: Low
Power Mode (the real power-profiles `Select`), Battery Health with a `Show
Detail` action, the Last 24 Hours / Last 10 Days `SegmentedControl` bound to
`power.historyRange`, the live charge-status pill (UPower's charge state plus
the real time-to-empty/time-to-full estimate), the `SettingsChart` Battery
Level chart with 100/50/0 % and range-labelled axes, and the Options… footer.
The old separate Charging row is folded into the pill; UPower exposes no
last-update timestamp, so the pill's second line is the estimate rather than
the mockup's example clock time. The Options… sheet holds the real `power.*`
policy rows, the Linux-only Power Adapter values, and the Lock Screen link;
they stay live when UPower is absent. “Last charged to 100%” and Screen On
Usage remain honest not-available rows.

**Label and desktop logic.** The sidebar says **Energy** on every machine.
macOS uses Battery on a laptop and Energy on a desktop for the same pane; the
mockup is a MacBook capture, so it says Battery. The id stays `battery` and
`settings://pane/energy` is an alias. The laptop/desktop row split and the
deliberate divergences from the capture are in
[26-settings-logic.md](26-settings-logic.md) §2 and
[ADR 0300](adr/0300-energy-category.md). Do not rename the category back to
match the capture.

Displays is the mockup order: a `SettingsIconRow` display row (the real output
name plus its logical `W × H (scale) · Hz`, chevron into the per-output sheet),
stacked `SettingsRadioGroup` resolution radios with real logical sizes,
“Show all resolutions” (a view filter over the five scale steps, not a new
key), the scaling note, Brightness, automatic brightness, Ambient color
(adaptive color), Preset (the color-profile path), Refresh rate (the real
current rate plus the disabled switching reason), the “When connected to a
projector” section, and Advanced…/Night Light… footers. The large preview art
is gone: the mockup's display row is the identity, and the per-output sheet
keeps the real selector. Arrangement, Rotation, Color Profile (as Preset),
Use as, and the advanced capability rows live in the Advanced… sheet; Night
Light's three rows live in the Night Light… sheet. The projector section is
the capability-gated `Projector mode` row only when the bridge marks an output
as a TV/projector; otherwise it is an absence row with the reason. The bridge
marks it from the EDID-derived name/model/manufacturer text Qt exposes
(`DF_DISPLAY_FIXTURE=projector` is the deterministic seam), never from a
hardcoded output list. `make t289-settings-battery-displays-capture` writes
both panes' present, sheet-open, projector, and absent stills in light and
dark.

### 5.4 Appearance

| Mockup block | Target |
|---|---|
| Appearance | Three visual mode tiles (Light/Dark/Auto), not a segmented control |
| Glass | The rendered glass preview + tint slider; keep `appearance.transparency` and state its attached-menu scope honestly |
| Theme | Accent swatches (+ custom hex) + Text highlight popup; four icon-style tiles + Folder color popup |
| Windows | Sidebar icon size popup; Tint window background with wallpaper color toggle |
| Show scroll bars | Stacked radios |
| Click in the scroll bar to | Stacked radios |
| Footer | Advanced Appearance… and More in Accessibility Settings… |

The Glass preview is 120 pt (the mockup's is 198 pt) and is the rendered
`Glass.resolve("menuAttached", …)` output, not a second formula.
`appearance.transparency` is the attached-menu tint only: it does not re-tint
the Dock, the menu bar, or windows, and the row description says so. A global
glass-tint key is a later task with its own consumer.

The four icon-style tiles are `appearance.iconStyle` (Default/Dark/Clear/
Tinted), mapped through the installed-theme provider. The installed icon theme
stays in the Advanced Appearance sheet as `appearance.iconTheme`, and an
explicit theme wins over the style mapping. Text highlight and Folder color are
single swatch popups over the design-system palette; the custom `#rrggbb`
entry point is accent-only. The sidebar icon size row lives in the Windows
card; the two scroll-bar controls are stacked `SettingsRadioGroup`s whose
option labels may elide at 640 px.

The Linux-only Appearance rows (icon theme, cursor theme, cursor size, window
control layout, material effects, accent from wallpaper) stay reachable in an
**Advanced Appearance…** sheet. They have real owners and must not disappear,
but the main pane follows the mockup. The footer's second action opens
Accessibility → Display; the pane does not duplicate an Accessibility row.

The separate Preview group is removed; the Glass preview is the preview.
`make t290-settings-appearance-capture` writes the pane, modes, Glass, both
colour popups, the Advanced sheet, and the reduce-transparency/
increase-contrast states in light and dark.

### 5.5 Accessibility

Mockup: intro + Vision, Physical and Motor, Hearing, Speech, and General
sections, each a big list of sub-panes. The sub-pane inventory is the mockup's
`accessibility/*` entries.

Dispositions:

| Mockup sub-pane | Disposition |
|---|---|
| Screen Reader | delegate (Orca/AT-SPI); keep status and hand-off |
| Zoom | ship (`accessibility.magnifier`, `magnifierZoom`, `magnifierMode`) |
| Hover Text | omit (no owner) |
| Display | ship the rows with real keys; omit ownerless rows |
| Motion | ship `accessibility.reduceMotion` and `dimFlashingLights`; omit the rest |
| Read & Speak | delegate (speech-dispatcher/Orca) |
| Audio Descriptions | omit (no owner) |
| Voice Control | omit (no owner) |
| Keyboard | ship `input.keyboardNavigation`; delegate/omit the rest |
| Pointer Control | ship the rows with real input keys; omit the rest |
| Switch Control | omit (no owner) |
| Hearing Devices | delegate/omit (no owner) |
| Audio | ship the rows with real sound keys; omit the rest |
| Subtitles and Captioning | omit (no owner) |
| Live Captions | omit (no owner) |
| Spoken Name | omit (no owner) |
| Live Speech | delegate (speech-dispatcher); keep the sample action |
| Custom Voice | omit (no owner) |
| Vocal Shortcuts | omit (no owner) |
| Shortcut | ship (`accessibility.shortcut`, the shared `ShortcutRecorder`) |

The top-level list keeps the mockup's section order. A sub-pane that ends up
with no owner is omitted from the list and recorded in `SettingsMap` with the
reason; the section heading stays if at least one sibling remains. This is the
largest deliberate reduction from the mockup. It is reviewed in T291a–T291e.

T291a makes the list executable. Accessibility is intro text plus a
`SettingsBigList`; each child adds one row and one in-place sub-pane body, never
a disabled placeholder. Vision ships first: the sub-pane keeps **Screen
Reader** (read-only AT-SPI status + `Open Orca`), **Zoom**, **Display**,
**Motion**, and **Read & Speak**. Hover Text, Invert colors, Show window title
icons, Show toolbar button shapes, Display contrast, Menu bar size, Pointer
colors, Color filters, Auto-play animated images, Prefer non-blinking cursor,
and Audio Descriptions are `omitted` map rows with a reason; they are not
rendered and not search-indexed. The AT-SPI absence state keeps the structure:
the live status rows disable with the note while the settingsd rows stay live
([adr/0297](adr/0297-accessibility-big-list-and-vision.md)).

T291b adds **Physical and Motor** after Vision, in mockup order, with the hand
tile. The sub-pane keeps **Keyboard** (Full keyboard access on the stored
`input.keyboardNavigation` key, and the on-screen-keyboard delegate to
`onboard`/`wvkbd`) and **Pointer Control** (Double-click speed on the live
`input.doubleClickSpeed`). Voice Control, Sticky Keys, Slow Keys, Mouse Keys,
Ignore built-in trackpad, and Switch Control have no Linux owner and are
`omitted` map rows with a reason; they are not rendered and not search-indexed.
An absent on-screen-keyboard program keeps the row and disables only `Open`
with the missing-program reason. The capture writes the row, the sub-pane, the
accommodation state, and the absent delegate in six stills.

T291c adds **Hearing** after Physical and Motor, in mockup order, with the
speaker tile. Only **Audio** has a real Linux owner: **Balance** writes the
stored-only `sound.balance` key shared with the Sound pane. Hearing Devices
(no hearing-aid or LE Audio service), Subtitles and Captioning (no caption
delegate), Live Captions (no live-caption engine), and Spoken Name (no
on-device name model) have no owner and are `omitted` map rows with a reason;
they are not rendered and not search-indexed. The mockup's Mono audio, Flash
the screen on alert, and Background sounds sit in the Audio group but have no
key or service here, so they are omissions rather than disabled placeholders.
The absent AT-SPI bridge leaves the Audio group and its settingsd row live.
The capture writes the list and sub-pane in light/dark, the accommodation
state, and the absent-bridge state in six stills.

T291d adds **Speech** after Hearing, in mockup order, with the speaker tile.
The build has no first-party speech service and no voice model, so the
sub-pane keeps one group, **Live Speech**, with one real owner: the
speech-dispatcher / espeak-ng delegate the Vision Read & Speak row already
uses. **Speak sample** speaks through the session engine; when neither program
is installed the row stays visible and disables only its action with the
missing-program reason. Custom Voice (no on-device voice model or cloning
service) and Vocal Shortcuts (no vocal-shortcut trigger engine) have no owner
and are `omitted` map rows with a reason; they are not rendered and not
search-indexed. The section ships alive because Live Speech has a real owner;
it is not removed. The capture writes the list and sub-pane in light/dark and
the absent-engine state in five stills.

T291e adds **General** last, in mockup order, with the gear tile. It ships one
group, **Accessibility Shortcut**, whose **Accessibility shortcut** row reuses
the existing `ShortcutRecorder` (no second editor) and writes the live
`accessibility.shortcut` key; the compositor installs the chord and the
`accessibility-shortcut` action opens this pane. The row is live, not
stored-only: there is no delegate to be absent, so it records with no AT-SPI
host. This closes the pane: the list is Vision, Physical and Motor, Hearing,
Speech, General; every section in the table above is either rendered from its
shipped rows or omitted with a map reason; and no row stays `planned`. The
`tst_settings_accessibility` close-out case walks every mockup section and every
omitted row, and `make t291e-settings-accessibility-general-capture` writes the
complete list, every shipped sub-pane, the recorded shortcut, the reduce-motion
and increase-contrast states, and the absent-host list in light and dark.

### 5.6 Desktop & Dock

| Mockup block | Target |
|---|---|
| Dock | 10 mockup rows; rename Size/Magnification/Position to the mockup labels. The two Linux-only Dock rows (hover chooser, minimize reaction) move to an Advanced Dock sheet. |
| Desktop & Workspaces | Rename Stage → Workspace Focus; keep the real desktop/workspace keys. |
| Widgets | Show Widgets, Dim widgets, Default web browser (moved here). |
| Windows | Mockup labels; “Hold Super” replaces “Hold Option”; window rules move to an Advanced Windows sheet. |
| Workspace Overview | Merge the MissionControlPane rows here. Five mockup toggles plus the real workspace-count/gesture rows in an Advanced sheet. Footer Shortcuts… and Hot Corners… open the existing sheets. |

Landed in T292b: the five toggles render as the section; the workspace count,
the gestures, and the four corner assignments are in the Advanced Workspace
Overview sheet (Hot Corners…), and Shortcuts… opens Keyboard → Shortcuts.

The top-level Overview pane is removed; `settings://pane/mission-control` and
`settings://pane/overview` land on this section.

T292a landed the main surface: Dock, Desktop & Workspaces (with the Stage →
Workspace Focus rename), Widgets (Default web browser moved here), Windows
(“Hold Super”, the two edge-tile labels), and one **Window Overview** section
whose **Workspace Overview** disclosure opens the existing Mission Control
surface. The footer has **Advanced…** (the Advanced Dock sheet: hover chooser,
minimize reaction), **Advanced…** (the Advanced Windows sheet: application
window rules), **Shortcuts…**, and **Hot Corners…**. The Linux-only rows keep
their owners, and `rowSurface("desktop-dock", …)` answers `dialog` for the
three sheet rows. “Show application menus in the menu bar” is gone from this
pane (Menu Bar owns it).

T292b lands the **Workspace Overview** section itself, replacing the temporary
entry: the mockup description plus the five toggles (Automatically rearrange
Workspaces, Switch to an app's Workspace, Group windows by application,
Displays have separate Workspaces, Drag windows to the top to enter Workspace
Overview). Only Switch to an app's Workspace
(`overview.switchToAppSpace`) is live; rearrange, group-by-application, and
drag-to-Overview are disabled with their reasons, and per-display workspaces is
the read-only compositor fact. The mockup's “Spaces” is rendered as
“Workspaces”, the naming gate's term for virtual desktops. The real workspace count
(`workspaces.count`), the two gesture keys (`gestures.missionControl`,
`gestures.spaceSwitch`), and the four corner assignments plus their modifiers
(`overview.hotCorner*`) move from the deleted `MissionControlPane.qml` into an
**Advanced Workspace Overview** sheet, opened by the footer's **Hot Corners…**
(the sheet takes the measured 560 pt width so each corner row holds both
popups). **Shortcuts…** still hands off to Keyboard → Shortcuts. The map's row
sections are `Workspace Overview` and `Advanced Workspace Overview`, and
`rowSurface` answers `dialog` for the seven sheet rows.
`settings://pane/mission-control` and `settings://pane/overview` land on the
section. `make t292b-settings-workspace-overview-capture` writes the section,
the footer, the advanced sheet, and the absence pair in light/dark.

### 5.7 Menu Bar

| Mockup block | Target |
|---|---|
| Menu Bar Behaviour | Auto-hide, background, recent items. “Show suggestions in the menu bar gallery” is omitted (no owner). |
| OS Menu Glyph | Kept addition. Two radio rows, actual-size previews, Dragonfruit default, Tux option. |
| Menu Bar Controls | Clock + options, Search, Wi-Fi, Bluetooth, Battery + options, Focus + options, Sound + options, Accessibility, Quick Settings, Workspaces. Display is a disabled row with its reason. Now Playing, Text Input, Backups, and Timer are omitted until a status item exists. |
| Allow in the Menu Bar | Live third-party StatusNotifier list; empty state when none. |
| Footer | Add Controls… and Reset Menu Bar…. |

The former Quick Settings pane becomes **Menu Bar Controls** under Menu Bar.
The main pane's “Menu Bar Controls” section lists the status items and offers
“Customize Controls…”. That action opens the in-place **Menu Bar Controls**
sub-pane: pages, spans, modules, preview, Add Controls, Menu Bar Only,
per-display, and Reset. The shell's `quickSettings.*` keys and the Control
Center panel are unchanged.

The Linux-only rows (overflow policy, global app menu, per-display panel) move
to an Advanced Menu Bar sheet. They have real owners; they are not deleted.

T293a lands the restyle. The pane renders **Menu Bar Behaviour**, **OS Menu
Glyph**, **Menu Bar Controls**, and **Allow in the Menu Bar** in that order and
ends with one footer carrying **Add Controls…**, **Customize Controls…**,
**Advanced…**, and **Reset Menu Bar…** (four ghost actions sharing one
derived width, the same approach §5.6 uses). The former standalone "Menu Bar
Behavior" group, the Screen Casting row, the Notifications row, and the Reset
card are gone. The mockup's suggestion row and the four controls with no
status item (Now Playing, Text Input, Backups, Timer) are map `omitted` rows
with reasons; Display is an `omitted` row that still renders the disabled row
with its exact reason, because the user must see why the mockup item is not
there. The OS Menu Glyph is the pane's one radio group: its rows are
`SettingsGlyphOptionRow`, each with an actual-size strip (real
`Theme.controls.menuBar.height`, at least 184 pt wide, real
`Theme.controls.menuBar.iconSize` mark, wallpaper-contrast shadow, hover/open
plate, File Edit View sample), a focus ring, and an AT-SPI radio role; the
group follows `SettingsRadioGroup`'s keys (Space/Return selects the focused
row, the arrows move the selection). Dragonfruit is the install default and
the Reset value. The three Linux-only rows live in the **Advanced Menu Bar**
sheet, and Customize Controls… opens the Menu Bar Controls surface.
`make t293a-settings-menu-bar-capture` writes the main pane, the glyph group,
the controls list, the footer/third-party list, the Reset result, and the
advanced sheet in light/dark.

T293b lands the in-place **Menu Bar Controls** sub-pane. Customize Controls…
and the `control-center`/`quick-settings` deep links open it in place of the
pane list (Pages, Control sizes, Page names, Modules, Menu Bar Only, the live
preview, Add Controls…, Show on all displays, and Reset Quick Settings), with
a **‹ Menu Bar** back button and the toolbar title "Menu Bar Controls". The
body is the former `QuickSettingsPane.qml` (now
`shell/control-center/MenuBarControlsPane.qml`): the same `quickSettings.*`
keys and the same `ControlCenterRegistry`, never a second module list. The
main pane's **Menu Bar Controls** section stays the status-item list; the
sub-pane's rows are map rows under `menu-bar` with `rowSurface` = `subpane`.
`make t293b-settings-menu-bar-controls-capture` writes six stills per scheme.

### 5.8 Search

| Mockup block | Target |
|---|---|
| Show Related Content | omit (no owner) |
| About Search & Privacy… | link to the search privacy document |
| Reset Quick Keys | ship (`search.shortcut`) |
| Search History | omit (no history owner) |
| Share anonymous search diagnostics | omit (no telemetry) |
| Results from Apps | adapt to `search.categories`; the mockup's app list is a sample, not a second per-app registry |
| Results from Files | adapt to `search.indexLocations` |
| Results from System | adapt to the category/info rows |
| Results from Clipboard | keep `search.clipboardHistory` (stored-only); omit the age row until a consumer exists |
| Footer | keep Search Privacy…; Clear Clipboard History only if the clipboard store exposes a clear action |

The current Web Search and Indexing sections are Linux additions with real
owners; keep them in an “Advanced” section or sheet. The Search pane does not
duplicate the Menu Bar pane's Search visibility row.

### 5.9 Wallpaper and Screen Saver

- **Wallpaper**: wall-top preview, Dynamic/Your Photos/Landscape/Cityscape/
  Underwater/Earth/Abstract/Colors collection heads and tiles, selected ring,
  Picture popup, Colors. Keep the real wallpaper provider and the existing
  shuffle/slideshow keys. The footer keeps `Screen Saver…` and
  `Clock Appearance…`.
- **Screen Saver**: keep the top-level pane after Wallpaper. Restyle it with the
  shared components; keep the real clock/start-after/hot-corner controls.

### 5.10 Notifications and Focus

- **Notifications**: intro, Notifications description, Show previews,
  Show Notifications (sleeping/locked/mirroring), Application Notifications
  with the live app list and per-app editor. Remove the inline Focus section;
  Focus is its own pane.
- **Focus**: top-level list of the service's focus modes (Do Not Disturb,
  Reduce Interruptions, custom profiles) with chevrons, Add Focus…, Sync across
  devices (delegate/stored), and Focus status. Each mode opens the mockup's
  detail: Priority Notifications, Allowed People/Apps, Schedule, Filters.
  Map to the notification service's `focus`/`dnd` modes and the
  `focus.schedule.*`/`focus.shortcut` keys. Ownerless filters are omitted.

### 5.11 Sound and Lock Screen

- **Sound**: Sound Effects section (Alert sound, Play sound, Play sound effects
  through, Alert volume, Play sound on startup, Play UI sound effects, Play
  feedback when volume is changed), Output & Input tabs, device rows, volume/
  mute/balance, input level meter. Keep per-app volume only if its consumer
  exists; otherwise move it to an Advanced sheet.
- **Lock Screen**: display-off rows, warning note, require-password, user name/
  photo, password hints, lock message, When Switching User (login window
  style + power buttons), Accessibility Options…. The mockup's separate
  battery/power-adapter display-off values map to the existing idle keys with
  an honest one-row adaptation; do not invent a second idle engine.

### 5.12 Privacy & Security

Mockup: intro + big list of privacy categories + Security section + Advanced….

Map each category to a real portal/permission owner. Keep Location Services
(`privacy.locationServices`), Camera, Microphone, Screen Recording, Files &
Folders, Local Network, Bluetooth, Input Monitoring, and other categories the
portal store actually reports. Omit reference-only or ownerless categories
(Sensitive Content Warning, Personalized Ads, and any category with no portal
owner) with map reasons.

Security maps Software sources → Flatpak/distro, Disk encryption → LUKS,
Accessories → USBGuard, Background security updates → the existing policy if a
consumer exists, Restricted Mode → SELinux/AppArmor/Secure Boot, Advanced… →
the existing Advanced dialog. Analytics stays opt-in and stored-only.

The per-category detail page follows the mockup: description, app rows with
switches, Add/Remove, and the System Services row where the host exposes it.
No fake per-app permission is rendered.

### 5.13 Users & Groups and Online Accounts

- **Users & Groups**: mockup top-level list (User/Admin, Guest User), Add
  Group/Add User, automatic login disabled by LUKS, Network account server.
  Keep the real accountsservice/group provider; move the full user/group lists
  and dialogs to the mockup's detail/sheet surfaces.
- **Online Accounts**: rename, banner, empty state, Add Account…. Keep GOA
  as the delegate and the absent-service state.
- **Biometrics & Password**: removed. Delete the pane, map rows, tests, and
  docs; do not leave a dead sidebar entry or a stale deep link.

### 5.14 Input devices

- **Keyboard**: Keyboard section (repeat rate, delay, low-light brightness,
  brightness, backlight timeout, Super-key action, keyboard navigation,
  shortcuts), Text Input (Input Sources, Text Replacements), Dictation
  (languages, microphone, shortcut, privacy link), Set Up Keyboard…. Keep the
  real keys and the disabled-with-reason rows for hardware that is absent.
- **Mouse**: tracking speed, natural scrolling, secondary click, double-click
  speed, scrolling speed; Advanced… and Set Up Bluetooth Mouse… sheets for the
  extra real rows.
- **Trackpad**: the three mockup tabs and their rows; keep the real gesture
  keys. Extra rows stay in the matching tab or the advanced sheet.
- **Game Controllers**: controller list, Home Button section, Profiles section,
  Set up Game Controller…. The Home Button rows are omitted unless a real
  controller/consumer exists; the SDL mapping delegate stays honest.

### 5.15 Printers & Scanners

Default printer, default paper size, Printers list, empty state, Add Printer,
Scanner, or Fax…. Keep the real CUPS/SANE data; move jobs, supplies, and
printer details to the mockup's sheets.

## 6. Menu Bar and the OS menu glyph

The OS menu glyph is a kept addition that the mockup does not show. It remains
in the Menu Bar pane, after Menu Bar Behaviour and before Menu Bar Controls.

- `menu.osGlyph` stays the key; default `dragonfruit` on install and after
  Reset. Options are `dragonfruit` and `tux`.
- The control is a radio group. Each option is a row with a radio circle, the
  name, a one-line description, and an **actual-size** preview on the right.
- The preview is a strip at the real menu-bar height with the real glyph size,
  the wallpaper-contrast shadow, and the hover/open plate. It is not a scaled-up
  icon tile. A small “File Edit View” sample may sit beside the glyph to show
  the scale, but it must not imply a second menu bar. The strip is wide enough
  (at least 180 pt) that the glyph reads at its true size.
- Clicking anywhere on a row selects it; Space/Return works from the keyboard;
  the row has a focus ring and an AT-SPI radio role.
- The description says Dragonfruit is the default mark and Tux is available for
  a traditional Linux look.

The improved area replaces the current two small preview plates with the
actual-size strip and the clearer radio behavior. The rest of the Menu Bar
pane follows §5.7.

Landed in T293a: the strip is a fixed 184 pt so it reads the same at every
window width and never scales the mark; the sample menus sit inside the strip
at `Theme.controls.menuBar.fontSize`, so the preview documents the real bar
rather than a second one. The strip clips at its right edge under an
unusually narrow pane; it never shrinks the glyph.

## 7. Evidence and acceptance

Each task records:

- the mockup commit and the pane/section id;
- the deterministic capture target and its light/dark stills at 740 × 820
  (plus the section's absence, reduced-motion, increase-contrast, and scale
  states where relevant);
- the exact targeted test commands and results;
- every deviation from the mockup, with the reason and the owner;
- every omitted row, with its `SettingsMap` reason;
- the live `make demo` check, or an `OPEN` row naming the missing
  Wayland/host/seat and the exact rerun command.

The per-task bar is the targeted verify plus the capture artifact (or `OPEN`)
plus the filled-in `## Hand-off`. The full `make gates`/`make verify-fast` is
the release gate in T302b, not a per-task requirement. ADR 0295 records the
session-sizing policy.

T302a adds the manifest, the structural comparison against the mockup's DOM
measurements (window, sidebar, inset, row heights, title alignment, card
radius), and the per-pane human review checklist. T302b runs the full gate and
batches the human sign-off. Pixel-perfect comparison to the browser mockup is
not a gate; the structural and interaction comparison is. Screenshots stay
local under the git-ignored `docs/captures/`.

No task may add an Apple asset, a copied macOS screenshot, a proprietary name,
or a fake control. A checked task is agent-done, not human-signed-off.

## 8. Review flags

These are the decisions made under the “adapt or omit with a map reason” rule.
They are called out here so the category review can accept or change them
before the task is marked done.

| Flag | Decision | Review in |
|---|---|---|
| VPN visibility | Always visible with the mockup empty state; the old optional-profile gate is removed. | T285/T288 |
| Accessibility depth | Keep the sub-panes with a real owner/delegate; omit the reference-only sub-panes with map reasons. | T291a–T291e |
| Privacy categories | Keep the categories with a real portal owner; omit reference-only/ownerless categories with map reasons. | T298a/T298b |
| device management | Omitted; no Linux MDM/profile owner in scope. | T287 |
| Search history/telemetry | Omitted; no search-history or telemetry owner. | T294 |
| Menu Bar controls | Now Playing, Text Input, Backups, and Timer are omitted until a real status item exists. The former Quick Settings surface is the in-place Menu Bar Controls sub-pane (same `quickSettings.*` keys and the one registry). | T293a/T293b |
| Game Controllers Home Button | Omitted unless a real controller-service consumer exists. | T300d |
| Linux-only rows | Appearance icon/cursor/material, Dock hover/minimize, window rules, Menu Bar overflow/global menu, Search web/indexing, and Sound per-app volume stay reachable in advanced sheets. | T290/T292a/T293a/T294/T297a |
| Accent palette and icons | Use the design-system palette and Phosphor/Tabler glyphs, not the mockup's reference hex values or SVG paths. | T286/T290 |
| Glass tint scope | Keep `appearance.transparency` and state its attached-menu scope; a global glass-tint key is a follow-up. | T290 |
| T284 matrix | T302a supersedes the visual matrix; the real-host interaction matrix remains the release sign-off input. T302b batches the human sign-off. | T302a/T302b |

## 9. Task map

| Task | Category | File |
|---|---|---|
| T285 | IA, catalog, deep links, search index | [285-t-24.1a-settings-ia-and-deep-links.md](../tasks/system-settings-2/285-t-24.1a-settings-ia-and-deep-links.md) |
| T286 | Shell chrome, sidebar, toolbar, shared components | [286-t-24.1b-settings-shell-and-components.md](../tasks/system-settings-2/286-t-24.1b-settings-shell-and-components.md) |
| T287 | General | [287-t-24.1c-settings-general.md](../tasks/system-settings-2/287-t-24.1c-settings-general.md) |
| T288 | Wi-Fi, Bluetooth, Network, VPN | [288-t-24.1d-settings-connectivity.md](../tasks/system-settings-2/288-t-24.1d-settings-connectivity.md) |
| T289 | Energy, Displays | [289-t-24.1e-settings-battery-displays.md](../tasks/system-settings-2/289-t-24.1e-settings-battery-displays.md) |
| T290a–T290g | Appearance: build/test seam (a), mode tiles (b), Glass preview/tint (c), Theme card (d), Windows/scroll radios (e), Advanced sheet/footer (f), capture/scale/docs/ADR 0296 (g) | [plan/ROADMAP.md](../tasks/system-settings-2/plan/ROADMAP.md) |
| T291a | Accessibility list and Vision | [291a-accessibility-vision.md](../tasks/system-settings-2/291a-accessibility-vision.md) |
| T291b | Accessibility Physical and Motor | [291b-accessibility-physical-motor.md](../tasks/system-settings-2/291b-accessibility-physical-motor.md) |
| T291c | Accessibility Hearing | [291c-accessibility-hearing.md](../tasks/system-settings-2/291c-accessibility-hearing.md) |
| T291d | Accessibility Speech | [291d-accessibility-speech.md](../tasks/system-settings-2/291d-accessibility-speech.md) |
| T291e | Accessibility General and close-out | [291e-accessibility-general.md](../tasks/system-settings-2/291e-accessibility-general.md) |
| T292a | Desktop & Dock | [292a-desktop-dock.md](../tasks/system-settings-2/292a-desktop-dock.md) |
| T292b | Workspace Overview | [292b-workspace-overview.md](../tasks/system-settings-2/292b-workspace-overview.md) |
| T293a | Menu Bar and OS glyph | [293a-menu-bar.md](../tasks/system-settings-2/293a-menu-bar.md) |
| T293b | Menu Bar Controls | [293b-menu-bar-controls.md](../tasks/system-settings-2/293b-menu-bar-controls.md) |
| T294 | Search | [294-t-24.1j-settings-search.md](../tasks/system-settings-2/294-t-24.1j-settings-search.md) |
| T295a | Wallpaper | [295a-wallpaper.md](../tasks/system-settings-2/295a-wallpaper.md) |
| T295b | Screen Saver | [295b-screen-saver.md](../tasks/system-settings-2/295b-screen-saver.md) |
| T296a | Notifications | [296a-notifications.md](../tasks/system-settings-2/296a-notifications.md) |
| T296b | Focus | [296b-focus.md](../tasks/system-settings-2/296b-focus.md) |
| T297a | Sound | [297a-sound.md](../tasks/system-settings-2/297a-sound.md) |
| T297b | Lock Screen | [297b-lock-screen.md](../tasks/system-settings-2/297b-lock-screen.md) |
| T298a | Privacy | [298a-privacy.md](../tasks/system-settings-2/298a-privacy.md) |
| T298b | Security | [298b-security.md](../tasks/system-settings-2/298b-security.md) |
| T299a | Users & Groups | [299a-users-groups.md](../tasks/system-settings-2/299a-users-groups.md) |
| T299b | Online Accounts | [299b-internet-accounts.md](../tasks/system-settings-2/299b-internet-accounts.md) |
| T299c | Biometrics removal | [299c-biometrics-removal.md](../tasks/system-settings-2/299c-biometrics-removal.md) |
| T300a | Keyboard | [300a-keyboard.md](../tasks/system-settings-2/300a-keyboard.md) |
| T300b | Mouse | [300b-mouse.md](../tasks/system-settings-2/300b-mouse.md) |
| T300c | Trackpad | [300c-trackpad.md](../tasks/system-settings-2/300c-trackpad.md) |
| T300d | Game Controllers | [300d-game-controllers.md](../tasks/system-settings-2/300d-game-controllers.md) |
| T301 | Printers & Scanners | [301-t-24.1q-settings-printers.md](../tasks/system-settings-2/301-t-24.1q-settings-printers.md) |
| T302a | Visual evidence matrix | [302a-visual-evidence-matrix.md](../tasks/system-settings-2/302a-visual-evidence-matrix.md) |
| T302b | Release close-out and human sign-off | [302b-release-close-out.md](../tasks/system-settings-2/302b-release-close-out.md) |
