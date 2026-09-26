// SPDX-License-Identifier: MIT
//! The Screenshot portal's pure model (T-13.3a).
//!
//! The backend serves the standard
//! `org.freedesktop.impl.portal.Screenshot` interface so sandboxed
//! applications can ask the desktop for a single-frame capture. Like the
//! FileChooser (T-13.2a) the interface method is *synchronous* from the
//! frontend's point of view: it does not return until a presenter has
//! produced a capture, which is why [`ScreenshotCompletion`] is a one-shot
//! future the D-Bus method awaits.
//!
//! The desktop's own capture shortcut (Cmd+Shift+3/4) drives the same
//! selection overlay: it opens the presenter directly instead of registering a
//! portal request. The presenter seam is the one
//! [`ScreenshotRegistry`]; the selection UI is T-13.3a, and save/copy are
//! T-13.3b.
//!
//! ## The mode option
//!
//! The standard `Screenshot` options (`modal`, `interactive`, `handle_token`)
//! do not say *which* selection the desktop should show; that is the
//! presenter's job. Dragonfruit extends the options with a `mode` string
//! (`fullscreen` / `region` / `window`) so the desktop's own shortcut path and
//! the headless tests can request a specific mode. A request without it is
//! `fullscreen` unless it is `interactive`, in which case the presenter
//! chooses (the overlay opens in `region` selection mode). This module has no
//! D-Bus dependency, so the options/result logic and the request lifecycle are
//! unit-tested directly.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use zbus::zvariant::{OwnedValue, Str};

/// The standard Screenshot backend interface.
pub const SCREENSHOT_INTERFACE: &str = "org.freedesktop.impl.portal.Screenshot";

/// The Screenshot backend version. Version 2 is the current standard; the
/// interface documents none of the options this model decodes beyond the
/// Dragonfruit `mode` extension.
pub const SCREENSHOT_VERSION: u32 = 2;

/// Portal response codes shared with the caller.
pub const RESPONSE_SUCCESS: u32 = 0;
pub const RESPONSE_CANCELLED: u32 = 1;
pub const RESPONSE_OTHER: u32 = 2;

/// The Dragonfruit `mode` option key (an extension; see the module docs).
pub const MODE_OPTION: &str = "mode";

/// Which selection the overlay should present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureMode {
    /// The whole output, immediately.
    Fullscreen,
    /// A dragged rectangle.
    Region,
    /// One window, chosen under the pointer.
    Window,
}

impl CaptureMode {
    /// The stable label exposed on the wire and the diagnostic surface.
    pub const fn as_str(self) -> &'static str {
        match self {
            CaptureMode::Fullscreen => "fullscreen",
            CaptureMode::Region => "region",
            CaptureMode::Window => "window",
        }
    }

    /// Parse the `mode` option. Unknown or absent is `None`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fullscreen" => Some(CaptureMode::Fullscreen),
            "region" => Some(CaptureMode::Region),
            "window" => Some(CaptureMode::Window),
            _ => None,
        }
    }

    /// The mode a request with no explicit `mode` defaults to.
    pub const fn default_for(interactive: bool) -> Self {
        if interactive {
            CaptureMode::Region
        } else {
            CaptureMode::Fullscreen
        }
    }
}

/// The parsed `options` vardict. Only the keys the presenter needs are
/// decoded; unknown keys (including `handle_token`) are ignored.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScreenshotOptions {
    /// `modal` — whether the dialog is modal (default true).
    pub modal: bool,
    /// `interactive` — present a selection UI rather than capturing
    /// immediately.
    pub interactive: bool,
    /// The Dragonfruit `mode` extension, when present and valid.
    pub mode: Option<CaptureMode>,
}

impl ScreenshotOptions {
    /// Decode the subset of `options` the model understands. A key of the
    /// wrong type is treated as absent.
    pub fn from_raw(options: &HashMap<String, OwnedValue>) -> Self {
        let interactive = bool_of(options, "interactive").unwrap_or(false);
        ScreenshotOptions {
            modal: bool_of(options, "modal").unwrap_or(true),
            interactive,
            mode: string_of(options, MODE_OPTION).and_then(|value| CaptureMode::parse(&value)),
        }
    }

    /// The mode this request resolves to.
    pub fn capture_mode(&self) -> CaptureMode {
        self.mode
            .unwrap_or_else(|| CaptureMode::default_for(self.interactive))
    }
}

/// Why a screenshot request could not be registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenshotError {
    /// A request already exists at that handle.
    Duplicate(String),
    /// No request exists at that handle.
    Unknown(String),
}

impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenshotError::Duplicate(handle) => {
                write!(f, "screenshot {handle} already exists")
            }
            ScreenshotError::Unknown(handle) => write!(f, "no screenshot at {handle}"),
        }
    }
}

impl std::error::Error for ScreenshotError {}

