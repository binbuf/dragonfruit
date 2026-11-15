# System Settings

| | |
|---|---|
| **Status** | target design; 2026-11-14 pane-by-pane review |
| **Scope** | every System Settings pane, section, row, owner, absence path, and the shell/component contract they share |
| **Companions** | [08-settings.md](08-settings.md), [settings-business-logic-map.md](settings-business-logic-map.md), [23-macos-structural-surface-review.md](23-macos-structural-surface-review.md), [system-settings roadmap](../tasks/system-settings/plan/ROADMAP.md), [ADR 0262](adr/0262-system-settings-completion-contract.md) |

> The completion tasks T263–T284 live in the dedicated **system-settings**
> symphony set (`docs/tasks/system-settings/`). Run them with
> `./.symphony/symphony run --set system-settings`.

> The app is now **System Settings**. Internal paths remain `apps/settings`,
> `org.dragonfruit.Settings1`, and the `settingsd` service name; user-facing
> strings and docs use “System Settings”.

## 1. Why this document

The first System Settings pass (T222–T229a) landed breadth: 28 shipped
panes, real host adapters, an executable row map, live apply/validation, and
an absence matrix. A 2026-11-14 review of every pane found that breadth did
not become a top-tier experience:

- Several panes collapse to a single note when one host adapter is absent;
  the sections and settingsd-backed controls disappear with it.
- Many shipped controls write settingsd keys that no process consumes. The
  UI says “live”; the desktop does not change.
- Adapter writes are fire-and-forget. A polkit denial, missing backend, bad
  password, or failed `timectl` write is invisible, and the control stays in
  the state the user attempted.
- The macOS Tahoe information architecture is only partly present. The
  Accessibility pane has three settingsd rows against a reference that has
  five sections and dozens of rows; Appearance is missing icon/widget style,
  folder color, and scroll-bar click behavior; General is missing Autofill &
  Passwords and the Transfer or Reset structure.
- The shell was not the measured System Settings window: it opened at
  800×640, used a 192 pt sidebar, had no sidebar groups, opened Appearance
  instead of General, and lost focus/scroll/history on pane changes. T263
  lands the measured 740×820 window, the 232 pt grouped sidebar, General as
  the default, the split unified toolbar, and remembered geometry/last pane;
  T264 makes history sub-pane/row-aware, restores focus and scroll, and makes
  deep links, Help, and the menus real.
- Shared components have defects that affect every pane: a `SettingsRow`
  binding loop, clipped `Select` menus, non-reparented dialogs, missing focus
  rings, incomplete accessibility names, and long-string/layout failures.

This document is the target. It is deliberately row-level: a pane is only
real when every visible row has an owner, a consumer, and an absence path.
The executable projection remains
[`apps/settings/SettingsMap.qml`](../../apps/settings/SettingsMap.qml); the
human contract remains
[settings-business-logic-map.md](settings-business-logic-map.md). This
document adds the macOS Tahoe row inventory, the Linux adaptations, the
visual/behaviour contract, and the task sequence that closes the review.

## 2. Review method and evidence

- Every shipped pane was rendered offscreen through the real Settings shell
  with the deterministic settings fixture and inspected at 1000×760. The
  local harness is not committed.
- The row inventory was compared against the macOS Tahoe 26 System Settings
  reference inventory. Local macOS captures are reference only and never
  ship; no macOS screenshot is copied into this repository.
- The shell, panes, shared components, settingsd schema, settings client,
  bridge clients, adapters, tests, and capture scripts were read. The
  findings below name the file that proves them.
- The review does not invent measured values. Where a macOS value is not
  available in the in-repo evidence, the row is `OPEN` with a rerun command
  rather than a guessed number.

The reference facts used throughout:

| Surface | Target |
|---|---|
| Window | 740×820 pt measured; 52 pt unified chrome; 768 pt content viewport |
| Sidebar | 232 pt; grouped source list; search field at the top |
| Detail content | 508 pt wide; 30 pt horizontal content inset; 44 pt minimum rows, growing with description |
| Group card | 10 pt radius; 8 pt padding; 12 pt row horizontal padding; 1 pt separator |
| Controls | 40×24 pt switch (design-system token); compact segmented control; 160 pt slider; select sized to the longest option |
| Appearance | Light/Dark/Auto; accent; text highlight; icon/widget style; folder color; sidebar icon size; wallpaper tint; scroll bars; scroll-bar click behavior |
| Default pane | General, or the last visited pane when a preference exists |

The 740×820 window, 232 pt sidebar, 508 pt content width, and 30 pt content
inset come from the in-repo structural measurements
(`macos-design-review/docs/experience_measurements.json`,
`settings_pane` and `system_settings`). Before T263 the code used 800×640 and a
192 pt sidebar; T263 corrects it to the measured values.

### 2.1 Review coverage

| Pane | Captured | Main finding | Owner task |
|---|---|---|---|
| Wi-Fi | yes | Advanced is a dead end; no hotspot/scanning/known details; password survives cancel; duplicate absence note | T271 |
| Bluetooth | yes | Info button connects instead of showing info; no Forget; discovery does not start on power-on | T271 |
| Network | yes | No status dots or per-service details; firewall disabled by NM; VPN group disappears | T272 |
| Battery | yes | Two permanently empty charts; options incomplete; show-percentage has no consumer | T273 |
| General | yes | Whole list hides when one adapter is absent; About/Software Update/Storage/Reset vanish | T269, T270 (landed) |
| Storage | yes | Dead Manage row; no chart/total/free; volumes have no capacity | T273 |
| Appearance | yes | Missing icon/widget style, folder color, scroll-bar click; transparency slider active under override; seven one-row cards | T274 |
| Accessibility | yes | Three settingsd rows against a five-section macOS IA; map claims rows that do not exist | T275 |
| Quick Settings | yes | 37-row Up/Down list; unavailable modules offered; Edit Controls no-op; empty list restores defaults | T277 |
| Menu Bar | yes | No third-party section, Add Controls, Reset, or per-control options | T277 |
| Search | yes | No-op surface; shortcut free text; indexing rows name nonexistent keys; “wifi” returns no results | T265, T282 |
| Privacy & Security | yes | Flat un-carded list with one icon, no keyboard, missing master/analytics/accessory rows | T279 |
| Desktop & Dock | yes | Missing Desktop & Stage Manager, Widgets, Windows, Mission Control groups | T276 |
| Overview | yes | Only four rows; no workspace count/per-display/auto-rearrange/group-by-app | T276 |
| Displays | yes | Blank hardcoded preview; no arrangement/Night Light/color/refresh/HDR/VRR; duplicate advanced controls | T273 |
| Wallpaper | yes | 160×100 hero; Clock Appearance opens a pane with no clock controls; categories diverge from Tahoe; tile clicks are not acknowledged immediately and a second pick during an apply is ignored | T278 |
| Screen Saver | yes | No gallery; Start After blanks the display; None does not disable the saver | T278 |
| Notifications | yes | Global preferences are no-ops; per-app sheet missing half the options; duplicated status | T278 |
| Sound | yes | Mute zeroes the volume slider; Input tab shows output controls; Balance has no consumer | T280 |
| Focus | yes | Inert mode rows styled as radio controls; three duplicate state surfaces; no list/add/delete/filters | T278 |
| Lock Screen | yes | Require-password writes inactivity seconds; custom idle displays “Never”; no battery/AC split | T278 |
| Biometrics & Password | yes | Password dialog closes before the host result; no enrollment progress; no fast user switching | T279 |
| Users & Groups | yes | No password path for a new user; no change-password row; group delete has no confirmation | T279 |
| Online Accounts | code/absence | Pane disappears when GOA is absent; Add has no sign-in step | T279 |
| Printers & Scanners | yes | Add dialog disconnected from its fields; no remove/test/share; scanners have no action | T280 |
| Keyboard | yes | Input sources/shortcuts/dictation/text replacements incomplete; several no-consumer rows presented as live | T280 |
| Mouse | yes | Missing smart zoom, pointer acceleration, more gestures; shares keys with Trackpad | T280 |
| Trackpad | yes | Missing click pressure, zoom/smart zoom/rotate, and most More Gestures; shares keys with Mouse | T280 |
| Game Controllers | yes | Test dialog shows a stale result and has no close button; absence is one note | T280 |

## 3. The experience contract

### 3.1 Window and chrome

System Settings is one 740×820 pt window with a 52 pt unified toolbar. The
toolbar is split at the sidebar edge:

- Left of the split: the traffic lights and the sidebar; the sidebar material
  runs under the titlebar, as in the reference.
- Right of the split: Back/Forward, the pane title, and any pane-level action.
  The title is centred over the detail pane, not over the whole window.
- The window title (compositor/taskbar name) is the current pane title, or
  “System Settings” at the root. It is not a constant.
- The detail sheet is opaque; the sidebar and toolbar use the measured
  `settingsSidebar`/`settingsToolbar` material presets. Reduce Transparency
  uses the shared opaque plate; Increase Contrast raises separator ink.
- The content viewport is 768 pt high. The first group starts at the viewport
  top with the group's own top padding; the last group reserves 24 pt before
  the bottom edge. The 30 pt value is the horizontal inset measured from the
  scroll edge, not a vertical offset below the toolbar.
- The window remembers its geometry and last pane. The first launch opens
  General. A deep link may open any other pane.
- A mapped window is clickable at the pointer's current position without a
  motion event. The activating press both focuses the window and reaches the
  row: a stationary click is never a focus-only click.
- Minimum size is 720×480; at the minimum the sidebar remains scrollable and
  every row stays inside the card. The 740×820 layout is the design target,
  not a hard-coded maximum.

T263 lands this contract. The values are tokens
(`component.settingsWindow`: 740×820, 720×480 minimum, 52 pt chrome, 232 pt
sidebar, 508 pt detail, 30 pt inset), not app constants. The window and the
shell remember `settings.lastPane` plus `settings.window.{width,height,x,y}`
and `settings.window.positioned`; a deep link wins over the restored pane.
The split is literal: the sidebar's material covers the full window height
(including the toolbar strip and traffic lights), while the detail half owns
Back/Forward and the centred pane title.

### 3.2 Sidebar information architecture

The sidebar is a grouped source list, not one flat list. The order follows
macOS Tahoe as closely as the Linux ownership model allows:

| Group | Panes |
|---|---|
| Connectivity | Wi-Fi, Bluetooth, Network, VPN (optional; appears when a VPN profile exists) |
| General | General (About, Software Update, Storage, Autofill & Passwords, Date & Time, Language & Region, Login Items & Extensions, Sharing, Startup Disk, Backup, Transfer or Reset) |
| Personalization | Appearance, Accessibility, Quick Settings, Search, Privacy & Security |
| Desktop | Desktop & Dock, Overview, Displays, Wallpaper, Screen Saver, Notifications, Sound, Focus, Lock Screen |
| Accounts & Devices | Users & Groups, Online Accounts, Game Controllers, Printers & Scanners |
| Input | Keyboard, Mouse, Trackpad |
| System | Battery, Menu Bar |

Decisions:

- **Storage moves under General.** The top-level `storage` id remains a
  deep-link alias (`settings://pane/storage`) that opens General → Storage.
  The sidebar no longer shows a separate Storage row. This matches macOS and
  removes a duplicate concept.
- **Overview stays a top-level Linux feature** (workspaces, hot corners,
  gestures), grouped next to Desktop & Dock. Search synonyms include
  “Mission Control”.
- **VPN is optional.** It appears in Connectivity only when NetworkManager
  reports at least one VPN profile; otherwise the Network → VPN section is
  the only entry point. T272 makes it a real pane with the same one VPN model
  as Network → VPN.
