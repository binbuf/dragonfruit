# SPDX-License-Identifier: MIT
#
# Dragonfruit top-level task runner (T-01, FR-2): one command builds the
# full desktop, one command runs all tests. Drives the Cargo workspace
# (compositor, services, tools) and the CMake/Qt build (design system,
# shell, apps) as one lockstep set.

CARGO ?= cargo
CMAKE ?= cmake
CTEST ?= ctest
BUILD_DIR ?= build
SOAK_CYCLES ?= 100
DEMO_ARGS ?=
DEV_FULL_ARGS ?=
FIXTURES ?=
IDLE_TRACE_SECS ?= 60
DESTDIR ?=
PREFIX ?= /usr
DF_TOOLCHAIN ?= $(HOME)/.local/df-toolchain/usr
DF_DEVROOT ?= $(HOME)/.local/df-devroot/lib64

# The Qt toolchain's cmake and Qt runtime are dynamically linked against
# its own lib64 (libRHash, libQt6*, ...), so that directory must be on the
# loader path whenever the toolchain is used. This is independent of which
# cmake is first on PATH: a build.ninja configured with the toolchain cmake
# still calls it for autogen even when another cmake shadows it on PATH
# (see README "Toolchains").
ifneq ($(wildcard $(DF_TOOLCHAIN)/lib64),)
export LD_LIBRARY_PATH := $(DF_TOOLCHAIN)/lib64$(if $(LD_LIBRARY_PATH),:$(LD_LIBRARY_PATH))
endif

# Discover cmake/ninja from the local Qt toolchain prefix when they are
# not on PATH.
ifeq ($(shell command -v $(CMAKE) >/dev/null 2>&1 || command -v $(CTEST) >/dev/null 2>&1 || echo missing),missing)
ifneq ($(wildcard $(DF_TOOLCHAIN)/bin/cmake),)
export PATH := $(DF_TOOLCHAIN)/bin:$(PATH)
endif
endif

# User-space sysroot for the DRM native stack (libgbm/libseat/libinput/
# libudev headers + linker names) on machines without the -devel
# packages (see PROGRESS.md, T-02). Absent on normal systems.
ifneq ($(wildcard $(DF_DEVROOT)),)
export PKG_CONFIG_PATH := $(DF_DEVROOT)/pkgconfig$(if $(PKG_CONFIG_PATH),:$(PKG_CONFIG_PATH))
export RUSTFLAGS := $(RUSTFLAGS) -L $(DF_DEVROOT)
endif

.DEFAULT_GOAL := help
.PHONY: help all build cargo-build cmake-build configure test cargo-test qml-test \
        visual-test gallery-snapshot check-tokens lint fmt fmt-check clippy check dev dev-full demo soak e2e \
        idle-trace menubar-idle-trace latency-trace drm-bringup input-validation multi-gpu-validation drm-soak second-vt-validation settingsd-capture settings-wave-1-capture \
        t18-wallpaper-capture t18-absence-matrix t15-absence-matrix t15-breadth-capture t16-a11y-audit \
        t16-i18n-capture t16-kill-matrix t17-robustness-matrix t17-window-loop-capture t17-navigation-capture \
        t17-flatpak-browser-capture t17-visual-floor-capture t17-leak-lock-soak i18n-update \
        files-capture osd-dnd-capture portals-capture zoo-run dock-spacing-capture dock-magnify-capture dock-motion-capture dock-trash-capture dock-app-picker-capture dock-drops-capture dock-activation-capture dock-launch-origin-capture dock-folder-stack-capture dock-tooltip-capture dock-tahoe-capture dock-folder-pin-capture dock-chooser-actions-capture dock-chooser-scroll-capture dock-window-badge-capture dock-hover-chooser-capture dock-overflow-capture dock-trash-empty-capture dock-minimize-reaction-capture dock-keyboard-reorder-capture dock-dividers-capture dock-magnify-sweep-capture dock-plate-corners-capture dock-icon-mask-capture check-desktop-names check-no-capture-grab check-design-tokens check-i18n clean install

