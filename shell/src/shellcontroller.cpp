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
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFileSystemWatcher>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QProcess>
#include <QStandardPaths>

#include <cstdio>

#include "desktopentry.h"
#include "dockmodel.h"
#include "dockpins.h"
#include "shellprotocol.h"

namespace {

// How long a launch may take to map its first window before the Dock treats
// it as failed (T-10 section 8.5). Interim until app-index owns activation.
constexpr qint64 kLaunchTimeoutMs = 8000;

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

QVariantMap menuEntry(const QString &label, const QString &shortcut = QString(),
                      const QString &action = QString(), bool enabled = true)
{
    QVariantMap entry;
    entry.insert(QStringLiteral("label"), label);
    if (!shortcut.isEmpty())
        entry.insert(QStringLiteral("shortcut"), shortcut);
    if (!action.isEmpty())
        entry.insert(QStringLiteral("action"), action);
    if (!enabled)
        entry.insert(QStringLiteral("enabled"), false);
    return entry;
}

QVariantMap menuSeparator()
{
    QVariantMap entry;
    entry.insert(QStringLiteral("type"), QStringLiteral("separator"));
    return entry;
}

// The fixed system menu (the dragonfruit mark), always leftmost. The actions
// are session/system operations: Settings (T-16), App Store (no equivalent
// yet), Sleep/Restart/Shut Down/Log Out (logind, T-24), Lock Screen (T-26).
// Until those land they are dispatched as logged stubs (see T-09 hand-off).
QVariantList systemMenu(const QString &userName)
{
    QVariantList menu;
    menu << menuEntry(QStringLiteral("About This System"), QString(), QStringLiteral("about-system"));
    menu << menuSeparator();
    menu << menuEntry(QStringLiteral("System Settings\u2026"), QString(), QStringLiteral("settings"));
    menu << menuEntry(QStringLiteral("App Store"), QString(), QStringLiteral("app-store"), false);
    menu << menuSeparator();
    menu << menuEntry(QStringLiteral("Sleep"), QString(), QStringLiteral("sleep"));
    menu << menuEntry(QStringLiteral("Restart\u2026"), QString(), QStringLiteral("restart"));
    menu << menuEntry(QStringLiteral("Shut Down\u2026"), QString(), QStringLiteral("shut-down"));
    menu << menuSeparator();
    menu << menuEntry(QStringLiteral("Lock Screen"), QStringLiteral("Super+Ctrl+Q"),
                      QStringLiteral("lock-screen"));
    menu << menuEntry(QStringLiteral("Log Out %1\u2026").arg(userName),
                      QStringLiteral("Super+Shift+Q"), QStringLiteral("log-out"));
    return menu;
}

// The fixed application menu, always right of the system menu and always
// present (the focused app's name, or Files on the empty desktop). On macOS
// the application itself owns these items; here the shell synthesizes the
// standard set until the menu-broker (T-22) resolves them per app. The app's
// exported top-level menus (File/Edit/View/…) follow this menu.
QVariantList applicationMenu(const QString &appName)
{
    QVariantList menu;
    menu << menuEntry(QStringLiteral("About %1").arg(appName), QString(),
                      QStringLiteral("about"));
    menu << menuEntry(QStringLiteral("Settings\u2026"), QStringLiteral("Super+,"),
                      QStringLiteral("settings"));
    menu << menuSeparator();
    menu << menuEntry(QStringLiteral("Hide %1").arg(appName), QStringLiteral("Super+H"),
                      QStringLiteral("hide"));
    menu << menuEntry(QStringLiteral("Hide Others"), QStringLiteral("Super+Alt+H"),
                      QStringLiteral("hide-others"));
    menu << menuEntry(QStringLiteral("Show All"), QString(), QStringLiteral("show-all"));
    menu << menuSeparator();
    menu << menuEntry(QStringLiteral("Quit %1").arg(appName), QStringLiteral("Super+Q"),
                      QStringLiteral("quit"));
    return menu;
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
    delete m_dockWindow;
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

    // Interim app-index (T-23) and pinned-set persistence (T-15). The index
    // is scanned once at startup; a real app-index will push install/uninstall
    // events instead. Defaults are seeded only when no settings file exists,
    // so an intentionally emptied pin set is respected.
    m_index.scan();
    if (!m_pins.load())
        qWarning() << "shell: Dock pins:" << m_pins.lastError();
    if (!m_pins.fileExists()) {
        m_pins.setIds(DockPins::resolveDefaultPins(m_index));
        if (!m_pins.save())
            qWarning() << "shell: cannot seed Dock pins:" << m_pins.lastError();
    }
    // Interim `dock.*` settings model (T-10 section 19). The file watch is
    // the stand-in for settingsd's change signals (T-15); when T-15 lands it
    // replaces `onSettingsFileChanged` with a D-Bus subscription.
    if (!m_settings.load())
        qWarning() << "shell: Dock settings:" << m_settings.lastError();
    m_settingsWatcher = new QFileSystemWatcher(this);
    const QString settingsPath = DockSettings::defaultFilePath();
    if (QFile::exists(settingsPath))
        m_settingsWatcher->addPath(settingsPath);
    connect(m_settingsWatcher, &QFileSystemWatcher::fileChanged, this,
            &ShellController::onSettingsFileChanged);

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
    connect(m_item, SIGNAL(appMenuTriggered(int,int,QVariant)), this,
            SLOT(onAppMenuTriggered(int,int,QVariant)));
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
    connect(m_protocol, &ShellProtocol::dockConfigured, this,
            &ShellController::onDockConfigured);
    connect(m_protocol, &ShellProtocol::dockStateChanged, this,
            &ShellController::onDockStateChanged);
    connect(m_protocol, &ShellProtocol::dockPointerMoved, this,
            &ShellController::onDockPointerMoved);
    connect(m_protocol, &ShellProtocol::dockPointerButton, this,
            &ShellController::onDockPointerButton);
    connect(m_protocol, &ShellProtocol::dockPointerLeft, this,
            &ShellController::onDockPointerLeft);
    connect(m_protocol, &ShellProtocol::dockPopupPointerMoved, this,
            &ShellController::onDockPopupPointerMoved);
    connect(m_protocol, &ShellProtocol::dockPopupPointerButton, this,
            &ShellController::onDockPopupPointerButton);
    connect(m_protocol, &ShellProtocol::dockPopupPointerLeft, this,
            &ShellController::onDockPopupPointerLeft);
    connect(m_protocol, &ShellProtocol::attentionRequested, this,
            &ShellController::onDockAttention);

    applyStatusItems();
    // The system menu (dragonfruit mark) is fixed for the session; the Log Out
    // item is personalized with the account name.
    QString userName = qEnvironmentVariable("USER");
    if (userName.isEmpty())
        userName = QStringLiteral("user");
    m_item->setProperty("systemMenuItems", systemMenu(userName));
    applyFocusedApp();

    // The Dock is a second offscreen QML scene and its own `top` chrome
    // surface (T-10). It is created before the surfaces so the shell can read
    // the bar thickness and magnified band out of the QML tokens.
    m_dockWindow = new QQuickWindow;
    m_dockWindow->setColor(Qt::transparent);
    QQmlComponent dockComponent(m_engine);
    dockComponent.loadFromModule(QStringLiteral("Dragonfruit.Dock"), QStringLiteral("Dock"));
    if (dockComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: Dock QML error: %s\n",
                qPrintable(dockComponent.errorString()));
        return false;
    }
    QObject *dockObject = dockComponent.create();
    m_dockItem = qobject_cast<QQuickItem *>(dockObject);
    if (!m_dockItem) {
        fprintf(stderr, "dragonfruit-shell: Dock QML did not produce an item\n");
        return false;
    }
    m_dockItem->setParentItem(m_dockWindow->contentItem());
    connect(m_dockItem, SIGNAL(entryActivated(QVariant)), this,
            SLOT(onDockEntryActivated(QVariant)));
    connect(m_dockItem, SIGNAL(entryContextMenuRequested(QVariant,qreal,qreal)), this,
            SLOT(onDockEntryContextMenu(QVariant,qreal,qreal)));
    connect(m_dockItem, SIGNAL(dividerContextMenuRequested(qreal,qreal)), this,
            SLOT(onDockDividerContextMenu(qreal,qreal)));
    connect(m_dockItem, SIGNAL(menuActionRequested(QString,QVariant)), this,
            SLOT(onDockEntryMenuAction(QString,QVariant)));
    connect(m_dockItem, SIGNAL(windowActivated(QString)), this,
            SLOT(onDockWindowActivated(QString)));
    connect(m_dockItem, SIGNAL(pinnedOrderChanged(QVariant)), this,
            SLOT(onDockPinnedOrderChanged(QVariant)));
    connect(m_dockItem, SIGNAL(popoverChanged()), this, SLOT(onDockPopoverChanged()));
    connect(m_dockItem, SIGNAL(revealStateChanged()), this,
            SLOT(onDockRevealStateChanged()));
    // Apply `dock.*` before the surfaces exist (no reconfigure yet) so the
    // baseline bar thickness and magnified band reflect the saved size.
    applyDockSettings(false);
    m_dockBarThickness = qRound(m_dockItem->property("barThickness").toReal());
    m_dockThickness = m_dockBarThickness + qCeil(m_dockItem->property("magnifyBand").toReal());
    m_dockPosition = dockPosition();
    // For a bottom Dock the surface height is known up front; a vertical
    // Dock's height comes from the first configure (the output height).
    if (m_dockPosition == ShellProtocol::DockPosition::Bottom)
        m_dockHeight = m_dockThickness;
    else
        m_dockWidth = m_dockThickness;
    // The running projection may have arrived before the Dock scene existed.
    rebuildDockEntries();

