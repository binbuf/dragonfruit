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

// The parsed payload of one external drag, cached at drag *enter* so the drop
// can reuse it without a second receive (a Wayland data offer is readable
// once). `kind` is Application for the `application/x-dragonfruit-app` alias
// mime or a single `.desktop` URI list, otherwise Files. `valid` is false only
// for a mime the Dock does not consume.
struct DockDropPayloadData {
    DockDropPayload kind = DockDropPayload::Files;
    QString desktopId;  // set when `kind == Application`
    QStringList paths;  // set when `kind == Files`
    bool valid = false;
};

// Classify and parse a drag payload from its offered `mime` and the bytes the
// source wrote. This is the one payload decoder shared by the enter-time read
// and the drop path.
DockDropPayloadData parseDockDropPayload(const QString &mime, const QByteArray &data);

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

// The hover affordance text for a drop on `targetKind`, or an empty string
// when the pairing is a no-op (empty Dock, divider, a pin on Trash/stack).
// `targetName` is the app entry's display name for the "Open with" hint;
// `trashAvailable == false` reports the unavailable Trash (T-14.7f).
QString dockDropAffordance(const QString &targetKind, DockDropPayload payload,
                           const QString &targetName, bool trashAvailable);

// True when `pinned` already holds `id` or its resolved identity `resolvedId`,
// so a duplicate alias drop can be a visible no-op instead of a second append.
bool dockPinnedContains(const QStringList &pinned, const QString &id,
                        const QString &resolvedId);

// Spring-loading hover delay (ms), shared with Files (T-18). Dragging over a
// folder/stack for this long opens it; the hook exists but stacks are out of
// the first vertical slice (T-10 section 17).
constexpr int kSpringLoadMs = 500;

// The Downloads folder (`$XDG_DOWNLOAD_DIR` when set, else `~/Downloads`).
QString downloadsDirectory();
