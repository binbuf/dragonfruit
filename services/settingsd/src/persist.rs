// SPDX-License-Identifier: MIT
//! On-disk persistence for the desktop-settings model (T-08.1b).
//!
//! `settingsd` is the sole writer of `$XDG_CONFIG_HOME/dragonfruit/settings.json`
//! (falling back to `$HOME/.config/dragonfruit/settings.json`). The file is
//! JSON in the shape the shell's interim `DockSettings`/`DockPins` already
//! write, so that file is adopted without migration:
//!
//! ```json
//! {
//!   "schema": 1,
//!   "keys": {
//!     "dock.size": 0.5,
//!     "dock.autohide": false,
//!     "dock.pinned": ["a.desktop", "b.desktop"]
//!   }
//! }
//! ```
//!
//! * `schema` is the persisted-schema revision ([`SCHEMA_VERSION`]). A file
//!   without the field is revision 0 (pre-versioning) and is migrated at
//!   startup: the revision is stamped and every key the older file predates is
//!   filled with its default. A key added in a later revision carries its
//!   `since` revision, so the same mechanism upgrades it losslessly.
//! * `keys` maps each named key to a JSON value whose shape follows the key's
//!   declared type: bool → JSON bool, double/integer → JSON number, string →
//!   JSON string, string list → JSON array of strings.
//!
//! # Atomic writes
//!
//! A save is staged in a same-directory temporary file, flushed with
//! `sync_all`, then `rename(2)`d over the target — atomic on POSIX. A process
//! that dies mid-write leaves the previous file intact; the stale temporary
//! file is inert (loading ignores anything but `settings.json`).
//!
//! # Unknown entries
//!
//! Root fields other than `schema`/`keys` and key entries the schema does not
//! declare are preserved verbatim across a save, so a settingsd write never
//! clobbers state another interim writer left behind. A schema key always
//! wins over an unknown entry of the same name.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Number, Value as Json};

use crate::model::Settings;
use crate::schema::{self, KeyType, KEYS, SCHEMA_VERSION};
use crate::value::Value;

/// The directory under the XDG config home that settingsd owns.
pub const CONFIG_SUBDIR: &str = "dragonfruit";
/// The settings file name inside [`CONFIG_SUBDIR`].
pub const FILE_NAME: &str = "settings.json";
/// The JSON field naming the persisted-schema revision.
pub const SCHEMA_FIELD: &str = "schema";
/// The JSON field holding the named keys.
pub const KEYS_FIELD: &str = "keys";
/// The revision assumed for a file with no `schema` field (the interim shell
/// file before versioning, and any hand-written config).
pub const UNVERSIONED: u32 = 0;

/// Resolve `$XDG_CONFIG_HOME/dragonfruit/settings.json`, falling back to
/// `$HOME/.config/...`. `None` when neither variable is set.
pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))?;
    Some(base.join(CONFIG_SUBDIR).join(FILE_NAME))
}

