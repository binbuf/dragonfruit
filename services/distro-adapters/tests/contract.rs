// SPDX-License-Identifier: MIT
//! The T221 adapter contract tests: the matrix, the four stub backends, the
//! mock backend, distro detection, the dependency sets, and the live probe.

use std::path::PathBuf;

use dragonfruit_distro_adapters::*;

fn assert_not_supported<T: std::fmt::Debug>(
    result: Result<T, DistroError>,
    capability: Capability,
    backend: BackendId,
) {
    match result {
        Err(error @ DistroError::NotSupported { .. }) => {
            assert!(error.is_not_supported(), "{error:?}");
            assert_eq!(error.capability(), capability);
            let text = error.to_string();
            assert!(text.contains("not supported"), "{text}");
            assert!(text.contains(capability.id()), "{text}");
            assert!(text.contains(backend.as_str()), "{text}");
        }
        other => panic!("expected not supported for {capability:?}, got {other:?}"),
    }
}

#[test]
fn the_matrix_covers_every_supported_family_and_capability() {
    for family in DistroFamily::SUPPORTED {
        assert_eq!(specs_for(family).len(), Capability::ALL.len());
        for capability in Capability::ALL {
            let row = spec(capability, family).expect("matrix row");
            assert_eq!(row.capability, capability);
            assert_eq!(row.family, family);
            assert_eq!(row.backend, BackendId::from(family));
            assert!(!row.command.is_empty());
            assert!(!row.api.is_empty());
            assert!(!row.note.is_empty());
            assert!(
                !row.probe.is_empty() || capability == Capability::AppDiscovery,
                "{family:?} {capability:?}"
            );
        }
    }
    for capability in Capability::ALL {
        assert!(spec(capability, DistroFamily::Unknown).is_none());
    }
}

#[test]
fn the_stub_updates_backend_returns_not_supported() {
    for family in DistroFamily::SUPPORTED {
        let backend = BackendId::from(family);
        let stub = StubBackend::for_family(family).updates();
        assert_eq!(stub.capability(), Capability::Updates);
        assert_eq!(stub.backend(), backend);
        assert!(!stub.available());
        let status = stub.status();
        assert!(status.is_unsupported());
        assert_eq!(
            status.command(),
            spec(Capability::Updates, family).unwrap().api
        );
        assert_not_supported(stub.distribution_info(), Capability::Updates, backend);
        assert_not_supported(stub.check_updates(), Capability::Updates, backend);
        assert_not_supported(
            stub.install_updates(&[UpdateId::from("glibc")]),
            Capability::Updates,
            backend,
        );
        assert_not_supported(stub.reboot_required(), Capability::Updates, backend);
    }
}

#[test]
fn the_stub_firewall_backend_returns_not_supported() {
    for family in DistroFamily::SUPPORTED {
        let backend = BackendId::from(family);
        let stub = StubBackend::for_family(family).firewall();
        assert!(!stub.available());
        assert!(stub.status().is_unsupported());
        assert_not_supported(stub.firewall_status(), Capability::Firewall, backend);
        assert_not_supported(stub.set_enabled(true), Capability::Firewall, backend);
        assert_not_supported(
            stub.open_port(Port::new(443, Protocol::Tcp)),
            Capability::Firewall,
            backend,
        );
        assert_not_supported(
            stub.close_port(Port::new(443, Protocol::Tcp)),
            Capability::Firewall,
            backend,
        );
    }
}

#[test]
fn the_stub_users_backend_returns_not_supported() {
    for family in DistroFamily::SUPPORTED {
        let backend = BackendId::from(family);
        let stub = StubBackend::for_family(family).users();
        assert!(!stub.available());
        assert!(stub.status().is_unsupported());
        let user = NewUser {
            user_name: "dan".to_owned(),
            real_name: "Dan Doe".to_owned(),
            administrator: true,
            shell: None,
        };
        assert_not_supported(stub.admin_groups(), Capability::UsersGroups, backend);
        assert_not_supported(stub.list_groups(), Capability::UsersGroups, backend);
        assert_not_supported(stub.create_user(&user), Capability::UsersGroups, backend);
        assert_not_supported(stub.delete_user("dan"), Capability::UsersGroups, backend);
        assert_not_supported(
            stub.set_password("dan", &Secret::new("hunter2")),
            Capability::UsersGroups,
            backend,
        );
        assert_not_supported(stub.create_group("devs"), Capability::UsersGroups, backend);
        assert_not_supported(stub.delete_group("devs"), Capability::UsersGroups, backend);
        assert_not_supported(
            stub.set_group_members("devs", &["dan".to_owned()]),
            Capability::UsersGroups,
            backend,
        );
    }
}

