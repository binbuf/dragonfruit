// SPDX-License-Identifier: MIT
//! T-14.1a acceptance: drive `org.dragonfruit.AppIndex1` over a real session
//! bus against a fixture `.desktop` and icon-theme corpus.
//!
//! Each test stands up a private `dbus-daemon` session bus and serves the
//! interface exactly as [`dragonfruit_app_index::dbus::run`] does, then talks
//! to it through a second connection — the shape the Qt shell uses. A Wayland
//! fixture (an `app_id`) and an X11 fixture (a `WM_CLASS`) must resolve to the
//! expected desktop entry and themed icon.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use dragonfruit_app_index::dbus::{AppIndex1, DBUS_NAME, DBUS_PATH, INTERFACE};
use dragonfruit_app_index::icons::IconTheme;
use dragonfruit_app_index::index::AppIndex;
use zbus::blocking::Connection;

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
            .expect("dbus-daemon is required to run the T-14.1a integration test");
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

    fn connect(&self) -> Connection {
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

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "df-app-index-dbus-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Fixture { dir }
    }

    fn apps(&self) -> PathBuf {
        self.dir.join("applications")
    }

    fn icons(&self) -> PathBuf {
        self.dir.join("icons")
    }

    fn write_app(&self, name: &str, body: &str) {
        std::fs::create_dir_all(self.apps()).unwrap();
        std::fs::write(self.apps().join(name), body).unwrap();
    }

    fn write_icon(&self, relative: &str) {
        let path = self.icons().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"icon").unwrap();
    }

    fn seed(&self) {
        self.write_app(
            "org.dragonfruit.Files.desktop",
            "[Desktop Entry]\nType=Application\nName=Files\nIcon=system-file-manager\n\
             Exec=dragonfruit-files %U\n",
        );
        self.write_app(
            "firefox.desktop",
            "[Desktop Entry]\nType=Application\nName=Firefox\nIcon=firefox\n\
             StartupWMClass=firefox\nExec=firefox %u\n",
        );
        self.write_icon("hicolor/48x48/apps/firefox.png");
        self.write_icon("hicolor/scalable/apps/system-file-manager.svg");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Serve the fixture index on `bus` exactly as the daemon does.
fn serve(bus: &PrivateBus, fixture: &Fixture) -> Connection {
    let index = AppIndex::from_dirs([fixture.apps()]);
    let theme = IconTheme::from_roots([fixture.icons()], ["hicolor".to_owned()], Vec::new());
    zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .serve_at(DBUS_PATH, AppIndex1::from_index(index, theme))
        .expect("serve the app index interface")
        .build()
        .expect("build the service connection")
}

fn call<T: serde::de::DeserializeOwned + zbus::zvariant::Type>(
    client: &Connection,
    method: &str,
    body: impl serde::Serialize + zbus::zvariant::DynamicType,
) -> Result<T, zbus::Error> {
    let reply = client.call_method(Some(DBUS_NAME), DBUS_PATH, Some(INTERFACE), method, &body)?;
    Ok(reply.body().deserialize::<T>().expect("reply body decodes"))
}

fn json(payload: &str) -> serde_json::Value {
    serde_json::from_str(payload).expect("the view is valid JSON")
}

#[test]
fn a_wayland_fixture_resolves_to_its_desktop_entry_and_themed_icon() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("wayland");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    let record = json(&call::<String>(&client, "Resolve", ("org.dragonfruit.Files",)).unwrap());
    assert_eq!(record["desktopId"], "org.dragonfruit.Files.desktop");
    assert_eq!(record["name"], "Files");
    assert_eq!(record["icon"], "system-file-manager");
    assert_eq!(record["source"], "app_id");
    assert_eq!(record["valid"], true);
    // The themed icon resolved to the scalable SVG.
    assert!(
        record["iconPath"]
            .as_str()
            .unwrap()
            .ends_with("hicolor/scalable/apps/system-file-manager.svg"),
        "iconPath was {}",
        record["iconPath"]
    );
}

#[test]
fn an_x11_wm_class_fixture_resolves_through_startup_wm_class() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("x11");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    let record =
        json(&call::<String>(&client, "ResolveWindow", ("", "Navigator", "Firefox")).unwrap());
    assert_eq!(record["desktopId"], "firefox.desktop");
    assert_eq!(record["name"], "Firefox");
    assert_eq!(record["source"], "startup_wm_class");
    assert!(
        record["iconPath"]
            .as_str()
            .unwrap()
            .ends_with("hicolor/48x48/apps/firefox.png"),
        "iconPath was {}",
        record["iconPath"]
    );
}

#[test]
fn a_miss_resolves_to_empty_string_and_is_recorded() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("miss");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    let record = call::<String>(
        &client,
        "ResolveWindow",
        ("", "sun-awt-X11-XFramePeer", "Java"),
    )
    .unwrap();
    assert!(record.is_empty());

    let misses = json(&call::<String>(&client, "Misses", ()).unwrap());
    assert_eq!(misses[0], "java");

    let (entries, resolved, unresolved): (u32, u64, u64) = call(&client, "Stats", ()).unwrap();
    assert_eq!(entries, 2);
    assert_eq!(resolved, 0);
    assert_eq!(unresolved, 1);
}

#[test]
fn enumerate_lists_every_record_and_icon_path_resolves_by_name() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("enumerate");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    let records = json(&call::<String>(&client, "Enumerate", ()).unwrap());
    let records = records.as_array().unwrap();
    assert_eq!(records.len(), 2);
    // Ordered by desktop id: firefox.desktop before org.dragonfruit.Files.
    assert_eq!(records[0]["desktopId"], "firefox.desktop");
    assert_eq!(records[1]["desktopId"], "org.dragonfruit.Files.desktop");

    let path = call::<String>(&client, "IconPath", ("firefox", 48i32)).unwrap();
    assert!(path.ends_with("hicolor/48x48/apps/firefox.png"), "{path}");

    // An unknown icon name is the empty string, not an error.
    let missing = call::<String>(&client, "IconPath", ("nope-nope", 48i32)).unwrap();
    assert!(missing.is_empty());
}

#[test]
fn lookup_by_desktop_id_answers_even_without_a_window() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("lookup");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    let record = json(&call::<String>(&client, "Lookup", ("firefox.desktop",)).unwrap());
    assert_eq!(record["name"], "Firefox");
    assert!(call::<String>(&client, "Lookup", ("absent.desktop",))
        .unwrap()
        .is_empty());
}

#[allow(dead_code)]
fn assert_path_is_within(path: &str, root: &Path) {
    assert!(
        Path::new(path).starts_with(root),
        "{path} not under {root:?}"
    );
}
