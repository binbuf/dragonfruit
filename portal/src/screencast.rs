// SPDX-License-Identifier: GPL-3.0-only
//! The ScreenCast portal's pure model (T-13.4a).
//!
//! The backend serves the standard
//! `org.freedesktop.impl.portal.ScreenCast` interface so sandboxed
//! applications can ask the desktop to share a monitor or a window. The
//! interface is a three-step session: `CreateSession` registers the session,
//! `SelectSources` carries the picker (the frontend blocks while the desktop's
//! source picker is up), and `Start` returns the chosen source as a stream.
//!
//! Like the FileChooser (T-13.2a) and Screenshot (T-13.3a) presenters, the
//! picker is a one-shot future: [`ScreenCastRegistry::begin_select`] registers
//! the request and hands back a [`ScreenCastCompletion`] the D-Bus method
//! awaits; the shell's picker resolves it through the diagnostic
//! `CompleteScreenCast`/`CancelScreenCast` seam.
//!
//! ## Streaming is [`crate::stream`]
//!
//! [`ScreenCastStream`] carries the chosen id, its source type, and the
//! negotiated [`StreamMode`]: a live PipeWire node when a transport creates
//! one, or the named stills fallback otherwise. The picker and the session
//! lifecycle do not change with the transport.
//!
//! ## Live streams are owned by their session (T335b)
//!
//! `Start` creates one node per chosen source and remembers it on the session.
//! A session owns its nodes: a second `Start` on the same session releases the
//! previous set first, `Close` (a client disconnect, an app exit, or a session
//! reset) releases them, and the presenter-loss watch releases every live
//! stream when the trusted shell goes away. A created node is never leaked,
//! and a transport that refuses is still the named fallback, never a
//! placeholder id.
//!
//! ## What is advertised
//!
//! The backend advertises version 3 of the interface: the `source_type`
//! stream property (v3) is present, while persistence (`persist_mode` /
//! `restore_data`, v4) is deliberately absent because this slice cannot
//! restore a session yet. Virtual monitors (`4`) are not advertised; monitors
//! (`1`) and windows (`2`) are.
//!
//! This module has no D-Bus dependency, so the options/results logic and the
//! session lifecycle are unit-tested directly.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use zbus::zvariant::{Array, OwnedValue, Str, Structure, Type, Value};

use crate::stream::{FallbackReason, StreamMode, StreamNegotiator, StreamSource, StreamTransport};

/// The standard ScreenCast backend interface.
pub const SCREENCAST_INTERFACE: &str = "org.freedesktop.impl.portal.ScreenCast";

/// The ScreenCast backend version. Version 3 adds the `source_type` stream
/// property; persistence (v4) and the later additions are not implemented.
pub const SCREENCAST_VERSION: u32 = 3;

/// A bitmask of the source types this backend can offer: monitors and windows.
pub const AVAILABLE_SOURCE_TYPES: u32 = SOURCE_MONITOR | SOURCE_WINDOW;

/// A bitmask of the cursor modes this backend can offer: hidden and embedded.
pub const AVAILABLE_CURSOR_MODES: u32 = CURSOR_HIDDEN | CURSOR_EMBEDDED;

/// Portal response codes shared with the caller.
pub const RESPONSE_SUCCESS: u32 = 0;
pub const RESPONSE_CANCELLED: u32 = 1;
pub const RESPONSE_OTHER: u32 = 2;

/// `types` bit: share an existing monitor.
pub const SOURCE_MONITOR: u32 = 1;
/// `types` bit: share an application window.
pub const SOURCE_WINDOW: u32 = 2;
/// `types` bit: extend with a virtual monitor (not advertised yet).
pub const SOURCE_VIRTUAL: u32 = 4;

/// `cursor_mode` value: the cursor is not part of the stream.
pub const CURSOR_HIDDEN: u32 = 1;
/// `cursor_mode` value: the cursor is embedded in the stream buffers.
pub const CURSOR_EMBEDDED: u32 = 2;
/// `cursor_mode` value: the cursor is sent as stream metadata.
pub const CURSOR_METADATA: u32 = 4;

/// The `types` option key.
pub const TYPES_OPTION: &str = "types";
/// The `multiple` option key.
pub const MULTIPLE_OPTION: &str = "multiple";
/// The `cursor_mode` option key.
pub const CURSOR_MODE_OPTION: &str = "cursor_mode";

/// The type of one shared source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceType {
    /// A whole monitor.
    Monitor,
    /// One application window.
    Window,
    /// A virtual monitor (reserved; not advertised yet).
    Virtual,
}

impl SourceType {
    /// The stable label exposed on the wire and the diagnostic surface.
    pub const fn as_str(self) -> &'static str {
        match self {
            SourceType::Monitor => "monitor",
            SourceType::Window => "window",
            SourceType::Virtual => "virtual",
        }
    }

    /// The `types` bitmask value.
    pub const fn bit(self) -> u32 {
        match self {
            SourceType::Monitor => SOURCE_MONITOR,
            SourceType::Window => SOURCE_WINDOW,
            SourceType::Virtual => SOURCE_VIRTUAL,
        }
    }

    /// Parse a single `source_type` value.
    pub const fn parse(value: u32) -> Option<Self> {
        match value {
            SOURCE_MONITOR => Some(SourceType::Monitor),
            SOURCE_WINDOW => Some(SourceType::Window),
            SOURCE_VIRTUAL => Some(SourceType::Virtual),
            _ => None,
        }
    }
}

/// The parsed `options` vardict of `SelectSources` (and the type defaults of
/// `CreateSession`). Only the keys the picker and the result need are decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenCastOptions {
    /// `types` bitmask; defaults to monitors only, per the spec.
    pub types: u32,
    /// `multiple`; defaults to false.
    pub multiple: bool,
    /// `cursor_mode`; defaults to hidden.
    pub cursor_mode: u32,
}