- **Online Accounts is visible even when the host service is absent.** The
  pane shows an explicit “Internet accounts are not installed” state with a
  delegate action. It does not vanish from the sidebar; a familiar macOS
  entry must not silently disappear. T263 removes the old optional-hiding
  rule; T279 owns the absent-service surface.
- **Activation is motion-independent.** A sidebar row activates on one
  stationary primary click. Hover decorates the row; it never gates the
  click. After the list scrolls under a stationary pointer, the next click
  hits the row now under the pointer, not the row that was there at the last
  motion. A click that loses the event because the compositor routed it
  through a stale cached location is a P0 bug, not an acceptable quirk. T263
  ray-tests the live pointer position before a press (`input.rs`), so a window
  that mapped or a list that scrolled under a resting pointer cannot swallow
  the first click.
- **Omitted panes are not in the sidebar.** Apple Account, iCloud, Screen
  Time, Siri, Wallet, Game Center, Find My, AirDrop/Handoff, Device
  Management, and AppleCare stay omitted with their existing reasons.

### 3.3 Row and control anatomy

Every pane uses the same `SettingsGroup` and `SettingsRow` anatomy:

- Label, optional description, control slot, optional disclosure chevron,
  optional trailing action, optional help affordance.
- A disclosure row opens from a click anywhere on the row, not only the
  chevron. Return/Space on a focused row activates the same action.
- Activation is position- and motion-independent: press and release at one
  point activates the row or control on the first click, with no prior
  hover, motion, or wheel event. The whole row or tile is the hit target,
  not only its glyph, caption, or chevron. A press that becomes a drag is
  the only case that cancels the tap.
- Every row is a keyboard stop with a visible focus ring. Disclosure/help
  sub-controls are reachable by Tab, not only by pointer.
- Every control exposes an accessible name, current value, and range/allowed
  values. A switch inside a labelled row must not be an unnamed switch.
- Long labels elide; long descriptions wrap; the control slot never squeezes
  the label to zero width. `Button` text elides rather than overflows.
- Group titles wrap or elide; the final row in a group suppresses its
  separator; a group reserves one bottom gap, not two.
- Controls use the design-system sizes: 40×24 switch (design-system token),
  compact segmented control (not full-width), 160 pt slider, select sized to
  the longest option, 10 pt card radius. Accent follows the live theme; the
  default accent is reviewed in T274 so the pane does not read as an
  unrelated brand colour.
- Typography follows the system font: 13 pt labels/controls, 11 pt
  descriptions, 13 pt medium section headers, 15 pt semibold pane title. The
  chrome does not introduce a display face.
- Popups, selects, dialogs, and sheets are reparented to the window overlay
  layer. They are never clipped by the detail `ScrollView`; long option lists
  scroll and clamp to the available rect.
- Motion follows the shared reduced-motion path. Hover/press/focus feedback
  is tokenized; no pane invents a duration.

T266 lands the component half of this contract:

- `SettingsRow` sizes its control slot from the child's implicit size, so the
  `childrenRect` binding loop is gone and a long label keeps a readable
  column. The row has a visible focus ring; disclosure rows activate from
  anywhere on the row; the chevron and help control are Tab stops; a stock
  control takes the row label as its accessible name when the pane sets none.
