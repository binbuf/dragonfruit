# Settings → business logic map

| | |
|---|---|
| **Status** | target design; the T222 executable projection is `apps/settings/SettingsMap.qml` |
| **Date** | 2026-11-06 |
| **Scope** | every macOS System Settings pane, sub-pane, and row that Dragonfruit ships, adapts, delegates, or omits |
| **Companions** | [22-total-experience.md](22-total-experience.md), [distro-compatibility.md](distro-compatibility.md), [interface-naming-and-licensing.md](interface-naming-and-licensing.md) |

> **2026-11-14 review.** This map remains the executable owner/absence
> contract. The pane-by-pane target, including the complete macOS Tahoe row
> inventory, the Linux adaptations, the visual/behaviour contract, and the
> findings that the first pass did not close, is
> [24-system-settings.md](24-system-settings.md). A row that this map marks
> shipped but that the review lists as missing, mislabelled, or
> consumer-less is not done; the owning task is in the
> [system-settings roadmap](../tasks/system-settings/plan/ROADMAP.md)
> T263–T284.
>
> **2026-11-16 backend contract (T282).** The schema now carries an explicit
> `stored_only` flag and a semantic rule per key. A row whose key is stored-only
> must say so (the `SettingsMap.row()` `storedOnly` argument); a row whose key
> has a shipped consumer must name one of the process/module roots
> `shell/`, `apps/settings`, `apps/files`, `compositor/`, `session/`,
> `services/wallpaperd`, `dragonfruit-system-status`, or `settingsd`.
> `cargo test -p dragonfruit-settingsd` enforces that split, and
> `tst_settings_schema_mirror` compares the client default/constraint tables
> with `dragonfruit-settingsd --print-schema-json`. Adapter rows use the
> `writeFinished` outcome (`applied`/`accepted`/`denied`/`failed`/`absent`);
> a denial is a row state, never an implicit success. See
> [24-system-settings.md](24-system-settings.md) §3.7 and ADR
> [0283](adr/0283-stored-only-settings-keys.md).

## 1. Purpose

A settings pane is only real when each visible row changes something a user can
observe, through an owner that is allowed to change it. This document is the
row-level contract:

- **What** the macOS pane/row is.
- **Whether** Dragonfruit ships, adapts, delegates, or omits it.
- **Who owns** the state (compositor, shell, `settingsd`, host daemon, distro
  adapter, portal, or a named third-party component).
- **What happens** when the owner is absent.
- **Where** the implementation lives or will live.

The pane catalog is the source of truth for the Settings app's `shipped` flag.
A row may not appear without an owner; an owner may not be added without an
absence path.

## 2. Dispositions

| Disposition | Meaning |
|---|---|
| **Ship** | First-party pane/row; Dragonfruit owns the UI and the state or a host-service adapter |
| **Adapt** | Same intent, different Linux concept or capability |
| **Delegate** | Dragonfruit provides the entry point; a named third-party service/app owns the function |
| **Omit** | Not shipped; the reason and any replacement are recorded |
| **Existing** | Already implemented; this map records the owner and gaps |
| **Partial** | Implemented but a named part is missing |

## 3. Ownership classes

| Class | Owner | Examples |
|---|---|---|
| `settingsd` | Dragonfruit settings daemon | Appearance, Dock, menu bar, workspaces, input prefs, lock policy |
| `compositor` | Rust/Smithay compositor | Outputs, windows, workspaces, input, gestures, hot corners, lock, magnifier, materials |
| `shell` | Qt Quick shell | Menu bar, Dock, Control Center, notifications, OSD, switcher, overview, search UI, drawer, lock UI |
| `host service` | Existing Linux daemon | NetworkManager, BlueZ, PipeWire/WirePlumber, UPower, UDisks2, CUPS, SANE, accountsservice, colord, fprintd, PackageKit, GOA, evolution-data-server |
| `portal` | xdg-desktop-portal | Permissions, file chooser, screenshot, screencast, settings, global shortcuts |
| `distro adapter` | Dragonfruit Rust trait | Package manager, updates, firewall, time sync, locale, users/groups, bootloader |
| `third-party` | Optional external app/service | Orca, ibus/fcitx5, restic/Pika, Tracker/Baloo, GNOME Online Accounts |
| `omit` | Not shipped | iCloud, Siri, Screen Time, Wallet, Game Center, Find My, AirDrop/Continuity |

## 4. Pane catalog

macOS 27 System Settings order, with the Dragonfruit disposition. The Settings
app's sidebar may group panes differently, but the catalog is complete.

| # | macOS pane | Dragonfruit pane | Disposition | Primary owner |
|---|---|---|---|---|
| 1 | Apple Account | — | Omit | Apple service |
| 2 | Wi-Fi | Wi-Fi | Ship | NetworkManager |
| 3 | Bluetooth | Bluetooth | Ship | BlueZ |
| 4 | Network | Network | Ship/Adapt | NetworkManager + firewall adapter |
| 4.1 | — | VPN | Ship/Adapt | NetworkManager VPN (always-visible pane) |
| 5 | Battery | Battery | Ship | UPower + power-profiles-daemon |
| 6 | General | General | Ship | `settingsd` + adapters |
| 6.1 | About | About This System | Ship | host info + distro adapter |
| 6.2 | Software Update | Software Update | Ship | PackageKit/distro adapter |
| 6.3 | Storage | Storage | Ship | UDisks2 + filesystem usage |
| 6.4 | AirDrop & Handoff | — | Omit (optional LocalSend/KDE Connect later) | No permissive drop-in |
| 6.5 | Login Items & Extensions | Login Items & Extensions | Ship/Adapt | XDG autostart + systemd user units + portals |
| 6.6 | Language & Region | Language & Region | Ship/Adapt | `localectl`/locale + `settingsd` formats |
| 6.7 | Date & Time | Date & Time | Ship/Adapt | `timedatectl`/chrony + `settingsd` formats |
| 6.8 | Sharing | Sharing | Adapt/Delegate | Samba/NFS/portal screen share |
| 6.9 | Time Machine | Backup | Adapt/Delegate | restic/Pika Backup/Timeshift |
| 6.10 | Transfer or Reset | Reset | Adapt | `settingsd` reset; no factory reset |
| 6.11 | Startup Disk | Startup Disk | Adapt (read-only) | UDisks2/bootloader adapter |
| 6.12 | Device Management | — | Omit | MDM is Apple-specific |
| 6.13 | AutoFill & Passwords | Passwords & Autofill | Adapt/Delegate | Secret Service/password manager |
| 7 | Appearance | Appearance | Ship | `settingsd` + compositor |
| 8 | Accessibility | Accessibility | Ship/Delegate | `settingsd` + compositor + Orca/speech-dispatcher |
| 9 | Control Center | Menu Bar → Menu Bar Controls | Moved under Menu Bar | `settingsd` + shell + adapters |
| 10 | Siri & Spotlight | Search | Adapt | Search service + `settingsd`; Siri omitted |
| 11 | Privacy & Security | Privacy & Security | Ship/Adapt | Portals + polkit + LUKS + firewall adapter |
| 12 | Desktop & Dock | Desktop & Dock | Ship | `settingsd` → compositor/shell |
| 12.1 | Mission Control | Desktop & Dock → Workspace Overview | Moved under Desktop & Dock | `settingsd` + compositor |
| 13 | Displays | Displays | Ship/Adapt | Compositor + colord/display adapter |
| 14 | Wallpaper | Wallpaper | Ship | `settingsd` + `wallpaperd` + compositor |
| 15 | Screen Saver | Screen Saver | Ship | Shell/compositor + `settingsd` |
| 16 | Notifications | Notifications | Ship | Notification service + `settingsd` |
| 17 | Sound | Sound | Ship | PipeWire/WirePlumber + `settingsd` |
| 18 | Focus | Focus | Ship | Notification service + `settingsd` |
| 19 | Screen Time | — | Omit | Apple-specific |
| 20 | Lock Screen | Lock Screen | Ship | `settingsd` + compositor/lock + logind |
| 21 | Touch ID & Password | — | Removed (no fingerprint feature) | — |
| 22 | Users & Groups | Users & Groups | Ship/Adapt | accountsservice + distro adapter + polkit |
| 23 | Internet Accounts | Online Accounts (`online-accounts`) | Adapt/Delegate; title keeps the generic name, `internet-accounts` is an alias | GNOME Online Accounts if present; else absent state |
| 24 | Game Center | — | Omit | Apple service |
| 25 | Wallet & Apple Pay | — | Omit | Apple service |
| 26 | Printers & Scanners | Printers & Scanners | Ship | CUPS + SANE |
| 27 | Keyboard | Keyboard | Ship/Adapt | `settingsd` + compositor input + ibus/fcitx5 |
| 28 | Mouse | Mouse | Ship | `settingsd` + compositor input |
| 29 | Trackpad | Trackpad | Ship/Adapt | `settingsd` + compositor input |
| 30 | Game Controllers | Game Controllers | Adapt/Delegate | udev/SDL; optional |

### 4.1 Executable projection (T222)

`apps/settings/SettingsMap.qml` is the runtime projection of this catalog and
the row tables below. It owns each pane's id, label, icon, shipped flag, owner,
deep link (`settings://pane/<id>`), and absence kind, plus row-level
`{ id, section, label, control, owner, key, absence, state }` metadata. The
Settings debug view reads it directly; T223's search index reads the same rows.

- `apps/settings/SettingsPanes.qml` is the one ordered sidebar catalog. Its
  entries mirror `SettingsMap.panes` one-for-one, and
  `apps/settings/tests/tst_settings_map.qml` fails on any drift.
- A row `state` is `shipped` (visible in the current pane), `planned` (lands
  with T225–T229), or `omitted` (deliberately absent). Every `shipped` row must
  have a non-empty owner, key/API, and one of the five absence kinds; the test
  feeds the validator an unowned row to prove it fails.
- T285 reduces the top-level catalog to the 26 panes in the 27-entry sidebar
  (the update status row is the 27th entry). `storage` stays a hidden General
  sub-pane; `mission-control` and `control-center` moved under Desktop & Dock
  and Menu Bar. `SettingsPanes.aliases`/`SettingsMap.aliases` are the alias
  table; `SettingsPanes.removed`/`SettingsMap.removed` carry the reason the
  three ids left the catalog (Biometrics is refused). New panes use the map
  names; user-facing titles are `Passwords & Keys`, `Backups`,
  `Workspace Overview`, and `Menu Bar Controls`. The macOS label *Internet
  Accounts* stays the generic **Online Accounts** title because the naming gate
  maps it; `internet-accounts` is the deep-link alias and search synonym.
- Omitted panes are not in the catalog and therefore cannot appear in the
  sidebar. Their executable ids and reasons live in `SettingsMap.omittedPanes`;
  the test asserts each is absent and documented.
- Help → Pane & Row Owners… opens the in-app debug view; Help →
  Business-Logic Map opens this document. `DF_SETTINGS_DEBUG=1` opens the
  debug view on launch for scripted checks.
