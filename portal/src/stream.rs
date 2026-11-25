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
//! transport is [`LiveTransport`]: its watch is
//! [`crate::screencast_bridge::ScreenCastBridge`], the portal's own trusted
//! client of the compositor, opened from the session provisioning
//! ([ADR 0346](../../docs/design/adr/0346-screencast-bridge-trusted-portal-client.md)).
//! A session with no provisioning, a refused handshake, or a compositor without
//! a PipeWire host is the named [`FallbackReason::ProducerUnavailable`]
//! fallback, so a standard client is never handed a placeholder node.
//! Rather than returning a placeholder node id with no explanation, every
//! stream carries its mode (`df_stream_mode`: `pipewire` or `stills`) and, in
//! fallback, the reason (`df_fallback`). The shell reads the same mode off the
//! diagnostic surface and tells the user in the picker. A watch that answers
//! simply stops selecting the fallback; the contract does not change.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

    /// Open a fresh compositor connection after the party the streams depend
    /// on left. Returns whether a live transport is available afterwards. The
    /// default holds no connection and reports the current availability.
    fn reconnect(&mut self) -> bool {
        self.mode() == StreamMode::PipeWire
    }

    /// Why this transport is not live, for the log and the diagnostics. The
    /// default is `None`: a transport that holds no connection has nothing to
    /// report.
    fn error(&self) -> Option<String> {
        None
    }

    /// The one-thread coordinator for batches of [`StreamTransport`] calls.
    ///
    /// A live transport's connection (the Wayland bridge, T335b) belongs to
    /// one thread, while the portal serves requests on several. Every call that
    /// may reach the transport — a `Start`, a release, a close, a reconnect —
    /// runs through this, so the transport never sees two threads. The default
    /// is the call itself: a transport with no connection has nothing to
    /// confine.
    fn on_stream_thread<R>(&mut self, call: impl FnOnce(&mut Self) -> R) -> R
    where
        Self: Sized,
    {
        call(self)
    }

    /// Reopen the connection. A transport that can take a resolver (`LiveTransport`)
    /// installs it first, so a test never writes the process environment.
    fn rescan(&mut self, connect: Option<ConnectFn>) -> bool {
        let _ = connect;
        self.reconnect()
    }

    /// Prove the live connection still answers, now. Only a transport that
    /// holds a connection implements this; the default has nothing to check.
    fn health(&mut self) -> Result<(), WatchError> {
        Ok(())
    }
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

/// The live transport (T335a) over an injected [`CompositorWatch`]: it asks
/// for a PipeWire video node and reports the node id the client links to.
///
/// The direct watch is a seam for the negotiation tests ([`FnWatch`]) and the
/// no-compositor path ([`AbsentCompositor`]); the shipped daemon uses
/// [`LiveTransport`], which owns the bridge connection on one thread. A
/// creation the compositor refuses is the
/// [`FallbackReason::ProducerUnavailable`] fallback — never a fake node id.
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

/// The shipped live transport (T335b): the compositor bridge, reopened on
/// demand.
///
/// The bridge is a trusted client of the compositor, and its one connection is
/// the lifetime of every stream it created. When the party those streams
/// depend on goes away, the compositor releases them; the next `Start` must
/// therefore open a **fresh** connection rather than reuse a dead one
/// ([`LiveTransport::reconnect`]). Until a bridge is open the transport
/// honestly reports [`StreamMode::Stills`], so the picker names the fallback
/// and a `Start` is the named refusal — a live session is never promised by a
/// stream that cannot deliver frames.
///
/// ## One thread owns the connection
///
/// A Wayland event queue belongs to the thread that created it, while the
/// portal's bus object dispatches a `Start` on whichever executor thread is
/// free. So the connection lives on one worker thread: the first request
/// starts it, and every create, release, resume, and reopen is a job on that
/// thread, in order. The transport itself is the message port the bus threads
/// use. A job never nests because jobs only ever touch the bridge they own.
pub struct LiveTransport {
    /// The connection, its records, and the resolver that opens it. `Some`
    /// before the worker starts; the worker owns it afterwards.
    state: Option<Box<LiveState>>,
    /// The service side's end of the worker, once started.
    worker: Option<Worker>,
    mode: StreamMode,
    watch_error: Option<String>,
    /// A resolver a caller installed for the next reopen
    /// ([`LiveTransport::rescan_with`]); otherwise the open one is reused.
    pending: Option<ConnectFn>,
    /// Why the last create was refused (the compositor's own detail), cleared
    /// by the next create that succeeds.
    last_create_error: Option<String>,
}

