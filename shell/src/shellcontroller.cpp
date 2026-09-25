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
#include <QtGlobal>
#include <QtMath>

#include <QCoreApplication>
#include <QDateTime>
#include <QDesktopServices>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QProcess>
#include <QStandardPaths>
#include <QStyleHints>
#include <QUrl>

#include <cstdio>

#include "desktopentry.h"
#include "dockdrops.h"
#include "dockmodel.h"
#include "downloadsmonitor.h"
#include "shellprotocol.h"
#include "systemstatusclient.h"
#include "themebinding.h"

namespace {

// How long a launch may take to map its first window before the Dock treats
// it as failed (T-10 section 8.5). Interim until app-index owns activation.
constexpr qint64 kLaunchTimeoutMs = 8000;

QVariantMap statusItem(const QString &id, const QString &icon, const QString &label,
                       const QString &accessibleName, bool available, qreal level = 0.8,
                       bool enabled = true)
{
    QVariantMap item;
    item.insert(QStringLiteral("id"), id);
    item.insert(QStringLiteral("icon"), icon);
    item.insert(QStringLiteral("label"), label);
    item.insert(QStringLiteral("accessibleName"), accessibleName);
    item.insert(QStringLiteral("available"), available);
    item.insert(QStringLiteral("enabled"), enabled);
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

QVariantMap menuSubmenu(const QString &label, const QVariantList &items)
{
    QVariantMap entry;
    entry.insert(QStringLiteral("label"), label);
    entry.insert(QStringLiteral("type"), QStringLiteral("submenu"));
    entry.insert(QStringLiteral("submenu"), items);
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
// menu-broker lands. Applied to every focused app until T-14.7 retires it.
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
                             menuEntry(QStringLiteral("Enter Full Screen"), QStringLiteral("Ctrl+Ctrl+F")),
                             menuSeparator(),
                             menuSubmenu(QStringLiteral("Sort By"),
                                         QVariantList{menuEntry(QStringLiteral("Name"), QString(), QStringLiteral("sort-name")),
                                                      menuEntry(QStringLiteral("Size"), QString(), QStringLiteral("sort-size")),
                                                      menuEntry(QStringLiteral("Kind"), QString(), QStringLiteral("sort-kind"))})});
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
    delete m_switcherWindow;
    delete m_overviewWindow;
    delete m_dockWindow;
    delete m_window;
}

QString ShellController::lastError() const
{
    return m_protocol ? m_protocol->lastError() : QStringLiteral("shell not started");
}

