// SPDX-License-Identifier: MIT
//! T-18.1a acceptance (session bus): drive `org.dragonfruit.Wallpaper1` over a
//! real private `dbus-daemon`, exactly as the service serves it.
//!
//! `Preload` performs the eager catalogue load and returns the JSON snapshot;
//! the `Status`/`Items`/`DefaultSource`/`BuiltinDefaultSource` properties
//! round-trip; `ItemsChanged`/`StatusChanged` fire; and an absent network
//! leaves the service answering with the shipped default and an offline state.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use dragonfruit_wallpaperd::cache::CacheLayout;
use dragonfruit_wallpaperd::dbus::{Wallpaper1, DBUS_NAME, DBUS_PATH, INTERFACE};
use dragonfruit_wallpaperd::model::{Category, Status};
use dragonfruit_wallpaperd::provider::{Provider, ServiceState};
use dragonfruit_wallpaperd::source::MockSource;

/// A private session bus that lives exactly as long as the test.
struct PrivateBus {
    address: String,
    child: Child,
}

impl PrivateBus {
    fn start() -> Self {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("dbus-daemon is required to run the T-18.1a session-bus test");
        let stdout = child.stdout.take().expect("piped dbus-daemon stdout");
        let mut reader = BufReader::new(stdout);
        let mut address = String::new();
        reader
            .read_line(&mut address)
            .expect("dbus-daemon prints its address");
        PrivateBus {
            address: address.trim().to_owned(),
            child,
        }
    }

