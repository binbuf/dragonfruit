// SPDX-License-Identifier: MIT
//! The FileChooser portal's pure model (T-13.2a).
//!
//! The backend serves the standard
//! `org.freedesktop.impl.portal.FileChooser` interface so sandboxed
//! applications can ask the desktop for a file to open or save. The interface
//! method is *synchronous* from the frontend's point of view: it does not
//! return until a presenter has chosen, which is why [`ChooserCompletion`] is
//! a one-shot future the D-Bus method awaits.
//!
//! The browsing is files-core's ([09-files.md]): [`Location`] normalizes the
//! presenter's selection into the portal's canonical `file://` URIs (any URI
//! that cannot be normalized is discarded, as the portal spec requires) and
//! [`list_directory`] is the listing seam the picker UI drives over the same
//! streaming model Files uses. This module has no D-Bus dependency, so the
//! options/result logic and the request lifecycle are unit-tested directly.
//!
//! **The presenter is deferred to T-13.2b.** The dialog does not exist yet;
//! a presenter (today the test client, tomorrow the design-system picker)
//! observes a request and completes it through the diagnostic surface. The
//! registry is the one seam both use.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use dragonfruit_files_core::{DirectoryModel, Location, Node, StdFsSource};
use zbus::zvariant::{Array, OwnedValue, Str};

/// The standard FileChooser backend interface.
pub const FILE_CHOOSER_INTERFACE: &str = "org.freedesktop.impl.portal.FileChooser";

/// The FileChooser backend version. Version 3 is the first with the
/// `directory` option; the public interface documents version 4.
pub const FILE_CHOOSER_VERSION: u32 = 3;

/// Portal response codes shared with the caller.
pub const RESPONSE_SUCCESS: u32 = 0;
pub const RESPONSE_CANCELLED: u32 = 1;
pub const RESPONSE_OTHER: u32 = 2;

/// The three chooser entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChooserKind {
    /// Pick one or more existing files (or folders).
    OpenFile,
    /// Pick a location to save one file.
    SaveFile,
    /// Pick a folder to save several named files into.
    SaveFiles,
}

impl ChooserKind {
    /// The stable label exposed on the wire and the diagnostic surface.
    pub const fn as_str(self) -> &'static str {
        match self {
            ChooserKind::OpenFile => "open",
            ChooserKind::SaveFile => "save",
            ChooserKind::SaveFiles => "save-files",
        }
    }
}

/// Why a chooser request could not be registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChooserError {
    /// A request already exists at that handle.
    Duplicate(String),
    /// No request exists at that handle.
    Unknown(String),
    /// The location was not a URI and not a usable local path.
    InvalidLocation(String),
    /// The location used a scheme files-core cannot address locally.
    UnsupportedScheme(String),
    /// The listing failed.
    ListingFailed(String),
}

impl std::fmt::Display for ChooserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChooserError::Duplicate(handle) => write!(f, "chooser {handle} already exists"),
            ChooserError::Unknown(handle) => write!(f, "no chooser at {handle}"),
            ChooserError::InvalidLocation(uri) => write!(f, "invalid location {uri:?}"),
            ChooserError::UnsupportedScheme(uri) => {
                write!(f, "location {uri:?} is not a local file:// location")
            }
            ChooserError::ListingFailed(message) => write!(f, "listing failed: {message}"),
        }
    }
}

impl std::error::Error for ChooserError {}

/// The parsed `options` vardict. Only the keys the picker and the result
/// normalization need are decoded; `go` filters and choices are kept as their
/// raw wire values so the picker can render them and echo them back.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChooserOptions {
    /// `multiple` — allow a multi-selection.
    pub multiple: bool,
    /// `directory` — select folders instead of files (OpenFile only).
    pub directory: bool,
    /// `modal` — whether the dialog is modal (default true).
    pub modal: bool,
    /// `accept_label` — the accept button's label.
    pub accept_label: Option<String>,
    /// `current_name` — a suggested filename (SaveFile only).
    pub current_name: Option<String>,
    /// `current_folder` — a suggested folder.
    pub current_folder: Option<PathBuf>,
    /// `current_file` — the file being saved (SaveFile only).
    pub current_file: Option<PathBuf>,
    /// `files` — the names to save (SaveFiles only).
    pub files: Vec<PathBuf>,
}

