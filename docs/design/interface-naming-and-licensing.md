# Interface naming, assets, and third-party licensing

| | |
|---|---|
| **Status** | target policy |
| **Date** | 2026-11-06 |
| **Scope** | every user-facing name, icon, sound, font, cursor, wallpaper, and optional third-party dependency |
| **Related** | [22-total-experience.md](22-total-experience.md), [distro-compatibility.md](distro-compatibility.md), [ADR 0220](adr/0220-generic-naming-and-third-party-substitution.md) |

## 1. Principles

1. **Reproduce the interaction, not the identity.** The mental model, layout,
   motion, and quality bar are the target. Apple names, logos, symbols,
   sounds, wallpapers, and code are not.
2. **Generic names in the UI.** Every user-facing string uses a descriptive,
   non-Apple term. Internal code names may remain for continuity, but they must
   not leak into the UI, packaging metadata, or screenshots.
3. **Original or permissively licensed assets.** Every icon, sound, font,
   cursor, wallpaper, and illustration has a recorded source and license.
4. **Third-party services are separate works.** GPL/LGPL services may be used
   as separate processes or dynamically linked libraries; their code is never
   copied into the MIT codebase.
5. **Optional means optional.** The core desktop must work without any
   GPL/optional third-party component; every optional feature has an absence
   path.
6. **One review gate.** A name/asset/dependency change is not complete until
   the gate and `NOTICE` are updated.

> **Reading note.** Older design docs and source identifiers still say
> "Control Center", "Mission Control", "Spaces", "Spotlight", "Launchpad", and
> "Stage Manager". Those are internal/legacy names for continuity. The
> user-facing map below is authoritative for strings, screenshots, packaging,
> and new UI. A string audit may migrate the old names gradually; it must not
> reintroduce an Apple term.

## 2. Naming map

### 2.1 Product and shell

| macOS / Apple term | Dragonfruit user-facing term | Internal/code term | Notes |
|---|---|---|---|
| macOS | Dragonfruit | `dragonfruit` | The DE, not an OS kernel |
| Mac / Macintosh | system / computer | `host` | Avoid "Mac" in UI |
| Apple menu | System menu | `ShellMenus::systemMenu` | Leftmost menu |
| Finder | Files | `apps/files` | First-party file manager |
| Launchpad | Applications | `shell/apps-drawer` | Applications drawer |
| Spotlight | Search | `search` / `app-index` | Search service + UI |
| Mission Control | Overview | `overview` | Window/workspace overview |
| Spaces | Workspaces | `workspaces` | Virtual desktops |
| Stage Manager | Stage | `stageManager` | Window-management mode |
| Dock | Dock | `shell/dock` | Generic desktop term; keep |
| Menu bar | Menu bar | `shell/menubar` | Generic |
| System Settings | Settings | `apps/settings` | First-party Settings app |
| Control Center | Quick Settings | `shell/control-center` | Generic panel name |
| Notification Center | Notifications | `shell/notifications` | History + banners |
| App Switcher | App Switcher | `shell/switcher` | Cmd+Tab equivalent |
| Quick Look | Preview | `quicklook` | Spacebar preview |
| Force Quit | Force Quit | `forcequit` | Generic |
| About This Mac | About This System | `about` | Settings/General/About |
| Activity Monitor | System Monitor | optional | Not first-party core |
| Disk Utility | Disks | delegate | GNOME Disks or future app |
| Console | Logs | optional | — |
| Keychain Access | Passwords | delegate | Secret Service |
| Automator | Shortcuts | optional/delegate | Not core |
| Boot Camp | — | omit | Not a Linux concept |
| Rosetta | — | omit | Not our stack |
| Metal | Vulkan/OpenGL | technical | Not user-facing |
| Aqua | Dragonfruit design language | `design-system` | — |
| Liquid Glass | Glass | `glass` | User-facing "Transparency" |
| SF Symbols | — | Tabler/Phosphor | Icon libraries |
| San Francisco | Inter | `Inter` | Font |
| Retina | HiDPI | technical | — |
| ProMotion | Adaptive Refresh | `display.vrr` | Where supported |
| True Tone | Adaptive Color | `display.adaptiveColor` | Where supported |
| Night Shift | Night Light | `display.nightLight` | Standard Linux term |
| Force Touch / Force Click | Pressure Click | `input.pressureClick` | Where supported |
| AirDrop | Nearby Transfer | optional | Delegate to LocalSend/KDE Connect |
| AirPlay | Screen Casting | `screencast` | Portal/standard term |
| Handoff / Continuity | — | omit | Apple service |
| Universal Control | — | omit | Apple service |
| Sidecar | — | omit | Apple service |
| iCloud | — | omit | Apple service |
| Apple ID | Account | optional | Only if a first-party account model exists |
| App Store | Software Center | `updates`/`software` | Distro/Flathub |
| Siri | — | omit | Apple service |
| Apple Intelligence | — | omit | Apple service |
| Screen Time | — | omit | Apple service |
| Wallet & Apple Pay | — | omit | Apple service |
| Game Center | — | omit | Apple service |
| Find My | — | omit | Apple service |
| Touch ID | Fingerprint | `biometrics` | fprintd |
| FileVault | Disk Encryption | `diskEncryption` | LUKS status |
| Time Machine | Backup | `backup` | restic/Pika/Timeshift |
| Migration Assistant | — | omit | Not a Linux concept |
| Setup Assistant | Welcome | `setup` | First-run |
| Software Update | Updates | `updates` | PackageKit/distro |
| System Information | System Info | `sysinfo` | About/detail |