bool ShellController::start(const QString &socketName, const QString &tokenHex, int barHeight)
{
    m_barHeight = barHeight;
    m_protocol = new ShellProtocol(this);
    connect(m_protocol, &ShellProtocol::fatal, this, [](const QString &message) {
        qWarning() << "shell:" << message;
    });

    // Interim app-index (T-23). The index is scanned once at startup; a real
    // app-index will push install/uninstall events instead.
    m_index.scan();
    // T-08.2a: settingsd is the single Dock-settings owner. The client is the
    // live `org.dragonfruit.Settings1` client; it is seeded with the schema
    // defaults so the Dock works when the daemon is absent (the dev tool does
    // not start settingsd), and a GetAll resyncs once it appears. There is no
    // file owner and no `QFileSystemWatcher`: `Changed` signals are the only
    // notification.
    m_settingsClient = new DbusSettingsClient(this);
    connect(m_settingsClient, &SettingsClient::changed, this,
            &ShellController::onSettingsChanged);
    // A daemon snapshot (or the initial mock refresh) re-seeds defaults if the
    // persisted pinned set is empty.
    connect(m_settingsClient, &SettingsClient::refreshed, this,
            &ShellController::seedDefaultDockPins);
    // T-08.2c: the compositor's motion/input policy is a separate view of the
    // same client (it is not part of DockConfig), so it is applied on every
    // settings change and on a fresh daemon snapshot.
    connect(m_settingsClient, &SettingsClient::changed, this,
            &ShellController::applyCompositorPolicy);
    connect(m_settingsClient, &SettingsClient::refreshed, this,
            &ShellController::applyCompositorPolicy);
    // T-09.3: the wallpaper selection is a separate view of the same client;
    // forward it to the compositor on every change and fresh snapshot.
    connect(m_settingsClient, &SettingsClient::changed, this,
            &ShellController::applyWallpaperPolicy);
    connect(m_settingsClient, &SettingsClient::refreshed, this,
            &ShellController::applyWallpaperPolicy);
    // T-09.5: the display selection is another view of the same client;
    // forward it to the compositor on every change and fresh snapshot.
    connect(m_settingsClient, &SettingsClient::changed, this,
            &ShellController::applyDisplayPolicy);
    connect(m_settingsClient, &SettingsClient::refreshed, this,
            &ShellController::applyDisplayPolicy);
    // `auto` follows the host scheme for the compositor too; ThemeBinding owns
    // the Theme side, this keeps the forwarded scheme live.
    if (QStyleHints *hints = QGuiApplication::styleHints()) {
        connect(hints, &QStyleHints::colorSchemeChanged, this,
                &ShellController::applyCompositorPolicy);
    }
    m_dockConfig = dockConfigFromValues(m_settingsClient->values());
    if (!m_settingsClient->isAvailable()) {
        // No daemon: seed now so the first frame already shows the default
        // pins; when the daemon later appears, its `dock.pinned` wins.
        seedDefaultDockPins();
    }

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
    // T-08.2b: the design-system Theme is bound to settingsd through the same
    // client as the Dock (one bus connection). The binding owns `Theme.dark`
    // (`appearance.colorScheme`) and `Theme.reducedMotion`
    // (`accessibility.reduceMotion`) for the whole process, so no QML side
    // assigns them.
    m_themeBinding = new ThemeBinding(m_settingsClient, m_engine, this);
    m_themeBinding->apply();
    // T-08.2c: forward the initial motion/input policy now that the protocol
    // is authenticated (the compositor is the sole applier).
    applyCompositorPolicy();
    // T-09.3: forward the persisted wallpaper selection. The protocol retries
    // once the manager has announced its Spaces.
    applyWallpaperPolicy();
    // T-09.5: forward the persisted display selection. The protocol retries
    // once the manager has announced its outputs.
    applyDisplayPolicy();
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
    // Scene-graph render path (FR-14): the menu-bar dropdown's open/close
    // fade is committed from the scene graph, not a sampling timer.
    connect(m_window, &QQuickWindow::afterRendering, this,
            &ShellController::onMenuAfterRendering);

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
    connect(m_protocol, &ShellProtocol::dockKeyboardFocused, this,
            &ShellController::onDockKeyboardFocused);
    connect(m_protocol, &ShellProtocol::inputAction, this,
            &ShellController::onInputAction);
    connect(m_protocol, &ShellProtocol::dockExternalDragEntered, this,
            &ShellController::onDockExternalDragEntered);
    connect(m_protocol, &ShellProtocol::dockExternalDragMoved, this,
            &ShellController::onDockExternalDragMoved);
    connect(m_protocol, &ShellProtocol::dockExternalDragLeft, this,
            &ShellController::onDockExternalDragLeft);
    connect(m_protocol, &ShellProtocol::dockExternalDropped, this,
            &ShellController::onDockExternalDropped);

    // T-07.5a/T-07.5b system-status bridge: the model decodes the host's
    // Wi-Fi/volume/battery views; the client is the live session-bus client,
    // or the fixture client when `DF_STATUS_FIXTURE` is set (headless/capture
    // only). Wired before the first apply so the initial views reach the bar.
    m_statusModel = new SystemStatusModel(this);
    if (qEnvironmentVariableIsSet("DF_STATUS_FIXTURE"))
        m_statusClient = new MockSystemStatusClient(this);
    else
        m_statusClient = new DbusSystemStatusClient(this);
    connect(m_statusModel, &SystemStatusModel::changed, this,
            &ShellController::applyStatusMenuData);
    connect(m_statusClient, &SystemStatusClient::wifiState, this,
            &ShellController::onWifiState);
    connect(m_statusClient, &SystemStatusClient::audioState, this,
            &ShellController::onAudioState);
    connect(m_statusClient, &SystemStatusClient::batteryState, this,
            &ShellController::onBatteryState);
    connect(m_statusClient, &SystemStatusClient::joinReport, this,
            &ShellController::onStatusReport);
    connect(m_statusClient, &SystemStatusClient::writeReport, this,
            &ShellController::onStatusReport);
    connect(m_item, SIGNAL(wifiJoinRequested(QString,QString)), this,
            SLOT(onWifiJoinRequested(QString,QString)));
    connect(m_item, SIGNAL(volumeSetRequested(double)), this,
            SLOT(onVolumeSetRequested(double)));
    connect(m_item, SIGNAL(muteToggleRequested()), this, SLOT(onMuteToggleRequested()));
    connect(m_item, SIGNAL(statusMenuRefreshRequested(QString)), this,
            SLOT(onStatusMenuRefreshRequested(QString)));
    connect(m_item, SIGNAL(statusMenuOpened()), this, SLOT(onStatusMenuOpened()));
    connect(m_item, SIGNAL(statusMenuClosed()), this, SLOT(onStatusMenuClosed()));
    applyStatusMenuData();
    // Seed all three views once; the mock answers synchronously, the host is
    // asked asynchronously and answers with its signal.
    m_statusClient->refreshWifi();
    m_statusClient->refreshAudio();
    m_statusClient->refreshBattery();

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
    connect(m_dockItem, SIGNAL(keyboardFocusReleaseRequested()), this,
            SLOT(onDockKeyboardFocusReleaseRequested()));
    connect(m_dockItem, SIGNAL(externalDropRequested(QString,QString,QString,bool)), this,
            SLOT(onDockExternalDropRequested(QString,QString,QString,bool)));
    connect(m_dockItem, SIGNAL(downloadActivated(QString)), this,
            SLOT(onDockDownloadActivated(QString)));
    connect(m_dockItem, SIGNAL(downloadsFolderRequested()), this,
            SLOT(onDockDownloadsFolderRequested()));
    connect(m_dockItem, SIGNAL(downloadsViewed()), this,
            SLOT(onDockDownloadsViewed()));
    connect(m_dockItem, SIGNAL(dockSizePreview(qreal)), this,
            SLOT(onDockSizePreview(qreal)));
    connect(m_dockItem, SIGNAL(dockSizeChanged(qreal)), this,
            SLOT(onDockSizeChanged(qreal)));
    // Scene-graph render path (FR-14): commit a frame whenever the Dock scene
    // graph renders one, so QML-driven animation (magnification, popover
    // fade/scale, drag gaps) reaches the compositor without a sampling timer.
    connect(m_dockWindow, &QQuickWindow::afterRendering, this,
            &ShellController::onDockAfterRendering);
    // The Trash entry's state comes from the home-trash watch (section 16);
    // deletions by any application update the full/count state.
    m_trash = new TrashMonitor(TrashMonitor::defaultRoot(), this);
    connect(m_trash, &TrashMonitor::changed, this, &ShellController::onTrashChanged);
    m_trash->start();
    m_dockItem->setProperty("trashFull", m_trash->isFull());
    m_dockItem->setProperty("trashCount", m_trash->itemCount());
    m_dockItem->setProperty("trashAvailable", m_trash->isAvailable());
    // The Downloads stack (section 17): a zero-polling watch of the folder
    // that feeds the stack popover and its new-items badge.
    m_downloads = new DownloadsMonitor(DownloadsMonitor::defaultDirectory(), this);
    connect(m_downloads, &DownloadsMonitor::changed, this, &ShellController::onDownloadsChanged);
    m_downloads->start();
    onDownloadsChanged();
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

    // Mission Control overview chrome (T-11 Slice B): a third offscreen scene.
    // It is a pure projection of the compositor's workspace/window state; the
    // compositor renders the live surfaces underneath.
    m_overviewWindow = new QQuickWindow;
    m_overviewWindow->setColor(Qt::transparent);
    QQmlComponent overviewComponent(m_engine);
    overviewComponent.loadFromModule(QStringLiteral("Dragonfruit.Overview"),
                                     QStringLiteral("Overview"));
    if (overviewComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: Overview QML error: %s\n",
                qPrintable(overviewComponent.errorString()));
        return false;
    }
    QObject *overviewObject = overviewComponent.create();
    m_overviewItem = qobject_cast<QQuickItem *>(overviewObject);
    if (!m_overviewItem) {
        fprintf(stderr, "dragonfruit-shell: Overview QML did not produce an item\n");
        return false;
    }
    m_overviewItem->setParentItem(m_overviewWindow->contentItem());
    connect(m_overviewItem, SIGNAL(workspaceActivated(int)), this,
            SLOT(onOverviewWorkspaceActivated(int)));
    connect(m_overviewItem, SIGNAL(windowActivated(QString)), this,
            SLOT(onOverviewWindowActivated(QString)));
    connect(m_overviewItem, SIGNAL(windowMovedToWorkspace(QString,int)), this,
            SLOT(onOverviewWindowMovedToWorkspace(QString,int)));
    connect(m_overviewItem, SIGNAL(dismissRequested()), this,
            SLOT(onOverviewDismissRequested()));
    connect(m_overviewWindow, &QQuickWindow::afterRendering, this,
            &ShellController::renderOverview);
    connect(m_protocol, &ShellProtocol::overviewConfigured, this,
            &ShellController::onOverviewConfigured);
    connect(m_protocol, &ShellProtocol::overviewChanged, this,
            &ShellController::onOverviewChanged);
    connect(m_protocol, &ShellProtocol::overviewProgress, this,
            &ShellController::onOverviewProgress);
    connect(m_protocol, &ShellProtocol::overviewDataChanged, this,
            &ShellController::onOverviewDataChanged);
    connect(m_protocol, &ShellProtocol::overviewPointerMoved, this,
            &ShellController::onOverviewPointerMoved);
    connect(m_protocol, &ShellProtocol::overviewPointerButton, this,
            &ShellController::onOverviewPointerButton);
    connect(m_protocol, &ShellProtocol::overviewPointerLeft, this,
            &ShellController::onOverviewPointerLeft);
    connect(m_protocol, &ShellProtocol::overviewKeyboardFocused, this,
            &ShellController::onOverviewKeyboardFocused);
    refreshOverviewData();

    // App-switcher overlay (T-06.2a): a fourth offscreen scene. It draws only
    // the centered app cards and scrim; the compositor renders the live
    // preview surfaces underneath from the same recency projection.
    m_switcherWindow = new QQuickWindow;
    m_switcherWindow->setColor(Qt::transparent);
    QQmlComponent switcherComponent(m_engine);
    switcherComponent.loadFromModule(QStringLiteral("Dragonfruit.Switcher"),
                                     QStringLiteral("AppSwitcher"));
    if (switcherComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: AppSwitcher QML error: %s\n",
                qPrintable(switcherComponent.errorString()));
        return false;
    }
    QObject *switcherObject = switcherComponent.create();
    m_switcherItem = qobject_cast<QQuickItem *>(switcherObject);
    if (!m_switcherItem) {
        fprintf(stderr, "dragonfruit-shell: AppSwitcher QML did not produce an item\n");
        return false;
    }
    m_switcherItem->setParentItem(m_switcherWindow->contentItem());
    connect(m_switcherWindow, &QQuickWindow::afterRendering, this,
            &ShellController::renderSwitcher);
    connect(m_protocol, &ShellProtocol::switcherConfigured, this,
            &ShellController::onSwitcherConfigured);
    connect(m_protocol, &ShellProtocol::appSwitcherChanged, this,
            &ShellController::onAppSwitcherChanged);

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
    // The Mission Control overview chrome is a full-output `overlay` surface,
    // unmapped until the overview opens (T-11 Slice B).
    if (!m_protocol->createOverviewSurface())
        return false;
    // The app-switcher overlay is a full-output `overlay` surface, unmapped
    // until the compositor opens the switcher (T-06.2a).
    if (!m_protocol->createSwitcherSurface())
        return false;

    // Launch timeout: a launch that produces no window within the bounded
    // window returns to not-running and raises a one-shot notice (T-10
    // section 8.5). This is the interim stand-in for the app-index
    // activation/attention signal (FR-4).
    m_launchTimer = new QTimer(this);
    m_launchTimer->setInterval(500);
    connect(m_launchTimer, &QTimer::timeout, this, &ShellController::onDockLaunchTick);

    // Dock animation clock (T-10 section 8.1): 16 ms while a launch or
    // attention bounce is in flight, stopped otherwise so the idle Dock
    // contributes zero wakeups (FR-8). The clock only advances the model; the
    // resulting QML change renders through the scene-graph commit path
    // (`afterRendering`, FR-14), not a render timer.
    m_dockAnimTimer = new QTimer(this);
    m_dockAnimTimer->setInterval(16);
    connect(m_dockAnimTimer, &QTimer::timeout, this, &ShellController::onDockAnimationTick);

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
    const QVariantMap wifi = m_statusModel ? m_statusModel->wifi() : QVariantMap();
    const QVariantMap audio = m_statusModel ? m_statusModel->audio() : QVariantMap();
    const QVariantMap battery = m_statusModel ? m_statusModel->battery() : QVariantMap();

    // Wi-Fi, volume, and battery are live from the bridge host (T-07.5a/b);
    // an absent daemon hides the slot, an error shows it visible but inert
    // (FR-4). `--placeholders` is gone, so no item is faked.
    QString wifiGlyph = wifi.value(QStringLiteral("glyph")).toString();
    if (wifiGlyph.isEmpty())
        wifiGlyph = QStringLiteral("wifi");
    items << statusItem(QStringLiteral("wifi"), wifiGlyph, QString(), tr("Wi-Fi"),
                        wifi.value(QStringLiteral("visible")).toBool(), 0.8,
                        wifi.value(QStringLiteral("enabled")).toBool());

    // Bluetooth, Focus, and Accessibility are later tasks (T-15/T-11); they
    // stay hidden until their adapters land.
    items << statusItem(QStringLiteral("bluetooth"), QStringLiteral("bluetooth"), QString(),
                        tr("Bluetooth"), false);

    QString audioGlyph = audio.value(QStringLiteral("glyph")).toString();
    if (audioGlyph.isEmpty())
        audioGlyph = QStringLiteral("volume");
    items << statusItem(QStringLiteral("volume"), audioGlyph, QString(), tr("Volume"),
                        audio.value(QStringLiteral("visible")).toBool(),
                        audio.value(QStringLiteral("volume")).toReal(),
                        audio.value(QStringLiteral("enabled")).toBool());

    // The battery is read-only; the model hides it both when UPower is absent
    // and when the machine has no present battery. Charging swaps the glyph.
    QString batteryGlyph = battery.value(QStringLiteral("glyph")).toString();
    if (batteryGlyph.isEmpty())
        batteryGlyph = QStringLiteral("battery");
    if (battery.value(QStringLiteral("charging")).toBool())
        batteryGlyph = QStringLiteral("battery-charging");
    items << statusItem(QStringLiteral("battery"), batteryGlyph, QString(), tr("Battery"),
                        battery.value(QStringLiteral("visible")).toBool(),
                        battery.value(QStringLiteral("level")).toReal(),
                        battery.value(QStringLiteral("enabled")).toBool());

    items << statusItem(QStringLiteral("focus"), QStringLiteral("focus"), QString(),
                        tr("Focus"), false);
    items << statusItem(QStringLiteral("accessibility"), QStringLiteral("accessibility"),
                        QString(), tr("Accessibility"), false);
    m_item->setProperty("statusItems", items);
}