impl Default for ScreenCastOptions {
    fn default() -> Self {
        ScreenCastOptions {
            types: SOURCE_MONITOR,
            multiple: false,
            cursor_mode: CURSOR_HIDDEN,
        }
    }
}

impl ScreenCastOptions {
    /// Decode the subset of `options` the model understands. A key of the
    /// wrong type is treated as absent.
    pub fn from_raw(options: &HashMap<String, OwnedValue>) -> Self {
        ScreenCastOptions {
            types: u32_of(options, TYPES_OPTION).unwrap_or(SOURCE_MONITOR),
            multiple: bool_of(options, MULTIPLE_OPTION).unwrap_or(false),
            cursor_mode: u32_of(options, CURSOR_MODE_OPTION).unwrap_or(CURSOR_HIDDEN),
        }
    }

    /// Whether the request allows monitors.
    pub const fn has_monitor(&self) -> bool {
        self.types & SOURCE_MONITOR != 0
    }

    /// Whether the request allows windows.
    pub const fn has_window(&self) -> bool {
        self.types & SOURCE_WINDOW != 0
    }

    /// Whether the request allows more than one source.
    pub const fn allows_multiple(&self) -> bool {
        self.multiple
    }
}

/// One live picker request: the identity the frontend passed, the parsed
/// options, and the session it belongs to. The picker reads it; the completer
/// resolves it.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenCastRequest {
    /// The Request object path the frontend supplied.
    pub handle: String,
    /// The Session object path the frontend supplied.
    pub session_handle: String,
    /// The calling application's id.
    pub app_id: String,
    /// The decoded options.
    pub options: ScreenCastOptions,
    /// The raw `options` vardict, kept so the diagnostic signal carries it
    /// unchanged.
    pub raw_options: HashMap<String, OwnedValue>,
}

impl ScreenCastRequest {
    /// Build a request from a D-Bus call.
    pub fn new(
        handle: impl Into<String>,
        session_handle: impl Into<String>,
        app_id: impl Into<String>,
        options: &HashMap<String, OwnedValue>,
    ) -> Self {
        ScreenCastRequest {
            handle: handle.into(),
            session_handle: session_handle.into(),
            app_id: app_id.into(),
            options: ScreenCastOptions::from_raw(options),
            raw_options: options.clone(),
        }
    }
}

/// One chosen source: the picker's token plus the type it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenCastSelection {
    /// The presenter's opaque source handle.
    pub id: String,
    /// The kind of source the handle names.
    pub source_type: SourceType,
}

impl ScreenCastSelection {
    /// A monitor selection.
    pub fn monitor(id: impl Into<String>) -> Self {
        ScreenCastSelection {
            id: id.into(),
            source_type: SourceType::Monitor,
        }
    }

    /// A window selection.
    pub fn window(id: impl Into<String>) -> Self {
        ScreenCastSelection {
            id: id.into(),
            source_type: SourceType::Window,
        }
    }
}

/// One stream the `Start` result carries. T-13.4a returns the chosen source
/// handle; T-13.4b negotiates the mode (live PipeWire, or the named stills
/// fallback) through [`crate::stream`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenCastStream {
    /// The PipeWire node id. `0` in the stills fallback.
    pub node_id: u32,
    /// The chosen source's opaque handle (a Dragonfruit extension property,
    /// the carry the shell/T-13.4b needs to address the source).
    pub source_id: String,
    /// The kind of the shared source.
    pub source_type: SourceType,
    /// How the source is delivered.
    pub mode: StreamMode,
    /// The named reason for the stills fallback, when one was used.
    pub fallback: Option<FallbackReason>,
}

impl ScreenCastStream {
    /// Project the stream onto the `(u, a{sv})` wire tuple.
    pub fn to_wire(&self) -> (u32, HashMap<String, OwnedValue>) {
        let mut properties = HashMap::new();
        properties.insert(
            "id".to_owned(),
            OwnedValue::from(Str::from(self.source_id.as_str())),
        );
        properties.insert(
            "source_type".to_owned(),
            OwnedValue::from(self.source_type.bit()),
        );
        properties.extend(crate::stream::stream_properties(self.mode, self.fallback));
        (self.node_id, properties)
    }

    /// Decode a wire tuple back into a stream (used by the response tests). A
    /// missing mode defaults to the stills fallback, so an old peer can never
    /// look like a live stream it did not negotiate.
    pub fn from_wire(node_id: u32, properties: &HashMap<String, OwnedValue>) -> Option<Self> {
        let source_id = properties
            .get("id")
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| String::try_from(value).ok())?;
        let bit = properties
            .get("source_type")
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| u32::try_from(value).ok())?;
        let mode = properties
            .get(crate::stream::STREAM_MODE_PROPERTY)
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| String::try_from(value).ok())
            .and_then(|label| StreamMode::parse(&label))
            .unwrap_or(StreamMode::Stills);
        let fallback = properties
            .get(crate::stream::STREAM_FALLBACK_PROPERTY)
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| String::try_from(value).ok())
            .and_then(|label| match label.as_str() {
                "pipewire-producer-unavailable" => Some(FallbackReason::ProducerUnavailable),
                "source-unavailable" => Some(FallbackReason::SourceUnavailable),
                _ => None,
            });
        Some(ScreenCastStream {
            node_id,
            source_id,
            source_type: SourceType::parse(bit)?,
            mode,
            fallback,
        })
    }
}

/// The portal's response plus results vardict.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenCastResponse {
    /// 0 success, 1 cancelled, 2 other error.
    pub response: u32,
    /// The results vardict.
    pub results: HashMap<String, OwnedValue>,
}

impl ScreenCastResponse {
    /// A successful response with the given results.
    pub fn success(results: HashMap<String, OwnedValue>) -> Self {
        ScreenCastResponse {
            response: RESPONSE_SUCCESS,
            results,
        }
    }

