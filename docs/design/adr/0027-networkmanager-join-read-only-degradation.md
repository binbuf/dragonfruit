# 0027 — NetworkManager joins degrade to read-only on polkit denial, at the adapter

## Status

accepted

## Context

T-07.2b adds the NetworkManager write path: join/activate a network. Joining is
authorized by polkit; a desktop running without an authorization agent (or
where the user is not permitted) gets a refusal instead of an activation. The
design names this as a risk — "if unavailable, degrade to read-only and record
it" ([07-system-integration.md](../07-system-integration.md)) — but the adapter
contract (ADR [0024](0024-system-adapter-contract.md), [0025](0025-adapter-subscription-lifecycle.md))
only knows the read states `Available`/`Unavailable`/`Error`. A refused write is
none of those: the daemon is present, the read path still works, and only the
write is blocked.

## Decision

- **A denied write is its own outcome, not a read error.** The transport seam
  answers an activation with `ActivateOutcome::{Accepted, Denied, Absent,
  Failed}`; `Denied` is kept distinct from `Failed` so the adapter can react to
  authorization specifically. The live D-Bus source classifies the error name
  (`org.freedesktop.NetworkManager.PermissionDenied`,
  `org.freedesktop.DBus.Error.AccessDenied`, `PolicyKit1.NotAuthorized`) and a
  message fallback ("not authorized"/"permission denied"/"polkit").
- **The degradation lives on the adapter, not in the snapshot.** A denial flips
  `NetworkManagerAdapter` to `WifiAccess::ReadOnly { note }`. `WifiSnapshot` is
  the daemon's state and stays untouched, so the network list keeps rendering.
  The read-only `StatusSlot` remains visible **and enabled**: read-only means
  the list is still usable, only the join affordance is off. The denial note is
  queryable (`access()`, `degradation_note()`) and survives a later `refresh`,
  because a successful read does not re-grant a write permission.
- **A degraded adapter refuses further joins locally.** It does not re-send a
  request polkit will refuse again, so a denied user cannot drive a request
  loop.
- **The write path stays behind the same seam.** `NetworkManagerSource::activate`
  is the only write, mockable with `MockNetworkManager::deny_joins`; no consumer
  sees zbus or an `NM*` type (ADR [0026](0026-concrete-adapters-in-their-own-crates.md)).

Rejected: flipping the adapter into `Error` on denial (would hide or deaden the
live network list, contradicting "read-only"); putting `read_only` into
`WifiSnapshot` (confuses session authorization with daemon state, and a refresh
would reset it); a generic write-permission field on the shared
`Adapter`/`StatusSlot` (premature — only networking has a polkit-gated write
today; revisit if audio/power gain one).

## Consequences

- T-07.5a builds the Wi-Fi menu from the concrete adapter: it disables the join
  rows when `access()` is read-only and can surface `degradation_note()`.
- T-07.3/T-07.4 (audio/power) are unaffected unless they gain a gated write, at
  which point the read-only concept should be lifted into the contract.
- The live D-Bus write path is compile-checked only in CI (no bus); the tested
  contract is the fixture-driven mock seam.