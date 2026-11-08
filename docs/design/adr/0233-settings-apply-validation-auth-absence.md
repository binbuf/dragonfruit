# 0233 — One Settings apply path, schema validation, polkit auth, and five absence kinds

## Status

accepted

## Context

T222/T223 gave every Settings pane a map row and a searchable anatomy, but a
control still called `Settings.set()` with no preflight validation and no
failure surface: a rejected write could leave a toggle claiming success, and an
account action ran without an authorization step. Absence was a collection of
ad-hoc notes per pane, so `hidden`, `read-only`, and `delegated` had no shared
meaning. T224 has to make apply, validate, revert, authorize, and degrade
honest without adding a second settings daemon or an app password field.

## Decision

- **One write path in the shared client.** `SettingsClient::beginWrite()` calls
  `validate()`, records the previous value and a monotonic generation, marks the
  key `pending`, and applies the value optimistically. `completeWrite()` either
  confirms it or reverts to the previous value and records the owner's message;
  a stale reply from an older generation is ignored, so a fast slider cannot
  clobber a newer value. `SettingsBridge.apply()`/`set()` expose the same path
  to QML and return `false` when the value was rejected before any change.
- **Validation mirrors the owner's schema.** `settingsSchemaDefaults()` remains
  the complete C++ mirror of `services/settingsd/src/schema.rs`;
  `settingsSchemaInfo()` adds the type, default, numeric range, and exhaustive
  enum for a key. The client rejects unknown keys, wrong types, out-of-range
  numbers, and values outside an enum before they leave QML. settingsd remains
  the final validator; the client table only prevents a known-bad write and
  gives the row an immediate message.
- **The error is part of the row.** `SettingsRow` gained `errorText`;
  `SettingsKeyRow` binds it to `Settings.writeErrors[key]`. A failed write
  reverts the bound control and shows the reason under the label, and a later
  owner change clears the stale message.
- **Privileged actions use the system authority.** `SettingsAuthorizer` calls
  polkit `CheckAuthorization` with `AllowUserInteraction`; the shell's
  registered polkit agent renders the prompt and owns the PAM conversation.
  The Settings app never sees or renders a password field. Every Users & Groups
  write is authorized first; a denial, cancellation, wrong password, or absent
  authority leaves the view untouched and publishes `Settings.accountsError`
  plus `authorizationFinished`.
- **Five absence kinds, one policy.** `SettingsAbsence` is the pure policy
  table: `live-on-defaults` keeps controls visible and writable, `read-only`
  and `disabled-with-reason` keep them visible but inert, `hidden` and
  `delegated` remove the control (delegated offers an explicit hand-off).
  `SettingsAbsenceNotice` renders the one-line reason/action. Panes keep their
  own control visibility/enabling because a pane can mix a live settingsd half
  with an absent adapter half.
- **Restart resync keeps ADR 0032's contract.** A write made while settingsd is
  away is in-memory for the session only and is reported as `deferred`; when
  the daemon returns, `GetAll` is authoritative, the while-down value is
  reconciled away, and the deferred state clears. The other controls keep
  their values and the pane does not lose its live half.

## Consequences

- T225–T229 should use `SettingsKeyRow` for every settingsd-backed row and
  `SettingsAbsenceNotice` for every absent adapter half; a control must not
  write a file or run a command from QML.
- New schema keys must update both `schema.rs` and
  `settingsSchemaDefaults()`; `tst_settings_apply` and the existing client
  tests fail on drift.
- `tst_settings_apply` covers range/enum/type validation, an owner failure
  with revert, service disappearance, and restart resync; `tst_settings_auth`
  covers allow/deny/cancel/dismissed/authority error/wrong-password against a
  fake polkit authority; `tst_settings_apply_ui` covers the inline error, all
  five absence kinds, the map's use of each kind, and cancelled/denied/wrong-
  password/allow account actions with no password field.
- The deterministic visual check is `make t224-settings-apply-capture`, writing
  `docs/captures/t224-settings-{live,write-failure,auth-cancel,service-absence}.png`.
