# System Settings — reference inventory

Dragonfruit's Settings app mirrors macOS System Settings **nearly verbatim** —
style, information architecture, section order, row labels, control types, and
wording. Only Apple/Mac-only concepts are dropped: Apple Account/iCloud,
AppleCare, Continuity/AirDrop, Siri/Apple Intelligence, Screen Time, Touch
ID/Apple Watch, App Store, Find My, FileVault. Generic terms are kept; Linux
deviations are chosen later.

Source: local-only captures under `docs/reference/macos/` that **never ship**
([14-risks.md](../design/14-risks.md)); raw per-image inventories in
`docs/reference/macos/notes/` (git-ignored). This is the IA/label source for
Wave 1 ([T-09](../design/tracks/09-settings-app-wave-1.md)) and Waves 2–3
([T-15](../design/tracks/15-system-services-breadth.md)); a capture wins on
disagreement.

## Window anatomy

From the General, Desktop & Dock, and Sound captures.

- **Window**: rounded corners, drop shadow, translucent toolbar and sidebar,
  light appearance; red/yellow/green traffic lights at the top left.
- **Toolbar**: back/forward chevrons in pill buttons (forward dimmed at the end
  of history) with the pane title centered; a sidebar search field with a
  magnifier glyph and the placeholder `Search` filters the pane list.
- **Split view**: sidebar fixed at ~25–30% width, detail pane fluid; the
  selected sidebar row is a solid blue fill with white text.
- **Sidebar rows**: icon + label, no trailing chevrons in most captures, no
  section headers (one capture shows a separator after the account row).
- **Header card**: pane icon, pane title, and a short description line
  (`General`, `Accessibility`, `Notifications`).
- **Grouped inset sections**: rounded light-gray cards with subtle separators
  and generous padding; `Privacy & Security` is the one flat list.
- **Row controls**: toggle rows, popup rows (value + down chevron), slider rows
  with end labels (`Small`/`Large`), disclosure rows with a right chevron,
  checkbox rows (blue check), radio groups, a segmented control / tab bar
  (selected segment blue), a stepper / number field, trailing action buttons
  (`Details...`, `Options...`, `Edit...`), and a bottom-right `?` help button;
  tables (`Sound` > `Output & Input`) use a header row (`Name`, `Type`) and a
  selected gray device row. No capture has an Apply button — controls apply
  live; some panes add a hint/warning line above the help button.

## Sidebar

Observed order, top to bottom. The captures also show a top `Search` field and
an Apple-only account row (avatar + name + `Apple Account`) above the pane
list — omitted; our account row is TBD. Routing per pane is under `Panes`.

- `Wi-Fi` — captured (9.20.59, sheet 9.23.53); no tracked task yet.
- `Bluetooth` — captured (9.21.00, sheet 9.23.47); T-15.1.
- `Network` — captured (9.21.03); no tracked task yet (T-15.15 = VPN only).
- `Battery` — captured (9.21.05); T-15.6.
- `General` — captured (9.20.49, 9.21.09); T-15.10.
- `Accessibility` — captured (9.21.12); T-15.14.
- `Appearance` — not captured; T-09 Wave 1 (T-09.2).
- `Apple Intelligence & Siri` — not captured; not planned (Apple-only).
- `Desktop & Dock` — captured (9.21.21); T-09 Wave 1 (T-09.4, `dock.*`).
- `Displays` — captured (9.21.29); T-09 Wave 1 (T-09.5, output API).
- `Menu Bar` — captured (9.21.32); T-15.9.
- `Spotlight` — not captured; no tracked task yet.
- `Wallpaper` — captured (9.21.35); T-09 Wave 1 (T-09.3).
- `Notifications` — captured (9.21.38); T-15.7.
- `Sound` — captured (9.21.42); T-15.3.
- `Focus` — not captured; T-15.7.
- `Screen Time` — not captured; not planned.
- `Lock Screen` — captured (9.21.45); T-15.8.
- `Privacy & Security` — captured (9.21.47); T-15.13.
- `Touch ID & Password` — captured (9.21.49); no tracked task (biometrics).
- `Users & Groups` — captured (9.21.51); T-15.11.
- `Internet Accounts` — not captured; not planned.
- `Keyboard` — captured (9.21.55); T-15.4.
- `Mouse` — not captured; T-15.4 (covers Keyboard, Mouse, Trackpad).
- `Trackpad` — captured (9.21.57); T-15.4.
- `Printers & Scanners` — captured (9.21.58); T-15.12.