void ShellController::applyStatusMenuData()
{
    if (!m_item || !m_statusModel)
        return;
    m_item->setProperty("wifiMenu", m_statusModel->wifi());
    m_item->setProperty("volumeMenu", m_statusModel->audio());
    m_item->setProperty("batteryMenu", m_statusModel->battery());
    applyStatusItems();
}

void ShellController::onStatusMenuOpened()
{
    m_menuOpen = true;
    updatePopupGeometry();
}

void ShellController::onStatusMenuClosed()
{
    m_menuOpen = false;
    // Let the close animation (popupClose = 100 ms) play before unmapping the
    // overlay surface, matching the app-menu dismissal.
    QTimer::singleShot(140, this, [this]() {
        if (!m_menuOpen)
            updatePopupGeometry();
    });
}

void ShellController::onStatusMenuRefreshRequested(const QString &itemId)
{
    if (!m_statusClient)
        return;
    if (itemId == QLatin1String("wifi"))
        m_statusClient->refreshWifi();
    else if (itemId == QLatin1String("volume"))
        m_statusClient->refreshAudio();
    else if (itemId == QLatin1String("battery"))
        m_statusClient->refreshBattery();
}

void ShellController::onWifiJoinRequested(const QString &ssid, const QString &secret)
{
    if (m_statusClient)
        m_statusClient->join(ssid, secret);
}

void ShellController::onVolumeSetRequested(double volume)
{
    if (m_statusClient)
        m_statusClient->setVolume(volume);
}

void ShellController::onMuteToggleRequested()
{
    if (!m_statusClient)
        return;
    const bool muted = m_statusModel
                           ? m_statusModel->audio().value(QStringLiteral("muted")).toBool()
                           : false;
    m_statusClient->setMute(!muted);
}

void ShellController::onWifiState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyWifiJson(json);
}

void ShellController::onAudioState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyAudioJson(json);
}

void ShellController::onBatteryState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyBatteryJson(json);
}

void ShellController::onStatusReport(const QByteArray &json)
{
    qInfo() << "shell: system-status action:" << SystemStatusModel::outcomeOf(json);
    // The action reply is not the new state: re-read the host, whose adapter
    // state is the single source of truth.
    if (m_statusClient) {
        m_statusClient->refreshWifi();
        m_statusClient->refreshAudio();
        m_statusClient->refreshBattery();
    }
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
    // a small demo menu stands in so the dropdown (input routing + overlay
    // surface) stays exercisable in a live session. T-14.7 retires it.
    m_item->setProperty("appMenuModel", demoAppMenu());
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
    // Track recency for the suggested entries (`dock.showRecentApps`, T-10
    // section 17). The list is kept even when the setting is off so toggling
    // it on is immediate; a rebuild only happens when it is visible.
    if (!appId.isEmpty()) {
        m_recentAppIds.removeAll(appId);
        m_recentAppIds.prepend(appId);
        while (m_recentAppIds.size() > 12)
            m_recentAppIds.removeLast();
        if (m_dockConfig.showRecentApps)
            rebuildDockEntries();
    }
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
    // The popup's fade/scale-in is QML-driven; the scene-graph hook commits
    // every frame it renders (FR-14).
}