impl ChooserOptions {
    /// Decode the subset of `options` the model understands. Unknown keys are
    /// ignored; a key of the wrong type is treated as absent.
    pub fn from_raw(options: &HashMap<String, OwnedValue>) -> Self {
        ChooserOptions {
            multiple: bool_of(options, "multiple").unwrap_or(false),
            directory: bool_of(options, "directory").unwrap_or(false),
            modal: bool_of(options, "modal").unwrap_or(true),
            accept_label: string_of(options, "accept_label"),
            current_name: string_of(options, "current_name"),
            current_folder: path_of(options, "current_folder"),
            current_file: path_of(options, "current_file"),
            files: paths_of(options, "files"),
        }
    }

    /// The location a presenter should open at, derived from
    /// `current_folder`/`current_file` through files-core so the picker and
    /// the result share one URI spelling. `None` when the request carries no
    /// hint.
    pub fn suggested_location(&self) -> Option<Location> {
        self.current_file
            .as_deref()
            .and_then(|path| path.parent())
            .or(self.current_folder.as_deref())
            .map(Location::file)
    }
}

/// One live chooser request: the identity the frontend passed and the parsed
/// options. The presenter reads it; the completer resolves it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChooserRequest {
    /// The Request object path the frontend supplied.
    pub handle: String,
    /// Which entry point created the request.
    pub kind: ChooserKind,
    /// The calling application's id.
    pub app_id: String,
    /// The parent window identifier (may be empty).
    pub parent_window: String,
    /// The dialog title (may be empty).
    pub title: String,
    /// The decoded options.
    pub options: ChooserOptions,
    /// The raw `options` vardict, kept so a presenter sees the filters and
    /// choices verbatim and the diagnostic signal carries them unchanged.
    pub raw_options: HashMap<String, OwnedValue>,
}

impl ChooserRequest {
    /// Build a request from a D-Bus call.
    pub fn new(
        kind: ChooserKind,
        handle: impl Into<String>,
        app_id: impl Into<String>,
        parent_window: impl Into<String>,
        title: impl Into<String>,
        options: &HashMap<String, OwnedValue>,
    ) -> Self {
        ChooserRequest {
            handle: handle.into(),
            kind,
            app_id: app_id.into(),
            parent_window: parent_window.into(),
            title: title.into(),
            options: ChooserOptions::from_raw(options),
            raw_options: options.clone(),
        }
    }

    /// The success response for a presenter's selection: every selection is
    /// normalized to a canonical `file://` URI and anything that cannot be
    /// normalized is discarded. A selection that normalizes to nothing is an
    /// error response, never a silent success.
    pub fn success(&self, selections: &[String]) -> ChooserResponse {
        let uris: Vec<String> = selections
            .iter()
            .filter(|selection| !selection.is_empty())
            .filter_map(|selection| normalize_uri(selection))
            .collect();
        if uris.is_empty() {
            return ChooserResponse::other();
        }
        let mut results: HashMap<String, OwnedValue> = HashMap::new();
        if let Some(value) = string_array(&uris) {
            results.insert("uris".to_owned(), value);
        }
        if matches!(self.kind, ChooserKind::OpenFile) {
            results.insert("writable".to_owned(), OwnedValue::from(false));
        }
        ChooserResponse {
            response: RESPONSE_SUCCESS,
            results,
        }
    }
}

/// The portal's response plus results vardict.
#[derive(Debug, Clone, PartialEq)]
pub struct ChooserResponse {
    /// 0 success, 1 cancelled, 2 other error.
    pub response: u32,
    /// The results vardict.
    pub results: HashMap<String, OwnedValue>,
}

