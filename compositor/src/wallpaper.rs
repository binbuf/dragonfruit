// SPDX-License-Identifier: MIT
//! Image wallpapers (T-05.4): `source`/`fit` sampling, a decode cache, and the
//! per-Space slide.
//!
//! Each Space carries its own wallpaper ([`crate::workspace::Wallpaper`]) and
//! the compositor is its only renderer. This module is the compositor side of
//! that contract:
//!
//! * [`sample_wallpaper`] maps an image's pixel size onto an output rectangle
//!   for the four documented [`WallpaperFit`] modes. It is pure geometry, so
//!   every fit mode is unit-testable without a GPU or a session.
//! * [`WallpaperCache`] decodes a `source` **once** and keeps the raster as a
//!   [`MemoryRenderBuffer`], so a large image is never decoded per frame — the
//!   render loop only imports the cached buffer and samples it.
//! * [`slide_slots`] is the per-Space slide: during a workspace switch the
//!   outgoing and incoming Spaces' wallpapers both translate horizontally with
//!   the same offset the live window surfaces take
//!   ([`crate::state::DfState::apply_overview_scene`]), so the background moves
//!   with its Space. At rest only the active Space is shown.
//!
//! `source` unset (or undecodable) falls back to the solid `color`: the
//! fallback path is the pre-T-05.4 behavior and still works.

use std::collections::HashMap;

use image::GenericImageView;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::utils::{Logical, Rectangle, Size, Transform};

use crate::workspace::{Wallpaper, WallpaperFit};

/// The sampled placement of an image wallpaper on an output: the sub-rectangle
/// of the source image drawn (`src`, in image pixels) and where it lands
/// (`dest`, in output-local logical pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallpaperSample {
    /// The crop of the source image, in image pixels.
    pub src: Rectangle<f64, Logical>,
    /// The destination rectangle, in output-local logical pixels.
    pub dest: Rectangle<i32, Logical>,
}

impl WallpaperSample {
    /// The full source image drawn at its native size (the identity sample).
    #[allow(dead_code)] // Introspection/tests; the render path samples per fit.
    pub fn whole(source: Size<i32, Logical>) -> Self {
        WallpaperSample {
            src: Rectangle::from_size(source.to_f64()),
            dest: Rectangle::from_size(source),
        }
    }
}

/// Map a `source`-sized image onto `target` for `fit`.
///
/// The four modes match the design ([03-workspaces.md] and the Settings
/// Wallpaper pane), with the semantics every desktop user expects:
///
/// * [`WallpaperFit::Fill`] — **cover**: scale uniformly until the image
///   covers `target`, cropping the overflow equally on both axes. The
///   destination is the whole target; only the source is cropped.
/// * [`WallpaperFit::Fit`] — **contain**: scale uniformly until the image fits
///   inside `target`, letterboxing the remainder. The source is whole; the
///   destination is centered and smaller than the target on one axis.
/// * [`WallpaperFit::Stretch`] — scale each axis independently to fill the
///   target (aspect not preserved).
/// * [`WallpaperFit::Center`] — draw at native size, centered; an image larger
///   than the target overflows and is clipped by the output.
///
/// A degenerate (zero/negative) source or target yields no drawn image
/// (`src` empty) rather than a divide-by-zero.
pub fn sample_wallpaper(
    source: Size<i32, Logical>,
    fit: WallpaperFit,
    target: Rectangle<i32, Logical>,
) -> WallpaperSample {
    let empty = WallpaperSample {
        src: Rectangle::from_size((0.0, 0.0).into()),
        dest: Rectangle::from_size((0, 0).into()),
    };
    if source.w <= 0 || source.h <= 0 || target.size.w <= 0 || target.size.h <= 0 {
        return empty;
    }
    let sw = f64::from(source.w);
    let sh = f64::from(source.h);
    let tw = f64::from(target.size.w);
    let th = f64::from(target.size.h);
    let full_src = Rectangle::from_size(source.to_f64());

    match fit {
        WallpaperFit::Fill => {
            // Cover: the scale is the larger of the two axis ratios; the
            // source crop is the centered part that then fills the target.
            let scale = (tw / sw).max(th / sh);
            let crop_w = (tw / scale).min(sw);
            let crop_h = (th / scale).min(sh);
            WallpaperSample {
                src: Rectangle::new(
                    ((sw - crop_w) / 2.0, (sh - crop_h) / 2.0).into(),
                    (crop_w, crop_h).into(),
                ),
                dest: target,
            }
        }
        WallpaperFit::Fit => {
            // Contain: scale to fit, centered inside the target.
            let scale = (tw / sw).min(th / sh);
            let w = (sw * scale).round().max(1.0) as i32;
            let h = (sh * scale).round().max(1.0) as i32;
            WallpaperSample {
                src: full_src,
                dest: Rectangle::new(
                    (
                        target.loc.x + (target.size.w - w) / 2,
                        target.loc.y + (target.size.h - h) / 2,
                    )
                        .into(),
                    (w, h).into(),
                ),
            }
        }
        WallpaperFit::Stretch => WallpaperSample {
            src: full_src,
            dest: target,
        },
        WallpaperFit::Center => WallpaperSample {
            src: full_src,
            dest: Rectangle::new(
                (
                    target.loc.x + (target.size.w - source.w) / 2,
                    target.loc.y + (target.size.h - source.h) / 2,
                )
                    .into(),
                source,
            ),
        },
    }
}