/// One live screenshot request: the identity the frontend passed, the parsed
/// options, and the resolved selection mode. The presenter reads it; the
/// completer resolves it.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenshotRequest {
    /// The Request object path the frontend supplied.
    pub handle: String,
    /// The calling application's id.
    pub app_id: String,
    /// The parent window identifier (may be empty).
    pub parent_window: String,
    /// The decoded options.
    pub options: ScreenshotOptions,
    /// The selection mode the presenter should open with.
    pub mode: CaptureMode,
    /// The raw `options` vardict, kept so the diagnostic signal carries it
    /// unchanged.
    pub raw_options: HashMap<String, OwnedValue>,
}

impl ScreenshotRequest {
    /// Build a request from a D-Bus call.
    pub fn new(
        handle: impl Into<String>,
        app_id: impl Into<String>,
        parent_window: impl Into<String>,
        options: &HashMap<String, OwnedValue>,
    ) -> Self {
        let decoded = ScreenshotOptions::from_raw(options);
        let mode = decoded.capture_mode();
        ScreenshotRequest {
            handle: handle.into(),
            app_id: app_id.into(),
            parent_window: parent_window.into(),
            options: decoded,
            mode,
            raw_options: options.clone(),
        }
    }

    /// The success response for a presenter's captured URI.
    ///
    /// The URI is normalized to a canonical `file://` URI through files-core,
    /// the same rule the FileChooser uses; a value that cannot be normalized
    /// is an error response, never a silent success.
    pub fn success(&self, uri: &str) -> ScreenshotResponse {
        let Some(normalized) = crate::chooser::normalize_uri(uri) else {
            return ScreenshotResponse::other();
        };
        let mut results: HashMap<String, OwnedValue> = HashMap::new();
        results.insert(
            "uri".to_owned(),
            OwnedValue::from(Str::from(normalized.as_str())),
        );
        ScreenshotResponse {
            response: RESPONSE_SUCCESS,
            results,
        }
    }
}

/// The portal's response plus results vardict.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenshotResponse {
    /// 0 success, 1 cancelled, 2 other error.
    pub response: u32,
    /// The results vardict.
    pub results: HashMap<String, OwnedValue>,
}

impl ScreenshotResponse {
    /// A successful response with the given results.
    pub fn success(results: HashMap<String, OwnedValue>) -> Self {
        ScreenshotResponse {
            response: RESPONSE_SUCCESS,
            results,
        }
    }

    /// The user cancelled.
    pub fn cancelled() -> Self {
        ScreenshotResponse {
            response: RESPONSE_CANCELLED,
            results: HashMap::new(),
        }
    }

    /// Any other failure.
    pub fn other() -> Self {
        ScreenshotResponse {
            response: RESPONSE_OTHER,
            results: HashMap::new(),
        }
    }

    /// The captured `file://` URI, when the response succeeded.
    pub fn uri(&self) -> Option<String> {
        self.results
            .get("uri")
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| String::try_from(value).ok())
    }
}

/// The waiter half of a one-shot completion. The D-Bus method awaits it; the
/// presenter's completion wakes it.
pub struct ScreenshotCompletion<T> {
    inner: Arc<Mutex<CompletionState<T>>>,
}

struct CompletionState<T> {
    value: Option<T>,
    waker: Option<Waker>,
    closed: bool,
}

impl<T> Future for ScreenshotCompletion<T> {
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
pub struct ScreenshotCompleter<T> {
    inner: Arc<Mutex<CompletionState<T>>>,
}

impl<T> ScreenshotCompleter<T> {
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
pub fn completion<T>() -> (ScreenshotCompleter<T>, ScreenshotCompletion<T>) {
    let inner = Arc::new(Mutex::new(CompletionState {
        value: None,
        waker: None,
        closed: false,
    }));
    (
        ScreenshotCompleter {
            inner: inner.clone(),
        },
        ScreenshotCompletion { inner },
    )
}

/// Lock completion state, recovering from a poisoned mutex.
fn lock_inner<T>(inner: &Mutex<CompletionState<T>>) -> MutexGuard<'_, CompletionState<T>> {
    inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One registered request and its completion.
struct PendingScreenshot {
    request: ScreenshotRequest,
    completer: ScreenshotCompleter<ScreenshotResponse>,
}

/// The bookkeeping for every live screenshot request.
#[derive(Default)]
pub struct ScreenshotRegistry {
    pending: BTreeMap<String, PendingScreenshot>,
}

impl ScreenshotRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a request and hand the caller the future to await. A duplicate
    /// handle is refused so a caller cannot hijack another request.
    pub fn begin(
        &mut self,
        request: ScreenshotRequest,
    ) -> Result<ScreenshotCompletion<ScreenshotResponse>, ScreenshotError> {
        if self.pending.contains_key(&request.handle) {
            return Err(ScreenshotError::Duplicate(request.handle));
        }
        let (completer, completion) = completion();
        self.pending.insert(
            request.handle.clone(),
            PendingScreenshot { request, completer },
        );
        Ok(completion)
    }

    /// A live request.
    pub fn request(&self, handle: &str) -> Option<&ScreenshotRequest> {
        self.pending.get(handle).map(|pending| &pending.request)
    }

    /// The `(handle, mode)` pairs a presenter should offer to handle.
    pub fn handles(&self) -> Vec<(String, CaptureMode)> {
        self.pending
            .iter()
            .map(|(handle, pending)| (handle.clone(), pending.request.mode))
            .collect()
    }

