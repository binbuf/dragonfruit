// SPDX-License-Identifier: MIT
//! The ScreenCast stream transport and its stills fallback (T-13.4b).
//!
//! [`crate::screencast`] stops at the *source handle*. This module is the one
//! place that turns a chosen source into a stream: a [`StreamTransport`]
//! creates the live PipeWire node when one is available, and a stills-only
//! fallback names itself when it is not. The negotiation is pure (no D-Bus and
//! no `libpipewire` link), so it is unit-tested directly and a later session can
//! swap in a real producer without touching the session, picker, or diagnostics.
//!
//! ## Why the fallback is named, not silent
//!
//! This build ships no in-process PipeWire producer (the compositor renders on
//! the private capture seam, and the portal crate deliberately does not link
//! `libpipewire`; see
//! [ADR 0081](../../docs/design/adr/0081-screencast-stream-negotiation-and-stills-fallback.md)).
//! A standard client therefore cannot yet receive live frames. Rather than
//! returning a placeholder node id with no explanation, every stream carries
//! its mode (`df_stream_mode`: `pipewire` or `stills`) and, in fallback, the
//! reason (`df_fallback`). The shell reads the same mode off the diagnostic
//! surface and tells the user in the picker. When a real transport lands the
//! fallback simply stops being selected; the contract does not change.

use std::collections::HashMap;

use zbus::zvariant::{OwnedValue, Str};

/// The stream's `id`-style property carrying the negotiated mode.
pub const STREAM_MODE_PROPERTY: &str = "df_stream_mode";
/// The stream's property naming why a fallback was used, when it was.
pub const STREAM_FALLBACK_PROPERTY: &str = "df_fallback";

/// How a shared source is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamMode {
    /// A live PipeWire video node: a standard client receives frames.
    PipeWire,
    /// No live producer: the stream is named as stills-only.
    Stills,
}

impl StreamMode {
    /// The stable label exposed on the wire and the diagnostic surface.
    pub const fn as_str(self) -> &'static str {
        match self {
            StreamMode::PipeWire => "pipewire",
            StreamMode::Stills => "stills",
        }
    }

    /// Parse a stable label.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pipewire" => Some(StreamMode::PipeWire),
            "stills" => Some(StreamMode::Stills),
            _ => None,
        }
    }
}

/// Why a live stream could not be created and the stills fallback was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FallbackReason {
    /// This build has no PipeWire producer; stills are all that can be offered.
    ProducerUnavailable,
    /// A producer exists but the chosen source cannot be captured.
    SourceUnavailable,
}

impl FallbackReason {
    /// The stable label exposed in the stream's `df_fallback` property.
    pub const fn as_str(self) -> &'static str {
        match self {
            FallbackReason::ProducerUnavailable => "pipewire-producer-unavailable",
            FallbackReason::SourceUnavailable => "source-unavailable",
        }
    }
}

/// One source a transport is asked to stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSource {
    /// The presenter's opaque source handle (`monitor:DP-1`, `window:7`).
    pub id: String,
    /// The source's kind.
    pub source_type: crate::screencast::SourceType,
    /// The negotiated cursor mode (`1` hidden, `2` embedded).
    pub cursor_mode: u32,
}

/// The result of negotiating one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedStream {
    /// The PipeWire node id, or `0` in the stills fallback.
    pub node_id: u32,
    /// How the source will be delivered.
    pub mode: StreamMode,
    /// The reason the fallback was chosen, when it was.
    pub fallback: Option<FallbackReason>,
}

impl NegotiatedStream {
    /// A live PipeWire stream at `node_id`.
    pub fn live(node_id: u32) -> Self {
        NegotiatedStream {
            node_id,
            mode: StreamMode::PipeWire,
            fallback: None,
        }
    }

    /// The named stills fallback.
    pub fn stills(reason: FallbackReason) -> Self {
        NegotiatedStream {
            node_id: 0,
            mode: StreamMode::Stills,
            fallback: Some(reason),
        }
    }
}

/// The one thing that creates a live stream. The shipped implementation is the
/// stills fallback; a real PipeWire producer implements this trait and the
/// negotiation wires it in unchanged.
pub trait StreamTransport: Send {
    /// How this transport delivers a source.
    fn mode(&self) -> StreamMode;

    /// Create the live node for `source`, or name why not.
    fn create(&mut self, source: &StreamSource) -> Result<u32, FallbackReason>;

    /// Release a node created by [`StreamTransport::create`]. The default is a
    /// no-op; the stills fallback never creates one.
    fn destroy(&mut self, _node_id: u32) {}
}

/// The shipped transport: no stream, but the fallback is explicit.
#[derive(Debug, Default)]
pub struct StillsTransport;

impl StillsTransport {
    /// A new stills transport.
    pub fn new() -> Self {
        StillsTransport
    }
}

impl StreamTransport for StillsTransport {
    fn mode(&self) -> StreamMode {
        StreamMode::Stills
    }

    fn create(&mut self, _source: &StreamSource) -> Result<u32, FallbackReason> {
        Err(FallbackReason::ProducerUnavailable)
    }
}

/// Turn a chosen source into a stream, falling back to named stills when the
/// transport cannot create a live node. Never silent: a fallback always carries
/// its reason.
pub struct StreamNegotiator {
    transport: Box<dyn StreamTransport>,
}

