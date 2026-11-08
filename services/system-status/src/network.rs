// SPDX-License-Identifier: MIT
//! The Network pane's combined view (T225).
//!
//! The Network pane and the Wi-Fi quick sheet share one NetworkManager model:
//! [`crate::wifi_view`] already carries the radio state, the visible networks,
//! the saved services, and the known Wi-Fi networks. This module adds the
//! firewall sub-view and renames the discriminator to `network`, so the pane
//! can render the Wi-Fi disclosure, the Firewall disclosure, and Other
//! Services from one read. The Wi-Fi pane uses the `wifi` view directly.

use dragonfruit_firewall::{FirewallAdapter, FirewallSource};
use dragonfruit_networkmanager::{NetworkManagerAdapter, NetworkManagerSource, NetworkResult};
use serde_json::{json, Value};

use crate::{firewall_view, wifi_view};

/// The `kind` discriminator the Network view carries.
const KIND_NETWORK: &str = "network";

/// Build the combined Network view: the Wi-Fi/service snapshot plus the
/// firewall sub-view.
pub fn network_view<N: NetworkManagerSource, F: FirewallSource>(
    wifi: &NetworkManagerAdapter<N>,
    firewall: &FirewallAdapter<F>,
) -> Value {
    let mut view = wifi_view(wifi);
    view["kind"] = json!(KIND_NETWORK);
    view["firewall"] = firewall_view(firewall);
    view
}

/// The JSON report for a non-join Network write.
pub fn network_report(result: &NetworkResult) -> Value {
    match result {
        NetworkResult::Applied => json!({ "outcome": "applied" }),
        NetworkResult::Denied { note } => json!({ "outcome": "denied", "note": note }),
        NetworkResult::Absent => json!({ "outcome": "absent" }),
        NetworkResult::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
        NetworkResult::Unsupported => json!({ "outcome": "unsupported" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_firewall::MockFirewall;
    use dragonfruit_networkmanager::{MockNetworkManager, NetworkManagerData};

    fn wifi() -> NetworkManagerAdapter<MockNetworkManager> {
        let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::present(
            NetworkManagerData::default(),
        ));
        adapter.refresh();
        adapter
    }

    fn firewall() -> FirewallAdapter<MockFirewall> {
        let mut adapter = FirewallAdapter::new(MockFirewall::present());
        adapter.refresh();
        adapter
    }

    #[test]
    fn the_network_view_carries_the_wifi_services_and_firewall() {
        let view = network_view(&wifi(), &firewall());
        assert_eq!(view["kind"], "network");
        assert_eq!(view["state"], "available");
        assert_eq!(view["firewall"]["kind"], "firewall");
        assert_eq!(view["firewall"]["state"], "available");
        assert_eq!(view["firewall"]["backend"], "firewalld");
    }

    #[test]
    fn an_absent_firewall_keeps_the_network_view_available() {
        let mut firewall = FirewallAdapter::new(MockFirewall::absent());
        firewall.refresh();
        let view = network_view(&wifi(), &firewall);
        assert_eq!(view["state"], "available");
        assert_eq!(view["firewall"]["state"], "unavailable");
        assert_eq!(view["firewall"]["detected"], false);
    }
}
