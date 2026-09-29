// SPDX-License-Identifier: MIT
#include "FilesDesktop.h"

#include <cstdio>

#include <QCoreApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSocketNotifier>
#include <QTimer>

#include "DesktopInput.h"
#include "DesktopProtocol.h"
#include "chromepointer.h"

FilesDesktop::FilesDesktop(QQmlEngine *engine, QObject *parent)
    : QObject(parent)
    , m_engine(engine)
{
}

FilesDesktop::~FilesDesktop()
{
    delete m_protocol;
}

QString FilesDesktop::lastError() const
{
    return m_protocol ? m_protocol->lastError() : QString();
}

bool FilesDesktop::start(const QString &socketName, const QString &tokenHex,
                         const QString &desktopUri)
{
    m_protocol = new DesktopProtocol(this);
    if (!m_protocol->connectToCompositor(socketName, tokenHex))
        return false;

    m_window = new QQuickWindow;
    m_window->setColor(Qt::transparent);

    QQmlComponent component(m_engine);
    component.loadFromModule(QStringLiteral("Dragonfruit.Files"),
                             QStringLiteral("DesktopSurface"));
    if (component.isError()) {
        fprintf(stderr, "dragonfruit-files-desktop: QML error: %s\n",
                qPrintable(component.errorString()));
        return false;
    }
    QObject *object = component.create();
    m_item = qobject_cast<QQuickItem *>(object);
    if (!m_item) {
        fprintf(stderr, "dragonfruit-files-desktop: DesktopSurface QML did not produce an item\n");
        return false;
    }
    m_item->setParentItem(m_window->contentItem());
    m_item->setProperty("location", desktopUri);
    connect(object, SIGNAL(openRequested(QString, bool)), this,
            SIGNAL(openRequested(QString, bool)));

    connect(m_window, &QQuickWindow::afterRendering, this, &FilesDesktop::render);
    connect(m_protocol, &DesktopProtocol::configured, this, &FilesDesktop::onConfigured);
    connect(m_protocol, &DesktopProtocol::pointerMoved, this, &FilesDesktop::onPointerMoved);
    connect(m_protocol, &DesktopProtocol::pointerButton, this, &FilesDesktop::onPointerButton);
    connect(m_protocol, &DesktopProtocol::pointerLeft, this, &FilesDesktop::onPointerLeft);
    connect(m_protocol, &DesktopProtocol::keyboardFocused, this,
            &FilesDesktop::onKeyboardFocused);
    connect(m_protocol, &DesktopProtocol::keyEvent, this, &FilesDesktop::onKeyEvent);
    connect(m_protocol, &DesktopProtocol::surfaceClosed, this, &FilesDesktop::onSurfaceClosed);
    connect(m_protocol, &DesktopProtocol::fatal, this,
            [](const QString &message) { fprintf(stderr, "dragonfruit-files-desktop: %s\n",
                                                  qPrintable(message)); });

    if (!m_protocol->createDesktopSurface())
        return false;

    const int fd = m_protocol->displayFd();
    if (fd >= 0) {
        m_notifier = new QSocketNotifier(fd, QSocketNotifier::Read, this);
        connect(m_notifier, &QSocketNotifier::activated, this,
                [this]() { m_protocol->dispatch(); });
    }
    return true;
}

void FilesDesktop::onConfigured(int width, int height)
{
    if (width <= 0 || height <= 0)
        return;
    m_width = width;
    m_height = height;
    fprintf(stderr, "dragonfruit-files-desktop: configured %dx%d\n", width, height);
    scheduleRender();
}

void FilesDesktop::onPointerMoved(qreal x, qreal y)
{
    if (!m_window)
        return;
    ChromePointer::send(m_window, QEvent::MouseMove, QPointF(x, y), Qt::NoButton, m_buttons,
                        m_keyboardModifiers);
    scheduleRender();
}

void FilesDesktop::onPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_window)
        return;
    const Qt::MouseButton qtButton = desktopMouseButton(button);
    if (qtButton == Qt::NoButton)
        return;
    if (pressed)
        m_buttons |= qtButton;
    else
        m_buttons &= ~qtButton;
    ChromePointer::send(m_window,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease,
                        QPointF(x, y), qtButton, m_buttons, m_keyboardModifiers);
    scheduleRender();
}

void FilesDesktop::onPointerLeft()
{
    if (!m_window)
        return;
    // Park the pointer outside the scene so any hover clears; the button state
    // is preserved for a drag that briefly leaves the surface.
    ChromePointer::send(m_window, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton, m_buttons,
                        m_keyboardModifiers);
    scheduleRender();
}

void FilesDesktop::onKeyboardFocused(bool focused)
{
    if (focused && m_item)
        m_item->forceActiveFocus();
}

void FilesDesktop::onKeyEvent(quint32 key, bool pressed, quint32 modifiers)
{
    if (!m_window)
        return;
    m_keyboardModifiers = desktopModifiersFromXkb(modifiers);
    const Qt::Key qtKey = desktopKeyFromEvdev(key);
    if (qtKey == Qt::Key_unknown)
        return;
    QKeyEvent event(pressed ? QEvent::KeyPress : QEvent::KeyRelease, qtKey, m_keyboardModifiers);
    QCoreApplication::sendEvent(m_window, &event);
    scheduleRender();
}

void FilesDesktop::onSurfaceClosed()
{
    fprintf(stderr, "dragonfruit-files-desktop: surface closed by the compositor\n");
    QCoreApplication::quit();
}

void FilesDesktop::scheduleRender()
{
    if (m_renderPending)
        return;
    m_renderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_renderPending = false;
        render();
    });
}

void FilesDesktop::render()
{
    if (!m_window || !m_item || !m_protocol)
        return;
    // `grabWindow` renders the scene a second time, which emits `afterRendering`
    // again; skip that re-entrant frame (the shell's FrameCommitGate pattern).
    if (m_committing)
        return;
    if (m_width <= 0 || m_height <= 0)
        return;
    m_item->setWidth(m_width);
    m_item->setHeight(m_height);
    if (m_window->width() != m_width || m_window->height() != m_height)
        m_window->resize(m_width, m_height);
    if (!m_window->isVisible())
        m_window->show();
    m_committing = true;
    const QImage image = m_window->grabWindow();
    m_committing = false;
    if (!image.isNull()) {
        m_lastCommitted = image.size();
        m_protocol->commitImage(image);
    }
}