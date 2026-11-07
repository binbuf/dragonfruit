# T202 — menu families

Model and capture evidence for the attached/detached/status menu presets. Live stills are produced by `make menu-families-capture`.

## Light attached File

| metric | resolved | target | delta |
|---|---|---|---|
| alpha | 0.552 | 0.550 | +0.002 |
| blur | 10.0 pt | 10.0 pt | +0.0 pt |
| shadow extent | 20 pt | 20 pt | +0 pt |
| bottom corner | 12.5 pt | 12.5 pt | — |

## Dark File decision

- Rejected crop: captures/coverage/dark_menu_file__neutral_white.png (pixel-identical to the Light File crop)
- Valid rows: mid, black
- White row: open
- calibrated the attached baseline against the valid dark context/status family; a controlled white-row recapture remains open

## Fallback

- Trusted shell menu: sampled-glass
- App-owned menu: software-fallback (alpha 0.980)
- Reduce Transparency: solid

## Motion

- Close 183 ms, terminal scale 0.95; Reduce Motion collapses it.

## Captures

- `docs/captures/t202-attached-dark.png`
- `docs/captures/t202-attached-light-high.png`
- `docs/captures/t202-attached-light-low.png`
- `docs/captures/t202-attached-light.png`
- `docs/captures/t202-context-desktop.png`
- `docs/captures/t202-context-dock.png`
- `docs/captures/t202-context-files.png`
- `docs/captures/t202-status-bluetooth.png`
- `docs/captures/t202-status-wifi.png`