- T223's `apps/settings/SettingsSearch.qml` indexes the shipped pane titles,
  the section names, and the `shipped` rows above, plus synonyms and help text
  (owner, key/API, absence kind, pane absence note). It never indexes an
  unshipped pane or a `planned`/`omitted` row. Results deep-link as
  `settings://pane/<id>` or `settings://pane/<id>/row/<rowId>`; the shell
  resolves a row by `SettingsRow.settingsId` first and label/synonym second,
  then scrolls/highlights it. A row that is custom pane chrome lands on its
  section group instead. A General row whose surface is closed opens the
  in-place sub-pane first (`SettingsMap.subPaneForRow`); a Software Update row
  opens its dialog. A row the resolver cannot open is refused and is not
  indexed (`SettingsMap.isRowLandable`). Each pane heading below carries an
  `<a id="pane-<id>">` anchor for the bottom-right `?` help control and the
  Help menu.

| Executable omitted id | Reference pane | Why |
|---|---|---|
| `vendor-account` | Apple Account | Vendor account service; no Linux equivalent |
| `nearby-transfer` | AirDrop & Handoff | No permissive drop-in; optional LocalSend/KDE Connect later |
| `managed-devices` | Device Management | Mobile-device management is vendor-specific |
| `usage-limits` | Screen Time | Vendor parental controls; not shipped |
| `game-service` | Game Center | Vendor game service; no equivalent |
| `wallet` | Wallet & Apple Pay | Vendor payment service; not shipped |
| `device-location` | Find My | Vendor device-location service; not shipped |

## 5. Row-level mapping

The tables below use these columns:

- **macOS row** — the reference label.
- **Dragonfruit row** — the shipped label (generic if different).
- **Control** — toggle, popup, slider, segmented, disclosure, table, button.
- **Owner / key** — the state owner and the proposed key/API.
- **Behaviour / absence** — what the row does and what happens when the owner is
  missing.

Proposed keys are namespaced and must be added to `settingsd` with the
additive-only rule. Existing keys are shown as `existing: …`.

<a id="pane-wifi"></a>
### 5.1 Wi-Fi

Owner: NetworkManager (`org.freedesktop.NetworkManager`). Absence: every
section stays; the host controls disable with a reason and the settingsd
prompt rows remain live (T271, ADR 0271).

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Wi-Fi | Wi-Fi | Toggle | NM `WirelessEnabled` | Enables/disables the radio; disabled when NM absent |
| Current network | Current network | Info + Details… | NM `ActiveConnection` | SSID, signal, security, IP; the sheet adds last-used, auto-join, metered, IPv6, DNS, proxy, MAC randomization, 802.1X, Forget, Disconnect |
| Personal Hotspot | Personal Hotspot | Toggle + Create… | NM `802-11-wireless.mode=ap` | Adds and activates an AP-mode connection; Turn Off deactivates it; disabled with a reason when no Wi-Fi device exists. T288: lives in the Advanced sheet, not the main flow |
| Known Network | Known networks | List + ⋯ | NM `Settings.Connection` | Auto-join and Details in the row; connect/disconnect/forget in the sheet; empty state |
| Other Networks | Other networks | List + Scan | NM `AccessPoint` / `AccessPoint.RequestScan` | Connect, hidden network, real scan; scanning/empty/error rows; `SettingsScanRow` is the progress state |
| Other… | Join hidden network… | Footer dialog | NM | SSID/security; T288 moves it to the mockup's Other… footer |
| Ask to join networks | Ask to join networks | Popup | `network.wifi.askToJoin` | **Stored-only**: the shell has no prompt consumer yet; NM autoconnect remains authoritative |
| Ask to join hotspots | Ask to join hotspots | Popup | `network.wifi.askToJoinHotspots` | **Stored-only**, same reason |
| Advanced… | Advanced… | Sheet | NM connection settings | Live read of TCP/IP, metered, IPv6, DNS, proxies, MAC randomization, 802.1X, hardware |
| — | Quick sheet | Panel | Shell | Same state; Settings… deep link |

<a id="pane-bluetooth"></a>
### 5.2 Bluetooth

Owner: BlueZ (`org.bluez`). Absence: every section stays; power/discovery
disable and no write is attempted (T271, ADR 0271).

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Bluetooth | Bluetooth | Toggle | BlueZ `Adapter1.Powered` | Radio on/off; turning on starts discovery while the pane is open |
| Discoverable as… | Discoverable note | Info | BlueZ `Adapter1.Discoverable` | The computer name; shown only while the pane is open, the radio is on, and the adapter is discoverable |
| My Devices | My Devices | List + info | BlueZ `Device1` | `SettingsDeviceRow`; connect/disconnect and an info sheet with address/type/battery/forget |
| Nearby Devices | Nearby Devices | List | BlueZ discovery | `SettingsScanRow` searching state, then empty/error |
| — | Advanced… | Sheet | BlueZ + `settingsd` | Adapter name, address, discoverable, pairable |
| — | Quick sheet | Panel | Shell | Same state; Settings… deep link |

<a id="pane-network"></a>
### 5.3 Network

Owner: NetworkManager + firewall adapter. Absence: per-service rows show
`Unavailable`; no write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Wi-Fi | Wi-Fi | Service row + disclosure | NM | `SettingsServiceRow`; connected/off status dot; opens the Wi-Fi pane |
| Firewall | Firewall | Service row + switch | firewall adapter | On/off/options where the backend reports status; `firewalld`/`ufw` unprivileged, `nftables`/`iptables` read-only/delegate; independent of NM |
| Other Services | Other services | Service rows | NM devices | `SettingsServiceRow`; Ethernet, bridge, bond, VLAN; status dot, Details disclosure, ⋯ actions |
| VPN | VPN | Section / always-visible pane | NM VPN | Native WireGuard is a connection type; plugin Add only with the editor; incomplete profiles say Needs setup; the top-level VPN pane is always visible (T288) |
| … | Service actions | Menu | NM | Connect/Disconnect, service order, make inactive, delete, add |
| Advanced… | Details… | Sheet | NM | TCP/IP, subnet, router, DNS, search domains, IPv6, proxy, hardware, metered, MAC randomization, 802.1X |
| Locations | Profiles | Popup | NM | Saved connection profiles; adapt macOS Locations |
| — | Firewall options | Sheet | firewall adapter | Open ports, services, zones/rules, backend |
| — | Add service… | Sheet | NM | Ethernet, Wi-Fi, bridge, bond, VLAN with validation; VPN is added from the VPN section |
| — | More / Set Service Order… | Popup + sheet | NM `autoconnect-priority` | T288: the mockup's More popup; the sheet writes real service priority |

**T225 status.** The Wi-Fi and Network panes are shipped. Wi-Fi writes the
radio, joins (visible/hidden), forgets, autoconnects, scans, disconnects, and
starts/stops the personal hotspot; the two `network.wifi.*` prompt keys are
stored-only until the shell grows a prompt consumer. **T271 status.** Wi-Fi and
Bluetooth complete the reference row set: current-network and device sheets,
scanning/empty/error states, known-network disconnect, the AP-mode hotspot,
BlueZ device info/battery/forget, power-on discovery, the computer name, and
structural absence. **T272 status.** Network is the service list: status dots,
per-service Details sheets, the actions menu (connect/disconnect, order, make
inactive, delete, add), and the Add Service sheet for Ethernet/Wi-Fi/VPN/
bridge/bond/VLAN. The firewall adapter detects firewalld/ufw/nftables; nftables
is Unavailable, never a fake Off, and the firewall stays usable when NM is
absent or the VPN adapter is read-only. Every Network/VPN
write reports applied/denied/failed/absent. `Profiles` remains planned (a future
location/profile editor). **T272a status.** Native WireGuard is created as a
`wireguard` connection (private key, address, one peer, optional DNS/MTU) and is
never a VPN plugin. A plugin Add requires `nm-connection-editor`; the created
profile stays `Needs setup` and cannot connect until the editor completes it.
Firewall reads are unprivileged where the host exposes them: ufw falls back to
`systemctl show` + `/etc/ufw/ufw.conf`, nftables to `/etc/nftables.conf`, and
iptables is named read-only/delegate with `sudo iptables -S`. Only a host with
no manager says `No firewall detected`. Writes keep `pkexec` and report a
denial. **T288 status.** The four connectivity panes follow
[`25-system-settings-visual-parity.md`](25-system-settings-visual-parity.md)
§5.2: Wi-Fi is the mockup intro + toggle + current/Known/Other icon rows with
`SettingsScanRow`, the Other…/Advanced… footers, and the stored-only Ask to
Join rows (Personal Hotspot moved into the Advanced sheet); Bluetooth is the
intro + toggle + conditional discoverable note + My Devices/Nearby Devices;
Network is one `SettingsServiceRow` card (Wi-Fi, Firewall), Other Services,
and the More popup with a real Set Service Order sheet; and the VPN pane is
always visible with its intro, "No VPN configurations" empty state, and Add
VPN Configuration… footer. The optional-profile gate is gone. The VPN fixture
grew an `absent` mode so the capture can show the structural absence without a
second process.

<a id="pane-battery"></a>
### 5.4 Battery

Owner: UPower + power-profiles-daemon/platform profile. Absence: note; no
profile write; read-only level if available.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Low Power Mode | Low Power Mode | Popup | `power.profile` | `power-saver`/`balanced`/`performance`; PPD or platform_profile |
| Battery Health | Battery Health | Info + Show Detail | UPower | Capacity/health/cycles; `Unknown` if absent |
| Charging | Battery status | Pill | UPower | Real charge state plus the time-to-full/time-to-empty estimate (the old Charging row is folded in) |
| Last 24 Hours / Last 10 Days | History range | Segmented | `settingsd` + sampler | `power.historyRange`; real session points; explicit empty state with Refresh |
| Last charged to… | Last charged | Info | UPower | `Not available`; UPower exposes no timestamp |
| Battery Level | Battery level | Chart | `services/power` sampler | `SettingsChart` bars over the real sampler points; no fake data |
| Screen On Usage | Screen-on usage | Info | session sampler | One row naming the missing session half; no second empty chart |
| Options… | Options… | Sheet | `settingsd` | The sheet holds Show percentage (menu bar), dim on battery (shell display), prevent sleep (session idle), the disabled wake-for-network/video rows, the Power Adapter values, and the Power Settings/Lock Screen link |
| — | Power adapter | Info | UPower | Live source; charge limit says not reported; hidden when UPower is absent |
| — | Power settings | Rows | `settingsd` | Sleep/display-off timers link to Lock Screen; live without a battery |

**T273 status.** The sampler lives in `services/power` (bounded, one point per
read, ten days at one minute) and `system-status` publishes it as `history`.
The pane draws it, or shows an explicit empty state with **Refresh history**
until two points exist. `power.showPercentage` now changes the menu-bar label,
`power.dimOnBattery` lowers the forwarded brightness, and
`power.preventSleep` disables the session idle engine's suspend stage. The
Options group stays live when UPower is absent.

**T289 status.** The pane follows the mockup order and uses the shared
`SettingsChart`. The range control is the `power.historyRange` key, the status
pill reads UPower's live charge state, and the Options… footer opens a sheet
that owns every `power.*` policy row, the Power Adapter values, and the Lock
Screen link. The rows keep their `SettingsMap` entries; `tst_settings_battery`
walks the rendered rows (sheet included) and fails on a row with no map row.

<a id="pane-general"></a>
### 5.5 General

Owner: `settingsd` + host/distro adapters. Each sub-pane has its own owner and
absence path; the executable rows are in `SettingsMap.qml`.