    /// The number of live requests.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether no request is live.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Resolve a request with a presenter's captured URI. Returns the response
    /// delivered to the awaiting caller, or `None` for an unknown handle.
    pub fn complete(&mut self, handle: &str, uri: &str) -> Option<ScreenshotResponse> {
        let pending = self.pending.remove(handle)?;
        let response = pending.request.success(uri);
        pending.completer.complete(response.clone());
        Some(response)
    }

    /// Resolve a request as cancelled. Returns the response, or `None` for an
    /// unknown handle.
    pub fn cancel(&mut self, handle: &str) -> Option<ScreenshotResponse> {
        let pending = self.pending.remove(handle)?;
        let response = ScreenshotResponse::cancelled();
        pending.completer.complete(response.clone());
        Some(response)
    }
}

/// The shared, live screenshot registry: the D-Bus objects and the diagnostic
/// surface all read the same bookkeeping.
pub type SharedScreenshot = Arc<Mutex<ScreenshotRegistry>>;

/// A shared, empty registry.
pub fn registry() -> SharedScreenshot {
    Arc::new(Mutex::new(ScreenshotRegistry::new()))
}

/// Lock a registry, recovering from a poisoned mutex so a panicking D-Bus call
/// never takes the service down.
pub fn lock(registry: &SharedScreenshot) -> MutexGuard<'_, ScreenshotRegistry> {
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

/// A string option.
fn string_of(options: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    options
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| String::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    use zbus::zvariant::Str;

    fn text(value: &str) -> OwnedValue {
        OwnedValue::from(Str::from(value))
    }

    #[test]
    fn the_mode_option_selects_the_selection() {
        let mut options = HashMap::new();
        options.insert(MODE_OPTION.to_owned(), text("window"));
        assert_eq!(
            ScreenshotOptions::from_raw(&options).capture_mode(),
            CaptureMode::Window
        );

        // A missing mode is fullscreen unless interactive.
        let interactive = {
            let mut options = HashMap::new();
            options.insert("interactive".to_owned(), OwnedValue::from(true));
            ScreenshotOptions::from_raw(&options)
        };
        assert!(interactive.interactive);
        assert_eq!(interactive.capture_mode(), CaptureMode::Region);

        // `modal` defaults to true, per the spec.
        assert!(ScreenshotOptions::from_raw(&HashMap::new()).modal);
    }

    #[test]
    fn a_request_resolves_its_mode() {
        let mut options = HashMap::new();
        options.insert(MODE_OPTION.to_owned(), text("region"));
        let request = ScreenshotRequest::new("/req/1", "org.example.App", "", &options);
        assert_eq!(request.mode, CaptureMode::Region);
        assert_eq!(request.mode.as_str(), "region");
        assert!(request.options.modal);
    }

    #[test]
    fn a_successful_capture_normalizes_the_uri() {
        let request = ScreenshotRequest::new("/req/1", "app", "", &HashMap::new());
        let response = request.success("/tmp/shot one.png");
        assert_eq!(response.response, RESPONSE_SUCCESS);
        assert_eq!(
            response.uri().as_deref(),
            Some("file:///tmp/shot%20one.png")
        );

        // A foreign scheme is refused, never a silent success.
        assert_eq!(request.success("trash:///x").response, RESPONSE_OTHER);
        assert_eq!(request.success("").response, RESPONSE_OTHER);
    }

    #[test]
    fn the_registry_round_trips_one_request_to_completion() {
        let mut registry = ScreenshotRegistry::new();
        let request = ScreenshotRequest::new("/req/1", "app", "", &HashMap::new());
        let mut completion = registry.begin(request).expect("begin");
        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry.handles(),
            vec![("/req/1".to_owned(), CaptureMode::Fullscreen)]
        );

        let duplicate = ScreenshotRequest::new("/req/1", "app", "", &HashMap::new());
        assert_eq!(
            registry.begin(duplicate).err(),
            Some(ScreenshotError::Duplicate("/req/1".to_owned()))
        );

        let response = registry
            .complete("/req/1", "/tmp/one.png")
            .expect("complete");
        assert_eq!(response.response, RESPONSE_SUCCESS);
        assert!(registry.is_empty());
        assert!(matches!(poll_once(&mut completion), Poll::Ready(Some(_))));
        assert!(registry.complete("/req/1", "/tmp/x").is_none());
    }

    #[test]
    fn cancelling_resolves_the_waiter() {
        let mut registry = ScreenshotRegistry::new();
        let request = ScreenshotRequest::new("/req/9", "app", "", &HashMap::new());
        let mut completion = registry.begin(request).expect("begin");
        let response = registry.cancel("/req/9").expect("cancel");
        assert_eq!(response.response, RESPONSE_CANCELLED);
        assert!(matches!(
            poll_once(&mut completion),
            Poll::Ready(Some(ScreenshotResponse {
                response: RESPONSE_CANCELLED,
                ..
            }))
        ));
        assert!(registry.cancel("/req/9").is_none());
    }

    /// Poll a future once without an executor: used to assert a completion is
    /// already ready.
    fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        Pin::new(future).poll(&mut context)
    }
}
