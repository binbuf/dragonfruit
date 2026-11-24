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
//! The live producer is a disposable host tool the compositor spawns
//! (`compositor/src/screencast_producer.rs`); the portal crate deliberately
//! does not link `libpipewire` (see
//! [ADR 0081](../../docs/design/adr/0081-screencast-stream-negotiation-and-stills-fallback.md)
//! and [ADR 0344](../../docs/design/adr/0344-screencast-producer-process-boundary.md)).
//! The transport asks for a node through [`CompositorWatch`]. The shipped
//! daemon configures no watch endpoint, so every stream is the named
//! [`FallbackReason::ProducerUnavailable`] fallback: a standard client cannot
//! yet receive live frames because no shipped bridge answers the watch
//! ([ADR 0345](../../docs/design/adr/0345-screencast-bridge-and-portal-ownership.md)).
//! Rather than returning a placeholder node id with no explanation, every
//! stream carries its mode (`df_stream_mode`: `pipewire` or `stills`) and, in
//! fallback, the reason (`df_fallback`). The shell reads the same mode off the
//! diagnostic surface and tells the user in the picker. A watch that answers
//! simply stops selecting the fallback; the contract does not change.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

/// The rate a stream asks for when the client names none (the portal interface
/// has no rate option; the compositor clamps it).
pub const DEFAULT_STREAM_FPS: u32 = 30;

/// The live transport (T335a): it asks the compositor for a PipeWire video
/// node through an injected [`CompositorWatch`] and reports the node id the
/// client links to.
///
/// The watch is a seam because the portal crate never links the compositor or
/// a PipeWire client: the shipped backend answers through the trusted shell's
/// one private-protocol connection ([`ShellBridgeWatch`]), and the tests use
/// a deterministic stand-in ([`FnWatch`]). A creation the compositor refuses
/// is the [`FallbackReason::ProducerUnavailable`] fallback — never a fake node
/// id.
pub struct PipeWireTransport {
    watch: Box<dyn CompositorWatch>,
    nodes: Mutex<HashMap<u32, String>>,
}

impl PipeWireTransport {
    /// A transport over `watch`.
    pub fn new(watch: Box<dyn CompositorWatch>) -> Self {
        PipeWireTransport {
            watch,
            nodes: Mutex::new(HashMap::new()),
        }
    }

    /// A transport whose compositor is absent: every creation is the named
    /// fallback.
    pub fn unavailable() -> Self {
        PipeWireTransport::new(Box::new(AbsentCompositor))
    }

    /// The compositor-side token a live node was created with, used to release
    /// it.
    pub fn token(&self, node_id: u32) -> Option<String> {
        lock_nodes(&self.nodes).get(&node_id).cloned()
    }
}

fn lock_nodes(
    nodes: &Mutex<HashMap<u32, String>>,
) -> std::sync::MutexGuard<'_, HashMap<u32, String>> {
    nodes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl std::fmt::Debug for PipeWireTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let live = self.nodes.lock().map_or(0, |nodes| nodes.len());
        f.debug_struct("PipeWireTransport")
            .field("live", &live)
            .finish()
    }
}

impl StreamTransport for PipeWireTransport {
    fn mode(&self) -> StreamMode {
        StreamMode::PipeWire
    }

    fn create(&mut self, source: &StreamSource) -> Result<u32, FallbackReason> {
        let token = format!("{}-{}", source.source_type.as_str(), source.id);
        let created = self
            .watch
            .create(&token, source, DEFAULT_STREAM_FPS)
            .map_err(|_| FallbackReason::ProducerUnavailable)?;
        lock_nodes(&self.nodes).insert(created.node_id, created.token);
        Ok(created.node_id)
    }

    fn destroy(&mut self, node_id: u32) {
        // A released node is forgotten, so a second release never reaches the
        // compositor as a ghost.
        if let Some(token) = lock_nodes(&self.nodes).remove(&node_id) {
            self.watch.destroy(&token);
        }
    }
}

/// One node the compositor created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedNode {
    /// The node id a portal client links to.
    pub node_id: u32,
    /// The compositor-side token the stream was created with.
    pub token: String,
}