impl ChooserResponse {
    /// A successful response with the given results.
    pub fn success(results: HashMap<String, OwnedValue>) -> Self {
        ChooserResponse {
            response: RESPONSE_SUCCESS,
            results,
        }
    }

    /// The user cancelled.
    pub fn cancelled() -> Self {
        ChooserResponse {
            response: RESPONSE_CANCELLED,
            results: HashMap::new(),
        }
    }

    /// Any other failure.
    pub fn other() -> Self {
        ChooserResponse {
            response: RESPONSE_OTHER,
            results: HashMap::new(),
        }
    }

    /// The selected `file://` URIs, when the response succeeded.
    pub fn uris(&self) -> Vec<String> {
        self.results
            .get("uris")
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| Vec::<String>::try_from(value).ok())
            .unwrap_or_default()
    }
}

/// Normalize one presenter selection into a canonical `file://` URI.
///
/// A value carrying a scheme is parsed by files-core and must be `file://`;
/// anything else (a `trash://` item, a bare name) is discarded. A value with
/// no scheme is treated as a local path and encoded by files-core. This is the
/// portal's "backends must normalize URIs ... into file:// URIs" rule.
pub fn normalize_uri(selection: &str) -> Option<String> {
    if selection.is_empty() {
        return None;
    }
    match Location::parse(selection) {
        Ok(location) => {
            let path = location.to_file_path()?;
            Some(Location::file(path).uri().to_owned())
        }
        Err(_) => Some(Location::file(selection).uri().to_owned()),
    }
}

/// One row the picker can render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooserEntry {
    /// The display name (lossy for non-UTF-8 names).
    pub name: String,
    /// The item's canonical URI.
    pub uri: String,
    /// Whether the item is a directory.
    pub directory: bool,
    /// The byte size, when known.
    pub size: Option<u64>,
}

/// A directory listing produced by the files-core model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryListing {
    /// The listed location's URI.
    pub location: String,
    /// The sorted entries.
    pub entries: Vec<ChooserEntry>,
}

/// List one local directory through files-core's streaming model and return
/// its sorted entries. The call blocks until the listing completes; the picker
/// (T-13.2b) drives it for a request's suggested folder.
pub fn list_directory(uri: &str) -> Result<DirectoryListing, ChooserError> {
    if uri.is_empty() {
        return Err(ChooserError::InvalidLocation(uri.to_owned()));
    }
    let location =
        Location::parse(uri).map_err(|_| ChooserError::InvalidLocation(uri.to_owned()))?;
    if !location.is_file() {
        return Err(ChooserError::UnsupportedScheme(uri.to_owned()));
    }

    let mut model = DirectoryModel::new();
    let handle = model.begin(Arc::new(StdFsSource::new()), location.clone());
    while let Some(event) = handle.recv() {
        if !model.apply(event) {
            break;
        }
        if model.is_complete() {
            break;
        }
    }
    if let Some(error) = model.error() {
        return Err(ChooserError::ListingFailed(error.to_string()));
    }

    let entries = model.ordered().map(entry_of).collect();
    Ok(DirectoryListing {
        location: location.uri().to_owned(),
        entries,
    })
}

/// Project one files-core [`Node`] onto the picker's row shape.
fn entry_of(node: &Node) -> ChooserEntry {
    ChooserEntry {
        name: node.display_name().into_owned(),
        uri: node.uri().to_owned(),
        directory: node.is_dir(),
        size: node.size(),
    }
}

/// The waiter half of a one-shot completion. The D-Bus method awaits it; the
/// presenter's completion wakes it.
pub struct ChooserCompletion<T> {
    inner: Arc<Mutex<CompletionState<T>>>,
}

struct CompletionState<T> {
    value: Option<T>,
    waker: Option<Waker>,
    closed: bool,
}

