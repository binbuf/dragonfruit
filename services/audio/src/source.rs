// SPDX-License-Identifier: MIT
//! The transport seam: the raw PipeWire/WirePlumber read/write and its mock.
//!
//! An [`AudioSource`] is the only thing that talks to the daemon. The live
//! source is [`crate::CommandAudio`] (WirePlumber's `pw-dump`/`wpctl` CLI);
//! tests and CI use [`MockAudio`], which serves a fixture with no daemon on
//! the machine. The adapter ([`crate::AudioAdapter`]) turns one raw read into
//! the typed snapshot and drives the shared `Subscription`.
//!
//! Decoding stops here: volume is the linear 0..=1 value the menu bar shows,
//! already un-cubed from WirePlumber's stored channel volume, so nothing above
//! the adapter repeats the session-manager arithmetic.

use dragonfruit_system_adapters::AdapterError;

/// The result of one PipeWire/WirePlumber read.
///
/// This is deliberately flat and daemon-shaped: the live source builds it from
/// a `pw-dump` snapshot, a fixture parses straight into it, and nothing above
/// the adapter ever sees it.
///
/// Output nodes (`Audio/Sink`) and input nodes (`Audio/Source`) are both read,
/// with their separate default routing metadata (`default.audio.sink` /
/// `default.audio.source`). T-15.3a adds the input half and the default-device
/// switches; the volume half is unchanged from T-07.3.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioData {
    /// `node.name` of the default sink, from WirePlumber's `default` metadata
    /// (`default.audio.sink`); `None` when no default is set.
    pub default_sink: Option<String>,
    /// `node.name` of the default source, from WirePlumber's `default`
    /// metadata (`default.audio.source`); `None` when no default is set.
    pub default_source: Option<String>,
    /// Every sink node (`media.class` = `Audio/Sink`) in the graph.
    pub sinks: Vec<SinkData>,
    /// Every source node (`media.class` = `Audio/Source`) in the graph.
    pub sources: Vec<SourceData>,
}

/// One PipeWire sink node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SinkData {
    /// The PipeWire node id.
    pub id: u32,
    /// `node.name`, the stable identifier used to match the default.
    pub name: String,
    /// `node.description` (falling back to `node.nick`), the human label.
    pub description: String,
    /// The linear volume, 0..=1, un-cubed from `Props.channelVolumes`.
    pub volume: f32,
    /// `Props.mute`.
    pub muted: bool,
}

/// One PipeWire source (input) node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceData {
    /// The PipeWire node id.
    pub id: u32,
    /// `node.name`, the stable identifier used to match the default.
    pub name: String,
    /// `node.description` (falling back to `node.nick`), the human label.
    pub description: String,
    /// The linear volume, 0..=1, un-cubed from `Props.channelVolumes`.
    pub volume: f32,
    /// `Props.mute`.
    pub muted: bool,
}

/// The result of one write (volume, mute, or a default-device switch).
///
/// These are explicit user actions (`wpctl set-volume` / `set-mute` /
/// `set-default`), never a poll. A write that lands does not invent a
/// snapshot: the daemon pushes the resulting state and the host re-reads the
/// adapter, so the snapshot stays the single source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetOutcome {
    /// WirePlumber applied the change.
    Applied,
    /// The daemon is absent; there is nothing to control.
    Absent,
    /// The write failed for any other reason.
    Failed(AdapterError),
}

/// Reads and controls audio over some transport.
///
/// The read result is a three-way answer, exactly as the adapter contract
/// needs it:
///
/// * `Ok(Some(data))` — the daemon answered; `data` is the live read.
/// * `Ok(None)` — the daemon is absent. A normal state; the slot hides.
/// * `Err(error)` — the daemon is present but the read failed; the slot shows
///   visible and inert with the message.
///
/// The source is never polled by a consumer: the host calls
/// [`AudioAdapter::refresh`](crate::AudioAdapter::refresh) when WirePlumber
/// signals a change.
pub trait AudioSource {
    /// One read of the daemon.
    fn read(&mut self) -> Result<Option<AudioData>, AdapterError>;

