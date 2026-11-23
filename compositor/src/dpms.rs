// SPDX-License-Identifier: MIT
#![allow(dead_code)] // The pure API is also exercised by the lib unit tests.
//! Per-output DPMS state (T316).
//!
//! DPMS is a property of one connector, not a compositor-wide mode: one
//! monitor can blank while another stays on. The pure [`DpmsModel`] owns the
//! requested on/off state and the honest fallback for a backend that cannot
//! apply it. The DRM rail supplies the apply closure (the connector's `DPMS`
//! property); nested and headless keep the state and report their fallback
//! ([ADR 0313](../../docs/design/adr/0313-idle-dpms-ownership.md)).
//!
//! A failed hardware write is reported, not fatal: the previous state stays
//! live and [`DpmsFallback::Failed`] tells the shell that the panel did not
//! change. A later request (or the resume path) retries.

use std::collections::HashMap;

/// The backend that owns the panel power for this session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpmsBackend {
    /// The DRM/KMS rail: apply the connector's `DPMS` property.
    Drm,
    /// The nested development window: keep the state, no host panel control.
    Nested,
    /// The headless rail: keep the state, no panel exists.
    Headless,
}

impl DpmsBackend {
    /// The stable name used in logs and the hand-off.
    pub const fn name(self) -> &'static str {
        match self {
            DpmsBackend::Drm => "drm",
            DpmsBackend::Nested => "nested",
            DpmsBackend::Headless => "headless",
        }
    }

    /// The fallback a successfully applied state reports on this backend.
    pub const fn fallback(self) -> DpmsFallback {
        match self {
            DpmsBackend::Drm => DpmsFallback::None,
            DpmsBackend::Nested => DpmsFallback::Nested,
            DpmsBackend::Headless => DpmsFallback::Headless,
        }
    }
}

/// Why the panel state may not be applied by hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpmsFallback {
    /// The connector property was applied (DRM success).
    None,
    /// The nested window has no host panel control.
    Nested,
    /// The headless rail has no panel.
    Headless,
    /// A DRM connector property write failed; the previous state is unchanged.
    Failed,
}

impl DpmsFallback {
    /// The stable wire value in `df_output.dpms.fallback`.
    pub const fn wire(self) -> u32 {
        match self {
            DpmsFallback::None => 0,
            DpmsFallback::Nested => 1,
            DpmsFallback::Headless => 2,
            DpmsFallback::Failed => 3,
        }
    }

    /// The stable name used in logs.
    pub const fn name(self) -> &'static str {
        match self {
            DpmsFallback::None => "none",
            DpmsFallback::Nested => "nested",
            DpmsFallback::Headless => "headless",
            DpmsFallback::Failed => "failed",
        }
    }
}

/// The reported state of one output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DpmsReport {
    /// Whether the panel is on after the request.
    pub on: bool,
    /// How the state was reached, or why it was not.
    pub fallback: DpmsFallback,
}

impl DpmsReport {
    /// A state that is on with no fallback (the default for a DRM output).
    pub const fn on() -> Self {
        DpmsReport {
            on: true,
            fallback: DpmsFallback::None,
        }
    }
}

/// The result of one state request: the reported state plus the hardware
/// error, when a write failed. The caller logs the error; the report still
/// reaches the shell so the UI can show the fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpmsOutcome {
    pub report: DpmsReport,
    pub error: Option<String>,
}

/// The per-output DPMS state for the session.
#[derive(Debug, Clone)]
pub struct DpmsModel {
    backend: DpmsBackend,
    /// Outputs whose panel is off. Absent means on.
    off: HashMap<String, ()>,
    /// The last fallback per output, so a failed write can be retried and
    /// reported without guessing. Absent uses [`DpmsBackend::fallback`].
    fallbacks: HashMap<String, DpmsFallback>,
}

impl DpmsModel {
    /// Start with every output on and the backend's default fallback.
    pub fn new(backend: DpmsBackend) -> Self {
        DpmsModel {
            backend,
            off: HashMap::new(),
            fallbacks: HashMap::new(),
        }
    }

    /// Set the backend once at startup. Resets the reported fallback because
    /// no hardware request has happened yet.
    pub fn set_backend(&mut self, backend: DpmsBackend) {
        self.backend = backend;
        self.fallbacks.clear();
    }

    /// The active backend.
    pub const fn backend(&self) -> DpmsBackend {
        self.backend
    }

    /// Whether `name` is currently blanked.
    pub fn is_off(&self, name: &str) -> bool {
        self.off.contains_key(name)
    }

    /// The current report for `name`.
    pub fn report(&self, name: &str) -> DpmsReport {
        DpmsReport {
            on: !self.is_off(name),
            fallback: self
                .fallbacks
                .get(name)
                .copied()
                .unwrap_or_else(|| self.backend.fallback()),
        }
    }