    fn connect(&self) -> zbus::blocking::Connection {
        zbus::blocking::connection::Builder::address(self.address.as_str())
            .expect("valid bus address")
            .build()
            .expect("connect to the private session bus")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[zbus::proxy(
    interface = "org.dragonfruit.Wallpaper1",
    default_service = "org.dragonfruit.Wallpaper1",
    default_path = "/org/dragonfruit/Wallpaper1"
)]
trait Wallpaper {
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn last_fetch(&self) -> zbus::Result<u64>;
    #[zbus(property)]
    fn default_source(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn builtin_default_source(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn items(&self) -> zbus::Result<String>;

    fn refresh(&self) -> zbus::Result<String>;
    fn preload(&self) -> zbus::Result<String>;
}

/// Serve the interface on `bus` exactly as [`dragonfruit_wallpaperd::dbus::run`]
/// does, minus the parking loop.
fn serve(bus: &PrivateBus, state: Arc<ServiceState>) -> zbus::blocking::Connection {
    zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .serve_at(DBUS_PATH, Wallpaper1::new(state))
        .expect("serve the wallpaper interface")
        .build()
        .expect("build the service connection")
}

/// A cache in a temp dir and a provider over it.
fn fixture(builtin: Option<PathBuf>) -> (tempfile::TempDir, Arc<MockSource>, Arc<ServiceState>) {
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let source = Arc::new(MockSource::online());
    let state = Arc::new(ServiceState::new(
        source.clone(),
        Provider::load(cache, builtin),
    ));
    (dir, source, state)
}

fn seed(source: &MockSource, category: Category, pageids: &[u64]) {
    let items: Vec<_> = pageids
        .iter()
        .map(|&pageid| MockSource::parsed_item(category, pageid))
        .collect();
    for &pageid in pageids {
        source.with_download(
            &format!("https://upload.wikimedia.org/{pageid}.jpg"),
            b"image-bytes",
        );
    }
    source.with_items(category, items);
}

#[test]
fn preload_round_trips_properties_and_fires_the_change_signals() {
    let bus = PrivateBus::start();
    let builtin = std::env::temp_dir().join("df-wallpaper-builtin.jpg");
    let (_dir, source, state) = fixture(Some(builtin.clone()));
    seed(&source, Category::Nature, &[5, 6]);
    let _service = serve(&bus, state);
    let client = bus.connect();
    let proxy = WallpaperProxyBlocking::builder(&client)
        .build()
        .expect("proxy builds");

    // Before any Preload the service is lazy: idle, empty, shipped default set.
    assert_eq!(proxy.status().unwrap(), "idle");
    assert_eq!(proxy.items().unwrap(), "[]");
    assert_eq!(proxy.default_source().unwrap(), "");
    assert_eq!(
        PathBuf::from(proxy.builtin_default_source().unwrap()),
        builtin
    );

    // Listen for the change signals, then Preload.
    let signals =
        zbus::blocking::Proxy::new(&client, DBUS_NAME, DBUS_PATH, INTERFACE).expect("signal proxy");
    let mut items = signals.receive_signal("ItemsChanged").unwrap();
    let mut status = signals.receive_signal("StatusChanged").unwrap();
    let snapshot = proxy.preload().unwrap();

    let snapshot: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(snapshot["status"], "ready");
    assert!(snapshot["lastFetch"].as_u64().unwrap() > 0);
    assert_eq!(snapshot["items"].as_array().unwrap().len(), 2);
    let default = snapshot["defaultSource"].as_str().unwrap();
    assert!(default.ends_with("nature/5.jpg"), "got {default}");
    assert_eq!(
        PathBuf::from(snapshot["builtinDefaultSource"].as_str().unwrap()),
        builtin
    );

    // The properties now read back the populated catalogue.
    assert_eq!(proxy.status().unwrap(), "ready");
    let items_json: serde_json::Value = serde_json::from_str(&proxy.items().unwrap()).unwrap();
    assert_eq!(items_json.as_array().unwrap().len(), 2);
    assert_eq!(items_json[0]["artist"], "Test Artist");
    assert_eq!(items_json[0]["licenseShortName"], "CC BY-SA 4.0");
    assert!(items_json[0]["localPath"]
        .as_str()
        .unwrap()
        .ends_with("nature/5.jpg"));

    // Both signals arrived.
    assert!(
        status.next().is_some(),
        "StatusChanged fires after the fetch"
    );
    assert!(items.next().is_some(), "ItemsChanged fires after the fetch");

    // A second Preload is a no-op (the cache is fresh): no extra catalogue reads
    // beyond the first refresh's six categories.
    let before = source.catalogue_calls().len();
    proxy.preload().unwrap();
    assert_eq!(source.catalogue_calls().len(), before);
}

#[test]
fn an_offline_service_survives_and_serves_the_builtin_default() {
    let bus = PrivateBus::start();
    let builtin = std::env::temp_dir().join("df-wallpaper-builtin-offline.jpg");
    let dir = tempfile::tempdir().unwrap();
    let cache = CacheLayout::new(dir.path().join("wallpapers"));
    let source = Arc::new(MockSource::offline());
    let state = Arc::new(ServiceState::new(
        source.clone(),
        Provider::load(cache, Some(builtin.clone())),
    ));
    let _service = serve(&bus, state);
    let client = bus.connect();
    let proxy = WallpaperProxyBlocking::builder(&client)
        .build()
        .expect("proxy builds");

    // With an empty cache and no network the shipped default resolves and the
    // service answers `offline` instead of crashing.
    let snapshot: serde_json::Value = serde_json::from_str(&proxy.preload().unwrap()).unwrap();
    assert_eq!(snapshot["status"], Status::Offline.as_str());
    assert_eq!(snapshot["items"].as_array().unwrap().len(), 0);
    assert_eq!(
        PathBuf::from(snapshot["builtinDefaultSource"].as_str().unwrap()),
        builtin
    );
    assert_eq!(proxy.status().unwrap(), "offline");
    assert_eq!(proxy.items().unwrap(), "[]");

    // Refresh is explicit and safe too.
    assert!(serde_json::from_str::<serde_json::Value>(&proxy.refresh().unwrap()).is_ok());
}

#[test]
fn the_interface_constants_are_served() {
    let bus = PrivateBus::start();
    let (_dir, _source, state) = fixture(None);
    let _service = serve(&bus, state);
    let client = bus.connect();
    let reply = client
        .call_method(
            Some(DBUS_NAME),
            DBUS_PATH,
            Some("org.freedesktop.DBus.Introspectable"),
            "Introspect",
            &(),
        )
        .unwrap();
    let xml: String = reply.body().deserialize().unwrap();
    for needle in [
        "<interface name=\"org.dragonfruit.Wallpaper1\">",
        "name=\"Preload\"",
        "name=\"Refresh\"",
        "name=\"StatusChanged\"",
        "name=\"ItemsChanged\"",
        "name=\"BuiltinDefaultSource\"",
    ] {
        assert!(xml.contains(needle), "introspection lacks {needle}");
    }
}