void ShellController::onAppMenuClosed()
{
    m_menuOpen = false;
    // Let the close animation (popupClose = 100 ms) play before unmapping the
    // overlay surface; the scene-graph hook commits the frames it renders.
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

void ShellController::onMenuAfterRendering()
{
    if (!m_menuFrameGate.frameRendered())
        return;
    if (!m_menuSceneGraphCommitLogged) {
        m_menuSceneGraphCommitLogged = true;
        qInfo() << "shell: menu-bar scene-graph commit path active (FR-14)";
    }
    scheduleRender();
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
    // An open Dock popover or Dock keyboard focus owns the keys (navigation,
    // Escape; T-10 sections 13/20); otherwise the menu bar does.
    if (m_dockWindow && m_dockItem
            && (m_dockKeyboardFocused
                || m_dockItem->property("popoverOpen").toBool())) {
        QCoreApplication::sendEvent(m_dockWindow, &event);
        scheduleDockRender();
    } else {
        QCoreApplication::sendEvent(m_window, &event);
        scheduleRender();
    }
}

void ShellController::onDockKeyboardFocused(bool focused)
{
    m_dockKeyboardFocused = focused;
    if (!m_dockItem)
        return;
    m_dockItem->setProperty("keyboardFocused", focused);
    if (focused) {
        // The Dock is the keyboard target: reveal it if auto-hidden, move
        // active focus into the scene so QML Keys handlers fire, and start
        // keyboard navigation on the first entry (T-10 section 20).
        QMetaObject::invokeMethod(m_dockItem, "reveal");
        QMetaObject::invokeMethod(m_dockItem, "beginKeyboardNavigation");
        if (m_dockWindow)
            m_dockItem->forceActiveFocus();
    } else {
        QMetaObject::invokeMethod(m_dockItem, "endKeyboardNavigation");
    }
    scheduleDockRender();
}

void ShellController::onDockKeyboardFocusReleaseRequested()
{
    // Escape left Dock keyboard navigation (T-10 section 20). The Dock has
    // already cleared its ring; ask the compositor to restore the active
    // window's keyboard focus. The resulting wl_keyboard.leave routes back
    // through onDockKeyboardFocused(false), so the state stays compositor-led.
    if (m_dockKeyboardFocused)
        m_protocol->releaseKeyboardFocus();
}

void ShellController::onDockExternalDragEntered(bool payloadIsApp, qreal x, qreal y)
{
    if (!m_dockItem)
        return;
    // Reveal a hidden Dock so the drop target is visible (T-10 section 15).
    QMetaObject::invokeMethod(m_dockItem, "reveal");
    // The payload count is unknown until the drop; the Dock only needs the
    // kind to choose the highlight or the live gap.
    QMetaObject::invokeMethod(m_dockItem, "beginExternalDrag", Q_ARG(QVariant, payloadIsApp),
                              Q_ARG(QVariant, 0));
    onDockExternalDragMoved(x, y);
}

void ShellController::onDockExternalDragMoved(qreal x, qreal y)
{
    if (!m_dockItem)
        return;
    // Dock-surface-local -> offscreen-scene coordinates (the same offset the
    // pointer bridge applies for a popover's headroom/gutter).
    QMetaObject::invokeMethod(m_dockItem, "externalDragTo",
                              Q_ARG(QVariant, x + m_dockItemOffsetX),
                              Q_ARG(QVariant, y + m_dockItemOffsetY));
    scheduleDockRender();
}

void ShellController::onDockExternalDragLeft()
{
    if (!m_dockItem)
        return;
    QMetaObject::invokeMethod(m_dockItem, "externalDragLeft");
    scheduleDockRender();
}

void ShellController::onDockExternalDropped(bool payloadIsApp, const QString &desktopId,
                                            const QStringList &paths, qreal x, qreal y)
{
    m_externalPayloadIsApp = payloadIsApp;
    m_externalDesktopId = desktopId;
    m_externalPaths = paths;
    m_externalDropX = x + m_dockItemOffsetX;
    m_externalDropY = y + m_dockItemOffsetY;
    if (!m_dockItem) {
        m_externalDesktopId.clear();
        m_externalPaths.clear();
        return;
    }
    // The Dock resolves the target and synchronously emits
    // `externalDropRequested`, which performs the action below.
    QMetaObject::invokeMethod(m_dockItem, "externalDrop", Q_ARG(QVariant, m_externalDropX),
                              Q_ARG(QVariant, m_externalDropY));
    scheduleDockRender();
}

void ShellController::onDockExternalDropRequested(const QString &targetId,
                                                  const QString &targetKind,
                                                  const QString &desktopId, bool payloadIsApp)
{
    Q_UNUSED(targetId)
    Q_UNUSED(payloadIsApp)
    // Use the drop-time classification, not the QML's enter-time kind: a
    // single `.desktop` URI is an app alias even though it arrived as files.
    const DockDropAction action = dockDropActionFor(
        targetKind, m_externalPayloadIsApp ? DockDropPayload::Application
                                           : DockDropPayload::Files);
    switch (action) {
    case DockDropAction::PinApp: {
        const QString id = m_externalDesktopId;
        if (id.isEmpty()) {
            qWarning() << "shell: Dock external app drop had no desktop id";
        } else if (!m_dockConfig.pinned.contains(id)) {
            QStringList ids = m_dockConfig.pinned;
            ids.append(id);
            writeDockSetting(QStringLiteral("dock.pinned"), ids);
            rebuildDockEntries();
            qInfo() << "shell: Dock pinned" << id << "from an external drop";
        }
        break;
    }
    case DockDropAction::OpenWithApp:
        if (desktopId.isEmpty())
            qWarning() << "shell: Dock file drop on an unresolved app entry";
        else
            launchDockAppWithFiles(desktopId, m_externalPaths);
        break;
    case DockDropAction::TrashFiles: {
        const int trashed = m_trash->trash(m_externalPaths);
        qInfo() << "shell: Dock trashed" << trashed << "of" << m_externalPaths.size()
                << "dropped items";
        break;
    }
    case DockDropAction::MoveToDownloads: {
        const QString downloads = downloadsDirectory();
        QDir().mkpath(downloads);
        for (const QString &path : m_externalPaths) {
            const QString destination =
                downloads + QLatin1Char('/') + QFileInfo(path).fileName();
            if (QFile::rename(path, destination))
                qInfo() << "shell: Dock moved" << path << "to Downloads";
            else
                qWarning() << "shell: Dock could not move" << path << "to Downloads";
        }
        break;
    }
    case DockDropAction::None:
    default:
        break;
    }
    m_externalPayloadIsApp = false;
    m_externalDesktopId.clear();
    m_externalPaths.clear();
}

void ShellController::onInputAction(const QString &action, const QString &)
{
    if (action == QLatin1String("toggle-dock")) {
        // Super+Option+D toggles `dock.autohide` (T-10 sections 15/20). The
        // auto-hide change flips the reserved zone, so reconfigure.
        writeDockSetting(QStringLiteral("dock.autohide"), !m_dockConfig.autohide);
        applyDockSettings(true);
    } else if (action == QLatin1String("focus-dock")) {
        // The compositor has already focused the Dock surface; reveal it if
        // hidden so the focus ring is visible.
        if (m_dockItem)
            QMetaObject::invokeMethod(m_dockItem, "reveal");
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
    // The output length along the Dock axis is only known now, so re-run the
    // section 5.1 overflow clamp. This never resets the Repeater model unless
    // the hidden set changed (a divider resize reconfigures the surface).
    applyDockOverflowResult(computeDockOverflow(), true);
    renderDock();
}

ShellProtocol::DockPosition ShellController::dockPosition() const
{
    const QString position = m_dockConfig.position;
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
    const int low = qRound(m_dockItem->property("iconSizeMin").toReal());
    const int high = qRound(m_dockItem->property("iconSizeMax").toReal());
    return dockIconSize(size, low, high);
}

// Push the `dock.*` settings onto the Dock QML. `reconfigure` is false at
// startup (the chrome surface does not exist yet) and true for a live change
// that alters the surface extent or reserved zone.
void ShellController::applyDockSettings(bool reconfigure)
{
    if (!m_dockItem)
        return;
    m_dockItem->setProperty("iconSize", iconSizeForSize(m_dockConfig.size));
    m_dockItem->setProperty("magnification", m_dockConfig.magnification);
    m_dockItem->setProperty("autoHide", m_dockConfig.autohide);
    // Enabling auto-hide starts hidden; disabling it always reveals. The
    // QML owns the reveal/hide state machine (T-10 section 15).
    QMetaObject::invokeMethod(m_dockItem,
                              m_dockConfig.autohide ? "hide" : "reveal");
    m_dockItem->setProperty("showIndicators", m_dockConfig.showIndicators);
    m_dockItem->setProperty("minimizeIntoTileIcon", m_dockConfig.minimizeIntoTileIcon);
    m_dockItem->setProperty("animateOpening", m_dockConfig.animateOpening);
    m_dockItem->setProperty("showRecentApps", m_dockConfig.showRecentApps);
    // The position is applied to the surface anchor (T-10 section 5); the
    // QML layout mirrors for a vertical Dock.
    const ShellProtocol::DockPosition position = dockPosition();
    m_dockItem->setProperty("position",
                            position == ShellProtocol::DockPosition::Left ? QStringLiteral("left")
                            : position == ShellProtocol::DockPosition::Right
                                    ? QStringLiteral("right")
                                    : QStringLiteral("bottom"));
    // The reduced-motion policy is no longer sent from here: T-08.2c routes
    // the whole motion/input policy through `applyCompositorPolicy()` so the
    // settingsd value has exactly one applier (the compositor).
    if (!reconfigure || !m_protocol)
        return;
    // Adopt the new edge before the layout clamp so `computeDockOverflow`
    // reads the right axis; a changed stretch dimension is only known from the
    // next configure, so pause rendering until it arrives (no stale frame at
    // the wrong aspect).
    const bool positionChanged = position != m_dockPosition;
    m_dockPosition = position;
    if (positionChanged) {
        if (position == ShellProtocol::DockPosition::Bottom) {
            m_dockHeight = m_dockThickness;
            m_dockWidth = 0;
        } else {
            m_dockWidth = m_dockThickness;
            m_dockHeight = 0;
        }
    }
    // `dock.showRecentApps` changes the app region, so rebuild the entries;
    // the rebuild also applies the section 5.1 overflow clamp.
    rebuildDockEntries();
    // A live geometry change moves the bar, so any open popover is dismissed
    // rather than left floating detached (T-10 section 22).
    QMetaObject::invokeMethod(m_dockItem, "closePopovers");
    // The (possibly clamped) icon size set the baseline bar and the magnified
    // band; re-apply the surface extent, anchor, and reserved zone (auto-hide
    // reserves nothing).
    m_dockBarThickness = qRound(m_dockItem->property("barThickness").toReal());
    m_dockThickness = m_dockBarThickness + qCeil(m_dockItem->property("magnifyBand").toReal());
    const int exclusive = m_dockConfig.autohide ? 0 : m_dockBarThickness;
    if (!m_protocol->configureDockSurface(position, m_dockThickness, exclusive))
        qWarning() << "shell: cannot reconfigure the Dock surface:"
                   << m_protocol->lastError();
    renderDock();
}

// T-08.2a: write one Dock key through settingsd (the single owner) and refresh
// the typed local view. The synchronous `changed` echo is suppressed so the
// caller can pick the right apply path (geometry reconfigure vs the size-only
// divider path); a daemon-originated change arrives outside this guard.
void ShellController::writeDockSetting(const QString &key, const QVariant &value)
{
    if (!m_settingsClient)
        return;
    m_localSettingsWrite = true;
    m_settingsClient->set(key, value);
    m_localSettingsWrite = false;
    m_dockConfig = dockConfigFromValues(m_settingsClient->values());
}

// Seed the installed default pinned set once per session when `dock.pinned` is
// empty (the schema default). This is the shell's only "first run" decision;
// settingsd owns the value from then on.
void ShellController::seedDefaultDockPins()
{
    if (m_defaultPinsSeeded)
        return;
    m_defaultPinsSeeded = true;
    if (!m_dockConfig.pinned.isEmpty())
        return;
    const QStringList defaults = resolveDefaultDockPins(m_index);
    if (defaults.isEmpty())
        return;
    writeDockSetting(QStringLiteral("dock.pinned"), defaults);
    rebuildDockEntries();
}

// The divider resize handle (T-10 section 5): the live preview re-lays-out the
// Dock from an in-memory value only (no settingsd write per pointer move), and
// the release commits the value through settingsd. Neither path rebuilds the
// entries: the Repeater model must stay stable while the QML delegate's
// DragHandler holds the pointer.
void ShellController::onDockSizePreview(qreal fraction)
{
    m_dockConfig.size = qBound(0.0, fraction, 1.0);
    applyDockSizeOnly();
}

void ShellController::onDockSizeChanged(qreal fraction)
{
    writeDockSetting(QStringLiteral("dock.size"), fraction);
    applyDockSizeOnly();
}

void ShellController::applyDockSizeOnly()
{
    if (!m_dockItem || !m_protocol)
        return;
    // The divider drag is the natural place to bound `dock.size`: the clamp
    // keeps the effective icon size at what fits the output, without touching
    // the entries (the QML delegate holds the pointer; section 5.1).
    applyDockOverflowResult(computeDockOverflow(), false);
    const int exclusive = m_dockConfig.autohide ? 0 : m_dockBarThickness;
    if (!m_protocol->configureDockSurface(m_dockPosition, m_dockThickness, exclusive))
        qWarning() << "shell: cannot resize the Dock surface:" << m_protocol->lastError();
    renderDock();
}

void ShellController::onSettingsChanged(const QString &key, const QVariant &)
{
    // The synchronous echo of a local optimistic write is handled by the
    // caller (which knows whether the surface must reconfigure or only
    // re-lay-out). A daemon-originated `Changed` is applied here.
    if (m_localSettingsWrite || !m_settingsClient)
        return;
    const DockConfig updated = dockConfigFromValues(m_settingsClient->values());
    if (updated == m_dockConfig)
        return;
    m_dockConfig = updated;
    applyDockSettings(true);
    fprintf(stderr, "dragonfruit-shell: Dock settings changed (live: %s)\n",
            qPrintable(key));
}

void ShellController::applyCompositorPolicy()
{
    // T-08.2c: one view of the settingsd motion/input keys, forwarded to the
    // compositor over the private protocol. The compositor is the sole
    // applier; the shell keeps no second settings source.
    if (!m_settingsClient || !m_protocol)
        return;
    const CompositorPolicy policy =
        compositorPolicyFromValues(m_settingsClient->values(), ThemeBinding::hostDark());
    if (m_compositorPolicySent && policy == m_compositorPolicy)
        return;
    m_compositorPolicy = policy;
    m_compositorPolicySent = true;
    m_protocol->setReducedMotion(policy.reducedMotion);
    m_protocol->setMotionPolicy(policy.colorScheme, policy.titlebarDoubleClick,
                                policy.minimizedAnimation);
    m_protocol->setInputPolicy(policy.repeatDelayMs, policy.repeatRateHz,
                               policy.gesturesEnabled, policy.gestureSpaceSwitch,
                               policy.gestureMissionControl);
    fprintf(stderr,
            "dragonfruit-shell: compositor motion/input policy applied "
            "(scheme=%s reducedMotion=%d repeat=%d/%d gestures=%d)\n",
            qPrintable(policy.colorScheme), policy.reducedMotion ? 1 : 0,
            policy.repeatDelayMs, policy.repeatRateHz, policy.gesturesEnabled ? 1 : 0);
}

void ShellController::applyWallpaperPolicy()
{
    // T-09.3: one view of the settingsd wallpaper keys, forwarded to the
    // compositor as per-Space `df_workspace.set_wallpaper`. The compositor is
    // the sole applier; the shell keeps no second settings source.
    if (!m_settingsClient || !m_protocol)
        return;
    const WallpaperSettings settings =
        wallpaperSettingsFromValues(m_settingsClient->values());
    if (m_wallpaperSent && settings == m_wallpaperSettings)
        return;
    m_wallpaperSettings = settings;
    m_wallpaperSent = true;
    m_protocol->setWallpaper(settings.source, wallpaperFitFromName(settings.fit),
                             settings.showOnAllSpaces);
    fprintf(stderr,
            "dragonfruit-shell: wallpaper applied (fit=%s allSpaces=%d source=%s)\n",
            qPrintable(settings.fit), settings.showOnAllSpaces ? 1 : 0,
            settings.source.isEmpty() ? "(solid)" : qPrintable(settings.source));
}

void ShellController::applyDisplayPolicy()
{
    // T-09.5: one view of the settingsd display keys, forwarded to the
    // compositor as `df_output.set_scale` / `df_output.set_transform`. The
    // compositor is the sole applier; the shell keeps no second settings
    // source. The protocol retries once the manager has announced its outputs.
    if (!m_settingsClient || !m_protocol)
        return;
    const DisplaySettings settings = displaySettingsFromValues(m_settingsClient->values());
    if (m_displaySent && settings == m_displaySettings)
        return;
    m_displaySettings = settings;
    m_displaySent = true;
    m_protocol->setDisplayPolicy(settings.scale, outputTransformFromName(settings.rotation));
    fprintf(stderr, "dragonfruit-shell: display applied (scale=%.3f rotation=%s)\n",
            settings.scale, qPrintable(settings.rotation));
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
    m_dockAllEntries = withBounce(buildDockEntries(
        m_dockConfig.pinned, m_index, m_runningEntries, m_launchStates,
        m_dockConfig.showRecentApps ? m_recentAppIds : QStringList()));
    // A full rebuild always re-publishes the entries (bounce phases changed);
    // the clamp only decides which ones survive and at what icon size.
    applyDockOverflowResult(computeDockOverflow(), true, true);
    scheduleDockRender();
}

// The T-10 section 5.1 overflow clamp: given the full entry list and the
// output length along the Dock axis, bound the icon size to what fits and hide
// overflow temporary/recent entries (recents first, then temporaries; pinned
// are never dropped). The divider is always present and the stack/Trash are
// the two fixed entries.
DockOverflowResult ShellController::computeDockOverflow() const
{
    if (!m_dockItem)
        return {};
    const int gap = qRound(m_dockItem->property("gap").toReal());
    const int dividerWidth = qRound(m_dockItem->property("dividerWidth").toReal());
    const int iconMin = qRound(m_dockItem->property("iconSizeMin").toReal());
    const int iconMax = qRound(m_dockItem->property("iconSizeMax").toReal());
    const int available = m_dockPosition == ShellProtocol::DockPosition::Bottom
            ? m_dockWidth
            : m_dockHeight;
    return applyDockOverflow(m_dockAllEntries, available, iconSizeForSize(m_dockConfig.size),
                             iconMin, iconMax, gap, dividerWidth, 2,
                             !m_dockConfig.minimizeIntoTileIcon);
}

void ShellController::applyDockOverflowResult(const DockOverflowResult &overflow,
                                              bool allowEntries, bool forceEntries)
{
    if (!m_dockItem)
        return;
    m_dockItem->setProperty("iconSize", overflow.iconSize);
    m_dockBarThickness = qRound(m_dockItem->property("barThickness").toReal());
    m_dockThickness = m_dockBarThickness + qCeil(m_dockItem->property("magnifyBand").toReal());
    warnDockOverflow(overflow);
    if (!allowEntries)
        return;
    const bool hiddenChanged = overflow.hiddenTemporary != m_dockHiddenTemporary
            || overflow.hiddenRecent != m_dockHiddenRecent;
    // A geometry change during a divider resize must not destroy the delegate
    // that holds the pointer, so only reset the Repeater model when the hidden
    // set actually changed (the internal-reorder lesson); a full rebuild
    // forces the fresh entries through.
    if (!hiddenChanged && !forceEntries)
        return;
    m_dockHiddenTemporary = overflow.hiddenTemporary;
    m_dockHiddenRecent = overflow.hiddenRecent;
    m_dockItem->setProperty("entries", overflow.entries);
}

void ShellController::warnDockOverflow(const DockOverflowResult &overflow)
{
    if (!overflow.clamped || m_dockOverflowWarned)
        return;
    m_dockOverflowWarned = true;
    qWarning() << "shell: Dock content exceeds the output; clamped the icon size to"
               << overflow.iconSize << "px and hid"
               << (overflow.hiddenTemporary + overflow.hiddenRecent)
               << "temporary/recent entries"
               << (overflow.overflowed ? "(pinned content still overflows)" : "");
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
    // FR-4: attention stops on focus. An app that already holds focus needs
    // no bounce, and guarding here makes the relative order of the focused
    // and attention broadcasts irrelevant (a newly activated window is
    // focused by the compositor, so its launch feedback is the launch state,
    // not a bounce).
    if (appId == m_appId)
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
        openTrashInFiles();
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

void ShellController::openTrashInFiles()
{
    // Files is T-18; resolve it through the interim index and launch/activate
    // it at the `trash://` URI. The design's `org.dragonfruit.Files1` service
    // name is the activation target once T-18 lands.
    DesktopEntry files = m_index.byId(QStringLiteral("org.dragonfruit.Files.desktop"));
    if (!files.valid)
        files = m_index.resolve(QStringLiteral("org.dragonfruit.Files"));
    if (!DesktopEntryIndex::isLaunchable(files)) {
        qWarning() << "shell: Dock Trash requested but Files is not installed (T-18)";
        return;
    }
    const QStringList argv =
        DesktopEntryIndex::buildLaunchCommand(files, {QStringLiteral("trash://")});
    if (argv.isEmpty()) {
        qWarning() << "shell: Files .desktop has no usable Exec for trash://";
        return;
    }
    qint64 pid = 0;
    if (!QProcess::startDetached(argv.first(), argv.mid(1), QDir::homePath(), &pid))
        qWarning() << "shell: cannot open Trash in Files:" << argv.first();
    else
        qInfo() << "shell: Dock opened Trash in Files (pid" << pid << ")";
}

void ShellController::showDockAppInFiles(const QString &desktopId, const QString &appId)
{
    // Resolve the app to reveal. A pinned entry carries the desktop id; a
    // running temporary entry may only carry the compositor app id.
    DesktopEntry app = m_index.byId(desktopId);
    if (!app.valid)
        app = m_index.resolve(appId);
    if (!DesktopEntryIndex::isLaunchable(app)) {
        qWarning() << "shell: Show in Files: cannot resolve app" << desktopId << appId;
        return;
    }
    const QStringList appArgv = DesktopEntryIndex::buildLaunchCommand(app);
    if (appArgv.isEmpty())
        return;
    const QString target = appArgv.first();
    // Files is T-18; the design's `org.dragonfruit.Files1` service name is the
    // activation target once it lands. Until then launch the Files .desktop
    // with the executable path, the same interim pattern as the Trash.
    DesktopEntry files = m_index.byId(QStringLiteral("org.dragonfruit.Files.desktop"));
    if (!files.valid)
        files = m_index.resolve(QStringLiteral("org.dragonfruit.Files"));
    if (!DesktopEntryIndex::isLaunchable(files)) {
        qWarning() << "shell: Show in Files requested but Files is not installed (T-18):"
                   << target;
        return;
    }
    const QStringList argv = DesktopEntryIndex::buildLaunchCommand(files, {target});
    if (argv.isEmpty()) {
        qWarning() << "shell: Files .desktop has no usable Exec for" << target;
        return;
    }
    qint64 pid = 0;
    if (!QProcess::startDetached(argv.first(), argv.mid(1), QDir::homePath(), &pid))
        qWarning() << "shell: cannot show" << target << "in Files:" << argv.first();
    else
        qInfo() << "shell: Dock revealed" << app.name << "in Files (pid" << pid << ")";
}

void ShellController::launchDockApp(const QString &desktopId)
{
    launchDockAppWithFiles(desktopId, QStringList());
}

void ShellController::launchDockAppWithFiles(const QString &desktopId, const QStringList &files)
{
    const DesktopEntry entry = m_index.byId(desktopId);
    if (!DesktopEntryIndex::isLaunchable(entry)) {
        failDockLaunch(desktopId, QStringLiteral("no usable .desktop entry"));
        return;
    }
    const QStringList argv = DesktopEntryIndex::buildLaunchCommand(entry, files);
    if (argv.isEmpty()) {
        failDockLaunch(desktopId, QStringLiteral("empty launch command"));
        return;
    }
    qint64 pid = 0;
    if (!QProcess::startDetached(argv.first(), argv.mid(1), QDir::homePath(), &pid)) {
        failDockLaunch(desktopId, QStringLiteral("QProcess::startDetached failed"));
        return;
    }
    qInfo() << "shell: Dock launched" << entry.name << "pid" << pid
            << (files.isEmpty() ? QString() : QStringLiteral("with %1 file(s)").arg(files.size()));
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

// --- Mission Control overview chrome (T-11 Slice B) -------------------------

void ShellController::refreshOverviewData()
{
    if (!m_overviewItem)
        return;
    m_overviewItem->setProperty("workspaces", m_protocol->overviewWorkspaces());
    m_overviewItem->setProperty("minimizedWindows", m_protocol->minimizedWindows());
    m_overviewItem->setProperty("windows", m_protocol->overviewWindows());
}

void ShellController::onOverviewConfigured(int width, int height, quint32)
{
    // The overview surface covers the whole output; both the pre-layout and
    // the resolved configure are that size, so accept the first positive one.
    if (width <= 0 || height <= 0)
        return;
    m_overviewWidth = width;
    m_overviewHeight = height;
    fprintf(stderr, "dragonfruit-shell: overview configured %dx%d\n", width, height);
    if (m_overviewActive)
        renderOverview();
}

void ShellController::onOverviewChanged(bool active)
{
    m_overviewActive = active;
    if (!m_overviewItem)
        return;
    m_overviewItem->setProperty("active", active);
    if (active) {
        // The compositor owns workspace truth; re-read the projection and map
        // the chrome surface. The live window surfaces are already being
        // transformed by the compositor underneath.
        refreshOverviewData();
        if (m_overviewItem)
            m_overviewItem->forceActiveFocus();
        renderOverview();
    } else {
        // Selection round-trip or a reverse transition: unmap the chrome so it
        // stops capturing input; the compositor restores the normal scene.
        if (m_protocol)
            m_protocol->hideOverview();
    }
}

void ShellController::onOverviewProgress(qreal progress, const QString &)
{
    if (!m_overviewItem)
        return;
    m_overviewItem->setProperty("progress", progress);
    if (m_overviewActive)
        scheduleOverviewRender();
}

void ShellController::onOverviewDataChanged()
{
    if (!m_overviewActive)
        return;
    refreshOverviewData();
    scheduleOverviewRender();
}

void ShellController::onOverviewWorkspaceActivated(int index)
{
    if (m_protocol)
        m_protocol->activateWorkspace(index);
}

void ShellController::onOverviewWindowActivated(const QString &windowId)
{
    // The selection round-trip (FR-5): the compositor restores the window,
    // activates its Space, focuses it, and leaves the overview.
    if (m_protocol)
        m_protocol->selectToplevel(windowId);
}

void ShellController::onOverviewWindowMovedToWorkspace(const QString &windowId, int index)
{
    // FR-7: dragging a window's overview representation onto another Space
    // moves it there. The compositor re-emits the window's `workspace_entered`
    // and the next `overviewDataChanged` re-reads the projection.
    if (m_protocol)
        m_protocol->moveToplevelToWorkspace(windowId, index);
}

void ShellController::onOverviewDismissRequested()
{
    if (m_protocol && m_overviewActive)
        m_protocol->exitMissionControl();
}

void ShellController::onOverviewKeyboardFocused(bool)
{
    // Escape is handled by the overview QML's Keys handler; the compositor
    // owns the keyboard focus transfer (the surface is OnDemand).
    if (m_overviewItem && m_overviewActive)
        m_overviewItem->forceActiveFocus();
}

void ShellController::onOverviewPointerMoved(qreal x, qreal y)
{
    if (!m_overviewWindow)
        return;
    const QPointF p(x, y);
    QMouseEvent event(QEvent::MouseMove, p, p, Qt::NoButton, m_overviewButtons,
                      Qt::NoModifier);
    QCoreApplication::sendEvent(m_overviewWindow, &event);
    scheduleOverviewRender();
}

void ShellController::onOverviewPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_overviewWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    else if (button == 0x111)
        qtButton = Qt::RightButton;
    if (pressed)
        m_overviewButtons |= qtButton;
    else
        m_overviewButtons &= ~qtButton;
    const QPointF p(x, y);
    QMouseEvent event(pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p, p,
                      qtButton, m_overviewButtons, Qt::NoModifier);
    QCoreApplication::sendEvent(m_overviewWindow, &event);
    scheduleOverviewRender();
}

void ShellController::onOverviewPointerLeft()
{
    if (!m_overviewWindow)
        return;
    QMouseEvent event(QEvent::MouseMove, QPointF(-1, -1), QPointF(-1, -1), Qt::NoButton,
                      m_overviewButtons, Qt::NoModifier);
    QCoreApplication::sendEvent(m_overviewWindow, &event);
    scheduleOverviewRender();
}

void ShellController::scheduleOverviewRender()
{
    if (m_overviewRenderPending)
        return;
    m_overviewRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_overviewRenderPending = false;
        renderOverview();
    });
}

void ShellController::renderOverview()
{
    if (!m_overviewActive || !m_overviewWindow || !m_overviewItem)
        return;
    if (m_overviewWidth <= 0 || m_overviewHeight <= 0)
        return;
    // The scene graph just rendered; skip the re-entrant frame our own
    // `grabWindow` readback produces (FR-14 pattern shared with the Dock).
    if (!m_overviewFrameGate.frameRendered())
        return;
    if (!m_overviewSceneGraphCommitLogged) {
        m_overviewSceneGraphCommitLogged = true;
        qInfo() << "shell: overview scene-graph commit path active";
    }
    m_overviewItem->setWidth(m_overviewWidth);
    m_overviewItem->setHeight(m_overviewHeight);
    if (m_overviewWindow->width() != m_overviewWidth
        || m_overviewWindow->height() != m_overviewHeight)
        m_overviewWindow->resize(m_overviewWidth, m_overviewHeight);
    if (!m_overviewWindow->isVisible())
        m_overviewWindow->show();
    m_overviewFrameGate.beginCommit();
    const QImage image = m_overviewWindow->grabWindow();
    m_overviewFrameGate.endCommit();
    if (!image.isNull() && m_protocol)
        m_protocol->commitOverviewImage(image);
}

// --- app-switcher overlay (T-06.2a) -----------------------------------------

void ShellController::onSwitcherConfigured(int width, int height, quint32)
{
    if (width <= 0 || height <= 0)
        return;
    m_switcherWidth = width;
    m_switcherHeight = height;
    fprintf(stderr, "dragonfruit-shell: app-switcher configured %dx%d\n", width, height);
    if (m_switcherActive)
        renderSwitcher();
}

void ShellController::onAppSwitcherChanged(bool active, const QVariantList &entries,
                                           const QString &selectedAppId, int direction)
{
    m_switcherActive = active;
    if (!m_switcherItem)
        return;
    // The compositor owns the recency order and the selection; the shell only
    // renders the projection. `selectedIndex` is resolved by matching the
    // selected app id, so the shell never sorts or re-derives recency.
    int selectedIndex = -1;
    for (int i = 0; i < entries.size(); ++i) {
        const QVariantMap entry = entries.at(i).toMap();
        if (!selectedAppId.isEmpty()
            && entry.value(QStringLiteral("appId")).toString() == selectedAppId) {
            selectedIndex = i;
            break;
        }
    }
    m_switcherItem->setProperty("active", active);
    m_switcherItem->setProperty("entries", entries);
    m_switcherItem->setProperty("selectedIndex", selectedIndex);
    m_switcherItem->setProperty("direction", direction);
    if (active) {
        renderSwitcher();
    } else if (m_protocol) {
        // The compositor closed the switcher (commit or cancel): unmap the
        // overlay so it stops compositing over the restored scene.
        m_protocol->hideSwitcher();
    }
}

void ShellController::scheduleSwitcherRender()
{
    if (m_switcherRenderPending)
        return;
    m_switcherRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_switcherRenderPending = false;
        renderSwitcher();
    });
}