#### 5.5.1 About

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Device illustration | Device illustration | Art | Design system | Original artwork |
| Device name | Device name | Text (read-only) | hostname/distro adapter | `host.hostname` |
| Chip / Memory / macOS | Processor / Memory / GPU / Disk / OS | Info | host info | `host.info`; serial is hidden when the DMI read is not permitted |
| More Info… | More Info… | Button | host info | Opens the full host read: hostname, model, OS, kernel, architecture, processor, memory, GPU, disk, serial |
| Regulatory Certification | Regulatory info | Link | distro/legal | Delegated; no legal page ships |

#### 5.5.2 Software Update

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Automatic updates | Automatic updates | Toggle | `settingsd` → update provider | `updates.auto` / `updates.autoInstall`; both default off and are never silently enabled |
| Check for Updates | Check for updates | Button | native/PackageKit provider | `dnf`/`apt-get`/`pacman`/`pkcon` check; progress phase; absent provider disables only the update controls |
| Update Now | Install updates | Button | native/PackageKit provider | One explicit install; progress phase and restart state |
| Advanced… | Automatic update policy | Sheet | `settingsd` → update provider | The two policy toggles; the provider owns the restart phase |

#### 5.5.3 Storage

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Storage bar chart | Storage chart | Chart | UDisks2 + filesystem usage | `storage.usage` from `df`; stacked categories plus the free remainder, legend, total/used/free in GiB |
| Categories | Categories | List | filesystem usage | Per-category byte totals and a share bar; never double-counting one device |
| Manage… | Show Detail | Button | Files/Storage | Live delegate to the file manager; formatting and partitioning stay in the disk utility |
| — | Volumes | List | UDisks2 | Mount/unmount with capacity; absent UDisks2 keeps the group and its reason |
| — | Removable media | List | UDisks2 | Eject; absent → note |
| Recommendations | Recommendations | List | filesystem usage | Derived from live byte totals; empty Trash and review categories |

#### 5.5.4 Login Items & Extensions

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Open at Login | Open at login | List + toggle + remove | XDG autostart + `settingsd` | `~/.config/autostart`, `/etc/xdg/autostart`; a system entry is disabled with a user override, never edited in place; a missing command is a broken-item warning (T270) |
| — | Add login item… | Picker | XDG autostart adapter | Installed `.desktop` applications not already at login; writes the user autostart entry (T270) |
| Allow in Background | Allow in background | List + toggle | systemd user units | `systemctl --user enable/disable`; static/masked units are read-only |
| Extensions | Extensions | Grouped list | portal backends | Grouped by the backend that serves the surface; delegated, no fake toggles (T270) |

#### 5.5.5 Language & Region

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Preferred languages | Languages | Ordered list + add/remove | `locale.languages` → `localectl` | The first entry is applied with `localectl set-locale LANG=…`; the order is a durable settingsd key; a session restart may be required (T270) |
| Region | Region | Searchable picker | locale adapter | Choosing a region sets the locale the desktop formats with (T270) |
| Calendar | Calendar | Popup | `settingsd` | `locale.calendar`; Gregorian/ISO/Buddhist/Japanese |
| Temperature | Temperature | Popup | `settingsd` | `locale.temperature`; °C/°F |
| Measurement system | Measurement | Popup | `settingsd` | `locale.measurement`; metric/imperial |
| Date format | Date format | Popup | `settingsd` | `locale.dateFormat`; short/medium/long |
| Number format | Number format | Popup | `settingsd` | `locale.numberFormat`; locale/decimal/comma |
| First day of week | First day of week | Popup | `settingsd` | `locale.firstDay`; locale/Sunday/Monday/Saturday |

#### 5.5.6 Date & Time

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Set date and time automatically | Set automatically | Toggle | `timedatectl`/chrony adapter | `timedatectl set-ntp`; absent `timedatectl` shows the absence note and keeps the clock rows live |
| Time zone | Time zone | Searchable picker | `timedatectl` | `timedatectl set-timezone` from `list-timezones`; searchable by substring (T270) |
| Set time zone automatically | Set time zone automatically | Toggle | location service adapter | Inert with its reason: no location service is installed (T270) |
| Date and time | Date and time | Fields | `timedatectl` | Prefilled, validated `YYYY-MM-DD HH:MM:SS`; requires auth; disabled while NTP is on; busy/denied/error is shown (T270) |
| 24-hour time | 24-hour time | Toggle | `settingsd` | `clock.use24Hour`; the menu-bar clock consumes it |
| Show seconds | Show seconds | Toggle | `settingsd` | `menu.clock.showSeconds` |
| Show date | Show date | Toggle | `settingsd` | `menu.clock.showDate` |
| Clock options… | Clock options… | Sheet | `settingsd` | `menu.clock.*` |

#### 5.5.7 Sharing

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| — | Local hostname | Text + Edit | hostname adapter | `hostnamectl set-hostname`; inline DNS-label validation; the reason names a missing `hostnamectl` (T270) |
| File Sharing | File sharing | Toggle + Options | Samba/NFS adapter | `smb.service`/`smbd.service`; installed services only; the Options sheet reports the unit and share count |
| Screen Sharing | Screen sharing | Toggle + Options | portal ScreenCast/RDP | `gnome-remote-desktop.service`/`wayvnc.service`; installed services only |
| Media Sharing | Media sharing | Toggle + Options | DLNA adapter | `rygel.service`/`minidlna.service`; installed services only |
| Printer Sharing | Printer sharing | Toggle + Options | CUPS | `cups.service`; the Options sheet reports the queue count |
| Remote Login | Remote login | Toggle + Options | OpenSSH adapter | `sshd.service`; admin auth; the Options sheet reports the configured port |
| Remote Management | Remote management | Toggle + Options | distro adapter | `cockpit.socket`; installed services only |
| Internet Sharing | Internet sharing | Toggle | NetworkManager | Hidden unless a shared connection exists |

The row description carries the service description only; on/off is the
toggle and authentication is the Options sheet, never a duplicated suffix
(T270). The Services group keeps its title and absence note when systemd is
absent.

#### 5.5.8 Backups (Time Machine)

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Back Up Automatically | Back up automatically | Toggle | `settingsd` → backup backend | `backup.auto`; off by default; disabled when no backend is installed (T270) |
| — | Backup backend | Popup | backup adapter | restic/Pika/Timeshift detection; automatic selects the installed one |
| Select Disk… | Destination | Text + picker | `settingsd` → backup backend | `backup.destination`; a portal folder picker and an absolute-path/URL validator gate Save; the backend's detected destination is the hint (T270) |
| Options… | Schedule | Popup | `settingsd` → backup backend | `backup.schedule`; off/hourly/daily/weekly/monthly; disabled when no backend is installed (T270) |
| Back Up Now | Back up now | Button | backup adapter | One explicit run; disabled when the backend cannot run non-interactively |
| — | Restore… | Button | backup adapter | Delegated restore entry (Timeshift GUI/Pika); absent backend disables it |

The absent note names the missing system status host, not a missing backend;
the policy rows stay live (T270).

#### 5.5.9 Startup Disk / Reset

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Startup Disk | Startup disk | Info + list + Restart | UDisks2/bootloader adapter | Read-only; the boot disk first, every disk with its size in GiB; Restart is one confirmed logind request; no bootloader rewrite (T270) |
| Transfer or Reset | Transfer | Delegates | distro tools | No migration assistant ships; the transfer assistant row is disabled with its reason (T270/T287) |
| Transfer or Reset | Reset | Destructive button | `settingsd` | `org.dragonfruit.Settings1.Reset`; typed `RESET` confirmation; resets Dragonfruit settings only, never personal files or host configuration (T270) |
| Erase All Content… | Erase All Content and Settings | Info | — | Omitted with a Linux reason: erasing every file and account is an operating-system reinstall (T270) |
| Device Management | — | Omit | — | MDM is Apple-specific |

#### 5.5.10 Passwords & Keys

Owner: Secret Service + the user's password manager/browser. Absence: the row
shows the available provider or a note; it does not store secrets itself.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| AutoFill & Passwords | Passwords & autofill | Disclosure | Secret Service/portal | Opens the installed password manager or a read-only provider list |
| Passwords… | Passwords… | Button | Secret Service | Delegated to GNOME Secrets/KeePassXC or the browser |
| AutoFill from… | Autofill source | Popup | `defaults.passwordManager` | Provider selection; hidden if none |
| — | Locked keyring | Info | Secret Service | Status/unlock entry |

**T226 status.** The General group ships with real host/distro adapters:
`timedatectl`/`localectl`, XDG autostart, systemd user/system units, the
restic/Pika/Timeshift detection, `df` usage, and `findmnt` startup-disk reads.
The native/PackageKit update provider is attached to the bridge host; the
Settings pane and the Control Center tile read the same `Updates` view. Each
sub-pane carries its own state and absence note; Storage was still a
first-class sidebar pane at T226 and the General row navigated to it. The
About `More Info…` button is live; the regulatory link is the one delegated
omission.

**T269 status.** The General list is complete and never collapses. About,
Software Update, Storage, Passwords & Keys, Date & Time, Language & Region,
Login Items & Extensions, Sharing, Startup Disk, Backups, and Transfer or
Reset are always rows; each summary says `Unavailable` only for its own absent
half. Every row opens an in-place sub-pane, including Software Update (no
dialog) and Storage (the `storage` id and deep link remain aliases).
About shows model, processor, memory, graphics, serial, OS/kernel/
architecture, the live screen name/resolution, and volume/free/total, with a
real System Report sheet. Passwords & Keys inventories installed providers and
the Secret Service lock state; `defaults.passwordManager` records the
delegate, and Settings stores no secret. Transfer or Reset is the last row;
its content is T270.

**T287 status.** The rendered General list is the mockup intro plus 11
`SettingsBigRow`s with category tiles and the Passwords & Keys/Backups sub
lines; Device Management is a `state: "omitted"` map row with the reason “no
Linux MDM/profile owner in scope”. The ownerless mockup rows are omissions,
not disabled controls: Energy Compliance, Beta Updates, AutoFill Passwords and
Keys, Delete After Use, Text in Images, Applications, Translation Languages,
Content Caching, Bluetooth Sharing, and Remote Application Scripting. Sharing
renders its Content & Media, Accessories & Internet, and Advanced sections
from the live service list; a section with no installed service is not drawn.

<a id="pane-appearance"></a>
### 5.6 Appearance

