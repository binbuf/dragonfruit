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
IDLE_TRACE_SECS ?= 60
DF_TOOLCHAIN ?= $(HOME)/.local/df-toolchain/usr
DF_DEVROOT ?= $(HOME)/.local/df-devroot/lib64

# Discover cmake/ninja from the local Qt toolchain prefix when they are
# not on PATH (see README "Toolchains").
ifeq ($(shell command -v $(CMAKE) >/dev/null 2>&1 || command -v $(CTEST) >/dev/null 2>&1 || echo missing),missing)
ifneq ($(wildcard $(DF_TOOLCHAIN)/bin/cmake),)
export PATH := $(DF_TOOLCHAIN)/bin:$(PATH)
export LD_LIBRARY_PATH := $(DF_TOOLCHAIN)/lib64$(if $(LD_LIBRARY_PATH),:$(LD_LIBRARY_PATH))
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
        visual-test gallery-snapshot check-tokens lint fmt fmt-check clippy check dev demo soak e2e \
        idle-trace menubar-idle-trace latency-trace settingsd-capture settings-wave-1-capture \
        files-capture check-desktop-names check-no-capture-grab check-design-tokens clean

help:
	@echo "Dragonfruit build targets:"
	@echo "  make build    — build everything (Rust workspace + Qt/CMake)"
	@echo "  make test     — run all tests (cargo + ctest + gallery visual regression)"
	@echo "  make e2e      — T-01…T-07 Foundation vertical-slice + conformance suites"
	@echo "  make idle-trace — T-03.1a 60 s idle/animation frame budget trace"
	@echo "  make latency-trace — T-03.1b nested input-to-photon latency capture"
	@echo "  make settingsd-capture — T-08.3 settingsd flip + restart capture"
	@echo "  make settings-wave-1-capture — T-09.6b Settings wave stills (light/dark/reduced + panes)"
	@echo "  make files-capture — T-10.7 Files slice stills + large-directory scroll trace"
	@echo "  make demo     — T-01 loop demo (nested; headless/scripted in CI)"
	@echo "  make lint     — fmt --check, clippy, qmllint, token freshness, desktop-name gate"
	@echo "  make check    — lint + test + teardown soak gate"
	@echo "  make dev      — dragonfruit dev --nested (daily workflow)"
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
e2e: build
	$(CARGO) test -p dragonfruit-compositor \
	    --test milestone_e2e \
	    --test window_conformance \
	    --test xwayland_conformance \
	    --test shell_protocol_conformance \
	    --test shell_idle_trace \
	    --test idle_trace \
	    --test latency_trace \
	    --test animation_clock \
	    --test protocol_surface
	$(CARGO) test -p dragonfruit-system-adapters
	$(CARGO) test -p dragonfruit-networkmanager
	$(CARGO) test -p dragonfruit-audio
	$(CARGO) test -p dragonfruit-power
	$(CARGO) test -p dragonfruit-system-status
	$(CARGO) test -p dragonfruit-settingsd
	$(CARGO) test -p dragonfruit-files-core
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

# T-10.7: the Files slice capture. Needs a host Wayland session, `spectacle`,
# `ffmpeg`, `gdbus`, Pillow and the built tree; drives the nested demo over the
# synthetic-input harness and writes the stills, clip and large-directory
# scroll trace under docs/captures/t10-files.*.
files-capture: build
	bash scripts/capture-files.sh

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

lint: fmt-check clippy qml-test check-tokens check-design-tokens check-desktop-names check-no-capture-grab

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

# The full gate: everything CI runs, locally in one command.
check: lint test soak

dev: build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --nested --shell

# T-01.6a: one command that builds and launches the loop demo. With a host
# Wayland session it opens the nested compositor for the human walkthrough
# (shell + a Qt/Wayland app + an X11 app + the printed checklist); with none
# — CI — it runs the headless scripted half. `DEMO_ARGS=--headless` forces
# the scripted path on a dev machine (`make e2e` does this).
demo: build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --demo $(DEMO_ARGS)

soak: cargo-build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --soak $(SOAK_CYCLES)

clean:
	rm -rf $(BUILD_DIR)
	$(CARGO) clean