help:
	@echo "Dragonfruit build targets:"
	@echo "  make build    — build everything (Rust workspace + Qt/CMake)"
	@echo "  make test     — run all tests (cargo + ctest + gallery visual regression)"
	@echo "  make e2e      — T-01…T-07 Foundation vertical-slice + conformance suites"
	@echo "  make idle-trace — T-03.1a 60 s idle/animation frame budget trace"
	@echo "  make latency-trace — T-03.1b nested input-to-photon latency capture"
	@echo "  make multi-gpu-validation — T-03.4 multi-GPU import/fallback validation"
	@echo "  make drm-soak — T-03.4 100-cycle teardown soak + one DRM session cycle"
	@echo "  make second-vt-validation — T-12.6c second-VT preflight/selection + one real cycle"
	@echo "  make settingsd-capture — T-08.3 settingsd flip + restart capture"
	@echo "  make settings-wave-1-capture — T-09.6b Settings wave stills (light/dark/reduced + panes)"
	@echo "  make t18-wallpaper-capture — T-18.2 Wallpaper pane stills (fetching skeleton + filled)"
	@echo "  make t18-absence-matrix — T-18.3 headless absence/state matrix transcript"
	@echo "  make t15-absence-matrix — T-15.16 headless absent-daemon masking matrix transcript"
	@echo "  make t15-breadth-capture — T-15.16 whole-desktop breadth still (all providers absent)"
	@echo "  make t16-kill-matrix — T-16.8a headless crash/kill matrix transcript"
	@echo "  make t17-robustness-matrix — T-17.5a absent-daemon + crash/kill premium-gate transcript"
	@echo "  make t17-window-loop-capture — T-17.1a nested window-loop stills, clip + transcript"
	@echo "  make t17-navigation-capture — T-17.1b workspace/Mission Control/app-switch stills, clip + transcript"
	@echo "  make t17-flatpak-browser-capture — T-17.1c live Flatpak browser file-choose/screenshot/screen-share stills, clip + transcript"
	@echo "  make t17-visual-floor-capture — T-17.3 visual-floor/reduced-motion chrome stills + gallery review sheet"
	@echo "  make t17-leak-lock-soak — T-17.5b leak tripwire + lock enforcement transcript"
	@echo "  make files-capture — T-10.7 Files slice stills + large-directory scroll trace"
	@echo "  make osd-dnd-capture — T-11.4b OSD card + menu-bar DND still"
	@echo "  make portals-capture — T-13.7 Flatpak portal round-trips + picker still"
	@echo "  make zoo-run — T-14.6a strange-app zoo matrix + desktop still"
	@echo "  make demo     — T-01 loop demo (nested; headless/scripted in CI)"
	@echo "  make lint     — fmt --check, clippy, qmllint, token freshness, desktop-name gate"
	@echo "  make check    — lint + test + teardown soak gate"
	@echo "  make dev      — dragonfruit dev --nested (daily workflow, thin services)"
	@echo "  make dev-full — nested session with settingsd/system-status/etc. on a private bus"
	@echo "                  (FIXTURES=1 for deterministic pane fixtures)"
	@echo "  make install  — lay out session units + shipped wallpaper under DESTDIR/PREFIX"
	@echo "  make soak     — teardown hygiene: N clean cycles (default 100)"
	@echo "  make clean    — remove build artifacts (keeps cargo cache)"

all: build

build: cargo-build cmake-build

cargo-build:
	$(CARGO) build --workspace

# Configure once, then build; `make configure` forces reconfiguration.
cmake-build:
	@[ -f $(BUILD_DIR)/build.ninja ] || $(CMAKE) -S . -B $(BUILD_DIR) -G Ninja
	$(CMAKE) --build $(BUILD_DIR)

configure:
	$(CMAKE) -S . -B $(BUILD_DIR) -G Ninja

test: cargo-test qml-test visual-test

cargo-test:
	$(CARGO) test --workspace