Owner: `settingsd` + compositor/design system. Absence: live on schema
defaults; no error banner.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Light/Dark/Auto | Appearance | Tiles | existing: `appearance.colorScheme` | Live theme; auto follows host style |
| Accent color | Accent color | Swatches + custom hex | existing: `appearance.accent` | Live; Multicolor (empty) or any `#rrggbb` |
| Highlight color | Text highlight color | Swatch popup | `appearance.highlight` | Selection wash; live; design-system palette only |
| Icon & widget style | Icon & widget style | Tiles | `appearance.iconStyle` | Provider-gated; disabled with the reason when no installed theme matches |
| Folder color | Folder color | Swatch popup | `appearance.folderColor` | Files/Theme folder glyph; Automatic keeps the artwork; design-system palette only |
| Sidebar icon size | Sidebar icon size | Popup | `appearance.sidebarIconSize` | Small/medium/large |
| Allow wallpaper tinting | Tint window background with wallpaper color | Toggle | `appearance.wallpaperTint` | Materials adapt to wallpaper |
| Liquid Glass | Glass tint | Slider | existing: `appearance.transparency` | 0..1; attached menus only; measured 0.5255 midpoint; disabled under Reduce Transparency |
| Show scroll bars | Show scroll bars | Stacked radio | `appearance.scrollBars` | Automatic/when-scrolling/always, visually distinct |
| Click in the scroll bar to | Click in the scroll bar to | Stacked radio | `appearance.scrollBarClick` | Jump to spot or next page |
| — | Icon theme | Popup (Advanced Appearance) | `appearance.iconTheme` | Installed-theme provider; explicit theme wins over the style |
| — | Cursor theme / size | Popup + slider (Advanced Appearance) | `appearance.cursorTheme`, `appearance.cursorSize` | Provider-gated; applied at the next session start |
| — | Window control layout | Popup (Advanced Appearance) | `appearance.windowControls` | Disabled with reason; traffic lights stay left |
| — | Material effects | Popup (Advanced Appearance) | `appearance.material` | Disabled with reason; Glass budget owns the tier |
| — | Accent from wallpaper | Toggle (Advanced Appearance) | `appearance.accentFromWallpaper` | Disabled with reason; needs the wallpaper sampler |

The landed pane is six cards plus a footer: **Appearance** (mode tiles),
**Glass** (the rendered attached-menu preview and the tint slider), **Theme**
(accent, Text highlight, icon style, Folder color), **Windows** (sidebar icon
size, wallpaper tint), **Show scroll bars**, and **Click in the scroll bar
to**, followed by **Advanced Appearance…** and **More in Accessibility
Settings…**. The six Linux-only rows live in the Advanced Appearance sheet.
The Motion link is gone: `accessibility.reduceMotion` keeps its Accessibility
owner and `Theme.reducedMotion` binding.

**T290 status.** The pane is six cards plus the Advanced Appearance sheet; the
T274 layout had a separate Preview group and a Desktop integration card. The
Glass preview is the preview and the only tint slider is the attached-menu
`appearance.transparency`. Icon style
(`appearance.iconStyle`) and the installed icon theme (`appearance.iconTheme`)
are separate controls: the style is a pane tile, the theme is an Advanced
Appearance row. Highlight and Folder color are single swatch popups over the
design-system palette; only the accent accepts a custom hex. Folder color is
consumed by the design-system `Icon` folder glyph in Files and the Dock;
scroll-bar click is consumed by `ScrollView`; icon/cursor rows use the real
installed-theme inventory. Transparency is disabled and its write discarded
under Reduce Transparency.

**T227 status.** The pane ships every row. Highlight, sidebar icon size,
wallpaper tinting, and scroll bars are settingsd keys consumed live by
`Theme.highlightOverride`/`selection`, the design-system `Sidebar`,
`Glass.wallpaperTint`, and `ScrollView`. Motion is a deep link to the existing
Accessibility toggle, not a second key.

<a id="pane-accessibility"></a>
### 5.7 Accessibility

Owner: `settingsd` + compositor + host accessibility services. Delegated
functions name the external component. Absence: the five-section structure
stays; only the live status rows and the delegates that need a program
disable with a reason.

T275 status: the pane renders every reference row as a settingsd control, a
read-only status, a delegate action, a stored preference, or an explicit
omission. The screen reader is a status plus `Open Orca`, never a toggle
(`org.a11y.Status` is read-only, ADR 0144). The compositor magnifier and the
recorded shortcut are live over the v12 `set_accessibility_policy` request.

T291a status: the long scroll is replaced by the mockup intro and a big list of
in-place sub-panes ([adr/0297](../adr/0297-accessibility-big-list-and-vision.md)).
Vision ships first; Physical and Motor, Hearing, Speech, and General follow in
T291b–T291e. The Vision sub-pane groups its rows under **Screen Reader**,
**Zoom**, **Display**, **Motion**, and **Read & Speak**. A Vision row with no
real owner is a map `omitted` row with a reason and is neither rendered nor
search-indexed; it is not a disabled control.

T291b status: Physical and Motor is the second big-list row, in mockup order.
Its sub-pane has **Keyboard** and **Pointer Control**. The shipped rows are
Full keyboard access (stored-only `input.keyboardNavigation`; no compositor
consumer), On-screen keyboard (the `onboard`/`wvkbd` delegate; `Open` disables
when neither program is installed), and Double-click speed (live
`input.doubleClickSpeed`, read by the compositor pointer policy). Voice
Control, Sticky Keys, Slow Keys, Mouse Keys, Ignore built-in trackpad, and
Switch Control are `omitted` map rows with reasons; they are not rendered and
not search-indexed.

T291c status: Hearing is the third big-list row, in mockup order. Its sub-pane
has one rendered group, **Audio**, with Balance on the stored-only
`sound.balance` key shared with the Sound pane. **Hearing Devices** has no
hearing-aid or LE Audio service, **Subtitles and Captioning** has no caption
delegate, **Live Captions** has no live-caption engine, and **Spoken Name** has
no on-device name model; each is represented only by `omitted` map rows with a
reason. Mono audio, Flash the screen on alert, and Background sounds are
Audio-group omissions for the same reason. The absent AT-SPI bridge leaves the
settingsd row live.

#### 5.7.1 Vision

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| VoiceOver | Screen reader | Status + Open | Orca (AT-SPI) `org.a11y.Status.ScreenReaderEnabled` | Read-only live status; `Open Orca` launches the external reader or disables with a reason |
| — | Accessibility bridge | Status | AT-SPI `org.a11y.Status.IsEnabled` | Read-only live status |
| Hover Text | Hover text | — | — | Omitted: no hover-text overlay |
| — Invert colors | Invert colors | — | — | Omitted: no whole-scene inversion |
| — Increase contrast | Increase contrast | Toggle | `accessibility.increaseContrast` | Live `Glass` |
| — Reduce transparency | Reduce transparency | Toggle | `accessibility.reduceTransparency` | Live `Glass` |
| — Differentiate without color | Differentiate without color | Toggle | `accessibility.differentiateWithoutColor` | Live `Theme` check glyph |
| — Show window title icons | Show window title icons | — | — | Omitted: no title-icon slot |
| — Show toolbar button shapes | Show toolbar button shapes | — | — | Omitted: no shape pass |
| — Display contrast | Display contrast | — | — | Omitted: no contrast pass |
| — Text size | Text size | Popup | `accessibility.textSize` | Live `Theme.textScale` |
| — Menu bar size | Menu bar size | — | — | Omitted: fixed menu-bar height |
| — Pointer size | Pointer size | Slider | `appearance.cursorSize` | Stored-only; applied at the next session start |
| — Pointer colors | Pointer colors | — | — | Omitted: no per-colour cursor override |
| — Color filters | Color filters | — | — | Omitted: no filter pass |
| Zoom | Use keyboard shortcuts to zoom | Toggle | `accessibility.magnifier` | Live compositor magnifier |
| — | Zoom level | Slider | `accessibility.magnifierZoom` | Live (1.0–8.0) |
| — | Follow | Popup | `accessibility.magnifierMode` | Live (`follow-focus`/`follow-caret`) |
| Motion | Reduce motion | Toggle | `accessibility.reduceMotion` | Live Theme + compositor |
| — Dim flashing lights | Dim flashing lights | Toggle | `accessibility.dimFlashingLights` | Live Dock attention amplitude |
| — Auto-play animated images | Auto-play animated images | — | — | Omitted: no animated-image surface |
| — Prefer non-blinking cursor | Prefer non-blinking cursor | — | — | Omitted: toolkit-owned cursor blink |
| Read & Speak | Read & speak | Action | speech-dispatcher/espeak-ng | Delegated; `Speak Sample` disabled when absent |
| Audio Descriptions | Audio descriptions | — | — | Omitted: content-level |

#### 5.7.2 Hearing

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Hearing Devices | Hearing devices | — | — | Omitted: no hearing-aid or LE Audio service |
| — Flash the screen | Flash the screen on alert | — | — | Omitted: no full-output alert flash surface |
| — Play stereo as mono | Mono audio | — | — | Omitted: no per-session downmix |
| — Balance | Balance | Slider | `sound.balance` | Stored-only; audio consumer deferred |
| — Background sounds | Background sounds | — | — | Omitted: no background-sound player |
| Captions | Captions | — | — | Omitted: no live-caption or subtitle delegate |
| RTT | RTT | — | — | Omitted: no real-time text service |
| Name Recognition | Name recognition | — | — | Omitted: no on-device name model |
| Spoken Name | Spoken name | — | — | Omitted: no on-device spoken-name model |

#### 5.7.3 Physical and Motor

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Voice Control | Voice control | — | — | Omitted: no voice-control or speech-to-pointer engine |
| — Full keyboard access | Full keyboard access | Toggle | `input.keyboardNavigation` | Stored-only; no compositor consumer |
| — Sticky Keys | Sticky keys | — | — | Omitted: no sticky-key modifier latch |
| — Slow Keys | Slow keys | — | — | Omitted: no slow-key acceptance delay |
| — Accessibility Keyboard | On-screen keyboard | Action | onboard/wvkbd | Delegated; `Open` disabled when absent |
| — Double-click speed | Double-click speed | Slider | `input.doubleClickSpeed` | Live compositor pointer |
| — Mouse Keys | Mouse keys | — | — | Omitted: no keyboard pointer mode |
| — Ignore built-in trackpad | Ignore built-in trackpad | — | — | Omitted: no per-device ignore |
| Switch Control | Switch control | — | — | Omitted: no switch-control delegate or scanning source |

#### 5.7.4 Speech

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Live Speech | Live speech | Action | speech-dispatcher/espeak-ng | Delegated; `Speak Sample` disabled when absent |
| Personal Voice | Personal voice | — | — | Omitted: no equivalent |
| Vocal Shortcuts | Vocal shortcuts | — | — | Omitted: no equivalent |
| Siri | — | — | — | Omitted |

#### 5.7.5 General

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Shortcut | Accessibility shortcut | Recorder | `accessibility.shortcut` | Live compositor shortcut engine; opens this pane |

<a id="pane-control-center"></a>
### 5.8 Control Center

