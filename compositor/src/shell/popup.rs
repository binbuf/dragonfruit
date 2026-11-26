// SPDX-License-Identifier: MIT
//! Popup presentation transactions (T354).
//!
//! A popup used to be a set of independent layer requests: placement,
//! material regions and input were staged separately, and a late GPU frame or
//! a transport switch could commit after the popup had retargeted or closed.
//! The transaction joins those requests with one generation and one content
//! sequence, latches them on the same `wl_surface.commit` as the pixels, and
//! owns the retained visual snapshot after a logical close.
//!
//! The state machine is pure so the stale/superseded rules, the closing
//! snapshot, the input policy and the bounded release lifetime are unit-tested
//! without a compositor. The protocol layer fills the committed geometry and
//! material coverage at commit time.

use smithay::utils::{Logical, Rectangle};

/// Upper bound on a client-declared retained snapshot lifetime. A shell may
/// ask for a longer visual tail, but a bug or a hostile trusted client must
/// not pin a full-size buffer indefinitely.
pub const MAX_POPUP_RETAIN_MS: u32 = 5_000;

/// The popup transaction lifecycle edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupPhase {
    Open,
    Retarget,
    Closing,
    Complete,
    Cancel,
}

impl PopupPhase {
    pub fn from_wire(value: u32) -> Self {
        match value {
            1 => PopupPhase::Retarget,
            2 => PopupPhase::Closing,
            3 => PopupPhase::Complete,
            4 => PopupPhase::Cancel,
            _ => PopupPhase::Open,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PopupPhase::Open => "open",
            PopupPhase::Retarget => "retarget",
            PopupPhase::Closing => "closing",
            PopupPhase::Complete => "complete",
            PopupPhase::Cancel => "cancel",
        }
    }

    /// Whether this phase keeps the accepted visual snapshot alive.
    pub fn retains(self) -> bool {
        matches!(self, PopupPhase::Closing | PopupPhase::Complete)
    }
}

/// The content transport a client declares for a popup frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupTransport {
    None,
    Shm,
    Dmabuf,
}

impl PopupTransport {
    pub fn from_wire(value: u32) -> Self {
        match value {
            1 => PopupTransport::Shm,
            2 => PopupTransport::Dmabuf,
            _ => PopupTransport::None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PopupTransport::None => "none",
            PopupTransport::Shm => "shm",
            PopupTransport::Dmabuf => "dmabuf",
        }
    }
}

/// One commit-latched popup transaction request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupRequest {
    pub generation: u32,
    pub content_serial: u32,
    pub phase: PopupPhase,
    pub transport: PopupTransport,
    pub input_active: bool,
    pub retain_ms: u32,
}

/// The accepted popup snapshot: everything a compositor animation needs to
/// outlive the input owner, joined at the commit that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupSnapshot {
    pub generation: u32,
    pub content_serial: u32,
    pub phase: PopupPhase,
    /// The transport the client declared for this generation.
    pub declared_transport: PopupTransport,
    /// The transport the committed buffer actually used. A mismatch is the
    /// documented shm fallback (or a transport switch), not a stale frame.
    pub actual_transport: PopupTransport,
    pub input_active: bool,
    /// The owning output, or `None` for the per-output overlay rule.
    pub output: Option<String>,
    pub geometry: Rectangle<i32, Logical>,
    pub paint_extent: Rectangle<i32, Logical>,
    pub material_regions: usize,
    pub accepted_at_ms: u128,
    /// Absolute deadline for the retained visual; 0 while no close started.
    pub retained_until_ms: u128,
}

impl PopupSnapshot {
    /// Whether this snapshot is still inside its declared visual lifetime.
    pub fn retained(&self, now_ms: u128) -> bool {
        self.phase.retains() && self.retained_until_ms > now_ms
    }
}

/// Why a request did not replace the accepted snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupReject {
    /// An older generation or an older content serial.
    Stale,
    /// A new lifecycle edge for a generation that already completed or
    /// cancelled, or a geometry retarget during the closing tail.
    Superseded,
    /// A retarget/close with no open generation.
    NotOpen,
}

impl PopupReject {
    pub fn name(self) -> &'static str {
        match self {
            PopupReject::Stale => "stale",
            PopupReject::Superseded => "superseded",
            PopupReject::NotOpen => "not-open",
        }
    }
}

