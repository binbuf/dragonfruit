# 0234 — Network panes: one NetworkManager model, priority order, and the firewall adapter

## Status

accepted — T225 implements the Wi-Fi, Network, VPN, and firewall Settings
surfaces.

## Context

T225 ships four surfaces that look related but have three different owners:
NetworkManager (radio, access points, saved connections, active connections,
VPN connections), `settingsd` (the two Wi-Fi prompt policies), and a firewall
manager (firewalld/ufw/nftables). The macOS reference groups them into a Wi-Fi
pane, a Network pane, and a Wi-Fi quick sheet. The task also requires honest
absence for NetworkManager, the firewall, and a missing VPN plugin, and the
design notes require the Wi-Fi quick sheet and pane to share one model.

Two mapping questions had no earlier answer: how a macOS **service order** maps
to NetworkManager, and what a detected nftables install may claim.

## Decision

- **One model.** The bridge host's `wifi` view is the NetworkManager projection:
  radio, current network, visible APs, saved services, known networks, and the
  active address. The Wi-Fi pane and the shell's Wi-Fi quick sheet both read
  that view; the quick sheet adds only presentation. The Network pane reads the
  same snapshot through the combined `network` view (the `wifi` fields plus a
  `firewall` sub-view). No second network backend exists.
- **Service order is `connection.autoconnect-priority`.** NetworkManager has no
  ordered service list. The pane sorts saved non-VPN services by descending
  priority (then connected state, then name) and a Move up/down writes a
  priority just past the neighbour. Higher priority wins, matching
  NetworkManager's own preference. The UUID, never the row index, is the write
  identity.
- **The Wi-Fi prompt policy is two settingsd text keys**,
  `network.wifi.askToJoin` (`off`/`notify`/`ask`/`new`) and
  `network.wifi.askToJoinHotspots` (`off`/`ask`/`automatic`), schema revision
  19. They decide whether the shell prompts; NetworkManager autoconnect stays
  authoritative.
- **The firewall is one adapter with manager precedence** firewalld → ufw →
  nftables. firewalld and ufw report status and accept enable/disable/open
  port/open service; writes use `pkexec` when present so the shell polkit agent
  owns the prompt. nftables is detected but has no service status and no
  first-party rule editor (ADR 0229), so it is `detected: true`,
  `statusKnown: false`, `enabled: null` (Unavailable), and its writes are
  Unsupported. No manager is `detected: false` and "No firewall detected".
- **VPN plugins are discovered, not invented.** The installed list is the
  NetworkManager plugin files plus the built-in WireGuard entry. Add writes a
  saved `vpn` connection (name, plugin service type, optional gateway); the
  full credential/editor surface remains out of scope. A host with no plugin
  still lists saved connections and hides the Add affordance.
- **Absence is layered.** NetworkManager absent disables the Wi-Fi/Network
  controls with a reason. Firewall absent shows "No firewall detected" with no
  toggle; firewall status-unavailable shows Unavailable with no fake Off. A
  missing VPN plugin hides Add, not the saved connections.

## Consequences

- Later Settings tasks use the `wifi`/`network` view shapes; they must not add a
  second NetworkManager cache or re-derive the service order.
- The priority mapping is reversible and daemon-native; a future ordered-service
  API in NetworkManager can replace the two-write move without changing the
  pane contract.
- nftables stays a detection/absence row until a separate rule-editor task
  reopens ADR 0229.
- The two prompt keys are additive and owned by `apps/settings`; the shell
  consumes them. A later change to the modes is a new schema revision.