impl<T> Future for ChooserCompletion<T> {
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
pub struct ChooserCompleter<T> {
    inner: Arc<Mutex<CompletionState<T>>>,
}

impl<T> ChooserCompleter<T> {
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
pub fn completion<T>() -> (ChooserCompleter<T>, ChooserCompletion<T>) {
    let inner = Arc::new(Mutex::new(CompletionState {
        value: None,
        waker: None,
        closed: false,
    }));
    (
        ChooserCompleter {
            inner: inner.clone(),
        },
        ChooserCompletion { inner },
    )
}

/// Lock completion state, recovering from a poisoned mutex.
fn lock_inner<T>(inner: &Mutex<CompletionState<T>>) -> MutexGuard<'_, CompletionState<T>> {
    inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One registered request and its completion.
struct PendingChooser {
    request: ChooserRequest,
    completer: ChooserCompleter<ChooserResponse>,
}

/// The bookkeeping for every live chooser request.
#[derive(Default)]
pub struct ChooserRegistry {
    pending: BTreeMap<String, PendingChooser>,
}

impl ChooserRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a request and hand the caller the future to await. A duplicate
    /// handle is refused so a caller cannot hijack another request.
    pub fn begin(
        &mut self,
        request: ChooserRequest,
    ) -> Result<ChooserCompletion<ChooserResponse>, ChooserError> {
        if self.pending.contains_key(&request.handle) {
            return Err(ChooserError::Duplicate(request.handle));
        }
        let (completer, completion) = completion();
        self.pending.insert(
            request.handle.clone(),
            PendingChooser { request, completer },
        );
        Ok(completion)
    }

    /// A live request.
    pub fn request(&self, handle: &str) -> Option<&ChooserRequest> {
        self.pending.get(handle).map(|pending| &pending.request)
    }

