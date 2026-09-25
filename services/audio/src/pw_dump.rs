// SPDX-License-Identifier: MIT
//! The live transport: WirePlumber/PipeWire through their control CLI.
//!
//! There is no stable D-Bus API for sink volume and the native `libpipewire`
//! headers are not part of the pinned toolchain, so the adapter talks to
//! WirePlumber through the tools it ships: [`pw-dump`](https://docs.pipewire.org)
//! for the graph read (JSON) and `wpctl` for the volume/mute write. The
//! parsing churn — the PipeWire JSON schema and the `wpctl` surface — is
//! isolated here; [`crate::AudioData::from_pw_dump`] is the pinned seam and is
//! tested against a captured fixture
//! (see docs/design/adr/0028-audio-adapter-over-wireplumber-cli.md).
//!
//! The source constructs no process until it is called, so building an adapter
//! is free and a session without PipeWire still boots. A tool that cannot be
//! run, or that exits non-zero because there is no PipeWire core to connect
//! to, is treated as absence; a read that returns but cannot be parsed is an
//! error.

use std::process::Command;

use dragonfruit_system_adapters::AdapterError;
use serde_json::Value;

use crate::source::{AudioData, AudioSource, SetOutcome, SinkData};

/// The PipeWire graph-dump tool (ships with `pipewire`).
pub const PW_DUMP_BIN: &str = "pw-dump";
/// The WirePlumber control CLI (ships with `wireplumber`).
pub const WPCTL_BIN: &str = "wpctl";
/// The symbolic id `wpctl` resolves to the current default sink.
pub const DEFAULT_SINK_TARGET: &str = "@DEFAULT_AUDIO_SINK@";

impl AudioData {
    /// Decode one `pw-dump` snapshot into the raw audio read.
    ///
    /// The snapshot is a JSON array of PipeWire objects. We take the
    /// `default.audio.sink` entry from the `default` metadata object and every
    /// node whose `media.class` is `Audio/Sink`. Volume is WirePlumber's
    /// channel volume (stored on a cubic curve) un-cubed to the linear value
    /// the menu bar shows.
    ///
    /// This is the one place the PipeWire JSON schema is known: it is pinned
    /// by the captured fixture and the churn stays behind the source seam.
    pub fn from_pw_dump(json: &str) -> Result<AudioData, AdapterError> {
        let root: Value = serde_json::from_str(json).map_err(|error| {
            AdapterError::new(format!("PipeWire: invalid pw-dump JSON: {error}"))
        })?;
        let objects = root
            .as_array()
            .ok_or_else(|| AdapterError::new("PipeWire: pw-dump root is not an array"))?;

        let mut default_sink = None;
        let mut sinks = Vec::new();
        for object in objects {
            match object.get("type").and_then(Value::as_str) {
                Some("PipeWire:Interface:Metadata") => {
                    if object
                        .pointer("/props/metadata.name")
                        .and_then(Value::as_str)
                        == Some("default")
                    {
                        default_sink = default_sink_name(object);
                    }
                }
                Some("PipeWire:Interface:Node") => {
                    if let Some(sink) = sink_from_node(object) {
                        sinks.push(sink);
                    }
                }
                _ => {}
            }
        }

        Ok(AudioData {
            default_sink,
            sinks,
        })
    }
}

/// The `default.audio.sink` name from one metadata object.
///
/// `value` is usually an already-decoded object (`{"name": "..."}`), but older
/// PipeWire versions encode it as a JSON string; handle both.
fn default_sink_name(metadata: &Value) -> Option<String> {
    let entries = metadata.get("metadata")?.as_array()?;
    for entry in entries {
        if entry.get("key").and_then(Value::as_str) != Some("default.audio.sink") {
            continue;
        }
        let value = entry.get("value")?;
        if let Some(name) = value.get("name").and_then(Value::as_str) {
            return Some(name.to_owned());
        }
        if let Some(encoded) = value.as_str() {
            if let Ok(decoded) = serde_json::from_str::<Value>(encoded) {
                if let Some(name) = decoded.get("name").and_then(Value::as_str) {
                    return Some(name.to_owned());
                }
            }
        }
    }
    None
}

