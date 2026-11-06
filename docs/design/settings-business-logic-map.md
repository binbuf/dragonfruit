# Settings → business logic map

| | |
|---|---|
| **Status** | target design |
| **Date** | 2026-11-06 |
| **Scope** | every macOS System Settings pane, sub-pane, and row that Dragonfruit ships, adapts, delegates, or omits |
| **Companions** | [22-total-experience.md](22-total-experience.md), [distro-compatibility.md](distro-compatibility.md), [interface-naming-and-licensing.md](interface-naming-and-licensing.md) |

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
| 9 | Control Center | Control Center | Ship | `settingsd` + shell + adapters |
| 10 | Siri & Spotlight | Search | Adapt | Search service + `settingsd`; Siri omitted |
| 11 | Privacy & Security | Privacy & Security | Ship/Adapt | Portals + polkit + LUKS + firewall adapter |
| 12 | Desktop & Dock | Desktop & Dock | Ship | `settingsd` → compositor/shell |
| 13 | Displays | Displays | Ship/Adapt | Compositor + colord/display adapter |
| 14 | Wallpaper | Wallpaper | Ship | `settingsd` + `wallpaperd` + compositor |
| 15 | Screen Saver | Screen Saver | Ship | Shell/compositor + `settingsd` |
| 16 | Notifications | Notifications | Ship | Notification service + `settingsd` |
| 17 | Sound | Sound | Ship | PipeWire/WirePlumber + `settingsd` |
| 18 | Focus | Focus | Ship | Notification service + `settingsd` |
| 19 | Screen Time | — | Omit | Apple-specific |
| 20 | Lock Screen | Lock Screen | Ship | `settingsd` + compositor/lock + logind |
| 21 | Touch ID & Password | Biometrics & Password | Adapt | `fprintd` + PAM + distro adapter |
| 22 | Users & Groups | Users & Groups | Ship/Adapt | accountsservice + distro adapter + polkit |
| 23 | Internet Accounts | Online Accounts | Adapt/Delegate | GNOME Online Accounts if present; else omit |
| 24 | Game Center | — | Omit | Apple service |
| 25 | Wallet & Apple Pay | — | Omit | Apple service |
| 26 | Printers & Scanners | Printers & Scanners | Ship | CUPS + SANE |
| 27 | Keyboard | Keyboard | Ship/Adapt | `settingsd` + compositor input + ibus/fcitx5 |
| 28 | Mouse | Mouse | Ship | `settingsd` + compositor input |
| 29 | Trackpad | Trackpad | Ship/Adapt | `settingsd` + compositor input |
| 30 | Game Controllers | Game Controllers | Adapt/Delegate | udev/SDL; optional |

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

### 5.1 Wi-Fi

Owner: NetworkManager (`org.freedesktop.NetworkManager`). Absence: the pane
shows a one-line note, the toggle is disabled, no write is attempted.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Wi-Fi | Wi-Fi | Toggle | NM `WirelessEnabled` | Enables/disables the radio; disabled when NM absent |
| Current network | Current network | Info + Details… | NM `ActiveConnection` | SSID, signal, security, IP; Details opens connection editor |
| Known Network | Saved networks | List + ⋯ | NM `Settings.Connection` | Auto-join, forget, priority; empty state |
| Other Networks | Available networks | List | NM `AccessPoint` | Join, hidden network; scanning state |
| Other… | Join hidden network… | Dialog | NM | SSID/security |
| Ask to join networks | Ask to join networks | Popup | `network.wifi.askToJoin` | Shell prompt policy; NM autoconnect remains authoritative |
| Ask to join hotspots | Ask to join hotspots | Popup | `network.wifi.askToJoinHotspots` | Prompt policy; may be unavailable |
| Advanced… | Advanced… | Sheet | NM + `settingsd` | TCP/IP, DNS, proxies, MAC randomization, hardware |
| — | Quick sheet | Panel | Shell | Same state; Settings… deep link |

### 5.2 Bluetooth

Owner: BlueZ (`org.bluez`). Absence: note, toggle disabled, no write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Bluetooth | Bluetooth | Toggle | BlueZ `Adapter1.Powered` | Radio on/off |
| Discoverable as… | Discoverable as… | Info | BlueZ `Adapter1.Discoverable` | Only while the pane is open, matching the reference |
| My Devices | Paired devices | List + info | BlueZ `Device1` | Connect/disconnect, forget, info |
| Nearby Devices | Nearby devices | List | BlueZ discovery | Searching/empty/error |
| — | Advanced… | Sheet | BlueZ + `settingsd` | Adapter name, class, pairing policy |
| — | Quick sheet | Panel | Shell | Same state; Settings… deep link |

