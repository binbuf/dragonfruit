// SPDX-License-Identifier: MIT
// Screenshot save/copy (T-13.3b).
//
// The captured image arrives from the compositor's portal-presenter capture
// path (`ShellProtocol::captureScreenshot`). This class is the shell half of
// "save and copy": it writes the PNG into the Pictures/Screenshots folder and
// puts the image on the clipboard. It owns no Wayland, no D-Bus, and no
// policy about *how* the pixels were produced, so the headless tests drive it
// directly and the capture handler just calls `deliver`.
//
// The save directory honours `DF_SCREENSHOT_DIR` (used by the tests and by a
// capture fixture); otherwise it is the XDG Pictures dir plus
// `Screenshots`.
#pragma once

#include <QDateTime>
#include <QImage>
#include <QObject>
#include <QString>

class ScreenshotWriter : public QObject
{
    Q_OBJECT

public:
    // The saved file and whether it reached disk and the clipboard.
    struct Result {
        QString path;
        bool saved = false;
        bool copied = false;
    };

    explicit ScreenshotWriter(QObject *parent = nullptr);

    // The directory captures are written to. `DF_SCREENSHOT_DIR` when set,
    // else `<XDG Pictures>/Screenshots`; empty when neither is available.
    static QString saveDirectory();

    // A filesystem-safe timestamped file name (no extension).
    static QString baseName(const QDateTime &now);

    // The full save path for `now`; a `-N` suffix keeps it unique.
    static QString savePath(const QDateTime &now = QDateTime::currentDateTime());

    // Write `image` as PNG to `path`, creating the parent directory. Returns
    // whether the file was written.
    static bool save(const QImage &image, const QString &path);

    // Put `image` (and the `file://` URI of `filePath` when non-empty) on the
    // clipboard. Returns whether a clipboard was available.
    static bool copy(const QImage &image, const QString &filePath = QString());

    // Save and then copy `image`: the one "capture done" step.
    Result deliver(const QImage &image) const;
};