    /// The `(handle, kind)` pairs a presenter should offer to handle.
    pub fn handles(&self) -> Vec<(String, ChooserKind)> {
        self.pending
            .iter()
            .map(|(handle, pending)| (handle.clone(), pending.request.kind))
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

    /// Resolve a request with a presenter's selection. Returns the response
    /// delivered to the awaiting caller, or `None` for an unknown handle.
    pub fn complete(&mut self, handle: &str, selections: &[String]) -> Option<ChooserResponse> {
        let pending = self.pending.remove(handle)?;
        let response = pending.request.success(selections);
        pending.completer.complete(response.clone());
        Some(response)
    }

    /// Resolve a request as cancelled. Returns the response, or `None` for an
    /// unknown handle.
    pub fn cancel(&mut self, handle: &str) -> Option<ChooserResponse> {
        let pending = self.pending.remove(handle)?;
        let response = ChooserResponse::cancelled();
        pending.completer.complete(response.clone());
        Some(response)
    }
}

/// The shared, live chooser registry: the D-Bus objects and the diagnostic
/// surface all read the same bookkeeping.
pub type SharedChooser = Arc<Mutex<ChooserRegistry>>;

/// A shared, empty registry.
pub fn registry() -> SharedChooser {
    Arc::new(Mutex::new(ChooserRegistry::new()))
}

/// Lock a registry, recovering from a poisoned mutex so a panicking D-Bus call
/// never takes the service down.
pub fn lock(registry: &SharedChooser) -> MutexGuard<'_, ChooserRegistry> {
    registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Build an `as` value from strings.
fn string_array(values: &[String]) -> Option<OwnedValue> {
    let strings: Vec<Str<'_>> = values
        .iter()
        .map(|value| Str::from(value.as_str()))
        .collect();
    OwnedValue::try_from(Array::from(strings)).ok()
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

/// A path option stored as a null-terminated `ay`.
fn path_of(options: &HashMap<String, OwnedValue>, key: &str) -> Option<PathBuf> {
    let bytes = options
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| Vec::<u8>::try_from(value).ok())?;
    Some(decode_path_bytes(&bytes))
}

/// A list of path options stored as a null-terminated `aay`.
fn paths_of(options: &HashMap<String, OwnedValue>, key: &str) -> Vec<PathBuf> {
    options
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| Vec::<Vec<u8>>::try_from(value).ok())
        .map(|lists| lists.iter().map(|bytes| decode_path_bytes(bytes)).collect())
        .unwrap_or_default()
}

/// Decode filesystem bytes (with the spec's trailing nul) into a path.
fn decode_path_bytes(bytes: &[u8]) -> PathBuf {
    let trimmed = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(OsString::from_vec(trimmed.to_vec()))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(trimmed).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::{Type, Value};

    fn text(value: &str) -> OwnedValue {
        OwnedValue::from(Str::from(value))
    }

    fn null_terminated(entry: &str) -> Vec<u8> {
        let mut bytes = entry.as_bytes().to_vec();
        bytes.push(0);
        bytes
    }

    fn ay(entry: &str) -> OwnedValue {
        OwnedValue::try_from(Array::from(null_terminated(entry))).expect("ay")
    }

    fn paths(entries: &[&str]) -> OwnedValue {
        let mut outer = Array::new(<Vec<u8> as Type>::SIGNATURE);
        for entry in entries {
            outer
                .append(Value::Array(Array::from(null_terminated(entry))))
                .expect("append an ay");
        }
        OwnedValue::try_from(outer).expect("aay")
    }

    fn rules() -> HashMap<String, OwnedValue> {
        let mut options = HashMap::new();
        options.insert("multiple".to_owned(), OwnedValue::from(true));
        options.insert("directory".to_owned(), OwnedValue::from(true));
        options.insert("modal".to_owned(), OwnedValue::from(false));
        options.insert("accept_label".to_owned(), text("Choose"));
        options.insert("current_name".to_owned(), text("report.txt"));
        options
    }

    #[test]
    fn options_decode_the_keys_the_picker_needs() {
        let options = ChooserOptions::from_raw(&rules());
        assert!(options.multiple);
        assert!(options.directory);
        assert!(!options.modal);
        assert_eq!(options.accept_label.as_deref(), Some("Choose"));
        assert_eq!(options.current_name.as_deref(), Some("report.txt"));
        // A missing `modal` defaults to true, per the spec.
        let mut empty = HashMap::new();
        empty.insert("multiple".to_owned(), OwnedValue::from(false));
        assert!(ChooserOptions::from_raw(&empty).modal);
    }

    #[test]
    fn a_null_terminated_byte_array_decodes_to_a_path() {
        let mut options = HashMap::new();
        options.insert("current_folder".to_owned(), ay("/tmp/docs"));
        let decoded = ChooserOptions::from_raw(&options);
        assert_eq!(decoded.current_folder, Some(PathBuf::from("/tmp/docs")));
        assert_eq!(
            decoded.suggested_location().map(|l| l.uri().to_owned()),
            Some("file:///tmp/docs".to_owned())
        );
    }

    #[test]
    fn a_save_files_list_decodes_to_paths() {
        let mut options = HashMap::new();
        options.insert("files".to_owned(), paths(&["a.txt", "b.txt"]));
        let decoded = ChooserOptions::from_raw(&options);
        assert_eq!(
            decoded.files,
            vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")]
        );
    }

    #[test]
    fn selections_normalize_to_canonical_file_uris_and_foreign_uris_are_discarded() {
        assert_eq!(
            normalize_uri("file:///tmp/a%20b").as_deref(),
            Some("file:///tmp/a%20b")
        );
        assert_eq!(
            normalize_uri("/tmp/a b").as_deref(),
            Some("file:///tmp/a%20b")
        );
        assert_eq!(normalize_uri("trash:///x"), None);
        assert_eq!(normalize_uri(""), None);
    }

    #[test]
    fn a_successful_request_builds_the_uris_result() {
        let request = ChooserRequest::new(
            ChooserKind::OpenFile,
            "/req/1",
            "org.example.App",
            "",
            "Open",
            &HashMap::new(),
        );
        let response = request.success(&["file:///tmp/one".to_owned(), "/tmp/two".to_owned()]);
        assert_eq!(response.response, RESPONSE_SUCCESS);
        assert_eq!(
            response.uris(),
            vec!["file:///tmp/one".to_owned(), "file:///tmp/two".to_owned()]
        );
        assert_eq!(
            response.results.get("writable"),
            Some(&OwnedValue::from(false))
        );
    }

    #[test]
    fn a_selection_that_normalizes_to_nothing_is_an_error() {
        let request = ChooserRequest::new(
            ChooserKind::SaveFile,
            "/req/2",
            "app",
            "",
            "",
            &HashMap::new(),
        );
        assert_eq!(
            request.success(&["trash:///x".to_owned()]).response,
            RESPONSE_OTHER
        );
        assert_eq!(request.success(&[]).response, RESPONSE_OTHER);
    }

    #[test]
    fn the_registry_round_trips_one_request_to_completion() {
        let mut registry = ChooserRegistry::new();
        let request = ChooserRequest::new(
            ChooserKind::OpenFile,
            "/req/1",
            "app",
            "",
            "Open",
            &HashMap::new(),
        );
        let mut completion = registry.begin(request).expect("begin");
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.handles().len(), 1);

        // A duplicate handle is refused.
        let duplicate = ChooserRequest::new(
            ChooserKind::SaveFile,
            "/req/1",
            "app",
            "",
            "",
            &HashMap::new(),
        );
        assert_eq!(
            registry.begin(duplicate).err(),
            Some(ChooserError::Duplicate("/req/1".to_owned()))
        );

        let response = registry
            .complete("/req/1", &["/tmp/one".to_owned()])
            .expect("complete");
        assert_eq!(response.response, RESPONSE_SUCCESS);
        assert!(registry.is_empty());

        // The completion is ready without parking: poll it once.
        assert!(matches!(poll_once(&mut completion), Poll::Ready(Some(_))));
        // A second completion is refused.
        assert!(registry.complete("/req/1", &[]).is_none());
    }

