# Brand and OS-menu marks

The editable source artwork for the menu bar's OS menu glyph.

| File | Mark |
|---|---|
| `dragonfruit.svg` | The Dragonfruit brand mark (the shipped default). |
| `tux.svg` | The alternate minimal penguin mark offered in System Settings → Menu Bar. |

Both files are compiled into the design-system QML module at the stable prefix
`qrc:/icons/brand/<name>.svg`. The menu bar cannot tint an `Image` reliably
under every scene-graph backend, so `DragonfruitLogo.qml` and `TuxLogo.qml`
render the paths through the generated `design-system/BrandGlyphs.qml`
registry.

Edit the SVGs here, then run:

```sh
./scripts/gen-brand-glyphs.py
```

`make check-brand` fails if the generated registry is stale. `TuxLogo.qml`
paints every source path in the menu bar foreground, so internal detail paths
flatten into one monochrome mark that takes the bar's wallpaper foreground,
contrast shadow, and hover/open plate.
