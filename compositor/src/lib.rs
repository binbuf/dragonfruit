// SPDX-License-Identifier: MIT
//! The compositor's reusable library surface.
//!
//! Most of the compositor lives in the `dragonfruit-compositor` binary. This
//! library exists so the pure policy modules can be unit- and conformance-
//! tested from `compositor/tests/` without spawning the binary. Right now it
//! exposes the XDnD bridge model ([`xdnd`]) and the DRM bring-up
//! classification ([`drm_bringup`]); the T-14.5 conformance test drives the
//! former's codec against a real Xwayland server, and the T-03.2 test pins
//! the latter's open/ready decision without hardware.

pub mod drm_bringup;
pub mod xdnd;