/// Why a settings file could not be read.
#[derive(Debug)]
pub enum LoadError {
    /// The file exists but could not be read.
    Io(io::Error),
    /// The file is not a JSON object.
    Malformed(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(error) => write!(f, "{error}"),
            LoadError::Malformed(message) => write!(f, "malformed settings file: {message}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<io::Error> for LoadError {
    fn from(error: io::Error) -> Self {
        LoadError::Io(error)
    }
}

/// The result of reading the settings file: the live store, the writer that
/// can save it again, and the migration outcome.
#[derive(Debug)]
pub struct Loaded {
    /// The store, seeded with schema defaults and overlaid with the file.
    pub settings: Settings,
    /// The writer for this path, carrying the preserved unknown entries.
    pub persistence: Persistence,
    /// The revision found on disk ([`UNVERSIONED`] when there was no field).
    pub schema: u32,
    /// True when the file predated [`SCHEMA_VERSION`] and was upgraded.
    pub migrated: bool,
    /// True when a file existed and parsed.
    pub found: bool,
}

/// The unknown JSON preserved across a settingsd save.
#[derive(Debug, Clone, Default)]
struct Extras {
    root: Map<String, Json>,
    keys: Map<String, Json>,
}

/// Owns the path (and the unknown entries to preserve) so a changed store can
/// be written back atomically. Cheap to clone into a D-Bus object.
#[derive(Debug, Clone)]
pub struct Persistence {
    path: PathBuf,
    extras: Extras,
}

impl Persistence {
    /// A writer with no preserved entries, for a path with no readable file.
    pub fn new(path: PathBuf) -> Self {
        Persistence {
            path,
            extras: Extras::default(),
        }
    }

    /// The file this writer targets.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Serialize `settings` in the documented shape and replace the file
    /// atomically. Creates the parent directory when needed.
    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        let document = self.document(settings);
        let mut bytes = serde_json::to_vec_pretty(&document).map_err(io::Error::other)?;
        bytes.push(b'\n');
        atomic_write(&self.path, &bytes)
    }

    /// Build the full JSON document for `settings`: preserved unknown root
    /// fields, the current revision, and every schema key plus preserved
    /// unknown key entries.
    fn document(&self, settings: &Settings) -> Map<String, Json> {
        let mut root = self.extras.root.clone();
        root.insert(SCHEMA_FIELD.to_owned(), Json::from(SCHEMA_VERSION));
        let mut keys = self.extras.keys.clone();
        for spec in KEYS {
            let value = settings
                .get(spec.key)
                .expect("the store holds every schema key");
            keys.insert(spec.key.to_owned(), value_to_json(value));
        }
        root.insert(KEYS_FIELD.to_owned(), Json::Object(keys));
        root
    }
}

/// Read `path`. A missing file is not an error: it yields the schema defaults
/// with `found = false`.
pub fn load(path: &Path) -> Result<Loaded, LoadError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Loaded {
                settings: Settings::new(),
                persistence: Persistence::new(path.to_owned()),
                schema: UNVERSIONED,
                migrated: false,
                found: false,
            });
        }
        Err(error) => return Err(LoadError::Io(error)),
    };
    let document: Json =
        serde_json::from_slice(&bytes).map_err(|error| LoadError::Malformed(error.to_string()))?;
    let Json::Object(root) = document else {
        return Err(LoadError::Malformed("the root is not a JSON object".into()));
    };
    Ok(from_document(path, root))
}

/// Apply a parsed root object to the schema, migrating it to the current
/// revision.
fn from_document(path: &Path, mut root: Map<String, Json>) -> Loaded {
    let schema = root
        .get(SCHEMA_FIELD)
        .and_then(Json::as_u64)
        .map(|version| version as u32)
        .unwrap_or(UNVERSIONED);
    let migrated = schema < SCHEMA_VERSION;

    let mut raw_keys = match root.remove(KEYS_FIELD) {
        Some(Json::Object(keys)) => keys,
        _ => Map::new(),
    };

    // A migration fills keys the older revision predates with their default.
    if migrated {
        for spec in KEYS {
            raw_keys
                .entry(spec.key.to_owned())
                .or_insert_with(|| value_to_json(&spec.default.to_value()));
        }
    }

    let mut settings = Settings::new();
    let mut unknown = Map::new();
    for (key, json) in raw_keys {
        let Some(spec) = schema::spec(&key) else {
            unknown.insert(key, json);
            continue;
        };
        // A stored value that no longer validates falls back to the default
        // seeded above rather than rejecting the whole file.
        if let Some(value) = json_to_value(spec.kind, &json) {
            let _ = settings.set(spec.key, value);
        }
    }

    // Unknown root fields survive; `schema`/`keys` are regenerated on save.
    root.remove(SCHEMA_FIELD);
    let mut persistence = Persistence::new(path.to_owned());
    persistence.extras = Extras {
        root,
        keys: unknown,
    };

    Loaded {
        settings,
        persistence,
        schema,
        migrated,
        found: true,
    }
}

