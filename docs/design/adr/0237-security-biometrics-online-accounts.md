# 0237 — Security, biometrics, and online accounts are host-stack projections

- **Status:** Accepted
- **Date:** 2026-11-08
- **Task:** T228

## Context

The Privacy & Security, Biometrics & Password, and Online Accounts panes have
no single desktop daemon. Linux exposes the relevant state through the
xdg-desktop-portal PermissionStore, the distribution package manager and
Flatpak, `lsblk`, SELinux/AppArmor/Secure Boot tools, fprintd, PAM,
AccountsService, and optionally GNOME Online Accounts. macOS has TCC
categories, Touch ID, and Internet Accounts that do not map one-for-one.

## Decision

1. **Security is a second view of `dragonfruit-privacy-adapter`.** A new
   `SecuritySource` reads software sources (`flatpak remotes --show-disabled`
   plus the distro manager), `lsblk -J` for LUKS/crypt devices,
   `getenforce`/`aa-status`/`mokutil --sb-state` for hardening, the installed
   `xdg-desktop-portal` `.portal` descriptors for Extensions, and a compact
   Advanced inventory. The only write is a Flatpak remote enable/disable;
   distribution repositories, LUKS, hardening, extensions, and Advanced are
   read-only.
2. **Biometrics and Online Accounts are new views of
   `dragonfruit-account-adapter`.** `BiometricsSource` reads fprintd over the
   system bus, scans `/etc/pam.d/*` for `pam_fprintd.so`, and changes the
   password through AccountsService's `SetPassword`. `OnlineAccountsSource`
   reads the GOA object manager and provider descriptors and writes
   Add/Remove/`*Disabled` through GOA.
3. **PAM, LUKS, hardening, and extensions are read-only.** The map's proposed
   Hardening toggle and Extensions enable/disable would have no consumer or
   would edit system policy, so they ship as honest status rows. The
   fingerprint unlock/authentication rows reflect PAM and are hidden when PAM
   is not configured.
4. **GOA is optional and hidden when absent.** `online-accounts` is shipped
   with `optional: true`; the sidebar, search index, and deep-link path all
   consult `Settings.onlineAccountsAvailable`. The pane never reimplements
   OAuth.
5. **One shared source per bridge host.** `SharedSource` lets the privacy and
   security adapters share one host-stack source, and the account, biometrics,
   and online-accounts adapters share another, without moving the source twice.
   Absence stays per owner: a missing portal keeps Security live, a missing
   fprintd keeps PAM/password live, and a missing GOA hides only Online
   Accounts.

## Consequences

- The bridge host's existing `Privacy` and `Accounts` interfaces grow the
  security/biometrics/online-accounts reads and writes; the shell and Settings
  app still never link an adapter.
- `SettingsPanes.shippedPanes` now depends on
  `Settings.onlineAccountsAvailable`; the search index rebuilds when GOA
  appears or disappears.
- No new `settingsd` keys are introduced. The portal store and the host tools
  remain the state; the panes declare no durable preference they cannot
  consume.
- A real fingerprint enrollment needs hardware and an interactive sensor; CI
  drives the mock and the live path is exercised by `fprintd`'s own API when a
  reader exists.