impl LiveState {
    /// Release one node. A released node is forgotten, so a second release
    /// never reaches the compositor as a ghost; a node of the previous
    /// connection is simply unknown here.
    fn destroy(&mut self, node_id: u32) {
        let Some(token) = self.nodes.remove(&node_id) else {
            return;
        };
        if let Some(watch) = self.watch.as_mut() {
            watch.destroy(&token);
        }
    }

    /// Prove the bridge still answers (see [`CompositorWatch::health`]).
    fn health(&mut self) -> Result<(), WatchError> {
        match self.watch.as_mut() {
            Some(watch) => watch.health(),
            None => Err(WatchError(self.watch_error.clone().unwrap_or_else(|| {
                "the live transport is not connected".to_owned()
            }))),
        }
    }
}

/// Everything the connection's thread owns.
struct LiveState {
    /// Opens the bridge from the backend's provisioning.
    connect: ConnectFn,
    watch: Option<Box<dyn CompositorWatch>>,
    watch_error: Option<String>,
    /// The compositor-side token of each live node, so a release addresses the
    /// stream the create made.
    nodes: HashMap<u32, String>,
    /// The picker's current answer (`pipewire` only while a bridge is open).
    mode: StreamMode,
}

/// How [`LiveTransport`] opens its watch: the shipped bridge resolves the
/// environment; a test opens a socket by hand. It runs on the connection's
/// thread.
pub type ConnectFn = Box<dyn FnMut() -> Result<Box<dyn CompositorWatch>, String> + Send>;

/// A call for the connection's thread; it reports through its own channel.
type Job = Box<dyn FnOnce(&mut LiveState) + Send>;

/// One message to the connection's thread.
enum Message {
    /// Run `call` on the connection.
    Job(Job),
    /// Hand the connection back and exit ([`Drop for LiveTransport`]).
    Return(std::sync::mpsc::Sender<Box<LiveState>>),
}

/// The service side's end of the connection's thread.
struct Worker {
    sender: std::sync::mpsc::Sender<Message>,
}

impl Worker {
    /// Run `call` on the connection's thread, in order, and wait for its
    /// answer. A worker that ended (a panic inside a job) is an error, so the
    /// caller degrades to the named fallback rather than hanging.
    fn run<R: Send + 'static>(
        &self,
        call: impl FnOnce(&mut LiveState) -> R + Send + 'static,
    ) -> Result<R, WorkerLost> {
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let job: Job = Box::new(move |state| {
            let result = call(state);
            let _ = result_sender.send(result);
        });
        self.sender
            .send(Message::Job(job))
            .map_err(|_| WorkerLost)?;
        result_receiver.recv().map_err(|_| WorkerLost)
    }
}

/// The name of the connection's thread (visible in a backtrace).
const CONNECTION_THREAD: &str = "dragonfruit-screencast-bridge";

/// The connection's thread ended (it panics with a job that panics).
#[derive(Debug, Clone, Copy)]
pub struct WorkerLost;

impl std::fmt::Display for WorkerLost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the screen-share bridge thread is gone; the request never reached the compositor"
        )
    }
}

impl std::error::Error for WorkerLost {}

impl LiveTransport {
    /// A live transport over `connect`. The connection is opened immediately on
    /// the calling thread; a failure is recorded and named, never fatal.
    pub fn new(mut connect: ConnectFn) -> Self {
        let (watch, watch_error) = match connect() {
            Ok(watch) => (Some(watch), None),
            Err(error) => (None, Some(error)),
        };
        let mode = if watch.is_some() {
            StreamMode::PipeWire
        } else {
            StreamMode::Stills
        };
        LiveTransport {
            state: Some(Box::new(LiveState {
                connect,
                watch,
                watch_error,
                nodes: HashMap::new(),
                mode,
            })),
            worker: None,
            mode,
            watch_error: None,
            pending: None,
            last_create_error: None,
        }
    }

    /// The shipped transport: open the compositor bridge from the backend's
    /// provisioning ([`crate::screencast_bridge_from_environment_watch`]).
    pub fn from_environment() -> Self {
        LiveTransport::new(Box::new(crate::screencast_bridge_from_environment_watch))
    }