    if (!m_protocol->createMenuBarSurface(barHeight, barHeight))
        return false;
    // The dropdown rides a separate `overlay` chrome surface so transient
    // menus composite above fullscreen windows while the bar keeps its own
    // `top` surface and reserved zone (T-09).
    if (!m_protocol->createPopupSurface())
        return false;
    // The Dock reserves its baseline bar thickness; with auto-hide on it
    // reserves nothing (section 2).
    const int dockExclusive = m_dockItem->property("autoHide").toBool() ? 0 : m_dockBarThickness;
    if (!m_protocol->createDockSurface(m_dockPosition, m_dockThickness, dockExclusive))
        return false;
    // Dock context menus and the window chooser ride a second `overlay`
    // surface anchored to the Dock's edge (T-10 sections 9/13).
    if (!m_protocol->createDockPopupSurface(m_dockPosition))
        return false;

    // The shell snapshots the QML scene into a shm buffer on demand; a popup
    // open/close animation needs a short burst of frames to be legible.
    m_animationTimer = new QTimer(this);
    m_animationTimer->setInterval(16);
    connect(m_animationTimer, &QTimer::timeout, this, [this]() {
        render();
        if (--m_animationTicks <= 0)
            m_animationTimer->stop();
    });

    // Launch timeout: a launch that produces no window within the bounded
    // window returns to not-running and raises a one-shot notice (T-10
    // section 8.5). This is the interim stand-in for the app-index
    // activation/attention signal (FR-4).
    m_launchTimer = new QTimer(this);
    m_launchTimer->setInterval(500);
    connect(m_launchTimer, &QTimer::timeout, this, &ShellController::onDockLaunchTick);

