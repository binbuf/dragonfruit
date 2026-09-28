# 0123 — The Sound pane and tile ride the audio bridge host; Sound Effects and Balance are settingsd keys

## Status

accepted

## Context

T-15.3a grew the `dragonfruit-audio` adapter with input devices and
default-device routing ([0121](0121-sound-routing-adapter.md)), but nothing
consumed the new half: `services/system-status` still serialised only `sinks`
in `audio_view`, the Settings app had no Sound pane, and the Control Center
Sound tile reflected only volume/mute. T-15.3b ships the Settings pane and the
Control Center tile as one functional unit (the no-half-panes rule).

The macOS reference pane has two groups (`docs/reference/System_Preferences.md`
§ Sound): `Sound Effects` (alert sound, play effects through, alert volume,
three playback toggles) and `Output & Input` (Output/Input tabs, a device table
with `Name`/`Type`, output volume + mute, and balance). Device selection, output
volume, and mute map cleanly to the PipeWire/WirePlumber adapter. The task file
left the `Sound Effects` and `Balance` providers "TBD (T-15.x)", but no later
T-15 task owns them and the track forbids dead controls.

## Decision

- **The routing half rides the existing bridge host.** `StatusHost` gains
  `set_default_sink(id)` / `set_default_source(id)` and `audio_view` carries
  `sources`, `sourceCount`, and `defaultSource`. The `Audio` interface adds
  `SetDefaultSink(id)` / `SetDefaultSource(id)`. The Settings app gets its own
  `SoundClient` seam (live `DbusSoundClient` + `DF_SOUND_FIXTURE` mock) exactly
  like Bluetooth/Storage ([0118](0118-bluetooth-pane-and-tile.md),
  [0120](0120-storage-pane-and-tile.md)); that client owns the two routing
  writes and the volume/mute writes. The shell's `SystemStatusClient` only
  widens its decoded `audio` view (it never writes routing; the Control Center
  tile reflects the default output device). Device selection makes the chosen
  node the default; the existing volume/mute writes then target it.
- **The `Sound Effects` and `Balance` rows become settingsd keys.** A new
  `sound` `KeyGroup` and seven keys (`sound.alertSound`,
  `sound.playEffectsThrough`, `sound.alertVolume`, `sound.playOnStartup`,
  `sound.uiEffects`, `sound.volumeFeedback`, `sound.balance`) are additive in
  schema revision 11, owned by `apps/settings`, defaulted off/at the reference
  values. This is the T-08 host-services provider for the rows the reference
  draws, so every shipped control persists and applies live.
- **The alert/UI-sound playback engine is the one deferred item.** The pane
  chooses and persists the preference; actually synthesising an alert chime or
  UI click is a follow-up. The captured preview button and the `?` help are
  omitted (ADR [0122](0122-tahoe-interface-language-across-chrome.md)).
- **Absence is the adapter's two hide rules.** WirePlumber absent or no device
  hides `Output & Input` and shows the pane's note; the settingsd-backed rows
  stay live on the schema defaults. Input volume has no adapter write, so the
  Input tab shows only the device table — no inert slider.

## Consequences

- `docs/settings-keys.md` and the C++ schema-defaults mirror move to revision
  11 in lockstep (`schema_doc.rs` enforces it).
- Per-application stream routing, input volume/mute, and balance application to
  the graph remain unimplemented; adding them means extending the same
  `AudioSource` seam and the pane, not the bridge shape.
- The Control Center Sound tile stays compact: it reflects the default output
  device but does not enumerate or switch devices (matching macOS); routing is
  the Settings pane's job.