    /// Whether a bridge is open right now.
    pub fn is_connected(&self) -> bool {
        self.state.as_ref().map_or_else(
            || self.mode == StreamMode::PipeWire,
            |state| state.watch.is_some(),
        )
    }

    /// Why the last connection attempt failed (cleared by a successful one).
    pub fn error(&self) -> Option<String> {
        // The last create's refusal is the most recent truth: a compositor
        // with no PipeWire host has an open bridge but refuses the node, and
        // that detail is what the diagnostic must show.
        self.last_create_error
            .clone()
            .or_else(|| {
                self.state
                    .as_ref()
                    .and_then(|state| state.watch_error.clone())
            })
            .or_else(|| self.watch_error.clone())
    }

    /// The compositor-side token of a live node (`None` once released, or after
    /// a reconnect).
    pub fn find_token(&self, node_id: u32) -> Option<String> {
        self.state
            .as_ref()
            .and_then(|state| state.nodes.get(&node_id).cloned())
    }

    /// Open a fresh bridge and drop the previous one: the old connection's
    /// streams are gone with it, and a reused connection would answer nothing.
    /// A session restart re-issues the portal token, so this re-reads the
    /// backend's provisioning. Returns whether a verified bridge is open
    /// afterwards.
    pub fn reconnect(&mut self) -> bool {
        let pending = self.pending.take();
        match self.dispatch(move |state| LiveTransport::reattach(state, pending)) {
            Ok(reopened) => {
                self.mode = if reopened {
                    StreamMode::PipeWire
                } else {
                    StreamMode::Stills
                };
                self.watch_error = if reopened {
                    None
                } else {
                    self.state
                        .as_ref()
                        .and_then(|state| state.watch_error.clone())
                        .or_else(|| Some("the reopen was refused".to_owned()))
                };
                reopened
            }
            Err(_) => {
                self.mode = StreamMode::Stills;
                self.watch_error = Some(WorkerLost.to_string());
                false
            }
        }
    }

    /// [`LiveTransport::reconnect`] over `connect` instead of the backend's
    /// provisioning: a test's own socket and token, so no test writes the
    /// process environment.
    pub fn rescan_with(&mut self, connect: ConnectFn) -> bool {
        self.pending = Some(connect);
        self.reconnect()
    }

    /// Reopen `state` over its resolver, verified. A connect alone is not proof
    /// of life: a compositor that just died can leave its socket path
    /// answering for a moment. The handshake plus one flushed request is the
    /// verification, so the picker is never told `pipewire` by a connection
    /// that is already gone.
    fn reattach(state: &mut LiveState, pending: Option<ConnectFn>) -> bool {
        if let Some(connect) = pending {
            state.connect = connect;
        }
        state.watch = None;
        // The old connection's streams are gone with it; their tokens would
        // address nothing on the new one.
        state.nodes.clear();
        let (watch, watch_error) = match (state.connect)() {
            Ok(mut watch) => match watch.health() {
                Ok(()) => (Some(watch), None),
                Err(error) => (None, Some(error.0)),
            },
            Err(error) => (None, Some(error)),
        };
        state.watch = watch;
        state.watch_error = watch_error;
        state.mode = if state.watch.is_some() {
            StreamMode::PipeWire
        } else {
            StreamMode::Stills
        };
        state.mode == StreamMode::PipeWire
    }