### 5.3 Network

Owner: NetworkManager + firewall adapter. Absence: per-service rows show
`Unavailable`; no write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Wi-Fi | Wi-Fi | Disclosure | NM | Connected/off; opens Wi-Fi pane |
| Firewall | Firewall | Disclosure | firewall adapter | On/off/options; `firewalld`/`ufw`/`nftables` backend |
| Other Services | Other services | List | NM devices | Ethernet, USB, Thunderbolt, bridge, bond, VLAN |
| VPN | VPN | Disclosure | NM VPN | Add/configure/connect |
| … | Service actions | Menu | NM | Set service order, import/export, make inactive |
| Advanced… | Advanced… | Sheet | NM | TCP/IP, DNS, proxies, hardware |
| Locations | Profiles | Popup | NM | Saved connection profiles; adapt macOS Locations |
| — | Firewall options | Sheet | firewall adapter | Open ports, services, zones/rules |
| — | Add service… | Sheet | NM | Connection editor (limited first pass) |

### 5.4 Battery

Owner: UPower + power-profiles-daemon/platform profile. Absence: note; no
profile write; read-only level if available.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Low Power Mode | Power mode | Popup | `power.profile` | `power-saver`/`balanced`/`performance`; PPD or platform_profile |
| Battery Health | Battery health | Info | UPower | Capacity/health/cycles; `Unknown` if absent |
| Charging | Charging | Info | UPower | State/time-to-full |
| Last 24 Hours / Last 10 Days | Usage range | Segmented | `settingsd` + UPower | Our sampled history; empty state |
| Last charged to… | Last charged | Info | UPower | Date/time |
| Battery Level | Battery level | Chart | UPower | History; no fake data |
| Screen On Usage | Screen-on usage | Chart | `settingsd`/session | Sampled by our service; empty state |
| Options… | Battery options | Sheet | `settingsd` | Show percentage, menu-bar item, low-battery warning |
| — | Power settings | Rows | `settingsd` | Sleep/display-off timers link to Lock Screen |

### 5.5 General

Owner: `settingsd` + host/distro adapters. Each sub-pane has its own owner.

#### 5.5.1 About

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Device illustration | Device illustration | Art | Design system | Original artwork |
| Device name | Device name | Text | hostname/distro adapter | Editable where permitted |
| Chip/Memory/Serial/macOS | Processor/Memory/Serial/OS | Info | host info | Read-only; serial may be hidden |
| More Info… | System Information… | Button | host info | Opens a detailed info window |
| Regulatory Certification | Regulatory info | Link | distro/legal | Original text; optional |

#### 5.5.2 Software Update

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Automatic updates | Automatic updates | Toggle | PackageKit/distro adapter | Backend-specific policy |
| Check for Updates | Check for updates | Button | PackageKit | Progress/result |
| Update Now | Install updates | Button | PackageKit | Progress/reboot |
| Advanced… | Options… | Sheet | distro adapter | Offline updates, reboot policy |

#### 5.5.3 Storage

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Categories | Categories | Chart/list | UDisks2 + filesystem usage | Apps, documents, media, system, other |
| Manage… | Manage storage… | Button | Files/Storage | Opens Files or a cleanup view |
| — | Volumes | List | UDisks2 | Mount/eject/format (delegated where needed) |

#### 5.5.4 Login Items & Extensions

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Open at Login | Start at login | List + add/remove | XDG autostart + `settingsd` | `~/.config/autostart`, `/etc/xdg/autostart`; enable/disable |
| Allow in Background | Background services | List | systemd user units + portals | Show/enable/disable; admin units via polkit |
| Extensions | Extensions | List | portal extensions | Quick Look, file manager, etc. |

#### 5.5.5 Language & Region

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Preferred languages | Languages | List + add | `localectl`/locale adapter | Reorder, apply; session restart may be required |
| Region | Region | Popup | locale adapter | Formats |
| Calendar | Calendar | Popup | `settingsd` | Gregorian/other; locale |
| Temperature | Temperature | Popup | `settingsd` | °C/°F |
| Measurement system | Measurement | Popup | `settingsd` | Metric/imperial |
| Date format | Date format | Popup | `settingsd` | Short/medium/long |
| Number format | Number format | Popup | `settingsd` | Locale grouping/decimal |
| First day of week | First day of week | Popup | `settingsd` | Calendar |