# T-01…T-07 milestone: the Foundation vertical slice (shell + Wayland app +
# X11 app in one live headless session) plus every per-ticket conformance
# suite. This is the fast, CI-able "does the whole thing work together"
# check; `make test` runs it too, this just makes the milestone explicit.
#
# The T-01.6a demo harness is part of the gate: `make demo` with a forced
# headless backend is the scripted half (launch + teardown, assert no leaked
# socket), the same target CI and the human walkthrough use.
#
# The portal integration tests call `dbus::serve` in-process and block on
# zbus from the test thread, so they run single-threaded: parallel blocking
# connections can starve the shared executor and wedge the gate (T107).
e2e: build
	$(CARGO) test -p dragonfruit-compositor \
	    --test milestone_e2e \
	    --test window_conformance \
	    --test xwayland_conformance \
	    --test xdnd_conformance \
	    --test shell_protocol_conformance \
	    --test shell_idle_trace \
	    --test idle_trace \
	    --test latency_trace \
	    --test animation_clock \
	    --test reduced_motion_sweep \
	    --test protocol_surface \
	    --test session_lock_conformance \
	    --test suspend_resume_conformance \
	    --test drm_bringup \
	    --test input_validation \
	    --test multi_gpu
	$(CARGO) test -p dragonfruit-system-adapters
	$(CARGO) test -p dragonfruit-networkmanager
	$(CARGO) test -p dragonfruit-audio
	$(CARGO) test -p dragonfruit-power
	$(CARGO) test -p dragonfruit-bluetooth
	$(CARGO) test -p dragonfruit-storage
	$(CARGO) test -p dragonfruit-input
	$(CARGO) test -p dragonfruit-overview
	$(CARGO) test -p dragonfruit-notify-adapter
	$(CARGO) test -p dragonfruit-lock-adapter
	$(CARGO) test -p dragonfruit-menubar-adapter
	$(CARGO) test -p dragonfruit-update-adapter
	$(CARGO) test -p dragonfruit-account-adapter
	$(CARGO) test -p dragonfruit-printer-adapter
	$(CARGO) test -p dragonfruit-privacy-adapter
	$(CARGO) test -p dragonfruit-accessibility-adapter
	$(CARGO) test -p dragonfruit-system-status
	$(CARGO) test -p dragonfruit-settingsd
	$(CARGO) test -p dragonfruit-files-core
	$(CARGO) test -p dragonfruit-lock-auth
	$(CARGO) test -p dragonfruit-session
	$(CARGO) test -p dragonfruit-app-index
	$(CARGO) test -p dragonfruit-wallpaperd
	$(CARGO) test -p xdg-desktop-portal-dragonfruit -- --test-threads=1
	$(MAKE) demo DEMO_ARGS=--headless

# T-03.1a: the idle/animation frame budget trace. `make e2e` runs the same
# test at the short in-suite window; this target sets the acceptance window
# (default 60 s, override with IDLE_TRACE_SECS) and streams the raw counters
# (`scripts/idle-trace.sh` wraps it and records the log).
idle-trace: cargo-build
	DF_IDLE_TRACE_SECS=$(IDLE_TRACE_SECS) $(CARGO) test -p dragonfruit-compositor \
	    --test idle_trace -- --nocapture

# T-07.6b: the menu-bar idle trace — the mapped menubar chrome surface sits
# idle with the live status items, and must contribute zero frames and zero
# client wakeups (FR-6). `scripts/capture-live-menubar.sh` records the raw
# output to docs/captures/t07-shell-idle-trace.txt.
menubar-idle-trace: cargo-build
	$(CARGO) test -p dragonfruit-compositor \
	    --test shell_idle_trace -- --nocapture

# T-03.1b: the nested input-to-photon latency capture. Needs a host Wayland
# session and the built toolchain tree (`make build`); records the raw samples
# to docs/captures/t03-latency-nested.txt. `scripts/latency-trace.sh` wraps it.
latency-trace: cargo-build
	bash scripts/latency-trace.sh

# T-03.2: the DRM first bring-up probe. On a host with a free seat it starts
# the DRM backend and records the capture/trace; with no free seat it records
# an explicit OPEN marker (the unit is marked open, not skipped). Always exits
# 0 — the hardware rail consumes the artifact, not the exit code.
drm-bringup: cargo-build
	bash scripts/drm-bringup.sh

# T-03.3: the hardware input validation matrix. Needs the compositor to own a
# libinput seat; with no free seat it records an explicit OPEN matrix (the unit
# is marked open, not skipped) and the hardware rail sweeps it. Always exits 0.
input-validation: cargo-build
	bash scripts/input-validation.sh