/// The result of applying one transaction request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupOutcome {
    Accepted,
    Rejected(PopupReject),
}

impl PopupOutcome {
    pub fn accepted(self) -> bool {
        matches!(self, PopupOutcome::Accepted)
    }
}

/// A released retained snapshot, for the trace and for resource accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupRelease {
    pub generation: u32,
    pub content_serial: u32,
    pub reason: &'static str,
}

/// One chrome surface's popup transaction owner.
#[derive(Debug, Default)]
pub struct PopupTransaction {
    current: Option<PopupSnapshot>,
    last_release: Option<PopupRelease>,
}

impl PopupTransaction {
    /// The accepted snapshot, if a transaction is open or retained.
    pub fn snapshot(&self) -> Option<&PopupSnapshot> {
        self.current.as_ref()
    }

    /// Whether this surface has ever entered a transaction. A legacy surface
    /// (no request) keeps the pre-T354 whole-surface input policy.
    pub fn active(&self) -> bool {
        self.current.is_some()
    }

    /// The input policy the compositor enforces for this surface.
    ///
    /// A legacy client has no transaction and keeps the surface input region
    /// as its only policy. A transaction makes the policy explicit and drops
    /// it at the first closing edge, independently of the retained visual.
    pub fn input_active(&self) -> bool {
        self.current
            .as_ref()
            .map_or(true, |snapshot| snapshot.input_active)
    }

    /// Whether the retained visual is still valid at `now_ms`.
    pub fn retained(&self, now_ms: u128) -> bool {
        self.current
            .as_ref()
            .is_some_and(|snapshot| snapshot.retained(now_ms))
    }

    /// The last released snapshot, consumed by the protocol trace.
    pub fn take_release(&mut self) -> Option<PopupRelease> {
        self.last_release.take()
    }

    /// Apply one commit-latched request.
    ///
    /// `actual_transport` is the committed buffer's transport (`None` for a
    /// transaction-only commit). The caller fills geometry/material coverage
    /// with [`Self::set_committed`] after an accepted update.
    pub fn apply(
        &mut self,
        request: PopupRequest,
        actual_transport: PopupTransport,
        now_ms: u128,
    ) -> PopupOutcome {
        match request.phase {
            PopupPhase::Open => self.open(request, actual_transport, now_ms),
            PopupPhase::Retarget => self.retarget(request, actual_transport),
            PopupPhase::Closing => self.closing(request, actual_transport, now_ms),
            PopupPhase::Complete => self.complete(request, actual_transport, now_ms),
            PopupPhase::Cancel => self.cancel(request),
        }
    }

    fn open(
        &mut self,
        request: PopupRequest,
        actual_transport: PopupTransport,
        now_ms: u128,
    ) -> PopupOutcome {
        if let Some(current) = self.current.as_ref() {
            if request.generation <= current.generation {
                return PopupOutcome::Rejected(if request.generation < current.generation {
                    PopupReject::Stale
                } else {
                    PopupReject::Superseded
                });
            }
            // A reopen supersedes the retained snapshot immediately; the new
            // generation owns the next visual lifetime.
            self.release("superseded");
        }
        self.current = Some(PopupSnapshot {
            generation: request.generation,
            content_serial: request.content_serial,
            phase: PopupPhase::Open,
            declared_transport: request.transport,
            actual_transport,
            input_active: request.input_active,
            output: None,
            geometry: Rectangle::default(),
            paint_extent: Rectangle::default(),
            material_regions: 0,
            accepted_at_ms: now_ms,
            retained_until_ms: 0,
        });
        PopupOutcome::Accepted
    }

