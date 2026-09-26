# Inter — the Dragonfruit system font

Inter is the desktop's system font. Every first-party surface (menu bar,
Dock, window chrome, Settings, Files, the component gallery, lock screen,
OSD, menus and dialogs) renders in it, the role SF Pro plays on macOS.
It is an open-source sans-serif designed for user interfaces.

| | |
|---|---|
| Upstream | [github.com/rsms/inter](https://github.com/rsms/inter) |
| Version | 4.001 (`git-66647c0bb`, variable `opsz` + `wght`) |
| Source | Google Fonts variable build, 2025-09-10 |
| License | SIL OFL 1.1 — [../../LICENSES/OFL-1.1.txt](../../LICENSES/OFL-1.1.txt) |

| File | Use |
|---|---|
| `Inter-VariableFont_opsz,wght.ttf` | Roman, weights 100–900 |
| `Inter-Italic-VariableFont_opsz,wght.ttf` | Italic, weights 100–900 |

The faces are compiled into `libs/system-font` as Qt resources and installed
as the process-wide application font by `Dragonfruit::installSystemFont()`,
called once by each first-party entry point before QML loads. The family name
is declared as `primitive.font.family` in the design tokens. Packaging will
additionally install the faces system-wide (fontconfig) so third-party apps
inherit them; that is deliberately not wired up yet.