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
        visual-test gallery-snapshot check-tokens lint fmt fmt-check clippy check dev soak e2e \
        check-desktop-names check-no-capture-grab check-design-tokens clean

help:
	@echo "Dragonfruit build targets:"
	@echo "  make build    — build everything (Rust workspace + Qt/CMake)"
	@echo "  make test     — run all tests (cargo + ctest + gallery visual regression)"
	@echo "  make e2e      — T-01…T-07 Foundation vertical-slice + conformance suites"
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
e2e: cargo-build
	$(CARGO) test -p dragonfruit-compositor \
	    --test milestone_e2e \
	    --test window_conformance \
	    --test xwayland_conformance \
	    --test shell_protocol_conformance \
	    --test idle_trace \
	    --test protocol_surface

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

soak: cargo-build
	$(CARGO) run -p dragonfruit-dev --bin dragonfruit -- dev --soak $(SOAK_CYCLES)

clean:
	rm -rf $(BUILD_DIR)
	$(CARGO) clean