/// A decoded wallpaper raster, kept as a GPU-importable memory buffer.
///
/// `size` is the source image's pixel size; the buffer never changes after
/// construction, so the renderer's texture import is stable across frames.
#[derive(Debug)]
pub struct DecodedWallpaper {
    pub size: Size<i32, Logical>,
    buffer: MemoryRenderBuffer,
}

impl DecodedWallpaper {
    /// The cached buffer, for the render loop to import and sample.
    pub fn buffer(&self) -> &MemoryRenderBuffer {
        &self.buffer
    }
}

/// The per-compositor wallpaper decode cache (T-05.4): `source` path -> the
/// decoded raster, so a large image is decoded once and never per frame.
///
/// A source that fails to decode is cached as absent, so a broken path is not
/// retried every frame (the color fallback renders instead). [`Self::clear`]
/// drops every entry, including failed ones, so replacing an image on disk can
/// take effect.
#[derive(Debug, Default)]
pub struct WallpaperCache {
    decoded: HashMap<String, Option<DecodedWallpaper>>,
    decodes: u64,
}

#[allow(dead_code)] // Decode counters/clearing are the T-08/T-16 settings and test surface.
impl WallpaperCache {
    pub fn new() -> Self {
        WallpaperCache::default()
    }

    /// The decoded wallpaper for `source`, decoding it on first use. Returns
    /// `None` when the source is absent or undecodable (color fallback).
    pub fn get(&mut self, source: &str) -> Option<&DecodedWallpaper> {
        if !self.decoded.contains_key(source) {
            let decoded = decode(source);
            if decoded.is_some() {
                self.decodes += 1;
            }
            self.decoded.insert(source.to_string(), decoded);
        }
        self.decoded.get(source).and_then(Option::as_ref)
    }

    /// How many distinct sources were successfully decoded (the per-frame
    /// "no decode" counter: this must not grow after the first frame).
    pub fn decodes(&self) -> u64 {
        self.decodes
    }

    /// Number of sources visited, decoded or not.
    pub fn len(&self) -> usize {
        self.decoded.len()
    }

    pub fn is_empty(&self) -> bool {
        self.decoded.is_empty()
    }

    /// Forget every decoded raster (and every cached miss).
    pub fn clear(&mut self) {
        self.decoded.clear();
        self.decodes = 0;
    }
}

/// Decode `source` from disk into an RGBA memory buffer, or `None`.
fn decode(source: &str) -> Option<DecodedWallpaper> {
    let image = image::open(source).ok()?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let size = Size::from((width as i32, height as i32));
    let rgba = image.to_rgba8();
    // `to_rgba8` is R,G,B,A in memory; `Abgr8888` maps to GL `RGBA8`/
    // `UNSIGNED_BYTE`, which reads that byte order (T-05.4 ADR 0019).
    let buffer = MemoryRenderBuffer::from_slice(
        rgba.as_raw(),
        Fourcc::Abgr8888,
        (width as i32, height as i32),
        1,
        Transform::Normal,
        None,
    );
    Some(DecodedWallpaper { size, buffer })
}