    fn retarget(
        &mut self,
        request: PopupRequest,
        actual_transport: PopupTransport,
    ) -> PopupOutcome {
        let Some(current) = self.current.as_mut() else {
            return PopupOutcome::Rejected(PopupReject::NotOpen);
        };
        if request.generation < current.generation {
            return PopupOutcome::Rejected(PopupReject::Stale);
        }
        if matches!(current.phase, PopupPhase::Complete | PopupPhase::Cancel) {
            return PopupOutcome::Rejected(PopupReject::Superseded);
        }
        if request.generation == current.generation {
            if current.phase == PopupPhase::Closing {
                // The closing snapshot is frozen: a late retarget must not
                // move or resize the visual the compositor is animating.
                return PopupOutcome::Rejected(PopupReject::Superseded);
            }
            if request.content_serial <= current.content_serial {
                return PopupOutcome::Rejected(PopupReject::Stale);
            }
        }
        current.generation = request.generation;
        current.content_serial = request.content_serial;
        current.phase = PopupPhase::Retarget;
        current.declared_transport = request.transport;
        current.actual_transport = actual_transport;
        current.input_active = request.input_active;
        PopupOutcome::Accepted
    }

    fn closing(
        &mut self,
        request: PopupRequest,
        actual_transport: PopupTransport,
        now_ms: u128,
    ) -> PopupOutcome {
        let Some(current) = self.current.as_mut() else {
            return PopupOutcome::Rejected(PopupReject::NotOpen);
        };
        if request.generation < current.generation {
            return PopupOutcome::Rejected(PopupReject::Stale);
        }
        if matches!(current.phase, PopupPhase::Complete | PopupPhase::Cancel) {
            return PopupOutcome::Rejected(PopupReject::Superseded);
        }
        if request.generation == current.generation
            && request.content_serial < current.content_serial
        {
            return PopupOutcome::Rejected(PopupReject::Stale);
        }
        // The first closing edge starts the bounded lifetime. Closing frames
        // may update the content inside that window; they must not extend it.
        let starts_lifetime =
            current.phase != PopupPhase::Closing || request.generation > current.generation;
        current.generation = request.generation;
        current.content_serial = request.content_serial.max(current.content_serial);
        current.phase = PopupPhase::Closing;
        current.declared_transport = request.transport;
        current.actual_transport = actual_transport;
        current.input_active = false;
        if starts_lifetime {
            current.retained_until_ms =
                now_ms + u128::from(request.retain_ms.min(MAX_POPUP_RETAIN_MS));
        }
        PopupOutcome::Accepted
    }

    fn complete(
        &mut self,
        request: PopupRequest,
        actual_transport: PopupTransport,
        now_ms: u128,
    ) -> PopupOutcome {
        let Some(current) = self.current.as_mut() else {
            return PopupOutcome::Rejected(PopupReject::NotOpen);
        };
        if request.generation < current.generation {
            return PopupOutcome::Rejected(PopupReject::Stale);
        }
        if current.phase == PopupPhase::Cancel {
            return PopupOutcome::Rejected(PopupReject::Superseded);
        }
        current.generation = request.generation;
        current.content_serial = request.content_serial.max(current.content_serial);
        current.phase = PopupPhase::Complete;
        current.declared_transport = request.transport;
        current.actual_transport = actual_transport;
        current.input_active = false;
        if current.retained_until_ms <= now_ms {
            self.release("complete");
        }
        PopupOutcome::Accepted
    }

    fn cancel(&mut self, request: PopupRequest) -> PopupOutcome {
        let Some(current) = self.current.as_ref() else {
            return PopupOutcome::Rejected(PopupReject::NotOpen);
        };
        if request.generation < current.generation {
            return PopupOutcome::Rejected(PopupReject::Stale);
        }
        self.release("cancel");
        PopupOutcome::Accepted
    }

    /// Record the committed geometry, owning output and material coverage for
    /// the accepted generation.
    pub fn set_committed(
        &mut self,
        output: Option<String>,
        geometry: Rectangle<i32, Logical>,
        paint_extent: Rectangle<i32, Logical>,
        material_regions: usize,
    ) {
        if let Some(current) = self.current.as_mut() {
            current.output = output;
            current.geometry = geometry;
            current.paint_extent = paint_extent;
            current.material_regions = material_regions;
        }
    }

    /// Release the snapshot and remember why. Returns whether anything was
    /// actually released.
    pub fn release(&mut self, reason: &'static str) -> bool {
        let Some(current) = self.current.take() else {
            return false;
        };
        self.last_release = Some(PopupRelease {
            generation: current.generation,
            content_serial: current.content_serial,
            reason,
        });
        true
    }

