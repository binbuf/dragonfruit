// SPDX-License-Identifier: MIT
//! Print the T221 cross-distro capability probe.
//!
//! The probe never executes a distribution command: it reads
//! `/etc/os-release` and checks the matrix's tool names for executable bits
//! on `PATH`. The output names the detected distribution, the selected stub
//! backend, the host capability state, and each adapter's status.

fn main() {
    let report = dragonfruit_distro_adapters::DistroReport::detect();
    print!("{}", report.render());
}