/// The horizontal offset, in output widths, of the Space at `index` during a
/// switch of `direction` at `progress`.
///
/// This is the exact formula the live window surfaces use
/// ([`crate::state::DfState::apply_overview_scene`]): the active Space slides
/// out in the switch direction and its neighbour slides in from the opposite
/// edge, so the wallpaper and the windows move together. `None` when the Space
/// is neither of the two the slide shows.
pub fn slide_offset(
    active: usize,
    index: usize,
    count: usize,
    direction: i32,
    progress: f64,
) -> Option<f64> {
    let target = active as i32 + direction;
    if index == active {
        Some(-f64::from(direction) * progress)
    } else if target >= 0 && target < count as i32 && index as i32 == target {
        Some(-f64::from(direction) * progress + f64::from(direction))
    } else {
        None
    }
}

/// The visible wallpaper slots for an output: `(Space index, offset_x)` in
/// output-local logical pixels, for the active Space and (mid-switch) the
/// neighbour sliding in.
///
/// A slot is included only while some part of it can be on-screen
/// (`|offset_x| < width`), so a settled scene shows exactly the active Space
/// and never decodes a neighbour's image. At `progress == 0` only the active
/// Space remains.
pub fn slide_slots(
    active: usize,
    count: usize,
    direction: i32,
    progress: f64,
    width: i32,
) -> Vec<(usize, i32)> {
    if count == 0 || width <= 0 {
        return Vec::new();
    }
    let mut slots = Vec::new();
    for index in 0..count {
        let Some(offset) = slide_offset(active, index, count, direction, progress) else {
            continue;
        };
        let x = (offset * f64::from(width)).round() as i32;
        if x > -width && x < width {
            slots.push((index, x));
        }
    }
    slots
}

/// One wallpaper the render layer draws this frame: which Space, where its
/// top-left is (output-local logical), and the [`Wallpaper`] itself.
#[derive(Debug, Clone, PartialEq)]
pub struct WallpaperSlot {
    pub index: usize,
    pub offset_x: i32,
    pub wallpaper: Wallpaper,
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::Point;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    #[test]
    fn fill_covers_the_target_and_crops_the_long_axis() {
        // A 2:1 image onto a 1:1 target covers by width and crops height.
        let sample = sample_wallpaper((800, 400).into(), WallpaperFit::Fill, rect(0, 0, 400, 400));
        assert_eq!(sample.dest, rect(0, 0, 400, 400));
        // The crop keeps the target aspect (1:1) and is centered vertically.
        assert!((sample.src.size.w - 400.0).abs() < 1e-6);
        assert!((sample.src.size.h - 400.0).abs() < 1e-6);
        assert!((sample.src.loc.y - 0.0).abs() < 1e-6, "{sample:?}");
        // A 4:1 image onto a 1:1 target: centered horizontal crop.
        let wide = sample_wallpaper(
            (1600, 400).into(),
            WallpaperFit::Fill,
            rect(10, 20, 400, 400),
        );
        assert_eq!(wide.dest, rect(10, 20, 400, 400));
        assert!((wide.src.size.w - 400.0).abs() < 1e-6);
        assert!((wide.src.loc.x - 600.0).abs() < 1e-6, "centered crop");
    }

    #[test]
    fn fit_letterboxes_and_stays_inside_the_target() {
        // A 2:1 image into a 1:1 target: contain fits width, letterbox top and
        // bottom, centered.
        let sample = sample_wallpaper((800, 400).into(), WallpaperFit::Fit, rect(0, 0, 400, 400));
        assert_eq!(sample.src, Rectangle::from_size((800.0, 400.0).into()));
        assert_eq!(sample.dest, rect(0, 100, 400, 200));
        // The image never escapes the target.
        assert!(rect(0, 0, 400, 400).contains_rect(sample.dest));
    }

    #[test]
    fn stretch_fills_each_axis_and_center_is_native_size() {
        let image = (800, 400).into();
        let target = rect(10, 20, 400, 400);
        let stretch = sample_wallpaper(image, WallpaperFit::Stretch, target);
        assert_eq!(stretch.dest, target);
        assert_eq!(stretch.src, Rectangle::from_size((800.0, 400.0).into()));

        let center = sample_wallpaper(image, WallpaperFit::Center, target);
        assert_eq!(center.dest, rect(10 - 200, 20, 800, 400));
        assert_eq!(center.src, Rectangle::from_size((800.0, 400.0).into()));
    }

