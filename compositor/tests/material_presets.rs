// SPDX-License-Identifier: MIT
//! T195 acceptance: the generated optical presets resolve consistently.
//!
//! The heavy cases live next to the resolver; this integration test pins the
//! contract the compositor gate runs: variant precedence, the three measured
//! light anchors, the relative detached/status response, unchanged unmeasured
//! slider roles, distinct dark attached/detached presets, the veil/unsupported
//! distinction, and the safe default rollout.

use dragonfruit_compositor::material_preset::{
    resolve, safe_solid, Accessibility, Backend, Preset, RenderMode, ResolveInput, Rollout, Scheme,
    Tier,
};

fn replacement(preset: Preset, scheme: Scheme) -> ResolveInput {
    ResolveInput {
        preset,
        scheme,
        rollout: Rollout::Replacement,
        ..ResolveInput::default()
    }
}

#[test]
fn light_slider_anchors_are_measured_and_piecewise() {
    for (position, alpha, tint) in [
        (0.0, 0.732, 0.953),
        (0.5255, 0.552, 0.993),
        (1.0, 0.415, 0.997),
    ] {
        let resolved = resolve(ResolveInput {
            slider: position,
            ..replacement(Preset::MenuAttached, Scheme::Light)
        });
        assert!((resolved.alpha - alpha).abs() < 1e-6);
        assert!((resolved.tint - tint).abs() < 1e-6);
        assert!(resolved.slider_applied);
    }
    let quarter = resolve(ResolveInput {
        slider: 0.25,
        ..replacement(Preset::MenuAttached, Scheme::Light)
    });
    assert!(quarter.alpha > 0.552 && quarter.alpha < 0.732);
}

#[test]
fn slider_does_not_move_unmeasured_roles() {
    for preset in Preset::ALL {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let low = resolve(ResolveInput {
                slider: 0.0,
                ..replacement(preset, scheme)
            });
            let high = resolve(ResolveInput {
                slider: 1.0,
                ..replacement(preset, scheme)
            });
            let direct = preset == Preset::MenuAttached && scheme == Scheme::Light;
            let relative = matches!(preset, Preset::MenuDetached | Preset::AppsDrawer | Preset::StatusMenu)
                && scheme == Scheme::Light;
            assert_eq!(
                low.slider_applied,
                direct || relative,
                "{} {scheme:?}",
                preset.name()
            );
            if !direct && !relative {
                assert_eq!((low.alpha, low.tint), (high.alpha, high.tint));
            }
        }
    }
}

#[test]
fn light_detached_and_status_follow_the_attached_curve_relatively() {
    for preset in [Preset::MenuDetached, Preset::StatusMenu] {
        let default = resolve(replacement(preset, Scheme::Light));
        let explicit = resolve(ResolveInput {
            slider: dragonfruit_compositor::design_tokens::glass::slider::DEFAULT_POSITION,
            ..replacement(preset, Scheme::Light)
        });
        assert!((default.alpha - explicit.alpha).abs() < 1e-6);
        assert!((default.tint - explicit.tint).abs() < 1e-6);
        let low = resolve(ResolveInput {
            slider: 0.0,
            ..replacement(preset, Scheme::Light)
        });
        let high = resolve(ResolveInput {
            slider: 1.0,
            ..replacement(preset, Scheme::Light)
        });
        assert!(low.slider_applied && high.slider_applied);
        assert!(low.alpha > default.alpha && default.alpha > high.alpha);
        assert!(low.tint < default.tint && default.tint < high.tint);
    }
}

#[test]
fn dark_attached_and_detached_are_distinct() {
    let attached = resolve(replacement(Preset::MenuAttached, Scheme::Dark));
    let detached = resolve(replacement(Preset::MenuDetached, Scheme::Dark));
    assert_ne!((attached.alpha, attached.tint), (detached.alpha, detached.tint));
}

#[test]
fn accessibility_precedence_is_explicit() {
    let reduced = resolve(ResolveInput {
        slider: 0.0,
        accessibility: Accessibility {
            reduce_transparency: true,
            increase_contrast: true,
        },
        ..replacement(Preset::MenuAttached, Scheme::Light)
    });
    assert_eq!(reduced.render_mode, RenderMode::Solid);
    assert!(!reduced.slider_applied);
    assert_eq!(reduced.blur, 0.0);
    assert!(reduced.alpha >= 0.9);
    assert!(reduced.edge > 0.0);
    assert!(reduced.foreground > 0.0);
}