    /// Release the snapshot when `output_name` is detached. An all-output
    /// overlay (`output == None`) is conservatively released too: it may have
    /// been shown on the removed display by the per-output overlay rule.
    pub fn release_for_output(&mut self, output_name: &str) -> bool {
        let Some(current) = self.current.as_ref() else {
            return false;
        };
        if current
            .output
            .as_deref()
            .is_some_and(|name| name != output_name)
        {
            return false;
        }
        self.release("output")
    }

    /// Release a snapshot whose declared lifetime has elapsed.
    pub fn expire(&mut self, now_ms: u128) -> bool {
        let expired = self.current.as_ref().is_some_and(|current| {
            current.phase.retains()
                && current.retained_until_ms > 0
                && now_ms >= current.retained_until_ms
        });
        if expired {
            self.release("expired")
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn request(
        generation: u32,
        content_serial: u32,
        phase: PopupPhase,
        transport: PopupTransport,
        input_active: bool,
        retain_ms: u32,
    ) -> PopupRequest {
        PopupRequest {
            generation,
            content_serial,
            phase,
            transport,
            input_active,
            retain_ms,
        }
    }

    fn open(transaction: &mut PopupTransaction, generation: u32, serial: u32, now: u128) {
        let outcome = transaction.apply(
            request(
                generation,
                serial,
                PopupPhase::Open,
                PopupTransport::Shm,
                true,
                0,
            ),
            PopupTransport::Shm,
            now,
        );
        assert!(outcome.accepted(), "{outcome:?}");
        transaction.set_committed(
            Some("eDP-1".into()),
            Rectangle::new(Point::from((10, 20)), Size::from((200, 300))),
            Rectangle::new(Point::from((10, 20)), Size::from((220, 320))),
            2,
        );
    }

    #[test]
    fn stale_generation_and_serial_are_rejected() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 4, 10, 1_000);

        let stale = transaction.apply(
            request(3, 11, PopupPhase::Retarget, PopupTransport::Shm, true, 0),
            PopupTransport::Shm,
            1_001,
        );
        assert_eq!(stale, PopupOutcome::Rejected(PopupReject::Stale));

        let old_serial = transaction.apply(
            request(4, 9, PopupPhase::Retarget, PopupTransport::Shm, true, 0),
            PopupTransport::Shm,
            1_002,
        );
        assert_eq!(old_serial, PopupOutcome::Rejected(PopupReject::Stale));

        let newer = transaction.apply(
            request(4, 12, PopupPhase::Retarget, PopupTransport::Shm, true, 0),
            PopupTransport::Shm,
            1_003,
        );
        assert_eq!(newer, PopupOutcome::Accepted);
        assert_eq!(transaction.snapshot().unwrap().content_serial, 12);
    }

