// SPDX-License-Identifier: GPL-3.0-or-later
#include "shellcontroller.h"

#include <QDebug>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSocketNotifier>
#include <QVariantList>
#include <QVariantMap>

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

    connect(m_item, SIGNAL(controlCenterRequested()), this, SLOT(onControlCenterRequested()));
    connect(m_item, SIGNAL(missionControlRequested()), this, SLOT(onMissionControlRequested()));
    connect(m_item, SIGNAL(statusItemActivated(QString)), this,
            SLOT(onStatusItemActivated(QString)));
    if (QObject *clock = m_item->findChild<QObject *>(QStringLiteral("clock")))
        connect(clock, SIGNAL(nowChanged()), this, SLOT(onClockTick()));

    connect(m_protocol, &ShellProtocol::configured, this, &ShellController::onConfigured);
    connect(m_protocol, &ShellProtocol::focusedAppChanged, this,
            &ShellController::onFocusedAppChanged);

    applyStatusItems();
    applyFocusedApp();

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
    // lands the bar renders the application name only (FR-2 priority 3).
    m_item->setProperty("appMenuModel", QVariantList());
}

void ShellController::onConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's anchor/size; skip it so we do not
    // render a full-output buffer, and render the real bar configure.
    if (height != m_barHeight) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout configure %dx%d\n", width,
                height);
        return;
    }
    m_width = width;
    m_height = height;
    fprintf(stderr, "dragonfruit-shell: menu bar configured %dx%d\n", width, height);
    render();
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

void ShellController::render()
{
    if (!m_item || m_width <= 0 || m_height <= 0)
        return;
    m_item->setWidth(m_width);
    m_item->setHeight(m_height);
    if (m_window->width() != m_width || m_window->height() != m_height)
        m_window->resize(m_width, m_height);
    if (!m_window->isVisible())
        m_window->show();
    const QImage image = m_window->grabWindow();
    if (!m_protocol->commitImage(image))
        qWarning() << "shell: failed to commit the menu bar:" << m_protocol->lastError();
}