void ShellController::renderSwitcher()
{
    if (!m_switcherActive || !m_switcherWindow || !m_switcherItem)
        return;
    if (m_switcherWidth <= 0 || m_switcherHeight <= 0)
        return;
    // The scene graph just rendered; skip the re-entrant frame our own
    // `grabWindow` readback produces (FR-14 pattern shared with the Dock).
    if (!m_switcherFrameGate.frameRendered())
        return;
    if (!m_switcherSceneGraphCommitLogged) {
        m_switcherSceneGraphCommitLogged = true;
        qInfo() << "shell: app-switcher scene-graph commit path active";
    }
    m_switcherItem->setWidth(m_switcherWidth);
    m_switcherItem->setHeight(m_switcherHeight);
    if (m_switcherWindow->width() != m_switcherWidth
        || m_switcherWindow->height() != m_switcherHeight)
        m_switcherWindow->resize(m_switcherWidth, m_switcherHeight);
    if (!m_switcherWindow->isVisible())
        m_switcherWindow->show();
    m_switcherFrameGate.beginCommit();
    const QImage image = m_switcherWindow->grabWindow();
    m_switcherFrameGate.endCommit();
    if (!image.isNull() && m_protocol)
        m_protocol->commitSwitcherImage(image);
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
        if (!id.isEmpty() && !m_dockConfig.pinned.contains(id)) {
            QStringList ids = m_dockConfig.pinned;
            ids.append(id);
            writeDockSetting(QStringLiteral("dock.pinned"), ids);
            rebuildDockEntries();
        }
    } else if (action == QLatin1String("remove_from_dock")) {
        const QString id = map.value(QStringLiteral("desktopId")).toString();
        if (!id.isEmpty() && m_dockConfig.pinned.contains(id)) {
            QStringList ids = m_dockConfig.pinned;
            ids.removeAll(id);
            writeDockSetting(QStringLiteral("dock.pinned"), ids);
            rebuildDockEntries();
        }
    } else if (action == QLatin1String("quit")) {
        m_protocol->closeApp(map.value(QStringLiteral("appId")).toString());
    } else if (action == QLatin1String("open")) {
        const QString id = map.value(QStringLiteral("desktopId")).toString();
        if (!id.isEmpty())
            launchDockApp(id);
    } else if (action == QLatin1String("assign_to")) {
        // The app Options submenu (T-10 section 13). "This Desktop" moves the
        // app's live windows to the active Space; "All Desktops" and "None"
        // have no compositor assignment model yet (T-04/T-05).
        const QString target = map.value(QStringLiteral("target")).toString();
        const QString appId = map.value(QStringLiteral("appId")).toString();
        if (target == QLatin1String("this")) {
            m_protocol->assignAppToActiveWorkspace(appId);
        } else if (target == QLatin1String("all")) {
            qInfo() << "shell: Dock Assign to All Desktops is pending"
                       "(no sticky-window model, T-04/T-05):" << appId;
        } else if (target == QLatin1String("none")) {
            qInfo() << "shell: Dock Assign to None is pending"
                       "(no app-assignment model, T-04/T-05):" << appId;
        }
    } else if (action == QLatin1String("open_at_login")) {
        // T-24 owns login items; the entry point is wired.
        qInfo() << "shell: Dock Open at Login is pending (T-24)"
                << map.value(QStringLiteral("desktopId")).toString();
    } else if (action == QLatin1String("show_in_files")) {
        showDockAppInFiles(map.value(QStringLiteral("desktopId")).toString(),
                           map.value(QStringLiteral("appId")).toString());
    } else if (action == QLatin1String("toggle_magnification")) {
        // The divider menu toggle (T-10 section 13/19); no surface change.
        writeDockSetting(QStringLiteral("dock.magnification"),
                         m_dockConfig.magnification > 0 ? 0.0 : 0.5);
        applyDockSettings(false);
    } else if (action == QLatin1String("toggle_autohide")) {
        // Auto-hide flips the reserved zone, so the surface must reconfigure.
        writeDockSetting(QStringLiteral("dock.autohide"), !m_dockConfig.autohide);
        applyDockSettings(true);
    } else if (action == QLatin1String("set_position")) {
        // The divider's "Position on Screen" submenu (T-10 section 5). The
        // surface is re-anchored live; the QML layout mirrors vertically.
        const QString position = map.value(QStringLiteral("position")).toString();
        if (!position.isEmpty() && position != m_dockConfig.position) {
            writeDockSetting(QStringLiteral("dock.position"), position);
            applyDockSettings(true);
        }
    } else if (action == QLatin1String("open_dock_settings")) {
        // T-16 owns the Desktop & Dock pane; the entry point is wired.
        qInfo() << "shell: Dock Settings requested (T-16)";
    } else if (action == QLatin1String("open_downloads_folder")) {
        onDockDownloadsFolderRequested();
    } else if (action == QLatin1String("open_trash")) {
        openTrashInFiles();
    } else if (action == QLatin1String("empty_trash")) {
        // The QML confirmation already ran; perform the destructive operation
        // (T-10 section 13/16).
        const int removed = m_trash ? m_trash->empty() : -1;
        if (removed < 0)
            qWarning() << "shell: cannot empty the Trash:"
                       << (m_trash ? m_trash->lastError() : QStringLiteral("no monitor"));
        else
            qInfo() << "shell: emptied" << removed << "Trash entries";
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
    writeDockSetting(QStringLiteral("dock.pinned"), ids);
    rebuildDockEntries();
}

