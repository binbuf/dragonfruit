// SPDX-License-Identifier: MIT
// Pure external-drop resolution for the Dock (T-10, section 12).
//
// External drops arrive from another client (Files, a launcher) as a
// `text/uri-list` payload. This module has no Wayland, QML, or state: it
// parses the payload and decides what a drop on a given entry does, so the
// decision is unit-testable and the shell only performs the resulting action.
#pragma once

#include <QByteArray>
#include <QString>
#include <QStringList>

// What an external drag carries.
enum class DockDropPayload {
    Files,       // one or more file/folder URIs
    Application, // an application alias (a `.desktop` file or app mime)
};

// What a drop does, resolved from the target entry and the payload.
enum class DockDropAction {
    None,            // empty Dock, divider, or a nonsensical pairing
    PinApp,          // an application alias dropped on the Dock pins it
    OpenWithApp,     // files dropped on an app entry open with that app
    TrashFiles,      // files dropped on the Trash move to trash
    MoveToDownloads, // files dropped on the Downloads stack move into it
};

// Parse a `text/uri-list` payload (RFC 2483): one URI per line, `#` comments
// and blank lines ignored, CRLF tolerated. Only `file://` URIs are returned,
// as local paths; a non-file URI is dropped. Percent escapes are decoded.
QStringList parseUriList(const QByteArray &data);

// True when the URI list names a single application alias (a `.desktop`
// file), which the Dock pins rather than opening a file with.
bool uriListIsApplication(const QStringList &paths);

// The desktop id for a `.desktop` path (`/usr/share/applications/foo.desktop`
// -> `foo.desktop`), or an empty string when the path is not a desktop file.
QString desktopIdForFile(const QString &path);

// Resolve the action for a drop on `targetKind` ("pinned" | "temporary" |
// "recent" | "minimized" | "trash" | "stack" | "divider" | ""). An
// application alias always pins; files follow the target (app entry, Trash,
// or Downloads stack) and are a no-op anywhere else.
DockDropAction dockDropActionFor(const QString &targetKind, DockDropPayload payload);

// Spring-loading hover delay (ms), shared with Files (T-18). Dragging over a
// folder/stack for this long opens it; the hook exists but stacks are out of
// the first vertical slice (T-10 section 17).
constexpr int kSpringLoadMs = 500;

// The Downloads folder (`$XDG_DOWNLOAD_DIR` when set, else `~/Downloads`).
QString downloadsDirectory();