#### 5.5.6 Date & Time

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Set date and time automatically | Set automatically | Toggle | `timedatectl`/chrony adapter | NTP on/off |
| Time zone | Time zone | Popup/map | `timedatectl` | Search/list |
| Date and time | Date and time | Fields | `timedatectl` | Manual set requires auth |
| 24-hour time | 24-hour time | Toggle | `settingsd` | Clock format |
| Show seconds | Show seconds | Toggle | `settingsd` | Clock format |
| Show date | Show date | Toggle | `settingsd` | Menu-bar clock |
| Clock options… | Clock options… | Sheet | `settingsd` | Date/time style, flash separators |

#### 5.5.7 Sharing

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| File Sharing | File sharing | Toggle | Samba/NFS adapter | On/off; shares; auth |
| Screen Sharing | Screen sharing | Toggle | portal ScreenCast/RDP | On/off; permissions |
| Media Sharing | Media sharing | Toggle | DLNA adapter | Optional |
| Printer Sharing | Printer sharing | Toggle | CUPS | Optional |
| Remote Login | Remote login | Toggle | OpenSSH adapter | On/off; admin auth |
| Remote Management | Remote management | Toggle | distro adapter | Optional |
| Internet Sharing | Internet sharing | Toggle | NetworkManager | Optional |

#### 5.5.8 Backup (Time Machine)

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Back Up Automatically | Back up automatically | Toggle | backup adapter | restic/Pika/Timeshift backend |
| Back Up Now | Back up now | Button | backup adapter | Progress/result |
| Select Disk… | Choose destination… | Button | FileChooser portal | Destination selection |
| Options… | Options… | Sheet | backup adapter | Exclusions, schedule, retention |
| — | Restore… | Button | backup adapter | Restore browser (delegated first pass) |

#### 5.5.9 Startup Disk / Reset / Device Management

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Startup Disk | Startup disk | Info | UDisks2/bootloader adapter | Read-only first pass; no bootloader rewrite |
| Transfer or Reset | Reset | Button | `settingsd` | Reset Dragonfruit settings; no factory reset |
| Erase All Content… | — | Omit | — | Not a Linux concept |
| Device Management | — | Omit | — | MDM is Apple-specific |

#### 5.5.10 AutoFill & Passwords

Owner: Secret Service + the user's password manager/browser. Absence: the row
shows the available provider or a note; it does not store secrets itself.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| AutoFill & Passwords | Passwords & autofill | Disclosure | Secret Service/portal | Opens the installed password manager or a read-only provider list |
| Passwords… | Passwords… | Button | Secret Service | Delegated to GNOME Secrets/KeePassXC or the browser |
| AutoFill from… | Autofill source | Popup | `defaults.passwordManager` | Provider selection; hidden if none |
| — | Locked keyring | Info | Secret Service | Status/unlock entry |

### 5.6 Appearance

Owner: `settingsd` + compositor/design system. Absence: live on schema
defaults; no error banner.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Light/Dark/Auto | Appearance | Segmented | existing: `appearance.colorScheme` | Live theme; auto follows host style |
| Accent color | Accent color | Swatches | existing: `appearance.accent` | Live; custom color |
| Highlight color | Highlight color | Popup | `appearance.highlight` | Selection/highlight |
| Sidebar icon size | Sidebar icon size | Popup | `appearance.sidebarIconSize` | Small/medium/large |
| Allow wallpaper tinting | Wallpaper tinting | Toggle | `appearance.wallpaperTint` | Materials adapt to wallpaper |
| Liquid Glass | Transparency | Slider | existing: `appearance.transparency` | 0..1; measured 0.5255 midpoint |
| Show scroll bars | Scroll bars | Popup | `appearance.scrollBars` | Automatic/always/when scrolling |
| — | Motion | Link | `accessibility.reduceMotion` | Deep link to Accessibility |

### 5.7 Accessibility

Owner: `settingsd` + compositor + host accessibility services. Delegated
functions name the external component. Absence: the row shows a note and
disables only the delegated control.