#[test]
fn the_stub_time_locale_and_autologin_backends_return_not_supported() {
    for family in DistroFamily::SUPPORTED {
        let backend = BackendId::from(family);

        let time = StubBackend::for_family(family).time();
        assert!(!time.available());
        assert_not_supported(time.time_status(), Capability::Time, backend);
        assert_not_supported(time.set_timezone("UTC"), Capability::Time, backend);
        assert_not_supported(time.set_ntp(true), Capability::Time, backend);

        let locale = StubBackend::for_family(family).locale();
        assert!(!locale.available());
        assert_not_supported(locale.locale_status(), Capability::Locale, backend);
        assert_not_supported(locale.available_locales(), Capability::Locale, backend);
        assert_not_supported(
            locale.set_locale("en_US.UTF-8"),
            Capability::Locale,
            backend,
        );

        let autologin = StubBackend::for_family(family).autologin();
        assert!(!autologin.available());
        assert_not_supported(autologin.autologin_status(), Capability::Autologin, backend);
        assert_not_supported(
            autologin.set_autologin(Some("dan")),
            Capability::Autologin,
            backend,
        );
    }
}

#[test]
fn the_stub_default_apps_and_discovery_backends_return_not_supported() {
    for family in DistroFamily::SUPPORTED {
        let backend = BackendId::from(family);

        let apps = StubBackend::for_family(family).default_apps();
        assert!(!apps.available());
        assert_not_supported(apps.roles(), Capability::DefaultApps, backend);
        assert_not_supported(
            apps.default_app("browser"),
            Capability::DefaultApps,
            backend,
        );
        assert_not_supported(
            apps.set_default_app("browser", "org.mozilla.firefox.desktop"),
            Capability::DefaultApps,
            backend,
        );
        assert_not_supported(
            apps.mime_handler("text/html"),
            Capability::DefaultApps,
            backend,
        );

        let discovery = StubBackend::for_family(family).app_discovery();
        assert!(!discovery.available());
        assert_not_supported(
            discovery.discovery_paths(),
            Capability::AppDiscovery,
            backend,
        );
        assert_not_supported(
            discovery.install_sources(),
            Capability::AppDiscovery,
            backend,
        );
    }
}

#[test]
fn distro_detection_covers_the_supported_matrix_and_derivatives() {
    let fedora =
        Distro::from_os_release("ID=fedora\nVERSION_ID=44\nPRETTY_NAME=\"Fedora Linux 44\"\n");
    assert_eq!(fedora.family, DistroFamily::Fedora);
    assert_eq!(fedora.tier(), SupportTier::Tier1);
    assert_eq!(fedora.admin_groups(), &["wheel"]);

    let ubuntu =
        Distro::from_os_release("ID=ubuntu\nVERSION_ID=24.04\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n");
    assert_eq!(ubuntu.family, DistroFamily::Ubuntu);
    assert_eq!(ubuntu.tier(), SupportTier::Tier1);
    assert_eq!(ubuntu.admin_groups(), &["sudo", "admin"]);

    let ubuntu_next =
        Distro::from_os_release("ID=ubuntu\nVERSION_ID=26.04\nPRETTY_NAME=\"Ubuntu 26.04\"\n");
    assert_eq!(ubuntu_next.tier(), SupportTier::Tier2);

    let debian = Distro::from_os_release("ID=debian\nVERSION_ID=13\nPRETTY_NAME=\"Debian 13\"\n");
    assert_eq!(debian.family, DistroFamily::Debian);
    assert_eq!(debian.tier(), SupportTier::Tier1);

    let arch = Distro::from_os_release("ID=arch\nPRETTY_NAME=\"Arch Linux\"\n");
    assert_eq!(arch.family, DistroFamily::Arch);
    assert_eq!(arch.tier(), SupportTier::Tier2);
    assert_eq!(arch.admin_groups(), &["wheel"]);

    let bazzite =
        Distro::from_os_release("ID=bazzite\nID_LIKE=\"fedora\"\nPRETTY_NAME=\"Bazzite\"\n");
    assert_eq!(bazzite.family, DistroFamily::Fedora);
    assert!(bazzite.derivative);
    assert_eq!(bazzite.tier(), SupportTier::Tier3);

    let pop =
        Distro::from_os_release("ID=pop\nID_LIKE=\"ubuntu debian\"\nPRETTY_NAME=\"Pop!_OS\"\n");
    assert_eq!(pop.family, DistroFamily::Ubuntu);
    assert!(pop.derivative);

    let manjaro =
        Distro::from_os_release("ID=manjaro\nID_LIKE=\"arch\"\nPRETTY_NAME=\"Manjaro\"\n");
    assert_eq!(manjaro.family, DistroFamily::Arch);
    assert!(manjaro.derivative);
    assert_eq!(manjaro.tier(), SupportTier::Tier3);

    let unknown = Distro::from_os_release("ID=void\nPRETTY_NAME=\"Void Linux\"\n");
    assert_eq!(unknown.family, DistroFamily::Unknown);
    assert_eq!(unknown.tier(), SupportTier::Unsupported);
    assert!(!unknown.is_supported());
}