    /// Apply one on/off request for `name`.
    ///
    /// `apply` is called only on the DRM rail, and only when the requested
    /// state differs from the live one or the previous write failed. Nested
    /// and headless never call it: they keep the state and report the named
    /// fallback. A DRM error leaves the previous state live and records
    /// [`DpmsFallback::Failed`].
    pub fn set<F>(&mut self, name: &str, on: bool, apply: F) -> DpmsOutcome
    where
        F: FnOnce(bool) -> Result<(), String>,
    {
        let current = self.report(name);
        if current.on == on && current.fallback != DpmsFallback::Failed {
            return DpmsOutcome {
                report: current,
                error: None,
            };
        }

        match self.backend {
            DpmsBackend::Drm => match apply(on) {
                Ok(()) => {
                    self.store(name, on, DpmsFallback::None);
                    DpmsOutcome {
                        report: self.report(name),
                        error: None,
                    }
                }
                Err(error) => {
                    // The previous on/off state stays live; only the report
                    // changes, so the shell can show the failure honestly.
                    self.fallbacks
                        .insert(name.to_string(), DpmsFallback::Failed);
                    DpmsOutcome {
                        report: self.report(name),
                        error: Some(error),
                    }
                }
            },
            backend => {
                self.store(name, on, backend.fallback());
                DpmsOutcome {
                    report: self.report(name),
                    error: None,
                }
            }
        }
    }

    /// Forget an output that was unplugged.
    pub fn forget(&mut self, name: &str) {
        self.off.remove(name);
        self.fallbacks.remove(name);
    }

    fn store(&mut self, name: &str, on: bool, fallback: DpmsFallback) {
        if on {
            self.off.remove(name);
        } else {
            self.off.insert(name.to_string(), ());
        }
        self.fallbacks.insert(name.to_string(), fallback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drm_request_applies_the_connector_state() {
        let mut model = DpmsModel::new(DpmsBackend::Drm);
        assert_eq!(model.report("DP-1"), DpmsReport::on());

        let mut applied = Vec::new();
        let outcome = model.set("DP-1", false, |on| {
            applied.push(on);
            Ok(())
        });
        assert_eq!(
            outcome,
            DpmsOutcome {
                report: DpmsReport {
                    on: false,
                    fallback: DpmsFallback::None
                },
                error: None
            }
        );
        assert_eq!(applied, [false]);
        assert!(model.is_off("DP-1"));
        assert!(!model.is_off("HDMI-A-1"), "state is per output");
    }

    #[test]
    fn nested_and_headless_keep_the_state_without_a_hardware_write() {
        for (backend, fallback) in [
            (DpmsBackend::Nested, DpmsFallback::Nested),
            (DpmsBackend::Headless, DpmsFallback::Headless),
        ] {
            let mut model = DpmsModel::new(backend);
            let outcome = model.set("OUT-1", false, |_| panic!("must not apply on {backend:?}"));
            assert_eq!(
                outcome.report,
                DpmsReport {
                    on: false,
                    fallback
                }
            );
            assert!(model.is_off("OUT-1"));
            assert_eq!(outcome.report.fallback.name(), fallback.name());
        }
    }

    #[test]
    fn a_failed_write_keeps_the_previous_state_and_reports_it() {
        let mut model = DpmsModel::new(DpmsBackend::Drm);
        let outcome = model.set("DP-1", false, |_| Err("EINVAL".to_string()));
        assert_eq!(
            outcome.report,
            DpmsReport {
                on: true,
                fallback: DpmsFallback::Failed
            },
            "a failed blank must not claim the panel is off"
        );
        assert_eq!(outcome.error.as_deref(), Some("EINVAL"));
        assert!(!model.is_off("DP-1"));

        // The next request retries even though the on/off value is unchanged.
        let outcome = model.set("DP-1", false, |_| Ok(()));
        assert_eq!(
            outcome.report,
            DpmsReport {
                on: false,
                fallback: DpmsFallback::None
            }
        );
        assert!(model.is_off("DP-1"));
    }

    #[test]
    fn an_unchanged_state_does_not_write_again() {
        let mut model = DpmsModel::new(DpmsBackend::Drm);
        let outcome = model.set("DP-1", true, |_| panic!("already on"));
        assert_eq!(outcome.report, DpmsReport::on());
        assert_eq!(outcome.error, None);
    }

    #[test]
    fn forgetting_an_output_returns_it_to_the_backend_default() {
        let mut model = DpmsModel::new(DpmsBackend::Drm);
        model.set("DP-1", false, |_| Ok(()));
        assert!(model.is_off("DP-1"));
        model.forget("DP-1");
        assert!(!model.is_off("DP-1"));
        assert_eq!(model.report("DP-1"), DpmsReport::on());
    }

    #[test]
    fn the_wire_values_match_the_protocol_documentation() {
        assert_eq!(DpmsFallback::None.wire(), 0);
        assert_eq!(DpmsFallback::Nested.wire(), 1);
        assert_eq!(DpmsFallback::Headless.wire(), 2);
        assert_eq!(DpmsFallback::Failed.wire(), 3);
    }
}