#### 5.7.1 Vision

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| VoiceOver | Screen reader | Toggle + Open | Orca (AT-SPI) | Starts/stops Orca; absent → note |
| Zoom | Magnifier | Toggle + options | compositor `magnifier` | Live zoom, follow focus/pointer; existing |
| Hover Text | Hover text | Toggle + options | shell overlay + `settingsd` | Large text under pointer |
| Display | Display | Disclosure | `settingsd` | Reduce motion, transparency, contrast, differentiate without color, invert, grayscale, cursor size |
| Motion | Motion | Toggle | existing: `accessibility.reduceMotion` | Live |
| Read & Speak | Read & speak | Disclosure | speech-dispatcher/espeak-ng/Piper | Selection/announcements; delegated |
| Audio Descriptions | — | Omit | — | Content-level, not shell |

#### 5.7.2 Hearing

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Hearing Devices | Hearing devices | Disclosure | BlueZ LE Audio/PipeWire | Device list; absent → note |
| Audio | Audio | Disclosure | PipeWire + `settingsd` | Mono audio, balance, flash screen |
| Captions | Captions | Disclosure | live-caption delegate | Optional; absent → note |
| Live Captions | Live captions | Toggle | speech-to-text delegate | Optional |
| Name Recognition | — | Omit | — | Apple-specific |

#### 5.7.3 Motor and General

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Voice Control | Voice control | Toggle | speech-to-text delegate | Optional; absent → note |
| Keyboard | Keyboard | Disclosure | `settingsd` + compositor | Sticky/slow keys, repeat |
| Pointer Control | Pointer control | Disclosure | compositor input | Speed, acceleration, buttons |
| Switch Control | Switch control | Disclosure | delegate | Optional; absent → note |
| Siri | — | Omit | — | Apple service |
| Shortcuts | Accessibility shortcuts | Sheet | `settingsd` | Keyboard shortcuts |

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
| Add Controls… | Add controls… | Sheet | `settingsd` | Customization |

### 5.9 Menu Bar

Owner: `settingsd` → shell/menu bar. Absence: live on defaults; no error.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Automatically hide and show the menu bar | Auto-hide menu bar | Popup | existing: `menu.autoHide` | Always/never/in fullscreen |
| Show menu bar background | Show menu bar background | Toggle | existing: `menu.showBackground` | Veil on/off; the glass task owns the material |
| Recent documents, applications, and servers | Recent items | Stepper | existing: `menu.recentItems` | Count 0–50 |
| Add Controls… | Add menu-bar controls… | Sheet | `settingsd` + registry | Add/remove/reorder |
| Clock | Clock | Checkbox + options | existing: `menu.clock.showDate`/`showSeconds` | Date/time/seconds; Calendar popover |
| Spotlight | Search | Checkbox | `menu.control.search` | Show/hide |
| Wi-Fi | Wi-Fi | Checkbox | existing: `menu.control.wifi` | Show/hide |
| Bluetooth | Bluetooth | Checkbox | existing: `menu.control.bluetooth` | Show/hide |
| Battery | Battery | Checkbox + options | existing: `menu.control.battery` | Show percentage/energy |
| AirDrop | — | Omit | — | No permissive equivalent |
| Focus | Focus | Checkbox + popup | existing: `menu.control.focus` | Show when active/always |
| Screen Mirroring | Screen Casting | Checkbox + popup | `menu.control.screenMirroring` | Show when active/always |
| Display | Display | Checkbox + popup | `menu.control.display` | Show when active/always |
| Sound | Sound | Checkbox + popup | existing: `menu.control.volume` | Show when active/always |
| — | Third-party items | List | StatusNotifier registry | Show/hide; overflow policy |

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
| — | Indexing status | Info | search service | Progress/error/absent |
| — | Rebuild index | Button | search service | Progress |

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
| Allow applications from | Software sources | Popup | distro adapter | Distro/Flathub policy; no fake Gatekeeper |
| FileVault | Disk encryption | Info | LUKS adapter | Read-only status; unlock/recovery delegated |
| Lockdown Mode | Hardening mode | Toggle | `settingsd`/distro adapter | Optional; conservative defaults |
| Extensions | Extensions | List | portal extensions | Enable/disable |
| Advanced… | Advanced… | Sheet | adapters | Firewall, network, etc. |

### 5.12 Desktop & Dock