impl Default for StreamNegotiator {
    fn default() -> Self {
        StreamNegotiator::stills()
    }
}

impl StreamNegotiator {
    /// A negotiator over `transport`.
    pub fn new(transport: Box<dyn StreamTransport>) -> Self {
        StreamNegotiator { transport }
    }

    /// The shipped stills-only negotiator.
    pub fn stills() -> Self {
        StreamNegotiator::new(Box::new(StillsTransport::new()))
    }

    /// How this negotiator delivers a source.
    pub fn mode(&self) -> StreamMode {
        self.transport.mode()
    }

    /// Negotiate one source. A transport failure becomes a named fallback.
    pub fn negotiate(&mut self, source: &StreamSource) -> NegotiatedStream {
        match self.transport.create(source) {
            Ok(node_id) => NegotiatedStream::live(node_id),
            Err(reason) => NegotiatedStream::stills(reason),
        }
    }

    /// Release a live node.
    pub fn destroy(&mut self, node_id: u32) {
        self.transport.destroy(node_id);
    }
}

/// The mode/fallback properties one stream adds to its wire tuple.
pub fn stream_properties(
    mode: StreamMode,
    fallback: Option<FallbackReason>,
) -> HashMap<String, OwnedValue> {
    let mut properties = HashMap::new();
    properties.insert(
        STREAM_MODE_PROPERTY.to_owned(),
        OwnedValue::from(Str::from(mode.as_str())),
    );
    if let Some(reason) = fallback {
        properties.insert(
            STREAM_FALLBACK_PROPERTY.to_owned(),
            OwnedValue::from(Str::from(reason.as_str())),
        );
    }
    properties
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screencast::SourceType;

    /// A transport that produces a node id, for the live half of negotiation.
    struct MockTransport {
        next_node: u32,
        destroyed: Vec<u32>,
        refuse: bool,
    }

    impl StreamTransport for MockTransport {
        fn mode(&self) -> StreamMode {
            StreamMode::PipeWire
        }

        fn create(&mut self, _source: &StreamSource) -> Result<u32, FallbackReason> {
            if self.refuse {
                return Err(FallbackReason::SourceUnavailable);
            }
            let node = self.next_node;
            self.next_node += 1;
            Ok(node)
        }

        fn destroy(&mut self, node_id: u32) {
            self.destroyed.push(node_id);
        }
    }

    fn source(id: &str) -> StreamSource {
        StreamSource {
            id: id.to_owned(),
            source_type: SourceType::Monitor,
            cursor_mode: 1,
        }
    }

    #[test]
    fn the_modes_round_trip_through_their_labels() {
        for mode in [StreamMode::PipeWire, StreamMode::Stills] {
            assert_eq!(StreamMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(StreamMode::parse("recording"), None);
        assert_eq!(StreamMode::PipeWire.as_str(), "pipewire");
        assert_eq!(StreamMode::Stills.as_str(), "stills");
    }

    #[test]
    fn a_live_transport_produces_a_pipewire_node() {
        let mut negotiator = StreamNegotiator::new(Box::new(MockTransport {
            next_node: 42,
            destroyed: Vec::new(),
            refuse: false,
        }));
        let stream = negotiator.negotiate(&source("monitor:DP-1"));
        assert_eq!(stream.mode, StreamMode::PipeWire);
        assert_eq!(stream.node_id, 42);
        assert_eq!(stream.fallback, None);
        negotiator.destroy(stream.node_id);
    }

    #[test]
    fn the_shipped_transport_names_the_stills_fallback_and_never_silently_succeeds() {
        let mut negotiator = StreamNegotiator::stills();
        assert_eq!(negotiator.mode(), StreamMode::Stills);
        let stream = negotiator.negotiate(&source("window:7"));
        assert_eq!(stream.mode, StreamMode::Stills);
        assert_eq!(stream.node_id, 0);
        assert_eq!(
            stream.fallback,
            Some(FallbackReason::ProducerUnavailable),
            "a fallback always carries a reason"
        );
    }

    #[test]
    fn a_refused_source_is_a_named_fallback_not_a_fake_node() {
        let mut negotiator = StreamNegotiator::new(Box::new(MockTransport {
            next_node: 1,
            destroyed: Vec::new(),
            refuse: true,
        }));
        let stream = negotiator.negotiate(&source("window:7"));
        assert_eq!(stream.node_id, 0);
        assert_eq!(stream.mode, StreamMode::Stills);
        assert_eq!(stream.fallback, Some(FallbackReason::SourceUnavailable));
    }

    #[test]
    fn the_wire_properties_name_the_mode_and_reason() {
        let live = stream_properties(StreamMode::PipeWire, None);
        assert_eq!(
            String::try_from(live.get(STREAM_MODE_PROPERTY).unwrap().try_clone().unwrap()).unwrap(),
            "pipewire"
        );
        assert!(!live.contains_key(STREAM_FALLBACK_PROPERTY));

        let stills = stream_properties(
            StreamMode::Stills,
            Some(FallbackReason::ProducerUnavailable),
        );
        assert_eq!(
            String::try_from(
                stills
                    .get(STREAM_FALLBACK_PROPERTY)
                    .unwrap()
                    .try_clone()
                    .unwrap()
            )
            .unwrap(),
            "pipewire-producer-unavailable"
        );
    }
}
