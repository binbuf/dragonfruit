# Changelog

All notable user-visible changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **Large lists.** Folder stacks and the Add Application picker now render
  through one virtualized list viewport instead of a `Repeater` that built a
  delegate for every entry. Object count follows the visible rows plus a small
  cache margin, a live folder or inventory update changes only the affected
  rows, and the highlighted row follows its stable path or desktop id instead
  of its old numeric position.
- **Menu-bar clock.** The status row now shares one clock source and measures
  slot widths with a passive text-metric probe. A hidden sizing row no longer
  instantiates complete status widgets or a second clock boundary timer, so an
  idle bar has one clock wakeup and the measured overflow budget still matches
  the visible slots.
- **Chrome damage.** GPU chrome commits now damage only the pixels that
  changed. The Dock publishes the strip's complete painted bounds and the shell
  damages the union of the previous and current frame, so a pointer sweep, a
  magnification change, or a launch bounce no longer repaints every transparent
  pixel of the strip. Each surface translates that report through its own
  viewport crop, an unchanged surface sends nothing, and a moved scene, an
  unreceived frame, or a fresh buffer always falls back to a full damage.
- **Menu popups.** Dock popovers and menu-bar dropdowns now render into their
  own content-sized render targets instead of growing the strip's shared scene.
  Opening a narrow menu no longer enlarges the Dock or bar attachments, and the
  popup allocation is reused within one interaction session before it is
  released. The previous shared-scene and stable-budget paths remain available
  through `DRAGONFRUIT_POPUP_TARGETS` for the T238 comparison.
- **Menu-bar rendering.** Pointer movement, clock ticks, popup geometry, and
  policy updates now go through the menu bar's existing frame pacer instead of
  rendering directly. Unchanged hover state no longer produces duplicate
  frames, and a popup's rectangle is submitted with the same paced frame that
  draws its content.
- **Tray responsiveness.** StatusNotifier items and their menus are now read
  and activated off the shell thread, with cached snapshots in app-index and
  change-driven updates instead of a two-second poll. A slow or unresponsive
  tray exporter no longer stalls the menu bar, Dock animation, or menu
  dismissal.
- **Dock responsiveness.** Folder-stack listings, moves into pinned folders,
  and file drops on the Trash now run off the shell thread, so a slow folder,
  a large listing, or a cross-filesystem copy no longer freezes Dock hover,
  menus, or Escape handling.
- **Applications drawer material.** The drawer no longer follows the menu
  transparency slider into a near-opaque plate, and it keeps its sampled glass
  at a global Minimal degrade. The card is fixed at a mostly-opaque glass
  (0.88 light / 0.90 dark) with a stronger frost, rim and corner lens.

## [0.3.0] - 2026-11-12

The first tagged Dragonfruit release. This is a complete early-testing desktop:
a Wayland compositor, the Qt Quick shell, first-party apps, system services,
the design system, and the packaging/install tooling.

### Added

- **Compositor.** Smithay-based Wayland compositor with nested, headless, and
  DRM/GBM backends; server-side decorations and traffic lights; window
  lifecycle motion; Spaces and Mission Control; the app switcher; Xwayland
  compatibility; the liquid-glass material pipeline and software fallback.
- **Shell.** Menu bar with the global menu, system menu, status items,
  clock/date and calendar, menu-bar extras, Quick Settings, overflow/notch
  handling and auto-hide; Dock with magnification, stacks, Trash, badges, the
  app chooser, drag-and-drop, keyboard reordering, multi-display placement, the
  lifecycle menu, and launch/minimize/restore/genie motion; Control Center and
  customizable controls; Notification Center, banners, Focus profiles, and
  widgets; OSDs; Overview; Applications drawer; desktop items; lock screen.
- **System Settings.** The full pane set, including Wi-Fi/Network/VPN/firewall,
  General/Date & Time/Language/Sharing/Backup/Login Items/Updates,
  Appearance/Quick Settings/Search/Screen Saver/Desktop & Stage/Displays,
  Privacy & Security/Biometrics & Password/Online Accounts,
  Sound/Keyboard/Mouse/Trackpad/Game Controllers/Users & Groups/Printers/
  Battery, Wallpaper/Notifications/Focus/Lock Screen, and the two-surface About
  experience. Settings search, deep links, live apply, validation,
  confirm/revert, polkit authorization, and explicit absence states are wired
  through one apply path.
- **Files.** files-core browsing, sorting, file operations, trash and watching;
  the Files app with sidebar, favorites, volumes, list/grid views, selection,
  context menus, rename/new-folder/trash, file opening and thumbnails, desktop
  integration, the measured window chrome and status bar, and the Get Info
  inspector.
- **Services and integration.** settingsd with a typed schema, persistence and
  migrations; app-index; menu-broker; the session supervisor and systemd user
  units; the notification service; wallpaperd; system-status and the
  NetworkManager/BlueZ/PipeWire/UPower/UDisks/power/audio/storage adapters;
  the xdg-desktop-portal backend; the cross-distro adapter contract for Fedora,
  Ubuntu, Debian, and Arch.
- **Design system.** The `Dragonfruit` QML module, tokens, typography, motion
  presets, the Tabler line/Phosphor fill icon split, original app artwork,
  gallery snapshots, and the accessibility/naming/i18n/asset gates.
- **Packaging and developer workflow.** Portable `tar.gz`, Debian, Fedora RPM,
  and Arch binary release assets; the tag-driven release workflow; `make
  dist`/`make stage`; the nested and native session harnesses; diagnostics and
  capture tooling; the testing ladder and release runbook.

### Changed

- The product version advances from the `0.1.0` development baseline to
  `0.3.0`; release packages derive their version from the `v0.3.0` tag.
- The shell, Settings, and Files share the measured macOS-style window chrome,
  menu surfaces, materials, and motion language introduced across the T-20 to
  T-23 tracks.
- The private IPC set, settings schema, and session state remain independently
  versioned contracts; they are not tied to the product version.

### Fixed

- Dock popup, context-menu, overflow, selection, quit/Force Quit, Trash,
  auto-hide, resizing, corner, and multi-display defects found in the final
  acceptance passes.
- Window resizing, rounded window app corners, and light/dark Trash artwork.
- Native-session input, cursor-theme startup, and a DRM render-loop stall.
- Wallpaper collection rows/delegates and apply-state handling.
- Settings account-absence tests and several verification regressions.

### Known limitations

- The real DRM/hardware path remains hardware-open: multi-GPU and rotated
  outputs use fallbacks, and native direct-scanout/cursor/overlay-plane
  evidence is still open.
- The live PipeWire screencast producer and DRM screenshot capture are not
  complete.
- The production idle service and a concrete logind suspend backend are not
  wired.
- Files still lacks tabs/new-window, copy/move/paste, undo, progress/conflict
  handling, Open With, and full watcher/menu publication.
- Official Fedora COPR/source packaging and Debian policy/source packaging are
  still open; the published binary assets are early-testing packages.
- Some Control Center links/actions, per-display Control Center panels, and
  measured open/close/expand motion remain follow-ups.