### 2.2 Menu-bar extras and panels

| macOS term | Dragonfruit term | Notes |
|---|---|---|
| Wi-Fi menu | Wi-Fi | Standard |
| Bluetooth menu | Bluetooth | Standard |
| Battery menu | Battery | Standard |
| Sound menu | Sound | Standard |
| Spotlight menu | Search | Generic |
| Control Center | Quick Settings | Panel |
| Notification Center | Notifications | Panel |
| Date & Time | Date & Time | Generic |
| Focus | Focus | Generic |
| Screen Mirroring | Screen Casting | Standard |
| AirDrop | Nearby Transfer | Optional |
| Stage Manager | Stage | Mode |
| Now Playing | Now Playing | Generic |
| Hearing | Hearing | Accessibility |
| Keyboard Brightness | Keyboard Brightness | Generic |
| Screen Recording | Screen Recording | Generic |

### 2.3 Settings pane names

| macOS pane | Dragonfruit pane | Notes |
|---|---|---|
| Apple Account | — | Omit |
| Wi-Fi | Wi-Fi | — |
| Bluetooth | Bluetooth | — |
| Network | Network | — |
| Battery | Battery | — |
| General | General | — |
| About | About This System | — |
| Software Update | Updates | — |
| Storage | Storage | — |
| AirDrop & Handoff | — | Omit |
| Login Items & Extensions | Login Items & Extensions | Generic |
| Language & Region | Language & Region | Generic |
| Date & Time | Date & Time | Generic |
| Sharing | Sharing | Generic |
| Time Machine | Backup | — |
| Transfer or Reset | Reset | — |
| Startup Disk | Startup Disk | Read-only |
| Device Management | — | Omit |
| Appearance | Appearance | — |
| Accessibility | Accessibility | — |
| Control Center | Quick Settings | Panel settings |
| Siri & Spotlight | Search | Siri omitted |
| Privacy & Security | Privacy & Security | — |
| Desktop & Dock | Desktop & Dock | — |
| Displays | Displays | — |
| Wallpaper | Wallpaper | — |
| Screen Saver | Screen Saver | — |
| Notifications | Notifications | — |
| Sound | Sound | — |
| Focus | Focus | — |
| Screen Time | — | Omit |
| Lock Screen | Lock Screen | — |
| Touch ID & Password | Biometrics & Password | — |
| Users & Groups | Users & Groups | — |
| Internet Accounts | Online Accounts | GOA optional |
| Game Center | — | Omit |
| Wallet & Apple Pay | — | Omit |
| Printers & Scanners | Printers & Scanners | — |
| Keyboard | Keyboard | — |
| Mouse | Mouse | — |
| Trackpad | Trackpad | — |
| Game Controllers | Game Controllers | Optional |