Owner: `settingsd` + shell `ControlCenterPolicy` + adapters.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Control Center modules | Modules | List | `settingsd` | Show/hide, order |
| Wi-Fi | Wi-Fi | Checkbox | existing: `menu.control.wifi` | Module + menu-bar item |
| Bluetooth | Bluetooth | Checkbox | existing: `menu.control.bluetooth` | Module + menu-bar item |
| AirDrop | — | Omit | — | No permissive equivalent |
| Focus | Focus | Checkbox | `menu.control.focus` | Module + menu-bar item |
| Screen Mirroring | Screen mirroring | Checkbox | `menu.control.screenMirroring` | Module + menu-bar item |
| Now Playing | Now Playing | Checkbox | `menu.control.nowPlaying` | Module |
| Sound | Sound | Checkbox | existing: `menu.control.volume` | Module + menu-bar item |
| Display | Display | Checkbox | `menu.control.display` | Brightness module |
| Battery | Battery | Checkbox | existing: `menu.control.battery` | Module + menu-bar item |
| Dark Mode | Dark mode | Checkbox | `menu.control.darkMode` | Module |
| Stage Manager | Stage Manager | Checkbox | `menu.control.stageManager` | Module |
| Screen Recording | Screen recording | Checkbox | `menu.control.screenRecording` | Module |
| Keyboard Brightness | Keyboard brightness | Checkbox | `menu.control.keyboardBrightness` | Module |
| Hearing | Hearing | Checkbox | `menu.control.hearing` | Module |
| Menu Bar Only | Menu bar items | List | existing: `menu.control.*` | Clock, Search, Wi-Fi, Bluetooth, Battery, Focus, Screen Mirroring, Display, Sound |
| Other Modules | Other modules | List | `settingsd` | Accessibility shortcuts, etc. |
| Add Controls… | Add controls… | Sheet | `settingsd` + registry | Customization |
| — | Pages | List + add/rename/delete/select | `quickSettings.pageCount` / `pageNames` | Live |
| — | Control sizes | Menu | `quickSettings.spans` | Live |
| — | Preview | Panel | registry + `quickSettings.modules` | Live |
| — | Reset Quick Settings | Button | `quickSettings.*` + `menu.control.*` | Live |

**T227 status.** The pane ships the ordered module list and the menu-bar-only
set. `quickSettings.modules` is the live Control Center order (removing an id
hides that module immediately); `quickSettings.menuBarOnly` removes an item
from the panel while its existing `menu.control.*` mark stays in the menu bar.

**T277 status.** The pane consumes `ControlCenterRegistry` directly: the 37
modules are grouped by the same 22 categories as Edit Controls, with a
category filter and a live 4-column preview. Pages have durable names
(`quickSettings.pageNames`), add/rename/delete/select; spans cycle through the
measured set; the Add Controls sheet lists only modules with a real provider
and shows the registry's reason on the ownerless ones. The per-display row is
disabled with the exact one-surface reason. An explicit empty module list is a
real empty panel (only the Edit Controls footer), not the default fallback.
Reset restores the modules, pages, names, spans, per-display policy,
menu-bar-only set, and the `menu.control.*` marks the surface owns.

<a id="pane-menu-bar"></a>
### 5.9 Menu Bar

Owner: `settingsd` → shell/menu bar. Absence: live on defaults; no error.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Automatically hide and show the menu bar | Auto-hide menu bar | Popup | existing: `menu.autoHide` | Always/never/in fullscreen |
| Show menu bar background | Show menu bar background | Toggle | existing: `menu.showBackground` | Veil on/off; the glass task owns the material |
| — | OS menu glyph | Radio + preview | `menu.osGlyph` | Dragonfruit or Tux; previewed at real menu-bar size with the bar's shadow |
| Recent documents, applications, and servers | Recent items | Stepper | existing: `menu.recentItems` | Count 0–50 |
| Add Controls… | Add menu-bar controls… | Sheet | `settingsd` + registry | Add/remove/reorder |
| Clock | Clock | Checkbox + options | existing: `menu.clock.showDate`/`showSeconds` | Date/time/seconds; Calendar popover |
| Spotlight | Search | Checkbox | `menu.control.search` | Show/hide |
| Wi-Fi | Wi-Fi | Checkbox | existing: `menu.control.wifi` | Show/hide |
| Bluetooth | Bluetooth | Checkbox | existing: `menu.control.bluetooth` | Show/hide |
| Battery | Battery | Checkbox + options | existing: `menu.control.battery` + `power.showPercentage` | Show percentage/energy |
| AirDrop | — | Omit | — | No permissive equivalent |
| Focus | Focus | Checkbox + options | existing: `menu.control.focus` + `menu.control.focusAlways` | Show when active, or always |
| Screen Mirroring | Screen Casting | Disabled row | `menu.control.screenMirroring` | No menu-bar item; start casting from Quick Settings |
| Display | Display | Disabled row | `menu.control.display` | No menu-bar item; the Display module lives in Quick Settings |
| Sound | Sound | Checkbox + options | existing: `menu.control.volume` + `menu.control.volumeWhenActive` | Show always, or hide while muted |
| — | Third-party items | List | StatusNotifier registry + `menu.tray.hidden` | Allow/deny each live item |
| — | Overflow policy | Popup | `menu.overflow` | Automatic or always-show overflow (Linux-only) |
| — | Workspace indicator | Toggle | `menu.control.workspaces` | Live active-Space projection (Linux-only) |
| — | Show application menus | Toggle | `menu.global` | Linux-only; first-party local menus when off |
| — | Show on every display | Disabled row | `quickSettings.perDisplay` | One shell surface; per-display needs a compositor overlay per output |
| — | Reset Control Center | Button | `menu.control.*` + options | Restore the menu-bar surface |

**T277 status.** The pane has one Clock row with the visibility mark and the
Clock Options sheet (declared inside the pane, so the T266 dialog adopts the
window overlay). Battery, Focus, and Sound have per-control option sheets
(`power.showPercentage`, `menu.control.focusAlways`,
`menu.control.volumeWhenActive`). Display and Screen Casting are listed
disabled with the exact "no menu-bar item" reason. The third-party list is the
live app-index StatusNotifier view and writes `menu.tray.hidden`, which the
shell's `applyStatusItems` reads. The Linux-only rows are live: `menu.overflow`
maps to `StatusItemRegistry.forceOverflow`, `menu.control.workspaces` maps to
the compositor's active-Space projection, and `menu.global` is the existing
global-menu toggle. Reset restores the behavior keys, clock options,
visibility marks, per-control options, tray allowances, and Linux-only rows.

<a id="pane-search"></a>
### 5.10 Search (Spotlight-equivalent)

Owner: search service + `app-index` + `settingsd`. Siri is omitted.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Siri | — | Omit | — | Apple service |
| Search results | Search results | Checkboxes | `search.categories` | Applications, files, folders, actions, calculations, definitions, web |
| Show Search in menu bar | Show Search in menu bar | Toggle | `menu.control.search` | Menu-bar item |
| Search privacy | Search privacy | Disclosure | `search.indexLocations` | Include/exclude folders |
| Keyboard shortcut | Shortcut | Shortcut recorder | `search.shortcut` | Global shortcut |
| Web search | Web search | Toggle | `search.web` | Uses default browser |
| — | Clipboard history | Toggle | `search.clipboardHistory` | **stored-only**: the shell always observes the clipboard (T281) |
| — | Indexing status | Info | search service | Progress/error/absent |
| — | Rebuild index | Button | search service | Progress |

**T227 status.** The pane ships categories, indexed locations, the shortcut
recorder, web search, and the menu-bar visibility key as durable settingsd
preferences. Indexing status and Rebuild are disabled with the T246 reason
until the search service exists; the pane never draws a fake index.

<a id="pane-privacy"></a>
### 5.11 Privacy & Security

Owner: portals, polkit, LUKS, firewall adapter. Linux has no exact equivalent
for every macOS TCC category; map to the closest portal/permission and state
the difference.

#### 5.10.1 Privacy

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Location Services | Location | Disclosure | portal Location/Geoclue | Per-app permission |
| Contacts | Contacts | Disclosure | portal/EDS | Per-app permission |
| Calendars | Calendars | Disclosure | portal/EDS | Per-app permission |
| Reminders | Reminders | Disclosure | portal/EDS | Per-app permission |
| Photos | Photos | Disclosure | portal/files | Per-app permission |
| Camera | Camera | Disclosure | portal | Per-app permission |
| Microphone | Microphone | Disclosure | portal | Per-app permission |
| Speech Recognition | Speech recognition | Disclosure | portal | Per-app permission |
| Accessibility | Accessibility | Disclosure | AT-SPI/portal | Per-app permission |
| Full Disk Access | Full disk access | Disclosure | portal/polkit | Best-effort; document limits |
| Files and Folders | Files and folders | Disclosure | portal FileChooser/document portal | Per-app permission |
| Screen & System Audio Recording | Screen recording | Disclosure | portal ScreenCast | Per-app permission |
| Automation | Automation | Disclosure | portal/D-Bus policy | Best-effort |
| App Management | App management | Disclosure | package manager/portal | Best-effort |
| Developer Tools | Developer tools | Disclosure | portal | Per-app permission |
| App Intents | — | Omit | — | Apple-specific |

#### 5.10.2 Security

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Allow applications from | Software sources | Popup | distro adapter + Flatpak | Distro repositories read-only; Flatpak remotes writable |
| FileVault | Disk encryption | Info | LUKS adapter | Read-only status; unlock/recovery delegated |
| Lockdown Mode | Hardening mode | Info | SELinux/AppArmor + Secure Boot | Read-only status; no fake toggle |
| Extensions | Extensions | List | portal extensions | Read-only inventory |
| Advanced… | Advanced… | Sheet | adapters | Firewall, network, etc. |

**T228 status.** The pane renders the fifteen macOS privacy rows mapped to the
closest portal permission, with the Linux difference on every row and the
remaining real portal tables under **Other portal permissions**. Per-app state
is live and writable for the mapped tables; delegated rows name their Linux
owner and never draw a TCC-style toggle. Security ships Software sources
(Flatpak remotes writable, distribution repositories read-only), read-only
Disk encryption (LUKS), read-only Hardening (SELinux/AppArmor + Secure Boot),
installed portal Extensions (read-only), and the Advanced sheet. The proposed
Hardening *toggle* and the Extensions *enable/disable* were deliberately
demoted to read-only status: there is no desktop consumer for a hardening
preference and portal backend selection is system policy, so a toggle would be
a dead affordance.

<a id="pane-desktop-dock"></a>
### 5.12 Desktop & Dock

