# macOS structural surface review

| | |
|---|---|
| **Status** | target design |
| **Date** | 2026-11-08 |
| **Release** | v0.2.0 |
| **Reference build** | macOS 27.0.1 (26A434), Apple silicon, 1800×1169 pt @2× |
| **Reference evidence** | `macos-design-review/docs/SURFACE-REVIEW.md`, `experience_measurements.json`, `REPORT.md` §3.9 |
| **Scope** | window-chrome height taxonomy, About, Applications drawer, Notification Center, Control Center, Finder |
| **Related** | [total experience](22-total-experience.md), [window decorations](05-window-decorations.md), [Files](09-files.md), [Settings](08-settings.md), [experience schema](experience-schema.md), [ADR 0241](adr/0241-macos-structural-surface-review.md) |

## 1. Why this document exists

The material campaign answered **what the glass is made of**. The 2026-11-08
structural review answers **how the chrome and the major shell surfaces are
arranged**: the window-chrome height system, the two About surfaces, the
Spotlight Apps drawer, Notification Center's material ramp, Control Center's
module grid, and Finder's window anatomy.

Those findings are now part of the Dragonfruit design contract. This document
distils them into implementation rules and names the task that owns each rule.
It is a design target; `docs/design/STATUS.md` remains the implementation
status.

## 2. Window chrome is a style system, not one height

### 2.1 Measured taxonomy

`macos-design-review/tools/titlebar_probe.swift` creates one AppKit window per
toolbar style and reads `contentLayoutRect` back.

| Style | Chrome height | Dragonfruit style id | Notes |
|---|---:|---|---|
| Borderless / panel | **0 pt** | `borderless` | HUDs, Quick Look-style panels, overlays |
| Titled, no toolbar | **32 pt** | `titlebar` | Minimum real window chrome; close/minimise/zoom live here |
| `.fullSizeContentView` | **32 pt** | `titlebar` | Content underlaps; layout inset stays 32 pt |
| Transparent title bar | **32 pt** | `titlebar` | Material changes, height does not |
| `.unifiedCompact` | **40 pt** | `unifiedCompact` | Shorter toolbar row; regular controls |
| `.expanded` | **48 pt** | `expanded` | Title above the toolbar row |
| `.unified` | **52 pt** | `unified` | Default modern toolbar |
| `.unified` + icon-and-label | **66 pt** | `unified` + labelled row | Disk Utility; not a separate style |
| `.preference` | **88 pt** | `preference` | Title row plus centred preference toolbar |

The practical range is **0 → 32 → 40 → 48 → 52 → 66 → 88 pt**. Real apps can
land between or above the probe values: Pages' template chooser is 38–43 pt
(12×14 pt traffic lights), a Pages document is 70 pt (28 pt title row + 42 pt
toolbar), and Safari Settings is 88 pt (32 pt title row + 56 pt toolbar).

**52 pt is not a universal window chrome height.** It is the unified-toolbar
height. A title-bar-only window is 32 pt, and a compact utility window can
still use a 52 pt unified toolbar (Calculator is the counter-example to “small
window means small chrome”).

### 2.2 Real-app confirmation

| App / window | Chrome | Evidence |
|---|---:|---|
| TextEdit document | 32 pt | close-button inset 8 pt; formatting bar is a separate 20 pt accessory |
| Finder Get Info | 32 pt | auxiliary window; content starts 32 pt below the frame |
| About This System | 32 pt | utility panel; no visible title text |
| Finder, Safari, System Settings, Photos, Activity Monitor, Calculator | 52 pt | `AXToolbar` or split view starts 52 pt below the frame |
| Disk Utility | 66 pt | unified toolbar with icon-and-label items |
| Pages template chooser | 38–43 pt | compact chrome; 12×14 pt traffic lights |
| Pages document | 70 pt | expanded chrome: 28 pt title row + 42 pt toolbar |
| Safari Settings | 88 pt | preference chrome: 32 pt title row + 56 pt toolbar |

### 2.3 Anatomy

- **Traffic lights:** 16×16 pt.
  - Title-bar-only window: 8 pt inset from the top, centring them in a 32 pt
    bar.
  - Unified toolbar: 18 pt inset, centring them in a 52 pt bar.
  - Horizontal spacing is 23 pt between close, minimise and zoom.
