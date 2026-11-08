# SPDX-License-Identifier: MIT
#
# Binary RPM for the GitHub release assets. This is deliberately not the
# Fedora review/COPR spec: it packages an already-staged tree so the same
# binaries can be published as a tar.gz, DEB, and RPM. Official Fedora
# packaging (source RPM, vendored crates, review) is T164.
#
# Build with scripts/package.sh, which passes df_version, df_release,
# df_stage, and df_date.

%global debug_package %{nil}
%global df_version %{?df_version}%{!?df_version:0.0.0}
%global df_release %{?df_release}%{!?df_release:1}
%global df_qt_min %{?df_qt_min}%{!?df_qt_min:6.8}
%global df_stage %{?df_stage}%{!?df_stage:/nonexistent}
%global df_date %{?df_date}%{!?df_date:Thu Jan 01 2026}

Name:           dragonfruit-desktop
Version:        %{df_version}
Release:        %{df_release}%{?dist}
Summary:        Modern Wayland desktop environment
License:        MIT
URL:            https://github.com/dragonfruit-desktop/dragonfruit
BuildArch:      %{_arch}

# Required host stack. Optional services are weak dependencies below; the
# desktop must start and work with this set only (docs/design/12-packaging.md).
Requires:       glibc
Requires:       libstdc++
Requires:       qt6-qtbase >= %{df_qt_min}
Requires:       qt6-qtdeclarative >= %{df_qt_min}
Requires:       qt6-qtwayland >= %{df_qt_min}
Requires:       systemd
Requires:       dbus-daemon
Requires:       xdg-desktop-portal
Requires:       xorg-x11-server-Xwayland
Requires:       libwayland-client
Requires:       libxkbcommon
Requires:       libdrm
Requires:       mesa-libgbm
Requires:       libinput
Requires:       libseat
Requires:       dbus-libs
Requires:       mesa-libEGL
Requires:       mesa-libGL
Requires:       pam

Recommends:     NetworkManager
Recommends:     bluez
Recommends:     pipewire
Recommends:     wireplumber
Recommends:     upower
Recommends:     udisks2
Recommends:     polkit
Recommends:     xdg-desktop-portal-gtk
Recommends:     flatpak
Recommends:     cups
Recommends:     sane-backends
Recommends:     colord
Recommends:     fprintd
Recommends:     accountsservice
Recommends:     gnome-online-accounts
Recommends:     orca
Recommends:     speech-dispatcher
Recommends:     ibus
Recommends:     fcitx5
Recommends:     firewalld

%description
Dragonfruit is a Wayland desktop environment built with Rust and Qt Quick.

This package installs the compositor, shell, session services, Settings,
Files, and the XDG desktop portal backend. It installs alongside another
desktop; choose Dragonfruit at the login screen. It does not remove another
desktop or change the display-manager selection.

This is an early binary release asset. Official Fedora packaging and COPR are
tracked separately (T164).

%prep

%build

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a %{df_stage}/. %{buildroot}/

%post
update-desktop-database -q %{_datadir}/applications >/dev/null 2>&1 || :
gtk-update-icon-cache -q -t -f %{_datadir}/icons/hicolor >/dev/null 2>&1 || :

%postun
update-desktop-database -q %{_datadir}/applications >/dev/null 2>&1 || :
gtk-update-icon-cache -q -t -f %{_datadir}/icons/hicolor >/dev/null 2>&1 || :

%files
%license %{_datadir}/doc/dragonfruit/LICENSE.md
%license %{_datadir}/doc/dragonfruit/LICENSES
%doc %{_datadir}/doc/dragonfruit/README.md
%doc %{_datadir}/doc/dragonfruit/NOTICE
%{_datadir}/doc/dragonfruit/manifest.txt

%{_bindir}/dragonfruit-app-index
%{_bindir}/dragonfruit-compositor
%{_bindir}/dragonfruit-files
%{_bindir}/dragonfruit-menu-broker
%{_bindir}/dragonfruit-notifications
%{_bindir}/dragonfruit-pam-helper
%{_bindir}/dragonfruit-session
%{_bindir}/dragonfruit-session-entry
%{_bindir}/dragonfruit-settings
%{_bindir}/dragonfruit-settingsd
%{_bindir}/dragonfruit-shell
%{_bindir}/dragonfruit-system-status
%{_bindir}/dragonfruit-wallpaperd
%{_bindir}/xdg-desktop-portal-dragonfruit

%{_prefix}/lib/systemd/user/dragonfruit-session.target
%{_prefix}/lib/systemd/user/dragonfruit-*.service

%{_datadir}/wayland-sessions/dragonfruit.desktop
%{_datadir}/applications/org.dragonfruit.Files.desktop
%{_datadir}/applications/org.dragonfruit.Settings.desktop
%{_datadir}/dragonfruit
%{_datadir}/icons/capitaine-cursors
%{_datadir}/icons/hicolor/scalable/apps/org.dragonfruit.*.svg
%{_datadir}/xdg-desktop-portal/dragonfruit-portals.conf
%{_datadir}/xdg-desktop-portal/portals/dragonfruit.portal
%{_datadir}/dbus-1/services/org.dragonfruit.Files1.service
%{_datadir}/dbus-1/services/org.freedesktop.impl.portal.desktop.dragonfruit.service

%changelog
* %{df_date} Dragonfruit Developers <dragonfruit@users.noreply.github.com> - %{version}-%{release}
- Release build.