Owner: `settingsd` → compositor/shell. Absence: live on defaults; no error.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Size | Size | Slider | existing: `dock.size` | Live |
| Magnification | Magnification | Slider | existing: `dock.magnification` | Live |
| Position on screen | Position | Popup | existing: `dock.position` | Bottom/left/right |
| Minimized window animation | Minimize animation | Popup | existing: `dock.minimizedAnimation` | Genie/scale/none |
| Double-click title bar | Title-bar double-click | Popup | existing: `dock.titlebarDoubleClick` | Zoom/minimize/none |
| Minimize windows into app icon | Minimize into app icon | Toggle | existing: `dock.minimizeIntoTileIcon` | Live |
| Automatically hide/show Dock | Auto-hide Dock | Toggle | existing: `dock.autohide` | Live |
| Animate opening applications | Animate opening apps | Toggle | existing: `dock.animateOpening` | Live |
| Show indicators for open apps | Show running indicators | Toggle | existing: `dock.showIndicators` | Live |
| Show suggested and recent apps | Show recent apps | Toggle | existing: `dock.showRecentApps` | Live |
| Show items | Desktop items | Segmented | `desktop.showItems` | On desktop / in Stage Manager |
| Click wallpaper to show desktop | Click wallpaper to reveal desktop | Popup | `desktop.clickWallpaper` | Always / only in Stage Manager / never |
| Stage Manager | Stage Manager | Toggle | `stageManager.enabled` | Live |
| Show recent apps in Stage Manager | Recent apps in Stage Manager | Toggle | `stageManager.showRecentApps` | Live |
| Widgets | Widgets | Disclosure | `widgets.*` | Add/remove/size |
| Mission Control | Mission Control | Disclosure | `overview.*` | Shortcut, gestures, hot corners |
| Hot Corners… | Hot corners… | Sheet | existing: `overview.hotCorner*` | Four corners |
| Shortcuts… | Shortcuts… | Sheet | `settingsd` | Keyboard shortcuts |
| Windows | Windows | Disclosure | `tiling.*` | Tiling/split view |
| Default web browser | Default browser | Popup | `defaults.browser` + portal | Opens default-apps |

### 5.13 Displays

Owner: compositor output API + colord/display adapter. Absence: settingsd
controls live on defaults; output rows show `No display`/`Unavailable`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Built-in display preview | Display preview | Art | compositor | Original illustration |
| Resolution scaling tiles | Scale/resolution | Segmented | existing: `display.scale` + compositor | 1×/1.5×/2×; per-output |
| Brightness | Brightness | Slider | existing: `display.brightness` | Per-output; hardware if available |
| Automatically adjust brightness | Auto-brightness | Toggle | `display.autoBrightness` | Sensor-dependent |
| True Tone | Adaptive color | Toggle | `display.adaptiveColor` | Only if hardware exposes it |
| Preset | Color preset | Popup | colord | ICC profiles |
| Refresh rate | Refresh rate | Popup | compositor | Modes; VRR where exposed |
| When connected to TV | TV mode | Popup | compositor | Ask/mirror/extend |
| Night Shift | Night light | Disclosure | `display.nightLight` | Schedule/temperature |
| Color profile | Color profile | Popup | colord | ICC |
| Advanced… | Advanced… | Sheet | compositor/colord | HDR, VRR, rotation, overscan |
| Arrangement | Arrangement | Canvas | compositor | Drag displays; primary |
| Rotation | Rotation | Popup | existing: `display.rotation` | 0/90/180/270 |

### 5.14 Wallpaper

Owner: `settingsd` + `wallpaperd` + compositor. Absence: built-in default
always available; Featured empty, never an error.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Current wallpaper | Current wallpaper | Preview | existing: `wallpaper.source` | Live |
| Show on all Spaces | Show on all workspaces | Toggle | existing: `wallpaper.showOnAllSpaces` | Live |
| Screen Saver… | Screen saver… | Button | `settingsd` | Deep link |
| Clock Appearance… | Clock appearance… | Button | `settingsd` | Deep link |
| Dynamic Wallpapers | Dynamic | Grid | `wallpaperd` | Our own assets; optional |
| Your Photos | Your photos | Button | FileChooser portal | Add photo |
| Landscape | Landscapes | Grid | `wallpaperd` | Featured/offline |
| Cityscape | Cityscapes | Grid | `wallpaperd` | Featured/offline |
| — | Built-in | Grid | shipped assets | Default; offline |

### 5.15 Screen Saver

