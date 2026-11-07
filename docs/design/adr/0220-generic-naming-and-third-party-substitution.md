# 0220 — Generic naming and third-party substitution

## Status

accepted — governs user-facing names, assets, and optional dependencies. T220
landed the naming gate and asset manifest; the eight open questions are decided
in [ADR 0229](0229-naming-and-optional-service-decisions.md).

## Context

Dragonfruit reproduces the macOS interaction model. Apple names, icons, sounds,
fonts, wallpapers, and code are not part of the product. Some macOS features
have a mature Linux equivalent with a different license; others have no
permissive equivalent and must be omitted or delegated.

Without a written policy, contributors will copy Apple terminology into UI
strings, ship an asset with an unknown license, or link a GPL component into
the MIT codebase.

## Decision

- **Generic user-facing names.** Use the map in
  `docs/design/interface-naming-and-licensing.md`: Files, Applications, Search,
  Overview, Workspaces, Stage, Quick Settings, Notifications, Preview, Backup,
  Disk Encryption, Fingerprint, Nearby Transfer, etc. Internal code names may
  remain for continuity but must not leak into the UI.
- **Original or permissively licensed assets.** Tabler (MIT) and Phosphor
  (MIT) icons, Inter (OFL-1.1), an original default wallpaper, original sounds,
  and a permissively licensed cursor theme. No Apple bitmaps, SF Symbols,
  sounds, fonts, or wallpapers.
- **Third-party services are separate works.** GPL/LGPL services run as
  separate processes or are dynamically linked with notices. No GPL code is
  copied into MIT files and no GPL library is statically linked.
- **Optional means optional.** The core desktop works without any GPL/optional
  component. Every optional dependency has an absence path and a packaging
  marker.
- **License verification is a packaging gate.** The candidate table records the
  expected license; the actual package version is verified before release.
- **Review gate.** `scripts/check-desktop-names.sh` covers user-facing strings; the
  asset manifest and `NOTICE` are updated in the same change.

## Consequences

- The Settings catalog and shell strings use the generic map; a later string
  audit can find Apple terms.
- The first release may omit AirDrop/Continuity, iCloud, Siri, Screen Time,
  Wallet, Game Center, Find My, and App Store rather than ship a poor clone.
- Optional integrations (LocalSend/KDE Connect, restic/Pika/Timeshift, GOA,
  Tracker/Baloo/Recoll, Orca, ibus/fcitx5) are declared and tested for absence.
- Packaging carries the license/notice burden; the MIT codebase stays clean.
- A legal review is required before the first public release.