# T-03.4: multi-GPU import/fallback validation. With one card the hardware half
# is recorded OPEN (not skipped); always exits 0. The pure classification is
# pinned by compositor/tests/multi_gpu.rs.
multi-gpu-validation: cargo-build
	bash scripts/multi-gpu-validation.sh

# T-03.4: the 100-cycle automated teardown soak plus one DRM session cycle.
# The soak always runs; the DRM cycle needs a free logind seat, and with none
# it is recorded OPEN (not skipped). Always exits 0.
drm-soak: cargo-build
	bash scripts/drm-soak.sh

# T-12.6c: the second-VT dev harness. Records the dedicated-user refusal and
# the mocked-loginctl VT selection always; the one real start → switch → return
# → teardown cycle needs a free logind seat and the dedicated user, and with
# neither it is recorded OPEN (not skipped). Always exits 0.
second-vt-validation: cargo-build
	bash scripts/second-vt-validation.sh

# T-08.3: the settingsd flip + restart/resync track capture. Needs a host
# Wayland session, `spectacle`, `ffmpeg`, and Pillow; records the stills,
# transcript and clip under docs/captures/t08-settingsd.*. `scripts/
# capture-settingsd.sh` also `kill -9`s and restarts settingsd.
settingsd-capture: build
	bash scripts/capture-settingsd.sh

# T-09.6b: the Settings Wave-1 track capture. Needs a host Wayland session,
# `spectacle`, `ffmpeg`, Pillow, and the built tree; starts a scratch settingsd,
# opens each shipped pane in the nested demo, and writes the light/dark/
# reduced-motion and per-pane stills under docs/captures/t09-settings-wave-1.*.
settings-wave-1-capture: build
	bash scripts/capture-settings-wave-1.sh

# T-18.2: the Wallpaper-pane capture. Needs a host Wayland session, `spectacle`,
# Pillow, `gdbus`, the built tree and `cargo build -p dragonfruit-wallpaperd`;
# opens the Wallpaper pane with a scratch provider and writes the fetching
# (skeleton) and filled stills under docs/captures/t18-wallpaper.*.
t18-wallpaper-capture: build
	bash scripts/capture-t18-wallpaper.sh

# T-18.3: the headless absence/state matrix. Writes the reproduction transcript
# for every matrix row under docs/captures/t18-absence-matrix.txt. No host
# session and no network are required (offline is a dead proxy).
t18-absence-matrix: build
	bash scripts/t18-absence-matrix.sh

# T-15.16: the Wave-2/3 absent-daemon masking matrix. Writes the headless
# reproduction transcript for every shipped pane under
# docs/captures/t15-absence-matrix.txt. No host session and no daemons are
# required (absence is the state under test).
t15-absence-matrix: build
	bash scripts/t15-absence-matrix.sh

# T-15.16: the breadth capture. Needs a host Wayland session, `spectacle`,
# Pillow and the built tree; runs the nested demo with every provider absent
# and writes the whole-desktop still under docs/captures/t15-breadth.png.
t15-breadth-capture: build
	bash scripts/capture-t15-breadth.sh

# T-16.6a: the live AT-SPI dump and keyboard-only walkthrough. Needs a host
# Wayland session, python3 with pyatspi + Pillow, `spectacle` and the built
# tree; runs the nested demo twice (Settings and Files) with Qt accessibility
# on, dumps the accessibility trees, and drives a keyboard-only walkthrough
# over the synthetic-input harness. Writes docs/captures/t16-a11y-atspi.txt
# and t16-a11y-desktop.png.
t16-a11y-audit: build
	bash scripts/t16-a11y-audit.sh

# T-16.7: the locale-switch capture. Needs a host Wayland session, `spectacle`,
# Pillow and the built tree; runs the nested demo with `DRAGONFRUIT_LOCALE=es_ES`
# and writes the English/Spanish A/B stills under docs/captures/t16-i18n-*.png.
t16-i18n-capture: build
	bash scripts/capture-t16-i18n.sh

