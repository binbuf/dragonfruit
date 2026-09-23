# Progress Notes

Working notes for the strict work-unit plan in [ROADMAP.md](ROADMAP.md). One
section per work unit, in execution order. Add findings to the unit's section
whenever it teaches the next unit something (environment quirks, decisions
beyond the design docs, follow-ups). Newest details last within a section.

The legacy (phase-based) plan's notes are archived at
[`tasks/legacy/PROGRESS.md`](tasks/legacy/PROGRESS.md); T-xx numbers there are
legacy numbers and do not match this plan.

Status: `☐` not started · `◐` in progress · `☑` agent-done · `✔` human sign-off

## Status

| # | Unit | Status |
|---|------|--------|
| 001 | T-01.1 — Titlebar render element | ☐ |
| 002 | T-01.2 — Traffic-light actions | ☐ |
| 003 | T-01.3 — Titlebar drag, double-click, fullscreen reveal | ☐ |
| 004 | T-01.4 — Window menu | ☐ |
| 005 | T-01.5 — Decoration tier policy and X11 correctness | ☐ |
| 006 | T-01.6 — make demo harness and loop integration | ☐ |
| 007 | T-02.1 — Animation clock and window appear | ☐ |
| 008 | T-02.2 — Minimize and restore motion | ☐ |
| 009 | T-02.3 — Zoom and fullscreen transitions | ☐ |
| 010 | T-02.4 — Close ghost, interruptibility, capture | ☐ |
| 011 | T-03.1 — Nested performance budgets (host rail) | ☐ |
| 012 | T-03.2 — DRM first bring-up | ☐ |
| 013 | T-03.3 — Hardware input validation | ☐ |
| 014 | T-03.4 — DRM soak, teardown, runbook | ☐ |
| 015 | T-04.1 — Real shadows and rounded-corner clipping | ☐ |
| 016 | T-04.2 — Backdrop blur pass | ☐ |
| 017 | T-04.3 — Reusable scene-transform pass | ☐ |
| 018 | T-04.4 — Material degrade tiers, schemes, sign-off package | ☐ |
| 019 | T-05.1 — Live-surface transform into the overview grid | ☐ |
| 020 | T-05.2 — Hit-testing and selection on live representations | ☐ |
| 021 | T-05.3 — Drag a live representation between Spaces | ☐ |
| 022 | T-05.4 — Image wallpaper and per-Space slide | ☐ |
| 023 | T-05.5 — Desktop Reveal | ☐ |
| 024 | T-05.6 — Overview frame budget and capture | ☐ |
| 025 | T-06.1 — App-switcher state machine | ☐ |
| 026 | T-06.2 — Switcher overlay with live previews | ☐ |
| 027 | T-07.1 — Adapter framework and contract | ☐ |
| 028 | T-07.2 — Networking adapter (NetworkManager) | ☐ |
| 029 | T-07.3 — Audio adapter (PipeWire/WirePlumber) | ☐ |
| 030 | T-07.4 — Power adapter (UPower) | ☐ |
| 031 | T-07.5 — Status-item menus and placeholder removal | ☐ |
| 032 | T-07.6 — Absent-daemon matrix and capture | ☐ |
| 033 | T-08.1 — settingsd daemon core and D-Bus API | ☐ |
| 034 | T-08.2 — Consumer migration to settingsd | ☐ |
| 035 | T-08.3 — Restart, resync, and key-schema documentation | ☐ |
| 036 | T-09.1 — Settings app shell | ☐ |
| 037 | T-09.2 — Appearance pane | ☐ |
| 038 | T-09.3 — Wallpaper pane | ☐ |
| 039 | T-09.4 — Desktop & Dock pane | ☐ |
| 040 | T-09.5 — Displays-basic pane | ☐ |
| 041 | T-09.6 — Menu model, absence matrix, wave sign-off | ☐ |
| 042 | T-10.1 — files-core streaming listing and model | ☐ |
| 043 | T-10.2 — files-core operations and optimistic semantics | ☐ |
| 044 | T-10.3 — files-core trash and folder watcher | ☐ |
| 045 | T-10.4 — Files UI shell and views | ☐ |
| 046 | T-10.5 — Files performance budgets | ☐ |
| 047 | T-10.6 — Dock integration: one Trash source | ☐ |
| 048 | T-10.7 — Files capture and acceptance walkthrough | ☐ |
| 049 | T-11.1 — Notification service | ☐ |
| 050 | T-11.2 — DND/Focus policy and menu-bar reflection | ☐ |
| 051 | T-11.3 — Control Center panel | ☐ |
| 052 | T-11.4 — OSD and ambient capture | ☐ |
| 053 | T-12.1 — Session manager and services | ☐ |
| 054 | T-12.2 — Display-manager entry and logout teardown | ☐ |
| 055 | T-12.3 — Lock screen and enforcement | ☐ |
| 056 | T-12.4 — Idle timers and inhibitors | ☐ |
| 057 | T-12.5 — Suspend/resume, policy keys, kill matrix, capture | ☐ |
| 058 | T-13.1 — Portal backend skeleton and session service | ☐ |
| 059 | T-13.2 — FileChooser portal | ☐ |
| 060 | T-13.3 — Screenshot portal and capture UI | ☐ |
| 061 | T-13.4 — ScreenCast portal and source picker | ☐ |
| 062 | T-13.5 — Clipboard round-trips | ☐ |
| 063 | T-13.6 — polkit authentication agent | ☐ |
| 064 | T-13.7 — Flatpak validation and capture | ☐ |
| 065 | T-14.1 — app-index service | ☐ |
| 066 | T-14.2 — menu-broker | ☐ |
| 067 | T-14.3 — StatusNotifier/AppIndicator tray | ☐ |
| 068 | T-14.4 — DBusMenu bridge | ☐ |
| 069 | T-14.5 — XDnD bridge | ☐ |
| 070 | T-14.6 — Strange-app zoo | ☐ |
| 071 | T-14.7 — Retire interim paths | ☐ |
| 072 | T-15.1 — Bluetooth | ☐ |
| 073 | T-15.2 — Storage and removable media | ☐ |
| 074 | T-15.3 — Sound and routing | ☐ |
| 075 | T-15.4 — Keyboard, Mouse, and Trackpad | ☐ |
| 076 | T-15.5 — Mission Control and hot corners | ☐ |
| 077 | T-15.6 — Battery and power profiles | ☐ |
| 078 | T-15.7 — Notifications and Focus | ☐ |
| 079 | T-15.8 — Lock Screen policy | ☐ |
| 080 | T-15.9 — Menu Bar configuration | ☐ |
| 081 | T-15.10 — General, About, and Updates | ☐ |
| 082 | T-15.11 — Users and Groups | ☐ |
| 083 | T-15.12 — Printers and Scanners | ☐ |
| 084 | T-15.13 — Privacy and Security | ☐ |
| 085 | T-15.14 — Accessibility | ☐ |
| 086 | T-15.15 — Network advanced (VPN) | ☐ |
| 087 | T-15.16 — Absent-daemon matrix and breadth capture | ☐ |
| 088 | T-16.1 — Per-output chrome sizing and placement | ☐ |
| 089 | T-16.2 — Hotplug under load and lockstep | ☐ |
| 090 | T-16.3 — Fractional scaling | ☐ |
| 091 | T-16.4 — Suspend/resume soak | ☐ |
| 092 | T-16.5 — Graphics driver matrix | ☐ |
| 093 | T-16.6 — Accessibility audit | ☐ |
| 094 | T-16.7 — Localization and i18n | ☐ |
| 095 | T-16.8 — Crash recovery kill matrix | ☐ |
| 096 | T-16.9 — Fedora packaging and CI | ☐ |
| 097 | T-16.10 — Debian packaging and CI | ☐ |
| 098 | T-16.11 — Packaged-build performance re-measure | ☐ |
| 099 | T-17.1 — Nested full-loop verification | ☐ |
| 100 | T-17.2 — DRM full-loop verification | ☐ |
| 101 | T-17.3 — Visual floor and reduced-motion sign-off | ☐ |
| 102 | T-17.4 — Performance budget verification | ☐ |
| 103 | T-17.5 — Robustness verification | ☐ |
| 104 | T-17.6 — Unfamiliar-user test and sign-off report | ☐ |

