# T-14.6a strange-app zoo matrix

Scripted nested run (`scripts/zoo/zoo-run.sh`). Each app is launched against the private Dragonfruit socket; its raw identity is read from the compositor (`query identity`, plus `xprop` for the Xwayland clients), resolved through `dragonfruit-app-index`, its decoration tier read from `query decorations`, and its global-menu tier from `dragonfruit-menu-broker`. `pass` means the observed outcome matches the expected outcome below.

| App | Backend | Raw identity | Resolved desktop id | Identity | Decoration (expected) | Menu tier | Launch |
|---|---|---|---|---|---|---|---|
| Firefox (X11) | x11 | `org.mozilla.firefox` | `org.mozilla.firefox.desktop` | pass | CSD (CSD) pass | none pass | pass |
| xterm | x11 | `XTerm` | `xterm.desktop` | pass | SSD (SSD) pass | none pass | pass |
| Steam (X11 stand-in) | x11 | `Steam` | `steam.desktop` | pass | SSD (SSD) pass | none pass | pass |
| GNOME Calculator (GTK4) | wayland | `org.gnome.Calculator` | `org.gnome.Calculator.desktop` | pass | CSD (CSD) pass | none pass | pass |
| SDL game (SDL2) | wayland | `game.zoo.sdl` | `game.zoo.sdl.desktop` | pass | SSD (SSD) pass | none pass | pass |
| Electron app | wayland | `Electron` | `electron-zoo.desktop` | pass | SSD (SSD) pass | none pass | pass |

Notes:

- **Firefox (X11)** — real native Firefox via Xwayland; GTK draws its own client-side frame
- **xterm** — real xterm (distro package, extracted)
- **Steam (X11 stand-in)** — stand-in: raw X11 window with Steam's WM_CLASS (Steam not installable)
- **GNOME Calculator (GTK4)** — real GTK4/libadwaita app via flatpak
- **SDL game (SDL2)** — real SDL2 game window on the nested Wayland socket; the sample presents frames so SDL attaches its first buffer (T-14.6b fix)
- **Electron app** — real Electron/Chromium client; Chromium negotiates server-side decoration

The global-menu tier is `none` (the fixed application menu) for every zoo app: none of them exports a native or DBusMenu menu in this run, which is the expected fallback. The `dbusmenu`/`native` tiers are exercised by the T-14.4 bridge tests with `--mock-menu`.