    /// Set the default sink's linear volume (0..=1). The one volume write.
    fn set_volume(&mut self, volume: f32) -> SetOutcome;

    /// Set the default sink's mute state. The one mute write.
    fn set_mute(&mut self, muted: bool) -> SetOutcome;

    /// Route playback to the sink with this PipeWire node id: the output
    /// device switch (`wpctl set-default <id>`). One explicit write.
    fn set_default_sink(&mut self, id: u32) -> SetOutcome;

    /// Route capture to the source with this PipeWire node id: the input
    /// device switch (`wpctl set-default <id>`). One explicit write.
    fn set_default_source(&mut self, id: u32) -> SetOutcome;
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`AudioData`] with no PipeWire, and
/// `kill`/`restart` exercise absence and re-subscribe the way masking the real
/// daemon would. Its writes mutate the simulated graph the way WirePlumber
/// would, so a subsequent `refresh` sees the change.
#[derive(Debug, Clone, PartialEq)]
pub struct MockAudio {
    present: bool,
    data: Option<AudioData>,
    failure: Option<AdapterError>,
    write_failure: Option<AdapterError>,
    reads: u32,
    volume_writes: u32,
    mute_writes: u32,
    default_sink_writes: u32,
    default_source_writes: u32,
}

impl MockAudio {
    /// A daemon that is not running.
    pub fn absent() -> Self {
        MockAudio {
            present: false,
            data: None,
            failure: None,
            write_failure: None,
            reads: 0,
            volume_writes: 0,
            mute_writes: 0,
            default_sink_writes: 0,
            default_source_writes: 0,
        }
    }

    /// A running daemon that answers with `data`.
    pub fn present(data: AudioData) -> Self {
        MockAudio {
            present: true,
            data: Some(data),
            failure: None,
            write_failure: None,
            reads: 0,
            volume_writes: 0,
            mute_writes: 0,
            default_sink_writes: 0,
            default_source_writes: 0,
        }
    }

    /// A running daemon that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockAudio {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            write_failure: None,
            reads: 0,
            volume_writes: 0,
            mute_writes: 0,
            default_sink_writes: 0,
            default_source_writes: 0,
        }
    }

    /// Make every write fail with `message` (reads are unaffected).
    pub fn fail_writes(&mut self, message: impl Into<String>) {
        self.write_failure = Some(AdapterError::new(message));
    }

    /// Restore the default successful-write behavior.
    pub fn allow_writes(&mut self) {
        self.write_failure = None;
    }

    /// The daemon pushes fresh data.
    pub fn push(&mut self, data: AudioData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The daemon goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The daemon comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated daemon is running.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many volume writes the source has served.
    pub fn volume_writes(&self) -> u32 {
        self.volume_writes
    }

    /// How many mute writes the source has served.
    pub fn mute_writes(&self) -> u32 {
        self.mute_writes
    }

    /// How many default-sink (output routing) writes the source has served.
    pub fn default_sink_writes(&self) -> u32 {
        self.default_sink_writes
    }

    /// How many default-source (input routing) writes the source has served.
    pub fn default_source_writes(&self) -> u32 {
        self.default_source_writes
    }

    /// The simulated default sink's current linear volume, after any writes.
    pub fn volume(&self) -> Option<f32> {
        self.default_sink().map(|sink| sink.volume)
    }

    /// The simulated default sink's current mute state, after any writes.
    pub fn muted(&self) -> Option<bool> {
        self.default_sink().map(|sink| sink.muted)
    }

    /// The simulated default sink's `node.name`, after any routing writes.
    pub fn default_sink_name(&self) -> Option<&str> {
        self.data.as_ref()?.default_sink.as_deref()
    }

    /// The simulated default source's `node.name`, after any routing writes.
    pub fn default_source_name(&self) -> Option<&str> {
        self.data.as_ref()?.default_source.as_deref()
    }

    /// The simulated default source's current linear volume, after any writes.
    pub fn source_volume(&self) -> Option<f32> {
        self.default_source().map(|source| source.volume)
    }

    fn default_sink(&self) -> Option<&SinkData> {
        let data = self.data.as_ref()?;
        let name = data.default_sink.as_deref()?;
        data.sinks.iter().find(|sink| sink.name == name)
    }

    fn default_sink_mut(&mut self) -> Option<&mut SinkData> {
        let data = self.data.as_mut()?;
        let name = data.default_sink.clone()?;
        data.sinks.iter_mut().find(|sink| sink.name == name)
    }

    fn default_source(&self) -> Option<&SourceData> {
        let data = self.data.as_ref()?;
        let name = data.default_source.as_deref()?;
        data.sources.iter().find(|source| source.name == name)
    }

    /// A read/write failure, or absence, in that order.
    fn write_outcome(&mut self) -> Option<SetOutcome> {
        if !self.present {
            return Some(SetOutcome::Absent);
        }
        if let Some(error) = self.failure.clone() {
            return Some(SetOutcome::Failed(error));
        }
        if let Some(error) = self.write_failure.clone() {
            return Some(SetOutcome::Failed(error));
        }
        None
    }
}