### 2.4 Keyboard terminology

| macOS | Dragonfruit | Notes |
|---|---|---|
| Command (⌘) | Super (⌘/❖) | Display the Linux Super glyph; keep the muscle memory |
| Option (⌥) | Alt (⌥) | — |
| Control (⌃) | Ctrl (⌃) | — |
| Shift (⇧) | Shift (⇧) | — |
| Fn | Fn | — |
| Cmd+Tab | Super+Tab | App switcher |
| Cmd+Space | Super+Space | Search |
| Cmd+` | Super+` | Window cycle |
| Cmd+Ctrl+Q | Super+Ctrl+Q | Lock |
| Cmd+Q | Super+Q | Quit |

The shortcut grammar is Linux-like (Super/Alt/Ctrl) while the physical key
positions and the displayed glyphs preserve the macOS muscle memory where
possible. The keymap is documented in [../keymap.md](../keymap.md).

## 3. Asset policy

### 3.1 Icons

- **Tabler Icons** (MIT) — line glyphs for menus, status items, and controls.
- **Phosphor Icons** (MIT) — filled glyphs for Settings category tiles and app
  tiles.
- **Original app icons** — first-party apps use original artwork; no Apple
  app-icon shapes or colours.
- **Third-party app icons** — loaded from the app's desktop entry/theme; never
  redrawn or recoloured.
- **System symbols** — no SF Symbols. If a symbol has no Tabler/Phosphor
  equivalent, draw an original SVG and record it in the asset manifest.

### 3.2 Fonts

- **Inter** (SIL OFL 1.1) — UI text.
- **A monospace font** — for terminals/code; choose a permissively licensed
  font and record it.
- Do not bundle San Francisco or any Apple font.
- Font fallback must be explicit for CJK, Arabic, Hebrew, Devanagari, and
  emoji; record the fallback packages.

### 3.3 Sounds

- Original or permissively licensed sounds only.
- No Apple sound files or close derivatives.
- Record the source, license, and intended use for every sound.
- The sound set covers: startup, alert, volume feedback, notification, error,
  trash, lock/unlock, and a small UI click/toggle set.
- Respect `sound.uiEffects`, `sound.volumeFeedback`, and the alert volume.

### 3.4 Cursors

- **Capitaine Cursors** (LGPL) — existing choice; record the license and
  attribution.
- Alternative permissive cursor themes may be offered.
- HiDPI sizes (24/32/48/64) and hotspots must be correct.
- No Apple cursor artwork.

### 3.5 Wallpapers and illustrations

- Original shipped default wallpaper.
- Optional provider images carry their own license and attribution (Wikimedia
  Commons, asset-specific).
- No Apple wallpapers, product renders, or marketing images.
- Device illustrations in About/Settings are original line art.

### 3.6 Documentation and screenshots

- Reference screenshots from macOS are local-only and never committed to
  Dragonfruit.
- Dragonfruit screenshots use original assets and generic names.
- If a reference screenshot must be shown in a design doc, use a local link
  that is git-ignored, never a committed image.

## 4. Third-party equivalents

This table is the "buy vs build" record. `Separate process` means the component
runs as its own service/app and is not linked into MIT code. `Dynamic link`
means an LGPL library may be linked with notices. `Optional` means the core
desktop works without it.