#[test]
fn the_probe_reads_a_fixture_root_without_running_commands() {
    let root = unique_temp_dir("distro-probe");
    std::fs::create_dir_all(root.join("etc")).unwrap();
    std::fs::write(
        root.join("etc/os-release"),
        "ID=fedora\nVERSION_ID=44\nPRETTY_NAME=\"Fedora Linux 44\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("usr/bin")).unwrap();
    make_executable(&root.join("usr/bin/dnf5"));
    make_executable(&root.join("usr/bin/firewall-cmd"));

    let probe = SystemProbe::with_root_and_path(&root, vec![PathBuf::from("/usr/bin")]);
    let report = DistroReport::detect_with(&probe);

    assert_eq!(report.distro.family, DistroFamily::Fedora);
    assert_eq!(report.backend, BackendId::FEDORA);
    assert!(report
        .capability(Capability::Updates)
        .unwrap()
        .status
        .is_available());
    assert!(report
        .capability(Capability::Firewall)
        .unwrap()
        .status
        .is_available());
    assert!(report
        .capability(Capability::Autologin)
        .unwrap()
        .status
        .is_absent());
    assert!(report
        .capability(Capability::AppDiscovery)
        .unwrap()
        .status
        .is_available());
    assert!(report.render().contains("Fedora Linux 44"));

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_mock_backend_drives_available_absent_and_unsupported() {
    let mock = MockBackend::available();
    for capability in Capability::ALL {
        assert!(mock.is_available(capability), "{capability:?}");
    }
    assert_eq!(mock.updates().check_updates().unwrap().len(), 1);

    let absent = MockBackend::absent();
    for capability in Capability::ALL {
        assert!(!absent.is_available(capability), "{capability:?}");
        assert!(absent.status(capability).is_absent());
    }
    assert!(matches!(
        absent.updates().check_updates(),
        Err(DistroError::Absent { .. })
    ));

    let unsupported = MockBackend::unsupported();
    for capability in Capability::ALL {
        assert!(unsupported.status(capability).is_unsupported());
    }
    assert!(matches!(
        unsupported.updates().check_updates(),
        Err(DistroError::NotSupported { .. })
    ));
}

#[test]
fn the_mock_backend_round_trips_every_write() {
    let mock = MockBackend::available();

    let updates = mock.updates();
    let progress = updates
        .install_updates(&[UpdateId::from("mock-kernel")])
        .unwrap();
    assert_eq!(progress.phase, UpdatePhase::RebootRequired);
    assert!(updates.check_updates().unwrap().is_empty());
    assert!(updates.reboot_required().unwrap());

    let firewall = mock.firewall();
    firewall.set_enabled(false).unwrap();
    firewall.open_port(Port::new(22, Protocol::Tcp)).unwrap();
    assert!(!firewall.firewall_status().unwrap().enabled);
    assert!(firewall
        .firewall_status()
        .unwrap()
        .open_ports
        .contains(&Port::new(22, Protocol::Tcp)));
    firewall.close_port(Port::new(22, Protocol::Tcp)).unwrap();
    assert!(!firewall
        .firewall_status()
        .unwrap()
        .open_ports
        .contains(&Port::new(22, Protocol::Tcp)));

    let users = mock.users();
    users
        .create_user(&NewUser {
            user_name: "dan".to_owned(),
            real_name: "Dan Doe".to_owned(),
            administrator: true,
            shell: None,
        })
        .unwrap();
    assert_eq!(users.created_users().len(), 1);
    users.set_password("dan", &Secret::new("hunter2")).unwrap();
    users.create_group("devs").unwrap();
    users
        .set_group_members("devs", &["dan".to_owned()])
        .unwrap();
    assert!(users
        .list_groups()
        .unwrap()
        .iter()
        .any(|group| group.name == "devs" && group.members == vec!["dan".to_owned()]));
    users.delete_group("devs").unwrap();
    users.delete_user("dan").unwrap();
    assert!(users.created_users().is_empty());

    let time = mock.time();
    time.set_timezone("Europe/Berlin").unwrap();
    time.set_ntp(false).unwrap();
    let time_status = time.time_status().unwrap();
    assert_eq!(time_status.timezone, "Europe/Berlin");
    assert!(!time_status.ntp_enabled);

    let locale = mock.locale();
    locale.set_locale("de_DE.UTF-8").unwrap();
    assert_eq!(locale.locale_status().unwrap().locale, "de_DE.UTF-8");
    assert!(locale
        .available_locales()
        .unwrap()
        .contains(&"de_DE.UTF-8".to_owned()));

    let autologin = mock.autologin();
    autologin.set_autologin(Some("dan")).unwrap();
    assert_eq!(
        autologin.autologin_status().unwrap().user,
        Some("dan".to_owned())
    );

    let apps = mock.default_apps();
    assert_eq!(
        apps.default_app("browser").unwrap(),
        Some("org.mozilla.firefox.desktop".to_owned())
    );
    apps.set_default_app("browser", "org.chromium.Chromium.desktop")
        .unwrap();
    assert_eq!(
        apps.default_app("browser").unwrap(),
        Some("org.chromium.Chromium.desktop".to_owned())
    );
    assert_eq!(
        apps.mime_handler("text/html").unwrap(),
        Some("org.mozilla.firefox.desktop".to_owned())
    );
    assert_eq!(apps.roles().unwrap().len(), 3);

    let discovery = mock.app_discovery();
    assert!(!discovery.discovery_paths().unwrap().is_empty());
    assert!(!discovery.install_sources().unwrap().is_empty());

    assert!(mock.writes() >= 10);
}

#[test]
fn a_mock_write_failure_is_visible_and_not_counted() {
    let mock = MockBackend::available();
    mock.fail_writes("polkit: not authorized");
    let error = mock.firewall().set_enabled(true).unwrap_err();
    assert!(error.is_failed());
    assert!(error.to_string().contains("polkit: not authorized"));
    assert_eq!(mock.writes(), 0);
}

#[test]
fn a_capability_view_is_wire_friendly() {
    let status = MockBackend::available().updates().status();
    let view = status.view();
    assert!(view.available);
    assert_eq!(view.state, CapabilityState::Available);
    assert_eq!(view.capability, Capability::Updates);
    assert_eq!(view.backend, BackendId::MOCK);

    let absent = CapabilityStatus::Absent(Absence {
        capability: Capability::Firewall,
        missing: vec!["firewalld"],
        reason: "No firewall detected".to_owned(),
    });
    let view = absent.view();
    assert!(!view.available);
    assert_eq!(view.state, CapabilityState::Absent);
    assert_eq!(view.reason, "No firewall detected");
}

#[test]
fn the_dependency_sets_are_complete_and_reference_the_canonical_table() {
    for set in DependencySet::ALL {
        assert!(dependencies(set).count() > 0, "{set:?}");
    }
    assert_eq!(
        required().count() + recommended().count() + optional().count(),
        DEPENDENCIES.len()
    );
    for id in [
        "wayland",
        "systemd-logind",
        "networkmanager",
        "bluez",
        "pipewire",
        "upower",
        "udisks2",
        "polkit",
        "xdg-desktop-portal",
        "qt6",
    ] {
        let dependency = dependency(id).unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(dependency.set, DependencySet::Required);
    }
    for dependency in DEPENDENCIES {
        assert!(!dependency.absence.id().is_empty());
        assert!(!dependency.absence.description().is_empty());
        if let Some(service) = dependency.optional_service {
            let needle = format!("\"id\": \"{service}\"");
            assert!(
                OPTIONAL_SERVICES.contains(&needle),
                "{} references {service}",
                dependency.id
            );
        }
    }
}

#[test]
fn the_live_probe_prints_the_current_host_capabilities() {
    let report = DistroReport::detect();
    assert_eq!(report.capabilities.len(), Capability::ALL.len());
    assert!(report.available().len() <= Capability::ALL.len());
    println!("{}", report.render());
}

const OPTIONAL_SERVICES: &str = include_str!("../../../packaging/optional-services.json");

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "dragonfruit-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}