    /// A successful `Start` carrying the chosen streams.
    pub fn started(streams: &[ScreenCastStream]) -> Self {
        // `a(ua{sv})`: one struct per stream, built explicitly so the array's
        // element signature is the struct and not a variant.
        let mut array = Array::new(<(u32, HashMap<String, OwnedValue>) as Type>::SIGNATURE);
        for stream in streams {
            let (node_id, properties) = stream.to_wire();
            if array
                .append(Value::Structure(Structure::from((node_id, properties))))
                .is_err()
            {
                return ScreenCastResponse::other();
            }
        }
        let mut results = HashMap::new();
        if let Ok(value) = OwnedValue::try_from(array) {
            results.insert("streams".to_owned(), value);
        }
        ScreenCastResponse {
            response: RESPONSE_SUCCESS,
            results,
        }
    }

    /// The user cancelled.
    pub fn cancelled() -> Self {
        ScreenCastResponse {
            response: RESPONSE_CANCELLED,
            results: HashMap::new(),
        }
    }

    /// Any other failure.
    pub fn other() -> Self {
        ScreenCastResponse {
            response: RESPONSE_OTHER,
            results: HashMap::new(),
        }
    }

    /// The streams in a successful `Start` response.
    pub fn streams(&self) -> Vec<ScreenCastStream> {
        self.results
            .get("streams")
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| Vec::<(u32, HashMap<String, OwnedValue>)>::try_from(value).ok())
            .map(|wire| {
                wire.into_iter()
                    .filter_map(|(node_id, properties)| {
                        ScreenCastStream::from_wire(node_id, &properties)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// One live ScreenCast session.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenCastSession {
    /// The calling application's id.
    pub app_id: String,
    /// The object path the session is served at.
    pub handle: String,
    /// The last `SelectSources` options (or the spec defaults before one).
    pub options: ScreenCastOptions,
    /// The chosen sources, once the picker resolved.
    pub selections: Vec<ScreenCastSelection>,
}

impl ScreenCastSession {
    /// The streams this session's `Start` should return, negotiated through
    /// `negotiator`. A transport failure becomes the named stills fallback.
    pub fn streams(&mut self, negotiator: &mut StreamNegotiator) -> Vec<ScreenCastStream> {
        let cursor_mode = self.options.cursor_mode;
        self.selections
            .iter()
            .map(|selection| {
                let source = StreamSource {
                    id: selection.id.clone(),
                    source_type: selection.source_type,
                    cursor_mode,
                };
                let negotiated = negotiator.negotiate(&source);
                ScreenCastStream {
                    node_id: negotiated.node_id,
                    source_id: selection.id.clone(),
                    source_type: selection.source_type,
                    mode: negotiated.mode,
                    fallback: negotiated.fallback,
                }
            })
            .collect()
    }
}

/// Why a ScreenCast operation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenCastError {
    /// A session already exists at that path.
    Duplicate(String),
    /// No session exists at that path.
    UnknownSession(String),
    /// A picker request already exists at that handle.
    DuplicateRequest(String),
    /// No picker request exists at that handle.
    UnknownRequest(String),
    /// `Start` ran before a source was chosen.
    NoSelection(String),
}

impl std::fmt::Display for ScreenCastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenCastError::Duplicate(handle) => write!(f, "session {handle} already exists"),
            ScreenCastError::UnknownSession(handle) => write!(f, "no session at {handle}"),
            ScreenCastError::DuplicateRequest(handle) => {
                write!(f, "screen cast {handle} already exists")
            }
            ScreenCastError::UnknownRequest(handle) => write!(f, "no screen cast at {handle}"),
            ScreenCastError::NoSelection(handle) => {
                write!(f, "session {handle} has no chosen source")
            }
        }
    }
}

impl std::error::Error for ScreenCastError {}

/// The waiter half of a one-shot completion. The D-Bus method awaits it; the
/// presenter's completion wakes it.
pub struct ScreenCastCompletion<T> {
    inner: Arc<Mutex<CompletionState<T>>>,
}

struct CompletionState<T> {
    value: Option<T>,
    waker: Option<Waker>,
    closed: bool,
}

impl<T> Future for ScreenCastCompletion<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = lock_inner(&self.inner);
        if let Some(value) = inner.value.take() {
            return Poll::Ready(Some(value));
        }
        if inner.closed {
            return Poll::Ready(None);
        }
        inner.waker = Some(context.waker().clone());
        Poll::Pending
    }
}

/// The completing half of a one-shot completion.
pub struct ScreenCastCompleter<T> {
    inner: Arc<Mutex<CompletionState<T>>>,
}

impl<T> ScreenCastCompleter<T> {
    /// Deliver the value. Returns whether this completion won the race.
    pub fn complete(&self, value: T) -> bool {
        let mut inner = lock_inner(&self.inner);
        if inner.value.is_some() || inner.closed {
            return false;
        }
        inner.value = Some(value);
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
        true
    }
}

/// Build a one-shot completion pair.
pub fn completion<T>() -> (ScreenCastCompleter<T>, ScreenCastCompletion<T>) {
    let inner = Arc::new(Mutex::new(CompletionState {
        value: None,
        waker: None,
        closed: false,
    }));
    (
        ScreenCastCompleter {
            inner: inner.clone(),
        },
        ScreenCastCompletion { inner },
    )
}

/// Lock completion state, recovering from a poisoned mutex.
fn lock_inner<T>(inner: &Mutex<CompletionState<T>>) -> MutexGuard<'_, CompletionState<T>> {
    inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One registered picker request and its completion.
struct PendingScreenCast {
    request: ScreenCastRequest,
    completer: ScreenCastCompleter<ScreenCastResponse>,
}

/// A live PipeWire node one session holds: the id a client links to and the
/// source handle it streams, so a release and the diagnostic surface can name
/// the same stream.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveNode {
    node_id: u32,
    source_id: String,
}

