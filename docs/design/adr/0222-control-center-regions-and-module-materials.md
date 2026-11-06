# 0222 — Control Center outer, module and expanded-list materials are separate regions

- Status: accepted
- Date: 2026-11-06
- Task: T203

## Context

The Control Center was one Popup-material rectangle with every tile painted on
top as an opaque `surfaceSunken` block. The reference is three materials: a
tall outer plate, each module capsule, and a darker expanded list. The measured
Wi-Fi Light fit (0.391/0.317) is the *effective* response of a module on the
outer plate. The compositor's flat-band shader output is opaque, so applying
the effective alpha as a second layer over the 0.265/0.373 outer would compound
the alpha and miss the observed composite.

## Decision

- The Control Center declares independently keyed material regions on its
  `control-center` layer surface: one `outer` region (`controlCenterOuter`),
  one `module-*` region per visible tile (`controlCenterModule`), and an
  `expanded-wifi` / `expanded-bluetooth` region (`controlCenterExpanded`) when
  a list is open. Regions are clipped to the scroll viewport and re-declared
  when the panel scrolls or expands.
- The Light module preset carries the measured effective 0.391/0.317 directly.
  The compositor draws the outer region first and the opaque module region
  after it, so the module's final composite in its own region is the measured
  fit; the outer plate remains visible around the capsules. It is not a second
  alpha layered over the outer plate.
- The dark module has no reliable decomposition; it is an adapted effective
  0.65/0.44 so the opaque module reads darker than the 0.489/0.677 dark outer.
- The dark expanded list is near-opaque 0.96/0.13, keeping its composite within
  0.04 luma over white/mid/black. The unmeasured light list is a readable
  near-opaque light plate (0.94/0.96).
- All three presets use the measured 8 pt blur, 0.30 rim and 6 pt / 24 pt
  capsule-end lens family. The compositor's flat-band path consumes the
  resolved own-layer values; QML paints the same values only on the solid,
  software and Reduce Transparency paths.
- `MAX_PANEL_REGIONS` rises from 8 to 32 because the panel has 19 module
  capsules plus outer and up to two expanded lists. The bound stays explicit
  and the conformance test still rejects an over-limit list.

## Consequences

- A module or list change re-declares only the affected region set; stable
  keys keep the per-panel cache reusable.
- Legacy rollout keeps the old single-rect paint. Replacement hides the QML
  fills on the sampled path, so tint/rim is assigned once.
- Increase Contrast raises the resolved rim; Reduce Transparency resolves the
  shared opaque plate and disables sampling for every region.
- The expanded lists are view state only. Wi-Fi network rows are a read-only
  adapter projection; Bluetooth device rows keep their existing write action.