    // Dock animation clock (T-10 section 8.1): 16 ms while a launch or
    // attention bounce is in flight, stopped otherwise so the idle Dock
    // contributes zero wakeups (FR-8). The durable scene-graph render path
    // (FR-14) replaces the on-demand grab later.
    m_dockAnimTimer = new QTimer(this);
    m_dockAnimTimer->setInterval(16);
    connect(m_dockAnimTimer, &QTimer::timeout, this, &ShellController::onDockAnimationTick);

    // Dock popover open/close animation burst (the durable scene-graph render
    // path is still deferred; FR-14).
    m_dockPopupTimer = new QTimer(this);
    m_dockPopupTimer->setInterval(16);
    connect(m_dockPopupTimer, &QTimer::timeout, this, [this]() {
        renderDock();
        if (--m_dockPopupTicks <= 0)
            m_dockPopupTimer->stop();
    });

    const int fd = m_protocol->displayFd();
    m_notifier = new QSocketNotifier(fd, QSocketNotifier::Read, this);
    connect(m_notifier, &QSocketNotifier::activated, this, [this]() { m_protocol->dispatch(); });

    // The offscreen window hands active focus to the first focusable menu
    // title when it is shown, which draws that title's FocusRing as if the
    // bar were keyboard-focused. Put focus on the bar root instead so the
    // launch state is neutral; a popup takes focus itself when it opens.
    // The offscreen window can report a default cursor/focus at (0,0) when it
    // is shown, which highlights the first title (hover or focus ring) in the
    // first committed frame. Settle it once the event loop is running and
    // again after the first delayed re-render, then commit the neutral frame.
    QTimer::singleShot(0, this, &ShellController::settleInitialState);
    QTimer::singleShot(400, this, &ShellController::settleInitialState);
    return true;
}

void ShellController::settleInitialState()
{
    if (m_menuOpen || !m_item || !m_window)
        return;
    // Keep active focus on the bar root, not the first menu title.
    m_item->forceActiveFocus();
    // Clear any hover the window inherited from its default cursor position:
    // move the synthesized pointer off the bar. Real motion re-establishes
    // hover as soon as the user interacts.
    QMouseEvent move(QEvent::MouseMove, QPointF(-1, -1), QPointF(-1, -1), Qt::NoButton,
                     Qt::NoButton, Qt::NoModifier);
    QCoreApplication::sendEvent(m_window, &move);
    render();
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
    // The application menu is always present. With no focused window the
    // desktop is Files (the macOS "Finder owns the desktop" model, see
    // design/09-files.md), so the default application menu is Files.
    QString name = m_appId;
    if (name.isEmpty())
        name = m_appTitle;
    if (name.isEmpty())
        name = QStringLiteral("Files");
    m_item->setProperty("appName", name);
    m_item->setProperty("applicationMenuItems", applicationMenu(name));
    // The menu-broker (T-22) resolves a real menu model here; until it lands
    // the bar renders the fixed application menu only (FR-2 priority 3). In
    // placeholder mode a small demo menu stands in so the dropdown (input
    // routing + overlay surface) is exercisable in a live session.
    m_item->setProperty("appMenuModel", m_placeholders ? demoAppMenu() : QVariantList());
}

void ShellController::onConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's anchor/size; skip it. The bar
    // surface is always exactly one bar tall (the dropdown is a separate
    // `overlay` surface).
    if (height != m_barHeight) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout configure %dx%d\n", width,
                height);
        return;
    }
    m_width = width;
    m_height = height;
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
    // FR-4: attention stops when the app's window gains focus.
    clearAttention(appId);
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
    updatePopupGeometry();
    // Capture the popup fade/scale-in (popupOpen = 160 ms).
    startAnimationRenders(220);
}

void ShellController::onAppMenuClosed()
{
    m_menuOpen = false;
    // Let the close animation (popupClose = 100 ms) play before unmapping the
    // overlay surface.
    startAnimationRenders(160);
    QTimer::singleShot(140, this, [this]() {
        if (!m_menuOpen)
            updatePopupGeometry();
    });
}

void ShellController::onAppMenuTriggered(int menuIndex, int itemIndex, const QVariant &item)
{
    const QVariantMap map = item.toMap();
    const QString action = map.value(QStringLiteral("action")).toString();

    if (action == QLatin1String("quit")) {
        // The application menu's Quit closes the focused app; on the desktop
        // (Files menu) there is no app to close yet.
        if (!m_appId.isEmpty())
            m_protocol->closeApp(m_appId);
    } else if (action == QLatin1String("settings")) {
        // T-16 owns the Settings app; the entry point is wired.
        qInfo() << "shell: Settings requested (T-16)";
    } else if (action == QLatin1String("about") || action == QLatin1String("about-system")) {
        // The About panel is a shell dialog; T-16 owns General > About data.
        qInfo() << "shell: About requested (T-16)";
    } else if (action == QLatin1String("hide") || action == QLatin1String("hide-others")
               || action == QLatin1String("show-all")) {
        // App visibility is a compositor window-state operation (T-04); the
        // macOS Hide/Hide Others/Show All semantics are T-24 work.
        qInfo() << "shell: app visibility action (T-04/T-24):" << action;
    } else if (action == QLatin1String("sleep") || action == QLatin1String("restart")
               || action == QLatin1String("shut-down") || action == QLatin1String("log-out")) {
        // logind power/session actions land with T-24.
        qInfo() << "shell: session action requested (T-24):" << action;
    } else if (action == QLatin1String("lock-screen")) {
        // The lock screen is T-26.
        qInfo() << "shell: lock screen requested (T-26)";
    } else if (action == QLatin1String("app-store")) {
        // No app-store equivalent exists yet; the item is disabled (T-09
        // open question in design/04-shell.md).
        qInfo() << "shell: App Store requested (no equivalent yet)";
    } else {
        // T-22 dispatches the resolved action; the placeholder logs so the
        // demo has feedback and the bar repaints after the popup closes.
        qInfo() << "shell: app menu item activated:" << menuIndex << itemIndex << map;
    }
    scheduleRender();
}