Owner: `settingsd` → compositor/shell. Absence: live on defaults; no error.
The existing top-level Overview pane (`mission-control`) keeps its id and owns
the workspace, gesture, and hot-corner rows. T276 completes the macOS page and
adds the default-browser delegate.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Size | Size | Slider | existing: `dock.size` | Live |
| Magnification | Magnification | Slider | existing: `dock.magnification` | Live |
| Position on screen | Position on screen | Popup | existing: `dock.position` | Bottom/left/right |
| Minimized window animation | Minimized window animation | Popup | existing: `dock.minimizedAnimation` | Genie/scale/none |
| Double-click title bar | Window title bar double-click action | Popup | existing: `dock.titlebarDoubleClick` | Zoom/minimize/none |
| Minimize windows into app icon | Minimize windows into application icon | Toggle | existing: `dock.minimizeIntoTileIcon` | Live |
| Automatically hide/show Dock | Automatically hide and show the Dock | Toggle | existing: `dock.autohide` | Live |
| Animate opening applications | Animate opening applications | Toggle | existing: `dock.animateOpening` | Live |
| Show indicators for open apps | Show indicators for open applications | Toggle | existing: `dock.showIndicators` | Live |
| Show suggested and recent apps | Show suggested and recent apps in Dock | Toggle | existing: `dock.showRecentApps` | Live |
| — | Open a window chooser by hovering | Toggle | existing: `dock.chooserOnHover` | Live |
| — | Bounce app icons when windows minimize | Toggle | existing: `dock.minimizeReaction` | Live |
| — | Show application menus in menu bar | Toggle | existing: `menu.global` | Live |
| Show items | Show items | Segmented | `desktop.showItems` | Live: Files desktop consumes it |
| Click wallpaper to show desktop | Click wallpaper to reveal desktop | Popup | `desktop.clickWallpaper` | Disabled: desktop reveal is T245b |
| Stage Manager | Stage | Toggle | `stageManager.enabled` | Disabled: Stage is T242 |
| Show recent apps in Stage Manager | Recent apps in Stage | Toggle | `stageManager.showRecentApps` | Disabled: Stage is T242 |
| Show windows from an application | Show windows from an application | Popup | `stageManager.windowMode` | Disabled: Stage is T242 |
| Widgets | Show Widgets | Segmented | `widgets.show` | Disabled: desktop widgets are T245b |
| Dim widgets | Dim widgets on desktop | Popup | `widgets.dim` | Disabled: desktop widgets are T245b |
| Prefer tabs | Prefer tabs when opening documents | Popup | `windows.preferTabs` | Disabled: no first-party tabbed documents |
| Ask to keep changes | Ask to keep changes when closing documents | Toggle | `windows.askToKeepChanges` | Disabled: no first-party document app |
| Close windows on quit | Close windows when quitting an application | Toggle | `windows.closeOnQuit` | Disabled: app lifecycle is T243 |
| Drag to tile | Drag windows to screen edges to tile | Toggle | `tiling.dragToTile` | Disabled: tiling is T242 |
| Drag to fill | Drag windows to the menu bar to fill the screen | Toggle | `tiling.dragToFill` | Disabled: tiling is T242 |
| Hold modifier to tile | Hold Option while dragging to tile | Toggle | `tiling.modifier` | Disabled: tiling is T242 |
| Tiled windows have margins | Tiled windows have margins | Toggle | `tiling.margins` | Disabled: tiling is T242 |
| — | Application window rules… | Button | `windows.rules` | Disabled: the compositor window-rule engine is not present (T281) |
| Mission Control | Overview | Disclosure | `overview.*` | Opens the Overview pane |
| Hot Corners… | Hot Corners… | Disclosure | `overview.hotCorner*` | Opens the Overview pane |
| Shortcuts… | Shortcuts… | Sheet | `shortcuts.*` | Disabled: shortcut editor not implemented |
| Default web browser | Default web browser | Popup | `defaults.browser` + default-apps delegate | Live: app-index `WebBrowser` records, applied by `xdg-settings`; the Settings portal exposes the key read-only; T258 owns the full matrix |

<a id="pane-overview"></a>
### 5.12.1 Overview (the `mission-control` pane)

Owner: `settingsd` → compositor. Absence: live on defaults; the absence note
names the missing daemon. T276 adds the Mission Control and workspace rows.

| Row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|
| Automatically rearrange workspaces | Toggle | `overview.autoRearrange` | Disabled: automatic reordering is T240 |
| Switch to an application's workspace | Toggle | `overview.switchToAppSpace` | Live: the compositor follows the window's Space on activation |
| Group windows by application | Toggle | `overview.groupByApp` | Disabled: grouping is T240 |
| Displays have separate workspaces | Info | `overview.perDisplay` | Read-only: the compositor always keeps one Space list per output |
| Drag windows to the top to open Overview | Toggle | `overview.dragToOpen` | Disabled: drag-to-Overview is T240 |
| Workspace count | Popup | `workspaces.count` | Live: resizes every output's normal Space list in lockstep |
| Swipe up to open | Toggle | `gestures.missionControl` | Live |
| Swipe between Workspaces | Toggle | `gestures.spaceSwitch` | Live |
| Hot Corners | Action + modifier popups | `overview.hotCorner*` / `*Modifier` | Live: Command/Shift/Option/Control gate the dwell |
| Shortcuts… | Sheet | `shortcuts.*` | Disabled: shortcut editor not implemented |

<a id="pane-displays"></a>
### 5.13 Displays

Owner: compositor output API + colord/display adapter. Absence: settingsd
controls live on defaults; output rows show `No display`/`Unavailable`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Built-in display | Display | Icon row + sheet | platform outputs | Real output name and logical `W × H (scale) · Hz`; chevron opens the per-output sheet |
| Display role | Use as | Popup | compositor | Main/extend/mirror; disabled with the one-layout reason (Advanced sheet) |
| Resolution scaling tiles | Resolution | Radios | existing: `display.scale` + compositor | Five real scale steps labelled with their logical size; Show all resolutions filters the list |
| — | Show all resolutions | Toggle | `display.scale` | View filter over the five steps; not persisted |
| Brightness | Brightness | Slider | existing: `display.brightness` | Live; the compositor owns the applied level |
| Automatically adjust brightness | Automatically adjust brightness | Toggle | `display.autoBrightness` | Disabled: no ambient-light sensor input |
| True Tone | Ambient color | Toggle | `display.adaptiveColor` | Disabled: no white-point sensor |
| Preset | Preset | Popup | colord | Disabled: colord not integrated |
| Refresh rate | Refresh rate | Info + popup | compositor | Real current rate; switching disabled |
| When connected to a projector | Projector mode | Popup | compositor | Only when the host marks a TV/projector output; otherwise an absence row with the reason; disabled until a content-type policy exists |
| Night Shift | Night Light… | Sheet | `display.nightLight` | Toggle/schedule/temperature disabled: no gamma LUT |
| Advanced… | Advanced… | Sheet | compositor/colord | Arrangement, Rotation, Use as, Resolution list, HDR, VRR, overscan; no duplicate brightness |
| Arrangement | Arrangement | Canvas | compositor | Read-only cards of the real outputs; Arrange disabled |
| Rotation | Rotation | Popup | existing: `display.rotation` | 0/90/180/270 |
| — | Displays (per-output) | List | platform outputs | The display row's sheet; scopes the mode label and refresh |

**T273 status.** The pane reads the real platform output list, so the title,
preview label, aspect ratio, and refresh are the selected output's own. The
capability table (`libs/settings-client/displaycapabilities.*`) carries an
`area` and is the one source for every disabled row in both the main groups
and the advanced sheet; `shell/tests/tst_displayspolicy.cpp` asserts it. The
sheet no longer duplicates brightness or rotation. Per-output writes remain
the global `display.*` keys until a later backend task adds the key shape.

**T289 status.** The pane follows the mockup order: the display icon row, the
Resolution radios (real logical sizes), the scaling note, Brightness, Ambient
color, Preset, Refresh rate, the conditional projector section, and the
Advanced…/Night Light… footers. The large preview art and the separate
Rotation/Arrangement/Color Profile groups left the main flow for the Advanced…
sheet. The bridge output entry gains `projector`, derived from the
EDID-derived name/model/manufacturer text Qt exposes (fixture seam
`DF_DISPLAY_FIXTURE=projector`), so the projector section is conditional and
honest. Every rendered row has a map row (the pane tests walk the pane and its
three sheets).

<a id="pane-wallpaper"></a>
### 5.14 Wallpaper

Owner: `settingsd` + `wallpaperd` + compositor. Absence: built-in default
always available; Featured empty, never an error.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Current wallpaper | Current wallpaper | Preview | existing: `wallpaper.source` | Optimistic apply; old preview until ready |
| Show on all Spaces | Show on all workspaces | Toggle | existing: `wallpaper.showOnAllSpaces` | Live |
| — | Fit | Popup | existing: `wallpaper.fit` | Fill/fit/stretch/center; live |
| Screen Saver… | Screen saver… | Button | `settingsd` | Deep link |
| Clock Appearance… | Clock appearance… | Button | existing: `screenSaver.clock*` | Real clock controls |
| Dynamic Wallpapers | Dynamic | Grid | `wallpaperd` (Wikimedia Commons) | Featured/offline; Show All |
| Landscape | Landscape | Grid | `wallpaperd` (Wikimedia Commons) | Featured/offline; Show All |
| — | Water | Grid | `wallpaperd` (Wikimedia Commons) | Featured/offline; Show All |
| Cityscape | Cityscapes | Grid | `wallpaperd` (Wikimedia Commons) | Featured/offline; Show All |
| Earth | Earth | Grid | `wallpaperd` (Wikimedia Commons) | Featured/offline; Show All |
| — | Space | Grid | `wallpaperd` (NASA Image and Video Library) | Thumbnail first; full image on demand |
| — | Scenery | Grid | `wallpaperd` (Wikimedia Commons) | Featured/offline; Show All |
| Underwater | Underwater | Grid | `wallpaperd` (Wikimedia Commons) | Featured/offline; Show All |
| Shuffle Aerials | Shuffle | Grid | `wallpaperd` | Random Featured picture |
| — | Slideshow interval | Popup | `wallpaper.slideshow` | **stored-only**: no wallpaper-rotation timer yet (T281) |
| — | Colors | Grid | shipped assets | Original gradients |
| Your Photos | Your photos | Grid | FileChooser portal | Add Photo tile first; current custom photo |
| — | Built-in | Grid | shipped assets | Always available; shipped default |

**T278 status.** The hero is proportional to the content column; file URLs go
through `SettingsBridge::fileUrl` (QUrl); the provider has distinct
absent/offline/error/empty reasons; Colors and Shuffle are shipped rows. A
stationary click anywhere on a tile marks it and starts both spinners in the
same frame; a second pick supersedes; a provider/decode/timeout failure
reverts to the old wallpaper with an inline reason. **Screen Saver…**
navigates; **Clock Appearance…** opens the `screenSaver.clock*` controls.

<a id="pane-screen-saver"></a>
### 5.15 Screen Saver

Owner: shell/compositor + `settingsd`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Screen Saver | Screen saver | Gallery | `settingsd`/shell | Real clock/off previews |
| Start after | Start after | Popup | `idle.blank` | 2 h/5 h/never; None disables |
| Show with clock | Show with clock | Toggle | `screenSaver.clock` | Lock scene |
| Options… | Clock options… | Sheet | `settingsd` | Style-specific; disabled when None |
| Hot Corners… | Hot corners… | Button | existing: `overview.hotCorner*` | Opens Overview |

**T278 status.** The gallery has two real previews of the first-party lock
scene; the clock preview renders the live clock/date/seconds choices. None
disables Start After and the clock options with a reason. Start After offers
2 h/5 h/never and shows a custom idle value's real duration. Hot Corners
opens the actual Overview controls.

<a id="pane-notifications"></a>
### 5.16 Notifications

Owner: notification service + `settingsd`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Show previews | Show previews | Popup | existing: `notifications.showPreviews` | Stored policy |
| When display is sleeping | When display is asleep | Toggle | existing: `notifications.showWhenSleeping` | Stored policy |
| When screen is locked | When locked | Toggle | existing: `notifications.showWhenLocked` | Stored policy |
| When mirroring/sharing | When sharing | Toggle | existing: `notifications.showWhenMirroring` | Stored policy |
| Application Notifications | App notifications | List | notification service | Per-app; absent → note |
| Allow notifications | Allow | Toggle | notification service | Per-app |
| Banners | Style | Popup | notification service | Banner/alert/none |
| Show on lock screen | Lock screen | Toggle | `notifications.appOptions` | Stored per app |
| Notification Center count | Notification Center | Popup | `notifications.appOptions` | Off/1/5/10 |
| Show previews | Previews | Popup | `notifications.appOptions` | Always/when unlocked/never |
| Notification grouping | Grouping | Popup | `notifications.appOptions` | Automatic/by app/off |
| Sounds | Sound | Toggle | notification service | Per-app |
| Badges | Badge | Toggle | notification service | Dock badge |
| Focus | Focus | Link | Focus pane | Deep link; stays when absent |

