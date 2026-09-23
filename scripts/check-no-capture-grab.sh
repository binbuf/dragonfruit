#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-02 FR-8: no `wlr-screencopy`-style arbitrary-grab capture protocols
# are implemented or advertised, ever. Capture is portal-only
# (docs/design/02-compositor.md). This gate greps the compositor and the
# protocol XMLs for any screencopy/export-dmabuf/image-capture *use*; a
# hit fails the build. (Doc comments explaining the prohibition and the
# negative test's forbidden list are exempt: they reference the names
# only to keep them out.) The portal backend (T-27) captures via PipeWire
# inside the compositor's own process — it must not bind the protocol
# globals named below either.

set -euo pipefail
cd "$(dirname "$0")/.."

violations=$(grep -RniE \
    'zwlr_screencopy|zwlr_export_dmabuf|ext_output_image_capture|wlr-screencopy|screencopy_manager' \
    compositor/src compositor/Cargo.toml protocols/ portal/ 2>/dev/null \
    | grep -vE ':[0-9]+:[[:space:]]*(///|//!|//|\*)' || true)

if [ -n "$violations" ]; then
    echo "FAIL: capture-grab protocol references found (capture is portal-only):"
    echo "$violations"
    exit 1
fi

echo "no capture-grab protocol references (FR-8) — ok"