void ShellController::scheduleRender()
{
    if (m_renderPending)
        return;
    m_renderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_renderPending = false;
        render();
    });
}

void ShellController::startAnimationRenders(int ms)
{
    if (!m_animationTimer)
        return;
    m_animationTicks = qMax(1, ms / m_animationTimer->interval());
    if (!m_animationTimer->isActive())
        m_animationTimer->start();
}

void ShellController::updatePopupGeometry()
{
    if (!m_item) {
        return;
    }
    if (!m_menuOpen) {
        m_popupWidth = 0;
        m_popupHeight = 0;
        m_protocol->hidePopup();
        render();
        return;
    }
    // The open dropdown's rectangle in window coordinates. The popup QML
    // overflows the bar item; the shell reveals it through the overlay
    // surface placed at exactly this rectangle.
    const int x = qMax(0, qFloor(m_item->property("dropdownX").toReal()));
    const int y = qMax(0, qFloor(m_item->property("dropdownY").toReal()));
    const int width = qCeil(m_item->property("dropdownWidth").toReal());
    const int height = qCeil(m_item->property("dropdownHeight").toReal());
    if (width <= 0 || height <= 0) {
        m_popupWidth = 0;
        m_popupHeight = 0;
        m_protocol->hidePopup();
        render();
        return;
    }
    m_popupX = x;
    m_popupY = y;
    m_popupWidth = width;
    m_popupHeight = height;
    m_protocol->setPopupGeometry(x, y, width, height);
    render();
}

void ShellController::onPointerMoved(qreal x, qreal y)
{
    if (!m_window)
        return;
    QMouseEvent event(QEvent::MouseMove, QPointF(x, y), QPointF(x, y), Qt::NoButton, m_buttons,
                      Qt::NoModifier);
    QCoreApplication::sendEvent(m_window, &event);
    // Hover/title/row highlights changed in QML; push the new frame.
    scheduleRender();
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
    scheduleRender();
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
    scheduleRender();
}

void ShellController::onKeyboardFocused(bool focused)
{
    if (m_item)
        m_item->setProperty("shellFocused", focused);
    // The Dock surface holds keyboard focus while a context menu or window
    // chooser is open (OnDemand); a click-away or focus loss dismisses the
    // popover (T-10 section 13).
    if (!focused && m_dockItem)
        QMetaObject::invokeMethod(m_dockItem, "closePopovers");
}

void ShellController::onKeyEvent(quint32 key, bool pressed)
{
    if (!m_window)
        return;
    const Qt::Key qtKey = qtKeyFromEvdev(key);
    if (qtKey == Qt::Key_unknown)
        return;
    QKeyEvent event(pressed ? QEvent::KeyPress : QEvent::KeyRelease, qtKey, Qt::NoModifier);
    // An open Dock popover owns the keyboard (Escape/arrows); otherwise the
    // menu bar does (T-10 sections 13/20).
    if (m_dockWindow && m_dockItem
            && m_dockItem->property("popoverOpen").toBool()) {
        QCoreApplication::sendEvent(m_dockWindow, &event);
        scheduleDockRender();
    } else {
        QCoreApplication::sendEvent(m_window, &event);
        scheduleRender();
    }
}

// --- Dock (T-10) ------------------------------------------------------------

void ShellController::onDockConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the Dock's anchor/size; skip it. The Dock's extent
    // perpendicular to its edge is `m_dockThickness`: the height for a bottom
    // Dock, the width for a vertical one (T-10 section 5).
    const bool extentMatches = m_dockPosition == ShellProtocol::DockPosition::Bottom
            ? height == m_dockThickness
            : width == m_dockThickness;
    if (!extentMatches) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout Dock configure %dx%d\n", width,
                height);
        return;
    }
    m_dockWidth = width;
    m_dockHeight = height;
    fprintf(stderr, "dragonfruit-shell: Dock configured %dx%d\n", width, height);
    renderDock();
}

ShellProtocol::DockPosition ShellController::dockPosition() const
{
    const QString position = m_settings.position();
    if (position == QLatin1String("left"))
        return ShellProtocol::DockPosition::Left;
    if (position == QLatin1String("right"))
        return ShellProtocol::DockPosition::Right;
    return ShellProtocol::DockPosition::Bottom;
}

int ShellController::iconSizeForSize(double size) const
{
    if (!m_dockItem)
        return 48;
    const double low = m_dockItem->property("iconSizeMin").toReal();
    const double high = m_dockItem->property("iconSizeMax").toReal();
    return qRound(low + qBound(0.0, size, 1.0) * (high - low));
}

