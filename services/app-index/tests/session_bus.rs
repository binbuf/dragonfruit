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

    fn remove_app(&self, name: &str) {
        std::fs::remove_file(self.apps().join(name)).unwrap();
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
    serve_parts(bus, fixture).0
}

/// Serve the fixture index and return the live object too, so a test can start
/// the coalescer thread the daemon runs.
fn serve_parts(bus: &PrivateBus, fixture: &Fixture) -> (Connection, AppIndex1) {
    let index = AppIndex::from_dirs([fixture.apps()]);
    let theme = IconTheme::from_roots([fixture.icons()], ["hicolor".to_owned()], Vec::new());
    let object = AppIndex1::from_index(index, theme);
    let service = zbus::blocking::connection::Builder::address(bus.address.as_str())
        .expect("valid bus address")
        .name(DBUS_NAME)
        .expect("valid well-known name")
        .serve_at(DBUS_PATH, object.clone())
        .expect("serve the app index interface")
        .build()
        .expect("build the service connection");
    (service, object)
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

// T-14.1b: a fixture install/update/uninstall reaches the index through
// `Refresh`, and the diff is returned as the index events.
#[test]
fn refresh_reports_install_update_and_uninstall_and_updates_the_index() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("refresh");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    // A stable corpus has no events.
    assert_eq!(
        json(&call::<String>(&client, "Refresh", ()).unwrap()),
        json("[]")
    );

    // Install: a new desktop id appears.
    fixture.write_app(
        "org.example.New.desktop",
        "[Desktop Entry]\nType=Application\nName=New\nStartupWMClass=org.example.New\n\
         Exec=new-app\n",
    );
    let events = json(&call::<String>(&client, "Refresh", ()).unwrap());
    assert_eq!(events[0]["kind"], "installed");
    assert_eq!(events[0]["desktopId"], "org.example.New.desktop");
    assert_eq!(events[0]["name"], "New");

    // The new record is immediately enumerable.
    let records = json(&call::<String>(&client, "Enumerate", ()).unwrap());
    assert!(records
        .as_array()
        .unwrap()
        .iter()
        .any(|record| record["desktopId"] == "org.example.New.desktop"));

    // Update: an existing entry's record changes.
    fixture.write_app(
        "firefox.desktop",
        "[Desktop Entry]\nType=Application\nName=Firefox Nightly\nIcon=firefox\n\
         StartupWMClass=firefox\nExec=firefox %u\n",
    );
    let events = json(&call::<String>(&client, "Refresh", ()).unwrap());
    assert_eq!(events[0]["kind"], "updated");
    assert_eq!(events[0]["desktopId"], "firefox.desktop");
    assert_eq!(events[0]["name"], "Firefox Nightly");

    // Uninstall: a desktop id disappears.
    fixture.remove_app("org.example.New.desktop");
    let events = json(&call::<String>(&client, "Refresh", ()).unwrap());
    assert_eq!(events[0]["kind"], "uninstalled");
    assert_eq!(events[0]["desktopId"], "org.example.New.desktop");

    // The retained log drains once, oldest first.
    let drained = json(&call::<String>(&client, "IndexEvents", ()).unwrap());
    assert_eq!(drained.as_array().unwrap().len(), 3);
    assert_eq!(drained[0]["kind"], "installed");
    assert_eq!(drained[2]["kind"], "uninstalled");
    assert_eq!(
        json(&call::<String>(&client, "IndexEvents", ()).unwrap()),
        json("[]")
    );
}

// T-14.1b: window activity drives the running set and the recency order.
#[test]
fn window_activity_tracks_running_and_recency() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("activity");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    // A resolved Wayland app appears.
    let files =
        json(&call::<String>(&client, "WindowOpened", ("org.dragonfruit.Files", "", "")).unwrap());
    assert_eq!(files["desktopId"], "org.dragonfruit.Files.desktop");
    assert_eq!(files["windows"], 1);
    assert_eq!(files["running"], true);

    // An X11 app resolves through StartupWMClass.
    let firefox =
        json(&call::<String>(&client, "WindowOpened", ("", "Navigator", "Firefox")).unwrap());
    assert_eq!(firefox["desktopId"], "firefox.desktop");

    let running = json(&call::<String>(&client, "Running", ()).unwrap());
    assert_eq!(running.as_array().unwrap().len(), 2);

    // Recency is most recent first.
    let recent = json(&call::<String>(&client, "Recent", (0i32,)).unwrap());
    assert_eq!(recent[0]["desktopId"], "firefox.desktop");
    assert_eq!(recent[1]["desktopId"], "org.dragonfruit.Files.desktop");
    assert!(recent[0]["iconPath"]
        .as_str()
        .unwrap()
        .ends_with("hicolor/48x48/apps/firefox.png"));

    // A focus change reorders without changing the running set.
    assert!(call::<bool>(&client, "NoteActivity", ("org.dragonfruit.Files", "", "")).unwrap());
    let recent = json(&call::<String>(&client, "Recent", (0i32,)).unwrap());
    assert_eq!(recent[0]["desktopId"], "org.dragonfruit.Files.desktop");
    assert_eq!(recent[0]["running"], true);

    // Closing the last firefox window leaves the running set but keeps it in
    // recency.
    assert!(call::<bool>(&client, "WindowClosed", ("", "Navigator", "Firefox")).unwrap());
    let running = json(&call::<String>(&client, "Running", ()).unwrap());
    assert_eq!(running.as_array().unwrap().len(), 1);
    assert_eq!(running[0]["desktopId"], "org.dragonfruit.Files.desktop");
    let recent = json(&call::<String>(&client, "Recent", (0i32,)).unwrap());
    assert_eq!(recent.as_array().unwrap().len(), 2);
    assert_eq!(recent[0]["desktopId"], "org.dragonfruit.Files.desktop");
    assert_eq!(recent[1]["desktopId"], "firefox.desktop");
    assert_eq!(recent[1]["running"], false);

    // Closing an app that was not running is a no-op.
    assert!(!call::<bool>(&client, "WindowClosed", ("", "Navigator", "Firefox")).unwrap());

    // The activity log records the sequence.
    let events = json(&call::<String>(&client, "ActivityEvents", ()).unwrap());
    let kinds: Vec<&str> = events
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        vec!["app_running", "app_running", "focused", "app_exited"]
    );
}