# T-16.8a: the headless crash/kill matrix. Writes the reproduction transcript
# for every restartable component under docs/captures/t16-kill-matrix.txt. No
# host session and no real services are required (stand-ins + the headless
# backend are the state under test).
t16-kill-matrix: cargo-build
	bash scripts/t16-kill-matrix.sh

# T-17.5a: the premium-gate absent-daemon and crash/kill matrix. Writes the
# verification transcript under docs/captures/t17-robustness-matrix.txt: the
# session starts with every optional service absent, the T-15.16 absence matrix
# and the T-16.8a/T-16.8b kill/restart matrices re-run on the release tree, and
# the app-crash/lock kill-resistance rows. Headless; no host session, no VM.
t17-robustness-matrix: build
	bash scripts/t17-robustness-matrix.sh

# T-17.1a: the nested window-loop capture. Needs a host Wayland session,
# `spectacle`, python3+Pillow, ffmpeg (optional clip) and the built tree; runs
# launch/appear → focus → move → zoom → minimize → restore → close on a
# third-party Qt SSD client (kcalc), plus the CSD-unaffected and X11 SSD
# checks, writing the stills, clip and transcript under
# docs/captures/t17-window-loop.*.
t17-window-loop-capture: build
	bash scripts/capture-t17-window-loop.sh

# T-17.1b: the nested navigation capture. Needs a host Wayland session,
# `spectacle`, python3+Pillow, ffmpeg (optional clip) and the built tree; drives
# the workspace switch (keyboard + gesture + Mission Control strip click),
# Mission Control (keyboard + hot-corner pointer), and app switch (keyboard +
# preview click) through the synthetic-input harness, asserting each path via
# `query spaces`/`grid`/`wallpaper`/`switcher` and writing the stills, clip and
# transcript under docs/captures/t17-navigation.*.
t17-navigation-capture: build
	bash scripts/capture-t17-navigation.sh

# T-17.1c: the live Flatpak/browser capture. Needs a host Wayland session,
# `flatpak` with org.mozilla.firefox, /usr/libexec/xdg-desktop-portal,
# `spectacle`, python3 (host and sandbox) with PyGObject/Pillow, ffmpeg
# (optional clip) and the built tree. Runs the real Flatpak browser against the
# nested session with the live shell as the portal presenter, completing
# file-choose, screenshot, and screen-share by synthetic input and writing the
# stills, clip and transcript under docs/captures/t17-flatpak-browser.*.
t17-flatpak-browser-capture: build
	bash scripts/capture-t17-flatpak-browser.sh

# T-17.3: the visual-floor and reduced-motion sign-off capture. Needs a host
# Wayland session, `spectacle`, python3+Pillow, ffmpeg (optional) and the built
# tree; runs the nested demo once and captures the live chrome under dark,
# light, and dark+reduced motion (whole desktop, menu-bar, SSD titlebar, Dock,
# window menu), records the live `query material`/`query degrade` tones, and
# composes the review sheet against the design-system goldens under
# docs/captures/t17-visual-floor*.
t17-visual-floor-capture: build
	bash scripts/capture-t17-visual-floor.sh

# T-17.5b: leak and lock enforcement verification. Headless; needs only the
# built tree and python3. Runs `dragonfruit dev --soak 100` against an isolated
# XDG_RUNTIME_DIR and asserts the directory is empty afterwards, re-runs the
# session-lock conformance suite (including the 25-cycle no-leak loop) and
# asserts its scratch dir is empty, then records the DRM/VT master probe. The
# transcript under docs/captures/t17-leak-lock-soak.txt is the evidence; the
# reviewed note is docs/captures/t17-leak-lock-soak.md.
t17-leak-lock-soak: cargo-build
	bash scripts/t17-leak-lock-soak.sh

# T-10.7: the Files slice capture. Needs a host Wayland session, `spectacle`,
# `ffmpeg`, `gdbus`, Pillow and the built tree; drives the nested demo over the
# synthetic-input harness and writes the stills, clip and large-directory
# scroll trace under docs/captures/t10-files.*.
files-capture: build
	bash scripts/capture-files.sh

# T-11.4b: the OSD + DND stills. Needs a host Wayland session, `spectacle`,
# Pillow and the built tree; runs the nested demo with the status/notification
# fixtures and drives a real Control Center volume drag under docs/captures/
# t11-osd.* and t11-dnd.png.
osd-dnd-capture: build
	bash scripts/capture-osd-dnd.sh

