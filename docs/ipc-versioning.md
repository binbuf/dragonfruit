# IPC Versioning Policy

Dragonfruit is many cooperating processes. They talk over two channels,
and both are versioned contracts governed by this policy (from
[../.docs/design/01-architecture.md](../.docs/design/01-architecture.md)):

1. **Private Wayland protocols** (`protocols/*.xml`) —
   compositor ↔ shell communication: workspace enumeration, window
   state, Mission Control control, output configuration. The shell
   consumes a private, versioned interface; it never scrapes public
   protocols.
2. **D-Bus** — services (settingsd, menu-broker, app-index, portal,
   session) and host daemons (NetworkManager, BlueZ, PipeWire, UPower,
   UDisks, logind).

## The lockstep set

Compositor, shell, and protocol XMLs **ship as one lockstep set** per
release. Cross-version mixing is unsupported and detected at handshake.

- The set's version is `LOCKSTEP_VERSION` in
  `protocols/df-ipc/src/lib.rs` — the single source of truth. Every
  Dragonfruit process embeds it via the `df-ipc` crate.
- Every protocol XML carries the marker
  `dragonfruit lockstep-version: N`, where `N` must equal
  `LOCKSTEP_VERSION`. The `protocol_xmls_match_lockstep_version` test
  fails the build on any disagreement.
- Handshakes compare embedded lockstep versions with
  `df_ipc::assert_lockstep_compatible`. A mismatch produces a protocol
  error — never a crash.

## Additive-only within a stable release

Within a stable release, private Wayland protocols are
**additive-only**:

- New requests, events, and interfaces **may** be added.
- Existing requests and events **never** change meaning, argument
  order, or semantics, and **never** disappear.
- Interface `version` attributes bump only when new members are added;
  clients bind the version they understand.

Bumping `LOCKSTEP_VERSION` is a release event: it happens deliberately,
together, across compositor, shell, XMLs, and `df-ipc`, in one commit.
There are no partial bumps.

## D-Bus naming

D-Bus interfaces use `org.dragonfruit.*` names with a per-major-version
suffix, e.g. `org.dragonfruit.Settings1`:

- A breaking change to a D-Bus interface introduces a new major suffix
  (`Settings2`) and keeps the old name alive for one transition period.
- `df_ipc::is_valid_dbus_name` validates the form
  `org.dragonfruit.<PascalCase><Major>`; service binaries carry their
  planned names as compile-time-tested constants.

## Public desktop-name contract

`XDG_CURRENT_DESKTOP=dragonfruit` is chosen once and is a public
contract (toolkits, `portals.conf`, and `XDG_CURRENT_DESKTOP`-sensitive
libraries key off it — see
[../.docs/design/11-session-and-dev-workflow.md](../.docs/design/11-session-and-dev-workflow.md)).
Only the literal `dragonfruit` is legal anywhere in the tree;
`scripts/check-desktop-names.sh` fails the build (and CI) on any
hardcoded desktop name.
