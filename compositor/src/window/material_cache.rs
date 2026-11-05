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
use smithay::utils::{Buffer, Physical, Scale, Size};

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
    /// The damage tracker for the offscreen scene texture. It is reused while
    /// the region texture key is unchanged, so an unchanged scene renders
    /// nothing; a reallocation or region-size/scale change resets it and
    /// forces one full region render.
    tracker: Option<(SceneTextureKey, OutputDamageTracker)>,
}

impl<T> Default for SceneTextureCache<T> {
    fn default() -> Self {
        Self {
            texture: None,
            key: None,
            allocations: 0,
            id: Id::new(),
            commit: CommitCounter::default(),
            tracker: None,
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

    /// The scene element's commit, advanced by [`Self::note_scene_change`].
    pub(crate) fn commit(&self) -> CommitCounter {
        self.commit
    }

    /// Advance the scene commit when the offscreen scene changed this frame.
    ///
    /// A frame that rendered no damage leaves the commit untouched, so a
    /// stationary scene contributes no output damage.
    pub(crate) fn note_scene_change(&mut self, changed: bool) {
        if changed {
            self.commit.increment();
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
        self.allocations += 1;
    }

    /// Lifetime allocation count (test/diagnostic evidence).
    pub(crate) fn allocations(&self) -> u64 {
        self.allocations
    }

    /// Whether the cached tracker was built for `key`.
    pub(crate) fn tracker_matches(&self, key: SceneTextureKey) -> bool {
        self.tracker
            .as_ref()
            .is_some_and(|(tracker_key, _)| *tracker_key == key)
    }

    /// Install the scene damage tracker for `key`.
    pub(crate) fn set_tracker(&mut self, key: SceneTextureKey, tracker: OutputDamageTracker) {
        self.tracker = Some((key, tracker));
    }

    /// The scene damage tracker, if one has been installed.
    pub(crate) fn tracker_mut(&mut self) -> Option<&mut OutputDamageTracker> {
        self.tracker.as_mut().map(|(_, tracker)| tracker)
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
}

/// One panel's reusable Kawase textures plus its stable render-element id.
#[derive(Debug)]
pub(crate) struct PanelTextureSlot<T> {
    id: Id,
    textures: Vec<T>,
}

impl<T> PanelTextureSlot<T> {
    fn new() -> Self {
        Self {
            id: Id::new(),
            textures: Vec::new(),
        }
    }

    /// The panel's stable render-element id.
    pub(crate) fn id(&self) -> &Id {
        &self.id
    }

    /// The Kawase-chain textures for this panel.
    pub(crate) fn textures_mut(&mut self) -> &mut Vec<T> {
        &mut self.textures
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
        cache.note_scene_change(false);
        assert_eq!(
            cache.commit(),
            commit,
            "an unchanged scene must not advance the commit"
        );
        cache.note_scene_change(true);
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
}