- **Inline title:** vertically centred in the toolbar, at the leading edge
  after the traffic lights and navigation group. Finder's title occupies a
  227×52 pt region in a 900 pt-wide window.
- **Toolbar groups:** Finder uses an 81 pt back/forward group, a 164 pt
  view-mode group, then Group (47 pt), Share (36 pt), Edit Tags (38 pt),
  Action (37 pt), and Search (40 pt). Groups are separated by glass capsules,
  not one continuous bar.
- **Accessory bars are content, not chrome.** Finder's tab bar (28 pt), path
  bar (30 pt) and status bar (28 pt) sit inside the content area. TextEdit's
  formatting bar is a 20 pt visible bar inside a 36 pt AX group. Do not add
  these to the window-chrome height.

### 2.4 Implementation rules

1. Model chrome as a **style variant**, not a per-app constant:
   `borderless`, `titlebar`, `unifiedCompact`, `expanded`, `unified`,
   `preference`.
2. Use the measured heights as baselines. The 66 pt variant is a unified
   toolbar with icon-and-label items, not a separate style. Derive the final
   height from the style plus the toolbar row height, not from the style alone.
3. A title-bar-only window still needs a 32 pt drag region when the title text
   is hidden.
4. `.fullSizeContentView` and transparent title bars keep the 32 pt layout
   inset; they change what is painted behind the chrome, not the hit target.
5. Keep tab/path/status bars in the content layer. Finder's tab bar is 28 pt,
   its path bar 30 pt and its status bar 28 pt.
6. Map an app's toolbar style from its declared behaviour, not from its
   screenshot. A compact utility window can still use a 52 pt unified toolbar.
7. Re-measure on the target toolkit. Qt Quick controls and AppKit controls do
   not share the same intrinsic heights; the macOS values are the reference,
   not a Qt default.
8. Fullscreen/auto-hide chrome is still `OPEN`: the review did not capture the
   hidden/revealed states, long titles, or RTL mirroring.

**Owner:** T230b. The existing design-system `TitleBar` is a single 40 pt
token; T230b introduces the style table and migrates first-party windows.

## 3. About is two surfaces

### 3.1 About This System panel

| Element | Geometry / observation |
|---|---|
| Window | 280×503 pt, 32 pt title bar, no visible title text |
| Hero image | 194×194 pt, starting 29 pt below the frame top |
| Product name | 137×28 pt, centred |
| Product subtitle | 60×15 pt |
| Information rows | Chip, Memory, Startup disk, Serial number, OS |
| Primary action | “More Info…” 94×26 pt |
| Secondary action | “Regulatory Certification” 131×16 pt |
| Footer | Copyright block, 169×30 pt |

The panel is a **near-opaque light vibrancy surface**. A same-position on/off
capture over a flat band changed the band mean from about 253.3 to 251.9 sRGB
(mean |Δ| ≈ 4.7/255), with the wallpaper visible as a slight tint rather than
readable detail. A single `alpha/tint` blend is not a good model: the panel has
a vertical material gradient and a shadow. Treat it as a utility-window
vibrancy variant, not as popover glass.

The title bar has no visible title text, but it is still a standard 32 pt title
bar with close, minimise and zoom. The panel is the reference test case for
“chrome without a title”.

### 3.2 Settings → General → About

“More Info…” opens the canonical detailed About page in Settings:

| Element | Geometry / observation |
|---|---|
| Window | 740×820 pt, 52 pt chrome |
| Hero image | 128×128 pt |
| Product block | Name, chip, memory, serial number, warranty, parts & service |
| OS section | Product name, version, compatibility notice, Details… |
| Displays section | Built-in display, resolution, Display Settings… |
| Storage section | Volume name, free/total, Storage Settings… |
| Footer | System Report…, licence/regulatory links, copyright |
| Scroll content | 768 pt high in the 820 pt window; the page scrolls |

This page is the **single source of truth** for machine identity. The compact
panel is a summary that links to it.

### 3.3 Implementation rules

1. Keep the two surfaces distinct: a compact panel for the system menu and a
   full page for Settings.
2. The panel's title bar is blank; do not invent a title just because the
   chrome exists.
3. Use original product imagery and original names. Do not copy Apple
   artwork, the Apple logo, or the “macOS” wordmark.
