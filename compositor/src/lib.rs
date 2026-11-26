// SPDX-License-Identifier: GPL-3.0-or-later
//! The compositor's reusable library surface.
//!
//! Most of the compositor lives in the `dragonfruit-compositor` binary. This
//! library exists so the pure policy modules can be unit- and conformance-
//! tested from `compositor/tests/` without spawning the binary. Right now it
//! exposes the XDnD bridge model ([`xdnd`]) and its X11 target connection
//! ([`xdnd_bridge`]), the DRM bring-up classification
//! ([`drm_bringup`]), the hardware input validation matrix
//! ([`input_validation`]), the output scoping for popups and absolute input
//! ([`output_input`]), the multi-GPU import/fallback classification
//! ([`multi_gpu`]), the per-output refresh model ([`refresh`]), the pure
//! per-output presentation clock ([`presentation`]), the pure per-output
//! frame scheduler ([`frame_scheduler`]), and the clock-injected idle timer
//! ([`idle`]); the T-14.5 conformance test drives the first's codec against a
//! real Xwayland server, and the T-03.2/T-03.3/T-03.4/T-21.2/T315 tests pin
//! the others' decisions without hardware. The live ScreenCast producer
//! ([`screencast_producer`]) is here too: its node-id parser and frame
//! packets are tested without a PipeWire host.

pub mod design_tokens;
pub mod dpms;
pub mod drm_bringup;
pub mod frame_scheduler;
pub mod idle;
pub mod input_validation;
pub mod material_preset;
pub mod multi_gpu;
pub mod output_input;
pub mod pointer_policy;
pub mod presentation;
pub mod presentation_trace;
pub mod refresh;
pub mod screencast_producer;
pub mod window_menu;
pub mod xdnd;
pub mod xdnd_bridge;
