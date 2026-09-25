// SPDX-License-Identifier: MIT
//! `dragonfruit-session`: the session manager (T-12.1a).
//!
//! The desktop is a *session*: a fixed composition of processes with a
//! defined launch order and a restart policy
//! ([11-session-and-dev-workflow.md](../../docs/design/11-session-and-dev-workflow.md#session-composition-and-supervision)).
//! This crate owns that composition:
//!
//! * [`plan`] — [`ServiceSpec`], [`RestartPolicy`], and the [`SessionPlan`]
//!   that describes the default session (compositor first, everything else in
//!   parallel once its socket exists, the portal last).
//! * [`supervisor`] — [`Supervisor`], which spawns the services stage by
//!   stage, reaps exited children, and restarts them per policy.
//!
//! The compositor is the anchor: its exit ends the session and stops
//! everything else; it is never restarted. T-12.1b wires the environment,
//! the systemd user units, and the second-VT workflow onto this seam; T-12.2
//! adds the display-manager entry and logout teardown.

pub mod plan;
pub mod supervisor;

pub use plan::{PlanError, RestartPolicy, ServiceSpec, SessionPlan};
pub use supervisor::{ExitOutcome, ServiceState, SessionState, Supervisor, SupervisorEvent};

/// Planned well-known name on the user session bus
/// (docs/ipc-versioning.md).
pub const DBUS_NAME: &str = "org.dragonfruit.Session1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_well_known_name_follows_the_versioning_policy() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME), "{DBUS_NAME}");
        assert_eq!(DBUS_NAME, df_ipc::dbus_name("Session", 1).to_string());
    }
}
