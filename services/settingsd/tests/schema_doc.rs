// SPDX-License-Identifier: MIT
//! T-08.3 acceptance: the in-repo key schema for humans.
//!
//! `docs/settings-keys.md` is the human-facing table the Settings app and
//! later consumers read. It is only useful if it cannot silently drift from
//! the authoritative `schema::KEYS`, so this test parses the markdown table
//! and checks every key's name, D-Bus type, default, and **owner/consumer**
//! against the code. Adding a key to `schema.rs` without documenting it (or
//! renaming an owner in one place only) fails here.

use std::collections::BTreeMap;
use std::path::PathBuf;

use dragonfruit_settingsd::schema::{self, KeyDefault, KEYS};

/// The markdown table row for one key: `key -> [cells]`, key without
/// backticks.
fn documented_keys() -> BTreeMap<String, Vec<String>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/settings-keys.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

    let mut rows = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_owned())
            .collect();
        let Some(key) = cells.first() else { continue };
        let Some(key) = key.strip_prefix('`').and_then(|k| k.strip_suffix('`')) else {
            continue;
        };
        // Only rows whose first cell names a schema-shaped key belong to the
        // key table (the consumer map's first cells are component names).
        if schema::spec(key).is_some() {
            rows.insert(key.to_owned(), cells);
        }
    }
    rows
}

/// Render a declared default the way the doc does (unquoted, lists inline).
fn default_text(default: KeyDefault) -> String {
    match default {
        KeyDefault::Bool(v) => v.to_string(),
        KeyDefault::Number(v) => {
            if v.fract() == 0.0 {
                format!("{v:.1}")
            } else {
                v.to_string()
            }
        }
        KeyDefault::Integer(v) => v.to_string(),
        KeyDefault::Text("") => "`` (empty)".to_owned(),
        KeyDefault::Text(v) => format!("`{v}`"),
        KeyDefault::List(items) => format!("`[{}]`", items.join(", ")),
    }
}

#[test]
fn every_schema_key_is_documented_with_its_owner_and_consumer() {
    let rows = documented_keys();
    assert_eq!(
        rows.len(),
        KEYS.len(),
        "docs/settings-keys.md documents {} keys but the schema declares {}",
        rows.len(),
        KEYS.len()
    );

    for spec in KEYS {
        let cells = rows
            .get(spec.key)
            .unwrap_or_else(|| panic!("{} is missing from docs/settings-keys.md", spec.key));
        assert!(
            cells.len() >= 6,
            "{}: the documented row has too few columns: {cells:?}",
            spec.key
        );
        assert_eq!(cells[1], spec.kind.signature(), "{}: type", spec.key);
        assert_eq!(
            cells[2],
            default_text(spec.default),
            "{}: default",
            spec.key
        );
        assert_eq!(cells[4], spec.owner, "{}: owner", spec.key);
        assert_eq!(cells[5], spec.consumer, "{}: consumer", spec.key);
    }
}