The captures also show Apple-only rows lower in the list (`Game Center`,
`iCloud`, `Wallet & Apple Pay`), omitted, and a trailing third-party `macFUSE`
pane (Keyboard/Trackpad/Printers captures) — not part of the stock order.

## Panes

Each pane names its capture (`Screenshot-2026-09-18-at-<time>-PM.png`), then
the observed section titles and row labels in order with control types.

### Wi-Fi

`Screenshot-2026-09-18-at-9.20.59-PM.png` (pane) +
`Screenshot-2026-09-18-at-9.23.53-PM.png` (sheet) — routed to NetworkManager;
no tracked task yet.

- `Wi-Fi` (header toggle, on) — "Set up Wi-Fi to wirelessly connect your Mac
  to the internet. Turn on Wi-Fi, then choose a network to join. Learn
  More...".
- Current network: `<network>` (green dot, lock, signal), with a
  `Details...` button.
- `Known Network`: `<network>` (checkmark, lock, signal, `⋯` menu).
- `Other Networks`: a list of nearby networks (`<network>` placeholders; each
  lock, signal, `⋯`), then an `Other...` button.
- `Ask to join networks` (popup, `Notify`) and `Ask to join hotspots` (popup,
  `Ask To Join`), each with its description text.
- Quick-settings sheet: `Wi-Fi` toggle (on); `Known Network` > `<network>`;
  `Other Networks` (expanded, down chevron): a scrolling nearby-network list
  (names redacted; lock/signal per row), then `Other...` and
  `Wi-Fi Settings...`.

### Bluetooth

`Screenshot-2026-09-18-at-9.21.00-PM.png` (pane) +
`Screenshot-2026-09-18-at-9.23.47-PM.png` (sheet) — routed to BlueZ; T-15.1.

- `Bluetooth` (toggle, on) — "Connect to accessories you can use for
  activities such as streaming music, typing, and gaming. Learn more...";
  below it the static line "This Mac is discoverable as `<device>` while
  Bluetooth Settings is open."
- `My Devices`: `WF-1000XM6` and `WH-1000XM6`, list rows with `Not Connected`
  and info (`i`) buttons; a `?` help button.
- `Nearby Devices`: `Searching...` with a spinner.
- Quick-settings sheet: `Bluetooth` toggle (on); `WF-1000XM6` and
  `WH-1000XM6` as informational rows; a `Bluetooth Settings...` link.

### Network

`Screenshot-2026-09-18-at-9.21.03-PM.png` — routed to NetworkManager; no
tracked task yet.

- `Wi-Fi` — `Connected` (green dot), chevron.
- `Firewall` — `Inactive` (gray dot), chevron.
- `Other Services`: `USB 10/100/1000 LAN` and `Thunderbolt Bridge`, each
  `Not connected` (red dot), chevrons. Footer: a `...` dropdown and `?` help.

### Battery

`Screenshot-2026-09-18-at-9.21.05-PM.png` — routed to UPower +
power-profiles-daemon where present; T-15.6.

- `Low Power Mode` (popup, `Never`).
- `Battery Health` (info row, `Normal`, `i`); `Charging` (info row, `i`).
- Segmented control: `Last 24 Hours` (selected) / `Last 10 Days`.
- `Last charged to 100%` — `<date, time>`.
- `Battery Level` (bar chart; y `100%`/`50%`/`0%`; x `12 A` … `12 P`).
- `Screen On Usage` (empty line chart; y `60m`/`30m`/`0m`; footer `Sep 18`);
  `Options...` button and `?` help.

### General

`Screenshot-2026-09-18-at-9.20.49-PM.png` and
`Screenshot-2026-09-18-at-9.21.09-PM.png` — routed to settingsd + the distro
update provider; T-15.10. Sub-screen `About`:
`Screenshot-2026-09-18-at-9.22.28-PM.png`.

- Header: `General` — "Manage your overall setup and preferences for Mac,
  such as software updates, device language, AirDrop, and more."
- Disclosure rows: `About`, `Software Update`, `Storage`; Apple-only rows
  (omitted): `AppleCare & Warranty`, `AirDrop & Continuity`; then `AutoFill &
  Passwords`, `Date & Time`, `Language & Region`, `Login Items & Extensions`,
  `Sharing`, `Startup Disk`.
