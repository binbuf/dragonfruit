# Private protocols and IPC schemas

Private Wayland protocols for compositor↔shell communication, plus the
shared versioning crate (`df-ipc`) that every Dragonfruit process embeds.

## Rules

- **MIT license** for every protocol XML, so any third party (including
  other compositors and toolkits) may implement them freely
  (docs/licensing.md).
- **Lockstep set:** compositor, shell, and protocol XMLs ship together per
  release. Every XML carries a `dragonfruit lockstep-version: N` marker that
  must match `df_ipc::LOCKSTEP_VERSION` — enforced by the
  `protocol_xmls_match_lockstep_version` test, which fails the build on
  any disagreement.
- **Additive-only:** within a stable release, new requests/events may be
  added to interfaces; existing ones never change meaning or disappear.
  Cross-version mixing is detected at the handshake
  (`df_ipc::assert_lockstep_compatible`) and rejected.
- **D-Bus** services use `org.dragonfruit.*` names with a per-major-version
  suffix (e.g. `org.dragonfruit.Settings1`), validated by
  `df_ipc::is_valid_dbus_name`.

The full policy lives in [docs/ipc-versioning.md](../docs/ipc-versioning.md).
The actual shell protocol lands in T-07; `dragonfruit-core.xml` is the
versioning-convention exemplar.
