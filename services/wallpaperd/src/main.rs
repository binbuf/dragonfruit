// SPDX-License-Identifier: MIT
//! `dragonfruit-wallpaperd` — the T-18.1a wallpaper content provider.
//!
//! Serves `org.dragonfruit.Wallpaper1` on the user session bus. On start it
//! resolves the shipped `Default.jpg`, rebuilds the catalogue from the cache
//! with no network, then warms lazily in the background; a session without a
//! bus or without a network is a normal state.
//!
//! Debug helpers:
//!
//! ```text
//! dragonfruit-wallpaperd --print-builtin          # the resolved shipped path
//! dragonfruit-wallpaperd --print-status           # the catalogue JSON snapshot
//! dragonfruit-wallpaperd --preload                # eager refresh, then print
//! dragonfruit-wallpaperd --cache-path             # the resolved cache root
//! dragonfruit-wallpaperd --install-default <dir>  # package the shipped asset
//! ```

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use dragonfruit_wallpaperd::cache::CacheLayout;
use dragonfruit_wallpaperd::dbus;
use dragonfruit_wallpaperd::defaults;
use dragonfruit_wallpaperd::provider::{now_secs, Provider, ServiceState};
use dragonfruit_wallpaperd::source::{ContentSource, UreqHttp, WikipediaSource};

fn main() -> ExitCode {
    let mut print_builtin = false;
    let mut print_status = false;
    let mut preload = false;
    let mut print_cache = false;
    let mut install_prefix: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--print-builtin" => print_builtin = true,
            "--print-status" => print_status = true,
            "--preload" => preload = true,
            "--cache-path" => print_cache = true,
            "--install-default" => match args.next() {
                Some(prefix) => install_prefix = Some(PathBuf::from(prefix)),
                None => {
                    eprintln!("dragonfruit-wallpaperd: --install-default needs a prefix");
                    return ExitCode::from(2);
                }
            },
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("dragonfruit-wallpaperd: unknown argument {other:?}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    if let Some(prefix) = install_prefix {
        return match defaults::install_default(&prefix) {
            Ok(path) => {
                println!("{}", path.display());
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("dragonfruit-wallpaperd: cannot install the default: {error}");
                ExitCode::FAILURE
            }
        };
    }

    let cache = cache_layout();
    if print_cache {
        println!("{}", cache.root().display());
        return ExitCode::SUCCESS;
    }

    let builtin = defaults::resolve_default();
    if print_builtin {
        println!(
            "{}",
            builtin
                .as_deref()
                .map_or("", |path| path.to_str().unwrap_or(""))
        );
        return ExitCode::SUCCESS;
    }

    let source: Arc<dyn ContentSource> = Arc::new(WikipediaSource::new(UreqHttp::new()));
    let state = ServiceState::new(source, Provider::load(cache, builtin));

    if print_status || preload {
        if preload {
            state.refresh_if_needed(now_secs());
        }
        println!("{}", state.snapshot_json());
        return ExitCode::SUCCESS;
    }

    match dbus::run(state) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "dragonfruit-wallpaperd: cannot serve {} on the session bus: {error}",
                dbus::DBUS_NAME
            );
            ExitCode::FAILURE
        }
    }
}

/// The cache root from the environment, with a last-resort temp directory so
/// the service still runs without `$HOME`/`$XDG_CACHE_HOME`.
fn cache_layout() -> CacheLayout {
    CacheLayout::from_env().unwrap_or_else(|| {
        eprintln!(
            "dragonfruit-wallpaperd: no $XDG_CACHE_HOME or $HOME; using a temp cache directory"
        );
        CacheLayout::new(std::env::temp_dir().join("dragonfruit/wallpapers"))
    })
}

fn print_help() {
    println!(
        "dragonfruit-wallpaperd — T-18.1a wallpaper content provider\n\
         \n\
         Serves org.dragonfruit.Wallpaper1 on the user session bus.\n\
         Options:\n\
           --print-builtin          print the resolved shipped default path\n\
           --print-status           print the catalogue JSON snapshot\n\
           --preload                eager refresh, then print the snapshot\n\
           --cache-path             print the resolved cache root\n\
           --install-default <dir>  install Default.jpg under <dir>/share\n\
           -h, --help               show this help"
    );
}
