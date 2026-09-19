// SPDX-License-Identifier: MIT OR Apache-2.0
//! `xdg-desktop-portal-dragonfruit` — scaffold stub. The real portal
//! backend lands in T-27; this binary exists so the workspace, CI,
//! packaging, and licensing are wired from the first commit.

use df_ipc::{DESKTOP_NAME, LOCKSTEP_VERSION};

fn main() {
    println!(
        "xdg-desktop-portal-dragonfruit: stub (lockstep-ipc=v{LOCKSTEP_VERSION}, \
         desktop={DESKTOP_NAME})"
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn portals_conf_target_is_dragonfruit() {
        // portals.conf restricts portal backends to ours plus the generic
        // backends; the desktop name in that file is the same contract
        // (docs/ipc-versioning.md).
        assert_eq!(df_ipc::DESKTOP_NAME, "dragonfruit");
    }
}