impl AudioSource for MockAudio {
    fn read(&mut self) -> Result<Option<AudioData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn set_volume(&mut self, volume: f32) -> SetOutcome {
        self.volume_writes += 1;
        if let Some(outcome) = self.write_outcome() {
            return outcome;
        }
        if let Some(sink) = self.default_sink_mut() {
            sink.volume = volume.clamp(0.0, 1.0);
        }
        SetOutcome::Applied
    }

    fn set_mute(&mut self, muted: bool) -> SetOutcome {
        self.mute_writes += 1;
        if let Some(outcome) = self.write_outcome() {
            return outcome;
        }
        if let Some(sink) = self.default_sink_mut() {
            sink.muted = muted;
        }
        SetOutcome::Applied
    }

    fn set_default_sink(&mut self, id: u32) -> SetOutcome {
        self.default_sink_writes += 1;
        if let Some(outcome) = self.write_outcome() {
            return outcome;
        }
        let name = self
            .data
            .as_ref()
            .and_then(|data| data.sinks.iter().find(|sink| sink.id == id))
            .map(|sink| sink.name.clone());
        match name {
            Some(name) => {
                if let Some(data) = self.data.as_mut() {
                    data.default_sink = Some(name);
                }
                SetOutcome::Applied
            }
            None => SetOutcome::Failed(AdapterError::new(format!(
                "wpctl: no sink with node id {id}"
            ))),
        }
    }