| macOS capability | Candidate | License | Integration | Decision |
|---|---|---|---|---|
| File manager | Files (first-party) | MIT | — | Build |
| Launcher | Applications drawer (first-party) | MIT | — | Build |
| Search index | Tracker/Tracker Miners | GPL-2.0+ | Separate process | Optional backend |
| Search index | Baloo | LGPL-2.1+ | Separate process | Optional backend |
| Search index | Recoll | GPL-2.0+ | Separate process | Optional backend |
| Search index | built-in filename index | MIT | — | Default |
| Quick Look | Preview (first-party) + Evince/Okular/mpv | GPL/LGPL | Separate process | Build UI, delegate renderers |
| Backup | restic | BSD-2-Clause | CLI/JSON | Preferred backend |
| Backup | Pika Backup | GPL-3.0+ | Separate app | Optional |
| Backup | Timeshift | GPL-3.0+ | Separate app | Optional |
| Backup | Backrest | MIT | Separate service | Optional |
| Disk encryption | cryptsetup/LUKS | GPL-2.0+ | CLI/D-Bus | Read-only status |
| Fingerprint | fprintd | LGPL-2.1+ | D-Bus | Adapter |
| Nearby transfer | LocalSend | Apache-2.0 | Separate app/protocol | Optional |
| Nearby transfer | KDE Connect | GPL-2.0+ | Separate app | Optional |
| Nearby transfer | Warpinator | GPL-3.0+ | Separate app | Optional |
| Nearby transfer | Syncthing | MPL-2.0 | Separate service | Optional |
| Cloud files | Nextcloud | AGPL-3.0 | Separate app | Omit by default |
| Software center | GNOME Software | GPL-2.0+ | Separate app | Optional |
| Software center | KDE Discover | GPL-2.0+ | Separate app | Optional |
| Package abstraction | PackageKit | GPL-2.0+ | D-Bus | Adapter |
| Flatpak | Flatpak | LGPL-2.1+ | CLI/portal | Adapter |
| Snap | snapd | GPL-3.0 | CLI/portal | Optional |
| Online accounts | GNOME Online Accounts | LGPL-2.1+ | D-Bus | Optional |
| Calendar/contacts | evolution-data-server | LGPL-2.1+ | D-Bus | Optional |
| Mail | Thunderbird | MPL-2.0 | Separate app | User choice |
| Browser | user's browser | — | Default handler | Not first-party |
| Terminal | user's terminal | — | Default handler | Not first-party |
| Text editor | GNOME Text Editor/Kate | GPL/LGPL | Separate app | User choice |
| Archive | File Roller/Ark | GPL-2.0+ | Separate app | User choice |
| PDF | Evince/Okular | GPL-2.0+ | Separate app | User choice |
| Video | mpv/Celluloid | GPL-2.0+ | Separate app | User choice |
| Music | Lollypop/Amberol/Elisa | GPL/LGPL | Separate app | User choice |
| Photos | GNOME Photos/digiKam | GPL-2.0+ | Separate app | User choice |
| Screenshot | Dragonfruit portal + Flameshot | GPL-3.0 | Separate app | First-party + optional |
| Screen recording | Dragonfruit portal + OBS | GPL-2.0+ | Separate app | First-party + optional |
| Disks | GNOME Disks | GPL-2.0+ | Separate app | Delegate |
| Partitioning | GParted | GPL-2.0+ | Separate app | Delegate |
| System monitor | GNOME System Monitor/btop/htop | GPL/Apache | Separate app | Optional |
| Logs | GNOME Logs | GPL-2.0+ | Separate app | Optional |
| Passwords | GNOME Secrets/KeePassXC | GPL | Separate app | Delegate |
| Firewall | firewalld | GPL-2.0+ | D-Bus | Fedora backend |
| Firewall | ufw | GPL-3.0 | CLI | Ubuntu/Debian backend |
| Firewall UI | gufw | GPL-3.0 | Separate app | Optional |
| Updates | GNOME Software/Discover | GPL-2.0+ | Separate app | Optional |
| Display manager | GDM | GPL-2.0+ | Session entry | Supported |
| Display manager | SDDM | GPL-2.0+ | Session entry | Supported |
| Display manager | LightDM | GPL-3.0 | Session entry | Supported |
| Display manager | greetd | GPL-3.0 | Session entry | Supported |
| Color | colord | LGPL-2.1+ | D-Bus | Adapter |
| Printing | CUPS | Apache-2.0 | IPP/D-Bus | Adapter |
| Scanning | SANE | GPL-2.0+ | CLI/API | Adapter |
| Bluetooth | BlueZ | LGPL-2.1+ | D-Bus | Adapter |
| Network | NetworkManager | LGPL-2.1+ | D-Bus | Adapter |
| Audio | PipeWire | MIT | D-Bus/API | Adapter |
| Audio | WirePlumber | MIT | D-Bus/API | Adapter |
| Power | UPower | GPL-2.0+ | D-Bus | Adapter |
| Power profiles | power-profiles-daemon | GPL-3.0+ | D-Bus | Adapter |
| Storage | UDisks2 | LGPL-2.1+ | D-Bus | Adapter |
| Accounts | accountsservice | GPL-3.0+ | D-Bus | Adapter |
| Screen reader | Orca | LGPL-2.1+ | AT-SPI | Delegate |
| Speech | speech-dispatcher | GPL-2.0+ | API | Delegate |
| Speech synth | espeak-ng | GPL-3.0+ | CLI/API | Optional |
| Speech synth | Piper | MIT | API | Optional |
| IME | ibus | LGPL-2.1+ | D-Bus/API | Adapter |
| IME | fcitx5 | LGPL-2.1+ | D-Bus/API | Adapter |
| Cursors | Capitaine Cursors | LGPL | Asset | Existing |
| Icons | Tabler | MIT | Asset | Existing |
| Icons | Phosphor | MIT | Asset | Existing |
| Font | Inter | OFL-1.1 | Asset | Existing |
| Wallpaper | Wikimedia Commons | asset-specific | Provider | Existing |