/// Why a compositor did not create a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchError(pub String);

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WatchError {}

/// The compositor half of a live stream: create a video node and release it.
/// Object-safe and `Send`, so the registry can own it under its mutex exactly
/// like [`StreamTransport`].
pub trait CompositorWatch: Send {
    /// Ask the compositor for a node for `source` at `fps`. The caller bounds
    /// the wait; a timeout or a refused request is an error, never a
    /// placeholder id.
    fn create(
        &mut self,
        token: &str,
        source: &StreamSource,
        fps: u32,
    ) -> Result<CreatedNode, WatchError>;

    /// Release a stream. Idempotent; a compositor that is already gone needs no
    /// answer.
    fn destroy(&mut self, token: &str);
}

/// A compositor that is not there: every stream is the named fallback.
#[derive(Debug, Default)]
pub struct AbsentCompositor;

impl CompositorWatch for AbsentCompositor {
    fn create(
        &mut self,
        _token: &str,
        _source: &StreamSource,
        _fps: u32,
    ) -> Result<CreatedNode, WatchError> {
        Err(WatchError("no compositor connection".to_owned()))
    }

    fn destroy(&mut self, _token: &str) {}
}

/// The shipped watch: the trusted shell's private-protocol connection resolves
/// `create_screencast` and the stream's events.
///
/// The shell bridge issues `create_screencast` on its existing private-protocol
/// connection (v17) and publishes the answer here; the portal never opens a
/// compositor connection itself. A backend without a shell bridge (or with a
/// shell that has no compositor) is the named fallback, never a fake node.
#[derive(Debug, Clone, Default)]
pub struct ShellBridgeWatch {
    /// The endpoint the shell bridge publishes at (the trusted shell's
    /// stream-command socket). `None` when no bridge is configured: every
    /// creation is the named fallback.
    pub endpoint: Option<PathBuf>,
}

impl ShellBridgeWatch {
    /// A watch over the shell bridge at `endpoint`.
    pub fn new(endpoint: impl Into<PathBuf>) -> Self {
        ShellBridgeWatch {
            endpoint: Some(endpoint.into()),
        }
    }

    /// A watch with no bridge configured: the named fallback.
    pub fn absent() -> Self {
        ShellBridgeWatch { endpoint: None }
    }
}

impl CompositorWatch for ShellBridgeWatch {
    fn create(
        &mut self,
        _token: &str,
        _source: &StreamSource,
        _fps: u32,
    ) -> Result<CreatedNode, WatchError> {
        let endpoint = self.endpoint.as_ref().ok_or_else(|| {
            WatchError("no shell screencast bridge: the stills fallback is in use".to_owned())
        })?;
        Err(WatchError(format!(
            "the shell screencast bridge ({}) is not connected",
            endpoint.display()
        )))
    }

    fn destroy(&mut self, _token: &str) {}
}

/// The creation function a [`FnWatch`] wraps.
pub type CreateFn = Arc<dyn Fn(&StreamSource) -> Result<u32, FallbackReason> + Send + Sync>;

/// A watch over a plain closure, so a test (or a later bridge) drives the
/// transport without implementing the trait.
pub struct FnWatch {
    create: CreateFn,
    destroyed: Arc<Mutex<Vec<String>>>,
}