/// The bookkeeping for every live session and picker request.
#[derive(Default)]
pub struct ScreenCastRegistry {
    sessions: BTreeMap<String, ScreenCastSession>,
    pending: BTreeMap<String, PendingScreenCast>,
    negotiator: StreamNegotiator,
    /// The live PipeWire nodes a session created through `Start`, so they can
    /// be released when the session closes or starts again (T335b).
    nodes: BTreeMap<String, Vec<LiveNode>>,
}

impl ScreenCastRegistry {
    /// An empty registry over the shipped stills-only transport.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty registry over the shipped live transport (T335b): the
    /// compositor bridge opened from the backend's session provisioning, or
    /// the named stills fallback when that provisioning is absent.
    pub fn live() -> Self {
        ScreenCastRegistry::with_transport(Box::new(
            crate::stream::LiveTransport::from_environment(),
        ))
    }

    /// An empty registry over `transport`, for the live path and tests.
    pub fn with_transport(transport: Box<dyn StreamTransport>) -> Self {
        ScreenCastRegistry {
            negotiator: StreamNegotiator::new(transport),
            ..Self::default()
        }
    }

    /// How this registry's transport delivers a source (`pipewire` / `stills`).
    pub fn stream_mode(&self) -> StreamMode {
        self.negotiator.mode()
    }

    /// Register a session at `handle` for `app_id`. A duplicate path is
    /// refused so a caller cannot hijack another session.
    pub fn create_session(
        &mut self,
        app_id: impl Into<String>,
        handle: impl Into<String>,
    ) -> Result<(), ScreenCastError> {
        let handle = handle.into();
        if self.sessions.contains_key(&handle) {
            return Err(ScreenCastError::Duplicate(handle));
        }
        self.sessions.insert(
            handle.clone(),
            ScreenCastSession {
                app_id: app_id.into(),
                handle,
                options: ScreenCastOptions::default(),
                selections: Vec::new(),
            },
        );
        Ok(())
    }

    /// Register the picker for a `SelectSources` call and hand the caller the
    /// future to await. The session must exist.
    pub fn begin_select(
        &mut self,
        request: ScreenCastRequest,
    ) -> Result<ScreenCastCompletion<ScreenCastResponse>, ScreenCastError> {
        if !self.sessions.contains_key(&request.session_handle) {
            return Err(ScreenCastError::UnknownSession(request.session_handle));
        }
        if self.pending.contains_key(&request.handle) {
            return Err(ScreenCastError::DuplicateRequest(request.handle));
        }
        if let Some(session) = self.sessions.get_mut(&request.session_handle) {
            session.options = request.options;
        }
        let (completer, completion) = completion();
        self.pending.insert(
            request.handle.clone(),
            PendingScreenCast { request, completer },
        );
        Ok(completion)
    }

    /// A live picker request.
    pub fn request(&self, handle: &str) -> Option<&ScreenCastRequest> {
        self.pending.get(handle).map(|pending| &pending.request)
    }

    /// The `(request handle, session handle)` pairs a presenter should offer.
    pub fn handles(&self) -> Vec<(String, String)> {
        self.pending
            .iter()
            .map(|(handle, pending)| (handle.clone(), pending.request.session_handle.clone()))
            .collect()
    }

    /// A live session.
    pub fn session(&self, handle: &str) -> Option<&ScreenCastSession> {
        self.sessions.get(handle)
    }

    /// Resolve a picker request with the presenter's chosen sources and record
    /// them on the session. Returns the response delivered to the awaiting
    /// caller, or `None` for an unknown handle. An empty selection is an error
    /// response, never a silent success.
    pub fn complete(
        &mut self,
        handle: &str,
        selections: Vec<ScreenCastSelection>,
    ) -> Option<ScreenCastResponse> {
        let pending = self.pending.remove(handle)?;
        let response = if selections.is_empty() {
            ScreenCastResponse::other()
        } else {
            if let Some(session) = self.sessions.get_mut(&pending.request.session_handle) {
                session.selections = selections;
            }
            ScreenCastResponse::success(HashMap::new())
        };
        pending.completer.complete(response.clone());
        Some(response)
    }

    /// Resolve a picker request as cancelled. Returns the response, or `None`
    /// for an unknown handle.
    pub fn cancel(&mut self, handle: &str) -> Option<ScreenCastResponse> {
        let pending = self.pending.remove(handle)?;
        let response = ScreenCastResponse::cancelled();
        pending.completer.complete(response.clone());
        Some(response)
    }

    /// Resolve every live picker request as cancelled and remove it. Sessions
    /// are left alone: only a `SelectSources` caller is waiting. Returns the
    /// number resolved. The presenter-loss watch calls this when the shell
    /// leaves the bus (T311).
    pub fn cancel_all(&mut self) -> usize {
        let handles: Vec<String> = self.pending.keys().cloned().collect();
        handles
            .iter()
            .filter(|handle| self.cancel(handle).is_some())
            .count()
    }

    /// Build the `Start` response for a session's chosen sources, negotiating
    /// each stream through the transport.
    ///
    /// A session owns its live nodes: a second `Start` releases the previous
    /// set first, so a repeated `Start` (or a changed selection) never leaks a
    /// node. The new set is remembered (with its source handle) so
    /// [`ScreenCastRegistry::close_session`] and
    /// [`ScreenCastRegistry::release_streams`] can release it and the shell
    /// bridge can name it.
    pub fn start(&mut self, session_handle: &str) -> Result<ScreenCastResponse, ScreenCastError> {
        let session = self
            .sessions
            .get_mut(session_handle)
            .ok_or_else(|| ScreenCastError::UnknownSession(session_handle.to_owned()))?;
        if session.selections.is_empty() {
            return Err(ScreenCastError::NoSelection(session_handle.to_owned()));
        }
        let streams = session.streams(&mut self.negotiator);
        self.release_nodes(session_handle);
        let nodes: Vec<LiveNode> = streams
            .iter()
            .filter(|stream| stream.mode == StreamMode::PipeWire)
            .map(|stream| LiveNode {
                node_id: stream.node_id,
                source_id: stream.source_id.clone(),
            })
            .collect();
        if !nodes.is_empty() {
            self.nodes.insert(session_handle.to_owned(), nodes);
        }
        Ok(ScreenCastResponse::started(&streams))
    }

