# 0085 — Real `xdg-desktop-portal` frontend compatibility

## Status

accepted

## Context

T-13.1a…T-13.6 built the Dragonfruit portal backend and proved every
interface against a private `dbus-daemon` with a test presenter. T-13.7 is
the first task to run the **real** `xdg-desktop-portal` frontend and a
**real Flatpak** caller against the backend. That surfaced two
incompatibilities the private-bus tests could not see, because the tests
called the backend's methods directly and parsed its own data files:

1. **The discoverability files used `;` for comments.** GLib key files (the
   format `xdg-desktop-portal` parses `.portal` and `*-portals.conf` with)
   treat `#`, not `;`, as the comment character; a `;` line is a parse error.
   The frontend still loaded the files (warning-level failure), so nothing
   caught it.
2. **The implementation interfaces exported their version as `Version`.**
   The standard spells `AvailableSourceTypes`/`AvailableCursorModes` in
   Pascal case but `version` in lower case. zbus names a property after the
   Rust method by default, so the backend served `Version`; the frontend's
   generated proxy read `version`, saw 0, and silently disabled the features
   gated on it: ScreenCast `AvailableCursorModes` binding (cursor
   `SelectSources` was then rejected), the Screenshot `uri` result, and
   session persistence.

## Decision

- **The `.portal` descriptor and `*-portals.conf` use `#` comments.** The
  embedded files are the packaging source (`portal::data::install_into`), so
  the fix lands once for the shipped and the tested copy.
- **The `org.freedesktop.impl.portal.*` implementation interfaces export the
  standard lower-case `version` property.** Each `#[zbus(property)]` getter
  names it explicitly (`name = "version"`); FileChooser keeps no version
  property, matching the standard (its extra `Version` is gone).
- **A test pins the wire name.** `portal/tests/portals.rs`'s
  `the_standard_interfaces_expose_the_lowercase_version_property` reads
  `version` (and rejects `Version`) for Settings, Screenshot, ScreenCast, and
  GlobalShortcuts, so a regression fails without a Flatpak or a frontend.
- **The real-frontend walkthrough is a capture script, not a CI test.**
  `scripts/capture-portals.sh` stands up a private session bus, the real
  frontend, the backend, `settingsd`, and a host presenter, then runs a
  Python driver *inside* `flatpak run org.mozilla.firefox`. It writes
  `docs/captures/t13-portals.txt` (the round-trips) and `t13-portals.png`
  (the shell picker answering a Flatpak FileChooser). It is host-only and
  not part of `make e2e`.

## Consequences

- Any later portal interface must name its `version` property `version`, not
  rely on the zbus default, and must comment the data files with `#`.
- The frontend now reports ScreenCast version 3, `AvailableCursorModes` 3,
  and Screenshot version 2 to callers; the features those gate are live.
- The Flatpak validation is reproducible on a host with Flatpak but is not a
  gate; the in-repo private-bus tests remain the regression net. A future
  task can lift the driver into a Flatpak-based CI image if one exists.
- Clipboard has no portal (ADR 0082); the Flatpak clipboard path is the
  sandbox's Wayland data-device proxy, covered by T-13.5a's compositor
  conformance rather than this capture.