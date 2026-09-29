# 0176 — The T-17.3 visual-floor sign-off: one live sweep and a recorded approximation waiver

## Status

accepted

## Context

The premium gate needs the visual floor signed off: materials (blur, rounding,
shadows) present and correct in light and dark, every animation with a passing
reduced-motion variant, and no flat approximations left in shipped chrome. The
material contract is already frozen — the token-driven backdrop pass
([0013](0013-backdrop-blur-pass.md)), the Dock's own glass layers
([0102](0102-dock-material-role-and-qml-glass-layers.md)), a reduced-motion
sweep ([0155](0155-compositor-magnifier.md)), and the Tahoe interface language
([0122](0122-tahoe-interface-language-across-chrome.md)) — and the design-system
gallery `--strict` gate already enforces the component goldens, including their
`_reduced` twins. What was missing was one **premium-gate review artifact** that
compares the *live* chrome, not just components, in both schemes and under
reduced motion, without re-deriving the material language per ticket.

Two questions had to be settled: how the sign-off is captured and reviewed, and
how to treat ADR 0013/0091's explicitly deferred GPU blur under a checklist item
that says "no flat approximations remain".

## Decision

- The visual floor is signed off by a scripted, single-session capture:
  `scripts/capture-t17-visual-floor.sh` (`make t17-visual-floor-capture`) runs
  the nested demo once and `scripts/t17-visual-floor-driver.py` captures dark,
  light, and dark+reduced-motion chrome. One session is deliberate: the same
  compositor state produces every variant, so the comparison cannot drift
  between launches.
- The capture records the live `query material`/`query degrade` tones per
  variant into `docs/captures/t17-visual-floor.txt`, and captures the whole
  desktop plus the menu-bar band, the compositor SSD titlebar, and the Dock
  band.
- The review sheet `docs/captures/t17-visual-floor-gallery.png` pairs each
  captured desktop with the `window_*` golden and each captured titlebar with
  the `ssd_*` golden; `t17-visual-floor-menubar.png`/`-dock.png` stack the
  chrome bands across variants. The review notes live in
  `docs/captures/t17-visual-floor.md`.
- Reduced motion is signed off by three independent, already-green layers — the
  structural sweep, the `*_dark_reduced` goldens under `--strict`, and the
  one-frame-per-transition runtime conformance suites — plus the live fact that
  the reduced-motion still is pixel-identical to the dark still.
- **The "no flat approximations" item is waived, explicitly.** No untokened
  flat fill remains; every chrome surface resolves through `Theme.material` and
  the backdrop pass. But the backdrop blur is the token-driven feather
  approximation of ADR 0013/0091/0122 because the renderer has no GPU sampler;
  ADR 0122 already names the true refraction pass as a future material-track
  item. The waiver is recorded rather than silently implied.

## Consequences

- The premium gate has a reproducible live-floor artifact and a written verdict;
  the human sign-off (watch the demo, compare to the goldens) stays batched at
  the track boundary (see [SLICING-REVIEW.md](../SLICING-REVIEW.md)).
- The capture reuses the T-17.1a active-window capturer (KWin raise +
  `spectacle -a`) and lives outside `make e2e`; the automated floor is the sweep
  plus the gallery `--strict` gate.
- The GPU blur/refraction sampler is a named post-gate backlog item; when it
  lands it replaces the feather geometry behind the same tokens and this ADR's
  waiver is retired.
- The live window menu could not be opened on this host's X11/Qt SSD clients;
  the popup material is covered by the `menu_*`/`popup_*` goldens and the gap is
  a recorded follow-up, not a silent omission.