4. Long machine names, localized product names, and warranty/parts rows need
   truncation and wrapping tests.
5. Do not invent hardware values. A missing host value is an explicit
   “Not available” row, not a fake Apple value.

**Owner:** T230c. T226 shipped the General group with About as a dialog; T230c
replaces it with the measured two-surface anatomy.

## 4. Applications drawer is Spotlight Apps

### 4.1 What it actually is

On macOS 26 and 27, **Launchpad is gone**. The Dock tile labelled **Apps**
opens the **Applications** browse mode inside Spotlight. The same mode is
reachable with `Command-1` after Spotlight opens, or from the Spotlight menu.
There are no pages, no user folders, no drag-to-rearrange grid, and no page
dots.

The correct reference is **Spotlight Apps**, not the old Launchpad page model.

### 4.2 Measured geometry

| State | Geometry |
|---|---|
| Grid window | 844×577 pt, centred at 478,297 |
| Search field | 727×58 pt, with a 34×30 pt Back button and a 23×34 pt More button |
| Category chips | 26 pt high: Developer Tools, Utilities, Creativity, Productivity & Finance, Games, Social, Entertainment, Information & Reading, Other |
| Grid | 7 columns; cell 115×92 pt; icon 77×77 pt; label 18 pt high |
| Grid pitch | 115 pt horizontal, 102 pt vertical; 17 pt section gap |
| Suggestions | A first section above the alphabetical list |
| List view | Window expands to 1800×1044 pt; list column 640 pt wide, centred; row 636×56 pt; icon 46×46 pt |
| Search result | Single cell; the result row shows the app plus an action such as “Search Safari” |
| More menu | “View content as” → Grid / List |

The grid is a **scrollable, category-filtered collection** with a search-first
interaction, not a page-based launcher.

### 4.3 Information architecture

1. Search field is always present and focused on open.
2. Suggestions come first.
3. “All apps” follows in alphabetical order.
4. Category chips filter the collection; they do not create pages.
5. Grid/List is a view preference, not a different information architecture.
6. Launching an app closes the overlay and activates the app; there is no page
   transition to animate.

### 4.4 Implementation rules

1. Dragonfruit's drawer is **search-first, category-filtered, and
   scrollable**. Copying the old Launchpad page model is a regression against
   the reference build.
2. Keep the 7-column, 115×92 pt grid at this display size; derive columns from
   width rather than hard-coding 7.
3. The 77 pt icon and 18 pt label are the dominant visual weights; category
   chips are secondary.
4. List mode is a real alternate view, not a fallback. It expands the surface
   to full height and centres a 640 pt column.
5. Do not add folders or manual ordering unless Dragonfruit deliberately
   chooses to exceed the reference. The reference does not support them.
6. The drawer's search filters the app corpus only; system Search is a
   separate surface (T246/T247).

**Owner:** T248. The current drawer already has a 7-column search-first grid;
T248 adds the measured list mode, suggestions, launch/close feedback, and the
remaining keyboard/empty states, and removes the old folder/paging language
from the task and reference docs.

## 5. Notification Center is a vertical material ramp

### 5.1 Layout

Notification Center is a right-edge overlay, not a floating window:

| Element | Geometry |
|---|---|
| Overlay window | Full screen 1800×1169 pt; content right-aligned |
| Panel content | 344 pt wide, starting at x=1440, y=64; scroll area 752×1063 pt |
| Header | “Notification Center” 159×21 pt; Clear All 20×20 pt |
| Banner stack | 344×75 pt at y=93 |
| Widget | Calendar 164×164 pt at y=200 |
| Actions | Edit Widgets 87×24 pt; Clear Notifications menu 24×24 pt |
| Widget gallery | 1027×552 pt separate window; 240 pt category sidebar; 19 categories |
| Gallery previews | Small, medium, large, extra-large and extra-large portrait variants |

The panel is a **vertical stack**, not a single glass sheet: notifications
first, then widgets, then the Edit Widgets/Clear actions.

### 5.2 Materials

| Band | α | Tint |
|---|---:|---:|
| Notification banner | 0.549 | 0.976 |
| Banner stack | 0.551 | 0.706 |
| Lower Center panel | 0.407 | 0.852 |