**T278 status.** The four global presentation preferences are a stored
policy: the notification service has no display-sleep/locked/mirroring
input, so the pane says so instead of claiming they apply live. The
**Application Notifications** list is the service's observed inventory; the
whole row opens one sheet with the four service-owned choices (Allow, Style,
Sound, Badge) and the four extra options (Lock screen, Notification Center
count, Previews, Grouping), stored per app in `notifications.appOptions`.
The status text appears once. The **Focus** link stays available when the
service is absent.

<a id="pane-sound"></a>
### 5.17 Sound

Owner: PipeWire/WirePlumber + `settingsd`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Alert sound | Alert sound | Popup + play | existing: `sound.alertSound` | Original sounds; absent → note |
| Play sound effects through | Play effects through | Popup | existing: `sound.playEffectsThrough` | Device selection |
| Alert volume | Alert volume | Slider | existing: `sound.alertVolume` | Live |
| Play sound on startup | Startup sound | Toggle | existing: `sound.playOnStartup` | Live |
| Play user interface sound effects | UI sounds | Toggle | existing: `sound.uiEffects` | Live |
| Play feedback when volume changed | Volume feedback | Toggle | existing: `sound.volumeFeedback` | Live |
| Output | Output | Tab | PipeWire | Device table, volume, mute |
| Input | Input | Tab | PipeWire | Device table, volume, mute |
| — | Test | Button | PipeWire | Plays a test sound |
| — | Balance | Slider | `sound.balance` | Stored-only; no per-channel write |
| — | Per-app volume | List | PipeWire | Disabled: CLI exposes no stream levels |

**T280 status.** Alert sound has a Play preview
(`Audio.PlayAlertSound`). The Output slider keeps the real level while muted;
a commit unmutes first. The Input tab writes `audio.sourceVolume`/
`audio.sourceMuted` through their own calls. Test still calls
`Audio.PlayTestSound`: `pw-play` (falling back to `paplay`) plays
`audio-test-signal.oga`/`bell.oga`/`Front_Center.wav`. Balance stays stored
until a per-channel sink write lands, and the pane says so. Per-app volume is
disabled with the `pw-dump`/`wpctl` limitation rather than presented as live.

<a id="pane-focus"></a>
### 5.18 Focus

Owner: notification service + `settingsd`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Focus list | Focus list | List | notification service | One real radio list; Off + named + user-added |
| Add Focus | Add focus | Sheet | `settingsd`: `focus.customProfiles` | Validated name; selects custom |
| Delete Focus | Delete focus | Button + confirm | `settingsd`: `focus.customProfiles` | Only user-added Focus |
| Allowed people | Allowed people | List + filter | notification service | Observed `sender` hint; All/Allowed/Not allowed |
| Allowed apps | Allowed apps | List + filter | notification service | Per-app Focus allow list; All/Allowed/Not allowed |
| Time | Schedule | Toggle + time fields + days | `settingsd`: `focus.schedule.*` | Shell `FocusSchedule` applies the mode at the window boundary |
| Shortcuts | Shortcut | Recorder | `settingsd`: `focus.shortcut` | Validated; global registration deferred |

**T278 status.** One Focus list replaces the profile selector, mode
selector, and inert mode rows. Every row is a real radio control; selecting
one installs the service profile (or the custom profile for a user-added
name). Add/Delete Focus write `focus.customProfiles`/`focus.customProfile`.
The allowed apps and people lists carry an All/Allowed/Not-allowed filter.
**Schedule** writes `focus.schedule.enabled/start/end/days`; the shell's
`FocusSchedule` reads them and applies Focus at the window boundary,
respecting a manual override. **Shortcut** writes `focus.shortcut` with a
modifier+key validator; global registration is deferred with the shell
shortcut engine. Schedule and shortcut stay live on schema defaults when the
service is absent.

<a id="pane-lock-screen"></a>
### 5.19 Lock Screen

Owner: `settingsd` + compositor/lock + logind.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Turn display off on battery | Display off (one stage) | Popup | existing: `idle.blank` | One stage, documented |
| Turn display off on power | — | — | — | One-stage Linux adaptation |
| — | Dim screen when inactive | Popup | existing: `idle.dim` | The dim stage; live (T281) |
| — | Suspend when inactive | Popup | existing: `idle.suspend` | Off by default; live (T281) |
| Require password after… | Require password | Popup | `lock.requirePasswordAfter` | After display-off/lock; derives `idle.lock` |
| Show user name and photo | Show user name/photo | Toggle | existing: `lock.showUserNameAndPhoto` | Lock UI |
| Show password hints | Show password hints | Toggle + Set… | existing: `lock.showPasswordHints`/`lock.passwordHint` | Lock UI |
| Show message when locked | Lock message | Toggle + Set… | existing: `lock.showMessageWhenLocked`/`lock.message` | Lock UI |
| Login window shows | Login window | Radio | `login.windowStyle` | Disabled with reason |
| Show Sleep/Restart/Shut Down | Power buttons | Toggle | existing: `lock.showPowerButtons` | Lock UI |
| Accessibility Options… | Accessibility options… | Button | `settingsd` | Lock UI |
| — | Lock screen preview | Preview | shell | Renders the live lock keys |

**T278 status.** Linux has one display-off stage and the pane says so.
`Require password after` is the delay after display-off/lock, not raw
inactivity: the pane writes `lock.requirePasswordAfter` (-1 never, 0
immediately) and derives `idle.lock = idle.blank + after`; a custom idle
value shows its real duration. The preview renders the live identity,
message, hint, and power choices. Cancel in the message/hint editors
restores the stored draft. The shell pushes `lock.*` onto the first-party
lock scene; the compositor owns the fail-secure state.

<a id="pane-biometrics"></a>
**T279 status.** The privacy list is grouped into Location Services, App
access, Analytics & Improvements, and Security; each category carries its own
glyph and is a keyboard/AT-SPI `SettingsRow`. `privacy.locationServices` is
the desktop-level master (the portal has no global switch) and gates the
per-app location writes; `privacy.analytics` is opt-in, defaults off, and is
stored-only. Security adds the LUKS device sheet and the USBGuard accessory
row; a portal that is absent keeps the Security half live, and a security tool
that is absent disables only its row.

### 5.20 Biometrics & Password

Owner: `fprintd` + PAM + distro adapter. Absence: note; no fingerprint write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Change Password… | Change password… | Button | PAM/distro adapter | Auth required |
| Finger 1 / Add Fingerprint | Fingerprints | List + add | fprintd | Enroll/delete; absent → note |
| Use Touch ID to unlock | Use fingerprint to unlock | Toggle | PAM + `settingsd` | Live |
| Use Touch ID for purchases | — | Omit | — | Apple service |
| Use Touch ID for autofill | Use fingerprint for authentication | Toggle | PAM | Live |
| Use Touch ID for fast user switching | — | Omit | — | No display-manager fingerprint switch hook |

**T228 status.** fprintd lists/enrolls/deletes fingerprints over its system-bus
API; AccountsService `SetPassword` (polkit-authorized) changes the password and
the app never stores it. The PAM unlock/authentication rows are read-only
status rows, hidden when `pam_fprintd.so` is not configured. A host with no
reader keeps the password half live; a host with no AccountsService keeps the
fingerprint half live.

<a id="pane-users-groups"></a>
**T279 status.** The Add Fingerprint flow is a two-step wizard that reports
fprintd's own `EnrollStatus` stage id (with the raw id visible) and never a
percentage. A denied write is shown at the row and in the wizard; an
unsupported reader is the honest no-reader state while the password half stays
live. The password dialog waits for the host's report and keeps its message on
a denial.

### 5.21 Users & Groups

Owner: accountsservice + distro adapter + polkit. Absence: read-only list;
no add/remove.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| User list | Users | List | accountsservice | Name, account type, avatar |
| Guest User | Guest user | Toggle | distro adapter | If supported |
| Add User… | Add user… | Sheet | distro adapter | `useradd`/polkit; password |
| Add Group… | Add group… | Sheet | distro adapter | `groupadd`/polkit |
| Automatically log in as | Automatic login | Popup | display-manager adapter | Off/user; auth |
| Network account server | Directory account | Button | SSSD/FreeIPA adapter | Optional |
| — | Admin group | Info | distro adapter | `wheel`/`sudo` mapping |

**T229 status.** The pane lists the `wheel`/`sudo` admin group when the group
provider reports it, and adds a read-only **Directory account** row. The
directory projection reads `/etc/sssd/sssd.conf` (domains, `id_provider`) and
`realm list` when installed; joining or leaving a domain is delegated to the
distribution's `realm`/SSSD tooling. A host with no SSSD/winbind configuration
shows "No directory service is configured" and disables only Details.

<a id="pane-online-accounts"></a>
**T279 status.** Add User carries the password step (name, full name,
password, confirmation, all labelled) and waits for the host's report; the
per-user sheet adds one Change/Reset Password action and drops the duplicate
automatic-login toggle, leaving the one popup. User and group deletion both go
through a confirmation. Groups are served by the real `FileGroupProvider`
(NSS read; `groupadd`/`groupdel`/`gpasswd` under pkexec), so a host without a
provider disables only the group rows.

### 5.22 Online Accounts

Owner: GNOME Online Accounts if present; otherwise omit. Absence: the pane
shows an explicit "Online accounts are not installed" note.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Internet Accounts | Online accounts | List | GOA | Provider list |
| Add Account… | Add account… | Sheet | GOA | OAuth/provider flow |
| Mail/Contacts/Calendars/Notes | Sync toggles | Toggles | GOA | Per-account |
| — | Remove account | Button | GOA | Confirmation |

**T228 status.** The pane is shipped but optional: it appears only while
`org.gnome.OnlineAccounts` owns its session-bus name. Providers come from the
installed `.provider` descriptors, accounts/capabilities from GOA's object
manager, and Add/Remove/sync are explicit GOA writes. No OAuth is
reimplemented. With GOA absent the sidebar entry, search result, and deep link
are all gone — not a dead entry.

<a id="pane-printers"></a>
**T279 status.** The pane stays visible when GOA is absent: it states
"Internet accounts are not installed" and offers a delegate action that names
the distribution package. With GOA present, each account adds a Details sheet
(provider, identity, capabilities), a Sign in again action backed by GOA's
`EnsureCredentials`, and the existing sync toggles/removal confirmation. No
OAuth client is implemented.

### 5.23 Printers & Scanners

Owner: CUPS + SANE. Absence: note; no add/queue write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Default printer | Default printer | Popup | CUPS | Last used/specific |
| Default paper size | Default paper size | Popup | existing: `printers.defaultPaperSize` | Live |
| Printer list | Printers | List | CUPS | Status, queue, options |
| Add Printer, Scanner, or Fax… | Add printer or scanner… | Button | CUPS/SANE | Discovery |
| Options & Supplies… | Options & supplies… | Sheet | CUPS | Driver/status |
| — | Scanners | List + Open Scanner | SANE | Delegated; absent app disables |
| — | Remove printer | Button + confirm | CUPS | `lpadmin -x` |
| — | Print test page | Button | CUPS | `lp -d` |
| — | Share this printer | Toggle | CUPS | `lpadmin -o printer-is-shared` |

