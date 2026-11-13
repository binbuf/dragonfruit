// SPDX-License-Identifier: MIT
//! Reusable texture bookkeeping for the material pass (T-21.3).
//!
//! The scene texture and every panel's Kawase chain are allocated once and
//! reused. The scene cache key is deliberately narrow: a new texture is
//! required only when the region's buffer size, the output mode, or the output
//! scale changes. A moving panel that keeps the same padded-union size keeps
//! the same texture.
//!
//! Both caches also own the render-element identity for their slot. A panel's
//! `Id` is created when its [`PanelKey`] first appears and is reused every
//! frame; it is dropped only when the panel disappears. The scene texture owns
//! one `Id` per output for its whole lifetime. That is the damage-tracker
//! contract: an unchanged element is not new damage just because its texture
//! was rebuilt for this frame.
//!
//! This module is renderer-free: the generic slot/cache types are asserted
//! headless, and the backends supply `GlesTexture` as the stored type.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Size};

use crate::material_preset::{Preset, RenderMode};
use crate::render::RegionPaint;
use crate::window::blur::{BlurSpec, RegionFilter};
use crate::window::corner::CornerRadii;
use crate::window::decoration::ColorScheme;
use crate::window::degrade::DegradeTier;

/// The allocation identity of a scene texture.
///
/// The region origin is intentionally **not** part of the key. Moving the Dock
/// can change the padded union's origin while its size stays constant; that
/// must reuse the texture. A mode or scale change reallocates even when the
/// region happens to round to the same size, because the texture's coordinate
/// space is tied to the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SceneTextureKey {
    /// The region texture size in buffer pixels.
    size: Size<i32, Buffer>,
    /// The output's physical mode size.
    output: Size<i32, Physical>,
    /// The output scale, compared by bit pattern.
    scale: (u64, u64),
}

impl SceneTextureKey {
    pub(crate) fn new(
        size: Size<i32, Buffer>,
        output: Size<i32, Physical>,
        scale: Scale<f64>,
    ) -> Self {
        Self {
            size,
            output,
            scale: (scale.x.to_bits(), scale.y.to_bits()),
        }
    }

    pub(crate) fn size(&self) -> Size<i32, Buffer> {
        self.size
    }
}

/// One reusable texture plus the key it was allocated for.
///
/// The cache owns the scene element's stable [`Id`] and a [`CommitCounter`]
/// that advances only when the offscreen scene actually changed. A settled
/// scene therefore leaves the composited scene element untouched instead of
/// forcing a full-output repaint every frame.
#[derive(Debug)]
pub(crate) struct SceneTextureCache<T> {
    texture: Option<T>,
    key: Option<SceneTextureKey>,
    allocations: u64,
    id: Id,
    commit: CommitCounter,
    last_damage: Vec<Rectangle<i32, Physical>>,
    /// Advances when the texture is reallocated or its damage tracker is
    /// reset. Panel blur caches include it so a fresh/blank scene texture
    /// can never inherit a panel's previous blur result.
    generation: u64,
    /// The damage tracker for the offscreen scene texture. It is reused while
    /// the region texture key is unchanged, so an unchanged scene renders
    /// nothing; a reallocation or region-size/scale change resets it and
    /// forces one full region render.
    tracker: Option<(SceneTextureKey, Point<i32, Physical>, OutputDamageTracker)>,
    /// The scene-content epoch this texture was last rendered at (T238
    /// follow-up). Equal to the live epoch only while the scene has not
    /// changed; the renderer can then skip the damage-tracker walk entirely.
    last_epoch: Option<u64>,
    /// Hash of the element ids that intersected this region on the last
    /// render. A change means an element entered or left the bounded region;
    /// the tracker's previous state may not cover the area it vacated, so the
    /// region must be repainted instead of leaving stale pixels behind.
    region_elements: Option<u64>,
}

impl<T> Default for SceneTextureCache<T> {
    fn default() -> Self {
        Self {
            texture: None,
            key: None,
            allocations: 0,
            id: Id::new(),
            commit: CommitCounter::default(),
            last_damage: Vec::new(),
            generation: 0,
            tracker: None,
            last_epoch: None,
            region_elements: None,
        }
    }
}

impl<T> SceneTextureCache<T> {
    pub(crate) fn texture(&self) -> Option<&T> {
        self.texture.as_ref()
    }

    /// The stable render-element id of this output's scene texture.
    pub(crate) fn id(&self) -> Id {
        self.id.clone()
    }

    /// The scene element's commit, advanced by [`Self::note_scene_damage`].
    pub(crate) fn commit(&self) -> CommitCounter {
        self.commit
    }

    /// The scene texture's content generation. It changes on reallocation and
    /// on a damage-tracker reset, so a panel blur cache keyed by it cannot
    /// reuse pixels from a blank or re-scoped texture.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Remember the last changed commit's damage in scene-texture coordinates.
    pub(crate) fn note_scene_damage(&mut self, damage: Option<Vec<Rectangle<i32, Physical>>>) {
        if let Some(damage) = damage.filter(|damage| !damage.is_empty()) {
            self.last_damage = damage;
            self.commit.increment();
        }
    }

    fn damaged_source_since(
        &self,
        previous: Option<CommitCounter>,
        source: Rectangle<f64, Buffer>,
        padding: i32,
        scale: Scale<f64>,
    ) -> bool {
        match self.commit.distance(previous) {
            Some(0) => false,
            Some(1) => {
                let padding = f64::from(padding.max(0));
                // The first blur pass reads adjacent texels with linear
                // filtering, even at scales where the logical padding is small.
                let pad_x = (padding * scale.x).ceil().max(2.0);
                let pad_y = (padding * scale.y).ceil().max(2.0);
                let sample = Rectangle::<f64, Physical>::new(
                    (source.loc.x - pad_x, source.loc.y - pad_y).into(),
                    (source.size.w + 2.0 * pad_x, source.size.h + 2.0 * pad_y).into(),
                );
                self.last_damage
                    .iter()
                    .any(|damage| sample.overlaps(damage.to_f64()))
            }
            // A panel skipped several commits. Only the latest damage is
            // available, so rebuild conservatively.
            _ => true,
        }
    }