These are three different effective materials. The banner is the lightest; the
lower panel is darker and more transparent. Do not flatten them into one
`NotificationCenter` surface.

### 5.3 Information architecture

- Notifications are grouped into a stack; the top item is the stack header.
- “Clear All” clears the stack; the overflow menu clears notifications.
- Widgets live below notifications.
- “Edit Widgets” opens the gallery; widgets can be added to Notification
  Center or dragged to the desktop.
- Focus, Do Not Disturb, and per-app notification settings are reachable from
  the notification/settings flow, not from a separate panel in this surface.

### 5.4 Implementation rules

1. Model the panel as a **vertical material ramp**: banner → stack → lower
   panel.
2. Widget sizes in the live panel are not the same as gallery preview sizes.
   The Calendar widget measured 164×164 pt in the panel; the gallery preview
   showed a 102×102 pt thumbnail. Use the live panel size for layout and the
   gallery size only for the gallery.
3. Keep the right-edge overlay model; it preserves the desktop context and
   matches the swipe-in interaction.
4. The widget gallery is a separate window with its own sidebar and search; it
   is not a sheet inside the panel.
5. Empty state, actions/reply, Focus active, Weather/Calendar variants,
   multi-display, and motion remain `OPEN` until captured or implemented.

**Owner:** T234. The current banner/stack/lower-panel material roles already
exist; T234 adds the measured layout, widget stack/gallery, empty/actions/Focus
states, and motion.

## 6. Control Center is a nested dark-glass grid

### 6.1 Default panel geometry

The visible panel is right-aligned; the AX window container is larger than the
visible glass because it includes the off-screen hosting area.

| Element | Geometry |
|---|---|
| Module field | 292 pt wide at x=1494 |
| Grid | 4-column base of 64 pt cells; 12 pt gaps; modules span 1–2 columns/rows (64×64, 140×64, 140×140, 292×64) |
| Wi-Fi | 140×64 pt |
| Bluetooth | 140×64 pt |
| Now Playing | 140×140 pt |
| AirDrop | 140×64 pt |
| Stage Manager | 64×64 pt |
| Screen Mirroring | 64×64 pt |
| Dark Mode | 64×64 pt |
| Capture Screen | 64×64 pt |
| Focus | 140×64 pt |
| Display | 292×64 pt group with a 217×14 pt slider |
| Sound | 292×64 pt group with a 180×14 pt slider |
| AirPlay Audio | 26×26 pt |
| Edit Controls | 93×24 pt |

The 292 pt module field is exactly `4×64 + 3×12`.

### 6.2 Expanded states

| Module | Expanded sheet |
|---|---|
| Wi-Fi | 308×87 pt: known network row (280×32), Other Networks disclosure, Wi-Fi Settings… |
| Bluetooth | 308×64 pt: two device rows (280×32 each), Bluetooth Settings… |
| Focus | Two rows (Do Not Disturb, Reduce Interruptions) plus Focus Settings… |
| Screen Mirroring | Display Settings… plus an empty device list |
| Now Playing | 140×140 pt module; previous/play/next 30×30 pt |

The Wi-Fi and Bluetooth sheets expand the outer panel; the panel grows behind
the module. The expanded outer panel measures α=0.286/tint=0.257, close to the
default outer panel (0.265/0.373) but not identical.

### 6.3 Edit Controls catalogue

“Edit Controls” opens a 736×900 pt window with a 245 pt category sidebar and
22 categories:

All Controls, Battery, Clock, Connectivity, Desktop & Finder, Display &
Brightness, Fast User Switching, Focus, Home, Keyboards, Lock Screen, Notes,
Now Playing, Screenshot, Shortcuts, Sounds, Utilities, Voice Memos,
Accessibility, Hearing Accessibility, Motor Accessibility, Vision
Accessibility.

This is the real inventory of what Control Center can contain. The default
panel is only a subset.

### 6.4 Materials

| Surface | α | Tint |
|---|---:|---:|
| Outer panel | 0.265 | 0.373 |
| Wi-Fi module | 0.391 | 0.317 |
| Wi-Fi-expanded outer panel | 0.286 | 0.257 |
| Bluetooth expanded list | ~0.13 luma | not invertible |

