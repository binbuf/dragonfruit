// SPDX-License-Identifier: MIT
//! The session plan: what the session manager starts, in what order, and how
//! each process is supervised (T-12.1a).
//!
//! A [`SessionPlan`] groups [`ServiceSpec`]s into stages. Stage 0 starts
//! first; a later stage starts once every service in the previous stage is
//! up — and, for a `gate` service, once it has signalled readiness (the
//! compositor's private socket, wired by T-12.1b). The compositor is the
//! anchor: the session ends when it exits. Every other service is restarted
//! in place per its [`RestartPolicy`], exactly as
//! [11-session-and-dev-workflow.md](../../docs/design/11-session-and-dev-workflow.md#session-composition-and-supervision)
//! describes.

use std::collections::HashSet;
use std::path::PathBuf;

/// What to do when a supervised service exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Restart on every exit, clean or not.
    Always,
    /// Restart only when the exit was a failure (a non-zero status or a
    /// signal). A clean `0` exit stays exited.
    OnFailure,
    /// Never restart; the service stays exited.
    Never,
}

impl RestartPolicy {
    /// The spelling used in the default plan and on the command line.
    pub const fn as_str(self) -> &'static str {
        match self {
            RestartPolicy::Always => "always",
            RestartPolicy::OnFailure => "on-failure",
            RestartPolicy::Never => "never",
        }
    }

    /// Parse `always` / `on-failure` / `never`, case-insensitively.
    pub fn parse(text: &str) -> Option<RestartPolicy> {
        match text.to_ascii_lowercase().as_str() {
            "always" => Some(RestartPolicy::Always),
            "on-failure" | "onfailure" | "on_failure" => Some(RestartPolicy::OnFailure),
            "never" => Some(RestartPolicy::Never),
            _ => None,
        }
    }
}

/// One supervised process: what to run, when to start it, and how to restart
/// it.
#[derive(Debug, Clone)]
pub struct ServiceSpec {
    /// Stable, unique name (the session's handle for the process).
    pub name: String,
    /// The program to exec. A bare name is resolved through `PATH` by
    /// [`std::process::Command`].
    pub program: PathBuf,
    /// Arguments passed unchanged.
    pub args: Vec<String>,
    /// Extra environment for the child (T-12.1b owns the session environment;
    /// this is the seam). Empty today.
    pub env: Vec<(String, String)>,
    /// What to do when the process exits.
    pub policy: RestartPolicy,
    /// Launch stage: lower starts first. All services in a stage start
    /// together.
    pub stage: u32,
    /// The session ends when this service exits and is not restarted. Exactly
    /// one service (the compositor) sets it, and it must use
    /// [`RestartPolicy::Never`].
    pub ends_session: bool,
    /// Do not start the next stage until this service has been marked ready.
    /// The compositor sets it; T-12.1b signals it when the private socket
    /// exists.
    pub gate: bool,
}

impl ServiceSpec {
    /// A service with the default policy (`on-failure`), stage 0, and no
    /// gate or anchor flag.
    pub fn new(name: impl Into<String>, program: impl Into<PathBuf>) -> Self {
        ServiceSpec {
            name: name.into(),
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            policy: RestartPolicy::OnFailure,
            stage: 0,
            ends_session: false,
            gate: false,
        }
    }

    /// Set the argument list.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Set the restart policy.
    pub fn policy(mut self, policy: RestartPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set the launch stage.
    pub fn stage(mut self, stage: u32) -> Self {
        self.stage = stage;
        self
    }

    /// Mark this service as the session anchor.
    pub fn ends_session(mut self, ends_session: bool) -> Self {
        self.ends_session = ends_session;
        self
    }

    /// Require this service to be ready before the next stage starts.
    pub fn gate(mut self, gate: bool) -> Self {
        self.gate = gate;
        self
    }

    /// Add one environment variable for the child.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// The full command line, for the plan listing.
    pub fn command_line(&self) -> String {
        let mut line = self.program.display().to_string();
        for arg in &self.args {
            line.push(' ');
            line.push_str(arg);
        }
        line
    }
}

/// A whole session composition.
#[derive(Debug, Clone, Default)]
pub struct SessionPlan {
    pub services: Vec<ServiceSpec>,
}

impl SessionPlan {
    /// Wrap an ordered service list.
    pub fn new(services: Vec<ServiceSpec>) -> Self {
        SessionPlan { services }
    }

    /// The composition from the design doc: compositor first; shell and the
    /// user services in parallel once its private socket exists; the portal
    /// last. The compositor ends the session when it exits; everything else
    /// restarts in place.
    pub fn default_session() -> SessionPlan {
        SessionPlan::new(vec![
            ServiceSpec::new("compositor", "dragonfruit-compositor")
                .stage(0)
                .policy(RestartPolicy::Never)
                .ends_session(true)
                .gate(true),
            ServiceSpec::new("shell", "dragonfruit-shell")
                .stage(1)
                .policy(RestartPolicy::Always),
            ServiceSpec::new("settingsd", "dragonfruit-settingsd").stage(1),
            ServiceSpec::new("menu-broker", "dragonfruit-menu-broker").stage(1),
            ServiceSpec::new("app-index", "dragonfruit-app-index").stage(1),
            ServiceSpec::new("notifications", "dragonfruit-notifications").stage(1),
            ServiceSpec::new("portal", "xdg-desktop-portal-dragonfruit").stage(2),
        ])
    }

