// SPDX-License-Identifier: MIT OR Apache-2.0
//! Link helper for developer environments where the native DRM stack
//! (libgbm, libseat, libinput, libudev, libxkbcommon) only provides
//! runtime libraries (.so.<n>) without linker-name symlinks — e.g. a
//! user-space sysroot built from -devel RPMs (see PROGRESS.md, T-02).
//!
//! If `~/.local/df-devroot/lib64` exists, it is added to the linker
//! search path. Normal systems (CI, real dev machines with the -devel
//! packages installed) are unaffected.

fn main() {
    let home = std::env::var("HOME").unwrap_or_default();

    // User-space sysroot for the DRM native stack (libgbm/libseat/
    // libinput/libudev) built from -devel RPMs (see PROGRESS.md, T-02).
    let devroot = std::path::PathBuf::from(&home).join(".local/df-devroot/lib64");
    if devroot.is_dir() {
        println!("cargo:rustc-link-search=native={}", devroot.display());
        // The user-space sysroot also provides the runtime libraries.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", devroot.display());
    }

    // `libxkbcommon.so` linker-name fallback (see PROGRESS.md, T-01).
    let local_lib = std::path::PathBuf::from(home).join(".local/lib");
    if local_lib.join("libxkbcommon.so").exists() {
        println!("cargo:rustc-link-search=native={}", local_lib.display());
    }

    println!("cargo:rerun-if-changed=build.rs");
}
