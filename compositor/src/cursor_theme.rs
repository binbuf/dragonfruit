// SPDX-License-Identifier: MIT
//! Themed system cursors (T-03 cursor rail).
//!
//! The DRM backend owns the pointer image: clients either set a cursor
//! surface (`wl_pointer.set_cursor`) or ask for a named shape through
//! `wp_cursor_shape_v1`. For a named shape the compositor draws the desktop
//! theme, not a process- or toolkit-specific bitmap, so the pointer looks the
//! same over the shell, native Wayland clients, and Xwayland.
//!
//! The bundled theme is Capitaine cursors (LGPL-3.0-or-later; see
//! `assets/cursors/capitaine-cursors/README.md`). We parse the XCursor files
//! directly with the `xcursor` crate; no X11 connection is involved. The
//! theme is discovered in the standard XCursor directories, with the in-repo
//! development copy as the last fallback, and the chosen image is the one
//! closest to the output's physical cursor size.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use smithay::input::pointer::CursorIcon;
use xcursor::parser::{parse_xcursor, Image};

/// The desktop cursor theme, matching the assets in
/// `assets/cursors/capitaine-cursors/`.
pub const DEFAULT_THEME: &str = "capitaine-cursors";
/// The logical cursor size in pixels. XCursor themes name images by nominal
/// pixel size; 24 is the desktop's 1x size.
pub const DEFAULT_LOGICAL_SIZE: u32 = 24;

/// A loaded XCursor theme. Icons are parsed lazily and cached for the life of
/// the session; loading a cursor is a small one-time file read, never a
/// per-frame operation.
#[derive(Debug, Default)]
pub struct CursorTheme {
    theme_dir: Option<PathBuf>,
    cache: HashMap<String, Option<Vec<Image>>>,
    logical_size: u32,
}

impl CursorTheme {
    /// Load the configured theme. Never fails: a missing or malformed theme
    /// leaves `theme_dir` empty and callers fall back to the built-in arrow.
    pub fn load() -> Self {
        let theme_name = std::env::var("DRAGONFRUIT_CURSOR_THEME")
            .ok()
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                std::env::var("XCURSOR_THEME")
                    .ok()
                    .filter(|name| !name.trim().is_empty())
            })
            .unwrap_or_else(|| DEFAULT_THEME.to_string());
        let logical_size = std::env::var("DRAGONFRUIT_CURSOR_SIZE")
            .ok()
            .and_then(|size| size.parse::<u32>().ok())
            .or_else(|| {
                std::env::var("XCURSOR_SIZE")
                    .ok()
                    .and_then(|size| size.parse::<u32>().ok())
            })
            .filter(|size| *size > 0)
            .unwrap_or(DEFAULT_LOGICAL_SIZE);
        let theme_dir = find_theme_dir(&theme_name);
        if theme_dir.is_some() {
            crate::trace::log("cursor", &format!("theme {theme_name}"));
        } else {
            crate::trace::log("cursor", &format!("theme {theme_name} not found"));
        }
        Self {
            theme_dir,
            cache: HashMap::new(),
            logical_size,
        }
    }

    /// The logical cursor size in pixels (the theme's nominal size at 1x).
    pub fn logical_size(&self) -> u32 {
        self.logical_size
    }

    /// Choose the theme image for `icon` at `scale`, or `None` when the
    /// icon is not in the theme (the caller then keeps the system arrow).
    pub fn image_for(&mut self, icon: CursorIcon, scale: f64) -> Option<&Image> {
        let target = target_size(self.logical_size, scale);
        let mut chosen: Option<String> = None;
        for name in names_for(icon) {
            let name = name.to_string();
            let entry = self
                .cache
                .entry(name.clone())
                .or_insert_with(|| load_icon(self.theme_dir.as_deref(), &name));
            if entry.as_ref().is_some_and(|images| !images.is_empty()) {
                chosen = Some(name);
                break;
            }
        }
        let name = chosen?;
        let images = self.cache.get(&name)?.as_ref()?;
        best_image(images, target)
    }
}

/// The image size to ask the theme for: the logical size at the output's
/// integer scale, rounded up so a fractional scale stays crisp. The loader
/// then divides the chosen image size by the logical size to get the buffer
/// scale, so the pointer keeps a constant logical size.
fn target_size(logical: u32, scale: f64) -> u32 {
    let integer_scale = scale.max(1.0).ceil() as u32;
    logical.max(1).saturating_mul(integer_scale)
}

/// Pick the smallest image at least as large as `target`, or the largest image
/// when the theme has nothing that large. The first image of a size wins, so
/// an animated cursor draws its first frame.
fn best_image(images: &[Image], target: u32) -> Option<&Image> {
    images
        .iter()
        .filter(|image| image.size >= target)
        .min_by_key(|image| image.size)
        .or_else(|| images.iter().max_by_key(|image| image.size))
}