impl FnWatch {
    /// A watch whose creation is `create` and whose releases are recorded.
    pub fn new(create: CreateFn) -> Self {
        FnWatch {
            create,
            destroyed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// The tokens released so far.
    pub fn destroyed(&self) -> Vec<String> {
        self.destroyed
            .lock()
            .map(|tokens| tokens.clone())
            .unwrap_or_default()
    }

    /// Share `destroyed` as this watch's release ledger, so another party (an
    /// integration test) can read every token a transport released.
    pub fn recording_destroys(mut self, destroyed: Arc<Mutex<Vec<String>>>) -> Self {
        self.destroyed = destroyed;
        self
    }
}

impl CompositorWatch for FnWatch {
    fn create(
        &mut self,
        token: &str,
        source: &StreamSource,
        _fps: u32,
    ) -> Result<CreatedNode, WatchError> {
        (self.create)(source)
            .map(|node_id| CreatedNode {
                node_id,
                token: token.to_owned(),
            })
            .map_err(|reason| WatchError(reason.as_str().to_owned()))
    }

    fn destroy(&mut self, token: &str) {
        if let Ok(mut destroyed) = self.destroyed.lock() {
            destroyed.push(token.to_owned());
        }
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

    fn window_source(id: &str) -> StreamSource {
        StreamSource {
            id: id.to_owned(),
            source_type: SourceType::Window,
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

    /// A compositor stand-in that hands out increasing node ids.
    fn node_counter(start: u32) -> (Arc<Mutex<u32>>, CreateFn) {
        let next = Arc::new(Mutex::new(start));
        let create: CreateFn = {
            let next = next.clone();
            Arc::new(move |_source| {
                let mut node = next.lock().unwrap();
                let id = *node;
                *node += 1;
                Ok(id)
            })
        };
        (next, create)
    }

    #[test]
    fn the_live_transport_creates_a_node_and_releases_it_by_token() {
        let (_next, create) = node_counter(310);
        let destroyed = Arc::new(Mutex::new(Vec::new()));
        let capture = destroyed.clone();
        let watch = FnWatch {
            create,
            destroyed: capture.clone(),
        };
        let mut negotiator =
            StreamNegotiator::new(Box::new(PipeWireTransport::new(Box::new(watch))));
        assert_eq!(negotiator.mode(), StreamMode::PipeWire);

        let stream = negotiator.negotiate(&source("monitor:DP-1"));
        assert_eq!(stream.mode, StreamMode::PipeWire);
        assert_eq!(stream.node_id, 310);
        assert_eq!(stream.fallback, None);

        let second = negotiator.negotiate(&window_source("window:7"));
        assert_eq!(second.node_id, 311, "each source gets its own node");

        negotiator.destroy(stream.node_id);
        negotiator.destroy(stream.node_id);
        negotiator.destroy(second.node_id);
        let released = capture.lock().unwrap().clone();
        assert_eq!(
            released,
            vec![
                "monitor-monitor:DP-1".to_owned(),
                "window-window:7".to_owned()
            ],
            "one release per live node, never a ghost"
        );
    }

    #[test]
    fn a_compositor_that_refuses_is_the_named_fallback() {
        let mut transport = PipeWireTransport::unavailable();
        assert_eq!(transport.mode(), StreamMode::PipeWire);
        assert_eq!(
            transport.create(&source("monitor:DP-1")),
            Err(FallbackReason::ProducerUnavailable)
        );
        let mut negotiator = StreamNegotiator::new(Box::new(PipeWireTransport::unavailable()));
        let stream = negotiator.negotiate(&source("window:7"));
        assert_eq!(stream.mode, StreamMode::Stills);
        assert_eq!(stream.node_id, 0);
        assert_eq!(stream.fallback, Some(FallbackReason::ProducerUnavailable));
    }

    #[test]
    fn a_released_node_is_forgotten() {
        let (_next, create) = node_counter(501);
        let watch = FnWatch::new(create);
        let mut transport = PipeWireTransport::new(Box::new(watch));
        let node = transport.create(&source("monitor:HDMI-1")).unwrap();
        assert_eq!(node, 501);
        assert_eq!(
            transport.token(node),
            Some("monitor-monitor:HDMI-1".to_owned())
        );
        transport.destroy(node);
        assert_eq!(transport.token(node), None);
        // A second release is a no-op: nothing reaches the compositor.
        transport.destroy(node);
    }

    #[test]
    fn the_shipped_watch_without_its_bridge_names_the_fallback() {
        let mut transport = PipeWireTransport::new(Box::new(ShellBridgeWatch::new(
            "/run/dragonfruit/screencast",
        )));
        assert!(transport.create(&source("monitor:DP-1")).is_err());
        let mut transport = PipeWireTransport::new(Box::new(ShellBridgeWatch::absent()));
        assert_eq!(
            transport.create(&source("monitor:DP-1")),
            Err(FallbackReason::ProducerUnavailable)
        );
    }
}