**T280 status.** `Add Printer or Scanner…` resets every field, reports the
discovery read, then `lpadmin -p <name> -E -v <uri> -m <driver>` adds the
queue. The per-printer dialog adds Remove (confirmed), Print Test Page
(`lp -d <queue> /usr/share/cups/data/testprint`), Share, and the scanner list's
Open Scanner (`simple-scan`/`xsane`, delegated). **Options & Supplies…** reads
`lpoptions -p <queue> -l` with a loading state and writes one option per
change. A queue with no options is an honest empty sheet; a host with no CUPS
disables the writes. A full job manager and driver installation remain out of
scope.

<a id="pane-keyboard"></a>
### 5.24 Keyboard

Owner: `settingsd` + compositor input + ibus/fcitx5.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Key repeat rate | Key repeat rate | Slider | existing: `input.repeatRate` | Live |
| Delay until repeat | Repeat delay | Slider | existing: `input.repeatDelay` | Live |
| Adjust keyboard brightness in low light | Auto keyboard brightness | Toggle | existing: `input.adjustBrightnessLowLight` | Stored-only; backlight probe |
| Keyboard brightness | Keyboard brightness | Slider | existing: `input.keyboardBrightness` | Stored-only; backlight probe |
| Turn backlight off after inactivity | Backlight off after | Popup | existing: `input.backlightOffAfter` | Stored-only; backlight probe |
| Press ⌘ key to | Press Super key to | Popup | existing: `input.emojiKeyAction` | Stored-only; emoji bridge pending |
| Keyboard navigation | Keyboard navigation | Toggle | existing: `input.keyboardNavigation` | Stored-only; no consumer |
| Keyboard Shortcuts… | Keyboard shortcuts… | Sheet | shortcut engine | Read-only inspector |
| Input Sources | Input sources | Popup + Edit | ibus/fcitx5 + `localectl` | Add/remove/order; absent → note |
| — | Input method | Button | `input.method` | Disabled: IBus/Fcitx5 own the engine (T281) |
| — | Compose key | Select | `input.composeKey` | Disabled: compositor XKB option not exposed (T281) |
| Text Replacements… | Text replacements… | Sheet | `settingsd`/IME | Add/remove |
| Dictation | Dictation | Toggle + options | speech-to-text delegate | Optional; absent → note |

**T280 status.** `Input sources` is the ordered `input.sources` list (first
entry active) with Up/Down/Remove; the shell forwards the first entry to the
compositor keymap over `df_toplevel_manager.set_keyboard_layout` (v10).
`Text replacements` edits the `input.replacements` `from\tto` list.
`Dictation` is enabled only when a speech-to-text command is installed and its
Options sheet writes `input.dictationLanguage`/`Microphone`/`Shortcut`/
`AutoPunctuation`. `Keyboard shortcuts…` is a read-only inspector over the
shell table. The three brightness rows are disabled with a reason when no
keyboard-backlight LED exists and are stored-only until the bridge lands;
keyboard navigation and the emoji-panel action are stored-only too.

<a id="pane-mouse"></a>
### 5.25 Mouse

Owner: `settingsd` + compositor input.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Tracking speed | Pointer speed | Slider | `input.mouse.pointerSpeed` | Mouse-specific; stored-only bridge |
| Natural scrolling | Natural scrolling | Toggle | `input.mouse.naturalScroll` | Mouse-specific; stored-only bridge |
| Secondary click | Secondary click | Popup | `input.mouse.secondaryClick` | Right/left |
| Smart zoom | Smart zoom | Toggle | `input.mouse.smartZoom` | Stored-only; gesture bridge |
| Pointer acceleration | Pointer acceleration | Toggle | `input.mouse.pointerAcceleration` | Stored-only; acceleration bridge |
| Scroll speed | Scroll speed | Slider | `input.mouse.scrollSpeed` | Mouse-specific |
| Double-click speed | Double-click speed | Slider | `input.mouse.doubleClickSpeed` | Mouse-specific |
| — | Scroll method | Popup | `input.mouse.scrollMethod` | Two fingers/edge/button |
| — | Left-handed | Toggle | `input.mouse.leftHanded` | Mouse-specific |
| More gestures | More gestures | Group | `gestures.pages`/`fullscreen`/`notifications`/`appWindows`/`showDesktop` | Stored-only; recognizer family pending |

**T280 status.** A mouse and a trackpad are separate devices with separate
`input.mouse.*` and `input.trackpad.*` keys. The pointer-speed, natural-scroll,
secondary-click, scroll, double-click, and left-handed rows no longer share the
trackpad's keys. The new families are stored-only until the compositor's
per-device/gesture bridge reads them, and each row says so.

<a id="pane-trackpad"></a>
### 5.26 Trackpad

Owner: `settingsd` + compositor input (libinput).

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Tracking speed | Pointer speed | Slider | `input.trackpad.pointerSpeed` | Trackpad-specific |
| Click | Click pressure | Slider | `input.trackpad.clickPressure` | Hardware-dependent |
| Force Click and haptic feedback | Haptic feedback | Toggle | `input.haptics` | Hardware-dependent |
| Look up & data detectors | Look up | Popup | `input.lookUp` | Gesture mapping |
| Secondary click | Secondary click | Popup | `input.trackpad.secondaryClick` | Two-finger/click |
| Tap to click | Tap to click | Toggle | existing: `input.tapToClick` | Live |
| Natural scrolling | Natural scrolling | Toggle | `input.trackpad.naturalScroll` | Trackpad-specific |
| Zoom in or out | Pinch to zoom | Toggle | `input.trackpad.pinchZoom` | Trackpad-specific |
| Smart zoom | Smart zoom | Toggle | `input.trackpad.smartZoom` | Trackpad-specific |
| Rotate | Rotate | Toggle | `input.trackpad.rotate` | Trackpad-specific |
| More Gestures | More gestures | Tab | `gestures.enabled`/`spaceSwitch`/`missionControl` + the five families | Live master/swipe; families stored-only |
| Set Up Bluetooth Trackpad… | Pair trackpad… | Button | BlueZ | If supported |

**T280 status.** The Point & Click tab adds Click pressure; Scroll & Zoom
adds Pinch/Smart zoom/Rotate; More Gestures adds pages, full-screen apps,
Notification Center, app windows, and Show Desktop. Haptics is enabled only
when the inventory reports a touchpad; the preference is kept for the next one.
The preview states the active tab. The Bluetooth hand-off navigates to the
Bluetooth pane and disables when BlueZ is absent.

<a id="pane-game-controllers"></a>
### 5.27 Game Controllers

Owner: udev/SDL; optional. Absence: note; no mapping write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Connected controllers | Controllers | List | udev/SDL | Device info |
| Mapping | Mapping | Sheet + Close | SDL mapping | Optional; read-only |
| — | Test | Dialog + Close | SDL | Fresh report per open |
| — | Rumble test | — | SDL | Disabled with reason |

**T280 status.** The Game Controllers pane is shipped and optional. The input
adapter reads `/proc/bus/input/devices` for joystick/gamepad records and the
session's SDL mapping table, so a host with no controller is a live empty list.
`Test` clears the previous report, reads one event from the `js`/`event` node
with a two-second timeout, and reports event/timeout/permission/absent; both
Test and Mapping have explicit Close actions. `Rumble test` is disabled with
its reason. A remapper and mapping editor are deliberately out of scope.

## 6. Cross-cutting settings keys

The existing `settingsd` schema (revision 18) already covers Dock, workspaces,
gestures, appearance, wallpaper, displays, input, idle, menu, sound, overview,
notifications, lock, and printers. The new panes add namespaced keys; the exact
names are fixed by the task that implements each pane.

Proposed new groups:

| Group | Purpose |
|---|---|
| `network.*` | Wi-Fi prompt policy, VPN, firewall |
| `bluetooth.*` | Adapter name, discoverability, device policy |
| `power.*` | Power profile, battery display, low-battery warning |
| `search.*` | Categories, indexed locations, shortcut, web search |
| `privacy.*` | Portal permission display preferences (the store remains the portal) |
| `users.*` | User/group UI preferences (the store remains accountsservice) |
| `updates.*` | Automatic update policy, channel |
| `defaults.*` | Default browser, mail, media, file manager |
| `backup.*` | Backup backend, schedule, destination |
| `onlineAccounts.*` | Provider enablement (the store remains GOA) |
| `login.*` | Login window style, auto-login |
| `stageManager.*` | Enable, recent apps, stage behaviour |
| `tiling.*` | Tiling/split-view behaviour |
| `hud.*` | HUD placement, timeout, per-kind enable |
| `cursor.*` | Cursor theme, size, hide while typing |
| `keyboard.*` | Input sources, text replacements, dictation |
| `mouse.*` / `trackpad.*` | Pointer/gesture preferences |
| `controllers.*` | Game controller mappings |
| `accessibility.*` | Live accommodations and the recorded shortcut (delegated services keep their own surface) |
| `ime.*` | Input-method framework selection and options |

The rule remains: **one key, one owner, one consumer set**, declared in the
schema table with type, default, range/allowed values, owner, consumer, and
since-revision.

## 7. Absence and degradation

Every row must define one of:

1. **Live on defaults** — settingsd-backed; the daemon may be absent, but the
   control stays interactive on the schema defaults. A write made while it is
   away is in-memory for the session only; when the daemon returns, `GetAll`
   is authoritative and reconciles it away (ADR 0032).
2. **Disabled with explanation** — the owner is absent; the row is visible and
   inert, with a one-line reason.
3. **Hidden** — the row/control does not exist on this host (e.g. no
   fingerprint reader, no battery, no keyboard backlight).
4. **Read-only** — the value is observable but not writable (e.g. LUKS status,
   bootloader).
5. **Delegated** — the row opens the third-party service's own UI; if the
   service is absent, the row shows a note.

The pane catalog's `shipped` flag and the row-level owner are enforced in the
same change. A pane cannot ship with a row that has no owner; a row cannot ship
with an untested absence path. `SettingsMap.validate()` is the executable form
of that rule: the map test runs it over every pane/row and also feeds it an
intentionally unowned row to prove the gate fails.

## 8. Implementation order

1. **Catalog and map** — T222 (this document + the `SettingsPanes` catalog).
2. **IA/search/deep links** — T223.
3. **Live apply, auth, absence** — T224.
4. **Pane groups** — T225–T229.
5. **Cross-distro adapters** — T243–T246.
6. **Acceptance** — T247–T249.

## 9. References

- [Existing settings design](08-settings.md)
- [Existing settings keys](../settings-keys.md)
- [System Settings reference inventory](../reference/System_Preferences.md)
- [Total experience design](22-total-experience.md)
- [Cross-distro contract](distro-compatibility.md)
- [Naming/licensing](interface-naming-and-licensing.md)
- [ADR 0219](adr/0219-total-experience-scope-and-parity-bar.md)
- [ADR 0220](adr/0220-generic-naming-and-third-party-substitution.md)
- [ADR 0221](adr/0221-cross-distro-host-service-contract.md)
- [ADR 0231](adr/0231-executable-settings-map.md)
