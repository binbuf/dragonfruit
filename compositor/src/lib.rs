// SPDX-License-Identifier: MIT
//! The compositor's reusable library surface.
//!
//! Most of the compositor lives in the `dragonfruit-compositor` binary. This
//! library exists so the pure policy modules can be unit- and conformance-
//! tested from `compositor/tests/` without spawning the binary. Right now it
//! exposes the XDnD bridge model ([`xdnd`]), the DRM bring-up classification
//! ([`drm_bringup`]), the hardware input validation matrix
//! ([`input_validation`]), and the multi-GPU import/fallback classification
//! ([`multi_gpu`]); the T-14.5 conformance test drives the first's codec
//! against a real Xwayland server, and the T-03.2/T-03.3/T-03.4 tests pin the
//! others' decisions without hardware.

pub mod drm_bringup;
pub mod input_validation;
pub mod multi_gpu;
pub mod xdnd;