**License verification is part of the packaging task.** The table records the
expected license; the packaging metadata and `NOTICE` must be checked against
the actual package version before release. When a license is ambiguous, the
component is optional and separate-process until resolved.

## 5. Linking and distribution policy

1. **MIT codebase.** Original compositor, shell, services, protocols, design
   system, and apps remain MIT.
2. **Separate processes.** GPL services (PackageKit, Tracker, firewalld, Orca,
   etc.) are separate executables communicating over D-Bus/CLI/portal. No GPL
   code is copied into MIT files.
3. **Dynamic linking.** LGPL libraries (Qt, NetworkManager client, colord,
   etc.) may be dynamically linked with the LGPL notices and the ability to
   replace the library.
4. **No static GPL linking.** Do not statically link a GPL library into a
   Dragonfruit binary.
5. **Optional dependencies.** The core desktop does not require a GPL service.
   Packaging marks optional dependencies as `Recommends`/`Suggests` (or the
   distro equivalent).
6. **Asset licenses.** Every asset has a source and license in `NOTICE` or an
   asset manifest; no asset with an unknown license.
7. **Contributions.** A contribution that adds a third-party dependency must
   update this table, `NOTICE`, and the packaging metadata in the same change.
8. **No Apple code or assets.** Clean-room implementation from measured
   behaviour; no decompiled Apple code, no Apple binaries, no SF Symbols, no
   Apple sounds/wallpapers.

## 6. Review gate

Before a release:

- [ ] `scripts/check-desktop-names.sh` passes and covers user-facing strings.
- [ ] No Apple trademark appears in UI, desktop files, package metadata, or
      screenshots (except historical design docs marked local-only).
- [ ] Every new asset has a source/license entry.
- [ ] `NOTICE` matches the dependency and asset manifests.
- [ ] Every optional dependency has an absence test.
- [ ] Every GPL/LGPL service is separate-process or dynamically linked with
      notices.
- [ ] The naming map is reflected in the Settings catalog and shell strings.
- [ ] A legal review is completed before the first public release.

## 7. Open decisions

These are tracked in the task set and must be decided before implementation:

1. **Nearby transfer**: omit, LocalSend (Apache-2.0), KDE Connect (GPL), or
   Warpinator (GPL)?
2. **Search backend**: built-in, Tracker, Baloo, or Recoll?
3. **Backup UI**: restic + first-party front end, Pika, Timeshift, or no
   first-party UI?
4. **Online accounts**: GOA dependency or omit?
5. **Accessibility**: Orca integration depth and shell AT-SPI export.
6. **IME**: ibus, fcitx5, or both?
7. **Display manager**: recommend one or support all four?
8. **Firewall**: firewalld + ufw adapters or a single nftables abstraction?

The default position in this document is the safest permissive/optional path;
the task that implements the feature must record the final decision in an ADR.

## 8. References

- [Existing licensing policy](../licensing.md)
- [Existing design-system doc](10-design-system.md)
- [Existing risks doc](14-risks.md)
- [Cross-distro contract](distro-compatibility.md)
- [Total experience design](22-total-experience.md)
- [ADR 0220](adr/0220-generic-naming-and-third-party-substitution.md)