    fn set_default_source(&mut self, id: u32) -> SetOutcome {
        self.default_source_writes += 1;
        if let Some(outcome) = self.write_outcome() {
            return outcome;
        }
        let name = self
            .data
            .as_ref()
            .and_then(|data| data.sources.iter().find(|source| source.id == id))
            .map(|source| source.name.clone());
        match name {
            Some(name) => {
                if let Some(data) = self.data.as_mut() {
                    data.default_source = Some(name);
                }
                SetOutcome::Applied
            }
            None => SetOutcome::Failed(AdapterError::new(format!(
                "wpctl: no source with node id {id}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> AudioData {
        AudioData {
            default_sink: Some("sink-a".to_owned()),
            default_source: Some("source-a".to_owned()),
            sinks: vec![
                SinkData {
                    id: 1,
                    name: "sink-a".to_owned(),
                    description: "Speakers".to_owned(),
                    volume: 0.5,
                    muted: false,
                },
                SinkData {
                    id: 2,
                    name: "sink-b".to_owned(),
                    description: "Headphones".to_owned(),
                    volume: 0.25,
                    muted: true,
                },
            ],
            sources: vec![
                SourceData {
                    id: 10,
                    name: "source-a".to_owned(),
                    description: "Built-in Microphone".to_owned(),
                    volume: 0.8,
                    muted: false,
                },
                SourceData {
                    id: 11,
                    name: "source-b".to_owned(),
                    description: "USB Microphone".to_owned(),
                    volume: 0.4,
                    muted: false,
                },
            ],
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockAudio::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockAudio::failing("PipeWire: core unavailable");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "PipeWire: core unavailable");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockAudio::present(data());
        assert_eq!(mock.read().unwrap(), Some(data()));
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn a_volume_write_mutates_the_default_sink_only() {
        let mut mock = MockAudio::present(data());
        assert_eq!(mock.set_volume(0.75), SetOutcome::Applied);
        assert_eq!(mock.volume_writes(), 1);
        assert_eq!(mock.volume(), Some(0.75));
        let read = mock.read().unwrap().unwrap();
        assert_eq!(read.sinks[0].volume, 0.75);
        assert_eq!(read.sinks[1].volume, 0.25);
    }

    #[test]
    fn a_mute_write_toggles_the_default_sink_only() {
        let mut mock = MockAudio::present(data());
        assert_eq!(mock.set_mute(true), SetOutcome::Applied);
        assert_eq!(mock.mute_writes(), 1);
        assert_eq!(mock.muted(), Some(true));
        let read = mock.read().unwrap().unwrap();
        assert!(read.sinks[0].muted);
        assert!(read.sinks[1].muted);
    }

    #[test]
    fn writes_against_an_absent_daemon_are_absent() {
        let mut mock = MockAudio::absent();
        assert_eq!(mock.set_volume(0.5), SetOutcome::Absent);
        assert_eq!(mock.set_mute(true), SetOutcome::Absent);
        assert_eq!(mock.set_default_sink(1), SetOutcome::Absent);
        assert_eq!(mock.set_default_source(10), SetOutcome::Absent);
    }

    #[test]
    fn a_failing_write_is_reported_and_leaves_the_graph_untouched() {
        let mut mock = MockAudio::present(data());
        mock.fail_writes("wpctl: no such node");
        assert_eq!(
            mock.set_volume(0.9),
            SetOutcome::Failed(AdapterError::new("wpctl: no such node"))
        );
        assert_eq!(mock.volume(), Some(0.5));

        assert_eq!(
            mock.set_default_sink(2),
            SetOutcome::Failed(AdapterError::new("wpctl: no such node"))
        );
        assert_eq!(mock.default_sink_name(), Some("sink-a"));
    }

    #[test]
    fn a_default_sink_write_repoints_output_routing() {
        let mut mock = MockAudio::present(data());
        assert_eq!(mock.default_sink_name(), Some("sink-a"));

        assert_eq!(mock.set_default_sink(2), SetOutcome::Applied);
        assert_eq!(mock.default_sink_writes(), 1);
        assert_eq!(mock.default_sink_name(), Some("sink-b"));
        // Volume then follows the newly-routed sink.
        assert_eq!(mock.set_volume(1.0), SetOutcome::Applied);
        assert_eq!(mock.volume(), Some(1.0));

        let read = mock.read().unwrap().unwrap();
        assert_eq!(read.sinks[0].volume, 0.5);
        assert_eq!(read.sinks[1].volume, 1.0);
    }

    #[test]
    fn a_default_source_write_repoints_input_routing() {
        let mut mock = MockAudio::present(data());
        assert_eq!(mock.default_source_name(), Some("source-a"));

        assert_eq!(mock.set_default_source(11), SetOutcome::Applied);
        assert_eq!(mock.default_source_writes(), 1);
        assert_eq!(mock.default_source_name(), Some("source-b"));
        assert_eq!(mock.source_volume(), Some(0.4));

        // Sources are untouched by output volume writes.
        assert_eq!(mock.set_volume(0.1), SetOutcome::Applied);
        assert_eq!(mock.source_volume(), Some(0.4));
    }

    #[test]
    fn routing_to_an_unknown_node_id_is_a_failure() {
        let mut mock = MockAudio::present(data());
        assert!(matches!(mock.set_default_sink(999), SetOutcome::Failed(_)));
        assert!(matches!(
            mock.set_default_source(999),
            SetOutcome::Failed(_)
        ));
        assert_eq!(mock.default_sink_name(), Some("sink-a"));
        assert_eq!(mock.default_source_name(), Some("source-a"));
    }
}