    /// Whether `key` needs a fresh allocation.
    ///
    /// The cache grows to fit a larger region and reuses the larger texture
    /// when the region shrinks again. A Dock magnification sweep therefore
    /// reaches its maximum region once and then allocates nothing for the rest
    /// of the sweep. Output mode or scale changes always reallocate, because
    /// the texture's coordinate space is tied to the output. The region
    /// origin is not part of the key.
    pub(crate) fn needs_alloc(&self, key: SceneTextureKey) -> bool {
        let Some(current) = self.key else {
            return true;
        };
        if self.texture.is_none() {
            return true;
        }
        current.output != key.output
            || current.scale != key.scale
            || current.size.w < key.size.w
            || current.size.h < key.size.h
    }

    /// Install a freshly allocated texture and remember its key.
    ///
    /// A new texture starts with no previous content, so the damage tracker is
    /// reset to force a full region render on the next frame. The scene's
    /// element id and commit survive the reallocation.
    pub(crate) fn install(&mut self, texture: T, key: SceneTextureKey) {
        self.texture = Some(texture);
        self.key = Some(key);
        self.tracker = None;
        self.last_epoch = None;
        self.region_elements = None;
        self.allocations += 1;
        self.generation = self.generation.wrapping_add(1);
    }

    /// The scene-content epoch this texture was last rendered at.
    pub(crate) fn last_epoch(&self) -> Option<u64> {
        self.last_epoch
    }

    /// Remember that the texture now contains the scene at `epoch`.
    pub(crate) fn set_last_epoch(&mut self, epoch: u64) {
        self.last_epoch = Some(epoch);
    }

    /// Record the set of elements that intersected the bounded region this
    /// frame. Returns `true` when it changed since the last render.
    ///
    /// The damage tracker only sees elements that are submitted. When an
    /// element leaves the region it is filtered out before the tracker runs;
    /// its old pixels would otherwise survive in the cached scene texture and
    /// the menu-bar veil would show a ghost of the departed window. A set
    /// change therefore forces a full region repaint.
    pub(crate) fn note_region_elements(&mut self, hash: u64) -> bool {
        let changed = self.region_elements != Some(hash);
        self.region_elements = Some(hash);
        changed
    }

    /// Drop the retained damage tracker so the next render repaints the whole
    /// region. Used when the element set changes without a region reallocation.
    pub(crate) fn reset_tracker(&mut self) {
        self.tracker = None;
        self.last_epoch = None;
    }

    /// Whether the scene texture can be reused without walking the element
    /// list: the allocation and tracker match the region, and the scene
    /// content epoch is unchanged since the last render.
    pub(crate) fn can_skip(
        &self,
        key: SceneTextureKey,
        origin: Point<i32, Physical>,
        epoch: u64,
    ) -> bool {
        self.texture.is_some()
            && !self.needs_alloc(key)
            && self.tracker_matches(key, origin)
            && self.last_epoch == Some(epoch)
    }

    /// Lifetime allocation count (test/diagnostic evidence).
    pub(crate) fn allocations(&self) -> u64 {
        self.allocations
    }

    /// The allocated scene texture's RGBA byte size (`0` before the first
    /// allocation). Reported as material memory evidence.
    pub(crate) fn bytes(&self) -> u64 {
        self.key
            .map(|key| {
                let size = key.size();
                (size.w.max(0) as u64)
                    .saturating_mul(size.h.max(0) as u64)
                    .saturating_mul(4)
            })
            .unwrap_or(0)
    }

    /// The tracker is tied to the region origin even when the texture can be reused.
    pub(crate) fn tracker_matches(
        &self,
        key: SceneTextureKey,
        origin: Point<i32, Physical>,
    ) -> bool {
        self.tracker
            .as_ref()
            .is_some_and(|(tracker_key, tracker_origin, _)| {
                *tracker_key == key && *tracker_origin == origin
            })
    }

    /// Install the scene damage tracker for `key`.
    pub(crate) fn set_tracker(
        &mut self,
        key: SceneTextureKey,
        origin: Point<i32, Physical>,
        tracker: OutputDamageTracker,
    ) {
        self.tracker = Some((key, origin, tracker));
        // A fresh tracker has no retained element states, so the next render
        // must happen even if the scene epoch happens to match.
        self.last_epoch = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// The scene damage tracker, if one has been installed.
    pub(crate) fn tracker_mut(&mut self) -> Option<&mut OutputDamageTracker> {
        self.tracker.as_mut().map(|(_, _, tracker)| tracker)
    }
}

/// Stable identity of one chrome panel across frames (T-21.3).
///
/// The key hashes the surface's `df_layer_surface` namespace, its layer, and
/// its Wayland object id. Geometry is deliberately **not** part of the key: a
/// Dock that magnifies keeps one identity, so its render element's [`Id`] and
/// damage state stay stable while the geometry itself produces the damage.
/// Hashing keeps the key allocation-free on the per-frame path. The key is
/// scoped to one output's panel cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelKey(u64);

impl PanelKey {
    /// Build a panel key from the chrome surface's stable fields.
    pub fn new(namespace: &str, layer: u32, surface_id: u32) -> Self {
        let mut hasher = DefaultHasher::new();
        namespace.hash(&mut hasher);
        layer.hash(&mut hasher);
        surface_id.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Build a key for one independently keyed region on a chrome surface
    /// (T197). The region key joins the surface identity, so two cards on the
    /// same overlay keep separate element ids and blur textures even when
    /// their geometry swaps.
    pub fn for_region(namespace: &str, layer: u32, surface_id: u32, region_key: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        namespace.hash(&mut hasher);
        layer.hash(&mut hasher);
        surface_id.hash(&mut hasher);
        region_key.hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// The sample inputs that decide a region's filtered pixels (T197).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RegionSample {
    pub spec: BlurSpec,
    /// Padded source footprint: blur/lens kernel plus declared overscan.
    pub padding: i32,
    pub filter: RegionFilter,
}

/// Everything that can change a panel's blurred pixels.
///
/// The Dock's own hover surface is not part of the sampled scene, so a pointer
/// sweep over the Dock leaves this key unchanged and the cached blur can be
/// reused. Damage to the sampled scene, a source move, a blur-spec change, or an
/// output scale/chain change invalidates it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PanelBlurKey {
    scene_generation: u64,
    /// Per-panel scene epoch. It advances only when the scene damage actually
    /// intersects this panel's blur source (plus padding), so an off-panel
    /// window repaint does not rebuild the Dock's blur.
    scene_epoch: u64,
    scene_src: Rectangle<f64, Buffer>,
    spec: BlurSpec,
    /// The padded source footprint this region samples (T197). A declared
    /// rim/shadow/veil overscan or a wider refraction kernel enlarges it, so
    /// damage in the extra margin invalidates the cached texture.
    padding: i32,
    /// Filtered vs sharp; a veil never reuses a blurred texture.
    filter: RegionFilter,
    chain_size: Size<i32, Physical>,
    scale: (u64, u64),
}

impl PanelBlurKey {
    fn new(
        scene_generation: u64,
        scene_epoch: u64,
        scene_src: Rectangle<f64, Buffer>,
        sample: RegionSample,
        chain_size: Size<i32, Physical>,
        scale: Scale<f64>,
    ) -> Self {
        Self {
            scene_generation,
            scene_epoch,
            scene_src,
            spec: sample.spec,
            padding: sample.padding,
            filter: sample.filter,
            chain_size,
            scale: (scale.x.to_bits(), scale.y.to_bits()),
        }
    }