Control Center is **dark glass even in Light appearance**. The modules are
separate capsules nested on the outer panel; the measured module value is an
effective glass-on-glass value. Keep the nesting and the strong lensing at the
capsule ends; do not flatten it to one translucent rectangle.

### 6.5 Implementation rules

1. Use a 4-column base grid of 64 pt cells with 12 pt gaps. Modules span one or
   two columns/rows: 64×64 (1×1), 140×64 (2×1), 140×140 (2×2), 292×64 (4×1).
2. Keep the outer panel and the module capsules as separate materials.
3. Expanded modules grow the panel; they are not detached popovers.
4. The Edit Controls catalogue is part of the surface and needs its own
   sidebar/list geometry.
5. The panel supports multiple pages and user-arranged controls on the
   reference build; the default panel is not the whole design.
6. The menu-bar pinning path is a separate drag target; do not conflate it with
   the panel grid.
7. Battery, Keyboard Brightness, Accessibility, Fast User Switching, multiple
   pages, resize, pinning, and motion remain `OPEN` until captured or
   implemented.

**Owner:** T235. The current panel is a curated tile list; T235 moves it to the
measured grid, adds the edit catalogue and customization, and preserves the
existing outer/module/expanded material regions.

## 7. Finder is the reference file-manager anatomy

### 7.1 Window geometry

| Element | Geometry |
|---|---|
| Window | 900×600 pt at 100,120 |
| Toolbar | 900×52 pt |
| Sidebar | 198 pt wide, starting at the toolbar bottom |
| Tab bar | 702×28 pt below the toolbar when tabs are shown |
| Path bar | 688×30 pt at the bottom of the content region |
| Status bar | 702×28 pt at the bottom edge |
| Content scroll | 702×484 pt with sidebar and tab bar visible |
| Get Info window | 328×1034 pt, 32 pt title bar |
| Toolbar hidden | Content starts 64 pt below the frame; sidebar is hidden too |

The toolbar-hidden measurement confirms Apple's documentation: hiding the
toolbar also hides the Finder sidebar and moves the status bar.

### 7.2 Toolbar and sidebar

Toolbar groups, left to right:

- Back/forward: 81×52 pt, two 36×36 pt segments.
- View modes: 164×52 pt, four 40×36 pt segments (icon, list, column, gallery).
- Group: 47×52 pt menu button.
- Share: 36×52 pt.
- Edit Tags: 38×52 pt.
- Action: 37×52 pt menu button.
- Search: 40×52 pt search field.

Sidebar sections: Recents/Shared, Favorites, Locations, Tags. Rows are 32 pt
high; section headers are 19 pt high. The sidebar is a near-opaque vibrancy
surface (α=0.985/tint=0.858 in Light; α=0.935/tint=0.218 in Dark), not the
popover glass.

### 7.3 View modes

The four view modes are real, separate layouts:

- **Icon** — collection grid with 64×64 pt icons and labels.
- **List** — multi-column table.
- **Column** — browser columns.
- **Gallery** — large preview plus a filmstrip.

The view-mode segment is 40×36 pt; the selected segment is the only selected
state in the group. The window geometry (toolbar/sidebar/tab/path/status) does
not change when the view mode changes; only the content layout does.

### 7.4 Get Info

The Get Info window is a 328×1034 pt auxiliary window with a 32 pt title bar.
It uses a single scrolling column with sections: General, More Info, Name &
Extension, Comments, Open with, Preview, Sharing & Permissions. This is the
reference for Dragonfruit's property/inspector window: a narrow, tall,
single-column auxiliary window, not a wide sheet.

### 7.5 Implementation rules

1. Finder is the reference for the **sidebar + toolbar + content + bottom
   bars** window layout. Use it for Files and any inspector-heavy app.
2. The toolbar is 52 pt; the tab/path/status bars are content-layer bars.
3. Hiding the toolbar is a mode change, not just a visibility toggle: it also
   hides the sidebar and moves the status bar.
4. The sidebar is near-opaque vibrancy; the toolbar is nearly opaque. Do not
   apply the 0.55 popover alpha to Finder chrome.
5. The four view modes need separate layout components; a single grid with a
   different column count is not enough.
6. Search, gallery preview, tags, sidebar/toolbar customization, the dark
   all-view matrix, and motion remain `OPEN`.