# T-13.7: the portal track capture. Needs a host Wayland session, `flatpak`
# with a browser, python3+PyGObject, `spectacle`, Pillow and the built tree;
# runs the real frontend + backend on a private bus and a Flatpak client, then
# writes docs/captures/t13-portals.txt and t13-portals.png.
portals-capture: build
	bash scripts/capture-portals.sh

# T-12.5b: the T-12 session track capture. Needs a host Wayland session,
# `spectacle`, `ffmpeg`, Pillow and the built tree; locks the nested session
# with the real Cmd+Ctrl+Q shortcut and writes the desktop/lock stills, a
# short clip and the `query lock`/`query session` transcript under
# docs/captures/t12-session.*.
session-capture: build
	bash scripts/capture-session.sh

# T-14.6a: the strange-app zoo run. Needs a host Wayland session, `spectacle`,
# python3+Pillow, gcc + SDL2/X11 dev headers, an Electron install
# (ZOO_ELECTRON_DIR), the GNOME Calculator flatpak, and the built tree; writes
# docs/captures/t14-zoo.png and t14-zoo-matrix.{md,json}. Missing apps are
# recorded as "not run" rather than failing the target.
zoo-run: build
	bash scripts/zoo/zoo-run.sh

# T-14.7a: the Dock plate geometry + spacing stills. Needs a host Wayland
# session, `spectacle`, Pillow and the built tree; runs the nested demo with a
# scratch settingsd and writes the six position/scheme crops under
# docs/captures/t14-dock-spacing-*.png.
dock-spacing-capture: build
	bash scripts/capture-dock-spacing.sh

# T-14.7b: the Dock magnification sweep stills (left/centre/right × dark/light)
# and the pointer-sweep frame-budget probe. Needs a host Wayland session,
# `spectacle`, Pillow and the built tree.
dock-magnify-capture: build
	bash scripts/capture-dock-magnify.sh

# T-14.7y: the Dock magnification slow-sweep strips (six crops across a
# left-to-right sweep × light/dark, with the pre-fix row stacked above the
# fixed row). Needs a host Wayland session, `spectacle`, Pillow and the built
# tree.
dock-magnify-sweep-capture: build
	bash scripts/capture-dock-magnify-sweep.sh

# T-14.7c: the Dock motion frame trace (magnification sweep + context menu
# open/close + a `dock.size` change) and its two stills. Needs a host Wayland
# session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-motion-capture: build
	bash scripts/capture-dock-motion.sh

# T-14.7d: the Trash entry artwork stills (empty/full/unavailable × dark/light).
# Needs a host Wayland session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-trash-capture: build
	bash scripts/capture-dock-trash.sh

# T-14.7r: the Empty Trash progress/result stills (busy/success × dark/light,
# plus the busy-over-success composite). Uses the shell's trash-empty capture
# seam; needs a host Wayland session, `spectacle`, `gdbus`, Pillow and the
# built tree.
dock-trash-empty-capture: build
	bash scripts/capture-dock-trash-empty.sh

# T-14.7s: the minimize-to-icon reaction stills (bottom-light + right-dark,
# plus the composite). Uses the shell's minimize-reaction capture seam; needs a
# host Wayland session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-minimize-reaction-capture: build
	bash scripts/capture-dock-minimize-reaction.sh

# T-14.7t: the keyboard-reorder still (focus ring on the moved pinned entry),
# captured through the real compositor input path (Ctrl+F3 focuses the Dock,
# then Ctrl+Shift+Right reorders). Needs a host Wayland session, `spectacle`,
# `gdbus`, Pillow and the built tree.
dock-keyboard-reorder-capture: build
	bash scripts/capture-dock-keyboard-reorder.sh

# T-14.7e: the Add Application picker still (open, filtered, with a pinned row).
# Needs a host Wayland session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-app-picker-capture: build
	bash scripts/capture-dock-app-picker.sh

# T-14.7f: the Dock drop identity/affordance stills (app ghost, file→app,
# file→Trash) stacked into docs/captures/t14-dock-drops.png. Needs a host
# Wayland session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-drops-capture: build
	bash scripts/capture-dock-drops.sh