    #[test]
    fn cancelling_resolves_the_waiter() {
        let mut registry = ChooserRegistry::new();
        let request = ChooserRequest::new(
            ChooserKind::SaveFile,
            "/req/9",
            "app",
            "",
            "",
            &HashMap::new(),
        );
        let mut completion = registry.begin(request).expect("begin");
        let response = registry.cancel("/req/9").expect("cancel");
        assert_eq!(response.response, RESPONSE_CANCELLED);
        assert!(matches!(
            poll_once(&mut completion),
            Poll::Ready(Some(ChooserResponse {
                response: RESPONSE_CANCELLED,
                ..
            }))
        ));
        assert!(registry.cancel("/req/9").is_none());
    }

    #[test]
    fn listing_a_real_directory_uses_files_core() {
        let dir = std::env::temp_dir().join(format!("df-chooser-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("zeta.txt"), b"x").expect("file");
        std::fs::create_dir(dir.join("alpha")).expect("subdir");

        let uri = Location::file(&dir).uri().to_owned();
        let listing = list_directory(&uri).expect("listing");
        let names: Vec<&str> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        // Files-core's natural collation is folders-first, then name.
        assert_eq!(names, vec!["alpha", "zeta.txt"]);
        assert!(listing.entries[0].directory);
        assert_eq!(listing.entries[1].size, Some(1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn listing_refuses_a_foreign_scheme() {
        assert_eq!(
            list_directory("trash:///").err(),
            Some(ChooserError::UnsupportedScheme("trash:///".to_owned()))
        );
    }

    /// Poll a future once without an executor: used to assert a completion is
    /// already ready.
    fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        Pin::new(future).poll(&mut context)
    }
}