// Push the `dock.*` settings onto the Dock QML. `reconfigure` is false at
// startup (the chrome surface does not exist yet) and true for a live change
// that alters the surface extent or reserved zone.
void ShellController::applyDockSettings(bool reconfigure)
{
    if (!m_dockItem)
        return;
    m_dockItem->setProperty("iconSize", iconSizeForSize(m_settings.size()));
    m_dockItem->setProperty("magnification", m_settings.magnification());
    m_dockItem->setProperty("autoHide", m_settings.autohide());
    // Enabling auto-hide starts hidden; disabling it always reveals. The
    // QML owns the reveal/hide state machine (T-10 section 15).
    QMetaObject::invokeMethod(m_dockItem,
                              m_settings.autohide() ? "hide" : "reveal");
    m_dockItem->setProperty("showIndicators", m_settings.showIndicators());
    m_dockItem->setProperty("minimizeIntoTileIcon", m_settings.minimizeIntoTileIcon());
    m_dockItem->setProperty("animateOpening", m_settings.animateOpening());
    m_dockItem->setProperty("showRecentApps", m_settings.showRecentApps());
    // The position is applied to the surface anchor (T-10 section 5); the
    // QML layout mirrors for a vertical Dock.
    const ShellProtocol::DockPosition position = dockPosition();
    m_dockItem->setProperty("position",
                            position == ShellProtocol::DockPosition::Left ? QStringLiteral("left")
                            : position == ShellProtocol::DockPosition::Right
                                    ? QStringLiteral("right")
                                    : QStringLiteral("bottom"));
    // Reduced motion is a global animation policy: bind it onto the
    // design-system singleton so the whole shell reacts (T-10 section 20).
    if (m_engine) {
        if (QObject *theme = m_engine->singletonInstance<QObject *>(
                QStringLiteral("Dragonfruit"), QStringLiteral("Theme"))) {
            theme->setProperty("reducedMotion", m_settings.reduceMotion());
        }
    }
    if (!reconfigure || !m_protocol)
        return;
    // A live geometry change moves the bar, so any open popover is dismissed
    // rather than left floating detached (T-10 section 22).
    QMetaObject::invokeMethod(m_dockItem, "closePopovers");
    // The icon size changed the baseline bar and the magnified band; re-apply
    // the surface extent, anchor, and reserved zone (auto-hide reserves
    // nothing).
    m_dockBarThickness = qRound(m_dockItem->property("barThickness").toReal());
    m_dockThickness = m_dockBarThickness + qCeil(m_dockItem->property("magnifyBand").toReal());
    const bool positionChanged = position != m_dockPosition;
    m_dockPosition = position;
    if (positionChanged) {
        // The stretch dimension changes with the edge and is only known from
        // the next configure; pause rendering until it arrives so a stale
        // frame is never committed at the wrong aspect.
        if (position == ShellProtocol::DockPosition::Bottom) {
            m_dockHeight = m_dockThickness;
            m_dockWidth = 0;
        } else {
            m_dockWidth = m_dockThickness;
            m_dockHeight = 0;
        }
    }
    const int exclusive = m_settings.autohide() ? 0 : m_dockBarThickness;
    if (!m_protocol->configureDockSurface(position, m_dockThickness, exclusive))
        qWarning() << "shell: cannot reconfigure the Dock surface:"
                   << m_protocol->lastError();
    renderDock();
}

void ShellController::saveDockSettings()
{
    if (!m_settings.save())
        qWarning() << "shell: cannot save Dock settings:" << m_settings.lastError();
}

void ShellController::onSettingsFileChanged()
{
    // QSaveFile replaces the file, which drops the watch; re-add it. The
    // file may briefly not exist, so retry on the next event-loop turn.
    const QString path = DockSettings::defaultFilePath();
    QTimer::singleShot(0, this, [this, path]() {
        if (m_settingsWatcher && QFile::exists(path)
                && !m_settingsWatcher->files().contains(path))
            m_settingsWatcher->addPath(path);
    });

    DockSettings fresh;
    if (!fresh.load()) {
        qWarning() << "shell: Dock settings:" << fresh.lastError();
        return;
    }
    // Ignore the notification for our own write (settingsd will be the
    // single writer at T-15, so this guard disappears then).
    if (fresh.equals(m_settings))
        return;
    m_settings = fresh;
    applyDockSettings(true);
    fprintf(stderr, "dragonfruit-shell: Dock settings reloaded (live)\n");
}

void ShellController::onDockStateChanged(const QVariantList &entries)
{
    m_runningEntries = entries;
    // A window appeared: a pending launch for that app has succeeded, so its
    // transient launching/failed state clears (T-10 section 8.4).
    for (const QVariant &value : entries) {
        const QVariantMap map = value.toMap();
        if (map.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        const DesktopEntry resolved =
            m_index.resolve(map.value(QStringLiteral("appId")).toString());
        if (resolved.valid) {
            m_launchStates.remove(resolved.id);
            m_launchDeadlines.remove(resolved.id);
        }
    }
    if (m_launchDeadlines.isEmpty() && m_launchTimer && m_launchTimer->isActive())
        m_launchTimer->stop();
    rebuildDockEntries();
}

void ShellController::rebuildDockEntries()
{
    if (!m_dockItem)
        return;
    m_dockItem->setProperty("entries",
                            withBounce(buildDockEntries(m_pins.ids(), m_index, m_runningEntries,
                                                        m_launchStates)));
    scheduleDockRender();
}

QVariantList ShellController::withBounce(QVariantList entries) const
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    for (QVariant &value : entries) {
        QVariantMap map = value.toMap();
        double phase = -1.0;
        const QString appId = map.value(QStringLiteral("appId")).toString();
        const QString desktopId = map.value(QStringLiteral("desktopId")).toString();
        if (!appId.isEmpty() && now < m_attentionUntil.value(appId)) {
            // Attention bounce wins over a launch bounce for the same entry
            // and is taller/repeating (FR-4).
            map.insert(QStringLiteral("attention"), true);
            phase = dockAttentionBouncePhase(now - m_attentionStart.value(appId));
        } else if (!desktopId.isEmpty() && m_launchStart.contains(desktopId)) {
            phase = dockLaunchBouncePhase(now - m_launchStart.value(desktopId));
        }
        if (phase >= 0.0)
            map.insert(QStringLiteral("bounce"), phase);
        value = map;
    }
    return entries;
}

void ShellController::ensureDockAnimation()
{
    if (m_dockAnimTimer && !m_dockAnimTimer->isActive())
        m_dockAnimTimer->start();
}

void ShellController::clearAttention(const QString &appId)
{
    if (appId.isEmpty() || !m_attentionUntil.contains(appId))
        return;
    m_attentionUntil.remove(appId);
    m_attentionStart.remove(appId);
    rebuildDockEntries();
}