/// Listen for directed `Changed` signals on `client`, forwarding the payloads
/// over a channel so a test can bound its wait.
fn listen(client: &Connection) -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let listener = client.clone();
    std::thread::spawn(move || {
        let proxy = zbus::blocking::Proxy::new(&listener, DBUS_NAME, DBUS_PATH, INTERFACE)
            .expect("signal proxy");
        let mut signals = proxy.receive_signal("Changed").expect("listen for Changed");
        for message in signals.by_ref() {
            let (interests,): (String,) = message.body().deserialize().unwrap_or_default();
            if tx.send(interests).is_err() {
                break;
            }
        }
    });
    rx
}

/// Mutate the index/registry in a way that notes all three change kinds.
fn burst(fixture: &Fixture, client: &Connection) {
    fixture.write_app(
        "org.example.Burst.desktop",
        "[Desktop Entry]\nType=Application\nName=Burst\nStartupWMClass=burst\nExec=burst\n",
    );
    let _ = call::<String>(client, "Refresh", ()).unwrap();
    let _ = call::<String>(client, "WindowOpened", ("", "Navigator", "Firefox")).unwrap();
    let _ = call::<String>(client, "WindowOpened", ("org.dragonfruit.Files", "", "")).unwrap();
    let _ = call::<bool>(client, "NoteActivity", ("org.dragonfruit.Files", "", "")).unwrap();
}

// T-14.1c: a subscriber hears one coalesced signal per burst of changes, and
// no consumer has to poll. `Flush` closes the window deterministically.
#[test]
fn a_subscriber_receives_one_coalesced_signal_per_burst() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("subscribe");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();
    let rx = listen(&client);

    // Subscribe: the service echoes the canonical interest set.
    let echoed = call::<String>(&client, "Subscribe", ("all",)).unwrap();
    assert_eq!(echoed, "identity,recency,icons");
    assert_eq!(call::<u32>(&client, "SubscriberCount", ()).unwrap(), 1);

    burst(&fixture, &client);

    // Exactly one coalesced notice for the burst, carrying the union of the
    // changed kinds.
    assert_eq!(call::<u32>(&client, "Flush", ()).unwrap(), 1);
    let first = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("a coalesced Changed signal arrives");
    assert_eq!(first, "identity,recency,icons");
    // Nothing further is pending.
    assert_eq!(call::<u32>(&client, "Flush", ()).unwrap(), 0);
    assert!(
        rx.recv_timeout(std::time::Duration::from_millis(300))
            .is_err(),
        "a single coalesced signal per burst"
    );

    // Unsubscribing stops delivery.
    assert!(call::<bool>(&client, "Unsubscribe", ()).unwrap());
    assert_eq!(call::<u32>(&client, "SubscriberCount", ()).unwrap(), 0);
    let _ = call::<String>(&client, "WindowOpened", ("", "", "Steam")).unwrap();
    assert_eq!(call::<u32>(&client, "Flush", ()).unwrap(), 0);
    assert!(
        rx.recv_timeout(std::time::Duration::from_millis(300))
            .is_err(),
        "no signals after Unsubscribe"
    );
}

// T-14.1c: the production coalescer thread delivers without a manual flush.
#[test]
fn the_coalescer_thread_delivers_without_a_flush() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("coalesce-thread");
    fixture.seed();
    let (service, object) = serve_parts(&bus, &fixture);
    dragonfruit_app_index::dbus::spawn_coalescer(service.clone(), object);
    let client = bus.connect();
    let rx = listen(&client);

    assert_eq!(
        call::<String>(&client, "Subscribe", ("recency",)).unwrap(),
        "recency"
    );
    let _ = call::<String>(&client, "WindowOpened", ("org.dragonfruit.Files", "", "")).unwrap();

    // No flush: the thread wakes to the coalescing deadline and delivers.
    let payload = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the coalescer thread delivers the signal");
    assert_eq!(payload, "recency");
}

// T-14.1b: an unresolved window still gets a stable raw-key registry entry.
#[test]
fn an_unresolved_window_is_registered_by_its_raw_identity() {
    let bus = PrivateBus::start();
    let fixture = Fixture::new("raw");
    fixture.seed();
    let _service = serve(&bus, &fixture);
    let client = bus.connect();

    let opened = json(
        &call::<String>(
            &client,
            "WindowOpened",
            ("", "sun-awt-X11-XFramePeer", "Java"),
        )
        .unwrap(),
    );
    assert_eq!(opened["key"], "java");
    assert_eq!(opened["desktopId"], "java");
    assert_eq!(opened["resolved"], false);
    assert_eq!(opened["windows"], 1);

    let running = json(&call::<String>(&client, "Running", ()).unwrap());
    assert_eq!(running[0]["key"], "java");
    assert_eq!(running[0]["resolved"], false);

    // The identity still did not pollute the resolution audit.
    let (_, resolved, unresolved): (u32, u64, u64) = call(&client, "Stats", ()).unwrap();
    assert_eq!(resolved, 0);
    assert_eq!(unresolved, 0);
}