/// All theme names to try for a cursor shape. The freedesktop/cursor-shape
/// name comes first, then the standard XCursor aliases, then a few names
/// this theme uses for the same semantic shape.
fn names_for(icon: CursorIcon) -> Vec<&'static str> {
    let mut names = Vec::new();
    names.push(icon.name());
    names.extend_from_slice(icon.alt_names());
    match icon {
        CursorIcon::Move => names.extend_from_slice(&["fleur", "dnd-move"]),
        CursorIcon::Grab => names.extend_from_slice(&["openhand", "hand1"]),
        CursorIcon::Grabbing => names.extend_from_slice(&["dnd-move", "openhand"]),
        CursorIcon::AllScroll => names.extend_from_slice(&["all-scroll", "size_all"]),
        CursorIcon::AllResize => names.extend_from_slice(&["size_all", "fleur"]),
        CursorIcon::DndAsk => names.extend_from_slice(&["dnd-move"]),
        _ => {}
    }
    names
}

/// Read and parse one cursor from the theme's `cursors/` directory.
fn load_icon(theme_dir: Option<&Path>, name: &str) -> Option<Vec<Image>> {
    let path = theme_dir?.join("cursors").join(name);
    let bytes = std::fs::read(&path).ok()?;
    parse_xcursor(&bytes)
}

/// Theme search roots, in precedence order. An explicit path wins so tests,
/// captures, and a developer can point at a checkout without installing.
fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["DRAGONFRUIT_CURSOR_PATH", "XCURSOR_PATH"] {
        if let Some(value) = std::env::var_os(var) {
            for entry in std::env::split_paths(&value) {
                if !entry.as_os_str().is_empty() {
                    roots.push(entry);
                }
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".icons"));
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        roots.push(PathBuf::from(data_home).join("icons"));
    }
    let data_dirs =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    for entry in std::env::split_paths(&data_dirs) {
        if !entry.as_os_str().is_empty() {
            roots.push(entry.join("icons"));
        }
    }
    roots.push(PathBuf::from("/usr/share/icons"));
    // Development fallback: the theme is a tracked asset in this repository.
    // A packaged install instead has `share/icons/capitaine-cursors` on the
    // search path above.
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/cursors"));
    roots
}

/// Resolve a theme name to a directory containing `cursors/`.
fn find_theme_dir(theme: &str) -> Option<PathBuf> {
    for root in search_roots() {
        let candidate = root.join(theme);
        if candidate.join("cursors").is_dir() {
            return Some(candidate);
        }
        // Also accept a root that is itself the theme directory.
        if root.join("cursors").is_dir() && root.file_name().is_some_and(|n| n == theme) {
            return Some(root);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_theme() -> CursorTheme {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/cursors/capitaine-cursors");
        assert!(dir.join("cursors").is_dir(), "bundled theme missing");
        CursorTheme {
            theme_dir: Some(dir),
            cache: HashMap::new(),
            logical_size: DEFAULT_LOGICAL_SIZE,
        }
    }

    #[test]
    fn bundled_theme_loads_the_default_and_pointer_cursors() {
        let mut theme = bundled_theme();
        for icon in [CursorIcon::Default, CursorIcon::Pointer, CursorIcon::Text] {
            let image = theme.image_for(icon, 1.0).expect("cursor image");
            assert_eq!(image.size, 24);
            assert_eq!(image.width, image.height);
            assert!(!image.pixels_rgba.is_empty());
        }
    }

    #[test]
    fn scale_selects_a_larger_image_and_keeps_the_logical_size() {
        let mut theme = bundled_theme();
        let one = theme.image_for(CursorIcon::Default, 1.0).unwrap().size;
        let two = theme.image_for(CursorIcon::Default, 2.0).unwrap().size;
        let three = theme.image_for(CursorIcon::Default, 3.0).unwrap().size;
        assert_eq!(one, 24);
        assert_eq!(two, 48);
        assert_eq!(three, 96, "3x rounds up to the next available size");
        for (image, expected_scale) in [(one, 1), (two, 2), (three, 4)] {
            assert_eq!(image as i32 / DEFAULT_LOGICAL_SIZE as i32, expected_scale);
        }
    }

    #[test]
    fn alt_names_cover_the_legacy_xcursor_spellings() {
        assert!(names_for(CursorIcon::Default).contains(&"left_ptr"));
        assert!(names_for(CursorIcon::Pointer).contains(&"hand2"));
        assert!(names_for(CursorIcon::Text).contains(&"xterm"));
        assert!(names_for(CursorIcon::Move).contains(&"fleur"));
        assert!(names_for(CursorIcon::AllScroll).contains(&"all-scroll"));
    }

    #[test]
    fn missing_icons_fall_back_instead_of_panicking() {
        let mut theme = CursorTheme {
            theme_dir: Some(PathBuf::from("/nonexistent/dragonfruit-cursor-theme")),
            cache: HashMap::new(),
            logical_size: DEFAULT_LOGICAL_SIZE,
        };
        // No theme directory: the caller keeps the built-in arrow.
        assert!(theme.image_for(CursorIcon::Default, 1.0).is_none());
    }

    #[test]
    fn best_image_prefers_the_smallest_size_at_least_the_target() {
        let make = |size| Image {
            size,
            width: size,
            height: size,
            xhot: 0,
            yhot: 0,
            delay: 0,
            pixels_rgba: vec![0; (size * size * 4) as usize],
            pixels_argb: vec![],
        };
        let images = vec![make(24), make(48), make(96)];
        assert_eq!(best_image(&images, 24).unwrap().size, 24);
        assert_eq!(best_image(&images, 36).unwrap().size, 48);
        assert_eq!(best_image(&images, 200).unwrap().size, 96);
    }
}