# T-14.7g: the Dock launch/activate/missing stills. Needs a host Wayland
# session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-activation-capture: build
	bash scripts/capture-dock-activation.sh

# T-14.7l: the Dock launch-origin still (the launched window's appear originates
# at the pinned entry's icon) plus the recorded motion trace. Needs a host
# Wayland session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-launch-origin-capture: build
	bash scripts/capture-dock-launch-origin.sh

# T-14.7h: the Dock folder stack stills (resting, open, empty, long) stacked
# into docs/captures/t14-dock-folder-stack.png. Needs a host Wayland session,
# `spectacle`, `gdbus`, Pillow and the built tree.
dock-folder-stack-capture: build
	bash scripts/capture-dock-folder-stack.sh

# T-14.7i: the Dock hover name label stills (app, folder, Trash) stacked into
# docs/captures/t14-dock-tooltip.png. Needs a host Wayland session, `spectacle`,
# `gdbus`, Pillow and the built tree.
dock-tooltip-capture: build
	bash scripts/capture-dock-tooltip.sh

# T-14.7k: the Dock folder pin stills (pinned tile, open popover) stacked into
# docs/captures/t14-dock-folder-pin.png. Needs a host Wayland session,
# `spectacle`, `gdbus`, Pillow and the built tree.
dock-folder-pin-capture: build
	bash scripts/capture-dock-folder-pin.sh

# T-14.7m: the Dock window chooser per-window action stills (hovered row with
# Minimize + destructive Close, light and dark) stacked into
# docs/captures/t14-dock-chooser-actions.png. Needs a host Wayland session,
# `spectacle`, `gdbus`, Pillow and the built tree.
dock-chooser-actions-capture: build
	bash scripts/capture-dock-chooser-actions.sh

# T-14.7n: the Dock window chooser long-list stills (capped at 7 rows, scrolled
# mid-list, light and dark) stacked into
# docs/captures/t14-dock-chooser-scroll.png. Needs a host Wayland session,
# `spectacle`, `gdbus`, Pillow and the built tree.
dock-chooser-scroll-capture: build
	bash scripts/capture-dock-chooser-scroll.sh

# T-14.7o: the Dock window-count badge stills (a grouped app shows its count,
# a single-window app shows none; light, dark, and min/max icon sizes) stacked
# into docs/captures/t14-dock-window-badge.png. Needs a host Wayland session,
# `spectacle`, `gdbus`, Pillow and the built tree.
dock-window-badge-capture: build
	bash scripts/capture-dock-window-badge.sh

# T-14.7p: the Dock hover-open chooser stills (`dock.chooserOnHover` on, a
# dwell-open popover and a retarget between two grouped entries; light over
# dark) stacked into docs/captures/t14-dock-hover-chooser.png. Needs a host
# Wayland session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-hover-chooser-capture: build
	bash scripts/capture-dock-hover-chooser.sh

# T-14.7q: the Dock overflow cell and its open "More Windows" list (the capture
# seam produces a genuine overflow at a normal nested size; light over dark)
# stacked into docs/captures/t14-dock-overflow.png. Needs a host Wayland
# session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-overflow-capture: build
	bash scripts/capture-dock-overflow.sh

# T-14.7v: the Dock region-divider composites (three regions / empty tail /
# single region, light and dark) stacked into
# docs/captures/t14-dock-dividers-{light,dark}.png. Needs a host Wayland
# session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-dividers-capture: build
	bash scripts/capture-dock-dividers.sh

# T-14.7j: the Dock Tahoe visual language stills (resting/hovered/magnified/
# pressed in light and dark, plus the Minimal tier) stacked into
# docs/captures/t14-dock-tahoe-{light,dark,reduced}.png. Needs a host Wayland
# session, `spectacle`, Pillow and the built tree.
dock-tahoe-capture: build
	bash scripts/capture-dock-tahoe.sh

# T-14.7z: the Dock plate corner close-ups and the mid-magnification strip
# (docs/captures/t14-dock-plate-corners-{light,dark}.png and
# docs/captures/t14-dock-plate-magnified-{light,dark}.png). Needs a host
# Wayland session, `spectacle`, Pillow and the built tree.
dock-plate-corners-capture: build
	bash scripts/capture-dock-plate-corners.sh

