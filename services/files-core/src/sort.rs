// SPDX-License-Identifier: MIT
//! Sort keys and the natural name collation ([09-files.md]).
//!
//! A [`SortSpec`] is one immutable ordering: the primary [`SortKey`], a
//! [`SortDirection`], and the Finder "folders first" toggle. The model holds
//! one spec and can change it without touching node ids or the arrival order,
//! so a view re-sorts without invalidating selection.
//!
//! **Name collation is natural and numeric** (`file2` before `file10`),
//! case-insensitive with a case-sensitive tie-break. The design's
//! "locale-aware collation" (and its cross-locale test matrix) is a later
//! concern; the order here is deterministic and dependency-free, which is
//! what the streaming model needs. Raw bytes are not lost: comparison is over
//! the lossy display string, but nodes keep their original `OsString`.
//!
//! [`SortSpec::compare`] returns [`Ordering::Equal`] for genuinely equal
//! keys. That is deliberate: the model merges batches with a stable merge, so
//! equal keys stay in arrival order and a re-sort never shuffles them.
//!
//! [09-files.md]: ../../../docs/design/09-files.md

use std::cmp::Ordering;

use crate::{Node, NodeKind};

/// The field a listing is primarily ordered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortKey {
    /// Display name, natural/numeric order.
    Name,
    /// Item kind (`Directory` < `File` < `Symlink` < `Other`).
    Kind,
    /// Byte size; items with no size sort last ascending.
    Size,
    /// Modification time; items with no timestamp sort last ascending.
    Modified,
}

impl SortKey {
    /// Every sort key, for a UI's "Sort By" menu.
    pub const ALL: [SortKey; 4] = [
        SortKey::Name,
        SortKey::Kind,
        SortKey::Size,
        SortKey::Modified,
    ];

    /// The stable string a bridge or persistence layer stores.
    pub const fn as_str(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Kind => "kind",
            SortKey::Size => "size",
            SortKey::Modified => "modified",
        }
    }

    /// Parse [`SortKey::as_str`], accepting the older "date" alias.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "name" => Some(SortKey::Name),
            "kind" => Some(SortKey::Kind),
            "size" => Some(SortKey::Size),
            "modified" | "date" => Some(SortKey::Modified),
            _ => None,
        }
    }
}

/// Which way a [`SortKey`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SortDirection {
    /// Smallest / A→Z first. The default.
    #[default]
    Ascending,
    /// Largest / Z→A first.
    Descending,
}

impl SortDirection {
    /// The stable string a bridge or persistence layer stores.
    pub const fn as_str(self) -> &'static str {
        match self {
            SortDirection::Ascending => "ascending",
            SortDirection::Descending => "descending",
        }
    }

    /// Parse [`SortDirection::as_str`].
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ascending" | "asc" => Some(SortDirection::Ascending),
            "descending" | "desc" => Some(SortDirection::Descending),
            _ => None,
        }
    }
}

/// One complete ordering over a listing.
///
/// `folders_first` is applied before the primary key and is **not** reversed
/// by [`SortDirection::Descending`]: directories stay on top, matching the
/// Finder/Nautilus convention and the design's "folders-first toggle".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SortSpec {
    /// The primary field.
    pub key: SortKey,
    /// Ascending or descending.
    pub direction: SortDirection,
    /// Whether directories are grouped ahead of everything else.
    pub folders_first: bool,
}

impl Default for SortSpec {
    /// Name, ascending, folders first — the Finder default.
    fn default() -> Self {
        Self {
            key: SortKey::Name,
            direction: SortDirection::Ascending,
            folders_first: true,
        }
    }
}

impl SortSpec {
    /// A spec on `key` with the default direction and folders-first.
    pub fn new(key: SortKey) -> Self {
        Self {
            key,
            ..Self::default()
        }
    }

    /// Builder: set the direction.
    pub fn with_direction(mut self, direction: SortDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Builder: set the folders-first toggle.
    pub fn with_folders_first(mut self, folders_first: bool) -> Self {
        self.folders_first = folders_first;
        self
    }

    /// Order two nodes under this spec.
    ///
    /// [`Ordering::Equal`] means "same position": the model's stable merge
    /// then keeps the earlier-arriving node first.
    pub fn compare(&self, a: &Node, b: &Node) -> Ordering {
        if self.folders_first {
            match (a.is_dir(), b.is_dir()) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }
        let ordering = match self.key {
            SortKey::Name => natural_cmp(&a.display_name(), &b.display_name()),
            SortKey::Kind => kind_rank(a.kind()).cmp(&kind_rank(b.kind())),
            SortKey::Size => option_cmp(a.size(), b.size()),
            SortKey::Modified => option_cmp(a.modified(), b.modified()),
        };
        match self.direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    }
}

/// Directories first, then regular files, then symlinks, then anything else.
fn kind_rank(kind: NodeKind) -> u8 {
    match kind {
        NodeKind::Directory => 0,
        NodeKind::File => 1,
        NodeKind::Symlink => 2,
        NodeKind::Other => 3,
    }
}

/// `Some` sorts before `None` ascending, so unknown sizes/dates sink to the
/// bottom in the default direction.
fn option_cmp<T: Ord>(a: Option<T>, b: Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Natural, numeric, case-insensitive comparison of two names.
///
/// Digit runs compare by numeric value (`file2` < `file10`), so listing
/// `file1, file2, file10` is not `file1, file10, file2`. Non-digit characters
/// compare case-insensitively first, then by exact `char` so the order stays
/// total and deterministic.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut left = a.chars().peekable();
    let mut right = b.chars().peekable();
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a_ch), Some(b_ch)) => {
                if a_ch.is_ascii_digit() && b_ch.is_ascii_digit() {
                    let a_run = take_digits(&mut left);
                    let b_run = take_digits(&mut right);
                    match digit_run_cmp(&a_run, &b_run) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                } else {
                    left.next();
                    right.next();
                    let folded = fold(a_ch).cmp(&fold(b_ch));
                    if folded != Ordering::Equal {
                        return folded;
                    }
                    let exact = a_ch.cmp(&b_ch);
                    if exact != Ordering::Equal {
                        return exact;
                    }
                }
            }
        }
    }
}

