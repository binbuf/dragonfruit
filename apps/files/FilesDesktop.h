// SPDX-License-Identifier: MIT
// The Files-owned desktop process (T-19.3).
//
// `dragonfruit-files --desktop` is a separate process from the browser: it
// connects to the compositor over the private protocol, creates the one
// full-output `background` layer surface, renders `~/Desktop` offscreen, and
// commits the frame. It shares `files-core` and the QML views with the browser
// but not its crash domain — killing either leaves the other running.
//
// This class is the glue: it owns the offscreen QQuickWindow, loads the
// `Dragonfruit.Files` `DesktopSurface`, translates the compositor's pointer and
// keyboard stream into Qt events (`ChromePointer`, so the T-16.12 timestamp
// invariant holds), and commits each rendered frame through `DesktopProtocol`.
#pragma once

#include <QObject>
#include <QPointF>
#include <QSize>
#include <QString>

class QQmlEngine;
class QQuickWindow;
class QQuickItem;
class QSocketNotifier;
class DesktopProtocol;

class FilesDesktop : public QObject
{
    Q_OBJECT

public:
    explicit FilesDesktop(QQmlEngine *engine, QObject *parent = nullptr);
    ~FilesDesktop() override;

    // Connect, authenticate, create the desktop surface, and load the QML.
    // `desktopUri` is the `file://` directory the desktop lists (already
    // resolved through xdg-user-dirs by `FilesArguments`).
    bool start(const QString &socketName, const QString &tokenHex, const QString &desktopUri);

    QString lastError() const;

signals:
    // The desktop emitted an open: a directory opens a Files window at that
    // path, a file its default handler. Wired in `main.cpp`, where the process
    // launch seam lives.
    void openRequested(const QString &uri, bool isDir);

private slots:
    void onConfigured(int width, int height);
    void onPointerMoved(qreal x, qreal y);
    void onPointerButton(qreal x, qreal y, quint32 button, bool pressed);
    void onPointerLeft();
    void onKeyboardFocused(bool focused);
    void onKeyEvent(quint32 key, bool pressed, quint32 modifiers);
    void onSurfaceClosed();
    void scheduleRender();
    void render();

private:
    QQmlEngine *m_engine = nullptr;
    DesktopProtocol *m_protocol = nullptr;
    QQuickWindow *m_window = nullptr;
    QQuickItem *m_item = nullptr;
    QSocketNotifier *m_notifier = nullptr;

    int m_width = 0;
    int m_height = 0;
    bool m_renderPending = false;
    bool m_committing = false;
    Qt::MouseButtons m_buttons = Qt::NoButton;
    Qt::KeyboardModifiers m_keyboardModifiers = Qt::NoModifier;
    QSize m_lastCommitted;
};