    #[test]
    fn a_superseded_reopen_releases_the_previous_snapshot() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 1, 1_000);
        transaction.apply(
            request(2, 2, PopupPhase::Open, PopupTransport::Dmabuf, true, 0),
            PopupTransport::Dmabuf,
            1_010,
        );
        assert_eq!(
            transaction.take_release(),
            Some(PopupRelease {
                generation: 1,
                content_serial: 1,
                reason: "superseded",
            })
        );
        assert_eq!(transaction.snapshot().unwrap().generation, 2);
        assert_eq!(
            transaction.snapshot().unwrap().actual_transport,
            PopupTransport::Dmabuf
        );
    }

    #[test]
    fn closing_releases_input_but_retains_the_snapshot() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 7, 20, 1_000);
        assert!(transaction.input_active());

        let closing = transaction.apply(
            request(8, 20, PopupPhase::Closing, PopupTransport::Shm, false, 400),
            PopupTransport::Shm,
            1_100,
        );
        assert_eq!(closing, PopupOutcome::Accepted);
        assert!(!transaction.input_active());
        assert!(transaction.retained(1_100));
        assert!(transaction.retained(1_499));
        assert!(!transaction.retained(1_500));
        assert_eq!(transaction.snapshot().unwrap().retained_until_ms, 1_500);
    }

    #[test]
    fn closing_frames_do_not_extend_the_declared_lifetime() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 1, 0);
        transaction.apply(
            request(2, 2, PopupPhase::Closing, PopupTransport::Shm, false, 300),
            PopupTransport::Shm,
            100,
        );
        transaction.apply(
            request(2, 3, PopupPhase::Closing, PopupTransport::Shm, false, 300),
            PopupTransport::Shm,
            250,
        );
        assert_eq!(transaction.snapshot().unwrap().retained_until_ms, 400);
        assert_eq!(transaction.snapshot().unwrap().content_serial, 3);
    }

    #[test]
    fn a_late_retarget_cannot_move_the_closing_snapshot() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 1, 0);
        transaction.apply(
            request(2, 2, PopupPhase::Closing, PopupTransport::Shm, false, 300),
            PopupTransport::Shm,
            100,
        );
        let late = transaction.apply(
            request(2, 3, PopupPhase::Retarget, PopupTransport::Shm, true, 0),
            PopupTransport::Shm,
            200,
        );
        assert_eq!(late, PopupOutcome::Rejected(PopupReject::Superseded));
        assert!(!transaction.input_active());
    }

    #[test]
    fn complete_holds_until_the_deadline_then_expires() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 1, 0);
        transaction.apply(
            request(2, 2, PopupPhase::Closing, PopupTransport::Shm, false, 300),
            PopupTransport::Shm,
            100,
        );
        let complete = transaction.apply(
            request(2, 2, PopupPhase::Complete, PopupTransport::Shm, false, 0),
            PopupTransport::Shm,
            200,
        );
        assert_eq!(complete, PopupOutcome::Accepted);
        assert!(transaction.retained(399));
        assert!(!transaction.expire(399));
        assert!(transaction.expire(400));
        assert_eq!(
            transaction.take_release(),
            Some(PopupRelease {
                generation: 2,
                content_serial: 2,
                reason: "expired",
            })
        );
    }

    #[test]
    fn cancel_releases_immediately() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 1, 0);
        let cancel = transaction.apply(
            request(1, 1, PopupPhase::Cancel, PopupTransport::None, false, 0),
            PopupTransport::None,
            10,
        );
        assert_eq!(cancel, PopupOutcome::Accepted);
        assert!(transaction.snapshot().is_none());
        assert_eq!(transaction.take_release().unwrap().reason, "cancel");
    }

    #[test]
    fn output_removal_releases_only_the_owning_snapshot() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 1, 0);
        assert!(!transaction.release_for_output("HDMI-A-1"));
        assert!(transaction.snapshot().is_some());
        assert!(transaction.release_for_output("eDP-1"));
        assert!(transaction.snapshot().is_none());
    }

    #[test]
    fn a_transport_switch_is_accepted_but_an_older_serial_is_not() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 10, 0);
        let fallback = transaction.apply(
            request(1, 11, PopupPhase::Retarget, PopupTransport::Dmabuf, true, 0),
            PopupTransport::Shm,
            20,
        );
        assert_eq!(fallback, PopupOutcome::Accepted);
        let snapshot = transaction.snapshot().unwrap();
        assert_eq!(snapshot.declared_transport, PopupTransport::Dmabuf);
        assert_eq!(snapshot.actual_transport, PopupTransport::Shm);

        let late = transaction.apply(
            request(1, 10, PopupPhase::Retarget, PopupTransport::Dmabuf, true, 0),
            PopupTransport::Dmabuf,
            30,
        );
        assert_eq!(late, PopupOutcome::Rejected(PopupReject::Stale));
    }

    #[test]
    fn a_legacy_surface_keeps_input_and_has_no_retained_snapshot() {
        let transaction = PopupTransaction::default();
        assert!(transaction.input_active());
        assert!(!transaction.active());
        assert!(!transaction.retained(0));
        assert!(transaction.snapshot().is_none());
    }

    #[test]
    fn the_retained_lifetime_is_clamped() {
        let mut transaction = PopupTransaction::default();
        open(&mut transaction, 1, 1, 0);
        transaction.apply(
            request(
                2,
                2,
                PopupPhase::Closing,
                PopupTransport::Shm,
                false,
                u32::MAX,
            ),
            PopupTransport::Shm,
            0,
        );
        assert_eq!(
            transaction.snapshot().unwrap().retained_until_ms,
            u128::from(MAX_POPUP_RETAIN_MS)
        );
    }
}