fn take_digits(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut run = String::new();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_ascii_digit() {
            run.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    run
}

/// Compare two ASCII digit runs by value, ignoring leading zeros. Equal
/// numeric values with different widths (e.g. `01` vs `1`) fall back to the
/// shorter run first, so the comparison is total.
fn digit_run_cmp(a: &str, b: &str) -> Ordering {
    let a_trimmed = a.trim_start_matches('0');
    let b_trimmed = b.trim_start_matches('0');
    let by_len = a_trimmed.len().cmp(&b_trimmed.len());
    if by_len != Ordering::Equal {
        return by_len;
    }
    let by_value = a_trimmed.cmp(b_trimmed);
    if by_value != Ordering::Equal {
        return by_value;
    }
    a.len().cmp(&b.len())
}

fn fold(ch: char) -> char {
    ch.to_lowercase().next().unwrap_or(ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeKind;

    fn file(name: &str) -> Node {
        Node::new(name, format!("file:///fixture/{name}"), NodeKind::File)
    }

    fn dir(name: &str) -> Node {
        Node::new(name, format!("file:///fixture/{name}"), NodeKind::Directory)
    }

    #[test]
    fn names_sort_naturally() {
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("file10", "file2"), Ordering::Greater);
        assert_eq!(natural_cmp("file2", "file2"), Ordering::Equal);
        assert_eq!(natural_cmp("a", "b"), Ordering::Less);
        assert_eq!(natural_cmp("", "a"), Ordering::Less);
        assert_eq!(natural_cmp("", ""), Ordering::Equal);
    }

    #[test]
    fn names_sort_case_insensitively_then_by_exact_char() {
        assert_eq!(natural_cmp("apple", "Banana"), Ordering::Less);
        assert_eq!(natural_cmp("Apple", "apple"), Ordering::Less);
        assert_eq!(natural_cmp("apple", "Apple"), Ordering::Greater);
    }

    #[test]
    fn leading_zeros_do_not_change_numeric_value() {
        assert_eq!(natural_cmp("file1", "file01"), Ordering::Less);
        assert_eq!(natural_cmp("file007", "file7"), Ordering::Greater);
    }

    #[test]
    fn folders_first_is_not_reversed_by_descending() {
        let spec = SortSpec::new(SortKey::Name).with_direction(SortDirection::Descending);
        assert_eq!(spec.compare(&dir("zebra"), &file("apple")), Ordering::Less);
        assert_eq!(
            spec.compare(&file("zebra"), &dir("apple")),
            Ordering::Greater
        );
    }

    #[test]
    fn folder_toggle_can_be_turned_off() {
        let spec = SortSpec::new(SortKey::Name).with_folders_first(false);
        assert_eq!(
            spec.compare(&dir("zebra"), &file("apple")),
            Ordering::Greater
        );
    }

    #[test]
    fn kind_orders_directory_file_symlink_other() {
        let spec = SortSpec::new(SortKey::Kind);
        let symlink = Node::new("link", "file:///fixture/link", NodeKind::Symlink);
        let other = Node::new("sock", "file:///fixture/sock", NodeKind::Other);
        assert_eq!(spec.compare(&dir("a"), &file("z")), Ordering::Less);
        assert_eq!(spec.compare(&file("a"), &symlink), Ordering::Less);
        assert_eq!(spec.compare(&symlink, &other), Ordering::Less);
    }

    #[test]
    fn size_sorts_numbers_and_sinks_unknowns() {
        let spec = SortSpec::new(SortKey::Size).with_folders_first(false);
        let small = file("small").with_size(Some(1));
        let large = file("large").with_size(Some(10));
        let unknown = file("unknown");
        assert_eq!(spec.compare(&small, &large), Ordering::Less);
        assert_eq!(spec.compare(&small, &unknown), Ordering::Less);
        assert_eq!(spec.compare(&unknown, &small), Ordering::Greater);
    }

    #[test]
    fn modified_sorts_times_and_sinks_unknowns() {
        use std::time::SystemTime;
        let spec = SortSpec::new(SortKey::Modified).with_folders_first(false);
        let early = file("early").with_modified(Some(SystemTime::UNIX_EPOCH));
        let late = file("late").with_modified(Some(
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10),
        ));
        let unknown = file("unknown");
        assert_eq!(spec.compare(&early, &late), Ordering::Less);
        assert_eq!(spec.compare(&late, &unknown), Ordering::Less);
    }

    #[test]
    fn keys_and_directions_round_trip_through_strings() {
        for key in SortKey::ALL {
            assert_eq!(SortKey::parse(key.as_str()), Some(key));
        }
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            assert_eq!(SortDirection::parse(direction.as_str()), Some(direction));
        }
        assert_eq!(SortKey::parse("date"), Some(SortKey::Modified));
        assert_eq!(SortKey::parse("nope"), None);
        assert_eq!(SortDirection::parse("nope"), None);
    }
}