void ShellController::onDockPopoverChanged()
{
    // The popover fade/scale is QML-driven; the scene-graph hook commits
    // every frame it renders (FR-14).
    scheduleDockRender();
}

void ShellController::onDockRevealStateChanged()
{
    // The reveal/hide translation changed the committed image and the input
    // region (the hidden Dock keeps only its edge band; T-10 section 15).
    scheduleDockRender();
}

void ShellController::onTrashChanged()
{
    if (!m_dockItem)
        return;
    m_dockItem->setProperty("trashFull", m_trash->isFull());
    m_dockItem->setProperty("trashCount", m_trash->itemCount());
    m_dockItem->setProperty("trashAvailable", m_trash->isAvailable());
    scheduleDockRender();
}

void ShellController::onDownloadsChanged()
{
    if (!m_dockItem || !m_downloads)
        return;
    m_dockItem->setProperty("downloadsItems", m_downloads->items());
    m_dockItem->setProperty("downloadsCount", m_downloads->itemCount());
    m_dockItem->setProperty("downloadsBadge", m_downloads->newCount());
    scheduleDockRender();
}

void ShellController::onDockDownloadsViewed()
{
    if (m_downloads)
        m_downloads->markSeen();
}

void ShellController::onDockDownloadActivated(const QString &path)
{
    if (path.isEmpty())
        return;
    // Files is T-18; until it owns "open file", defer to the desktop's
    // registered handler through the standard xdg-open path.
    if (!QDesktopServices::openUrl(QUrl::fromLocalFile(path)))
        qWarning() << "shell: no handler for" << path;
    else
        qInfo() << "shell: Dock opened download" << path;
}