- `About`: device illustration + name (reference `MacBook Pro`, `14-inch,
  M5`); label/value rows `Chip`, `Memory`, `Serial number` (`<serial>`),
  `macOS`; `More Info...` button; `Regulatory Certification` link; Apple
  copyright footer (omitted).

### Accessibility

`Screenshot-2026-09-18-at-9.21.12-PM.png` — routed to settingsd → compositor
magnification + toolkit/AT-SPI; T-15.14.

- Header: `Accessibility` — "Personalize Mac in ways that work best for you
  with accessibility features for vision, hearing, motor, speech, and
  cognition. Learn more...".
- `Vision`: `VoiceOver`, `Zoom`, `Hover Text`, `Display`, `Motion`,
  `Read & Speak`, `Audio Descriptions` — disclosure rows.
- `Hearing`: `Hearing Devices`, `Audio`, `Captions`, `Live Captions`,
  `Name Recognition` — disclosure rows; the capture ends here.

### Desktop & Dock

`Screenshot-2026-09-18-at-9.21.21-PM.png` — routed to our own settings
(`dock.*` keys → shell/Dock); T-09 Wave 1 (T-09.4).

- `Dock`: `Size` (slider, `Small`/`Large`; `dock.size`), `Magnification`
  (slider, `Off`/`Small`/`Large`; `dock.magnification`), `Dock position on
  screen` (popup, `Bottom`; `dock.position`), `Minimized window animation`
  (popup, `Genie Effect`; `dock.minimizedAnimation`), `Window title bar
  double-click action` (popup, `Zoom`; `dock.titlebarDoubleClick`), `Minimize
  windows into application icon` (toggle, off; `dock.minimizeIntoTileIcon`),
  `Automatically hide and show the Dock` (toggle, off; `dock.autohide`),
  `Animate opening applications` (toggle, on; `dock.animateOpening`), `Show
  indicators for open applications` (toggle, on; `dock.showIndicators`),
  `Show suggested and recent apps in Dock` (toggle, on; `dock.showRecentApps`).
- `Desktop & Stage Manager`: `Show items` (segmented, `On Desktop` selected /
  `In Stage Manager`), `Click wallpaper to show desktop` (popup, `Always`),
  `Stage Manager` (toggle, off), `Show recent apps in Stage Manager` (toggle,
  on); no schema keys for this section yet — provider TBD.

### Displays

`Screenshot-2026-09-18-at-9.21.29-PM.png` — routed to the compositor output
API; T-09 Wave 1 (T-09.5; advanced color, night light, VRR are T-15/T-16).

- Preview: `Built-in Display` illustration.
- Resolution scaling tiles: `Larger Text` (selected), two partial tiles,
  `Default`, `More Space`; footer "Using a scaled resolution may affect
  performance.".
- `Brightness`: slider; `Automatically adjust brightness` (toggle, on);
  `True Tone` (toggle, on).
- Display settings: `Preset` (popup, `Apple XDR Display (P3-1600 nits)`),
  `Refresh rate` (popup, `ProMotion`), `When connected to TV` (popup,
  `Ask What to Show`).
- `Apple XDR`, `True Tone`, and `ProMotion` are Apple display features; we
  expose the compositor's outputs instead.

### Menu Bar

`Screenshot-2026-09-18-at-9.21.32-PM.png` — routed to settingsd → shell;
T-15.9.

- `Automatically hide and show the menu bar` (popup, `In Full Screen Only`);
  `Show menu bar background` (toggle, off); `Recent documents, applications,
  and servers` (stepper field, `10`).
- `Menu Bar Controls` — "System and app controls can be configured to appear
  in both Control Center and the menu bar.": `Add Controls...` (button);
  `Clock` (row with `Clock Options...`), `Spotlight`, `Wi-Fi`, `Bluetooth`,
  `Battery` (row with `Battery Options...`), `AirDrop` (Apple-only, omitted),
  `Focus`, `Screen Mirroring`, `Display`, `Sound` — checkboxes, checked; the
  last four have a popup (`Show When Active`).

### Wallpaper

`Screenshot-2026-09-18-at-9.21.35-PM.png` — routed to settingsd + compositor
(per-Space wallpaper); T-09 Wave 1 (T-09.3). Reference wallpaper names are
Apple-specific; our artwork is our own.

