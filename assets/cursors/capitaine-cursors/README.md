# Capitaine Cursors (vendored)

This directory contains the Dragonfruit system cursor theme, derived from
[Capitaine cursors](https://github.com/keeferrourke/capitaine-cursors) by
Keefer Rourke. The desktop uses it for every compositor-drawn cursor
(`compositor/src/cursor_theme.rs`), so the pointer, text, resize, and drag
cursors match the rest of the visual language.

## Provenance

- Upstream: <https://github.com/keeferrourke/capitaine-cursors>
- Upstream commit: `06c88433662a4004cf56a6e471b523a0a8880be0` (2021-03-03)
- Theme variant: `capitaine-cursors` (dark), the main pre-built theme from the
  upstream project as packaged by Arch Linux (`capitaine-cursors 4-3`).
- License: LGPL-3.0-or-later. See [`COPYING`](COPYING) for the upstream
  copyright notice and license text.

## Modifications

Only the compiled XCursor files are vendored, not the Inkscape/SVG sources.
The files were reduced to keep the repository small:

- the nominal sizes 24, 48, and 96 pixels are kept (the compositor chooses the
  closest size for the output scale and scales down only when necessary);
- animated cursors (`wait`, `progress`, ...) keep one frame per nominal size;
  the current cursor renderer draws the first frame and does not animate it;
- the upstream XCursor aliases (`left_ptr`, `hand2`, `xterm`, `watch`, …)
  are kept as relative symlinks so Qt, GTK, and Xwayland clients that look up
  legacy names still resolve the same shapes;
- no pixel data was changed.

The upstream SVG sources, `build.sh`, and the full-size pre-built theme are
available at the upstream commit above. To regenerate this directory, build the
theme with `./build.sh -t dark` upstream and keep the three sizes above.

## Runtime lookup

The compositor looks for `capitaine-cursors` in the standard XCursor theme
directories (`$XCURSOR_PATH`, `$XDG_DATA_HOME/icons`, `$XDG_DATA_DIRS/icons`,
`/usr/share/icons`) and in this in-repo directory for a development build.
`DRAGONFRUIT_CURSOR_THEME` overrides the theme name and
`DRAGONFRUIT_CURSOR_SIZE` overrides the logical cursor size (default 24).

Only the dark variant is vendored. The upstream project also publishes
`capitaine-cursors-light`; install it in one of the directories above and set
`DRAGONFRUIT_CURSOR_THEME=capitaine-cursors-light` to use it.
