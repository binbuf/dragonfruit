# ADR 0273 — Network/VPN capability hardening: native WireGuard, editor delegation, unprivileged firewall reads

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-15 |
| **Task** | T272a |
| **Related** | ADR 0221, 0229, 0234, 0272; `24-system-settings.md` §4.3; `distro-compatibility.md` §9 |

## Context

T272 shipped the Network service list, the firewall sheet, and an optional VPN
pane over NetworkManager. Three writes still crossed a distribution boundary
dishonestly:

1. WireGuard was listed as a VPN plugin
   (`org.freedesktop.NetworkManager.wireguard`) and the Add sheet wrote a
   `connection.type = "vpn"` profile with that service type. NetworkManager's
   WireGuard is a native connection type, so that profile could never connect.
2. "Add VPN" created a plugin profile with only a remote and presented it as
   ready. A profile without credentials is a dead profile.
3. The firewall read ran `ufw status`, which needs root on a default Ubuntu
   install, and returned "No firewall detected" on an iptables-only host even
   when rules were present.

## Decision

- **Native WireGuard is a connection type, not a plugin.** Plugin discovery
  lists only installed `libnm-vpn-plugin-*.so` packages and never synthesises
  a WireGuard entry. The Add form creates a native `wireguard` profile from the
  private key, local address, and one peer (public key + endpoint), with
  optional DNS/MTU. A profile created this way is complete and connectable.
- **A plugin profile is only offered when it can be finished.** The Add form
  offers a plugin only when the plugin is installed and the connection editor
  (`nm-connection-editor`) exists. The write creates the profile and hands it to
  the editor; the row stays "Needs setup" and its connect/auto controls are
  disabled until a later read says it is complete. A "Complete Setup…" row
  action reopens the editor. With no editor, the row names the missing editor
  instead of offering an Add button that fails.
- **The firewall is a capability surface.** Reads are unprivileged where the
  host exposes them: ufw's on/off state comes from `systemctl` and
  `/etc/ufw/ufw.conf` when `ufw status` is refused; nftables reads a
  world-readable `/etc/nftables.conf` when `nft list ruleset` is refused; and
  iptables is named with "rules present / cannot be read unprivileged" and the
  exact `sudo` command. Only a host with no manager at all says "No firewall
  detected".
- **Writes keep the explicit authorisation path.** Firewall writes stay on
  `pkexec` (or the tool itself when `pkexec` is absent); a refusal is a
  reported denial, never a fake success. A persistent polkit helper is a
  separate decision.
- **No privileged daemon is introduced.** Where an unprivileged read is
  impossible, the pane delegates and records the backend as `OPEN`.

## Consequences

- The Network/VPN pane never creates a `vpn`/WireGuard service-type profile.
- Incomplete profiles are visible, labelled, and editable; no Add path can
  produce a row that the pane presents as ready.
- Firewall status is readable on default Fedora/Ubuntu/Debian/Arch installs or
  explicitly delegated with a reason and the owning tool.
- A host with no plugin, no native WireGuard, and no editor shows the install/
  delegate reason and no Add control.
- A future credential-collecting form can replace the editor hand-off without
  changing the profile-state contract.