- Current wallpaper preview: `Tahoe Evening`; `Show on all Spaces` (toggle,
  on); `Screen Saver...` and `Clock Appearance...` buttons.
- `Dynamic Wallpapers` (`Show All (34)`): `Tahoe`, `Sequoia`, `Macintosh`,
  `Sonoma` tiles with info badges.
- `Your Photos`: `Add Photo...` button.
- `Landscape` (`Show All (79)`): `Tahoe Day`, `Sequoia Sunrise`,
  `Sonoma Horizon`, `Goa Beaches` tiles with video and download overlays.
- `Cityscape` (`Show All (30)`): four tiles with video and download overlays.

### Notifications

`Screenshot-2026-09-18-at-9.21.38-PM.png` — routed to the notification
service; T-15.7.

- Header: `Notifications` — "Customize when and how notifications appear, if
  they play a sound, and which apps can send them. Learn more...".
- `Notification Center` — "Notification Center shows your notifications in
  the top-right corner of your screen. You can show and hide Notification
  Center by clicking the clock in the menu bar.": `Show previews` (popup,
  `When Unlocked`); `Show Notifications:` (`when display is sleeping`, toggle
  off; `when screen is locked`, toggle on; `when mirroring or sharing the
  display`, popup `Notifications Off`).
- `Application Notifications`: per-app disclosure rows — icon, name, and a
  state subtext (`Off`, `Badges, Sounds`), each opening per-app settings
  (app names redacted; the reference shows a mix of both states).

### Sound

`Screenshot-2026-09-18-at-9.21.42-PM.png` — routed to
PipeWire/WirePlumber; T-15.3.

- `Sound Effects`: `Alert sound` (popup, `Boop`, with a play button); `Play
  sound effects through` (popup, `Selected Sound Output Device`); `Alert
  volume` (slider, ~80%); `Play sound on startup` (toggle, on); `Play user
  interface sound effects` (toggle, on); `Play feedback when volume is
  changed` (toggle, off). `Boop` is a macOS-only alert sound; ours is ours.
- `Output & Input`: tabs `Output` (selected) / `Input`; device table with
  columns `Name` / `Type` and the selected row `MacBook Pro Speakers` /
  `Built-in`; `Output volume` (slider, ~40%, `Mute` checkbox checked);
  `Balance` (slider, centered, `Left`/`Right`); `?` help.

### Lock Screen

`Screenshot-2026-09-18-at-9.21.45-PM.png` — routed to settingsd + compositor
lock/idle policy; T-15.8.

- `Turn display off on battery when inactive` (popup, `For 10 minutes`); `Turn
  display off on power adapter when inactive` (popup, `For 20 minutes`) with
  the warning "Energy usage may be higher when this Mac is inactive for longer
  periods of time before the display turns off.".
- `Require password after screen saver begins or display is turned off`
  (popup, `After 5 seconds`).
- `Show user name and photo` (toggle, on); `Show password hints` (toggle,
  off); `Show message when locked` (toggle, off + `Set...` button).
- `When Switching User`: `Login window shows` (radio, `List of users`
  selected / `Name and password`); `Show the Sleep, Restart, and Shut Down
  buttons` (toggle, on); `Accessibility Options...` button.

### Privacy & Security

`Screenshot-2026-09-18-at-9.21.47-PM.png` — routed to settingsd + host services
(Secret Service, polkit) and portals; T-15.13. A flat list (no inset groups);
every row is a disclosure row.

- `Privacy` header — "Control which apps can access your data, location,
  camera, and microphone, and manage safety protections. Learn more...".
- `Location Services` (badge `1`), `Calendars` (`1 full access`), `Contacts`
  (`None`), `Files & Folders` (`18 apps`), `Full Disk Access` (`1 full
  access`), `Home` (`None`), `Media & Apple Music` (`3 apps`), `Passkeys
  Access for Web Browsers` (`1 full access`), `Photos` (`3 full access`),
  `Reminders` (`None`), `Accessibility` (badge `2`), `App Management`
  (badge `3`). `Media & Apple Music` keeps its reference name; we keep a
  media-library access row and drop the Apple Music half.

### Touch ID & Password

`Screenshot-2026-09-18-at-9.21.49-PM.png` — biometrics: fprintd + Secret
Service where present; no tracked task (T-15 has no biometrics unit).

- `Password`: "A login password has been set for this user." + `Change...`
  button.