    /// Which input invalidated the cached blur relative to `previous`.
    pub(crate) fn miss_reason(&self, previous: &Self) -> Option<BlurMissReason> {
        if self.scene_generation != previous.scene_generation
            || self.scene_epoch != previous.scene_epoch
        {
            Some(BlurMissReason::Scene)
        } else if self.scene_src != previous.scene_src {
            Some(BlurMissReason::Source)
        } else if self.spec != previous.spec || self.filter != previous.filter {
            Some(BlurMissReason::Spec)
        } else if self.chain_size != previous.chain_size || self.scale != previous.scale {
            Some(BlurMissReason::Chain)
        } else {
            None
        }
    }
}

/// Paint-only inputs for one material region (T197).
///
/// These are deliberately **not** part of [`PanelBlurKey`]: a scheme, tier,
/// or tint change repaints the region element and advances its commit, but the
/// filtered scene texture is unchanged and must be reused. Geometry and radius
/// are not here either; the element geometry and corner spans carry them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PanelPaintKey {
    scheme: ColorScheme,
    preset: Preset,
    tier: DegradeTier,
    mode: RenderMode,
    alpha: u32,
    tint: u32,
    edge: u32,
    edge_width: u32,
    foreground: u32,
    revision: u64,
    corners: CornerRadii,
    output_generation: u64,
}

impl PanelPaintKey {
    pub(crate) fn new(paint: RegionPaint, corners: CornerRadii, output_generation: u64) -> Self {
        let bits = |value: f32| {
            if value.is_finite() {
                value.to_bits()
            } else {
                f32::NAN.to_bits()
            }
        };
        Self {
            scheme: paint.scheme,
            preset: paint.preset,
            tier: paint.tier,
            mode: paint.mode,
            alpha: bits(paint.alpha),
            tint: bits(paint.tint),
            edge: bits(paint.edge),
            edge_width: bits(paint.edge_width),
            foreground: bits(paint.foreground),
            revision: paint.revision,
            corners,
            output_generation,
        }
    }
}

/// Why a panel blur cache entry was rebuilt (T-21.5 diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlurMissReason {
    First,
    Scene,
    Source,
    Spec,
    Chain,
}

/// Per-frame blur miss breakdown. A Dock hover with a static scene should
/// only advance `skips`; a `source` miss means the plate geometry is moving,
/// and a `scene` miss means the sampled scene changed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlurMissCounts {
    pub first: u64,
    pub scene: u64,
    pub source: u64,
    pub spec: u64,
    pub chain: u64,
}

impl BlurMissCounts {
    pub(crate) fn record(&mut self, reason: BlurMissReason) {
        match reason {
            BlurMissReason::First => self.first = self.first.saturating_add(1),
            BlurMissReason::Scene => self.scene = self.scene.saturating_add(1),
            BlurMissReason::Source => self.source = self.source.saturating_add(1),
            BlurMissReason::Spec => self.spec = self.spec.saturating_add(1),
            BlurMissReason::Chain => self.chain = self.chain.saturating_add(1),
        }
    }
}

/// One panel's reusable Kawase textures plus its stable render-element id.
#[derive(Debug)]
pub(crate) struct PanelTextureSlot<T> {
    id: Id,
    /// Stable id for the sharp-scene bevel element that sits over the body
    /// for the measured Dock. It must be distinct from `id` so the damage
    /// tracker treats the two elements independently.
    bevel_id: Id,
    /// The panel's own commit, advanced only when the cached blur is rebuilt.
    /// A scene change that alters the blurred pixels therefore damages the
    /// panel even when its geometry did not move.
    commit: CommitCounter,
    textures: Vec<T>,
    blur_key: Option<PanelBlurKey>,
    paint_key: Option<PanelPaintKey>,
    last_scene_commit: Option<CommitCounter>,
    /// The last scene-damage commit a sharp/veil element was built against
    /// (T200). The sharp path samples the scene texture directly, so it must
    /// damage the element whenever that texture changed.
    last_sharp_scene_commit: Option<CommitCounter>,
}

impl<T> PanelTextureSlot<T> {
    fn new() -> Self {
        Self {
            id: Id::new(),
            bevel_id: Id::new(),
            commit: CommitCounter::default(),
            textures: Vec::new(),
            blur_key: None,
            paint_key: None,
            last_scene_commit: None,
            last_sharp_scene_commit: None,
        }
    }

    /// The panel's stable render-element id.
    pub(crate) fn id(&self) -> &Id {
        &self.id
    }

    /// The stable id for the sharp-scene bevel overlay (measured Dock only).
    pub(crate) fn bevel_id(&self) -> &Id {
        &self.bevel_id
    }

    /// The panel element's commit, advanced whenever its blur is rebuilt or
    /// its paint-only inputs change.
    pub(crate) fn commit(&self) -> CommitCounter {
        self.commit
    }