Owner: shell/compositor + `settingsd`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Screen Saver | Screen saver | Grid | `settingsd`/shell | Style preview |
| Start after | Start after | Popup | `idle.blank`/`screensaver.idle` | Idle policy |
| Show with clock | Show with clock | Toggle | `screensaver.clock` | Lock screen |
| Options… | Options… | Sheet | `settingsd` | Style-specific |
| Hot Corners… | Hot corners… | Sheet | existing: `overview.hotCorner*` | Same policy |

### 5.16 Notifications

Owner: notification service + `settingsd`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Show previews | Show previews | Popup | existing: `notifications.showPreviews` | Always/when unlocked/never |
| When display is sleeping | When display is asleep | Toggle | existing: `notifications.showWhenSleeping` | Live |
| When screen is locked | When locked | Toggle | existing: `notifications.showWhenLocked` | Live |
| When mirroring/sharing | When sharing | Popup | existing: `notifications.showWhenMirroring` | Live |
| Application Notifications | App notifications | List | notification service | Per-app; absent → note |
| Allow notifications | Allow | Toggle | notification service | Per-app |
| Banners | Style | Popup | notification service | Banner/alert/none |
| Sounds | Sound | Toggle | notification service | Per-app |
| Badges | Badge | Toggle | notification service | Dock badge |
| Focus | Focus | Link | Focus pane | Deep link |

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
| Output | Output | Tab | PipeWire | Device table, volume, balance, mute |
| Input | Input | Tab | PipeWire | Device table, volume, mute |
| — | Test | Button | PipeWire | Plays a test sound |

### 5.18 Focus

Owner: notification service + `settingsd`.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Do Not Disturb | Do Not Disturb | Toggle | notification service | Live |
| Personal/Work/Sleep/Custom | Modes | List | notification service | Add/edit |
| Allowed people | Allowed people | List | notification service | Per-mode |
| Allowed apps | Allowed apps | List | notification service | Per-mode |
| Time | Schedule | Sheet | notification service | Start/end |
| Shortcuts | Shortcut | Recorder | `settingsd` | Global shortcut |

### 5.19 Lock Screen

Owner: `settingsd` + compositor/lock + logind.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Turn display off on battery | Display off on battery | Popup | existing: `idle.blank` | logind/compositor |
| Turn display off on power | Display off on AC | Popup | existing: `idle.blank` | logind/compositor |
| Require password after… | Require password | Popup | `lock.requirePasswordAfter` | PAM |
| Show user name and photo | Show user name/photo | Toggle | existing: `lock.showUserNameAndPhoto` | Lock UI |
| Show password hints | Show password hints | Toggle | existing: `lock.showPasswordHints` | Lock UI |
| Show message when locked | Lock message | Toggle + Set… | existing: `lock.showMessageWhenLocked`/`lock.message` | Lock UI |
| Login window shows | Login window | Radio | `login.windowStyle` | List/name+password |
| Show Sleep/Restart/Shut Down | Power buttons | Toggle | existing: `lock.showPowerButtons` | Lock UI |
| Accessibility Options… | Accessibility options… | Button | `settingsd` | Lock UI |

### 5.20 Biometrics & Password

Owner: `fprintd` + PAM + distro adapter. Absence: note; no fingerprint write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Change Password… | Change password… | Button | PAM/distro adapter | Auth required |
| Finger 1 / Add Fingerprint | Fingerprints | List + add | fprintd | Enroll/delete; absent → note |
| Use Touch ID to unlock | Use fingerprint to unlock | Toggle | PAM + `settingsd` | Live |
| Use Touch ID for purchases | — | Omit | — | Apple service |
| Use Touch ID for autofill | Use fingerprint for authentication | Toggle | PAM | Live |
| Use Touch ID for fast user switching | Fingerprint for user switching | Toggle | PAM/display manager | If supported |

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

### 5.22 Online Accounts

Owner: GNOME Online Accounts if present; otherwise omit. Absence: the pane
shows an explicit "Online accounts are not installed" note.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Internet Accounts | Online accounts | List | GOA | Provider list |
| Add Account… | Add account… | Sheet | GOA | OAuth/provider flow |
| Mail/Contacts/Calendars/Notes | Sync toggles | Toggles | GOA | Per-account |
| — | Remove account | Button | GOA | Confirmation |

### 5.23 Printers & Scanners