    /// Run `call` on the connection's thread, starting that thread with the
    /// first call. Everything the call does — a create, a release, a reopen —
    /// happens there, in order.
    fn dispatch<R: Send + 'static>(
        &mut self,
        call: impl FnOnce(&mut LiveState) -> R + Send + 'static,
    ) -> Result<R, WorkerLost> {
        if let Some(worker) = self.worker.as_ref() {
            let r = worker.run(call);
            return r;
        }
        // The first call: the caller's thread becomes the connection's thread,
        // and the call itself runs here before the loop takes over.
        let Some(mut state) = self.state.take() else {
            return Err(WorkerLost);
        };
        let result = call(&mut state);
        self.mode = state.mode;
        self.watch_error = state.watch_error.clone();
        let (sender, receiver) = std::sync::mpsc::channel::<Message>();
        let shared = Arc::new(Mutex::new(Some(state)));
        let slot = shared.clone();
        let spawn = std::thread::Builder::new()
            .name(CONNECTION_THREAD.to_owned())
            .spawn(move || {
                for message in receiver {
                    match message {
                        Message::Job(job) => {
                            // The state is in the slot between jobs; a job
                            // only borrows it and puts it back before the next.
                            let mut guard =
                                slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                            if let Some(mut state) = guard.take() {
                                job(&mut state);
                                *guard = Some(state);
                            }
                        }
                        Message::Return(back) => {
                            let state = slot
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner())
                                .take();
                            if let Some(state) = state {
                                let _ = back.send(state);
                            }
                            return;
                        }
                    }
                }
            });
        match spawn {
            Ok(_handle) => {
                self.worker = Some(Worker { sender });
            }
            Err(error) => {
                // No second thread: the calls keep running on the bus threads,
                // and the bridge itself refuses a request from a thread other
                // than the one that opened its queue (never corrupting it).
                self.watch_error = Some(format!("cannot start the bridge thread: {error}"));
                if let Ok(mut guard) = shared.lock() {
                    self.state = guard.take();
                }
            }
        }
        Ok(result)
    }
}

impl Drop for LiveTransport {
    fn drop(&mut self) {
        let Some(worker) = self.worker.take() else {
            return;
        };
        // Ask the connection's thread to hand the bridge back, then close it
        // here on the dropping thread. Bounded, so a wedged compositor cannot
        // hang shutdown; the process exit closes the socket regardless.
        let (back, received) = std::sync::mpsc::channel();
        if worker.sender.send(Message::Return(back)).is_ok() {
            if let Ok(state) = received.recv_timeout(Duration::from_secs(1)) {
                self.state = Some(state);
            }
        }
    }
}

impl StreamTransport for LiveTransport {
    fn mode(&self) -> StreamMode {
        self.mode
    }

    fn create(&mut self, source: &StreamSource) -> Result<u32, FallbackReason> {
        let source = source.clone();
        let created = self.dispatch(move |state| {
            // A compositor that started after the portal (a session restart
            // race) is found here; a reopened connection is verified.
            if state.watch.is_none() && !LiveTransport::reattach(state, None) {
                return Err("the screen-share bridge is not connected".to_owned());
            }
            let Some(watch) = state.watch.as_mut() else {
                return Err("the screen-share bridge is not connected".to_owned());
            };
            let created = watch
                .create(&source.id, &source, DEFAULT_STREAM_FPS)
                .map_err(|error| error.0)?;
            state.nodes.insert(created.node_id, created.token);
            Ok(created.node_id)
        });
        match created {
            Ok(Ok(node_id)) => {
                self.last_create_error = None;
                Ok(node_id)
            }
            Ok(Err(detail)) => {
                self.last_create_error = Some(detail);
                Err(FallbackReason::ProducerUnavailable)
            }
            Err(lost) => {
                self.last_create_error = Some(lost.to_string());
                Err(FallbackReason::ProducerUnavailable)
            }
        }
    }

    fn destroy(&mut self, node_id: u32) {
        let _ = self.dispatch(move |state| state.destroy(node_id));
    }

    fn reconnect(&mut self) -> bool {
        LiveTransport::reconnect(self)
    }

    fn rescan(&mut self, connect: Option<ConnectFn>) -> bool {
        match connect {
            Some(connect) => self.rescan_with(connect),
            None => self.reconnect(),
        }
    }

    fn health(&mut self) -> Result<(), WatchError> {
        let checked = self.dispatch(LiveState::health);
        match checked {
            Ok(result) => result,
            Err(lost) => Err(WatchError(lost.to_string())),
        }
    }

    fn error(&self) -> Option<String> {
        LiveTransport::error(self)
    }