void ShellController::onDockDownloadsFolderRequested()
{
    if (!m_downloads)
        return;
    const QString folder = m_downloads->directory();
    QDir().mkpath(folder);
    if (!QDesktopServices::openUrl(QUrl::fromLocalFile(folder)))
        qWarning() << "shell: cannot open the Downloads folder:" << folder;
    else
        qInfo() << "shell: Dock opened the Downloads folder";
}

void ShellController::onDockAfterRendering()
{
    if (!m_dockFrameGate.frameRendered())
        return;
    if (!m_dockSceneGraphCommitLogged) {
        m_dockSceneGraphCommitLogged = true;
        qInfo() << "shell: Dock scene-graph commit path active (FR-14)";
    }
    scheduleDockRender();
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
    // `grabWindow()` renders the scene again, which emits `afterRendering`;
    // the gate suppresses that re-entrant schedule so the commit cannot loop
    // (FR-14).
    m_dockFrameGate.beginCommit();
    const QImage image = m_dockWindow->grabWindow();
    m_dockFrameGate.endCommit();

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
    // See `renderDock()`: the readback re-renders and must not re-schedule.
    m_menuFrameGate.beginCommit();
    const QImage image = m_window->grabWindow();
    m_menuFrameGate.endCommit();
    if (!m_protocol->commitImage(image.copy(0, 0, m_width, m_barHeight)))
        qWarning() << "shell: failed to commit the menu bar:" << m_protocol->lastError();
    if (popupShown) {
        const QRect popupRect(m_popupX, m_popupY, m_popupWidth, m_popupHeight);
        if (!m_protocol->commitPopupImage(image.copy(popupRect)))
            qWarning() << "shell: failed to commit the menu popup:" << m_protocol->lastError();
    }
}
