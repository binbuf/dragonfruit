// SPDX-License-Identifier: MIT
// The desktop's system font (Inter, SIL OFL 1.1).
//
// One installation per process: register the bundled faces and make their
// family the application default, so every QML surface that does not name a
// family (all of them) renders in the desktop's type — the role SF Pro plays
// on macOS. Call it after QGuiApplication exists and before any QML loads.

#pragma once

namespace Dragonfruit {

// Idempotent: repeat calls are a no-op. On failure (resource missing, family
// unavailable) it warns and leaves the host default font in place rather than
// aborting the process.
void installSystemFont();

} // namespace Dragonfruit