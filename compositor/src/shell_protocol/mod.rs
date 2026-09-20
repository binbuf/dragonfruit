// SPDX-License-Identifier: MIT
//! Private shell protocols (T-07): generated server bindings.
//!
//! The XMLs live in `protocols/` and ship as a lockstep set with the
//! compositor and shell (docs/ipc-versioning.md). `wayland-scanner`
//! compiles them into the server-side objects and message enums; the
//! request handling and event broadcasting live in [`crate::shell`].
//!
//! Access to every interface except `df_core` is gated on a successful
//! `df_core.authenticate` with a one-time launch token (the trust model in
//! [`crate::shell::trust`]).

#![allow(clippy::all, unused_imports)]

pub mod core_protocol {
    use wayland_server;
    use wayland_server::protocol::*;

    pub mod __interfaces {
        use wayland_server::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-core.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_server_code!("../protocols/dragonfruit-core.xml");
}

pub mod shell_protocol {
    use wayland_server;
    use wayland_server::protocol::*;

    pub mod __interfaces {
        use wayland_server::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-shell.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_server_code!("../protocols/dragonfruit-shell.xml");
}

pub mod toplevel_protocol {
    use wayland_server;
    use wayland_server::protocol::*;

    pub mod __interfaces {
        use wayland_server::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/dragonfruit-toplevel.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_server_code!("../protocols/dragonfruit-toplevel.xml");
}