    #[test]
    fn a_degenerate_source_or_target_draws_nothing() {
        let target = rect(0, 0, 100, 100);
        let empty = sample_wallpaper((0, 0).into(), WallpaperFit::Fill, target);
        assert_eq!(empty.src.size, (0.0, 0.0).into());
        assert_eq!(empty.dest, rect(0, 0, 0, 0));
        let no_target = sample_wallpaper((10, 10).into(), WallpaperFit::Fit, rect(0, 0, 0, 0));
        assert_eq!(no_target.dest, rect(0, 0, 0, 0));
    }

    #[test]
    fn slide_offset_matches_the_window_slide_formula() {
        // Active 1 of 3, moving next (+1): active slides left, Space 2 enters
        // from the right edge.
        assert_eq!(slide_offset(1, 1, 3, 1, 0.0), Some(0.0));
        assert_eq!(slide_offset(1, 2, 3, 1, 0.0), Some(1.0));
        assert_eq!(slide_offset(1, 1, 3, 1, 0.5), Some(-0.5));
        assert_eq!(slide_offset(1, 2, 3, 1, 0.5), Some(0.5));
        assert_eq!(slide_offset(1, 1, 3, 1, 1.0), Some(-1.0));
        assert_eq!(slide_offset(1, 2, 3, 1, 1.0), Some(0.0));
        // The other Spaces are not part of the slide.
        assert_eq!(slide_offset(1, 0, 3, 1, 0.5), None);

        // Moving previous (-1): Space 0 enters from the left.
        assert_eq!(slide_offset(1, 1, 3, -1, 0.5), Some(0.5));
        assert_eq!(slide_offset(1, 0, 3, -1, 0.5), Some(-0.5));
        assert_eq!(slide_offset(1, 2, 3, -1, 0.5), None);
    }

    #[test]
    fn slide_slots_only_show_on_screen_spaces() {
        let width = 1280;
        // At rest: exactly the active Space, at the origin.
        assert_eq!(slide_slots(1, 3, 1, 0.0, width), vec![(1, 0)]);
        // Halfway through a next-switch: both Spaces are partially on-screen.
        assert_eq!(
            slide_slots(1, 3, 1, 0.5, width),
            vec![(1, -width / 2), (2, width / 2)]
        );
        // Settled on the next Space: the neighbour is now the only on-screen
        // slot, exactly at the origin.
        assert_eq!(slide_slots(1, 3, 1, 1.0, width), vec![(2, 0)]);
        // A switch at the end has no neighbour: only the active Space moves.
        assert_eq!(slide_slots(2, 3, 1, 0.5, width), vec![(2, -width / 2)]);
    }

    #[test]
    fn slide_slots_are_empty_for_a_degenerate_output() {
        assert!(slide_slots(0, 0, 1, 0.5, 1280).is_empty());
        assert!(slide_slots(0, 3, 1, 0.5, 0).is_empty());
    }

    #[test]
    fn cache_decodes_a_real_image_once_and_reuses_it() {
        // A 2x2 PNG written to a temp file: decoding is exercised end to end.
        let dir = std::env::temp_dir();
        let path = dir.join(format!("df-wallpaper-{}.png", std::process::id()));
        let mut image = image::RgbaImage::new(2, 2);
        image.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 0, image::Rgba([0, 255, 0, 255]));
        image.put_pixel(0, 1, image::Rgba([0, 0, 255, 255]));
        image.put_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
        image.save(&path).expect("write the test wallpaper png");
        let source = path.to_string_lossy().to_string();

        let mut cache = WallpaperCache::new();
        assert!(cache.is_empty());
        let decoded = cache.get(&source).expect("the png decodes");
        assert_eq!(decoded.size, (2, 2).into());
        assert_eq!(cache.decodes(), 1);
        // A second lookup is served from the cache: no re-decode.
        assert!(cache.get(&source).is_some());
        assert_eq!(cache.decodes(), 1);
        assert_eq!(cache.len(), 1);

        // A missing source is a cached miss: the color fallback renders.
        assert!(cache.get("/no/such/df-wallpaper.png").is_none());
        assert!(cache.get("/no/such/df-wallpaper.png").is_none());
        assert_eq!(cache.decodes(), 1);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