Owner: CUPS + SANE. Absence: note; no add/queue write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Default printer | Default printer | Popup | CUPS | Last used/specific |
| Default paper size | Default paper size | Popup | existing: `printers.defaultPaperSize` | Live |
| Printer list | Printers | List | CUPS | Status, queue, options |
| Add Printer, Scanner, or Fax… | Add printer or scanner… | Button | CUPS/SANE | Discovery |
| Options & Supplies… | Options & supplies… | Sheet | CUPS | Driver/status |
| — | Scanners | List | SANE | Scan action (delegated) |

### 5.24 Keyboard

Owner: `settingsd` + compositor input + ibus/fcitx5.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Key repeat rate | Key repeat rate | Slider | existing: `input.repeatRate` | Live |
| Delay until repeat | Repeat delay | Slider | existing: `input.repeatDelay` | Live |
| Adjust keyboard brightness in low light | Auto keyboard brightness | Toggle | existing: `input.adjustBrightnessLowLight` | Sensor-dependent |
| Keyboard brightness | Keyboard brightness | Slider | existing: `input.keyboardBrightness` | Hardware-dependent |
| Turn backlight off after inactivity | Backlight off after | Popup | existing: `input.backlightOffAfter` | Hardware-dependent |
| Press ⌘ key to | Press Super key to | Popup | existing: `input.emojiKeyAction` | Emoji/symbols |
| Keyboard navigation | Keyboard navigation | Toggle | existing: `input.keyboardNavigation` | Live |
| Keyboard Shortcuts… | Keyboard shortcuts… | Sheet | `settingsd` | Shortcut editor |
| Input Sources | Input sources | Popup + Edit | ibus/fcitx5 + `localectl` | Add/remove; absent → note |
| Text Replacements… | Text replacements… | Sheet | `settingsd`/IME | List |
| Dictation | Dictation | Toggle + options | speech-to-text delegate | Optional; absent → note |

### 5.25 Mouse

Owner: `settingsd` + compositor input.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Tracking speed | Pointer speed | Slider | existing: `input.pointerSpeed` | Live |
| Natural scrolling | Natural scrolling | Toggle | existing: `input.naturalScroll` | Live |
| Secondary click | Secondary click | Popup | `input.secondaryClick` | Right/left |
| Scroll speed | Scroll speed | Slider | `input.scrollSpeed` | Live |
| Double-click speed | Double-click speed | Slider | `input.doubleClickSpeed` | Live |
| — | Left-handed | Toggle | existing: `input.leftHanded` | Live |

### 5.26 Trackpad

Owner: `settingsd` + compositor input (libinput).

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Tracking speed | Pointer speed | Slider | existing: `input.pointerSpeed` | Live |
| Click | Click pressure | Slider | `input.tapToClick`/pressure | Hardware-dependent |
| Force Click and haptic feedback | Haptic feedback | Toggle | `input.haptics` | Hardware-dependent |
| Look up & data detectors | Look up | Popup | `input.lookUp` | Gesture mapping |
| Secondary click | Secondary click | Popup | `input.secondaryClick` | Two-finger/click |
| Tap to click | Tap to click | Toggle | existing: `input.tapToClick` | Live |
| Scroll & Zoom | Scroll & zoom | Tab | `input.naturalScroll`/`input.pinchZoom` | Live |
| More Gestures | More gestures | Tab | `gestures.*` | Mission Control/workspaces |
| Set Up Bluetooth Trackpad… | Pair trackpad… | Button | BlueZ | If supported |

### 5.27 Game Controllers

Owner: udev/SDL; optional. Absence: note; no mapping write.

| macOS row | Dragonfruit row | Control | Owner / key | Behaviour / absence |
|---|---|---|---|---|
| Connected controllers | Controllers | List | udev/SDL | Device info |
| Mapping | Mapping | Sheet | SDL mapping | Optional |
| — | Test | Button | SDL | Rumble/input test |

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
| `a11y.*` | Delegated accessibility service enablement and options |
| `ime.*` | Input-method framework selection and options |

The rule remains: **one key, one owner, one consumer set**, declared in the
schema table with type, default, range/allowed values, owner, consumer, and
since-revision.

## 7. Absence and degradation

Every row must define one of:

1. **Live on defaults** — settingsd-backed; the daemon may be absent, but the
   control stays interactive and mirrors when it returns.
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
with an untested absence path.

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