- `SettingsGroup` wraps its title, suppresses the final row separator, and
  owns one bottom gap (the pane's Column spacing, not a second group margin).
- `Select`/`ContextMenu`, `Popup`, and `Dialog` adopt the window `WindowOverlay`
  layer instead of a pane-local `popupParent`. A Select is sized to its longest
  option, flips/clamps inside the window, and scrolls a long list. A Dialog
  fits the window, traps Tab, scrolls its body, and keeps a visible Close
  action pinned above the scrolling content.
- `ScrollView` implements `automatic` (focused/hovered/moving), `when-scrolling`
  (moving or dragged), and `always` (any overflow) distinctly, exposes its
  value/orientation, supports both track-click behaviours, and stops its bars
  short of the window resize corner. The horizontal path is live behind the
  `horizontalScroll` opt-in.
- The compositor's live-position ray-test from T263 remains the stationary
  click fix. T266 adds the row focus path and the shared stationary-click
  harness (sidebar, row, tile, select, dialog button) plus the
  scroll-under-pointer regression.

### 3.3.1 Navigation, history, deep links, and help

T264 makes every navigation path truthful:

- **History is a navigation state, not a pane id.** Each entry is
  `{ paneId, subPaneId, rowId }`. In General → About, Back closes About and
  returns to the General list before a second Back leaves General; Forward
  restores the sub-pane and the row. Clicking the already-selected General
  sidebar row closes its sub-pane.
- **A pane change starts at the top and restores focus.** The detail view
  resets `contentY` and focus moves to the first keyboard stop in the new
  body (or the pane title). Focus is moved off the outgoing body before the
  `Loader` destroys it.
- **Deep links land on a surface or are refused.** A General row resolves to
  its in-place sub-pane (`about`, `software-update`, `passwords`, `storage`,
  `date-time`, `language`, `login`, `sharing`, `backup`, `startup`, `reset`).
  A row the resolver cannot open is not accepted, not indexed, and not
  guessed. A rendered row with a stable `settingsId` is never matched by
  label.
- **One Help action.** The bottom-right `?` and Help → Settings Help open
  `Settings.helpUrl("pane-<id>")`, i.e. the map's `#pane-<id>` anchor. Without
  the document the control disables and carries the reason.
- **The View menu is generated from the catalog.** View → Settings Pane lists
  every `SettingsPanes.shippedPanes` entry in sidebar order, checks the current
  pane, and marks Back/Forward from the shell history. Every published action
  dispatches; `activate()` returns false for an unknown action.
- **About hands geometry back.** The About utility panel and the in-window
  About page restore the user's pre-About window geometry when they close, so
  a resized window is never left at the 740×820 design default.
- **The debug view fits the minimum window.** Pane & Row Owners sizes its card
  and row list to the 720×480 frame, scrolls the list, and keeps its pane
  selector in step with the shell.

### 3.3.2 Keyboard and focus contract (T267)

Accessibility is a shell contract, not a per-pane checklist. The map is one
table in `design-system/Keymap.qml`; components ask it for the action a key
means in their scope, so documentation and behavior cannot drift.

| Scope | Keys | Action |
|---|---|---|
| shell | Ctrl+F | Focus the search field |
| shell | Alt+Left / Alt+Right | Back / Forward |
| shell | Escape | Clear search, then close the open sub-pane |
| sidebar | Up/Down | Previous/next pane (section headers are skipped) |
| sidebar | Home/End | First/last pane |
| sidebar | Return/Enter/Space | Open the selected pane |
| search field | Down/Up | Enter the ranked result list |
| search field | Escape | Clear, then return to the sidebar |
| results | Up/Down, Home/End | Move through results |
| results | Return/Enter | Open the selected result |
| results | Escape | Clear the search and return to the field |
| any | Tab / Shift+Tab | Qt's normal focus traversal |

- **No invisible tab stops.** Overlay focus scopes (`Dialog`, `Sheet`,
  `Popup`, `Popover`, `ContextMenu`) are focus owners, not Tab stops; their
  content owns the visible rings. The search clear affordance is a real Tab
  stop with a ring and Return/Space activation, and an absence-only pane's
  note is a focus stop so the pane is never pointer-only.
- **Every focusable item shows a ring.** Rows, sidebar rows, search result
  rows, buttons, switches, sliders, selects, disclosure/help controls, the
  search clear button, scroll views, and dialog/sheet controls use
  `FocusRing`. The result list rings its current row while the list owns
  focus rather than relying on the accent selection fill.
- **Focus is restored.** Pane and sub-pane changes focus the first keyboard
  stop in the new body; search activation lands on the row's control;
  Back/Forward restore the row and focus; closing a `Dialog` or `Sheet`
  returns focus to the control that opened it. Alt+Left/Right are never
  consumed by a scroll view as a horizontal nudge.
- **AT-SPI is named.** The shell, sidebar, search results, rows, controls,
  popups, dialogs, sheets, debug view, and absence notices expose role,
  name, description, and current value/range where the control has one. A
  row's accessible description includes its help and inline validation
  error. A newly visible validation error or absence note is announced
  once; a no-match or changed result count is announced once per query.
- **The debug view is keyboard reachable.** Pane & Row Owners is a Dialog
  with a named pane selector, a named row list, and one accessible list item
  per row carrying owner/key/absence/state.

### 3.3.3 Adaptive presentation and scale (T267)

The Settings app mirrors the same `accessibility.*` keys the shell uses; it
does not invent a second policy. `SettingsShell` binds them to `Theme` and
`Glass`, so the open window follows a key change without a restart:

- `accessibility.reduceMotion` sets `Theme.reducedMotion`; every motion token
  duration collapses to 0 while the state change stays legible.
- `accessibility.reduceTransparency` sets `Glass.reduceTransparency`; the
  material resolver takes the opaque accessibility plate.
- `accessibility.increaseContrast` sets `Glass.increaseContrast`; the
  material resolver boosts edges and the shared Settings components raise
  secondary/tertiary ink and separators to the primary ink.

RTL and pseudo-locale behavior:

- The shell sets `LayoutMirroring.enabled` from the application layout
  direction and inherits it to the tree, so the sidebar moves to the right,
  the detail pane to the left, and row label/control slots swap sides.
- Text alignment is set explicitly (Qt does not mirror it): sidebar labels,
  search field text, result labels/subtitles, row labels/descriptions,
  group titles, and the debug/absence notes right-align in RTL. The
  disclosure chevron points left. Control order inside a row is not
  reversed; only the label/control sides are.
- The long-string sweep covers the sidebar (section and pane labels), the
  titlebar title, search result labels/subtitles, rows and groups, dialogs,
  and both About surfaces. Long labels elide, descriptions wrap, and no
  control is pushed outside its row.

Scale:

- The same QML is tested at 1×, 1.5×, and 2× (`tst_settings_scale_*`) at
  740×820 and 720×480. The cases assert the device pixel ratio, the
  sidebar/detail split, the search field and titlebar bounds, every
  Appearance row's label/control non-overlap, and the scrollbar/resize-corner
  inset. They are not a screenshot scaled after the fact.
- The headless capture rail writes baseline, Increase Contrast, Reduce
  Transparency, Reduce Motion, RTL, and 2× stills under the gitignored
  `docs/captures/`.

Remaining hard limits are explicit `OPEN` rows:

| Limit | Why it stays open | Owner / rerun |
|---|---|---|
| A third-party AT-SPI application that does not expose a control | We cannot synthesize a role/name/value the application never exports; the shell can only name its own surfaces | T252 / a live AT-SPI audit |
| Offscreen 2× PNG is clipped to logical size by Qt's `grabImage` | The 2× QML case asserts `Screen.devicePixelRatio == 2` and the logical layout; a physical 2× still needs a host Wayland session | T283 / live capture rail |
| Orca/speech-dispatcher integration | Out of this task's scope; the shell exposes the semantics but does not own speech | T252 |
| Lock/screen-saver accessibility | Those surfaces keep their own a11y path; Settings does not wrap them | T252 / T278 |
| A hardware keyboard-layout or screen-reader-specific quirk | The headless tests cover the map and AT-SPI tree, not a specific host input stack | T284 real-host matrix |

### 3.4 Absence and empty states

A missing provider is a supported state, not an empty page. Every pane keeps
its section structure and follows one of five kinds:

| Kind | Behaviour |
|---|---|
| `live-on-defaults` | settingsd-backed controls stay visible and interactive on schema defaults; writes are session-only while the daemon is absent |
| `disabled-with-reason` | controls stay visible and inert; a one-line reason names the missing service and the rerun/install path |
| `hidden` | the row does not exist on this host; the pane still renders its other sections |
| `read-only` | values are observable but not writable; the reason says who owns the write |
| `delegated` | the row opens the third-party service's own UI; absent service shows a note and disables only the hand-off |

Rules:

- A pane never collapses to a single note. If the host adapter is absent,
  keep the pane header, the section titles, the settingsd-backed rows, and
  the disabled host rows with their reasons.
- A pane with two providers (for example General's updates and host-stack
  adapters) renders each half independently. One absent half does not hide
  the other.
- A missing settingsd is not an error banner; it is `live-on-defaults` with
  deferred writes. When the daemon returns, `GetAll` reconciles.
- A present-but-unreadable service is `Error`: visible, inert, and named. It
  never leaks into a neighbouring section.
- “No hardware” (no battery, no fingerprint reader, no trackpad, no
  backlight, no controller) is a visible empty state with an explanation,
  not a hidden pane.

### 3.5 Search and discovery

Search is the fastest path to a setting and must never lie:

- Index every shipped pane title, id, section, row label, synonym, help text,
  owner, key, and absence kind. Index pane ids and colloquial names:
  `wifi`, `wi-fi`, `control center`, `mission control`, `time machine`,
  `touch id`, `internet accounts`, `autofill`. The renamed panes keep their
  reference names as aliases (`control center` → Quick Settings, `mission
  control` → Overview, `internet accounts` → Online Accounts).
- Normalize hyphens and whitespace to one form *and* keep a hyphenless form,
  so `wi-fi`, `wifi`, and `Wi Fi` are the same query. The normalized and
  compact fields are precomputed in the index, not per keystroke.
- Rank exact label, label prefix, word start, substring, synonyms, pane
  description/section, pane title, then help text. A pane whose own name
  matches sorts first. Owner/key/absence/control terms are a separate,
  lower band: they stay searchable but never outrank a user-word match.
- Group results by pane with a per-pane cap and a “Show all” path. Show the
  pane header and the section name. Do not flood the list with every row of a
  matching pane; a capped group ends with a `Show all` row that expands in
  place. The list carries a result count.
- A result must be a live target. Do not index a `planned` or `omitted` row.
  A row inside a General sub-pane, a dialog, or a sheet is indexed only if
  the deep-link resolver can open that surface and land on the row.
- Use a truthful subtitle: the pane description (or its sidebar group) or the
  row's section, never the pane's absence note when the pane is available.
- Keyboard: Down from the search field enters the list; Up/Down/Home/End/
  Return/Escape work; Escape returns focus to the field and then the
  sidebar; the empty state is announced.
- Search is synchronous and local. Precompute the normalized index once;
  debounce keystrokes; do not rebuild the sidebar model on every character.
  A non-empty query replaces the sidebar list; an empty query restores it.
- AT-SPI: the results are one list whose items name their pane and section;
  the search clear affordance has its own accessible name.


### 3.6 Performance and live state

- Cold open: no more than one frame of work before the first pane paints.
  Adapter clients are created lazily per pane, not all 15 in the bridge
  constructor.
- Pane switch: within two frames at 60 Hz on the reference machine. The
  pane loader caches the last pane; switching back does not re-run
  `Component.onCompleted` side effects.
- Slider drag: one settingsd `Set` per commit, not per pixel. Use
  commit-on-release or a bounded debounce. The daemon coalesces its
  persistence write; a drag never produces one fsync per tick.
- Adapter reads: an open pane reflects current host state. `Refresh` is
  called on open and after a write; `State` is not the only read path.
- Adapter writes: parse the outcome report, surface a row-level error, and
  revert the control on `denied`, `failed`, or `absent`. No silent failure.
- Wallpaper/provider reads are asynchronous. A hung provider cannot block the
  UI thread.
- The shell and Settings share one settingsd client and one schema mirror.
  A cross-language test compares the client mirror with the daemon schema;
  drift fails the build.

**Landed (T268, [ADR 0268](adr/0268-settings-performance-lazy-clients-and-pane-cache.md)).**
The bridge constructs no adapter client and makes no adapter read. Each pane's
first `refresh*()` creates only its own client (`Settings.adapterClientCount`
is the diagnostic; zero after construction). `Settings.values`,
`Settings.writeErrors`, and `Settings.writeStates` are per-key property maps, so
a settingsd event re-evaluates only the bindings on that key. The 142-key
defaults/constraint/schema-info tables are built once. Sliders write on
`committed` (the Appearance transparency slider keeps its 60 ms bounded
debounce); the wallpaper provider is read with one asynchronous
`Properties.GetAll` and concurrent reads coalesce. `PaneLoader` keeps each
visited pane instantiated in a hidden holder, so returning to a pane does not
re-run its `Component.onCompleted` refreshes; a pane that needs fresh host state
asks for it explicitly.

The budget is a harness floor, not a hardware claim. Measure it with:

```sh
make t268-settings-performance
```

It writes `docs/captures/t268-settings-performance.json` (raw samples and
percentiles), `.log` (the full run), `-trace.txt` (the `DFTRACE` startup and
`paneLoaded cold|cached` lines), and `.txt` (the note below). The offscreen
software harness at 2026-11-15 recorded: cold open 729 ms; first pane visit
p50 57 / p95 68 / p99 68 ms; cached pane switch p50 8 / p95 10 / p99 19 ms;
0 missed deadlines.
The test fails above 2500 ms cold open, 1500 ms first-visit p95, or 150 ms
cached-switch p95. The reference-machine target for a cached switch remains two
60 Hz frames (33.3 ms); the harness budget is wider because the software scene
graph is slower than the GPU path. The live Wayland walk (open Settings, switch
five panes, drag Dock size) remains the human confirmation.


## 4. Pane-by-pane target

The tables below are the target row inventory. “Disposition” uses the
settings-map vocabulary: **ship**, **adapt**, **delegate**, **omit**. A row
marked `stored-only` is a deliberate durable preference with no live
consumer yet; it must say so in the schema and map, and the UI must not call
it live.

### 4.1 Wi-Fi — T271

Owner: NetworkManager. Absence: keep the pane, every section (Current
Network, Known Networks, Other Networks, Personal Hotspot), the Ask-to-Join
rows, and Advanced; disable the radio/join/list/hotspot controls with a
reason. T271 implements this structurally: no group disappears.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Wi-Fi | Wi-Fi | Toggle | NM `WirelessEnabled` | ship |
| Current network / Details | Current network | Info + Details… | NM `ActiveConnection` | ship; the sheet carries security/band/signal/address/last-used/auto-join plus Forget/Disconnect |
| Personal Hotspot | Personal Hotspot | Toggle + Create… | NM `802-11-wireless.mode=ap` | ship; a real `AddAndActivateConnection`, disabled with a reason when no Wi-Fi device exists |
| Known Networks | Known networks | List + ⋯ | NM `Settings.Connection` | ship; auto-join, details, forget, disconnect |
| Other Networks | Other networks | List + scanning state | NM `AccessPoint` | ship; Scan requests a real NM scan; scanning/empty/error rows |
| Other… | Join hidden network… | Dialog | NM `AddAndActivateConnection` | ship |
| Ask to join networks | Ask to join networks | Popup | `network.wifi.askToJoin` | **stored-only**; the shell has no prompt consumer yet, and the pane says so |
| Ask to join hotspots | Ask to join hotspots | Popup | `network.wifi.askToJoinHotspots` | **stored-only**, same reason |
| Advanced… | Advanced… | Sheet | NM connection settings | ship; live read of metered, IPv6, DNS, proxy, MAC randomization, 802.1X |
| — | Quick sheet | Panel | shell | ship |
| — | MAC randomization, metered, IPv6, DNS, proxy, 802.1X | Advanced sheet | NM | ship as read-only live rows; the full editor owns writes |

### 4.2 Bluetooth — T271

Owner: BlueZ. Absence: keep the Bluetooth section, Discoverable As, My
Devices, Nearby Devices, and Advanced with an empty state; disable
power/discovery only. T271 implements this structurally.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Bluetooth | Bluetooth | Toggle | BlueZ `Adapter1.Powered` | ship; turning on starts discovery while the pane is open |
| Discoverable as… | Discoverable As | Info | BlueZ `Adapter1.Discoverable` | ship; the computer name, never the adapter id |
| My Devices | My Devices | List + info | BlueZ `Device1` | ship; the info sheet carries address/type/battery/connect/disconnect/forget |
| Nearby Devices | Nearby Devices | List | BlueZ discovery | ship; scanning/empty/error |
| — | Advanced… | Sheet | BlueZ + `bluetooth.*` | ship; adapter name, address, discoverable, pairable |
| — | Quick sheet | Panel | shell | ship |
| — | LE Audio / battery level | Info | BlueZ `Battery1` / PipeWire | ship the `Battery1.Percentage` row; LE Audio where exposed |

### 4.3 Network — T272

Owner: NetworkManager + firewall adapter. Absence: per-service rows show
`Unavailable`; the firewall keeps its own state; no write.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Services | Services | List + status dot | NM `Devices`/`ActiveConnection` | ship; green/yellow/red/gray dot |
| VPN | VPN | Disclosure / top-level optional | NM VPN | ship; native WireGuard is a connection type, plugin Add only when the editor can finish it; the optional top-level pane appears when a profile exists |
| Firewall | Firewall | Toggle + Options | firewall adapter | ship; firewalld/ufw status unprivileged where the host exposes it, nftables/iptables read-only/delegate; independent of NM |
| Other Services | Other services | List | NM devices | ship |
| Options (⋯) | Service actions | Menu | NM | ship; activate/deactivate, priority, make inactive, delete, add |
| Advanced… | Details… | Sheet | NM | ship; TCP/IP, DNS, search domains, proxies, hardware, IPv6, metered, MAC randomization, 802.1X |
| Locations | Profiles | Popup/sheet | NM `Settings.Connection` | adapt |
| — | Firewall options | Sheet | firewall adapter | ship; open ports, services, zones/rules, backend |
| — | Add service… | Sheet | NM `AddConnection` | ship; Ethernet, Wi-Fi, bridge, bond, VLAN (VPN is added from the VPN section so it can be finished) |

**Landed (T272).** The Network pane is the Tahoe service list:

- Each service row carries a status dot (green connected, yellow
  connecting/disconnecting, red failed, gray idle/unavailable) and a
  per-service **Details** sheet. The sheet reads the live TCP/IP address,
  subnet mask, router, DNS servers, search domains, IPv6 method/addresses,
  proxy, MAC randomization, metered state, 802.1X key management, hardware
  address, interface, and auto-connect. The single text-wall Advanced dialog
  is gone; DNS/proxy/MAC/metered/IPv6/802.1X remain read-only live rows
  because the full connection editor owns their writes.
- The service actions menu owns Connect/Disconnect, Move Up/Down, Make
  Inactive, Delete Service, and Add Service. Service order is still
  NetworkManager's `autoconnect-priority`; the host re-reads and pushes the
  new order.
- The firewall is independent. A missing NetworkManager leaves the Firewall
  section live (its toggle, options, and rules work), and a read-only VPN
  adapter never disables it. A missing firewall shows “No firewall detected”;
  a manager that cannot report status (nftables) shows Unavailable, never a
  fake Off.
- The optional top-level **VPN** pane appears in Connectivity when
  NetworkManager reports a profile. It lists connections, connects and
  disconnects, toggles auto-connect, shows an Info sheet, and adds a
  WireGuard/OpenVPN profile. When no profile exists the pane leaves the
  sidebar and Network → VPN remains the entry point.
- **Add Service** offers Ethernet, Wi-Fi, bridge, bond, and VLAN with
  labelled fields and inline validation; a failed add reports its reason and
  creates no row. VPN is added from the VPN section so it can be completed.
  Every Network/VPN write reports `applied`, `denied`, `failed`, or `absent`;
  a failed write leaves the view untouched and shows the reason.
- The sections stay visible when NetworkManager, VPN, or the firewall is
  absent. Only the affected controls disable, each with a reason.

**Landed (T272a).** The three writes that cross a distro boundary are honest:

- **Native WireGuard.** Plugin discovery lists only installed
  `libnm-vpn-plugin-*.so` packages; `org.freedesktop.NetworkManager.wireguard`
  is never a plugin and no `vpn`/WireGuard service-type profile is written.
  The Add form's WireGuard entry creates a native `wireguard` connection from
  the private key, local address, and one peer (public key + endpoint), with
  optional DNS/MTU. The result is connectable immediately.
- **No dead plugin profiles.** A plugin entry is offered only when the plugin
  is installed and `nm-connection-editor` exists. Add creates the profile and
  hands it to the editor; the row reads "Needs setup", its connect/auto
  controls are disabled, and "Complete Setup…" reopens the editor. A host with
  no plugin, no native WireGuard, and no editor shows the install/delegate
  reason and no Add control.
- **Firewall reads without passwordless sudo.** ufw falls back to
  `systemctl show -p ActiveState --value ufw` and `/etc/ufw/ufw.conf` when
  `ufw status` is refused, so the On/Off toggle works for a normal user and
  the rules row names `sudo ufw status verbose`. nftables reads
  `/etc/nftables.conf` when `nft list ruleset` is refused and stays
  read-only/delegate. An iptables-only host reports "iptables rules present;
  no supported manager" (or that the rules cannot be read unprivileged) with
  `sudo iptables -S`; only a host with no manager says "No firewall detected".
- **Write auth is explicit.** Firewall writes keep the `pkexec` fallback and
  the denial/failure outcomes; the options sheet says each change may prompt.
  A persistent polkit helper is a separate decision.

**OPEN (T272a).** The live host walk is still human evidence, not a backend
gap: add and complete an OpenVPN profile on a host with the plugin, create and
connect a native WireGuard profile on a host with the kernel module, and open
Network as a normal user on Ubuntu and an nftables host. Rerun the headless
evidence with `make t272a-network-capability-capture`; on the host inspect with
`nmcli connection show`, `ufw status verbose`, `nft list ruleset`, and
`iptables -S`. No backend is left `OPEN`: ufw/nftables/iptables rules that need
privilege are delegated with their exact command by design.

Tests: `tst_settings_network`, `tst_settings_vpn`, `tst_settings_absence`;
`make t272-network-capture` writes the services/details/firewall/VPN/
add-service/absence stills in light/dark, and
`make t272a-network-capability-capture` writes the firewall backend and VPN
capability states. The live Wayland walk (details, reorder, firewall with NM
stopped, add a VPN, stop each adapter) is T284's.


### 4.4 Battery — T273

Owner: UPower + power-profiles-daemon/logind. Absence: keep the section
structure and settingsd options; disable host reads.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Low Power Mode | Power mode | Popup | `power.profile` | ship |
| Battery Health | Battery health | Info | UPower | ship; `Unknown` when absent |
| Usage History | Usage history | Segmented 24 h/10 d | sampler | adapt; honest “sampling not available” if no history |
| Options… | Options | Sheet | `power.*` | ship; dim on battery, prevent sleep, wake for network |
| Battery Level | Battery level | Chart | UPower | ship |
| Screen On Usage | Screen-on usage | Chart | session sampler | adapt |
| — | Show percentage | Toggle | `power.showPercentage` | ship; menu-bar consumer |
| — | Power settings | Rows | `idle.*` | ship; link to Lock Screen |
| — | Power adapter / charge threshold | Info | UPower | adapt (Linux-only) |

### 4.5 Storage — T273 (General → Storage)

Owner: UDisks2 + filesystem usage. Absence: keep the chart and the settings
rows; disable volume actions.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Storage bar chart | Storage chart | Chart | `storage.usage` | ship; total/used/free |
| All Volumes | Volumes | List | UDisks2 | ship; capacity, mount, eject |
| Recommendations | Recommendations | List | storage adapter | ship; empty Trash, remove old downloads |
| Manage… | Manage storage… | Button | Files/disk utility | delegate |
| Categories | Categories | Chart/list | UDisks2 + `df` | ship; Apps, Documents, System, Other Users, Free |
| — | Removable media | List | UDisks2 | ship |
| — | Snapshots / SMART | Info | btrfs/UDisks2 | adapt where available |

### 4.6 General — T269/T270

Owner: settingsd + host/distro adapters. Absence: each sub-pane keeps its
own state; one absent half does not hide the list.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| About | About | Disclosure | host info | ship |
| Software Update | Software Update | Disclosure | PackageKit/native provider | ship |
| Storage | Storage | Disclosure | UDisks2 + `df` | ship |
| Autofill & Passwords | Passwords & autofill | Disclosure | Secret Service/portal | adapt/delegate |
| Date & Time | Date & Time | Disclosure | `timedatectl` + settingsd | ship |
| Language & Region | Language & Region | Disclosure | `localectl` + settingsd | ship |
| Login Items & Extensions | Login Items & Extensions | Disclosure | XDG autostart + systemd + portals | ship |
| Sharing | Sharing | Disclosure | Samba/NFS/CUPS/SSH/portal | ship/adapt |
| Startup Disk | Startup Disk | Disclosure | UDisks2/bootloader | adapt/read-only |
| Time Machine | Backup | Disclosure | restic/Pika/Timeshift | adapt/delegate |
| Transfer or Reset | Transfer or Reset | Disclosure | settingsd reset + delegates | adapt |
| Device Management | — | Omit | — | omit: no MDM |
| AppleCare & Warranty | — | Omit | — | omit: no service |
| AirDrop & Handoff | — | Omit | — | omit: no permissive drop-in |

**Landed (T269).** The list never collapses: About and Software Update read
the host identity, the distribution provider is a second, independently
optional half, and every row's summary says `Unavailable` only for its own
missing owner. Every row opens an in-place sub-pane; Software Update is no
longer a dialog, Storage keeps `settings://pane/storage` as an alias, and
Autofill & Passwords inventories installed Secret Service providers without
storing a secret. Transfer or Reset is the last row; its content is T270.
About shows the model, processor, memory, graphics, serial, OS, real display
name/resolution, and volume/free/total, with a real System Report sheet.

#### 4.6.1 About — T269

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Device illustration | Device illustration | Art | design system | ship; original artwork |
| Device name | Device name | Text | hostname/distro | ship |
| Model / year | Model | Info | DMI | adapt; “Not available” when unreadable |
| Processor / Memory / Graphics | Processor / Memory / Graphics | Info | host info | ship |
| Serial | Serial | Info | DMI | ship; hidden when unreadable |
| macOS | OS / kernel / architecture | Info | host info | adapt |
| Displays | Display name + resolution | Info | compositor | ship |
| Storage | Volume + free/total | Info | UDisks2/`df` | ship |
| System Report | System report | Button | host info | ship; real report sheet |
| Regulatory Certification | Regulatory info | Link | distro/legal | delegate |

#### 4.6.2 Software Update — T269

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Available updates | Available updates | List | PackageKit/native | ship |
| Update Now / Tonight | Update Now | Button | provider | adapt; the provider offers no scheduled install, so “Tonight” is omitted |
| Automatic updates | Automatic updates | Toggle | `updates.auto` | ship; provider consumer |
| Install macOS updates | Install automatically | Toggle | `updates.autoInstall` | ship |
| Security responses | Security updates | Info | provider | adapt; the available security count, no separate provider key |
| Beta updates | — | Omit | — | omit: no beta channel |
| Last checked / progress | Last checked / progress | Info | provider | ship |

#### 4.6.3 Autofill & Passwords — T269

Owner: Secret Service/portal. Absence: provider list with a note; no secret
storage in Settings.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Open Passwords | Open password manager | Button | Secret Service/portal | delegate |
| Autofill passwords and passkeys | Autofill passwords | Info | `defaults.passwordManager` | adapt; delegation only, no credential store |
| Autofill from | Autofill source | Popup | `defaults.passwordManager` | ship |
| Verification codes | Keyring | Info | Secret Service | adapt; locked/unlocked when reported |
| — | Provider list | List | Secret Service | ship; read-only inventory |

#### 4.6.4 Date & Time — T270

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Set date and time automatically | Set automatically | Toggle | `timedatectl set-ntp` | ship |
| Time zone | Time zone | Searchable picker | `timedatectl` | ship |
| Set time zone automatically | Set time zone automatically | Toggle | geoclue adapter | adapt |
| Date and time | Date and time | Fields | `timedatectl` | ship; auth, disabled while NTP |
| 24-hour time | 24-hour time | Toggle | `clock.use24Hour` | ship |
| Show seconds | Show seconds | Toggle | `menu.clock.showSeconds` | ship |
| Show date | Show date | Toggle | `menu.clock.showDate` | ship |
| Clock options… | Clock options… | Sheet | `menu.clock.*` | ship |

#### 4.6.5 Language & Region — T270

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Preferred languages | Languages | Ordered list + add | `localectl`/locale adapter | ship |
| Region | Region | Popup | locale adapter | ship |
| Calendar | Calendar | Popup | `locale.calendar` | ship |
| Temperature | Temperature | Popup | `locale.temperature` | ship |
| Measurement system | Measurement | Popup | `locale.measurement` | ship |
| Date format | Date format | Popup | `locale.dateFormat` | ship |
| Number format | Number format | Popup | `locale.numberFormat` | ship |
| First day of week | First day of week | Popup | `locale.firstDay` | ship |
| Live Text | — | Omit | — | omit: no desktop consumer |
| Applications / Translation | — | Omit | — | omit: no per-app language store |

#### 4.6.6 Login Items & Extensions — T270

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Open at Login | Open at login | List + add/remove | XDG autostart | ship |
| Allow in Background | Allow in background | List + toggle | systemd user units | ship |
| Extensions | Extensions | List | portal extensions | delegate |
| — | Add login item… | Button | autostart adapter | ship |
| — | Broken item warning | Info | autostart adapter | ship |

#### 4.6.7 Sharing — T270

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| File Sharing | File sharing | Toggle + options | Samba/NFS | ship |
| Media Sharing | Media sharing | Toggle | DLNA | adapt |
| Screen Sharing | Screen sharing | Toggle | portal/RDP/VNC | delegate |
| Content Caching | — | Omit | — | omit: no equivalent |
| Bluetooth Sharing | Bluetooth sharing | Toggle | BlueZ | adapt |
| Printer Sharing | Printer sharing | Toggle | CUPS | ship |
| Internet Sharing | Internet sharing | Toggle | NM | ship |
| Remote Management | Remote management | Toggle | cockpit | adapt |
| Remote Login | Remote login | Toggle | OpenSSH | ship |
| Local hostname | Local hostname | Text + Edit | hostname adapter | ship |

#### 4.6.8 Startup Disk / Backup / Transfer or Reset — T270

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Startup Disk | Startup disk | List + Restart | UDisks2/bootloader | adapt/read-only |
| Time Machine | Backup | Toggle + destination + schedule | restic/Pika/Timeshift | adapt/delegate |
| Back Up Now | Back up now | Button | backup adapter | ship |
| Restore… | Restore… | Button | backup adapter | delegate |
| Transfer or Reset | Transfer or reset | Disclosure | settingsd + delegates | adapt |
| Erase All Content | — | Omit | — | omit: not a Linux concept |
| Migration Assistant | — | Delegate | distro tool | adapt where available |

**Landed (T270).** The seven remaining General sub-panes now match the row
inventory and keep their structure when their host half is absent:

- **Date & Time** replaces the raw IANA `Select` with a searchable
  `SearchablePicker` (a design-system component shared with Language & Region).
  The manual field is prefilled from the host clock, validates
  `YYYY-MM-DD HH:MM:SS` including a calendar round-trip, and disables while NTP
  is on. Every host write reports `busy`/`applied`/`denied`/`failed` through
  `Settings.generalWriteState`; a denial keeps its reason under the row. “Set
  time zone automatically” is a real inert row with its reason: no location
  service is installed. The clock-format rows and the live preview stay live
  when `timedatectl` is absent.
- **Language & Region** replaces the locale `Select` with an ordered preferred
  language list (add/remove/reorder), a searchable Region picker, and a preview
  that formats through the selected locale. Medium strips the long form's
  weekday so medium and long are distinct; the number preview distinguishes
  locale, decimal point, and decimal comma. The order is the `locale.languages`
  settingsd key (schema revision 29); its first entry is the locale the host is
  asked to apply, and opening the pane never writes the key. The format rows
  and preview stay live when `localectl` is absent.
- **Login Items & Extensions** uses the macOS labels, adds an installed-app
  picker (`Add Login Item…`) and a per-row Remove, warns about entries whose
  command is missing, and groups the Extensions section by the portal backend
  that serves it. All three groups stay visible with their own absence note.
- **Sharing** adds the Local hostname row with Edit and inline validation, a
  per-service Options sheet with the unit/address/port/shares/queues the host
  reports, and removes the duplicated `· On/Off` and `· Requires
  authentication` suffixes from the row description. The Services group keeps
  its title and absence note when systemd is absent.
- **Backup** adds a destination picker (the portal FileChooser in directory
  mode), an absolute-path/URL validator that gates Save, and disables Schedule
  and Back up automatically when no backend is installed. The absent note now
  names the missing host, not a missing backend; the policy rows stay live.
- **Startup Disk** lists every disk (`lsblk -J`) with the boot disk first,
  labels sizes in GiB instead of mislabelled GB, folds the duplicate Device row
  into the Startup disk description, and adds a Restart action behind a
  confirmation dialog. Changing the startup disk remains read-only.
- **Transfer or Reset** adds the Transfer section (two disabled delegate rows
  with their Linux reason), the destructive Reset with a typed `RESET`
  confirmation, and the Erase All Content section as a read-only Linux reason
  instead of a fake erase.

The row map gains `login.add`, `time.autoZone`, `sharing.hostname`,
`backup.backend`, `startup.disks`, `startup.restart`, and the omitted
`reset.erase`. `make t270-general-system-capture` writes every sub-pane plus
the host-absent state in light/dark; the live Wayland walk is T284's.

### 4.7 Appearance — T274

Owner: settingsd + compositor/design system. Absence: live on defaults.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Appearance | Appearance | Segmented | `appearance.colorScheme` | ship |
| Color | Accent color | Swatches + custom | `appearance.accent` | ship |
| Text highlight color | Text highlight color | Swatches + custom | `appearance.highlight` | ship |
| Icon & widget style | Icon & widget style | Popup/sheet | `appearance.iconStyle` | adapt; icon theme provider or disabled reason |
| Folder color | Folder color | Swatches | `appearance.folderColor` | adapt; Files consumer |
| Sidebar icon size | Sidebar icon size | Popup | `appearance.sidebarIconSize` | ship |
| Tint window background with wallpaper color | Wallpaper tinting | Toggle | `appearance.wallpaperTint` | ship |
| Show scroll bars | Scroll bars | Popup | `appearance.scrollBars` | ship |
| Click in the scroll bar to | Scroll-bar click | Popup | `appearance.scrollBarClick` | ship |
| — | Transparency | Slider | `appearance.transparency` | ship; disabled when Reduce Transparency |
| — | Motion | Link | `accessibility.reduceMotion` | ship |
| — | Icon theme / cursor theme / cursor size | Popup/slider | `appearance.iconTheme`, `appearance.cursor*` | adapt (Linux-only) |
| — | Window control layout | Popup | `appearance.windowControls` | adapt (Linux-only) |

### 4.8 Accessibility — T275

Owner: settingsd + compositor + AT-SPI + delegates. Absence: keep the
section structure; disable the delegated rows with a reason.

#### Vision

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| VoiceOver | Screen reader | Toggle + Open | Orca/AT-SPI | delegate; status + open |
| Hover Text | Hover text | Toggle + options | shell overlay | adapt |
| Display | Display | Disclosure | settingsd/compositor | ship |
| — Invert colors | Invert colors | Toggle | `accessibility.invertColors` | ship |
| — Increase contrast | Increase contrast | Toggle | `accessibility.increaseContrast` | ship |
| — Reduce transparency | Reduce transparency | Toggle | `accessibility.reduceTransparency` | ship |
| — Differentiate without color | Differentiate without color | Toggle | `accessibility.differentiateWithoutColor` | ship |
| — Show window title icons | Show title icons | Toggle | compositor | adapt |
| — Show toolbar button shapes | Toolbar button shapes | Toggle | design system | adapt |
| — Display contrast | Display contrast | Slider | compositor | adapt |
| — Text size | Text size | Popup | design system | ship |
| — Menu bar size | Menu bar size | Popup | shell | ship |
| — Pointer size / colors | Pointer size / colors | Slider/swatches | compositor | ship |
| — Color filters | Color filters | Popup/slider | compositor | adapt |
| Zoom | Magnifier | Toggle + options | compositor | ship |
| Motion | Motion | Toggle | `accessibility.reduceMotion` | ship |
| — Dim flashing lights | Dim flashing lights | Toggle | compositor | adapt |
| — Auto-play animated images | Auto-play images | Toggle | shell | adapt |
| — Prefer non-blinking cursor | Non-blinking cursor | Toggle | design system | ship |
| Read & Speak | Read & speak | Disclosure | speech-dispatcher/espeak-ng | delegate |
| Audio Descriptions | — | Omit | — | omit: content-level |

#### Hearing

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Audio | Audio | Disclosure | PipeWire/settingsd | ship |
| — Flash the screen | Flash screen on alert | Toggle | `accessibility.flashScreen` | ship |
| — Play stereo as mono | Mono audio | Toggle | `accessibility.monoAudio` | ship |
| — Background sounds | Background sounds | Toggle + picker | audio delegate | adapt |
| RTT | — | Omit | — | omit: no RTT service |
| Captions | Captions | Disclosure | live-caption delegate | delegate |
| Name Recognition | — | Omit | — | omit: no on-device name model |

#### Motor and Speech

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Voice Control | Voice control | Toggle | speech-to-text delegate | delegate |
| Keyboard | Keyboard | Disclosure | settingsd/compositor | ship |
| — Full keyboard access | Full keyboard access | Toggle | `accessibility.fullKeyboardAccess` | ship |
| — Sticky Keys | Sticky keys | Toggle + options | compositor | ship |
| — Slow Keys | Slow keys | Toggle + delay | compositor | ship |
| — Accessibility Keyboard | On-screen keyboard | Toggle | shell | delegate/adapt |
| Pointer Control | Pointer control | Disclosure | compositor | ship |
| — Double-click speed | Double-click speed | Slider | `input.doubleClickSpeed` | ship |
| — Mouse Keys | Mouse keys | Toggle | compositor | ship |
| — Ignore built-in trackpad | Ignore built-in trackpad | Toggle | compositor | adapt |
| Switch Control | Switch control | Disclosure | delegate | delegate |
| Live Speech | Live speech | Toggle | speech delegate | delegate |
| Personal Voice | — | Omit | — | omit: no equivalent |
| Vocal Shortcuts | — | Omit | — | omit: no equivalent |
| Siri | — | Omit | — | omit |
| Shortcut | Accessibility shortcut | Recorder | `accessibility.shortcut` | ship |

### 4.9 Quick Settings — T277

Owner: settingsd + shell `ControlCenterPolicy` + registry. Absence: live on
defaults; unavailable modules show a reason and are not addable.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Control Center modules | Modules | Categorized grid/list | `quickSettings.modules` | ship |
| — | Pages | List + add/rename/delete | `quickSettings.page*` | ship |
| — | Control sizes/spans | Menu | `quickSettings.spans` | ship |
| — | Add controls… | Sheet | registry + settingsd | ship |
| Wi-Fi / Bluetooth / Focus / Sound / Display / Battery / Dark Mode / Stage / Screen Recording / Keyboard Brightness / Hearing | Same, with Linux names | Checkbox/popup | `menu.control.*` | ship/adapt |
| Menu Bar Only | Menu bar items | List | `quickSettings.menuBarOnly` | ship |
| Other Modules | Other modules | List | registry | ship |
| — | Per-display | Toggle | `quickSettings.perDisplay` | adapt; disabled with reason until per-display surfaces |
| — | Reset Control Center | Button | `quickSettings.*` + `menu.control.*` | ship |
| — | Live preview | Panel | shell | ship |

### 4.10 Menu Bar — T277

Owner: settingsd → shell. Absence: live on defaults.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Automatically hide and show | Auto-hide menu bar | Popup | `menu.autoHide` | ship |
| Show menu bar background | Show background | Toggle | `menu.showBackground` | ship |
| Recent documents, applications, and servers | Recent items | Popup/stepper | `menu.recentItems` | ship |
| Menu Bar Controls | Menu bar controls | List + per-control options | `menu.control.*` | ship |
| Allow in the Menu Bar | Third-party items | List | StatusNotifier registry | ship |
| Add Controls… | Add controls… | Sheet | registry | ship |
| Reset Control Center | Reset menu bar | Button | `menu.control.*` | ship |
| Clock | Clock | Checkbox + options | `menu.clock.*` | ship |
| Spotlight | Search | Checkbox | `menu.control.search` | ship |
| Wi-Fi / Bluetooth / Battery / Focus / Screen Mirroring / Display / Sound | Same | Checkbox + options | `menu.control.*` | ship |
| — | Overflow policy | Popup | shell | adapt (Linux-only) |
| — | Workspace indicator | Checkbox | `menu.control.workspaces` | adapt (Linux-only) |
| — | Global app menu | Toggle | `menu.global` | ship (Linux-only) |

### 4.11 Search — T265

Owner: search service + app-index + settingsd. Absence: preferences stay
live; the indexing rows show the service state.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Results from Apps | Results from apps | Toggles | `search.categories` | ship |
| Results from System | Results from system | Toggles | `search.categories` | ship |
| Clipboard Search | Clipboard search | Toggle | `search.categories` | adapt |
| Search Privacy | Search privacy | Folder list + add | `search.indexLocations` | ship |
| Show Related Content | — | Omit | — | omit: no content partner |
| Keyboard shortcut | Shortcut | Recorder | `search.shortcut` | ship |
| Web search | Web search | Toggle | `search.web` | ship |
| Show Search in menu bar | Show in menu bar | Toggle | `menu.control.search` | ship |
| Reset Quick Keys | Reset quick keys | Button | `search.*` | adapt |
| Delete Search History | Clear search history | Button | search service | adapt |
| Help Apple Improve Search | — | Omit | — | omit: no vendor telemetry |
| Indexing status | Indexing status | Info | search service | ship |
| Rebuild index | Rebuild index | Button | search service | ship |

### 4.12 Privacy & Security — T279

Owner: portals + polkit + LUKS + firewall. Absence: keep the category list
with a note; disable per-app writes.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Location Services | Location | Master + per-app | portal Location | ship |
| Contacts / Calendars / Reminders / Photos | Same | Disclosure | portal/EDS | ship/adapt |
| Camera / Microphone | Camera / Microphone | Disclosure | portal | ship |
| Speech Recognition | Speech recognition | Disclosure | portal | adapt |
| Accessibility | Accessibility | Disclosure | AT-SPI/portal | ship |
| Full Disk Access | Full disk access | Disclosure | portal/polkit | adapt |
| Files and Folders | Files and folders | Disclosure | document portal | ship |
| Screen & System Audio Recording | Screen recording | Disclosure | ScreenCast portal | ship |
| Automation | Automation | Disclosure | D-Bus policy | adapt |
| App Management | App management | Disclosure | package manager/portal | adapt |
| Developer Tools | Developer tools | Disclosure | portal | adapt |
| Sensitive Content Warning | — | Omit | — | omit: no on-device model |
| Blocked Contacts | — | Omit | — | omit: no system contacts |
| Analytics & Improvements | Analytics | Toggle | opt-in telemetry | adapt; default off |
| Apple Advertising | — | Omit | — | omit |
| Allow applications from | Software sources | Popup | Flatpak/distro | ship |
| FileVault | Disk encryption | Info + Open | LUKS | read-only + delegate |
| Accessories | USB accessories | Popup | USBGuard | adapt |
| Lockdown Mode | Hardening mode | Info | SELinux/AppArmor/Secure Boot | read-only |
| Extensions | Extensions | List | portal extensions | delegate |
| Advanced… | Advanced… | Sheet | polkit/firewall/secure boot | ship |

### 4.13 Biometrics & Password — T279

Owner: fprintd + PAM + accountsservice. Absence: keep both halves
independently.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Change Password | Change password | Button | PAM/accountsservice | ship |
| Touch ID / Add Fingerprint | Fingerprints | List + add/delete | fprintd | ship |
| Use Touch ID to unlock | Use fingerprint to unlock | Toggle | PAM + `biometrics.unlock` | ship |
| Use Touch ID for purchases | — | Omit | — | omit |
| Use Touch ID for autofill | Use fingerprint for authentication | Toggle | PAM | adapt |
| Use Touch ID for fast user switching | Fingerprint for user switching | Toggle | PAM/display manager | adapt |
| Use Apple Watch | — | Omit | — | omit |

### 4.14 Users & Groups — T279

Owner: accountsservice + distro adapter + polkit. Absence: read-only list;
keep Add/Edit disabled with reasons.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Users | Users | List | accountsservice | ship |
| Add User | Add user | Sheet | `useradd`/polkit | ship; password step |
| Groups | Groups | List | group provider | ship |
| Add Group | Add group | Sheet | `groupadd`/polkit | ship |
| Automatically log in as | Automatic login | Popup | display-manager adapter | ship |
| Network account server | Directory account | Button | SSSD/realm | adapt/delegate |
| Guest User | Guest user | Toggle | distro adapter | adapt; hidden with reason |
| — | Admin group | Info | `wheel`/`sudo` | ship |
| — | Change/reset password | Button | accountsservice/polkit | ship |
| — | Delete user/group | Confirm | distro adapter | ship |

### 4.15 Online Accounts — T279

Owner: GNOME Online Accounts if present; otherwise a visible delegate state.
Absence: the pane stays in the sidebar with an explicit note.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Internet Accounts | Internet accounts | List | GOA | ship/adapt |
| Add Account… | Add account… | Sheet | GOA | delegate; OAuth in provider |
| Mail/Contacts/Calendars/Notes | Sync toggles | Toggles | GOA | ship |
| Details | Details | Sheet | GOA | ship |
| — | Remove account | Button + confirm | GOA | ship |
| — | Sign in again | Button | GOA | adapt |

### 4.16 Printers & Scanners — T280

Owner: CUPS + SANE. Absence: keep the sections; disable add/queue writes.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Default printer | Default printer | Popup | CUPS | ship |
| Default paper size | Default paper size | Popup | `printers.defaultPaperSize` | ship |
| Printers | Printers | List | CUPS | ship |
| Add Printer, Scanner, or Fax… | Add printer or scanner… | Button | CUPS/SANE | ship |
| Options & Supplies… | Options & supplies… | Sheet | CUPS | ship |
| — | Remove printer | Button + confirm | CUPS | ship |
| — | Print test page | Button | CUPS | ship |
| — | Share this printer | Toggle | CUPS | ship |
| — | Scanners | List + Open scanner | SANE | delegate |

### 4.17 Keyboard — T280

Owner: settingsd + compositor + ibus/fcitx5. Absence: preferences stay
live; hardware inventory shows a reason.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Key repeat / Delay until repeat | Key repeat / Repeat delay | Slider | `input.repeatRate`/`input.repeatDelay` | ship |
| Adjust keyboard brightness in low light | Auto keyboard brightness | Toggle | `input.adjustBrightnessLowLight` | adapt; hardware bridge |
| Keyboard brightness | Keyboard brightness | Slider | `input.keyboardBrightness` | adapt; hardware bridge |
| Turn backlight off after inactivity | Backlight off after | Popup | `input.backlightOffAfter` | adapt |
| Press fn key to / Press Super key to | Press Super key to | Popup | `input.emojiKeyAction` | adapt |
| Keyboard navigation | Keyboard navigation | Toggle | `input.keyboardNavigation` | ship; compositor consumer |
| Keyboard Shortcuts… | Keyboard shortcuts… | Sheet | shortcut engine | ship |
| Input Sources | Input sources | List + add | `input.sources` | ship |
| Text Replacements… | Text replacements… | Sheet | `input.replacements` | ship; IME consumer |
| Dictation | Dictation | Toggle + language/mic/shortcut | speech-to-text delegate | adapt/delegate |
| Touch Bar | — | Omit | — | omit |

### 4.18 Mouse — T280

Owner: settingsd + compositor/libinput. Absence: preferences stay live;
inventory shows a reason.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Tracking speed | Pointer speed | Slider | `input.pointerSpeed` | ship; mouse-specific key |
| Natural scrolling | Natural scrolling | Toggle | `input.naturalScroll` | ship; mouse-specific |
| Secondary click | Secondary click | Popup | `input.secondaryClick` | ship |
| Smart zoom | Smart zoom | Toggle | `input.smartZoom` | adapt |
| More Gestures | More gestures | Disclosure | `gestures.*` | adapt |
| Pointer acceleration | Pointer acceleration | Toggle/slider | `input.pointerAcceleration` | adapt |
| — | Left-handed | Toggle | `input.leftHanded` | ship |
| — | Scroll speed / method | Slider/popup | `input.scrollSpeed`/`input.scrollMethod` | ship |

### 4.19 Trackpad — T280

Owner: settingsd + compositor/libinput. Absence: preferences stay live;
no-trackpad state explains the kept preference.

#### Point & Click

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Tracking speed | Pointer speed | Slider | `input.trackpad.pointerSpeed` | ship; trackpad-specific |
| Click | Click pressure | Slider | `input.trackpad.clickPressure` | adapt |
| Force Click and haptic feedback | Haptics | Toggle | `input.haptics` | ship |
| Look up & data detectors | Look up | Popup | `input.lookUp` | ship |
| Secondary click | Secondary click | Popup | `input.trackpad.secondaryClick` | ship |
| Tap to click | Tap to click | Toggle | `input.tapToClick` | ship |

#### Scroll & Zoom

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Natural scrolling | Natural scrolling | Toggle | `input.trackpad.naturalScroll` | ship |
| Zoom in or out | Pinch to zoom | Toggle | `input.pinchZoom` | ship |
| Smart zoom | Smart zoom | Toggle | `input.smartZoom` | ship |
| Rotate | Rotate | Toggle | `input.rotate` | adapt |

#### More Gestures

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Swipe between pages | Swipe between pages | Popup | `gestures.pages` | ship |
| Swipe between full-screen applications | Swipe between full-screen apps | Popup | `gestures.fullscreen` | ship |
| Notification Center | Notification Center | Toggle | `gestures.notifications` | ship |
| Mission Control | Overview | Popup | `gestures.overview` | ship |
| App Exposé | App windows | Popup | `gestures.appExpose` | ship |
| Show Desktop | Show desktop | Toggle | `gestures.showDesktop` | ship |
| — | Set up Bluetooth trackpad… | Button | BlueZ | ship |

### 4.20 Desktop & Dock — T276

Owner: settingsd → compositor/shell. Absence: live on defaults.

#### Dock

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Size | Size | Slider | `dock.size` | ship |
| Magnification | Magnification | Slider | `dock.magnification` | ship |
| Position on screen | Position | Popup | `dock.position` | ship |
| Minimized window animation | Minimize animation | Popup | `dock.minimizedAnimation` | ship |
| Window title bar double-click action | Title-bar double-click | Popup | `dock.titlebarDoubleClick` | ship |
| Minimize windows into application icon | Minimize into app icon | Toggle | `dock.minimizeIntoTileIcon` | ship |
| Automatically hide and show the Dock | Auto-hide Dock | Toggle | `dock.autohide` | ship |
| Animate opening applications | Animate opening apps | Toggle | `dock.animateOpening` | ship |
| Show indicators for open applications | Show running indicators | Toggle | `dock.showIndicators` | ship |
| Show suggested and recent apps | Show recent apps | Toggle | `dock.showRecentApps` | ship |
| — | Open window chooser on hover | Toggle | `dock.chooserOnHover` | ship |
| — | Bounce app icons when windows minimize | Toggle | `dock.minimizeReaction` | ship |
| — | Show application menus in menu bar | Toggle | `menu.global` | ship (Linux-only) |

#### Desktop & Stage Manager

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Show Items | Show items | Segmented | `desktop.showItems` | ship |
| Click Wallpaper to reveal desktop | Click wallpaper | Popup | `desktop.clickWallpaper` | ship |
| Stage Manager | Stage | Toggle | `stageManager.enabled` | adapt; T242 owner |
| Show recent apps in Stage Manager | Recent apps in Stage | Toggle | `stageManager.showRecentApps` | adapt; T242 owner |
| Show windows from an application | Windows from an app | Popup | `stageManager.windowMode` | adapt; T242 owner |

#### Widgets and Windows

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Show Widgets | Show widgets | Segmented | `widgets.show` | adapt; T245b owner |
| Dim widgets on desktop | Dim widgets | Popup | `widgets.dim` | adapt; T245b owner |
| Use iPhone widgets | — | Omit | — | omit |
| Prefer tabs when opening documents | Prefer tabs | Popup | `windows.preferTabs` | ship |
| Ask to keep changes when closing documents | Ask to keep changes | Toggle | `windows.askToKeepChanges` | ship |
| Close windows when quitting an application | Close windows on quit | Toggle | `windows.closeOnQuit` | ship |
| Drag windows to screen edges to tile | Drag to tile | Toggle | `tiling.dragToTile` | adapt; T242 owner |
| Drag windows to menu bar to fill screen | Drag to fill | Toggle | `tiling.dragToFill` | adapt; T242 owner |
| Hold Option while dragging to tile | Hold Super to tile | Toggle | `tiling.modifier` | adapt; T242 owner |
| Tiled windows have margins | Tiled window margins | Toggle | `tiling.margins` | adapt; T242 owner |

#### Mission Control / Overview

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Automatically rearrange Spaces | Rearrange workspaces | Toggle | `overview.autoRearrange` | ship |
| Switch to a Space with open windows | Switch to app workspace | Toggle | `overview.switchToAppSpace` | ship |
| Group windows by application | Group windows by app | Toggle | `overview.groupByApp` | ship |
| Displays have separate Spaces | Separate workspaces per display | Toggle | `overview.perDisplay` | ship |
| Drag windows to top to enter Mission Control | Drag to Overview | Toggle | `overview.dragToOpen` | ship |
| Hot Corners… | Hot corners… | Sheet | `overview.hotCorner*` | ship; modifier support |
| Shortcuts… | Shortcuts… | Sheet | shortcut engine | ship |
| — | Default web browser | Popup | `defaults.browser` + portal | ship |

### 4.21 Overview — T276

Linux-specific workspace surface. Keep the top-level id and group it next to
Desktop & Dock.

| Row | Control | Owner / key | Disposition |
|---|---|---|---|
| Swipe up to open | Toggle | `overview.swipeUp` | ship |
| Swipe between workspaces | Toggle | `overview.swipeBetween` | ship |
| Workspace count | Stepper | `workspaces.count` | ship |
| Per-display workspaces | Toggle | `overview.perDisplay` | ship |
| Hot corners | Four corner popups + modifiers | `overview.hotCorner*` | ship |
| Shortcuts | Sheet | shortcut engine | ship |
| Animation budget / reduced motion | Info | compositor | adapt |

### 4.22 Displays — T273

Owner: compositor output API + colord/display adapter. Absence: settingsd
controls stay live; output rows show `No display`.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Built-in display preview | Display preview | Art | compositor | ship; real output name |
| Display role | Use as | Popup | compositor | ship; primary/extend/mirror |
| Resolution scaling | Scale/resolution | Tiles | `display.scale` + compositor | ship |
| Brightness | Brightness | Slider | `display.brightness` | ship |
| Automatically adjust brightness | Auto-brightness | Toggle | `display.autoBrightness` | adapt; sensor |
| True Tone | Adaptive color | Toggle | `display.adaptiveColor` | adapt; hardware |
| Rotation | Rotation | Popup | `display.rotation` | ship |
| Color profile / Preset | Color profile | Popup | colord | ship/adapt |
| When connected to TV | TV mode | Popup | compositor | adapt |
| Night Shift | Night light | Disclosure | `display.nightLight.*` | ship |
| Arrange | Arrangement | Canvas | compositor | ship |
| Refresh rate | Refresh rate | Popup | compositor | ship |
| Advanced… | Advanced… | Sheet | compositor/colord | ship; HDR, VRR, overscan |
| — | Per-output configuration | List | compositor | ship (Linux-only) |

### 4.23 Wallpaper — T278

Owner: settingsd + wallpaperd + compositor. Absence: shipped default always
available; Featured empty, never an error.

Selection is a visible state machine, not a background write:

1. The first press on any part of a tile marks it selected and starts the
   tile and hero spinner in the same frame, before any provider fetch or
   settings write. The click is acknowledged locally even when the image is
   not cached.
2. The previous wallpaper stays on screen until the new image is decoded and
   ready. The background swaps once, at that point, not when the click or
   the settings write happens.
3. A second tile click supersedes the in-flight pick; clicks are never
   ignored because an apply is pending. The latest pick is the one that
   commits.
4. `wallpaper.source` is written once per committed pick. A provider error,
   decode failure, or bounded timeout reverts the tile selection and the
   hero to the previous wallpaper and shows an inline reason. No spinner
   persists and no selected tile disagrees with the actual background.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Current wallpaper | Current wallpaper | Hero preview | `wallpaper.source` | ship; large preview |
| Show on all Spaces | Show on all workspaces | Toggle | `wallpaper.showOnAllSpaces` | ship |
| Clock Appearance… | Clock appearance… | Button | `lock.clock.*` | ship |
| Screen Saver… | Screen saver… | Button | screen saver pane | ship |
| Dynamic Wallpapers | Dynamic | Grid | wallpaperd | ship |
| Landscape | Nature / Landscape | Grid | wallpaperd | ship |
| Cityscape | Cityscapes | Grid | wallpaperd | ship |
| Underwater | Underwater | Grid | wallpaperd | ship |
| Earth | Earth | Grid | wallpaperd | ship |
| Shuffle Aerials | Shuffle | Grid | wallpaperd | adapt |
| Colors | Colors | Grid | shipped colors | ship (Linux addition) |
| Pictures | Your photos | Grid | FileChooser portal | ship |
| — | Fit | Popup | `wallpaper.fit` | ship |
| — | Per-workspace wallpaper | Toggle | `wallpaper.perWorkspace` | adapt (Linux-only) |
| — | Slideshow | Popup | `wallpaper.slideshow` | adapt (Linux-only) |

### 4.24 Screen Saver — T278

Owner: shell/compositor + settingsd. Absence: live on defaults.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Screen Saver | Screen saver | Gallery | `screenSaver.style` | ship; real preview |
| Start after | Start after | Popup | `idle.blank` / `screenSaver.idle` | ship; 2 h/5 h/never |
| Show with clock | Show with clock | Toggle | `screenSaver.clock` | ship |
| Options… | Clock options… | Sheet | `screenSaver.clock.*` | ship |
| Hot Corners… | Hot corners… | Sheet | `overview.hotCorner*` | ship |
| — | Lock scene preview | Preview | shell | ship (Linux-specific) |

### 4.25 Notifications — T278

Owner: notification service + settingsd. Absence: keep global preferences;
hide only the per-app list with a note.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Show previews | Show previews | Popup | `notifications.showPreviews` | ship; service consumer |
| When display is sleeping | When display is asleep | Toggle | `notifications.showWhenSleeping` | ship; service consumer |
| When screen is locked | When locked | Toggle | `notifications.showWhenLocked` | ship; service consumer |
| When mirroring or sharing | When sharing | Popup | `notifications.showWhenMirroring` | ship; service consumer |
| Application Notifications | App notifications | List + per-app sheet | notification service | ship |
| — Alert style | Style | Popup | service `SetAppPreference` | ship |
| — Show on lock screen | Lock screen | Toggle | service | ship |
| — Show in Notification Center | Notification Center count | Popup | service | ship |
| — Badge application icon | Badge | Toggle | service | ship |
| — Play sound | Sound | Toggle | service | ship |
| — Show previews | Previews | Popup | service | ship |
| — Notification grouping | Grouping | Popup | service | ship |
| — | Focus link | Link | Focus pane | ship |

### 4.26 Sound — T280

Owner: PipeWire/WirePlumber + settingsd. Absence: keep Sound Effects;
disable device routing with a note.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Alert sound | Alert sound | Popup + Play | `sound.alertSound` | ship; preview |
| Play sound effects through | Play effects through | Popup | `sound.playEffectsThrough` | ship |
| Alert volume | Alert volume | Slider | `sound.alertVolume` | ship |
| Play sound on startup | Startup sound | Toggle | `sound.playOnStartup` | ship |
| Play user interface sound effects | UI sounds | Toggle | `sound.uiEffects` | ship |
| Play feedback when volume is changed | Volume feedback | Toggle | `sound.volumeFeedback` | ship |
| Output | Output | Tab + device table | PipeWire | ship; volume, mute, balance |
| Input | Input | Tab + device table | PipeWire | ship; volume, mute |
| — | Test | Button | PipeWire | ship |
| — | Per-app volume | List | PipeWire | adapt (Linux-only) |

### 4.27 Focus — T278

Owner: notification service + settingsd. Absence: keep schedule and
shortcut; hide the profile list with a note.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Focus list | Focus list | List | notification service | ship |
| Do Not Disturb | Do not disturb | Row | service | ship |
| Add Focus | Add focus | Sheet | service | ship |
| Delete Focus | Delete focus | Button + confirm | service | ship |
| Share across devices | — | Omit | — | omit |
| Focus status | — | Omit | — | omit: no vendor status |
| Allowed people | Allowed people | List | service `sender` hint | ship |
| Allowed apps | Allowed apps | List | service | ship |
| Set a Schedule | Schedule | Toggle + time + days | `focus.schedule.*` | ship |
| Focus Filters | Filters | List | delegate | adapt |
| Shortcuts | Shortcut | Recorder | `focus.shortcut` | ship |

### 4.28 Lock Screen — T278

Owner: settingsd + compositor/lock + logind. Absence: live on defaults.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Turn display off on battery | Display off on battery | Popup | `idle.blankBattery` | ship/adapt |
| Turn display off on power adapter | Display off on AC | Popup | `idle.blankAC` | ship/adapt |
| Require password after | Require password after | Popup | `lock.requirePasswordAfter` | ship; correct semantics |
| Show user name and photo | Show user name/photo | Toggle | `lock.showUserNameAndPhoto` | ship |
| Show password hints | Show password hints | Toggle + Set | `lock.showPasswordHints`/`lock.passwordHint` | ship |
| Show message when locked | Show message when locked | Toggle + Set | `lock.showMessageWhenLocked`/`lock.message` | ship |
| Login window shows | Login window shows | Radio | `login.windowStyle` | ship or disabled with reason |
| Show Sleep/Restart/Shut Down | Power buttons | Toggle | `lock.showPowerButtons` | ship |
| Accessibility Options | Accessibility options | Button | `accessibility.*` | ship |
| — | Lock screen preview | Preview | shell | ship |
| — | Immediate lock | Option | `lock.immediate` | ship |

### 4.29 Game Controllers — T280

Owner: udev/SDL. Absence: keep the section with an empty state; disable
mapping/test.

| macOS row | Dragonfruit row | Control | Owner / key | Disposition |
|---|---|---|---|---|
| Connected controllers | Controllers | List | udev/SDL | ship |
| Mapping | Mapping | Sheet | SDL mapping | adapt; read-only first |
| Test | Test | Dialog + close | SDL | ship |
| — | Rumble test | Button | SDL | adapt |

## 5. Linux-specific additions

These are deliberate additions macOS does not have. Each must have a real
owner and consumer; otherwise it is `stored-only` or omitted.

| Area | Setting | Owner / key | Task |
|---|---|---|---|
| Wallpaper | Per-workspace wallpaper | `wallpaper.perWorkspace` | T278 |
| Wallpaper | Slideshow / shuffle | `wallpaper.slideshow` | T278 |
| Desktop | Tiling gaps, drag-to-tile, modifier | `tiling.*` | T276 |
| Desktop | Window rules | `windows.rules` | T276 |
| Desktop | Global app menu | `menu.global` | T276 |
| Overview | Workspace count / per-display | `overview.*`, `workspaces.count` | T276 |
| Appearance | Icon theme / cursor theme / cursor size | `appearance.iconTheme`, `appearance.cursor*` | T274 |
| Appearance | Window control layout | `appearance.windowControls` | T274 |
| Appearance | Material/performance/rollout | `appearance.material*` | T274 |
| Displays | Per-output configuration | compositor | T273 |
| Displays | Night light schedule/temperature | `display.nightLight.*` | T273 |
| Displays | HDR / VRR / refresh | compositor | T273 |
| Sound | Per-app volume | PipeWire | T280 |
| Keyboard | Input method / compose key | `input.method`, `input.ime.*` | T280 |
| Keyboard | Layout/model/options | `input.sources` | T280 |
| Trackpad | Gesture mapping | `gestures.*` | T280 |
| Power | Charge threshold / battery care | UPower | T273 |
| Session | Idle stages (dim/blank/lock/suspend) | `idle.*` | T278 |
| Network | Firewall rules / zones | firewall adapter | T272 |
| Network | WireGuard / bridge / bond / VLAN | NM | T272 |
| Updates | Package sources / Flatpak remotes | distro adapter | T269 |
| Backup | Backend selection | `backup.backend` | T270 |
| Privacy | Portal permission tables | portal | T279 |
| Privacy | USB accessories | USBGuard | T279 |
| Search | Clipboard history | search service | T265 |
| Menu Bar | Tray overflow / workspace indicator | shell | T277 |

## 6. Findings register

The 2026-11-14 review found the following. “P0” means the pane is visibly
broken or a shipped control is not truthful; “P1” means a major macOS row or
state is missing; “P2” means polish. P0-16 and P0-17 were added the same day
from the first live T263 run; they are part of the same release bar.

### P0 — broken or untruthful

| # | Finding | Evidence | Task |
|---|---|---|---|
| P0-1 | Absent adapters collapse whole panes to one note or hide whole sections (General, Battery, Storage, Users, Printers, Game Controllers, Online Accounts) | `GeneralPane.qml`, `BatteryPane.qml`, `StoragePane.qml`, `UsersGroupsPane.qml`, `PrintersScannersPane.qml`, `GameControllersPane.qml` | T269–T280 |
| P0-2 | Many shipped controls write keys with no consumer (sound, notifications, search, backup, locale, `power.showPercentage`, `input.*`, `idle.*`) | `services/settingsd/src/schema.rs`, pane QML | T282, pane tasks |
| P0-3 | Adapter writes are fire-and-forget; failures invisible; dialogs close before the result | `*Client.cpp`, `WifiPane.qml`, `UsersGroupsPane.qml`, `BiometricsPane.qml`, `PrintersScannersPane.qml` | T282, T271, T279, T280 |
| P0-4 | General hides About, Software Update, Storage, and Reset when one unrelated adapter is absent | `GeneralPane.qml` | T269 (landed) |
| P0-5 | Accessibility is three settingsd rows against a five-section macOS IA; map claims rows that do not exist | `AccessibilityPane.qml`, `SettingsMap.qml` | T275 |
| P0-6 | Focus mode rows are inert but styled/announced as radio controls; three duplicate state surfaces | `FocusPane.qml` | T278 |
| P0-7 | Search is a no-op surface; shortcut is free text; indexing rows name nonexistent keys | `SearchPane.qml`, `schema.rs` | T265, T282 |
| P0-8 | Notification global preferences are no-ops; per-app sheet is missing half the macOS options | `NotificationsPane.qml`, `schema.rs` | T278 |
| P0-9 | Select popups and pane dialogs are clipped by the detail `ScrollView`; long lists are unusable | `Select.qml`, `ContextMenu.qml`, `Dialog.qml`, `SettingsShell.qml` | T266 (landed: window overlay layer, flip/clamp/scroll, dialog fit) |
| P0-10 | `SettingsRow` has a binding loop; rows have no focus ring; toggles have no accessible names; long strings overflow | `SettingsRow.qml`, `Toggle.qml`, `Button.qml` | T266 (landed: implicit-size slot, focus ring, accessible names, elide), T267 (landed: one keyboard map, row/description/error announcements, RTL text alignment, 1×/1.5×/2× cases) |
| P0-11 | Sidebar cannot scroll to most panes; no groups; opens Appearance instead of General; window is 800×640 | `SettingsShell.qml`, `SettingsWindow.qml`, `Sidebar.qml` | T263 (landed) |
| P0-12 | Back/forward ignore subpanes; deep links cannot land in General subpanes/dialogs; help anchor is wrong; menu actions are dead | `SettingsShell.qml`, `SettingsWindow.qml`, `SettingsMenu.qml` | T264 (landed) |
| P0-13 | settingsd accepts NaN/Infinity (persist panic), loses concurrent updates, hides persistence failure, and the client schema mirror drifts | `schema.rs`, `persist.rs`, `dbus.rs`, `settingsclient.cpp` | T282 |
| P0-14 | Adapter clients show the startup snapshot; no live refresh; ~20 D-Bus calls at startup; slider writes fsync per tick | `*Client.cpp`, `SettingsBridge.cpp` | T268 (landed: lazy clients, per-key maps, cached schema tables, commit-on-release sliders, async provider read, cached pane loader), T282 (persistence hardening) |
| P0-15 | Pane tests run at 900/1000 px; many panes have no dark/absence/scale capture; no golden diff; no real-host matrix | `apps/settings/tests/`, `scripts/capture-*settings*` | T283, T284 |
| P0-16 | Wallpaper picks do not acknowledge the click reliably: a tile can look unselected, the spinner/preview state starts only after the provider round trip, a second pick during a pending apply is ignored, and the selected tile can disagree with the background | `apps/settings/WallpaperPane.qml`, `SettingsBridge.cpp`, `services/wallpaperd` | T278 |
| P0-17 | A stationary click on a sidebar row or pane control is often dropped; moving the pointer or scrolling first makes it land. The activating press can be consumed by focus or routed through a stale cached pointer location, so the first click looks inert | `apps/settings/SettingsShell.qml`, `design-system/components/Sidebar.qml`, `compositor/src/input.rs` | T263 (live-position ray-test landed), T266 (row focus + stationary-click harness landed) |

### P1 — missing major rows or states

| # | Finding | Evidence | Task |
|---|---|---|---|
| P1-1 | Appearance missing icon/widget style, folder color, scroll-bar click behavior; transparency slider stays active under Reduce Transparency; seven one-row cards | `AppearancePane.qml` | T274 |
| P1-2 | About hardcodes “Display: Not available”, duplicates details, resizes the window permanently | `AboutPage.qml`, `SettingsWindow.qml` | T269 (landed) |
| P1-3 | Wallpaper hero is 160×100; Clock Appearance opens a pane with no clock appearance; file URLs by string concatenation | `WallpaperPane.qml` | T278 |
| P1-4 | Screen Saver has no gallery; Start After blanks the display; None does not disable the saver | `ScreenSaverPane.qml` | T278 |
| P1-5 | Lock Screen “Require password after the screen turns off” writes inactivity seconds; custom idle values display “Never” | `LockScreenPane.qml` | T278 |
| P1-6 | Displays preview is mostly blank and hardcoded “Built-in Display”; advanced controls duplicate the main pane | `DisplaysPane.qml` | T273 |
| P1-7 | Storage “Manage storage…” is a dead row; no chart/total/free; volume rows have no capacity | `StoragePane.qml` | T273 |
| P1-8 | Battery renders two permanently empty charts; options are incomplete; range is duplicated | `BatteryPane.qml` | T273 |
| P1-9 | Users & Groups has no password path for a new user, no change-password row, group delete without confirmation | `UsersGroupsPane.qml` | T279 |
| P1-10 | Printers add dialog is disconnected from its visible fields; no remove/test/share; scanner rows have no action | `PrintersScannersPane.qml` | T280 |
| P1-11 | Sound mute zeroes the volume slider; Input tab shows output controls; Balance has no consumer | `SoundPane.qml` | T280 |
| P1-12 | Mouse and Trackpad share `input.pointerSpeed`/`input.naturalScroll`; most Tahoe gestures are missing | `InputPane.qml` | T280 |
| P1-13 | Privacy is a flat un-carded list with one icon, no keyboard access, and missing master/analytics/accessory rows | `PrivacyPane.qml` | T279 |
| P1-14 | Online Accounts disappears entirely when GOA is absent; Add has no sign-in step | `SettingsPanes.qml`, `OnlineAccountsPane.qml` | T279 |
| P1-15 | Quick Settings is a 37-row flat list with Up/Down buttons; unavailable modules are offered; Edit Controls is a no-op; empty list restores defaults | `QuickSettingsPane.qml` | T277 |
| P1-16 | Menu Bar has no third-party section, Add Controls, Reset, or per-control options | `MenuBarPane.qml` | T277 |
| P1-17 | Search results are a flat ungrouped list; “wifi”, “control center”, “mission control” return no results | `SettingsSearch.qml`, `SettingsSearchResults.qml` | T265 (landed) |
| P1-18 | General is missing Autofill & Passwords and the Transfer or Reset structure; sub-panes hide settingsd rows with their host adapter | `GeneralPane.qml`, `DateTimePane.qml`, `LanguageRegionPane.qml` | T269, T270 |
| P1-19 | Focus has no focus list/Add/Delete/Filters; profile/mode/rows duplicate the same state | `FocusPane.qml` | T278 |
| P1-20 | Keyboard brightness/navigation/dictation/replacements are presented as live with no consumer | `InputPane.qml`, `schema.rs` | T280, T282 |

### P2 — polish

| # | Finding | Task |
|---|---|---|
| P2-1 | Stale header comments claim implemented rows are omitted | T263–T281 |
| P2-2 | Pane labels drift from the map (`When display is asleep` vs `sleeping`, `Quick Settings` vs `Control Center`) | T266 |
| P2-3 | Duplicate icons: General/Biometrics, Displays/Screen Saver, Network/Online Accounts | T263 |
| P2-4 | Final row separators and double group gaps | T266 (landed) |
| P2-5 | Dialog titles repeat the first content line; no app icons in notification/focus lists | T266, T278 |
| P2-6 | “OPEN” and task IDs leak into user-facing descriptions | T266 |
| P2-7 | Debug view is larger than the minimum window and has no scroll | T264 (landed) |
| P2-8 | Search field clear button has no accessible name | T267 (landed: named Tab stop with a ring and Return/Space activation) |
| P2-9 | About panel’s regulatory overlay is not keyboard-accessible | T269 |
| P2-10 | Window resize grips cover the scrollbar corners | T266 (landed: corner inset) |

## 7. Task map

Tasks T263–T284 are the dedicated **system-settings** task set
(`docs/tasks/system-settings/`); run them with
`./.symphony/symphony run --set system-settings`. The `0.2.0` set keeps the
landed foundation (T222–T229a, T230c) and the rest of the release.

| Task | Scope | Depends on |
|---|---|---|
| T263 | Window, sidebar IA, pane chrome, default pane, persistence, stationary-click activation | — |
| T264 | Navigation, history, deep links, menus, help | T263 |
| T265 | Search and local discovery | T263, T264 |
| T266 | Shared components, row anatomy, popups, dialogs, scroll, pointer-activation contract | T263 |
| T267 | Accessibility, keyboard, adaptive presentation, scale | T266 |
| T268 | Startup and interaction performance budget | T263; coordinate with T282 |
| T269 | General: About, Software Update, Storage, Autofill & Passwords | T263, T264 |
| T270 | General: Date & Time, Language, Login Items, Sharing, Backup, Startup Disk, Transfer or Reset | T269 |
| T271 | Wi-Fi and Bluetooth | T263, T266 |
| T272 | Network services, VPN, firewall, connection details | T271 |
| T273 | Battery, Storage, Displays | T263, T266 |
| T274 | Appearance | T263, T266 |
| T275 | Accessibility | T263, T266, T267 |
| T276 | Desktop & Dock, Overview, Windows | T263, T266 |
| T277 | Quick Settings and Menu Bar | T263, T266 |
| T278 | Wallpaper (including the visible apply state machine), Screen Saver, Notifications, Focus, Lock Screen | T263, T266 |
| T279 | Privacy & Security, Biometrics, Users & Groups, Online Accounts | T263, T266 |
| T280 | Sound, Keyboard, Mouse, Trackpad, Printers, Game Controllers | T263, T266 |
| T281 | Linux-specific additions | Pane tasks |
| T282 | Settings backend contract and settingsd hardening | T263, T266; coordinate with T268 |
| T283 | Settings evidence and visual regression matrix | Pane tasks |
| T284 | Settings interaction, real-host, and close-out | T283 |

## 8. Acceptance

A System Settings task is agent-done when:

- `make gates` passes and the task’s named tests pass.
- Every visible row has an owner, key/API, and absence kind in
  `SettingsMap.qml`; every `settingsd` key resolves in the schema; every
  `live` row has a consumer test.
- The pane renders at 740×820, 800×640, and the 720×480 minimum without
  clipping, overflow, or an unreachable control.
- The pane has light/dark, absence, reduced-motion, increase-contrast,
  keyboard-focus, and 1×/1.5×/2× captures under the repository’s gitignored
  `docs/captures/`, plus a manifest entry in the experience schema.
- A pane with an absent provider keeps its section structure; a pane with a
  present provider shows current state and reports write failures.
- No user-facing string contains `OPEN`, a task id, or an internal service
  name unless the row is explicitly a diagnostic.
- The task’s `## Hand-off` records what landed, what is `OPEN`, and the exact
  rerun command.

The release-wide close-out remains T260–T262; T283/T284 feed them the
Settings-specific matrix.