void ShellController::onDockAttention(const QString &appId)
{
    if (appId.isEmpty())
        return;
    // An attention request reveals a hidden auto-hide Dock immediately
    // (T-10 section 15).
    if (m_dockItem)
        QMetaObject::invokeMethod(m_dockItem, "reveal");
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    m_attentionStart.insert(appId, now);
    m_attentionUntil.insert(appId, now + kAttentionBounceMs);
    ensureDockAnimation();
    rebuildDockEntries();
}

void ShellController::onDockAnimationTick()
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    for (auto it = m_attentionUntil.begin(); it != m_attentionUntil.end();) {
        if (it.value() <= now) {
            m_attentionStart.remove(it.key());
            it = m_attentionUntil.erase(it);
        } else {
            ++it;
        }
    }
    for (auto it = m_launchStart.begin(); it != m_launchStart.end();) {
        if (now - it.value() >= kLaunchBounceMs)
            it = m_launchStart.erase(it);
        else
            ++it;
    }
    rebuildDockEntries();
    if (m_attentionUntil.isEmpty() && m_launchStart.isEmpty() && m_dockAnimTimer)
        m_dockAnimTimer->stop();
}

void ShellController::onDockEntryActivated(const QVariant &entry)
{
    const QVariantMap map = entry.toMap();
    const QString kind = map.value(QStringLiteral("kind")).toString();
    // FR-4: clicking the entry stops its attention bounce.
    clearAttention(map.value(QStringLiteral("appId")).toString());
    if (kind == QLatin1String("trash")) {
        // Opening Trash in Files is T-18; the entry point is wired.
        qInfo() << "shell: Dock Trash activated (T-18 opens Trash)";
        return;
    }
    if (kind == QLatin1String("divider"))
        return;

    // Minimized-window entries restore their owning app's most recent window.
    if (kind == QLatin1String("minimized")) {
        const QString appId = map.value(QStringLiteral("appId")).toString();
        if (!appId.isEmpty())
            m_protocol->activateApp(appId);
        return;
    }

    if (map.value(QStringLiteral("running")).toBool()) {
        const QString appId = map.value(QStringLiteral("appId")).toString();
        if (!appId.isEmpty())
            m_protocol->activateApp(appId);
        return;
    }

    if (map.value(QStringLiteral("missing")).toBool()) {
        qWarning() << "shell: Dock entry is not installed (T-23 app-index):"
                   << map.value(QStringLiteral("desktopId")).toString();
        return;
    }

    const QString desktopId = map.value(QStringLiteral("desktopId")).toString();
    if (desktopId.isEmpty())
        return;
    // Coalesce a second click while a launch is already in flight.
    if (m_launchStates.value(desktopId) == QLatin1String("launching"))
        return;
    launchDockApp(desktopId);
}

void ShellController::launchDockApp(const QString &desktopId)
{
    const DesktopEntry entry = m_index.byId(desktopId);
    if (!DesktopEntryIndex::isLaunchable(entry)) {
        failDockLaunch(desktopId, QStringLiteral("no usable .desktop entry"));
        return;
    }
    const QStringList argv = DesktopEntryIndex::buildLaunchCommand(entry);
    if (argv.isEmpty()) {
        failDockLaunch(desktopId, QStringLiteral("empty launch command"));
        return;
    }
    qint64 pid = 0;
    if (!QProcess::startDetached(argv.first(), argv.mid(1), QDir::homePath(), &pid)) {
        failDockLaunch(desktopId, QStringLiteral("QProcess::startDetached failed"));
        return;
    }
    qInfo() << "shell: Dock launched" << entry.name << "pid" << pid;
    m_launchStates.insert(desktopId, QStringLiteral("launching"));
    m_launchStart.insert(desktopId, QDateTime::currentMSecsSinceEpoch());
    m_launchDeadlines.insert(desktopId, QDateTime::currentMSecsSinceEpoch() + kLaunchTimeoutMs);
    if (m_launchTimer && !m_launchTimer->isActive())
        m_launchTimer->start();
    ensureDockAnimation();
    rebuildDockEntries();
}

void ShellController::failDockLaunch(const QString &desktopId, const QString &reason)
{
    qWarning() << "shell: Dock launch failed for" << desktopId << ":" << reason;
    m_launchStates.insert(desktopId, QStringLiteral("failed"));
    m_launchDeadlines.remove(desktopId);
    rebuildDockEntries();
    scheduleLaunchStateClear(desktopId);
}

void ShellController::scheduleLaunchStateClear(const QString &desktopId)
{
    // A one-shot notice: the failed mark is transient (T-25 owns the real
    // notification surface).
    QTimer::singleShot(4000, this, [this, desktopId]() {
        if (m_launchStates.value(desktopId) == QLatin1String("failed")) {
            m_launchStates.remove(desktopId);
            rebuildDockEntries();
        }
    });
}

void ShellController::onDockLaunchTick()
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    bool any = false;
    for (auto it = m_launchDeadlines.begin(); it != m_launchDeadlines.end();) {
        if (it.value() <= now) {
            const QString desktopId = it.key();
            it = m_launchDeadlines.erase(it);
            if (m_launchStates.value(desktopId) == QLatin1String("launching")) {
                m_launchStates.insert(desktopId, QStringLiteral("failed"));
                qWarning() << "shell: Dock launch timed out for" << desktopId
                           << "(no window mapped)";
                scheduleLaunchStateClear(desktopId);
            }
        } else {
            any = true;
            ++it;
        }
    }
    if (!any && m_launchTimer)
        m_launchTimer->stop();
    rebuildDockEntries();
}

void ShellController::onDockEntryContextMenu(const QVariant &entry, qreal, qreal)
{
    // Context menus are the next T-10 slice; log so the interaction is visible.
    qInfo() << "shell: Dock context menu requested (T-10 menus pending):"
            << entry.toMap().value(QStringLiteral("name")).toString();
}

