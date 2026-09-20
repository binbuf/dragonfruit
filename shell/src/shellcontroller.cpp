// SPDX-License-Identifier: MIT
#include "shellcontroller.h"

#include <QDebug>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSocketNotifier>
#include <QVariantList>
#include <QVariantMap>
#include <QTimer>
#include <QtMath>

#include <QCoreApplication>
#include <QKeyEvent>
#include <QMouseEvent>

#include <cstdio>

#include "shellprotocol.h"

namespace {

QVariantMap statusItem(const QString &id, const QString &icon, const QString &label,
                       const QString &accessibleName, bool available, qreal level = 0.8)
{
    QVariantMap item;
    item.insert(QStringLiteral("id"), id);
    item.insert(QStringLiteral("icon"), icon);
    item.insert(QStringLiteral("label"), label);
    item.insert(QStringLiteral("accessibleName"), accessibleName);
    item.insert(QStringLiteral("available"), available);
    item.insert(QStringLiteral("level"), level);
    return item;
}

// Minimal evdev -> Qt key map for menu interaction (the popup and bar only
// consume navigation keys). wl_keyboard.key is the raw evdev code.
Qt::Key qtKeyFromEvdev(quint32 key)
{
    switch (key) {
    case 1:
        return Qt::Key_Escape;
    case 28:
        return Qt::Key_Return;
    case 57:
        return Qt::Key_Space;
    case 102:
        return Qt::Key_Home;
    case 103:
        return Qt::Key_Up;
    case 105:
        return Qt::Key_Left;
    case 106:
        return Qt::Key_Right;
    case 107:
        return Qt::Key_End;
    case 108:
        return Qt::Key_Down;
    default:
        return Qt::Key_unknown;
    }
}

QVariantMap menuEntry(const QString &label, const QString &shortcut = QString())
{
    QVariantMap entry;
    entry.insert(QStringLiteral("label"), label);
    if (!shortcut.isEmpty())
        entry.insert(QStringLiteral("shortcut"), shortcut);
    return entry;
}

// T-22 stand-in: a small app menu so the dropdown can be exercised before the
// menu-broker lands. Only used with `--placeholders`.
QVariantList demoAppMenu()
{
    QVariantList menu;

    QVariantMap file;
    file.insert(QStringLiteral("title"), QStringLiteral("File"));
    file.insert(QStringLiteral("items"),
                QVariantList{menuEntry(QStringLiteral("New Window"), QStringLiteral("Ctrl+N")),
                             menuEntry(QStringLiteral("Open..."), QStringLiteral("Ctrl+O")),
                             menuEntry(QStringLiteral("Close"), QStringLiteral("Ctrl+W"))});
    menu << file;

    QVariantMap edit;
    edit.insert(QStringLiteral("title"), QStringLiteral("Edit"));
    edit.insert(QStringLiteral("items"),
                QVariantList{menuEntry(QStringLiteral("Undo"), QStringLiteral("Ctrl+Z")),
                             menuEntry(QStringLiteral("Redo"), QStringLiteral("Ctrl+Shift+Z"))});
    menu << edit;

    QVariantMap view;
    view.insert(QStringLiteral("title"), QStringLiteral("View"));
    view.insert(QStringLiteral("items"),
                QVariantList{menuEntry(QStringLiteral("Zoom")),
                             menuEntry(QStringLiteral("Enter Full Screen"), QStringLiteral("Ctrl+Ctrl+F"))});
    menu << view;

    return menu;
}

} // namespace

ShellController::ShellController(QObject *parent)
    : QObject(parent)
{
}

ShellController::~ShellController()
{
    delete m_window;
}

QString ShellController::lastError() const
{
    return m_protocol ? m_protocol->lastError() : QStringLiteral("shell not started");
}