## Detailed notes

### T-01 — Loop v0 — window controls |

#### T-01.1 — Titlebar render element

_No notes yet._

#### T-01.2 — Traffic-light actions

_No notes yet._

#### T-01.3 — Titlebar drag, double-click, fullscreen reveal

_No notes yet._

#### T-01.4 — Window menu

_No notes yet._

#### T-01.5 — Decoration tier policy and X11 correctness

_No notes yet._

#### T-01.6 — make demo harness and loop integration

_No notes yet._

### T-02 — Loop v1 — lifecycle motion |

#### T-02.1 — Animation clock and window appear

_No notes yet._

#### T-02.2 — Minimize and restore motion

_No notes yet._

#### T-02.3 — Zoom and fullscreen transitions

_No notes yet._

#### T-02.4 — Close ghost, interruptibility, capture

_No notes yet._

### T-03 — Real session — bring-up & perf |

#### T-03.1 — Nested performance budgets (host rail)

_No notes yet._

#### T-03.2 — DRM first bring-up

_No notes yet._

#### T-03.3 — Hardware input validation

_No notes yet._

#### T-03.4 — DRM soak, teardown, runbook

_No notes yet._

### T-04 — Loop v2 — materials |

#### T-04.1 — Real shadows and rounded-corner clipping

_No notes yet._