void ShellController::onDockDividerContextMenu(qreal, qreal)
{
    qInfo() << "shell: Dock divider menu requested (T-10 menus pending)";
}

void ShellController::onDockPointerMoved(qreal x, qreal y)
{
    if (!m_dockWindow)
        return;
    // The compositor delivers Dock-surface coordinates in scene space; the
    // scene is offset inside the offscreen window when a popover needs
    // headroom (above a bottom Dock) or a side gutter (beside a vertical
    // Dock).
    const QPointF p(x + m_dockItemOffsetX, y + m_dockItemOffsetY);
    QMouseEvent event(QEvent::MouseMove, p, p, Qt::NoButton, m_dockButtons, Qt::NoModifier);
    QCoreApplication::sendEvent(m_dockWindow, &event);
    scheduleDockRender();
}

void ShellController::onDockPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_dockWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    else if (button == 0x111)
        qtButton = Qt::RightButton;
    if (pressed)
        m_dockButtons |= qtButton;
    else
        m_dockButtons &= ~qtButton;
    const QPointF p(x + m_dockItemOffsetX, y + m_dockItemOffsetY);
    QMouseEvent event(pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p, p,
                      qtButton, m_dockButtons, Qt::NoModifier);
    QCoreApplication::sendEvent(m_dockWindow, &event);
    scheduleDockRender();
}

void ShellController::onDockPointerLeft()
{
    if (!m_dockWindow)
        return;
    QMouseEvent event(QEvent::MouseMove, QPointF(-1, -1), QPointF(-1, -1), Qt::NoButton,
                      m_dockButtons, Qt::NoModifier);
    QCoreApplication::sendEvent(m_dockWindow, &event);
    scheduleDockRender();
}

void ShellController::onDockPopupPointerMoved(qreal x, qreal y)
{
    // Popover-local -> Dock-scene-local; `onDockPointerMoved` adds the scene
    // offset for the offscreen window.
    onDockPointerMoved(x + m_dockPopoverX, y + m_dockPopoverY);
}

void ShellController::onDockPopupPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    onDockPointerButton(x + m_dockPopoverX, y + m_dockPopoverY, button, pressed);
}

void ShellController::onDockPopupPointerLeft()
{
    onDockPointerLeft();
}

void ShellController::onDockEntryMenuAction(const QString &action, const QVariant &payload)
{
    const QVariantMap map = payload.toMap();
    if (action == QLatin1String("activate_window")) {
        m_protocol->selectToplevel(map.value(QStringLiteral("windowId")).toString());
    } else if (action == QLatin1String("show_all_windows")) {
        // T-11 owns a filtered Mission Control; until it lands this enters the
        // unfiltered overview (T-10 section 9).
        m_protocol->enterMissionControl();
    } else if (action == QLatin1String("keep_in_dock")) {
        const QString id = map.value(QStringLiteral("desktopId")).toString();
        if (!id.isEmpty() && m_pins.add(id) && m_pins.save())
            rebuildDockEntries();
    } else if (action == QLatin1String("remove_from_dock")) {
        const QString id = map.value(QStringLiteral("desktopId")).toString();
        if (!id.isEmpty() && m_pins.remove(id) && m_pins.save())
            rebuildDockEntries();
    } else if (action == QLatin1String("quit")) {
        m_protocol->closeApp(map.value(QStringLiteral("appId")).toString());
    } else if (action == QLatin1String("open")) {
        const QString id = map.value(QStringLiteral("desktopId")).toString();
        if (!id.isEmpty())
            launchDockApp(id);
    } else if (action == QLatin1String("toggle_magnification")) {
        // The divider menu toggle (T-10 section 13/19); no surface change.
        m_settings.setMagnification(m_settings.magnification() > 0 ? 0.0 : 0.5);
        saveDockSettings();
        applyDockSettings(false);
    } else if (action == QLatin1String("toggle_autohide")) {
        // Auto-hide flips the reserved zone, so the surface must reconfigure.
        m_settings.setAutohide(!m_settings.autohide());
        saveDockSettings();
        applyDockSettings(true);
    } else if (action == QLatin1String("open_dock_settings")) {
        // T-16 owns the Desktop & Dock pane; the entry point is wired.
        qInfo() << "shell: Dock Settings requested (T-16)";
    }
    scheduleDockRender();
}

void ShellController::onDockWindowActivated(const QString &windowId)
{
    m_protocol->selectToplevel(windowId);
    scheduleDockRender();
}

void ShellController::onDockPinnedOrderChanged(const QVariant &desktopIds)
{
    // A drag finished: reorder, promote ("Keep in Dock"), or remove. The Dock
    // sends the complete ordered pinned set (T-10 section 12, FR-9).
    QStringList ids;
    const QVariantList list = desktopIds.toList();
    for (const QVariant &value : list) {
        const QString id = value.toString();
        if (!id.isEmpty() && !ids.contains(id))
            ids.append(id);
    }
    m_pins.setIds(ids);
    if (!m_pins.save())
        qWarning() << "shell: cannot save Dock pins:" << m_pins.lastError();
    rebuildDockEntries();
}

void ShellController::onDockPopoverChanged()
{
    scheduleDockRender();
    startDockAnimationRenders(220);
}

void ShellController::onDockRevealStateChanged()
{
    // The reveal/hide translation changed the committed image and the input
    // region (the hidden Dock keeps only its edge band; T-10 section 15).
    scheduleDockRender();
}

void ShellController::startDockAnimationRenders(int ms)
{
    if (!m_dockPopupTimer)
        return;
    m_dockPopupTicks = qMax(1, ms / m_dockPopupTimer->interval());
    if (!m_dockPopupTimer->isActive())
        m_dockPopupTimer->start();
}

void ShellController::scheduleDockRender()
{
    if (m_dockRenderPending)
        return;
    m_dockRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_dockRenderPending = false;
        renderDock();
    });
}