#[test]
fn veil_is_explicit_and_unsupported_sampled_paths_are_safe() {
    for backend in [
        Backend::GpuSampled,
        Backend::GpuNoSampledBlur,
        Backend::Software,
    ] {
        let veil = resolve(ResolveInput {
            backend,
            ..replacement(Preset::MenuBarVeil, Scheme::Light)
        });
        assert_eq!(veil.render_mode, RenderMode::NoBlurVeil);
        assert_eq!(veil.fallback_reason, "none");
    }
    for (backend, mode) in [
        (Backend::GpuNoSampledBlur, RenderMode::Solid),
        (Backend::Software, RenderMode::SoftwareFallback),
    ] {
        let resolved = resolve(ResolveInput {
            backend,
            ..replacement(Preset::MenuDetached, Scheme::Dark)
        });
        assert_eq!(resolved.render_mode, mode);
        assert!(resolved.alpha >= 0.9);
        assert_eq!(resolved.blur, 0.0);
    }
    assert_eq!(safe_solid(Scheme::Light, "unknown-preset").render_mode, RenderMode::Solid);
}

#[test]
fn notification_materials_are_three_distinct_presets() {
    // T205: the measured Light banner/stack/lower fits and the Dark stack.
    for (preset, scheme, alpha, tint) in [
        (Preset::NotificationBanner, Scheme::Light, 0.549, 0.976),
        (Preset::NotificationStack, Scheme::Light, 0.551, 0.706),
        (Preset::NotificationStack, Scheme::Dark, 0.507, 0.550),
        (Preset::NotificationLowerPanel, Scheme::Light, 0.407, 0.852),
    ] {
        let resolved = resolve(replacement(preset, scheme));
        assert!((resolved.alpha - alpha).abs() < 1e-6, "{}", preset.name());
        assert!((resolved.tint - tint).abs() < 1e-6, "{}", preset.name());
        assert_eq!(resolved.render_mode, RenderMode::SampledGlass);
    }
    // The uncovered Dark banner/lower panel are explicit adaptations, not
    // invented measured fits.
    for preset in [Preset::NotificationBanner, Preset::NotificationLowerPanel] {
        let resolved = resolve(replacement(preset, Scheme::Dark));
        assert_eq!(resolved.confidence, "adapted", "{}", preset.name());
    }
    // All three are independently distinguishable over the mid backdrop.
    for scheme in [Scheme::Light, Scheme::Dark] {
        let mut outputs = Vec::new();
        for preset in [
            Preset::NotificationBanner,
            Preset::NotificationStack,
            Preset::NotificationLowerPanel,
        ] {
            let resolved = resolve(replacement(preset, scheme));
            outputs.push(0.502 * (1.0 - resolved.alpha) + resolved.tint * resolved.alpha);
        }
        let spread = outputs.iter().cloned().fold(f32::MIN, f32::max)
            - outputs.iter().cloned().fold(f32::MAX, f32::min);
        assert!(spread > 0.02, "{scheme:?} materials collapsed: {outputs:?}");
    }
}

#[test]
fn rollout_defaults_to_the_replacement_appearance() {
    assert_eq!(Rollout::default(), Rollout::Replacement);
    let replacement = resolve(ResolveInput {
        preset: Preset::Dock,
        ..ResolveInput::default()
    });
    assert_eq!(replacement.look, Rollout::Replacement);
    assert_eq!(replacement.render_mode, RenderMode::SampledGlass);
    let legacy = resolve(ResolveInput {
        preset: Preset::Dock,
        rollout: Rollout::Legacy,
        ..ResolveInput::default()
    });
    assert_eq!(legacy.look, Rollout::Legacy);
    assert_eq!(legacy.render_mode, RenderMode::SampledGlass);
    assert_eq!(Rollout::from_name("bogus"), None);
}

#[test]
fn persistent_roles_keep_the_sampled_floor() {
    let dock = resolve(ResolveInput {
        tier: Tier::Minimal,
        ..replacement(Preset::Dock, Scheme::Light)
    });
    assert_eq!(dock.render_mode, RenderMode::SampledGlass);
    let popup = resolve(ResolveInput {
        tier: Tier::Minimal,
        ..replacement(Preset::MenuDetached, Scheme::Light)
    });
    assert_eq!(popup.render_mode, RenderMode::Solid);
}