    /// Drop a session and any picker request that referenced it, releasing any
    /// live nodes the session created. Returns whether the session existed.
    ///
    /// This is the one session-close path: the portal's `Session.Close`, a
    /// client disconnect, and an app restart all land here.
    pub fn close_session(&mut self, handle: &str) -> bool {
        let existed = self.sessions.remove(handle).is_some();
        self.release_nodes(handle);
        self.pending
            .retain(|_, pending| pending.request.session_handle != handle);
        existed
    }

    /// Release every live stream, whichever session owns it, and answer every
    /// waiting picker with `cancelled`. The trusted shell is the only party
    /// that can create a node, so when it leaves (the presenter-loss watch) no
    /// stream can survive it (T335b). Returns `(streams released, requests
    /// cancelled)`.
    pub fn release_streams(&mut self) -> (usize, usize) {
        let live: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, nodes)| !nodes.is_empty())
            .map(|(handle, _)| handle.clone())
            .collect();
        let mut released = 0;
        for handle in live {
            released += self.release_nodes(&handle);
        }
        (released, self.cancel_all())
    }

    /// The live node ids one session holds (empty when it has none).
    pub fn live_nodes(&self, session_handle: &str) -> Vec<u32> {
        self.nodes
            .get(session_handle)
            .map(|nodes| nodes.iter().map(|node| node.node_id).collect())
            .unwrap_or_default()
    }

    /// The live nodes one session holds with the source each streams, in
    /// creation order (empty when it has none).
    pub fn live_streams(&self, session_handle: &str) -> Vec<(u32, String)> {
        self.nodes
            .get(session_handle)
            .map(|nodes| {
                nodes
                    .iter()
                    .map(|node| (node.node_id, node.source_id.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Create one monitor stream through the transport and return its node id,
    /// without a D-Bus session (the T335b live check, `--check-screencast
    /// --create`). The node is not owned by a session, so the caller owns it;
    /// a refusal is the transport's named fallback.
    pub fn create_stream_for_check(&mut self) -> Result<u32, String> {
        let source = crate::stream::StreamSource {
            id: "monitor:".to_owned(),
            source_type: SourceType::Monitor,
            cursor_mode: 1,
        };
        let negotiated = self.negotiator.negotiate(&source);
        if negotiated.mode != StreamMode::PipeWire {
            let reason = negotiated
                .fallback
                .map(|reason| reason.as_str())
                .unwrap_or("stills");
            // The bridge's own detail names what the compositor refused with.
            return Err(match self.negotiator.transport_error() {
                Some(detail) => format!("stills fallback ({reason}): {detail}"),
                None => format!("stills fallback ({reason})"),
            });
        }
        Ok(negotiated.node_id)
    }

    /// The number of live nodes across every session.
    pub fn live_node_count(&self) -> usize {
        self.nodes.values().map(Vec::len).sum()
    }

    /// Re-open the live transport's compositor connection after the party it
    /// depends on left (the presenter-loss watch, T335b). Lists the live
    /// streams were released by the caller first. Returns whether a verified
    /// live connection is open afterwards; on a stills transport this is a
    /// no-op.
    ///
    /// A restart re-issues the portal token, so the reopen re-reads the
    /// backend's provisioning; `connect` substitutes a test resolver.
    pub fn reconnect_transport(&mut self) -> bool {
        self.rescan_transport(None)
    }

    /// [`Self::reconnect_transport`] over a caller's resolver (a test's own
    /// socket and token), so it never writes the process environment.
    pub fn rescan_transport(&mut self, connect: Option<crate::stream::ConnectFn>) -> bool {
        self.nodes.clear();
        self.negotiator.rescan(connect)
    }

    /// Why the live transport is not connected, for the log and the
    /// diagnostic surface. `None` on a stills transport (nothing to connect).
    pub fn transport_error(&self) -> Option<String> {
        if self.negotiator.mode() == StreamMode::PipeWire {
            None
        } else {
            self.negotiator.transport_error()
        }
    }

    /// Release one session's nodes and forget them.
    fn release_nodes(&mut self, session_handle: &str) -> usize {
        let nodes = self.nodes.remove(session_handle).unwrap_or_default();
        for node in &nodes {
            self.negotiator.destroy(node.node_id);
        }
        nodes.len()
    }

    /// The number of live sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether there are no live sessions.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// The number of live picker requests.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

/// The shared, live ScreenCast registry: the D-Bus objects and the diagnostic
/// surface all read the same bookkeeping.
pub type SharedScreenCast = Arc<Mutex<ScreenCastRegistry>>;

/// A shared, empty registry over the shipped stills transport.
///
/// The daemon serves this one: until the trusted shell's bridge answers a
/// stream request, every `Start` is the named
/// [`crate::stream::FallbackReason::ProducerUnavailable`] fallback, which is
/// the honest state for a session with no shell or no PipeWire host.
pub fn registry() -> SharedScreenCast {
    Arc::new(Mutex::new(ScreenCastRegistry::new()))
}

/// The shared registry the daemon serves (T335b): the live transport opened
/// from the session provisioning, or the named stills fallback when the
/// portal was not given a compositor bridge. Must be built on the thread that
/// serves stream requests (the daemon's main thread).
pub fn registry_live() -> SharedScreenCast {
    Arc::new(Mutex::new(ScreenCastRegistry::live()))
}

/// Lock a registry, recovering from a poisoned mutex so a panicking D-Bus call
/// never takes the service down.
pub fn lock(registry: &SharedScreenCast) -> MutexGuard<'_, ScreenCastRegistry> {
    registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A boolean option.
fn bool_of(options: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    options
        .get(key)
        .and_then(|value| bool::try_from(value).ok())
}

/// An unsigned integer option.
fn u32_of(options: &HashMap<String, OwnedValue>, key: &str) -> Option<u32> {
    options
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| u32::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    use zbus::zvariant::Str;

    #[test]
    fn options_default_to_monitors_and_decode_the_three_keys() {
        let options = ScreenCastOptions::from_raw(&HashMap::new());
        assert_eq!(options.types, SOURCE_MONITOR);
        assert!(!options.multiple);
        assert_eq!(options.cursor_mode, CURSOR_HIDDEN);

        let mut raw = HashMap::new();
        raw.insert(
            TYPES_OPTION.to_owned(),
            OwnedValue::from(SOURCE_MONITOR | SOURCE_WINDOW),
        );
        raw.insert(MULTIPLE_OPTION.to_owned(), OwnedValue::from(true));
        raw.insert(
            CURSOR_MODE_OPTION.to_owned(),
            OwnedValue::from(CURSOR_EMBEDDED),
        );
        let decoded = ScreenCastOptions::from_raw(&raw);
        assert!(decoded.has_monitor() && decoded.has_window());
        assert!(decoded.allows_multiple());
        assert_eq!(decoded.cursor_mode, CURSOR_EMBEDDED);
    }

    #[test]
    fn source_types_round_trip_through_their_bit() {
        for source in [SourceType::Monitor, SourceType::Window, SourceType::Virtual] {
            assert_eq!(SourceType::parse(source.bit()), Some(source));
            assert_eq!(SourceType::parse(source.bit() | 8), None);
        }
        assert_eq!(SourceType::Monitor.as_str(), "monitor");
        assert_eq!(SourceType::Window.as_str(), "window");
    }

    #[test]
    fn a_request_decodes_its_session_and_options() {
        let mut raw = HashMap::new();
        raw.insert(TYPES_OPTION.to_owned(), OwnedValue::from(SOURCE_WINDOW));
        raw.insert(
            "handle_token".to_owned(),
            OwnedValue::from(Str::from("tok")),
        );
        let request = ScreenCastRequest::new("/req/1", "/session/1", "org.example.App", &raw);
        assert_eq!(request.handle, "/req/1");
        assert_eq!(request.session_handle, "/session/1");
        assert!(request.options.has_window());
        assert!(!request.options.has_monitor());
    }

    #[test]
    fn the_session_round_trips_create_select_start() {
        let mut registry = ScreenCastRegistry::new();
        registry
            .create_session("org.example.App", "/session/1")
            .expect("create");

        let mut raw = HashMap::new();
        raw.insert(
            TYPES_OPTION.to_owned(),
            OwnedValue::from(SOURCE_MONITOR | SOURCE_WINDOW),
        );
        raw.insert(MULTIPLE_OPTION.to_owned(), OwnedValue::from(true));
        let request = ScreenCastRequest::new("/req/1", "/session/1", "org.example.App", &raw);
        let mut completion = registry.begin_select(request).expect("begin_select");
        assert_eq!(registry.pending_len(), 1);
        assert_eq!(
            registry.handles(),
            vec![("/req/1".to_owned(), "/session/1".to_owned())]
        );
        assert_eq!(
            registry.session("/session/1").unwrap().options.types,
            SOURCE_MONITOR | SOURCE_WINDOW
        );

        // Start before a selection is refused.
        assert_eq!(
            registry.start("/session/1").err(),
            Some(ScreenCastError::NoSelection("/session/1".to_owned()))
        );

        let response = registry
            .complete(
                "/req/1",
                vec![
                    ScreenCastSelection::monitor("monitor:DP-1"),
                    ScreenCastSelection::window("window:7"),
                ],
            )
            .expect("complete");
        assert_eq!(response.response, RESPONSE_SUCCESS);
        assert!(matches!(poll_once(&mut completion), Poll::Ready(Some(_))));
        assert!(registry.pending_len() == 0);

        let started = registry.start("/session/1").expect("start");
        let streams = started.streams();
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0].source_id, "monitor:DP-1");
        assert_eq!(streams[0].source_type, SourceType::Monitor);
        // The shipped transport names the stills fallback: node id 0 and a
        // reason, never a silent success.
        assert_eq!(streams[0].node_id, 0);
        assert_eq!(streams[0].mode, StreamMode::Stills);
        assert_eq!(
            streams[0].fallback,
            Some(FallbackReason::ProducerUnavailable)
        );
        assert_eq!(streams[1].source_id, "window:7");
        assert_eq!(registry.stream_mode(), StreamMode::Stills);
    }

    /// A transport that creates deterministic node ids and records releases.
    struct RecordingTransport {
        next_node: u32,
        destroyed: Arc<Mutex<Vec<u32>>>,
    }

    impl StreamTransport for RecordingTransport {
        fn mode(&self) -> StreamMode {
            StreamMode::PipeWire
        }

        fn create(&mut self, _source: &StreamSource) -> Result<u32, FallbackReason> {
            let node = self.next_node;
            self.next_node += 1;
            Ok(node)
        }

        fn destroy(&mut self, node_id: u32) {
            self.destroyed.lock().unwrap().push(node_id);
        }
    }

    #[test]
    fn a_live_transport_fills_the_node_and_releases_it_on_close() {
        let destroyed = Arc::new(Mutex::new(Vec::new()));
        let mut registry = ScreenCastRegistry::with_transport(Box::new(RecordingTransport {
            next_node: 100,
            destroyed: destroyed.clone(),
        }));
        assert_eq!(registry.stream_mode(), StreamMode::PipeWire);

        registry.create_session("app", "/s").unwrap();
        let request = ScreenCastRequest::new("/r", "/s", "app", &HashMap::new());
        registry.begin_select(request).expect("begin_select");
        registry
            .complete("/r", vec![ScreenCastSelection::monitor("monitor:DP-1")])
            .expect("complete");

        let streams = registry.start("/s").expect("start").streams();
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].node_id, 100, "the live node id reaches the wire");
        assert_eq!(streams[0].mode, StreamMode::PipeWire);
        assert_eq!(streams[0].fallback, None);

        assert!(registry.close_session("/s"));
        assert_eq!(
            *destroyed.lock().unwrap(),
            vec![100],
            "the node is released"
        );
    }

    /// A registry with one started live session, returning the registry and
    /// the release ledger the transport records into.
    fn started_live_session(
        session_handle: &str,
        sources: &[ScreenCastSelection],
        first_node: u32,
    ) -> (ScreenCastRegistry, Arc<Mutex<Vec<u32>>>) {
        let destroyed = Arc::new(Mutex::new(Vec::new()));
        let mut registry = ScreenCastRegistry::with_transport(Box::new(RecordingTransport {
            next_node: first_node,
            destroyed: destroyed.clone(),
        }));
        registry.create_session("app", session_handle).unwrap();
        let request = ScreenCastRequest::new(
            format!("{session_handle}/req"),
            session_handle,
            "app",
            &HashMap::new(),
        );
        registry.begin_select(request).expect("begin_select");
        registry
            .complete(&format!("{session_handle}/req"), sources.to_vec())
            .expect("complete");
        registry.start(session_handle).expect("start");
        (registry, destroyed)
    }

    #[test]
    fn a_second_start_on_one_session_releases_the_first_set_of_nodes() {
        let (mut registry, destroyed) = started_live_session(
            "/s",
            &[
                ScreenCastSelection::monitor("monitor:DP-1"),
                ScreenCastSelection::window("window:7"),
            ],
            10,
        );
        assert_eq!(registry.live_streams("/s").len(), 2);
        assert_eq!(registry.live_node_count(), 2);

        // A repeated Start releases the previous set first: one live node set
        // at a time, never two.
        let streams = registry.start("/s").expect("second start").streams();
        assert_eq!(streams.len(), 2);
        assert_eq!(
            *destroyed.lock().unwrap(),
            vec![10, 11],
            "the first Start's nodes are released before the second creates its own"
        );
        assert_eq!(
            registry.live_nodes("/s"),
            vec![streams[0].node_id, streams[1].node_id]
        );

        assert!(registry.close_session("/s"));
        assert_eq!(*destroyed.lock().unwrap(), vec![10, 11, 12, 13]);
        assert_eq!(registry.live_node_count(), 0);
        assert!(registry.live_nodes("/s").is_empty());
    }

    #[test]
    fn two_sessions_stream_independently_and_close_only_their_own_nodes() {
        let destroyed = Arc::new(Mutex::new(Vec::new()));
        let mut registry = ScreenCastRegistry::with_transport(Box::new(RecordingTransport {
            next_node: 50,
            destroyed: destroyed.clone(),
        }));
        for (session, source) in [
            ("/s1", ScreenCastSelection::monitor("monitor:DP-1")),
            ("/s2", ScreenCastSelection::window("window:3")),
        ] {
            registry.create_session("app", session).unwrap();
            let request =
                ScreenCastRequest::new(format!("{session}/r"), session, "app", &HashMap::new());
            registry.begin_select(request).expect("begin_select");
            registry
                .complete(&format!("{session}/r"), vec![source])
                .expect("complete");
            registry.start(session).expect("start");
        }
        assert_eq!(registry.live_nodes("/s1"), vec![50]);
        assert_eq!(registry.live_nodes("/s2"), vec![51]);
        assert_eq!(
            registry.live_streams("/s1"),
            vec![(50, "monitor:DP-1".to_owned())]
        );

        // Closing one session leaves the other session's node running.
        assert!(registry.close_session("/s1"));
        assert_eq!(*destroyed.lock().unwrap(), vec![50]);
        assert_eq!(registry.live_nodes("/s2"), vec![51]);

        // Closing the unknown second time releases nothing again.
        assert!(!registry.close_session("/s1"));
        assert_eq!(*destroyed.lock().unwrap(), vec![50]);
        assert!(registry.close_session("/s2"));
        assert_eq!(*destroyed.lock().unwrap(), vec![50, 51]);
        assert_eq!(registry.live_node_count(), 0);
    }

    #[test]
    fn a_client_disconnect_and_the_presenter_loss_release_every_stream() {
        let destroyed = Arc::new(Mutex::new(Vec::new()));
        let mut registry = ScreenCastRegistry::with_transport(Box::new(RecordingTransport {
            next_node: 70,
            destroyed: destroyed.clone(),
        }));
        for session in ["/a", "/b"] {
            registry.create_session("app", session).unwrap();
            let request =
                ScreenCastRequest::new(format!("{session}/r"), session, "app", &HashMap::new());
            registry.begin_select(request).expect("begin_select");
            registry
                .complete(
                    &format!("{session}/r"),
                    vec![ScreenCastSelection::monitor("m")],
                )
                .expect("complete");
            registry.start(session).expect("start");
        }
        // A picker waits when the shell leaves, too.
        registry.create_session("app", "/c").unwrap();
        let waiting = ScreenCastRequest::new("/c/r", "/c", "app", &HashMap::new());
        let mut completion = registry.begin_select(waiting).expect("begin_select");

        let (released, cancelled) = registry.release_streams();
        assert_eq!(released, 2, "both live nodes are released");
        assert_eq!(cancelled, 1, "the waiting picker is answered");
        assert_eq!(*destroyed.lock().unwrap(), vec![70, 71]);
        assert_eq!(registry.live_node_count(), 0);
        assert!(matches!(
            poll_once(&mut completion),
            Poll::Ready(Some(ScreenCastResponse {
                response: RESPONSE_CANCELLED,
                ..
            }))
        ));

        // Sessions survive the presenter loss, but their streams do not: a
        // second loss with nothing live touches nothing.
        assert_eq!(registry.len(), 3);
        assert_eq!(registry.release_streams(), (0, 0));
        assert_eq!(*destroyed.lock().unwrap(), vec![70, 71]);
    }

    #[test]
    fn the_stills_fallback_owns_no_node() {
        let mut registry = ScreenCastRegistry::new();
        registry.create_session("app", "/s").unwrap();
        let request = ScreenCastRequest::new("/r", "/s", "app", &HashMap::new());
        registry.begin_select(request).expect("begin_select");
        registry
            .complete("/r", vec![ScreenCastSelection::monitor("monitor:DP-1")])
            .expect("complete");
        let streams = registry.start("/s").expect("start").streams();
        assert_eq!(streams[0].mode, StreamMode::Stills);
        assert_eq!(registry.live_node_count(), 0);
        assert!(registry.close_session("/s"));
        assert_eq!(registry.live_node_count(), 0);
    }

    #[test]
    fn duplicate_sessions_and_requests_are_refused() {
        let mut registry = ScreenCastRegistry::new();
        registry.create_session("app", "/s").unwrap();
        assert_eq!(
            registry.create_session("other", "/s"),
            Err(ScreenCastError::Duplicate("/s".to_owned()))
        );

        let request = ScreenCastRequest::new("/r", "/s", "app", &HashMap::new());
        registry.begin_select(request).expect("begin_select");
        let duplicate = ScreenCastRequest::new("/r", "/s", "app", &HashMap::new());
        assert_eq!(
            registry.begin_select(duplicate).err(),
            Some(ScreenCastError::DuplicateRequest("/r".to_owned()))
        );

        // A request for an unknown session is refused.
        let orphan = ScreenCastRequest::new("/r2", "/missing", "app", &HashMap::new());
        assert_eq!(
            registry.begin_select(orphan).err(),
            Some(ScreenCastError::UnknownSession("/missing".to_owned()))
        );
    }

    #[test]
    fn an_empty_selection_is_an_error_and_cancel_resolves_the_waiter() {
        let mut registry = ScreenCastRegistry::new();
        registry.create_session("app", "/s").unwrap();

        let request = ScreenCastRequest::new("/r", "/s", "app", &HashMap::new());
        let mut completion = registry.begin_select(request).expect("begin_select");
        let response = registry.complete("/r", Vec::new()).expect("complete");
        assert_eq!(response.response, RESPONSE_OTHER);
        assert!(matches!(
            poll_once(&mut completion),
            Poll::Ready(Some(ScreenCastResponse {
                response: RESPONSE_OTHER,
                ..
            }))
        ));
        // Nothing was recorded on the session.
        assert!(registry.start("/s").is_err());

        let request = ScreenCastRequest::new("/r2", "/s", "app", &HashMap::new());
        let mut completion = registry.begin_select(request).expect("begin_select");
        let response = registry.cancel("/r2").expect("cancel");
        assert_eq!(response.response, RESPONSE_CANCELLED);
        assert!(matches!(
            poll_once(&mut completion),
            Poll::Ready(Some(ScreenCastResponse {
                response: RESPONSE_CANCELLED,
                ..
            }))
        ));
        assert!(registry.cancel("/r2").is_none());
    }

    #[test]
    fn closing_a_session_drops_its_pending_picker() {
        let mut registry = ScreenCastRegistry::new();
        registry.create_session("app", "/s").unwrap();
        let request = ScreenCastRequest::new("/r", "/s", "app", &HashMap::new());
        registry.begin_select(request).expect("begin_select");
        assert_eq!(registry.pending_len(), 1);
        assert!(registry.close_session("/s"));
        assert_eq!(registry.pending_len(), 0);
        assert!(registry.is_empty());
        assert!(!registry.close_session("/s"));
    }

    #[test]
    fn the_presenter_loss_watch_cancels_every_pending_picker() {
        let mut registry = ScreenCastRegistry::new();
        registry.create_session("app", "/s").unwrap();
        let first = ScreenCastRequest::new("/r1", "/s", "app", &HashMap::new());
        let mut first_completion = registry.begin_select(first).expect("begin_select");
        let second = ScreenCastRequest::new("/r2", "/s", "app", &HashMap::new());
        let mut second_completion = registry.begin_select(second).expect("begin_select");

        assert_eq!(registry.cancel_all(), 2);
        assert_eq!(registry.pending_len(), 0);
        // The session itself survives: only the waiting callers are answered.
        assert!(registry.session("/s").is_some());
        for completion in [&mut first_completion, &mut second_completion] {
            assert!(matches!(
                poll_once(completion),
                Poll::Ready(Some(ScreenCastResponse {
                    response: RESPONSE_CANCELLED,
                    ..
                }))
            ));
        }
        assert_eq!(registry.cancel_all(), 0);
    }

    #[test]
    fn the_advertised_capabilities_are_honest() {
        assert_eq!(AVAILABLE_SOURCE_TYPES, SOURCE_MONITOR | SOURCE_WINDOW);
        assert_eq!(AVAILABLE_SOURCE_TYPES & SOURCE_VIRTUAL, 0);
        assert_eq!(AVAILABLE_CURSOR_MODES, CURSOR_HIDDEN | CURSOR_EMBEDDED);
        assert_eq!(SCREENCAST_VERSION, 3);
    }

    /// Poll a future once without an executor: used to assert a completion is
    /// already ready.
    fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        Pin::new(future).poll(&mut context)
    }
}