    fn on_stream_thread<R>(&mut self, call: impl FnOnce(&mut Self) -> R) -> R
    where
        Self: Sized,
    {
        // A batch runs on a bus thread, so its last step re-reads the mode the
        // connection's thread settled on (a reopen or a fallback changes it).
        let result = call(self);
        if let Ok((mode, error)) = self.dispatch(|state| (state.mode, state.watch_error.clone())) {
            self.mode = mode;
            self.watch_error = error;
        }
        result
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

    /// Prove the connection still answers, now, so a reconnect never reports
    /// live for a compositor that has already gone. The default holds no
    /// connection and always passes.
    fn health(&mut self) -> Result<(), WatchError> {
        Ok(())
    }
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

/// A watch configured with a bridge endpoint but no connection: every creation
/// is the named fallback.
///
/// T335b superseded the idea of relaying streams through the shell (the shell
/// binds v16 and never shares screens); the shipped watch is now
/// [`crate::screencast_bridge::ScreenCastBridge`], the portal's own trusted
/// client ([ADR 0346](../../docs/design/adr/0346-screencast-bridge-trusted-portal-client.md)).
/// This type stays for the "configured but absent" diagnostic used by the tests.
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

    /// Ask the transport to open a fresh compositor connection, when it has
    /// one to open ([`LiveTransport::reconnect`]). The default is a no-op for
    /// transports that hold no connection.
    pub fn reconnect(&mut self) -> bool {
        self.transport.reconnect()
    }

    /// Reopen over a caller's resolver when the transport is the live one (a
    /// test's own socket and token); otherwise the transport's own reconnect.
    /// The resolver is installed on the transport, which runs it on the
    /// connection's thread.
    pub fn rescan(&mut self, connect: Option<ConnectFn>) -> bool {
        self.transport.rescan(connect)
    }

    /// Why the transport is not live, when it can say (`None` otherwise).
    pub fn transport_error(&self) -> Option<String> {
        self.transport.error()
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

    /// A watch whose creation is answered by `node` and whose releases are
    /// recorded, for the `LiveTransport` ladder tests.
    fn live_watch(node: u32) -> (Box<dyn CompositorWatch>, Arc<Mutex<Vec<String>>>) {
        let destroyed = Arc::new(Mutex::new(Vec::new()));
        let create: CreateFn = Arc::new(move |_source| Ok(node));
        (
            Box::new(FnWatch::new(create).recording_destroys(destroyed.clone())),
            destroyed,
        )
    }

    #[test]
    fn a_live_transport_without_a_bridge_reports_stills_and_the_reason() {
        let mut transport = LiveTransport::new(Box::new(|| Err("no launch token".to_owned())));
        assert!(!transport.is_connected());
        assert_eq!(transport.error().as_deref(), Some("no launch token"));
        assert_eq!(
            transport.mode(),
            StreamMode::Stills,
            "a stream that cannot receive frames is never advertised as live"
        );
        assert_eq!(
            transport.create(&source("monitor:DP-1")),
            Err(FallbackReason::ProducerUnavailable)
        );
    }

    #[test]
    fn a_live_transport_creates_releases_and_reconnects() {
        let mut connects = 0;
        let mut transport = LiveTransport::new(Box::new(move || {
            connects += 1;
            Ok(live_watch(700 + connects).0)
        }));
        assert!(transport.is_connected());
        assert_eq!(transport.mode(), StreamMode::PipeWire);
        let node = transport.create(&source("monitor:DP-1")).unwrap();
        assert_eq!(node, 701);
        assert_eq!(transport.error(), None);

        // The presenter left: the next Start must not reuse the dead
        // connection, so a reconnect opens a fresh one with a fresh node.
        assert!(transport.reconnect());
        assert_eq!(transport.find_token(node), None, "old tokens are dropped");
        let next = transport.create(&source("window:7")).unwrap();
        assert_eq!(next, 702, "the new connection answers");
    }

    #[test]
    fn a_live_transport_retries_once_when_its_bridge_was_absent() {
        // The compositor comes up after the portal: the first attempt fails,
        // every later one succeeds.
        let attempts = Arc::new(Mutex::new(0u32));
        let seen = attempts.clone();
        let mut transport = LiveTransport::new(Box::new(move || {
            let mut attempts = match seen.lock() {
                Ok(attempts) => attempts,
                Err(poisoned) => poisoned.into_inner(),
            };
            *attempts += 1;
            if *attempts == 1 {
                Err("the compositor is not up yet".to_owned())
            } else {
                Ok(live_watch(810).0)
            }
        }));
        assert_eq!(transport.mode(), StreamMode::Stills);
        // The compositor appeared after the portal started: the next Start
        // finds it instead of reporting stills forever.
        assert_eq!(transport.create(&source("monitor:DP-1")), Ok(810));
        assert_eq!(transport.mode(), StreamMode::PipeWire);
        assert_eq!(transport.error(), None);
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