# T-14.7w: the three test-icon tiles (square / padded / round) masked into the
# Dock tile squircle (docs/captures/t14-dock-icon-mask-{light,dark}.png). Needs
# a host Wayland session, `spectacle`, `gdbus`, Pillow and the built tree.
dock-icon-mask-capture: build
	bash scripts/capture-dock-icon-mask.sh

qml-test:
	@[ -f $(BUILD_DIR)/build.ninja ] || $(CMAKE) -S . -B $(BUILD_DIR) -G Ninja
	$(CMAKE) --build $(BUILD_DIR) >/dev/null
	$(CTEST) --test-dir $(BUILD_DIR) --output-on-failure

# T-08 FR-6: headless gallery visual regression. Deterministic token/pixel
# invariants; `make gallery-snapshot` regenerates the art-direction goldens.
visual-test: cmake-build
	./scripts/check-gallery-snapshots.py

gallery-snapshot: cmake-build
	./scripts/check-gallery-snapshots.py --update

# T-08 FR-2: Theme.qml and design_tokens.rs must be generated from the one
# source. Fails if either is stale.
check-tokens:
	./scripts/gen-tokens.py --check

# T-19.1a: the Phosphor registry must match the vendored SVGs, and every QML
# glyph reference must name a vendored file (a typo fails the build).
check-phosphor:
	./scripts/gen-phosphor-glyphs.py --check
	./scripts/gen-app-icons.py --check
	./scripts/check-phosphor-icons.py

lint: fmt-check clippy qml-test check-tokens check-design-tokens check-desktop-names check-no-capture-grab check-i18n check-phosphor

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

check-desktop-names:
	./scripts/check-desktop-names.sh

# T-02 FR-8: capture is portal-only — no screencopy-style grabs, ever.
check-no-capture-grab:
	./scripts/check-no-capture-grab.sh

# T-08 FR-1/FR-2: design-system components consume tokens, never literals.
check-design-tokens:
	./scripts/check-design-tokens.sh

# T-16.7: string-extraction gate. Every `qsTr()`/`tr()` string in the shell,
# apps, and design-system components must be in the checked-in catalog template,
# no template entry may be stale, and no user-visible QML property may hold a
# bare literal. `make i18n-update` regenerates the template from the sources.
check-i18n:
	./scripts/i18n-extract.py --check

i18n-update:
	./scripts/i18n-extract.py --update

# The full gate: everything CI runs, locally in one command.
check: lint test soak

dev: build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --nested --shell

# The full nested dev session: the thin loop's core services (app-index,
# menu-broker) plus the real session services (settingsd, system-status,
# notifications, wallpaperd) on a private session bus with scratch XDG dirs,
# so Settings, Control Center, and the wallpaper provider are live without
# touching the host desktop or the developer's real settings. `FIXTURES=1`
# swaps in the deterministic pane fixtures for hosts whose daemons/hardware
# are absent. `make demo`/`make dev` stay thin and never start these.
dev-full: build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --nested --shell \
	    --services full --private-bus $(if $(FIXTURES),--fixtures,) $(DEV_FULL_ARGS)

# T-01.6a: one command that builds and launches the loop demo. With a host
# Wayland session it opens the nested compositor for the human walkthrough
# (shell + a Qt/Wayland app + an X11 app + the printed checklist); with none
# — CI — it runs the headless scripted half. `DEMO_ARGS=--headless` forces
# the scripted path on a dev machine (`make e2e` does this).
demo: build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --demo $(DEMO_ARGS)

soak: cargo-build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --soak $(SOAK_CYCLES)

# The packaging seam (T-12.2/T-18.1a): lay out the display-manager entry, the
# systemd user units, and the shipped default wallpaper under
# $(DESTDIR)$(PREFIX). T-32 packaging calls this.
install: cargo-build
	$(CARGO) run -p dragonfruit-session -- --install-session $(DESTDIR)$(PREFIX)

clean:
	rm -rf $(BUILD_DIR)
	$(CARGO) clean