    /// The distinct stages, ascending.
    pub fn stages(&self) -> Vec<u32> {
        let mut stages: Vec<u32> = self.services.iter().map(|spec| spec.stage).collect();
        stages.sort_unstable();
        stages.dedup();
        stages
    }

    /// The services in one stage, in plan order.
    pub fn services_in_stage(&self, stage: u32) -> impl Iterator<Item = &ServiceSpec> {
        self.services.iter().filter(move |spec| spec.stage == stage)
    }

    /// The single session anchor, if the plan has one.
    pub fn anchor(&self) -> Option<&ServiceSpec> {
        self.services.iter().find(|spec| spec.ends_session)
    }

    /// The plan invariants the supervisor relies on.
    pub fn validate(&self) -> Result<(), PlanError> {
        if self.services.is_empty() {
            return Err(PlanError::Empty);
        }
        let mut seen: HashSet<&str> = HashSet::new();
        for spec in &self.services {
            if !seen.insert(spec.name.as_str()) {
                return Err(PlanError::DuplicateName(spec.name.clone()));
            }
            if spec.ends_session && spec.policy != RestartPolicy::Never {
                return Err(PlanError::RestartableSessionAnchor(spec.name.clone()));
            }
        }
        let anchors: Vec<String> = self
            .services
            .iter()
            .filter(|spec| spec.ends_session)
            .map(|spec| spec.name.clone())
            .collect();
        if anchors.len() > 1 {
            return Err(PlanError::MultipleSessionAnchors(anchors));
        }
        Ok(())
    }
}

/// A plan that breaks an invariant the supervisor needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// A plan with no services is not a session.
    Empty,
    /// Two services share a name; the supervisor's handles would be ambiguous.
    DuplicateName(String),
    /// More than one service ends the session.
    MultipleSessionAnchors(Vec<String>),
    /// The session anchor must not restart: a restarted anchor cannot end the
    /// session.
    RestartableSessionAnchor(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::Empty => write!(f, "a session plan must have at least one service"),
            PlanError::DuplicateName(name) => write!(f, "duplicate service name {name:?}"),
            PlanError::MultipleSessionAnchors(names) => {
                write!(f, "more than one session anchor: {names:?}")
            }
            PlanError::RestartableSessionAnchor(name) => {
                write!(f, "session anchor {name:?} must use RestartPolicy::Never")
            }
        }
    }
}

impl std::error::Error for PlanError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_session_matches_the_design_composition() {
        let plan = SessionPlan::default_session();
        plan.validate().expect("the default plan is valid");

        let stages = plan.stages();
        assert_eq!(stages, vec![0, 1, 2]);

        let anchor = plan.anchor().expect("the compositor is the anchor");
        assert_eq!(anchor.name, "compositor");
        assert_eq!(anchor.stage, 0);
        assert_eq!(anchor.policy, RestartPolicy::Never);
        assert!(anchor.gate, "the shell waits for the compositor's socket");

        let shell = plan
            .services
            .iter()
            .find(|spec| spec.name == "shell")
            .unwrap();
        assert_eq!(shell.policy, RestartPolicy::Always);

        // Everything in stage 1 except the shell restarts on failure.
        for spec in plan.services_in_stage(1) {
            if spec.name != "shell" {
                assert_eq!(spec.policy, RestartPolicy::OnFailure, "{}", spec.name);
            }
        }

        // The portal registers with xdg-desktop-portal last.
        assert_eq!(
            plan.services_in_stage(2)
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            vec!["portal"]
        );
    }

    #[test]
    fn policy_round_trips_through_its_spelling() {
        for policy in [
            RestartPolicy::Always,
            RestartPolicy::OnFailure,
            RestartPolicy::Never,
        ] {
            assert_eq!(RestartPolicy::parse(policy.as_str()), Some(policy));
        }
        assert_eq!(
            RestartPolicy::parse("OnFailure"),
            Some(RestartPolicy::OnFailure)
        );
        assert_eq!(RestartPolicy::parse("nonsense"), None);
    }

    #[test]
    fn validate_rejects_a_restartable_anchor_and_duplicate_names() {
        let restartable = SessionPlan::new(vec![ServiceSpec::new("compositor", "x")
            .policy(RestartPolicy::Always)
            .ends_session(true)]);
        assert_eq!(
            restartable.validate(),
            Err(PlanError::RestartableSessionAnchor("compositor".into()))
        );

        let duplicate =
            SessionPlan::new(vec![ServiceSpec::new("a", "x"), ServiceSpec::new("a", "y")]);
        assert_eq!(
            duplicate.validate(),
            Err(PlanError::DuplicateName("a".into()))
        );

        let two_anchors = SessionPlan::new(vec![
            ServiceSpec::new("a", "x")
                .policy(RestartPolicy::Never)
                .ends_session(true),
            ServiceSpec::new("b", "y")
                .policy(RestartPolicy::Never)
                .ends_session(true),
        ]);
        assert!(matches!(
            two_anchors.validate(),
            Err(PlanError::MultipleSessionAnchors(_))
        ));

        assert_eq!(SessionPlan::default().validate(), Err(PlanError::Empty));
    }
}