bool ShellController::start(const QString &socketName, const QString &tokenHex, int barHeight,
                            bool placeholders)
{
    m_placeholders = placeholders;
    m_barHeight = barHeight;
    m_protocol = new ShellProtocol(this);
    connect(m_protocol, &ShellProtocol::fatal, this, [](const QString &message) {
        qWarning() << "shell:" << message;
    });

    if (!m_protocol->connectToCompositor(socketName))
        return false;
    if (!m_protocol->authenticate(tokenHex))
        return false;

    m_engine = new QQmlEngine(this);
#ifdef DF_QML_IMPORT_DIR
    // Static QML modules are not auto-registered for a plain executable;
    // import them from the build tree like the gallery does.
    m_engine->addImportPath(QStringLiteral(DF_QML_IMPORT_DIR));
#endif
    m_window = new QQuickWindow;
    m_window->setColor(Qt::transparent);

    QQmlComponent component(m_engine);
    component.loadFromModule(QStringLiteral("Dragonfruit.MenuBar"), QStringLiteral("MenuBar"));
    if (component.isError()) {
        fprintf(stderr, "dragonfruit-shell: QML error: %s\n",
                qPrintable(component.errorString()));
        return false;
    }
    QObject *object = component.create();
    m_item = qobject_cast<QQuickItem *>(object);
    if (!m_item) {
        fprintf(stderr, "dragonfruit-shell: MenuBar QML did not produce an item\n");
        return false;
    }
    m_item->setParentItem(m_window->contentItem());
    m_item->setProperty("showDate", true);

    connect(m_item, SIGNAL(controlCenterRequested()), this, SLOT(onControlCenterRequested()));
    connect(m_item, SIGNAL(missionControlRequested()), this, SLOT(onMissionControlRequested()));
    connect(m_item, SIGNAL(statusItemActivated(QString)), this,
            SLOT(onStatusItemActivated(QString)));
    connect(m_item, SIGNAL(appMenuOpened(int)), this, SLOT(onAppMenuOpened(int)));
    connect(m_item, SIGNAL(appMenuClosed()), this, SLOT(onAppMenuClosed()));
    if (QObject *clock = m_item->findChild<QObject *>(QStringLiteral("clock")))
        connect(clock, SIGNAL(nowChanged()), this, SLOT(onClockTick()));

    connect(m_protocol, &ShellProtocol::configured, this, &ShellController::onConfigured);
    connect(m_protocol, &ShellProtocol::focusedAppChanged, this,
            &ShellController::onFocusedAppChanged);
    connect(m_protocol, &ShellProtocol::pointerMoved, this, &ShellController::onPointerMoved);
    connect(m_protocol, &ShellProtocol::pointerButton, this, &ShellController::onPointerButton);
    connect(m_protocol, &ShellProtocol::pointerLeft, this, &ShellController::onPointerLeft);
    connect(m_protocol, &ShellProtocol::keyboardFocused, this,
            &ShellController::onKeyboardFocused);
    connect(m_protocol, &ShellProtocol::keyEvent, this, &ShellController::onKeyEvent);

    applyStatusItems();
    applyFocusedApp();

    m_surfaceHeight = barHeight;
    if (!m_protocol->createMenuBarSurface(barHeight, barHeight))
        return false;

    const int fd = m_protocol->displayFd();
    m_notifier = new QSocketNotifier(fd, QSocketNotifier::Read, this);
    connect(m_notifier, &QSocketNotifier::activated, this, [this]() { m_protocol->dispatch(); });
    return true;
}

void ShellController::applyStatusItems()
{
    QVariantList items;
    if (m_placeholders) {
        // Demo placeholders until the T-20 adapters land (ticket risk note).
        items << statusItem(QStringLiteral("wifi"), QStringLiteral("wifi"), QString(),
                            tr("Wi-Fi"), true);
        items << statusItem(QStringLiteral("bluetooth"), QStringLiteral("bluetooth"), QString(),
                            tr("Bluetooth"), true);
        items << statusItem(QStringLiteral("volume"), QStringLiteral("volume"), QString(),
                            tr("Volume"), true);
        items << statusItem(QStringLiteral("battery"), QStringLiteral("battery"), QString(),
                            tr("Battery"), true, 0.8);
        items << statusItem(QStringLiteral("focus"), QStringLiteral("focus"), QString(),
                            tr("Focus"), false);
        items << statusItem(QStringLiteral("accessibility"), QStringLiteral("accessibility"),
                            QString(), tr("Accessibility"), false);
    } else {
        // No adapters attached: every item degrades to hidden (FR-4).
        items << statusItem(QStringLiteral("wifi"), QStringLiteral("wifi"), QString(),
                            tr("Wi-Fi"), false);
        items << statusItem(QStringLiteral("bluetooth"), QStringLiteral("bluetooth"), QString(),
                            tr("Bluetooth"), false);
        items << statusItem(QStringLiteral("volume"), QStringLiteral("volume"), QString(),
                            tr("Volume"), false);
        items << statusItem(QStringLiteral("battery"), QStringLiteral("battery"), QString(),
                            tr("Battery"), false);
        items << statusItem(QStringLiteral("focus"), QStringLiteral("focus"), QString(),
                            tr("Focus"), false);
        items << statusItem(QStringLiteral("accessibility"), QStringLiteral("accessibility"),
                            QString(), tr("Accessibility"), false);
    }
    m_item->setProperty("statusItems", items);
}

void ShellController::applyFocusedApp()
{
    QString name = m_appId;
    if (name.isEmpty())
        name = m_appTitle;
    if (name.isEmpty())
        name = tr("Desktop");
    m_item->setProperty("appName", name);
    // The menu-broker (T-22) will resolve a real menu model here; until it
    // lands the bar renders the application name only (FR-2 priority 3). In
    // placeholder mode a small demo menu stands in so the dropdown (input
    // routing + grown bar surface) is exercisable in a live session.
    m_item->setProperty("appMenuModel", m_placeholders ? demoAppMenu() : QVariantList());
}