    /// Record paint-only inputs (scheme/tint/tier/radius/output generation).
    /// The filtered texture is reused; the element commit advances so the
    /// damage tracker repaints the new paint.
    pub(crate) fn note_paint(&mut self, key: PanelPaintKey) {
        if self.paint_key != Some(key) {
            self.paint_key = Some(key);
            self.commit.increment();
        }
    }

    /// Record the scene-damage commit a sharp/veil element sampled (T200).
    /// The scene cache advances its commit only when the offscreen scene
    /// actually changed, so a settled veil stays damage-free while a moving
    /// window under the bar repaints the veil.
    pub(crate) fn note_sharp_scene_commit(&mut self, commit: CommitCounter) {
        if self.last_sharp_scene_commit != Some(commit) {
            self.last_sharp_scene_commit = Some(commit);
            self.commit.increment();
        }
    }

    /// The Kawase-chain textures for this panel.
    pub(crate) fn textures_mut(&mut self) -> &mut Vec<T> {
        &mut self.textures
    }

    /// The composed result of the last successful Kawase chain.
    pub(crate) fn last_texture(&self) -> Option<&T> {
        self.textures.last()
    }

    /// The key of the cached chain, when one is present.
    pub(crate) fn blur_key(&self) -> Option<&PanelBlurKey> {
        self.blur_key.as_ref()
    }

    pub(crate) fn blur_key_for_scene<S>(
        &mut self,
        scene: &SceneTextureCache<S>,
        source: Rectangle<f64, Buffer>,
        sample: RegionSample,
        chain_size: Size<i32, Physical>,
        scale: Scale<f64>,
    ) -> PanelBlurKey {
        let mut epoch = self.blur_key.map_or(0, |key| key.scene_epoch);
        if scene.damaged_source_since(self.last_scene_commit, source, sample.padding, scale) {
            epoch = epoch.wrapping_add(1);
        }
        self.last_scene_commit = Some(scene.commit());
        PanelBlurKey::new(scene.generation(), epoch, source, sample, chain_size, scale)
    }

    /// Whether the cached chain can be reused for `key`.
    pub(crate) fn blur_cache_hit(&self, key: &PanelBlurKey) -> bool {
        self.blur_key.as_ref() == Some(key) && !self.textures.is_empty()
    }

    /// Invalidate the cached chain before a rebuild. Keeping the old key after
    /// a failed render would let the next frame sample a partially written
    /// texture.
    pub(crate) fn invalidate_blur_cache(&mut self) {
        self.blur_key = None;
    }

    /// Record a successful rebuild and advance the panel's damage commit.
    pub(crate) fn set_blur_key(&mut self, key: PanelBlurKey) {
        self.blur_key = Some(key);
        self.commit.increment();
    }

    #[cfg(test)]
    fn textures(&self) -> &[T] {
        &self.textures
    }
}

/// One Kawase-chain slot per chrome panel, keyed by stable panel identity.
///
/// A panel's slot is reused across frames even when its position in the
/// front-to-back list changes. When the panel set shrinks, the removed slots
/// are dropped by [`Self::trim_to`], which releases their `GlesTexture`
/// handles instead of keeping them alive for the rest of the session.
#[derive(Debug)]
pub(crate) struct PanelTextureSlots<T> {
    panels: HashMap<PanelKey, PanelTextureSlot<T>>,
    allocations: u64,
}

impl<T> Default for PanelTextureSlots<T> {
    fn default() -> Self {
        Self {
            panels: HashMap::new(),
            allocations: 0,
        }
    }
}

impl<T> PanelTextureSlots<T> {
    /// The slot for `key`, created on first use.
    pub(crate) fn slot(&mut self, key: &PanelKey) -> &mut PanelTextureSlot<T> {
        self.panels
            .entry(*key)
            .or_insert_with(PanelTextureSlot::new)
    }

    /// Drop every slot whose panel is not in `keys`.
    pub(crate) fn trim_to(&mut self, keys: &[PanelKey]) {
        self.panels.retain(|key, _| keys.contains(key));
    }

    /// Record one texture allocation in any panel slot.
    pub(crate) fn record_alloc(&mut self) {
        self.allocations += 1;
    }

    pub(crate) fn allocations(&self) -> u64 {
        self.allocations
    }

