// SPDX-License-Identifier: MIT
//! The audio snapshot the menu bar renders, decoded from one raw read.
//!
//! The model owns the aggregation a consumer should not repeat: it marks the
//! default sink, exposes its linear volume and mute, and derives the glyph and
//! label the status slot draws. T-07.5 renders [`AudioSnapshot::glyph`], the
//! volume slider from [`AudioSnapshot::level`], and the
//! [`AudioSnapshot::sinks`] list.

use crate::source::{AudioData, SinkData};

/// One sink the menu bar lists.
#[derive(Debug, Clone, PartialEq)]
pub struct Sink {
    /// The PipeWire node id.
    pub id: u32,
    /// The stable `node.name`.
    pub name: String,
    /// The human label (`node.description`).
    pub description: String,
    /// The linear volume, 0..=1.
    pub volume: f32,
    /// Whether the sink is muted.
    pub muted: bool,
    /// Whether this is the current default sink.
    pub default: bool,
}

impl Sink {
    /// The volume as the 0–100 integer the menu shows.
    pub fn volume_percent(&self) -> u8 {
        percent(self.volume)
    }
}

/// The audio snapshot a menu bar renders: the sink list and the default sink.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioSnapshot {
    /// Every sink, default first then discovery order.
    pub sinks: Vec<Sink>,
    /// `node.name` of the default sink, when one is set.
    pub default_sink: Option<String>,
}

impl AudioSnapshot {
    /// Build the snapshot from one raw read: mark the default sink (falling
    /// back to the first sink when WirePlumber named none) and clamp volumes.
    pub fn from_data(data: &AudioData) -> Self {
        let mut sinks: Vec<Sink> = data.sinks.iter().map(sink_from_data).collect();
        let default_name = resolved_default(data, &sinks);
        for sink in &mut sinks {
            sink.default = Some(&sink.name) == default_name.as_ref();
        }
        // The default sink sorts first so the menu header is stable.
        sinks.sort_by_key(|sink| !sink.default);
        AudioSnapshot {
            sinks,
            default_sink: default_name,
        }
    }

    /// The default sink, when the graph has one.
    pub fn default_sink(&self) -> Option<&Sink> {
        self.sinks.iter().find(|sink| sink.default)
    }

    /// The default sink's linear volume, or `0.0` when there is no sink.
    pub fn volume(&self) -> f32 {
        self.default_sink().map(|sink| sink.volume).unwrap_or(0.0)
    }

    /// Whether the default sink is muted.
    pub fn muted(&self) -> bool {
        self.default_sink().map(|sink| sink.muted).unwrap_or(false)
    }

    /// The default sink's volume as 0–100.
    pub fn volume_percent(&self) -> u8 {
        percent(self.volume())
    }

    /// The level a slider draws, 0..=1 (the linear volume).
    pub fn level(&self) -> f32 {
        self.volume()
    }

    /// The glyph the status slot draws (`StatusGlyph.qml` names).
    pub fn glyph(&self) -> &'static str {
        if self.muted() || self.volume() <= 0.0 {
            "volume-muted"
        } else {
            "volume"
        }
    }

    /// A one-line label for the menu bar / menu header.
    pub fn label(&self) -> String {
        if self.default_sink().is_none() {
            return "No output device".to_owned();
        }
        if self.muted() {
            return "Muted".to_owned();
        }
        format!("{}%", self.volume_percent())
    }

    /// The number of sinks in the graph.
    pub fn sink_count(&self) -> usize {
        self.sinks.len()
    }
}

/// Map one raw sink, clamping its volume to the renderable range.
fn sink_from_data(data: &SinkData) -> Sink {
    Sink {
        id: data.id,
        name: data.name.clone(),
        description: data.description.clone(),
        volume: data.volume.clamp(0.0, 1.0),
        muted: data.muted,
        default: false,
    }
}

/// The default sink name, falling back to the first sink when none is named.
fn resolved_default(data: &AudioData, sinks: &[Sink]) -> Option<String> {
    if let Some(name) = &data.default_sink {
        if sinks.iter().any(|sink| &sink.name == name) {
            return Some(name.clone());
        }
    }
    sinks.first().map(|sink| sink.name.clone())
}

/// Map a linear volume to the 0–100 integer the menu shows.
fn percent(volume: f32) -> u8 {
    (volume.clamp(0.0, 1.0) * 100.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sink(name: &str, volume: f32, muted: bool) -> SinkData {
        SinkData {
            id: 1,
            name: name.to_owned(),
            description: format!("{name} description"),
            volume,
            muted,
        }
    }

    fn data(default: Option<&str>, sinks: Vec<SinkData>) -> AudioData {
        AudioData {
            default_sink: default.map(str::to_owned),
            sinks,
        }
    }

    #[test]
    fn the_default_sink_is_marked_and_sorted_first() {
        let snapshot = AudioSnapshot::from_data(&data(
            Some("b"),
            vec![sink("a", 0.2, false), sink("b", 0.6, false)],
        ));
        assert_eq!(snapshot.default_sink.as_deref(), Some("b"));
        assert_eq!(snapshot.sink_count(), 2);
        assert_eq!(snapshot.sinks[0].name, "b");
        assert!(snapshot.sinks[0].default);
        assert!(!snapshot.sinks[1].default);
        assert_eq!(snapshot.volume_percent(), 60);
        assert_eq!(snapshot.glyph(), "volume");
        assert_eq!(snapshot.label(), "60%");
    }

    #[test]
    fn a_missing_default_falls_back_to_the_first_sink() {
        let snapshot = AudioSnapshot::from_data(&data(
            Some("ghost"),
            vec![sink("a", 0.3, false), sink("b", 0.4, false)],
        ));
        assert_eq!(snapshot.default_sink.as_deref(), Some("a"));
        assert!(snapshot.sinks[0].default);
    }

    #[test]
    fn a_named_default_that_is_not_named_at_all_still_resolves() {
        let snapshot = AudioSnapshot::from_data(&data(None, vec![sink("only", 0.5, false)]));
        assert_eq!(snapshot.default_sink.as_deref(), Some("only"));
    }

    #[test]
    fn mute_switches_the_glyph() {
        let snapshot = AudioSnapshot::from_data(&data(Some("a"), vec![sink("a", 0.6, true)]));
        assert!(snapshot.muted());
        assert_eq!(snapshot.glyph(), "volume-muted");
        assert_eq!(snapshot.label(), "Muted");
        assert_eq!(snapshot.level(), 0.6);
    }

    #[test]
    fn a_zero_volume_reads_as_the_muted_glyph() {
        let snapshot = AudioSnapshot::from_data(&data(Some("a"), vec![sink("a", 0.0, false)]));
        assert!(!snapshot.muted());
        assert_eq!(snapshot.glyph(), "volume-muted");
        assert_eq!(snapshot.label(), "0%");
    }

    #[test]
    fn an_empty_graph_has_no_output_device() {
        let snapshot = AudioSnapshot::from_data(&data(None, vec![]));
        assert_eq!(snapshot.sink_count(), 0);
        assert!(snapshot.default_sink().is_none());
        assert_eq!(snapshot.volume(), 0.0);
        assert_eq!(snapshot.glyph(), "volume-muted");
        assert_eq!(snapshot.label(), "No output device");
    }

    #[test]
    fn an_out_of_range_volume_is_clamped() {
        let snapshot = AudioSnapshot::from_data(&data(Some("a"), vec![sink("a", 4.0, false)]));
        assert_eq!(snapshot.volume(), 1.0);
        assert_eq!(snapshot.volume_percent(), 100);
    }
}