void ShellController::onConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's anchor/size; skip it. A real
    // configure is the bar height, or a taller one while a dropdown is open.
    if (height < m_barHeight || (!m_menuOpen && height != m_barHeight)) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout configure %dx%d\n", width,
                height);
        return;
    }
    m_width = width;
    m_height = height;
    m_surfaceHeight = height;
    fprintf(stderr, "dragonfruit-shell: menu bar configured %dx%d\n", width, height);
    render();
    // Canvas items paint on the scene graph after the first grab; re-render
    // once the event loop has run so the committed frame includes them.
    QTimer::singleShot(200, this, &ShellController::render);
}

void ShellController::onFocusedAppChanged(const QString &appId, const QString &title)
{
    m_appId = appId;
    m_appTitle = title;
    applyFocusedApp();
    render();
}

void ShellController::onControlCenterRequested()
{
    // The Control Center panel is T-21; the entry point is wired.
    qInfo() << "shell: Control Center requested (T-21)";
}

void ShellController::onMissionControlRequested()
{
    m_protocol->enterMissionControl();
}

void ShellController::onStatusItemActivated(const QString &itemId)
{
    // Status-item popups consume the T-20 adapters; until then this is a
    // no-op beyond the bar's own dismissal.
    qInfo() << "shell: status item activated:" << itemId;
}

void ShellController::onClockTick()
{
    render();
}

void ShellController::onAppMenuOpened(int)
{
    m_menuOpen = true;
    updateSurfaceHeight();
}

void ShellController::onAppMenuClosed()
{
    m_menuOpen = false;
    updateSurfaceHeight();
}

void ShellController::updateSurfaceHeight()
{
    // The dropdown is a child of the bar item that overflows the 28 px bar;
    // grow the chrome surface (and window) so the compositor reveals it. The
    // reserved zone is unchanged (the bar still reserves 28 px).
    int needed = m_barHeight;
    if (m_item) {
        const qreal bottom = m_item->property("dropdownBottom").toReal();
        if (bottom > 0)
            needed = qMax(needed, qCeil(bottom) + 2);
    }
    if (needed != m_surfaceHeight) {
        m_surfaceHeight = needed;
        m_protocol->setMenuBarSize(0, needed);
    }
}

void ShellController::onPointerMoved(qreal x, qreal y)
{
    if (!m_window)
        return;
    QMouseEvent event(QEvent::MouseMove, QPointF(x, y), QPointF(x, y), Qt::NoButton, m_buttons,
                      Qt::NoModifier);
    QCoreApplication::sendEvent(m_window, &event);
}

void ShellController::onPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_window)
        return;
    const Qt::MouseButton qtButton = button == 0x110 ? Qt::LeftButton : Qt::NoButton;
    if (pressed)
        m_buttons |= qtButton;
    else
        m_buttons &= ~qtButton;
    QMouseEvent event(pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, QPointF(x, y),
                      QPointF(x, y), qtButton, m_buttons, Qt::NoModifier);
    QCoreApplication::sendEvent(m_window, &event);
}

void ShellController::onPointerLeft()
{
    if (!m_window)
        return;
    // Move the pointer off-screen so hover states clear when the compositor
    // takes the pointer away (e.g. it moved onto a window).
    QMouseEvent event(QEvent::MouseMove, QPointF(-1, -1), QPointF(-1, -1), Qt::NoButton, m_buttons,
                      Qt::NoModifier);
    QCoreApplication::sendEvent(m_window, &event);
}

void ShellController::onKeyboardFocused(bool focused)
{
    if (m_item)
        m_item->setProperty("shellFocused", focused);
}

void ShellController::onKeyEvent(quint32 key, bool pressed)
{
    if (!m_window)
        return;
    const Qt::Key qtKey = qtKeyFromEvdev(key);
    if (qtKey == Qt::Key_unknown)
        return;
    QKeyEvent event(pressed ? QEvent::KeyPress : QEvent::KeyRelease, qtKey, Qt::NoModifier);
    QCoreApplication::sendEvent(m_window, &event);
}

void ShellController::render()
{
    if (!m_item || m_width <= 0 || m_surfaceHeight <= 0)
        return;
    // The bar item itself stays bar-height; an open dropdown overflows it and
    // is revealed by the taller window/surface.
    m_item->setWidth(m_width);
    m_item->setHeight(m_barHeight);
    if (m_window->width() != m_width || m_window->height() != m_surfaceHeight)
        m_window->resize(m_width, m_surfaceHeight);
    if (!m_window->isVisible())
        m_window->show();
    const QImage image = m_window->grabWindow();
    if (!m_protocol->commitImage(image))
        qWarning() << "shell: failed to commit the menu bar:" << m_protocol->lastError();
}