#### T-04.2 — Backdrop blur pass

_No notes yet._

#### T-04.3 — Reusable scene-transform pass

_No notes yet._

#### T-04.4 — Material degrade tiers, schemes, sign-off package

_No notes yet._

### T-05 — Loop v3 — Mission Control live |

#### T-05.1 — Live-surface transform into the overview grid

_No notes yet._

#### T-05.2 — Hit-testing and selection on live representations

_No notes yet._

#### T-05.3 — Drag a live representation between Spaces

_No notes yet._

#### T-05.4 — Image wallpaper and per-Space slide

_No notes yet._

#### T-05.5 — Desktop Reveal

_No notes yet._

#### T-05.6 — Overview frame budget and capture

_No notes yet._

### T-06 — Loop v4 — app switcher |

#### T-06.1 — App-switcher state machine

_No notes yet._

#### T-06.2 — Switcher overlay with live previews

_No notes yet._

### T-07 — Menu bar goes live |

#### T-07.1 — Adapter framework and contract

_No notes yet._

#### T-07.2 — Networking adapter (NetworkManager)

_No notes yet._

#### T-07.3 — Audio adapter (PipeWire/WirePlumber)

_No notes yet._

#### T-07.4 — Power adapter (UPower)

_No notes yet._

#### T-07.5 — Status-item menus and placeholder removal

_No notes yet._

#### T-07.6 — Absent-daemon matrix and capture

_No notes yet._

### T-08 — settingsd — one owner |

#### T-08.1 — settingsd daemon core and D-Bus API

_No notes yet._

#### T-08.2 — Consumer migration to settingsd

_No notes yet._

#### T-08.3 — Restart, resync, and key-schema documentation

_No notes yet._

### T-09 — Settings Wave 1 |

#### T-09.1 — Settings app shell

_No notes yet._

#### T-09.2 — Appearance pane

_No notes yet._

#### T-09.3 — Wallpaper pane

_No notes yet._

#### T-09.4 — Desktop & Dock pane

_No notes yet._

#### T-09.5 — Displays-basic pane

_No notes yet._

#### T-09.6 — Menu model, absence matrix, wave sign-off

_No notes yet._

### T-10 — Files MVP |

#### T-10.1 — files-core streaming listing and model

_No notes yet._

#### T-10.2 — files-core operations and optimistic semantics

_No notes yet._

#### T-10.3 — files-core trash and folder watcher

_No notes yet._

#### T-10.4 — Files UI shell and views

_No notes yet._

#### T-10.5 — Files performance budgets

_No notes yet._

#### T-10.6 — Dock integration: one Trash source

_No notes yet._

#### T-10.7 — Files capture and acceptance walkthrough

_No notes yet._

### T-11 — Control Center + notifications |