void ShellController::renderDock()
{
    if (!m_dockItem || !m_dockWindow || m_dockWidth <= 0 || m_dockHeight <= 0)
        return;
    m_dockItem->setWidth(m_dockWidth);
    m_dockItem->setHeight(m_dockHeight);

    // The Dock popover (context menu / window chooser) in Dock-scene
    // coordinates; empty when nothing is open. `popoverRect` keys on the
    // popover's `open || visible` so the open/close fade is still captured.
    const QVariantMap popover = m_dockItem->property("popoverRect").toMap();
    const int px = qFloor(popover.value(QStringLiteral("x")).toReal());
    const int py = qFloor(popover.value(QStringLiteral("y")).toReal());
    const int pw = qCeil(popover.value(QStringLiteral("w")).toReal());
    const int ph = qCeil(popover.value(QStringLiteral("h")).toReal());
    const bool hasPopover = pw > 0 && ph > 0;

    // A popover above a bottom bar needs headroom; beside a vertical bar it
    // needs a left or right gutter. Grow the offscreen window and offset the
    // Dock item so scene (0,0) stays the surface top-left and the whole
    // popover is inside the buffer.
    const int headroom = hasPopover ? qMax(0, -py) : 0;
    const int leftGutter = hasPopover ? qMax(0, -px) : 0;
    const int rightGutter = hasPopover ? qMax(0, (px + pw) - m_dockWidth) : 0;
    m_dockItemOffsetX = leftGutter;
    m_dockItemOffsetY = headroom;
    m_dockItem->setX(leftGutter);
    m_dockItem->setY(headroom);
    const int windowWidth = m_dockWidth + leftGutter + rightGutter;
    const int windowHeight = m_dockHeight + headroom;
    if (m_dockWindow->width() != windowWidth || m_dockWindow->height() != windowHeight)
        m_dockWindow->resize(windowWidth, windowHeight);
    if (!m_dockWindow->isVisible())
        m_dockWindow->show();
    const QImage image = m_dockWindow->grabWindow();

    // The input region is the visible bar plus the currently magnified or
    // bouncing icon rectangles; the transparent magnified band and the hidden
    // Dock pass clicks through (FR-13).
    QList<QRect> inputRects;
    const QVariantList rawRects = m_dockItem->property("inputRects").toList();
    for (const QVariant &value : rawRects) {
        const QVariantMap rect = value.toMap();
        inputRects.append(QRect(qFloor(rect.value(QStringLiteral("x")).toReal()),
                                qFloor(rect.value(QStringLiteral("y")).toReal()),
                                qCeil(rect.value(QStringLiteral("w")).toReal()),
                                qCeil(rect.value(QStringLiteral("h")).toReal())));
    }
    m_protocol->setDockInputRegion(inputRects);
    if (!m_protocol->commitDockImage(
                image.copy(leftGutter, headroom, m_dockWidth, m_dockHeight)))
        qWarning() << "shell: failed to commit the Dock:" << m_protocol->lastError();

    // Place and commit the popover on the Dock's overlay surface. The
    // popover is anchored to the Dock's edge, so the margins are measured
    // from that edge: scene y=0 is the Dock surface top and scene x=0 its
    // left. A bottom popover sits above the bar (bottom margin); a vertical
    // Dock's popover sits beside the bar (left/right margin) at the entry's
    // scene y (T-10 section 5).
    m_dockPopoverX = px;
    m_dockPopoverY = py;
    m_dockPopoverWidth = pw;
    m_dockPopoverHeight = ph;
    if (hasPopover) {
        int top = 0;
        int right = 0;
        int bottom = 0;
        int left = 0;
        switch (m_dockPosition) {
        case ShellProtocol::DockPosition::Left:
            top = py;
            left = px;
            break;
        case ShellProtocol::DockPosition::Right:
            top = py;
            right = m_dockWidth - (px + pw);
            break;
        case ShellProtocol::DockPosition::Bottom:
        default:
            bottom = m_dockHeight - (py + ph);
            left = px;
            break;
        }
        m_protocol->setDockPopupGeometry(top, right, bottom, left, pw, ph);
        const QRect popupRect(leftGutter + px, headroom + py, pw, ph);
        if (!m_protocol->commitDockPopupImage(image.copy(popupRect)))
            qWarning() << "shell: failed to commit the Dock popover:"
                       << m_protocol->lastError();
        m_dockPopoverMapped = true;
    } else if (m_dockPopoverMapped) {
        m_protocol->hideDockPopup();
        m_dockPopoverMapped = false;
    }
}

void ShellController::render()
{
    if (!m_item || m_width <= 0 || m_barHeight <= 0)
        return;
    // The bar item itself stays bar-height; an open dropdown overflows it and
    // is committed to the separate overlay surface.
    m_item->setWidth(m_width);
    m_item->setHeight(m_barHeight);
    const bool popupShown = m_popupWidth > 0 && m_popupHeight > 0;
    const int windowHeight =
        popupShown ? qMax(m_barHeight, m_popupY + m_popupHeight) : m_barHeight;
    if (m_window->width() != m_width || m_window->height() != windowHeight)
        m_window->resize(m_width, windowHeight);
    if (!m_window->isVisible())
        m_window->show();
    const QImage image = m_window->grabWindow();
    if (!m_protocol->commitImage(image.copy(0, 0, m_width, m_barHeight)))
        qWarning() << "shell: failed to commit the menu bar:" << m_protocol->lastError();
    if (popupShown) {
        const QRect popupRect(m_popupX, m_popupY, m_popupWidth, m_popupHeight);
        if (!m_protocol->commitPopupImage(image.copy(popupRect)))
            qWarning() << "shell: failed to commit the menu popup:" << m_protocol->lastError();
    }
}
