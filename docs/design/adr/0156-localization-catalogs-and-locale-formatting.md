# 0156 — Localization catalogs, runtime loading, and locale formatting

## Status

accepted

## Context

The T-16 platform-polish track asks for "shell and first-party apps
translatable; locale formatting"
([16-platform-polish-packaging.md](../tracks/16-platform-polish-packaging.md)).
The shell, Settings, Files, and the shared design-system components already
externalize ~909 user-visible strings with Qt's `qsTr()`/`tr()`, but nothing
installed a translator: every surface rendered its source (English) string, and
there was no catalog, no locale switch, and no gate to keep an unwrapped
literal out.

The obvious Qt path is `lupdate` + `lrelease` (`.ts` → binary `.qm`). Neither
tool ships in this workspace's pinned Qt toolchain, and the build must stay
hermetic and CI-safe; adding a Linguist dependency to every dev machine for one
feature is the wrong trade.

## Decision

- **The catalog format is Qt `.ts` XML, checked in under `translations/`.** It
  is the standard lupdate format, so it stays compatible with the usual tooling
  if it is ever adopted, but the workspace never runs `lupdate`/`lrelease`.
- **`dragonfruit-i18n` parses the catalog at runtime.** One small static
  library (`libs/i18n/`) owns locale resolution, a `QTranslator` subclass that
  reads `.ts` with `QXmlStreamReader`, the application install call, and the
  catalog search order. Every first-party entry point (shell, Settings, Files)
  calls `Dragonfruit::installTranslations(app)` after the application object
  exists and before any QML loads. Lookup tries `(context, source)` first and
  falls back to `source` text alone, so a context-less catalog still works and
  a string with no entry keeps its source text (a partial catalog is always
  readable).
- **Catalog search order** is `$DRAGONFRUIT_TRANSLATIONS_DIR`, the compiled-in
  source tree (so `make demo`/`make dev` are translated from a checkout), then
  `../share/dragonfruit/translations` beside the executable and the system
  install prefixes. `cmake --install` lays the `.ts` files under
  `${CMAKE_INSTALL_DATADIR}/dragonfruit/translations`; the distro packaging
  tasks install the same tree.
- **The effective locale is `DRAGONFRUIT_LOCALE` when set, else
  `QLocale::system()`** (which already honors `LANG`/`LC_*`). The chosen locale
  is installed with `QLocale::setDefault`, so `Qt.locale()` in QML and
  `QLocale()` in C++ format dates, times, sizes, and numbers for the same
  language. Locale is selected at process start.
- **The string-extraction gate is the invariant, not full coverage.**
  `scripts/i18n-extract.py` extracts every `qsTr()`/`tr()`/
  `QCoreApplication::translate()` string from `shell/`, `apps/`, and
  `design-system/components` (excluding tests), and fails when the sources and
  the checked-in template `translations/dragonfruit.ts` disagree or when a
  user-visible QML property holds a bare literal. It is wired into `make lint`
  and CI; `make i18n-update` regenerates the template.
- **`translations/dragonfruit_es.ts` is the reference translated locale** used
  by the locale-switch capture `scripts/capture-t16-i18n.sh` and the
  `tst_i18n` integration test. Product names that are the same in Spanish
  (Bluetooth, Mission Control, General) intentionally stay identical.

## Consequences

- Adding a user-visible string fails CI until it is externalized and the
  template is refreshed; adding a locale is a new `dragonfruit_<lang>.ts` file
  with no build change.
- Only startup locale selection is supported. In-place runtime switching would
  need `QQmlEngine::retranslate()` plus a language-change event; it is not
  required by this task.
- Spanish coverage is deliberately partial (chrome, Control Center, Dock,
  Files, session/portal dialogs, and the General pane). Untranslated strings
  fall back to English; the mechanism is what this slice lands, and later
  translations are additive.
- RTL layout polish beyond the components' built-in support is deferred, as
  the task states.