/// Decode a JSON value against a key's declared type. `None` when the JSON
/// shape does not match (wrong type, malformed list, non-finite number).
fn json_to_value(kind: KeyType, json: &Json) -> Option<Value> {
    match kind {
        KeyType::Bool => json.as_bool().map(Value::Bool),
        KeyType::Number => json.as_f64().map(Value::Number),
        KeyType::Integer => {
            if let Some(value) = json.as_i64() {
                Some(Value::Integer(value))
            } else {
                // Tolerate a whole number written as a JSON float.
                json.as_f64()
                    .filter(|value| value.fract() == 0.0)
                    .map(|value| Value::Integer(value as i64))
            }
        }
        KeyType::Text => json.as_str().map(|text| Value::Text(text.to_owned())),
        KeyType::TextList => json
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            })
            .map(Value::TextList),
    }
}

/// Encode a settings value as JSON in the documented shape.
fn value_to_json(value: &Value) -> Json {
    match value {
        Value::Bool(flag) => Json::Bool(*flag),
        Value::Number(number) => {
            let number = Number::from_f64(*number).expect("settings numbers are finite");
            Json::Number(number)
        }
        Value::Integer(integer) => Json::Number(Number::from(*integer)),
        Value::Text(text) => Json::String(text.clone()),
        Value::TextList(items) => Json::Array(
            items
                .iter()
                .map(|item| Json::String(item.clone()))
                .collect(),
        ),
    }
}

/// Write `bytes` to `path` atomically (staging, flush, rename).
fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    atomic_write_with_hook(path, bytes, || Ok(()))
}

/// The staged write, with a hook between `sync_all` and the rename. The hook
/// is the fault-injection point the atomicity test uses to model a process
/// dying (or a write failing) in the one window where the target could be
/// damaged; production always passes a no-op.
fn atomic_write_with_hook(
    path: &Path,
    bytes: &[u8],
    hook: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;

    let temporary = temporary_path(directory, path.file_name());
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;

    let staged = (|| -> io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        hook()
    })();
    drop(file);

    if let Err(error) = staged {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    // Best-effort: flush the directory entry so the rename survives a crash.
    if let Ok(handle) = fs::File::open(directory) {
        let _ = handle.sync_all();
    }
    Ok(())
}