    #[cfg(test)]
    fn slot_count(&self) -> usize {
        self.panels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    use smithay::backend::renderer::element::Element;
    use smithay::utils::{Point, Rectangle, Transform};

    fn size(w: i32, h: i32) -> Size<i32, Buffer> {
        Size::from((w, h))
    }

    fn output(w: i32, h: i32) -> Size<i32, Physical> {
        Size::from((w, h))
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Physical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn sample(spec: BlurSpec, padding: i32, filter: RegionFilter) -> RegionSample {
        RegionSample {
            spec,
            padding,
            filter,
        }
    }

    #[test]
    fn a_scene_key_changes_only_with_size_mode_or_scale() {
        let key = SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.5.into());
        assert_eq!(
            key,
            SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.5.into())
        );

        // A different region size, output mode, or scale is a new allocation.
        assert_ne!(
            key,
            SceneTextureKey::new(size(401, 80), output(1920, 1200), 1.5.into())
        );
        assert_ne!(
            key,
            SceneTextureKey::new(size(400, 80), output(1920, 1200), 2.0.into())
        );
        assert_ne!(
            key,
            SceneTextureKey::new(size(400, 80), output(2560, 1600), 1.5.into())
        );

        // The region origin is not in the key: a Dock that moves while keeping
        // the same padded-union size reuses the texture.
        let moved_origin = SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.5.into());
        assert_eq!(key, moved_origin);
        assert_eq!(key.size(), size(400, 80));
    }

    #[test]
    fn a_scene_cache_can_skip_only_at_the_same_epoch_key_and_origin() {
        use smithay::backend::renderer::damage::OutputDamageTracker;
        use smithay::utils::{Point, Transform};

        let mut cache: SceneTextureCache<u32> = SceneTextureCache::default();
        let key = SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.5.into());
        let origin = Point::<i32, Physical>::from((10, 20));
        cache.install(7, key);
        cache.set_tracker(
            key,
            origin,
            OutputDamageTracker::new(
                Size::<i32, Physical>::from((400, 80)),
                Scale::from(1.5),
                Transform::Normal,
            ),
        );
        assert!(!cache.can_skip(key, origin, 1), "no epoch recorded yet");
        cache.set_last_epoch(1);
        assert!(cache.can_skip(key, origin, 1));
        assert!(!cache.can_skip(key, origin, 2), "scene changed");
        assert!(
            !cache.can_skip(key, Point::from((11, 20)), 1),
            "region moved"
        );
        let bigger = SceneTextureKey::new(size(500, 80), output(1920, 1200), 1.5.into());
        assert!(!cache.can_skip(bigger, origin, 1), "region grew");
        // A tracker reset must force a render even at the same epoch.
        cache.set_tracker(
            key,
            origin,
            OutputDamageTracker::new(
                Size::<i32, Physical>::from((400, 80)),
                Scale::from(1.5),
                Transform::Normal,
            ),
        );
        assert!(!cache.can_skip(key, origin, 1));
    }

    #[test]
    fn a_changed_region_element_set_forces_a_full_region_repaint() {
        use smithay::backend::renderer::damage::OutputDamageTracker;
        use smithay::utils::{Point, Transform};

        let mut cache: SceneTextureCache<u32> = SceneTextureCache::default();
        let key = SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.5.into());
        let origin = Point::<i32, Physical>::from((10, 20));
        cache.install(7, key);
        cache.set_tracker(
            key,
            origin,
            OutputDamageTracker::new(
                Size::<i32, Physical>::from((400, 80)),
                Scale::from(1.5),
                Transform::Normal,
            ),
        );
        cache.set_last_epoch(1);
        assert!(cache.can_skip(key, origin, 1));

        // The first frame has no recorded set, then an unchanged set is a
        // no-op. A window leaving the bounded region changes the hash; the
        // caller resets the tracker and the cache can no longer skip.
        assert!(cache.note_region_elements(0xabc));
        assert!(!cache.note_region_elements(0xabc));
        assert!(cache.note_region_elements(0xdef));
        cache.reset_tracker();
        assert!(
            !cache.can_skip(key, origin, 1),
            "a departed element must force a full region repaint"
        );
    }

    #[test]
    fn a_scene_texture_grows_once_and_reuses_the_larger_texture() {
        let mut cache: SceneTextureCache<u32> = SceneTextureCache::default();
        let key = SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.5.into());
        assert!(cache.needs_alloc(key));
        assert_eq!(cache.texture(), None);

        cache.install(7, key);
        assert_eq!(cache.texture(), Some(&7));
        assert_eq!(cache.allocations(), 1);

        // Same key: the steady-state frame allocates nothing.
        for _ in 0..600 {
            assert!(!cache.needs_alloc(key));
        }
        assert_eq!(cache.allocations(), 1);

        // A shrink reuses the larger texture; a Dock sweep does not allocate
        // on the way back down.
        let smaller = SceneTextureKey::new(size(300, 70), output(1920, 1200), 1.5.into());
        assert!(!cache.needs_alloc(smaller));
        assert_eq!(cache.allocations(), 1);

        // A grow allocates once at the new maximum, then reuses it.
        let grown = SceneTextureKey::new(size(500, 90), output(1920, 1200), 1.5.into());
        assert!(cache.needs_alloc(grown));
        cache.install(8, grown);
        assert_eq!(cache.allocations(), 2);
        let shrunk_again = SceneTextureKey::new(size(420, 80), output(1920, 1200), 1.5.into());
        assert!(!cache.needs_alloc(shrunk_again));
        assert_eq!(cache.allocations(), 2);

        // A scale change reallocates even at the same region size.
        let scaled = SceneTextureKey::new(size(500, 90), output(1920, 1200), 2.0.into());
        assert!(cache.needs_alloc(scaled));
        cache.install(9, scaled);
        assert_eq!(cache.allocations(), 3);

        // An output mode change also reallocates.
        let mode = SceneTextureKey::new(size(500, 90), output(2560, 1600), 2.0.into());
        assert!(cache.needs_alloc(mode));
        cache.install(10, mode);
        assert_eq!(cache.allocations(), 4);
    }

    #[test]
    fn a_scene_slot_keeps_its_id_and_commits_only_real_changes() {
        let mut cache: SceneTextureCache<u32> = SceneTextureCache::default();
        let id = cache.id();
        let key = SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.5.into());
        cache.install(1, key);
        assert_eq!(cache.id(), id, "the scene identity survives allocation");

        let commit = cache.commit();
        cache.note_scene_damage(None);
        assert_eq!(
            cache.commit(),
            commit,
            "an unchanged scene must not advance the commit"
        );
        cache.note_scene_damage(Some(vec![rect(0, 0, 10, 10)]));
        assert_ne!(cache.commit(), commit);
        let changed = cache.commit();

        // A reallocation (mode/scale/region grow) keeps the same element id
        // and does not itself advance the scene commit.
        cache.install(2, key);
        assert_eq!(cache.id(), id);
        assert_eq!(cache.commit(), changed);
    }

    #[test]
    fn panel_slots_are_reused_and_trimmed_with_the_panel_set() {
        let mut slots: PanelTextureSlots<u32> = PanelTextureSlots::default();
        let dock = PanelKey::new("dock", 2, 7);
        let menu = PanelKey::new("menubar", 2, 8);

        slots.slot(&dock).textures_mut().push(10);
        slots.slot(&dock).textures_mut().push(11);
        slots.slot(&menu).textures_mut().push(20);
        assert_eq!(slots.slot_count(), 2);
        assert_eq!(slots.slot(&dock).textures(), &[10, 11]);
        assert_eq!(slots.slot(&menu).textures(), &[20]);

        // Re-accessing the same key reuses the slot, not a new one.
        slots.slot(&menu).textures_mut().push(21);
        assert_eq!(slots.slot(&menu).textures(), &[20, 21]);

        // Removing the menu panel drops its textures.
        slots.trim_to(&[dock]);
        assert_eq!(slots.slot_count(), 1);
        assert_eq!(slots.slot(&dock).textures(), &[10, 11]);

        // Removing every panel drops the remaining slot too.
        slots.trim_to(&[]);
        assert_eq!(slots.slot_count(), 0);
    }

    #[test]
    fn a_panel_blur_key_changes_with_scene_source_spec_and_scale() {
        let spec = crate::window::blur::BlurSpec::new(24.0, 10.0);
        let src: Rectangle<f64, Buffer> = Rectangle::new((0.0, 0.0).into(), (10.0, 10.0).into());
        let chain = Size::<i32, Physical>::from((1920, 1080));
        let scale = Scale::from(1.5);
        let base = PanelBlurKey::new(
            1,
            0,
            src,
            sample(spec, 24, RegionFilter::Filtered),
            chain,
            scale,
        );

        assert_eq!(
            base,
            PanelBlurKey::new(
                1,
                0,
                src,
                sample(spec, 24, RegionFilter::Filtered),
                chain,
                scale
            )
        );

        assert_ne!(
            base,
            PanelBlurKey::new(
                1,
                1,
                src,
                sample(spec, 24, RegionFilter::Filtered),
                chain,
                scale
            ),
            "a scene commit must invalidate the cached blur"
        );
        assert_ne!(
            base,
            PanelBlurKey::new(
                2,
                0,
                src,
                sample(spec, 24, RegionFilter::Filtered),
                chain,
                scale
            ),
            "a scene generation change must invalidate the cached blur"
        );
        let moved: Rectangle<f64, Buffer> = Rectangle::new((1.0, 0.0).into(), (10.0, 10.0).into());
        assert_ne!(
            base,
            PanelBlurKey::new(
                1,
                0,
                moved,
                sample(spec, 24, RegionFilter::Filtered),
                chain,
                scale
            ),
            "a panel source move must invalidate the cached blur"
        );
        assert_ne!(
            base,
            PanelBlurKey::new(
                1,
                0,
                src,
                sample(
                    crate::window::blur::BlurSpec::new(12.0, 10.0),
                    12,
                    RegionFilter::Filtered,
                ),
                chain,
                scale,
            ),
            "a blur-spec change must invalidate the cached blur"
        );
        assert_ne!(
            base,
            PanelBlurKey::new(
                1,
                0,
                src,
                sample(spec, 24, RegionFilter::Filtered),
                Size::<i32, Physical>::from((1280, 720)),
                scale,
            ),
            "a chain-size change must invalidate the cached blur"
        );
        assert_ne!(
            base,
            PanelBlurKey::new(
                1,
                0,
                src,
                sample(spec, 24, RegionFilter::Filtered),
                chain,
                Scale::from(2.0),
            ),
            "a scale change must invalidate the cached blur"
        );
    }

    #[test]
    fn a_panel_blur_rebuild_advances_its_damage_commit() {
        let mut slots: PanelTextureSlots<u32> = PanelTextureSlots::default();
        let dock = PanelKey::new("dock", 2, 7);
        let src: Rectangle<f64, Buffer> = Rectangle::new((0.0, 0.0).into(), (10.0, 10.0).into());
        let key = PanelBlurKey::new(
            1,
            0,
            src,
            sample(
                crate::window::blur::BlurSpec::new(24.0, 10.0),
                24,
                RegionFilter::Filtered,
            ),
            Size::<i32, Physical>::from((1920, 1080)),
            Scale::from(1.0),
        );
        let slot = slots.slot(&dock);
        assert!(!slot.blur_cache_hit(&key));
        assert_eq!(slot.commit(), CommitCounter::default());

        slot.textures_mut().push(7);
        slot.set_blur_key(key);
        assert!(slot.blur_cache_hit(&key));
        let first_commit = slot.commit();
        assert_ne!(first_commit, CommitCounter::default());

        // A cache hit does not advance the commit; a rebuild does.
        assert!(slot.blur_cache_hit(&key));
        assert_eq!(slot.commit(), first_commit);
        let mut changed = key;
        changed.scene_generation += 1;
        slot.set_blur_key(changed);
        assert_ne!(slot.commit(), first_commit);
    }

    #[test]
    fn panel_blur_ignores_unrelated_damage_and_rebuilds_for_padding_damage() {
        let mut scene = SceneTextureCache::<u32>::default();
        let mut panel = PanelTextureSlot::new();
        let src = Rectangle::new((80.0, 100.0).into(), (120.0, 40.0).into());
        let spec = BlurSpec::new(8.0, 10.0);
        let chain = output(300, 200);
        let scale = Scale::from(1.5);
        let base = panel.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 12, RegionFilter::Filtered),
            chain,
            scale,
        );
        panel.textures_mut().push(1);
        panel.set_blur_key(base);

        scene.note_scene_damage(Some(vec![rect(0, 0, 300, 20)]));
        let unrelated = panel.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 12, RegionFilter::Filtered),
            chain,
            scale,
        );
        assert!(panel.blur_cache_hit(&unrelated));

        // Only the padding intersects; the top edge of the plate still samples it.
        scene.note_scene_damage(Some(vec![rect(100, 90, 10, 5)]));
        let nearby = panel.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 12, RegionFilter::Filtered),
            chain,
            scale,
        );
        assert_eq!(nearby.miss_reason(&base), Some(BlurMissReason::Scene));
        panel.set_blur_key(nearby);
        let settled = panel.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 12, RegionFilter::Filtered),
            chain,
            scale,
        );
        assert!(panel.blur_cache_hit(&settled));

        // The slot missed two scene commits. The latest damage alone would
        // miss a change inside the panel from the earlier commit.
        scene.note_scene_damage(Some(vec![rect(100, 105, 10, 5)]));
        scene.note_scene_damage(Some(vec![rect(0, 0, 300, 20)]));
        let skipped = panel.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 12, RegionFilter::Filtered),
            chain,
            scale,
        );
        assert_eq!(skipped.miss_reason(&nearby), Some(BlurMissReason::Scene));
    }

    #[test]
    fn a_scene_reallocation_or_tracker_reset_advances_generation() {
        let mut cache: SceneTextureCache<u32> = SceneTextureCache::default();
        let key = SceneTextureKey::new(size(400, 80), output(1920, 1200), 1.0.into());
        assert_eq!(cache.generation(), 0);

        cache.install(1, key);
        assert_eq!(cache.generation(), 1);
        cache.set_tracker(
            key,
            (0, 0).into(),
            OutputDamageTracker::new((400, 80), Scale::from(1.0), Transform::Normal),
        );
        assert_eq!(cache.generation(), 2);
        assert!(cache.tracker_matches(key, (0, 0).into()));
        assert!(!cache.tracker_matches(key, (1, 0).into()));
        assert!(!cache.needs_alloc(key));
    }

    #[test]
    fn region_keys_keep_independent_identities_on_one_surface() {
        let surface = PanelKey::new("control-center", 3, 9);
        let outer = PanelKey::for_region("control-center", 3, 9, "outer");
        let module = PanelKey::for_region("control-center", 3, 9, "module-wifi");
        assert_ne!(outer, module);
        assert_ne!(outer, surface);
        assert_eq!(outer, PanelKey::for_region("control-center", 3, 9, "outer"));
        assert_ne!(
            outer,
            PanelKey::for_region("control-center", 3, 9, "module-wifi")
        );
    }

    #[test]
    fn paint_only_changes_reuse_the_blur_texture_but_damage_the_element() {
        let mut slots: PanelTextureSlots<u32> = PanelTextureSlots::default();
        let key = PanelKey::for_region("control-center", 3, 9, "outer");
        let src = Rectangle::new((0.0, 0.0).into(), (100.0, 50.0).into());
        let spec = BlurSpec::new(24.0, 10.0);
        let blur = PanelBlurKey::new(
            1,
            0,
            src,
            sample(spec, 24, RegionFilter::Filtered),
            Size::from((400, 200)),
            Scale::from(1.0),
        );
        let slot = slots.slot(&key);
        slot.textures_mut().push(7);
        slot.set_blur_key(blur);
        let paint = |tint: f32| {
            PanelPaintKey::new(
                crate::render::RegionPaint {
                    scheme: ColorScheme::Light,
                    preset: Preset::ControlCenterOuter,
                    tier: DegradeTier::Full,
                    mode: RenderMode::SampledGlass,
                    alpha: 0.5,
                    tint,
                    edge: 0.0,
                    edge_width: 0.0,
                    foreground: 0.0,
                    revision: 1,
                },
                CornerRadii::uniform(20),
                3,
            )
        };
        slot.note_paint(paint(0.5));
        let painted = slot.commit();
        // The same paint does not advance the element commit.
        slot.note_paint(paint(0.5));
        assert_eq!(slot.commit(), painted);
        // A tint-only change advances the element commit, but the cached blur
        // key (and therefore the Kawase texture) is still a hit.
        slot.note_paint(paint(0.9));
        assert_ne!(slot.commit(), painted);
        assert!(slot.blur_cache_hit(&blur));

        // A scheme/tier/output-generation change is also paint-only.
        slot.note_paint(PanelPaintKey::new(
            crate::render::RegionPaint {
                scheme: ColorScheme::Dark,
                preset: Preset::ControlCenterOuter,
                tier: DegradeTier::Reduced,
                mode: RenderMode::SampledGlass,
                alpha: 0.5,
                tint: 0.9,
                edge: 0.0,
                edge_width: 0.0,
                foreground: 0.0,
                revision: 2,
            },
            CornerRadii::uniform(20),
            4,
        ));
        assert!(slot.blur_cache_hit(&blur));
    }

    #[test]
    fn declared_overscan_padding_invalidates_on_outer_damage() {
        let mut scene = SceneTextureCache::<u32>::default();
        let mut panel = PanelTextureSlot::new();
        let src = Rectangle::new((80.0, 100.0).into(), (120.0, 40.0).into());
        let spec = BlurSpec::new(8.0, 10.0);
        let chain = output(300, 200);
        let scale = Scale::from(1.0);
        let base = panel.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 8, RegionFilter::Filtered),
            chain,
            scale,
        );
        panel.textures_mut().push(1);
        panel.set_blur_key(base);

        // Damage 20 px outside the source is inside a declared 32 px
        // overscan but outside the blur radius. It must invalidate the cached
        // texture when the region declares the larger footprint.
        scene.note_scene_damage(Some(vec![rect(60, 60, 10, 10)]));
        let wide = panel.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 32, RegionFilter::Filtered),
            chain,
            scale,
        );
        assert_eq!(wide.miss_reason(&base), Some(BlurMissReason::Scene));

        // The same damage with only the blur padding is unrelated.
        let mut narrow = PanelTextureSlot::new();
        narrow.textures_mut().push(2);
        let narrow_base = narrow.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 8, RegionFilter::Filtered),
            chain,
            scale,
        );
        narrow.set_blur_key(narrow_base);
        scene.note_scene_damage(Some(vec![rect(60, 60, 10, 10)]));
        let narrow_after = narrow.blur_key_for_scene(
            &scene,
            src,
            sample(spec, 8, RegionFilter::Filtered),
            chain,
            scale,
        );
        assert!(narrow.blur_cache_hit(&narrow_after));
    }

    #[test]
    fn a_sharp_region_never_reuses_a_filtered_blur_key() {
        let src = Rectangle::new((0.0, 0.0).into(), (10.0, 10.0).into());
        let spec = BlurSpec::new(8.0, 10.0);
        let chain = output(300, 200);
        let filtered = PanelBlurKey::new(
            1,
            0,
            src,
            sample(spec, 8, RegionFilter::Filtered),
            chain,
            Scale::from(1.0),
        );
        let sharp = PanelBlurKey::new(
            1,
            0,
            src,
            sample(spec, 8, RegionFilter::Sharp),
            chain,
            Scale::from(1.0),
        );
        assert_eq!(sharp.miss_reason(&filtered), Some(BlurMissReason::Spec));
    }

    #[test]
    fn panel_ids_are_stable_per_key_and_independent_of_order() {
        let mut slots: PanelTextureSlots<u32> = PanelTextureSlots::default();
        let dock = PanelKey::new("dock", 2, 7);
        let menu = PanelKey::new("menubar", 2, 8);
        let dock_id = slots.slot(&dock).id().clone();
        let menu_id = slots.slot(&menu).id().clone();
        assert_ne!(dock_id, menu_id);

        // Reordering the panel list does not change either identity.
        slots.trim_to(&[menu, dock]);
        assert_eq!(*slots.slot(&dock).id(), dock_id);
        assert_eq!(*slots.slot(&menu).id(), menu_id);

        // A panel that disappears and comes back is a new element: the old
        // slot (and its texture handles) was released.
        slots.trim_to(&[dock]);
        assert_ne!(*slots.slot(&menu).id(), menu_id);
    }

    #[test]
    fn trimming_a_panel_slot_releases_its_texture_handles() {
        let mut slots: PanelTextureSlots<Rc<()>> = PanelTextureSlots::default();
        let dock = PanelKey::new("dock", 2, 7);
        let texture = Rc::new(());
        let weak = Rc::downgrade(&texture);
        slots.slot(&dock).textures_mut().push(texture);
        assert!(weak.upgrade().is_some());

        slots.trim_to(&[]);
        assert!(
            weak.upgrade().is_none(),
            "the removed panel's texture must not stay alive in the cache"
        );
    }

    #[test]
    fn the_allocation_counter_is_monotonic() {
        let mut slots: PanelTextureSlots<u32> = PanelTextureSlots::default();
        assert_eq!(slots.allocations(), 0);
        slots.record_alloc();
        slots.record_alloc();
        assert_eq!(slots.allocations(), 2);
    }

    /// A minimal element that mirrors the material elements' damage contract:
    /// stable id, explicit commit, explicit geometry.
    #[derive(Clone)]
    struct TestElement {
        id: Id,
        commit: CommitCounter,
        geometry: Rectangle<i32, Physical>,
    }

    impl TestElement {
        fn new(id: Id, commit: CommitCounter, geometry: Rectangle<i32, Physical>) -> Self {
            Self {
                id,
                commit,
                geometry,
            }
        }
    }

    impl Element for TestElement {
        fn id(&self) -> &Id {
            &self.id
        }

        fn current_commit(&self) -> CommitCounter {
            self.commit
        }

        fn src(&self) -> Rectangle<f64, Buffer> {
            Rectangle::from_size(Size::from((
                self.geometry.size.w as f64,
                self.geometry.size.h as f64,
            )))
        }

        fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
            self.geometry
        }
    }

    fn damage(
        tracker: &mut OutputDamageTracker,
        elements: &[TestElement],
    ) -> Option<Vec<Rectangle<i32, Physical>>> {
        tracker
            .damage_output(1, elements)
            .expect("the tracker has a mode")
            .0
            .cloned()
    }

    #[test]
    fn a_stationary_panel_with_an_unchanged_scene_produces_no_damage() {
        let output = output(1920, 1080);
        let mut tracker = OutputDamageTracker::new(output, Scale::from(1.0), Transform::Normal);
        let mut slots: PanelTextureSlots<()> = PanelTextureSlots::default();
        let dock_key = PanelKey::new("dock", 2, 41);
        let dock_id = slots.slot(&dock_key).id().clone();
        let dock_rect = rect(600, 1000, 720, 80);

        let mut wallpaper = TestElement::new(
            Id::new(),
            CommitCounter::default(),
            Rectangle::from_size(output),
        );
        let dock = TestElement::new(dock_id.clone(), CommitCounter::default(), dock_rect);

        // The first frame introduces both elements, so the output is damaged.
        let first = damage(&mut tracker, &[wallpaper.clone(), dock.clone()]).expect("first frame");
        assert!(first.contains(&Rectangle::from_size(output)));

        // Same ids, geometry, and commits: a stationary Dock over an unchanged
        // wallpaper produces no damage at all (not even the panel rect).
        assert_eq!(
            damage(&mut tracker, &[wallpaper.clone(), dock.clone()]),
            None
        );

        // A real scene change (a new wallpaper commit on the same stable id)
        // damages the full output, because the scene element covers it.
        wallpaper.commit.increment();
        let changed =
            damage(&mut tracker, &[wallpaper.clone(), dock.clone()]).expect("wallpaper change");
        assert!(changed.contains(&Rectangle::from_size(output)));

        // Settled again: no damage while the wallpaper and Dock are stationary.
        assert_eq!(
            damage(&mut tracker, &[wallpaper.clone(), dock.clone()]),
            None
        );

        // A rebuilt panel blur advances the panel commit even when its
        // geometry did not move. That must damage the panel, not the full
        // output: this is the F5 stale-edge guard.
        let mut rebuilt = dock.clone();
        rebuilt.commit.increment();
        let panel_damage =
            damage(&mut tracker, &[wallpaper.clone(), rebuilt]).expect("panel rebuild damage");
        assert!(
            !panel_damage.contains(&Rectangle::from_size(output)),
            "a panel rebuild must not damage the full output: {panel_damage:?}"
        );
        for rect in &panel_damage {
            assert!(
                rect.intersection(dock_rect).is_some(),
                "panel rebuild damage outside the Dock: {rect:?}"
            );
        }

        // Moving the Dock damages only the old and new panel rects; the
        // full-output wallpaper element stays untouched.
        let moved = TestElement::new(
            dock_id,
            dock.commit,
            Rectangle::new(Point::from((700, 1000)), Size::from((720, 80))),
        );
        let moved_damage =
            damage(&mut tracker, &[wallpaper, moved]).expect("the Dock move is damage");
        assert!(
            !moved_damage.contains(&Rectangle::from_size(output)),
            "a Dock move must never damage the full output: {moved_damage:?}"
        );
        for rect in &moved_damage {
            assert!(
                rect.loc.y >= dock_rect.loc.y,
                "damage outside the Dock band: {rect:?}"
            );
        }
        assert!(moved_damage
            .iter()
            .any(|rect| rect.intersection(dock_rect).is_some()));
    }

    #[test]
    fn a_sharp_veil_commits_on_scene_damage_but_stays_settled_otherwise() {
        let mut slots: PanelTextureSlots<u32> = PanelTextureSlots::default();
        let key = PanelKey::new("menubar", 2, 11);
        let slot = slots.slot(&key);
        let scene = CommitCounter::default();
        // The first sample always damages (the element is new).
        slot.note_sharp_scene_commit(scene);
        let first = slot.commit();
        // The same scene commit is settled: no repeated veil damage.
        slot.note_sharp_scene_commit(scene);
        assert_eq!(slot.commit(), first);
        // Real scene damage advances the veil element's commit.
        let mut changed = scene;
        changed.increment();
        slot.note_sharp_scene_commit(changed);
        assert_ne!(slot.commit(), first);
        let damaged = slot.commit();
        slot.note_sharp_scene_commit(changed);
        assert_eq!(slot.commit(), damaged);
    }
}
