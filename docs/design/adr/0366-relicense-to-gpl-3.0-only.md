# 0366 — Relicense the first-party tree to GPL-3.0-only

Status: accepted (2026-11-25)

## Context

The project launched with a repo-wide MIT license. MIT maximised permissive
reuse, and it let any project — another compositor, a toolkit, a client
library, or a sandboxed app — embed Dragonfruit code without obligations.
That is no longer the goal: the desktop and its improvements should stay free
software, including when a fork, downstream build, or appliance ships a
modified version.

The surrounding stack already fits copyleft terms. Qt is used under the
(L)GPL, and the WhiteSur place artwork is GPL-3.0-only. The Rust dependency
graph is permissive (MIT, Apache-2.0, BSD, ISC, Zlib, Unlicense), so it remains
compatible with a GPL-3.0-only first-party tree. Earlier releases were
published under MIT; that grant cannot be withdrawn for those releases, and
this ADR does not attempt to do so.

## Decision

- **License every first-party component under GPL-3.0-only**, repo-wide:
  compositor, portal, services, tools, `df-ipc`, Dragonfruit protocol XMLs,
  design system, shell, apps, scripts, and build files. The normative policy
  is [licensing.md](../licensing.md).
- **Use the SPDX identifier `GPL-3.0-only`.** Every source file carries an
  `SPDX-License-Identifier: GPL-3.0-only` header, and every crate/package
  directory carries a `LICENSE` file pointing at
  `LICENSES/GPL-3.0-only.txt`.
- **Keep third-party components under their own licenses.** Vendored assets
  (Inter OFL, Tabler/Phosphor MIT, WhiteSur GPL-3.0-only, Capitaine cursors
  LGPL-3.0-or-later) and system/optional services keep their existing terms
  and notices in [NOTICE](../../NOTICE) and the asset manifest.
- **Require GPL-3.0-only compatibility.** AGPL, GPL-2.0-only, and other
  GPL-3.0-incompatible code must not be linked into a Dragonfruit binary.
  Optional services stay separate processes or dynamically linked libraries;
  their code is never copied into the first-party tree.
- **Accept contributions inbound = outbound under GPL-3.0-only.** Prior
  contributions were accepted under MIT, which permits sublicensing; future
  contributions are accepted under GPL-3.0-only. Legal review remains a
  blocker for the first public release.
- **Meet distribution obligations.** Every binary distribution ships the
  GPL-3.0 text and the corresponding source for the distributed version.
  Packaging metadata uses `GPL-3.0-only` (the Fedora spec `License:` field and
  the release assets).

## Consequences

- Forks and downstream distributions must publish their modified source under
  GPL-3.0-only. Code cannot be taken into a proprietary product.
- Projects that need permissive embedding can no longer reuse first-party
  Dragonfruit code; the documented third-party interfaces remain available
  under GPL-3.0 terms.
- The permissive Rust dependency graph stays compatible, so no crate changes
  are required. Adding an AGPL or GPL-2.0-only dependency is now a review
  blocker because it cannot be distributed as part of the combined work.
- The private protocol XMLs and `df-ipc` contracts remain open for third-party
  implementation, now under GPL-3.0-only.
- Documentation, badges, package metadata, and per-package license stubs point
  at GPL-3.0-only; the MIT text stays in `LICENSES/MIT.txt` for the vendored
  Tabler and Phosphor sets.

## Verification

- `git grep "SPDX-License-Identifier: MIT"` returns nothing outside third-party
  license texts.
- The `protocol_xmls_match_lockstep_version` test in `protocols/df-ipc`
  asserts the GPL-3.0-only marker on every Dragonfruit protocol XML.
- `make check-assets` validates the first-party asset rows and their
  `LICENSES/GPL-3.0-only.txt` license text.
- `make verify-fast` covers the build and test suites affected by the header
  change.
