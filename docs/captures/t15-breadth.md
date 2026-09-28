# T-15 breadth capture set (reviewed)

The T-15 breadth set is a **linked set**, not one recording: every Wave-2/3
subsystem has a Settings-pane still and a Control Center-tile still
(`t15-<slice><letter>-*`), and the adapter-only slices (the `*a` tasks, which
ship no surface of their own) have a whole-desktop still that proves the tree
still composites. The track design names this set
(`docs/design/tracks/15-system-services-breadth.md`,
"Capture: `docs/captures/t15-breadth.*`"). The single representative
whole-desktop still is [`t15-breadth.png`](t15-breadth.png), captured with
**every provider absent**; the pane/tile stills below are the per-subsystem
walkthrough.

Every still is compared against the matching local **macOS Tahoe 26** capture
(`docs/reference/macos/SystemSettings_*.md`, never shipped) using the shared
interface language in [ADR 0122](../design/adr/0122-tahoe-interface-language-across-chrome.md):
split Settings window, colored sidebar tiles, grouped inset cards, live-apply
controls, and the floating Control Center panel. The Apple-only rows are
dropped/adapted per that ADR and each pane's own reference section.

Naming: `a` = adapter desktop still, `b` = pane + tile still. All captures are
produced by `scripts/capture-t15-*.sh` with `spectacle` over the nested demo,
under the pane's fixture seams (`DF_*_FIXTURE`, `DF_SETTINGS_START_PANE`), so no
host daemon is needed. None are in `make e2e`.

| Subsystem | Slice | Pane still | Tile still | Adapter desktop |
|---|---|---|---|---|
| Bluetooth | T-15.1a/b | [bluetooth-pane](t15-1b-bluetooth-pane.png) | [bluetooth-control-center](t15-1b-bluetooth-control-center.png) | [adapter](t15-1a-bluetooth-adapter.png) |
| Storage | T-15.2a/b | [storage-pane](t15-2b-storage-pane.png) | [storage-control-center](t15-2b-storage-control-center.png) | [adapter](t15-2a-storage-adapter.png) |
| Sound & routing | T-15.3a/b | [sound-pane](t15-3b-sound-pane.png) | [sound-control-center](t15-3b-sound-control-center.png) | [audio-routing](t15-3a-audio-routing.png) |
| Keyboard/Mouse/Trackpad | T-15.4a/b | [keyboard-pane](t15-4b-keyboard-pane.png) | [input-control-center](t15-4b-input-control-center.png) | [adapter](t15-4a-input-adapter.png) |
| Mission Control & hot corners | T-15.5a/b | [mission-control-pane](t15-5b-mission-control-pane.png) | [mission-control-control-center](t15-5b-mission-control-control-center.png) | [adapter](t15-5a-mission-control-adapter.png) |
| Battery & power profiles | T-15.6a/b | [battery-pane](t15-6b-battery-pane.png) | [battery-control-center](t15-6b-battery-control-center.png) | [adapter](t15-6a-power-adapter.png) |
| Notifications & Focus | T-15.7a/b | [notifications](t15-7b-notifications-pane.png) · [focus](t15-7b-focus-pane.png) | [control-center](t15-7b-control-center.png) | [adapter](t15-7a-notify-adapter.png) |
| Lock Screen | T-15.8a/b | [lock-screen-pane](t15-8b-lock-screen-pane.png) | [lock-screen-control-center](t15-8b-lock-screen-control-center.png) | [adapter](t15-8a-lock-adapter.png) |
| Menu Bar | T-15.9a/b | [menu-bar-pane](t15-9b-menu-bar-pane.png) | [menu-bar-control-center](t15-9b-menu-bar-control-center.png) | [adapter](t15-9a-menubar-adapter.png) |
| General/About/Updates | T-15.10a/b | [general-pane](t15-10b-general-pane.png) | [general-control-center](t15-10b-general-control-center.png) | [adapter](t15-10a-update-adapter.png) |
| Users & Groups | T-15.11a/b | [users-pane](t15-11b-users-pane.png) | [users-control-center](t15-11b-users-control-center.png) | [adapter](t15-11a-account-adapter.png) |
| Printers & Scanners | T-15.12a/b | [printers-pane](t15-12b-printers-pane.png) | [printers-control-center](t15-12b-printers-control-center.png) | [adapter](t15-12a-printer-adapter.png) |
| Privacy & Security | T-15.13a/b | [privacy-pane](t15-13b-privacy-pane.png) | [privacy-control-center](t15-13b-privacy-control-center.png) | [adapter](t15-13a-privacy-adapter.png) |
| Accessibility | T-15.14a/b | [accessibility-pane](t15-14b-accessibility-pane.png) | [accessibility-control-center](t15-14b-accessibility-control-center.png) | [adapter](t15-14a-accessibility-adapter.png) |
| Network (VPN) | T-15.15a/b | [vpn-pane](t15-15b-vpn-pane.png) | [vpn-control-center](t15-15b-vpn-control-center.png) | [adapter](t15-15a-vpn-adapter.png) |
| Whole desktop (all providers absent) | T-15.16 | [breadth](t15-breadth.png) | — | — |

Wave-1 panes (Appearance, Wallpaper, Desktop & Dock, Displays) predate this
track and are captured in `t09-settings-wave-1-*.png`
(`scripts/capture-settings-wave-1.sh`); the Wi-Fi pane is not shipped. Sound's
Control Center tile is the `volume` slider tile; Keyboard/Mouse/Trackpad share
one tile; Notifications has no tile (Focus does). The absent state of every
shipped pane is the [T-15 absence matrix](t15-absence-matrix.md), not this
set.

[`README.md`](README.md) documents each script and its fixture seams.