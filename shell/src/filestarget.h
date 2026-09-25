// SPDX-License-Identifier: MIT
// The path a Dock "Show in Files" reveals (T-10.6c).
//
// Pure and Wayland-free so `tst_dockcore` can unit-test it. The Dock hands
// Files an absolute path; a `.desktop` Exec may be a bare program name, so the
// resolver makes it absolute (PATH-resolved) before Files ever sees it. Files
// then opens the containing folder and selects the item (`filesOpenTarget`,
// `apps/files/FilesArguments.h`).
#pragma once

#include "desktopentry.h"

#include <QString>

// The absolute executable path to reveal for `entry` — the first token of its
// launch command. An already-absolute path is returned as written; a bare
// program name is resolved on `PATH`. Empty when there is nothing revealable.
QString revealExecutable(const DesktopEntry &entry);