**Owner:** T230d for the measured chrome and Get Info inspector; T244a for the
column and gallery views. The remaining tab/search/tag/customization/motion
work stays tracked in `09-files.md` and `STATUS.md`.

## 8. Cross-surface rules

1. **Chrome height is a property of the window style, not the app.** The same
   app can use 32 pt for an inspector and 52 pt for its main window.
2. **There are at least four material families in this set:**
   - popover/menu glass (~0.55 alpha);
   - window chrome/vibrancy (~0.90–0.99 alpha);
   - Control Center dark modules (~0.39 alpha, dark even in Light);
   - Notification Center bands (0.407–0.551 alpha, three different values).
3. **Accessory bars are content.** Tabs, path bars, status bars and format
   bars do not increase the title-bar height.
4. **Expanded surfaces grow their parent.** Control Center modules expand the
   panel; Notification Center widgets live in the panel; the app drawer is a
   standalone Spotlight mode.
5. **Search-first launchers replace page-based launchers.** The reference build
   has no Launchpad pages or folders.
6. **Use original assets.** Reproduce the interaction and geometry, not
   Apple's bitmaps, product images, sounds, trademarks or code.

## 9. Evidence and OPEN rows

The shareable geometry is
`macos-design-review/docs/experience_measurements.json`. Local screenshots and
AX dumps remain in that repository's gitignored `captures/` directory.

The structural review still leaves these rows open:

| ID | Missing evidence | Owner |
|---|---|---|
| OPEN-CHROME-02 | Fullscreen/auto-hide title bar hidden/revealed states | T230b |
| OPEN-CHROME-03 | RTL and long-title truncation/mirroring | T230b |
| OPEN-ABOUT-01 | Dark mode and controlled material fit for the About panel | T230c |
| OPEN-ABOUT-02 | Localization and long machine names | T230c |
| OPEN-DRAWER-01 | Drawer open/close/launch motion | T248 |
| OPEN-DRAWER-02 | Full keyboard, empty/no-results, iPhone-app rows, multi-display | T248 |
| OPEN-NC-01 | Empty, actions, Focus, Weather/Calendar variants | T234 |
| OPEN-NC-02 | Notification Center open/close, banner and stack motion | T234 |
| OPEN-CC-01 | Battery, Keyboard Brightness, Accessibility, Fast User Switching modules | T235 |
| OPEN-CC-02 | Multiple pages, control resizing, menu-bar pinning | T235 |
| OPEN-CC-03 | Control Center open/close, expand/collapse and edit-mode motion | T235 |
| OPEN-FINDER-01 | Search, gallery preview, tags | T230d / T244a / Files track |
| OPEN-FINDER-02 | Sidebar/toolbar customization | Files track |
| OPEN-FINDER-03 | Dark all-view matrix and motion | T244a / Files track |

These are explicit debt rows, not silent assumptions. Each task must replace
its rows with a capture, a test, or a narrower `OPEN` row with a rerun command.

## 10. Task map

| Finding | Task | Where |
|---|---|---|
| Extend the experience manifest, thresholds and reference inventory | T230a | `docs/tasks/0.2.0/230a-t-22.2a-structural-review-intake.md` |
| Window chrome style taxonomy and traffic-light geometry | T230b | `docs/tasks/0.2.0/230b-t-22.2a-window-chrome-style-system.md` |
| About panel + full About page | T230c | `docs/tasks/0.2.0/230c-t-22.2a-about-two-surfaces.md` |
| Files chrome and Get Info inspector | T230d | `docs/tasks/0.2.0/230d-t-22.2a-files-chrome-inspector.md` |
| Files column and gallery views | T244a | `docs/tasks/0.2.0/244a-t-22.4-files-column-gallery.md` |
| Notification Center layout, widgets and ramp | T234 | `docs/tasks/0.2.0/234-t-22.2-notifications-focus-widgets.md` |
| Control Center grid, edit catalogue and customization | T235 | `docs/tasks/0.2.0/235-t-22.2-quick-settings-customization.md` |
| Spotlight Apps drawer model | T248 | `docs/tasks/0.2.0/248-t-22.5-apps-drawer.md` |
| Acceptance harness consumes the new rows | T260 | `docs/tasks/0.2.0/260-t-22.8-acceptance-harness.md` |
