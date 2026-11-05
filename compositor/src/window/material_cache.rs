// SPDX-License-Identifier: MIT
//! Reusable texture bookkeeping for the material pass (T-21.3).
//!
//! The scene texture and every panel's Kawase chain are allocated once and
//! reused. The cache key is deliberately narrow: a new texture is required
//! only when the region's buffer size, the output mode, or the output scale
//! changes. A moving panel that keeps the same padded-union size keeps the
//! same texture; its content is re-rendered every frame, so no stale pixels
//! survive a region-origin move.
//!
//! This module is renderer-free: the generic slot/cache types are asserted
//! headless, and the backends supply `GlesTexture` as the stored type.

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
#[derive(Debug)]
pub(crate) struct SceneTextureCache<T> {
    texture: Option<T>,
    key: Option<SceneTextureKey>,
    allocations: u64,
}

impl<T> Default for SceneTextureCache<T> {
    fn default() -> Self {
        Self {
            texture: None,
            key: None,
            allocations: 0,
        }
    }
}

impl<T> SceneTextureCache<T> {
    pub(crate) fn texture(&self) -> Option<&T> {
        self.texture.as_ref()
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
    pub(crate) fn install(&mut self, texture: T, key: SceneTextureKey) {
        self.texture = Some(texture);
        self.key = Some(key);
        self.allocations += 1;
    }

    /// Lifetime allocation count (test/diagnostic evidence).
    pub(crate) fn allocations(&self) -> u64 {
        self.allocations
    }
}

/// One Kawase-chain slot per chrome panel.
///
/// A panel's slot is reused across frames. When the panel set shrinks, the
/// removed slots are dropped by [`Self::trim`], which releases their
/// `GlesTexture` handles instead of keeping them alive for the rest of the
/// session. Slots are indexed by panel order; T190c replaces that with a
/// stable panel identity.
#[derive(Debug)]
pub(crate) struct PanelTextureSlots<T> {
    panels: Vec<Vec<T>>,
    allocations: u64,
}

impl<T> Default for PanelTextureSlots<T> {
    fn default() -> Self {
        Self {
            panels: Vec::new(),
            allocations: 0,
        }
    }
}

impl<T> PanelTextureSlots<T> {
    pub(crate) fn slot(&mut self, index: usize) -> &mut Vec<T> {
        if self.panels.len() <= index {
            self.panels.resize_with(index + 1, Vec::new);
        }
        &mut self.panels[index]
    }

    /// Drop every slot at or after `count`.
    pub(crate) fn trim(&mut self, count: usize) {
        self.panels.truncate(count);
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

    fn size(w: i32, h: i32) -> Size<i32, Buffer> {
        Size::from((w, h))
    }

    fn output(w: i32, h: i32) -> Size<i32, Physical> {
        Size::from((w, h))
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
    fn panel_slots_are_reused_and_trimmed_with_the_panel_set() {
        let mut slots: PanelTextureSlots<u32> = PanelTextureSlots::default();
        slots.slot(0).push(10);
        slots.slot(0).push(11);
        slots.slot(1).push(20);
        assert_eq!(slots.slot_count(), 2);
        assert_eq!(slots.slot(0), &vec![10, 11]);
        assert_eq!(slots.slot(1), &vec![20]);

        // Re-accessing the same index reuses the slot, not a new one.
        slots.slot(1).push(21);
        assert_eq!(slots.slot(1), &vec![20, 21]);

        // Removing the second panel drops its textures.
        slots.trim(1);
        assert_eq!(slots.slot_count(), 1);
        assert_eq!(slots.slot(0), &vec![10, 11]);

        // Removing every panel drops the remaining slot too.
        slots.trim(0);
        assert_eq!(slots.slot_count(), 0);
    }

    #[test]
    fn trimming_a_panel_slot_releases_its_texture_handles() {
        let mut slots: PanelTextureSlots<Rc<()>> = PanelTextureSlots::default();
        let texture = Rc::new(());
        let weak = Rc::downgrade(&texture);
        slots.slot(0).push(texture);
        assert!(weak.upgrade().is_some());

        slots.trim(0);
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
}