#### T-11.1 — Notification service

_No notes yet._

#### T-11.2 — DND/Focus policy and menu-bar reflection

_No notes yet._

#### T-11.3 — Control Center panel

_No notes yet._

#### T-11.4 — OSD and ambient capture

_No notes yet._

### T-12 — Session + lock + idle |

#### T-12.1 — Session manager and services

_No notes yet._

#### T-12.2 — Display-manager entry and logout teardown

_No notes yet._

#### T-12.3 — Lock screen and enforcement

_No notes yet._

#### T-12.4 — Idle timers and inhibitors

_No notes yet._

#### T-12.5 — Suspend/resume, policy keys, kill matrix, capture

_No notes yet._

### T-13 — Portals + capture + clipboard |

#### T-13.1 — Portal backend skeleton and session service

_No notes yet._

#### T-13.2 — FileChooser portal

_No notes yet._

#### T-13.3 — Screenshot portal and capture UI

_No notes yet._

#### T-13.4 — ScreenCast portal and source picker

_No notes yet._

#### T-13.5 — Clipboard round-trips

_No notes yet._

#### T-13.6 — polkit authentication agent

_No notes yet._

#### T-13.7 — Flatpak validation and capture

_No notes yet._

### T-14 — Global menu + app index + compat |

#### T-14.1 — app-index service

_No notes yet._

#### T-14.2 — menu-broker

_No notes yet._

#### T-14.3 — StatusNotifier/AppIndicator tray

_No notes yet._

#### T-14.4 — DBusMenu bridge

_No notes yet._

#### T-14.5 — XDnD bridge

_No notes yet._

#### T-14.6 — Strange-app zoo

_No notes yet._

#### T-14.7 — Retire interim paths

_No notes yet._

### T-15 — System services + Settings Waves 2–3 |

#### T-15.1 — Bluetooth

_No notes yet._

#### T-15.2 — Storage and removable media

_No notes yet._

#### T-15.3 — Sound and routing

_No notes yet._

#### T-15.4 — Keyboard, Mouse, and Trackpad

_No notes yet._

#### T-15.5 — Mission Control and hot corners

_No notes yet._

#### T-15.6 — Battery and power profiles

_No notes yet._

#### T-15.7 — Notifications and Focus

_No notes yet._

#### T-15.8 — Lock Screen policy

_No notes yet._

#### T-15.9 — Menu Bar configuration

_No notes yet._

#### T-15.10 — General, About, and Updates

_No notes yet._

#### T-15.11 — Users and Groups

_No notes yet._

#### T-15.12 — Printers and Scanners

_No notes yet._

#### T-15.13 — Privacy and Security

_No notes yet._

#### T-15.14 — Accessibility

_No notes yet._

#### T-15.15 — Network advanced (VPN)

_No notes yet._

#### T-15.16 — Absent-daemon matrix and breadth capture

_No notes yet._

### T-16 — Platform polish + packaging |

#### T-16.1 — Per-output chrome sizing and placement

_No notes yet._

#### T-16.2 — Hotplug under load and lockstep

_No notes yet._

#### T-16.3 — Fractional scaling

_No notes yet._

#### T-16.4 — Suspend/resume soak

_No notes yet._

#### T-16.5 — Graphics driver matrix

_No notes yet._

#### T-16.6 — Accessibility audit

_No notes yet._

#### T-16.7 — Localization and i18n

_No notes yet._

#### T-16.8 — Crash recovery kill matrix

_No notes yet._

#### T-16.9 — Fedora packaging and CI

_No notes yet._

#### T-16.10 — Debian packaging and CI

_No notes yet._

#### T-16.11 — Packaged-build performance re-measure

_No notes yet._

### T-17 — The premium experience gate |

#### T-17.1 — Nested full-loop verification

_No notes yet._

#### T-17.2 — DRM full-loop verification

_No notes yet._

#### T-17.3 — Visual floor and reduced-motion sign-off

_No notes yet._

#### T-17.4 — Performance budget verification

_No notes yet._

#### T-17.5 — Robustness verification

_No notes yet._

#### T-17.6 — Unfamiliar-user test and sign-off report

_No notes yet._