/// One `Audio/Sink` node, when `node` is one.
fn sink_from_node(node: &Value) -> Option<SinkData> {
    let props = node.pointer("/info/props")?;
    if props.get("media.class").and_then(Value::as_str) != Some("Audio/Sink") {
        return None;
    }
    let id = node.get("id").and_then(Value::as_u64)? as u32;
    let name = props
        .get("node.name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let description = props
        .get("node.description")
        .and_then(Value::as_str)
        .or_else(|| props.get("node.nick").and_then(Value::as_str))
        .unwrap_or(name.as_str())
        .to_owned();
    let (volume, muted) = node_volume(node);
    Some(SinkData {
        id,
        name,
        description,
        volume,
        muted,
    })
}

/// The linear volume and mute of a node, from its `Props` param.
fn node_volume(node: &Value) -> (f32, bool) {
    let props = node
        .pointer("/info/params/Props")
        .and_then(Value::as_array)
        .and_then(|params| params.first());
    let Some(props) = props else {
        return (0.0, false);
    };
    let muted = props.get("mute").and_then(Value::as_bool).unwrap_or(false);
    let raw = props
        .get("channelVolumes")
        .and_then(Value::as_array)
        .filter(|channels| !channels.is_empty())
        .map(|channels| {
            channels.iter().filter_map(Value::as_f64).sum::<f64>() / channels.len() as f64
        })
        .or_else(|| props.get("volume").and_then(Value::as_f64))
        .unwrap_or(0.0);
    (linear_volume(raw as f32), muted)
}

/// WirePlumber stores volume on a cubic curve; `wpctl` reports the linear
/// value. Invert the curve and clamp to the range the menu bar renders.
fn linear_volume(stored: f32) -> f32 {
    stored.max(0.0).cbrt().clamp(0.0, 1.0)
}

/// The real WirePlumber transport: `pw-dump` to read, `wpctl` to write.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommandAudio;

impl CommandAudio {
    /// A source that runs the control CLI on demand.
    pub const fn new() -> Self {
        CommandAudio
    }
}

impl AudioSource for CommandAudio {
    fn read(&mut self) -> Result<Option<AudioData>, AdapterError> {
        // A tool that cannot run, or that cannot reach a PipeWire core, is
        // absence — not an error, and never a startup blocker.
        let output = match Command::new(PW_DUMP_BIN).output() {
            Ok(output) => output,
            Err(_) => return Ok(None),
        };
        if !output.status.success() {
            return Ok(None);
        }
        let json = String::from_utf8_lossy(&output.stdout);
        AudioData::from_pw_dump(&json).map(Some)
    }

    fn set_volume(&mut self, volume: f32) -> SetOutcome {
        let volume = volume.clamp(0.0, 1.0);
        run_wpctl(&["set-volume", DEFAULT_SINK_TARGET, &format!("{volume}")])
    }

    fn set_mute(&mut self, muted: bool) -> SetOutcome {
        let value = if muted { "1" } else { "0" };
        run_wpctl(&["set-mute", DEFAULT_SINK_TARGET, value])
    }
}

/// Run one `wpctl` write and map its result to the adapter outcome.
fn run_wpctl(args: &[&str]) -> SetOutcome {
    let output = match Command::new(WPCTL_BIN).args(args).output() {
        Ok(output) => output,
        Err(error) => {
            return match error.kind() {
                std::io::ErrorKind::NotFound => SetOutcome::Absent,
                _ => SetOutcome::Failed(AdapterError::new(format!("wpctl: {error}"))),
            };
        }
    };
    if output.status.success() {
        return SetOutcome::Applied;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = stderr.trim();
    let message = if message.is_empty() {
        format!("wpctl {} failed", args.first().copied().unwrap_or_default())
    } else {
        format!("wpctl: {message}")
    };
    SetOutcome::Failed(AdapterError::new(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/pw-dump-office.json");

    #[test]
    fn the_fixture_parses_to_the_default_sink_and_list() {
        let data = AudioData::from_pw_dump(FIXTURE).expect("fixture parses");
        assert_eq!(
            data.default_sink.as_deref(),
            Some("alsa_output.pci-0000_02_01.0.analog-stereo")
        );
        assert_eq!(data.sinks.len(), 2);
        assert_eq!(data.sinks[0].id, 49);
        assert!(data.sinks[0].description.contains("Analog Stereo"));
        // Stored 0.216 = 0.6^3; the linear volume is 0.6.
        assert!((data.sinks[0].volume - 0.6).abs() < 0.001);
        assert!(!data.sinks[0].muted);
        // The synthesized second sink stores 0.125 = 0.5^3.
        assert!((data.sinks[1].volume - 0.5).abs() < 0.001);
        assert_eq!(
            data.sinks[1].name,
            "alsa_output.usb-Headset-00.analog-stereo"
        );
    }

    #[test]
    fn a_non_sink_node_is_ignored() {
        let json = r#"[
            {"id": 76, "type": "PipeWire:Interface:Node",
             "info": {"props": {"media.class": "Stream/Output/Audio"},
                      "params": {"Props": [{"volume": 1.0, "mute": false}]}}}
        ]"#;
        let data = AudioData::from_pw_dump(json).unwrap();
        assert!(data.sinks.is_empty());
        assert_eq!(data.default_sink, None);
    }

    #[test]
    fn malformed_json_is_an_error_not_absence() {
        let error = AudioData::from_pw_dump("not json").unwrap_err();
        assert!(error.message().starts_with("PipeWire: "));
    }

    #[test]
    fn a_root_that_is_not_an_array_is_an_error() {
        let error = AudioData::from_pw_dump("{}").unwrap_err();
        assert!(error.message().contains("root is not an array"));
    }

    #[test]
    fn a_string_encoded_metadata_value_is_decoded() {
        let json = r#"[
            {"type": "PipeWire:Interface:Metadata",
             "props": {"metadata.name": "default"},
             "metadata": [{"key": "default.audio.sink",
                           "value": "{\"name\": \"sink-a\"}"}]}
        ]"#;
        let data = AudioData::from_pw_dump(json).unwrap();
        assert_eq!(data.default_sink.as_deref(), Some("sink-a"));
    }

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not run any process; only a call does.
        let _source = CommandAudio::new();
    }
}