/// A hidden, unique sibling of the target in the same directory.
fn temporary_path(directory: &Path, file_name: Option<&OsStr>) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nonce = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = file_name.and_then(OsStr::to_str).unwrap_or(FILE_NAME);
    directory.join(format!(".{name}.{}.{}.tmp", std::process::id(), nonce))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "dragonfruit-settingsd-{tag}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Scratch(path)
        }

        fn file(&self) -> PathBuf {
            self.0.join(CONFIG_SUBDIR).join(FILE_NAME)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_raw(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn a_missing_file_loads_schema_defaults_without_creating_one() {
        let scratch = Scratch::new("missing");
        let path = scratch.file();
        let loaded = load(&path).unwrap();
        assert!(!loaded.found);
        assert!(!loaded.migrated);
        assert!(!path.exists(), "loading must not create the file");
        for spec in KEYS {
            assert_eq!(
                loaded.settings.get(spec.key).unwrap(),
                &spec.default.to_value()
            );
        }
    }

    #[test]
    fn every_key_round_trips_through_save_and_load() {
        let scratch = Scratch::new("roundtrip");
        let path = scratch.file();
        let loaded = load(&path).unwrap();
        let mut settings = loaded.settings;
        settings.set("dock.size", Value::Number(0.8)).unwrap();
        settings.set("dock.autohide", Value::Bool(true)).unwrap();
        settings
            .set("dock.pinned", Value::TextList(vec!["a.desktop".into()]))
            .unwrap();
        settings
            .set("appearance.colorScheme", Value::Text("dark".into()))
            .unwrap();
        settings.set("workspaces.count", Value::Integer(5)).unwrap();
        settings
            .set("input.repeatDelay", Value::Integer(350))
            .unwrap();

        loaded.persistence.save(&settings).unwrap();

        let reloaded = load(&path).unwrap();
        assert!(reloaded.found);
        assert!(!reloaded.migrated);
        for spec in KEYS {
            assert_eq!(
                reloaded.settings.get(spec.key).unwrap(),
                settings.get(spec.key).unwrap(),
                "{} did not round-trip",
                spec.key
            );
        }
    }

    #[test]
    fn the_file_is_the_documented_schema_and_keys_shape() {
        let scratch = Scratch::new("shape");
        let path = scratch.file();
        let mut settings = Settings::new();
        settings.set("dock.autohide", Value::Bool(true)).unwrap();
        Persistence::new(path.clone()).save(&settings).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let json: Json = serde_json::from_str(&text).unwrap();
        assert_eq!(json[SCHEMA_FIELD], Json::from(SCHEMA_VERSION));
        assert_eq!(json[KEYS_FIELD]["dock.autohide"], Json::Bool(true));
        assert_eq!(json[KEYS_FIELD]["dock.size"], Json::from(0.5));
        assert_eq!(json[KEYS_FIELD]["dock.pinned"], Json::Array(vec![]));
        assert_eq!(json[KEYS_FIELD]["workspaces.count"], Json::from(3));
    }

    #[test]
    fn a_crash_between_staging_and_rename_leaves_the_prior_file_intact() {
        let scratch = Scratch::new("crash");
        let path = scratch.file();
        write_raw(
            &path,
            "{\n  \"schema\": 1,\n  \"keys\": { \"dock.size\": 0.25 }\n}\n",
        );
        let before = fs::read_to_string(&path).unwrap();

        let mut settings = Settings::new();
        settings.set("dock.size", Value::Number(0.9)).unwrap();
        let mut bytes =
            serde_json::to_vec_pretty(&Persistence::new(path.clone()).document(&settings)).unwrap();

        // Model the process dying after the temporary file is staged and
        // flushed but before the rename commits.
        let error =
            atomic_write_with_hook(&path, &bytes, || Err(io::Error::other("simulated crash")))
                .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        bytes.clear();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            before,
            "the prior file must be untouched"
        );
        assert!(
            load(&path).unwrap().settings.get("dock.size").unwrap() == &Value::Number(0.25),
            "a reload sees the prior value"
        );

        // No stale temporary file is left behind to confuse a reader.
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "staged file was not cleaned up");
    }

    #[test]
    fn a_stale_temporary_file_does_not_affect_a_load() {
        let scratch = Scratch::new("stale");
        let path = scratch.file();
        write_raw(
            &path,
            "{\n  \"schema\": 1,\n  \"keys\": { \"dock.size\": 0.75 }\n}\n",
        );
        write_raw(
            &scratch.file().with_file_name(".settings.json.999.tmp"),
            "{ this is not the settings file }",
        );
        let loaded = load(&path).unwrap();
        assert_eq!(
            loaded.settings.get("dock.size").unwrap(),
            &Value::Number(0.75)
        );
    }

    #[test]
    fn a_revision_zero_fixture_migrates_to_the_current_schema() {
        let scratch = Scratch::new("migrate");
        let path = scratch.file();
        // The interim shell file before the `schema` field existed: a subset
        // of the dock keys, plus one entry a later revision added.
        write_raw(
            &path,
            "{\n  \"keys\": {\n    \"dock.size\": 0.8,\n    \"dock.autohide\": true,\n    \"unreleased.key\": 7\n  },\n  \"future\": true\n}\n",
        );

        let loaded = load(&path).unwrap();
        assert!(loaded.found);
        assert_eq!(loaded.schema, UNVERSIONED);
        assert!(loaded.migrated, "a revision-zero file reports a migration");
        assert_eq!(
            loaded.settings.get("dock.size").unwrap(),
            &Value::Number(0.8)
        );
        assert_eq!(
            loaded.settings.get("dock.autohide").unwrap(),
            &Value::Bool(true)
        );
        // Keys the older file predates take their schema defaults.
        assert_eq!(
            loaded.settings.get("workspaces.count").unwrap(),
            &Value::Integer(3)
        );

        // The upgraded file is written back at the current revision, preserving
        // the unknown entries.
        loaded.persistence.save(&loaded.settings).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let json: Json = serde_json::from_str(&text).unwrap();
        assert_eq!(json[SCHEMA_FIELD], Json::from(SCHEMA_VERSION));
        assert_eq!(json["future"], Json::Bool(true));
        assert_eq!(json[KEYS_FIELD]["unreleased.key"], Json::from(7));
        assert_eq!(json[KEYS_FIELD]["dock.size"], Json::from(0.8));

        // Reloading the upgraded file is a no-op migration.
        let again = load(&path).unwrap();
        assert!(!again.migrated);
        assert_eq!(again.schema, SCHEMA_VERSION);
        assert_eq!(again.settings, loaded.settings);
    }

    #[test]
    fn unknown_root_and_key_entries_survive_a_settingsd_write() {
        let scratch = Scratch::new("unknown");
        let path = scratch.file();
        write_raw(
            &path,
            "{\n  \"schema\": 1,\n  \"future\": { \"a\": 1 },\n  \"keys\": {\n    \"other.owner\": \"keep me\",\n    \"dock.size\": 0.5\n  }\n}\n",
        );
        let loaded = load(&path).unwrap();
        let mut settings = loaded.settings;
        settings.set("dock.size", Value::Number(0.6)).unwrap();
        loaded.persistence.save(&settings).unwrap();

        let json: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(json["future"], serde_json::json!({ "a": 1 }));
        assert_eq!(
            json[KEYS_FIELD]["other.owner"],
            Json::String("keep me".into())
        );
        assert_eq!(json[KEYS_FIELD]["dock.size"], Json::from(0.6));
    }

    #[test]
    fn invalid_stored_values_fall_back_to_defaults_without_failing_the_load() {
        let scratch = Scratch::new("invalid");
        let path = scratch.file();
        write_raw(
            &path,
            "{\n  \"schema\": 1,\n  \"keys\": {\n    \"dock.size\": 9.0,\n    \"dock.autohide\": \"yes\",\n    \"dock.position\": \"sideways\",\n    \"workspaces.count\": 3.0,\n    \"dock.pinned\": [1, 2]\n  }\n}\n",
        );
        let loaded = load(&path).unwrap();
        let spec = schema::spec;
        assert_eq!(
            loaded.settings.get("dock.size").unwrap(),
            &spec("dock.size").unwrap().default.to_value()
        );
        assert_eq!(
            loaded.settings.get("dock.autohide").unwrap(),
            &Value::Bool(false)
        );
        assert_eq!(
            loaded.settings.get("dock.position").unwrap(),
            &Value::Text("bottom".into())
        );
        // A whole number written as a float is tolerated for an integer key.
        assert_eq!(
            loaded.settings.get("workspaces.count").unwrap(),
            &Value::Integer(3)
        );
        assert_eq!(
            loaded.settings.get("dock.pinned").unwrap(),
            &Value::TextList(vec![])
        );
    }

    #[test]
    fn malformed_json_is_a_typed_load_error() {
        let scratch = Scratch::new("malformed");
        let path = scratch.file();
        write_raw(&path, "{ not json");
        assert!(matches!(load(&path).unwrap_err(), LoadError::Malformed(_)));

        write_raw(&path, "[]");
        assert!(matches!(load(&path).unwrap_err(), LoadError::Malformed(_)));
    }

    #[test]
    fn default_path_prefers_xdg_config_home() {
        // The path helper is pure aside from the environment; assert its shape
        // without mutating the process environment (tests run in parallel).
        let Some(path) = default_path() else {
            return;
        };
        assert!(path.ends_with(Path::new(CONFIG_SUBDIR).join(FILE_NAME)));
    }
}
