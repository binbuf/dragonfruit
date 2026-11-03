# 0187 — One menu-row anatomy and a glyph vocabulary for shortcut hints

## Status

accepted

## Context

`ContextMenu` and `MenuBarMenu` each carried their own row component. Both
printed the model's `shortcut` string verbatim on the same line as the label:
`Super+Shift+Z`, `Ctrl+Super+F`, `Shift+Cmd+Delete`. The text was long, the
`+` separators read like prose rather than keys, and the row had no reserved
trailing column: a submenu chevron shifted that row's hint left, and a long
label could collide with the hint. The two components could also drift because
their row layout was copied.

The design language already says menus have "right-aligned badges, submenu
chevrons, and shortcut hints" and adapts Apple's Command/Control/Option glyphs
to Dragonfruit's keymap roles ([0122](0122-tahoe-interface-language-across-chrome.md),
[keymap.md](../../keymap.md)). It did not define how a hint is laid out or how
a chord is rendered.

## Decision

- **One row component, `MenuRow.qml`, is shared by `ContextMenu` and
  `MenuBarMenu`.** Its anatomy is a fixed leading check/icon gutter, a flexible
  single-line label that elides, a right-aligned shortcut hint, and a fixed
  trailing chevron gutter. The check and chevron gutters are reserved even when
  empty, so labels and hints align across every row and between the two menu
  components.
- **One hint renderer, `ShortcutHint.qml`.** It parses the `shortcut` field
  into a canonical glyph sequence:
  - Super/Command -> `⌘`, Control -> `⌃`, Option/Alt -> `⌥`, Shift -> `⇧`,
    Function -> `fn`.
  - Key aliases -> `⌫` Delete, `↵` Return, `⎋` Escape, `⇥` Tab, `␣` Space,
    `⇪` Caps Lock, `⇞`/`⇟` Page Up/Down, and the arrow glyphs.
  - Modifiers are reordered to **Fn, Ctrl, Alt, Shift, Super** and followed by
    the key; `+` separators and the input order are never rendered.
  - A non-chord string (the Dock's `Space 2` window-row metadata) falls back to
    plain right-aligned text in the same cell.
- **The glyphs are text, not a new icon vendor.** They are all present in the
  bundled Inter face, so they tint with the row, scale with
  `component.shortcutHint.fontSize`, and need no SVG or build gate. The Super
  role uses the command glyph because the desktop's Command role is the Super
  key ([keymap.md](../../keymap.md)); this is a role label, not Apple artwork.
- **No keycap boxes.** The reference language shows plain right-aligned
  symbols, and boxes add noise at the 26 px menu-row height.
- **Tokens own the hint.** `component.shortcutHint.{gap,fontSize,
  modifierWeight,keyWeight}` is the single source for the hint; row height,
  padding, and the label/hint gap stay in the existing menu token groups.
- **Accessibility keeps the raw chord.** `ShortcutHint` is decorative to
  AT-SPI; the row exposes the original `shortcut` string as its accessible
  description, so a screen reader hears the chord even though the visual form
  is glyphs.

## Consequences

- Context menus, the top bar's dropdowns, submenus, the Dock, Files, Settings,
  and the tray all get the same row geometry and hint vocabulary without
  touching each call site.
- Adding a key alias or a modifier is one edit in `ShortcutHint.qml`; the
  menus cannot disagree.
- Menu-model key names are **data, not localized prose**. First-party models
  carry canonical English names (`Super`, `Delete`, `Return`) and only the row
  label goes through `qsTr()`; the Files menus were normalized in the same
  pass. A translated key word would fall back to plain text rather than a
  glyph.
- The design-system gallery's `Menu` and `ContextMenu` pages show every
  modifier glyph, a function-key chord, a glyph cluster, and the plain-text
  fallback, so the language is reviewable headless.
- A future localization or keyboard-layout change only touches the parser and
  the token group, not the menu components.