- `Touch ID`: `Finger 1` tile and `Add Fingerprint` tile.
- Toggles: `Use Touch ID to unlock your Mac` (on); `Use Touch ID for Apple
  Pay` (on — Apple-only); `Use Touch ID for purchases in iTunes Store, App
  Store, and Apple Books` (on — Apple-only); `Use Touch ID for autofilling
  passwords` (on); `Use Touch ID for fast user switching` (on); `?` help.

### Users & Groups

`Screenshot-2026-09-18-at-9.21.51-PM.png` — routed to accountsservice + the
distro provider; T-15.11.

- User list: `<user>` (`Admin`) and `Guest User` (`Off`), each with an info
  (`i`) button; `Add Group...` and `Add User...` buttons.
- `Automatically log in as` (popup, `Off`, disabled) — the reference helper
  text cites FileVault (Apple-only); ours cites disk encryption.
- `Network account server` + `Edit...` button; `?` help.

### Keyboard

`Screenshot-2026-09-18-at-9.21.55-PM.png` — routed to input settings
(libinput); T-15.4.

- Keyboard section: `Key repeat rate` (slider, `Off`/`Slow`/`Fast`); `Delay
  until repeat` (slider, `Long`/`Short`); `Adjust keyboard brightness in low
  light` (toggle, on); `Keyboard brightness` (slider); `Turn keyboard
  backlight off after inactivity` (popup, `Never`); `Press ⌘ key to` (popup,
  `Show Emoji & Symbols`); `Keyboard navigation` (toggle, off) with helper
  text and a `Keyboard Shortcuts...` button.
- `Text Input`: `Input Sources` (popup, `U.S.`) + `Edit...`; `Text
  Replacements...` button. `Dictation`: Dictation (toggle, off) with helper
  text; `Languages` (popup, `English (United States)`) + `Edit...`;
  `Microphone source` (popup, `Automatic (MacBook Pro Microphone)`).
- The `⌘` (Command) key is Apple-only; our key label and shortcut grammar are
  a Linux-like deviation decided later.

### Trackpad

`Screenshot-2026-09-18-at-9.21.57-PM.png` — routed to the compositor input API
+ libinput; T-15.4.

- Gesture preview area: a simulated trackpad and a gesture preview panel.
- Tabs: `Point & Click` (selected), `Scroll & Zoom`, `More Gestures`.
- `Tracking speed` (slider, `Slow`/`Fast`); `Click` (slider,
  `Light`/`Medium`/`Firm`); `Force Click and haptic feedback` (toggle, on)
  with helper text; `Look up & data detectors` (popup, `Force Click with One
  Finger`); `Secondary click` (popup, `Click or Tap with Two Fingers`); `Tap
  to click` (toggle, on).
- `Set Up Bluetooth Trackpad...` button; `?` help. `Force Click` is Apple
  hardware behavior; our equivalent is decided later.

### Printers & Scanners

`Screenshot-2026-09-18-at-9.21.58-PM.png` — routed to CUPS + SANE; T-15.12.

- `Default printer` (popup, `Last Printer Used`); `Default paper size`
  (popup, `US Letter`).
- `Printers`: `Canon MF230` (`Idle, Last Used`, green dot, chevron);
  `Add Printer, Scanner, or Fax...` button; `?` help.

## Not captured

Panes with no reference image:

- `Appearance` — T-09 Wave 1 (T-09.2; `appearance.colorScheme`,
  `appearance.accent`, plus `accessibility.reduceMotion`).
- `Focus` — T-15.7.
- `Spotlight` — no tracked task yet.
- `Screen Time` — not planned.
- `Mouse` — T-15.4.
- `General` > `Storage` — T-15.2 (storage and removable media).
- `Mission Control` / hot corners (`Desktop & Dock` section) — T-15.5.
- The lock screen itself (`Lock Screen` has only its settings pane).
- Notification banners (`Notifications` has only its settings pane).
- Control Center panel — T-11.

The same capture set also holds non-Settings references, distilled separately
in [macos-ui-inventory.md](macos-ui-inventory.md), not here:

- Finder (`9.20.30`); Desktop, Dock, and menu bar including the system menu
  (`9.20.18`, `9.20.46`).
- Spotlight search field (`9.24.16`, `9.24.19`); Launchpad / Applications grid
  (`9.22.17`); Activity Monitor (`9.23.13`).
