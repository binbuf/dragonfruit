// SPDX-License-Identifier: MIT
#include "shellcontroller.h"

#include <QChronoTimer>
#include <QDebug>
#include <QEventLoop>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QSocketNotifier>
#include <QVariantList>
#include <QVariantMap>
#include <QTimer>
#include <QtGlobal>
#include <QtMath>

#include <QCoreApplication>
#include <QDateTime>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QDBusVariant>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QKeyEvent>
#include <QPair>
#include <QWheelEvent>
#include <QProcess>
#include <QProcessEnvironment>
#include <QStandardPaths>
#include <QStyleHints>
#include <QUrl>

#include <cstdio>
#include <memory>

#include "appindexclient.h"
#include "apppicker.h"
#include "appsdrawer.h"
#include "chromedmabuf.h"
#include "chromeframeproducer.h"
#include "chromescale.h"
#include "chromestats.h"
#include "gpuchromeframeproducer.h"
#include "controlcenterpolicy.h"
#include "desktopentry.h"
#include "dockdrops.h"
#include "dockmodel.h"
#include "chromepointer.h"
#include "dockprojection.h"

#include "filestarget.h"
#include "focusstatus.h"
#include "launchfailure.h"
#include "lockinput.h"
#include "menubrokerpolicy.h"
#include "notificationclient.h"
#include "notificationmodel.h"
#include "shellprotocol.h"
#include "shellmenus.h"
#include "systemstatusclient.h"
#include "themebinding.h"

namespace {

// How long a launch may take to map its first window before the Dock treats
// it as failed (T-10 section 8.5). Interim until app-index owns activation.
constexpr qint64 kLaunchTimeoutMs = 8000;

// Startup readiness (first chrome paint). settingsd is a session dependency
// and answers `GetAll` on the session bus in well under a millisecond; the
// timeout is a failure bound for a wedged daemon, not a normal delay. The
// status host is optional, so its initial reads get a shorter bound and a
// late registration still fills the bar in afterwards.
constexpr int kInitialSettingsWaitMs = 1500;
constexpr int kInitialDisplayWaitMs = 1500;
constexpr int kInitialStatusWaitMs = 750;
// The three first-party status views that own a visible menu-bar slot.
constexpr int kInitialStatusWifi = 1 << 0;
constexpr int kInitialStatusAudio = 1 << 1;
constexpr int kInitialStatusBattery = 1 << 2;

// The wallpaper content provider (T-18.1b): the session-bus surface the shell
// reads the shipped and fetched defaults from, so the out-of-box background
// reaches the compositor even when settingsd is absent (ADR 0094/0114). The
// provider is never required: an absent service is a normal state.
const QString kWallpaperService = QStringLiteral("org.dragonfruit.Wallpaper1");
const QString kWallpaperPath = QStringLiteral("/org/dragonfruit/Wallpaper1");
const QString kWallpaperInterface = QStringLiteral("org.dragonfruit.Wallpaper1");

// Application identity (T-14.1a): the shell watches the well-known name so a
// service that starts after the shell (the session target starts both in
// parallel) still reaches the Dock without waiting for a user action.
const QString kAppIndexService = QStringLiteral("org.dragonfruit.AppIndex1");
const QString kAppIndexPath = QStringLiteral("/org/dragonfruit/AppIndex1");
const QString kAppIndexInterface = QStringLiteral("org.dragonfruit.AppIndex1");

// The Dock's offscreen buffer reserves a fixed popover headroom (above a
// bottom Dock / beside a vertical one) and side gutters so opening a
// menu/chooser/stack never resizes the render target on the open frame
// (T-14.7c). The Dock QML clamps the popover rectangle into this budget, so
// the values are a hard budget, not just a hint.
constexpr int kDockPopoverHeadroom = 320;
constexpr int kDockPopoverGutter = 320;

// Launch an app detached from the shell. The shell forces the offscreen QPA
// for its own chrome (`main.cpp`), so the app-launch environment scrubs it
// (see `appLaunchEnvironment`) or a Qt client would render offscreen and never
// map a window.
bool launchDetachedFromShell(const QStringList &argv, const QString &waylandDisplay,
                             qint64 *pid)
{
    if (argv.isEmpty())
        return false;
    QProcess process;
    process.setProcessEnvironment(appLaunchEnvironment(
        QProcessEnvironment::systemEnvironment(), waylandDisplay));
    process.setProgram(argv.first());
    process.setArguments(argv.mid(1));
    process.setWorkingDirectory(QDir::homePath());
    return process.startDetached(pid);
}

// The notification banner card (T-11.1a): a fixed card size and its gap below
// the menu bar. One banner shows at a time; the rest stay in the history.
constexpr int kBannerWidth = 380;
// The surface is tall enough for a banner with an action row; a banner with
// no actions draws only the top card and leaves the rest transparent.
constexpr int kBannerSurfaceHeight = 132;
constexpr int kBannerCardHeight = 96;
constexpr int kBannerCardActionsHeight = 132;
constexpr int kBannerTopGap = 8;

// The Notification Center (T204): a fixed panel surface anchored to the
// top-right corner below the menu bar. The QML panel fills the surface, so
// its input region is the whole thing; the surface is clamped to the output
// height on fractional scales like the Control Center.
constexpr int kNotificationCenterWidth = 360;
constexpr int kNotificationCenterHeight = 620;
constexpr int kNotificationCenterTopGap = 8;

// The Control Center panel (T-11.3a): a fixed panel surface anchored to the
// top-right corner below the menu bar. The panel fills the surface, so its
// input region is the whole thing.
// T-11.3b grows the panel to fit five tiles (Wi-Fi, Focus, Sound, Display,
// Dark Mode); T-13.5b adds the clipboard-history section below them; T-15.5b
// adds the Mission Control summary tile; T-15.9b adds the Menu Bar summary
// tile; T-15.12b adds the Printers tile; T-15.13b adds the Privacy tile;
// T-15.14b adds the compact Accessibility tile (the tiles' internal gap
// compacted once more to `xxs` so the fixed surface still holds every tile);
// T-15.15b adds the Network advanced (VPN) tile and raises the surface to the
// 1164 px the nested output leaves below the bar so the eighteenth tile fits.
// The surface is fixed and the panel fills it.
constexpr int kControlCenterWidth = 360;
constexpr int kControlCenterHeight = 1164;
constexpr int kControlCenterTopGap = 8;

// The OSD overlay (T-11.4a): a centered card. The surface is slightly larger
// than the card so its shadow is not clipped; the compositor centers an
// unanchored `overlay` surface on the output.
constexpr int kOsdSurfaceWidth = 220;
constexpr int kOsdSurfaceHeight = 220;

// The FileChooser picker (T-13.2b): a centered modal dialog rendered into a
// dedicated `file-chooser` overlay surface while a portal request waits.
constexpr int kChooserWidth = 640;
constexpr int kChooserHeight = 440;

// The ScreenCast source picker (T-13.4a): a centered dialog rendered into a
// dedicated `screencast` overlay surface while a portal request waits.
constexpr int kScreenCastWidth = 560;
constexpr int kScreenCastHeight = 460;

// The polkit authentication dialog (T-13.6): a centered dialog rendered into a
// dedicated `polkit` overlay surface while a privileged request waits.
constexpr int kPolkitWidth = 480;
constexpr int kPolkitHeight = 420;

// T231: the status-row priorities. Lower values sit closer to the menu side;
// every system extra is below the third-party band (1000+), so an overflow
// never promotes a StatusNotifier item above a system one.
namespace StatusPriority {
constexpr int Wifi = 10;
constexpr int Bluetooth = 20;
constexpr int Battery = 30;
constexpr int Volume = 40;
constexpr int Search = 50;
constexpr int Focus = 55;
constexpr int Notifications = 60;
constexpr int Clock = 70;
constexpr int ControlCenter = 80;
constexpr int MissionControl = 90;
constexpr int ThirdParty = 1000;
} // namespace StatusPriority

// T231: the one status-item shape every system and third-party extra fills.
// The QML registry (`shell/menubar/StatusItemRegistry.qml`) owns the order and
// overflow decisions; this builder only names the fields.
QVariantMap statusItem(const QString &id, const QString &icon, const QString &label,
                       const QString &accessibleName, bool available, qreal level = 0.8,
                       bool enabled = true, int priority = 0,
                       const QString &tooltip = QString(),
                       const QString &menu = QString(),
                       const QString &state = QStringLiteral("ok"),
                       const QString &kind = QStringLiteral("status"),
                       bool visible = true)
{
    QVariantMap item;
    item.insert(QStringLiteral("id"), id);
    item.insert(QStringLiteral("icon"), icon);
    item.insert(QStringLiteral("label"), label);
    item.insert(QStringLiteral("accessibleName"), accessibleName);
    item.insert(QStringLiteral("tooltip"),
                tooltip.isEmpty() ? accessibleName : tooltip);
    item.insert(QStringLiteral("menu"), menu);
    item.insert(QStringLiteral("priority"), priority);
    item.insert(QStringLiteral("visible"), visible);
    item.insert(QStringLiteral("available"), available && visible);
    item.insert(QStringLiteral("enabled"), enabled);
    item.insert(QStringLiteral("selected"), false);
    item.insert(QStringLiteral("state"), state);
    item.insert(QStringLiteral("kind"), kind);
    item.insert(QStringLiteral("thirdParty"), false);
    item.insert(QStringLiteral("essential"),
                kind == QLatin1String("clock")
                    || id == QLatin1String("control-center")
                    || id == QLatin1String("mission-control"));
    item.insert(QStringLiteral("level"), level);
    return item;
}

// Adapter views use "available"/"scanning" for live states; the registry's
// stable vocabulary is `ok`/`unavailable`/`error`/`attention`.
QString statusState(const QVariantMap &view)
{
    const QString state = view.value(QStringLiteral("state")).toString();
    if (state.isEmpty() || state == QLatin1String("available"))
        return view.value(QStringLiteral("visible")).toBool()
            ? QStringLiteral("ok") : QStringLiteral("unavailable");
    if (state == QLatin1String("error"))
        return QStringLiteral("error");
    if (state == QLatin1String("unavailable"))
        return QStringLiteral("unavailable");
    return QStringLiteral("ok");
}

// Minimal evdev -> Qt key map for menu interaction (the popup and bar only
// consume navigation keys). wl_keyboard.key is the raw evdev code.
Qt::Key qtKeyFromEvdev(quint32 key)
{
    switch (key) {
    case 1:
        return Qt::Key_Escape;
    case 14:
        return Qt::Key_Backspace;
    case 15:
        return Qt::Key_Tab;
    case 28:
        return Qt::Key_Return;
    case 57:
        return Qt::Key_Space;
    case 102:
        return Qt::Key_Home;
    case 103:
        return Qt::Key_Up;
    case 104:
        return Qt::Key_PageUp;
    case 109:
        return Qt::Key_PageDown;
    case 110:
        return Qt::Key_Insert;
    case 111:
        return Qt::Key_Delete;
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

// Translate the xkb `depressed` modifier mask carried by `wl_keyboard.modifiers`
// into Qt modifiers, so the chrome scenes see modifier chords (T-14.7t, the
// Dock's Ctrl+Shift+Arrow reorder). The mask bits are the Wayland
// WL_KEYBOARD_MODIFIER_MASK_* values: Shift 0x1, Control 0x4, Mod1/Alt 0x8,
// Mod4/Super 0x40.
Qt::KeyboardModifiers qtModifiersFromXkb(quint32 mask)
{
    Qt::KeyboardModifiers modifiers;
    if (mask & 0x1u)
        modifiers |= Qt::ShiftModifier;
    if (mask & 0x4u)
        modifiers |= Qt::ControlModifier;
    if (mask & 0x8u)
        modifiers |= Qt::AltModifier;
    if (mask & 0x40u)
        modifiers |= Qt::MetaModifier;
    return modifiers;
}

// One chrome key event: the Qt key plus the text a QML TextInput/TextField
// inserts. The shell is not an xkb client (it closes the keymap fd), so the
// fixed US/ASCII table in lockinput.cpp is the source of printable text. The
// old chrome path only mapped navigation keys and dropped letters, which is
// why the applications drawer's search field could not be typed into.
// (Qt key, text) for one evdev key. A QPair alias rather than a local named
// type keeps the translation extractor's context scan on the file's own name.
using ChromeKey = QPair<Qt::Key, QString>;

ChromeKey chromeKeyFromEvdev(quint32 key, Qt::KeyboardModifiers modifiers)
{
    const bool shift = modifiers.testFlag(Qt::ShiftModifier);
    const dragonfruit::LockKey mapped = dragonfruit::lockKeyFromEvdev(key, shift);
    switch (mapped.kind) {
    case dragonfruit::LockKeyKind::Character:
        return ChromeKey(Qt::Key(mapped.character.unicode()),
                         QString(mapped.character));
    case dragonfruit::LockKeyKind::Backspace:
        return ChromeKey(Qt::Key_Backspace, QStringLiteral("\b"));
    case dragonfruit::LockKeyKind::Return:
        return ChromeKey(Qt::Key_Return, QStringLiteral("\r"));
    case dragonfruit::LockKeyKind::Escape:
        return ChromeKey(Qt::Key_Escape, QString());
    case dragonfruit::LockKeyKind::Ignore:
        break;
    }
    return ChromeKey(qtKeyFromEvdev(key), QString());
}

// Send a complete key event to one chrome scene. Shortcut chords keep their
// key but carry no text, so Ctrl+C does not insert a control character.
void sendChromeKeyEvent(QQuickWindow *window, quint32 key, bool pressed,
                        quint32 modifiers)
{
    if (!window)
        return;
    const Qt::KeyboardModifiers mods = qtModifiersFromXkb(modifiers);
    ChromeKey mapped = chromeKeyFromEvdev(key, mods);
    if (!pressed
        || mods.testFlag(Qt::ControlModifier)
        || mods.testFlag(Qt::AltModifier)
        || mods.testFlag(Qt::MetaModifier)) {
        mapped.second.clear();
    }
    if (mapped.first == Qt::Key_unknown && mapped.second.isEmpty())
        return;
    QKeyEvent event(pressed ? QEvent::KeyPress : QEvent::KeyRelease, mapped.first, mods,
                    mapped.second);
    QCoreApplication::sendEvent(window, &event);
}

// The fixed system menu (the dragonfruit mark), always leftmost, lives in
// `shellmenus.cpp` so the T-12.6b "Quit to <previous desktop>" row is unit
// tested. The actions are session/system operations: System Settings opens the
// first-party Settings app (T-09); Lock Screen is wired (T-12.3a); App Store
// has no equivalent (disabled); About This System needs the General > About
// pane (T-15.10); Sleep/Restart/Shut Down/Log Out await the real-session work
// (T-12.6/T-16.4). Unlanded items dispatch as logged stubs.

// T-21.2c: push one applied material state into a chrome QML item. `prefix`
// selects the role-specific property group on a scene that hosts more than one
// role (the Dock/menu bar scenes expose `popupMaterial*` alongside their own
// `material*`). Returns true when a property changed so the caller only
// repaints the affected surface.
bool setMaterialProperties(QQuickItem *item, const AppliedMaterial &material,
                           const char *prefix = "material")
{
    if (!item)
        return false;
    const QByteArray base(prefix);
    const QByteArray blurName = base + "BlurActive";
    const QByteArray tierName = base + "Tier";
    const QByteArray fallbackName = base + "Fallback";
    const QVariant currentBlur = item->property(blurName.constData());
    if (!currentBlur.isValid())
        return false;
    const int tier = static_cast<int>(material.tier);
    const int fallback = static_cast<int>(material.fallback);
    bool changed = false;
    if (currentBlur.toBool() != material.blurActive) {
        item->setProperty(blurName.constData(), material.blurActive);
        changed = true;
    }
    if (item->property(tierName.constData()).toInt() != tier) {
        item->setProperty(tierName.constData(), tier);
        changed = true;
    }
    if (item->property(fallbackName.constData()).toInt() != fallback) {
        item->setProperty(fallbackName.constData(), fallback);
        changed = true;
    }
    return changed;
}

const char *materialRoleName(std::uint32_t role)
{
    switch (role) {
    case MaterialPolicy::Chrome:
        return "chrome";
    case MaterialPolicy::Dock:
        return "dock";
    case MaterialPolicy::Popup:
        return "popup";
    case MaterialPolicy::Drawer:
        return "drawer";
    default:
        return "unknown";
    }
}

} // namespace

ShellController::ShellController(QObject *parent)
    : QObject(parent)
{
    // The clipboard history exists before the protocol so its best-effort
    // persistence is read once at startup (T-13.5b).
    m_clipboard = new ClipboardHistory(this);
    connect(m_clipboard, &ClipboardHistory::changed, this,
            &ShellController::onClipboardHistoryChanged);
    loadClipboardHistory();
}

ShellController::~ShellController()
{
    // Jobs only queue results back here; finish before destroying the receiver.
    m_menuWorker.waitForDone();
    // The opt-in transport counters land on the clean-exit path too; SIGUSR1
    // remains the per-scenario trigger (T-21.1a).
    ChromeStats::instance().emitStats(QStringLiteral("exit"));
    // The producers own their QQuickWindows: a render-control window is a
    // child of the render control, and the software producer's destructor
    // deletes its own window. Delete the producers here (before the QML
    // engine is torn down) instead of deleting the windows and leaving the
    // producers with dangling window pointers. That double ownership was the
    // T192b full-rail teardown crash and made a failed start unrecoverable.
    const QList<ChromeFrameProducer *> producers = m_producers.values();
    m_producers.clear();
    qDeleteAll(producers);
}

QString ShellController::lastError() const
{
    return m_protocol ? m_protocol->lastError() : QStringLiteral("shell not started");
}

bool ShellController::start(const QString &socketName, const QString &tokenHex, int barHeight)
{
    m_barHeight = barHeight;
    m_waylandDisplay = socketName;
    // T-21.1d: the GPU rail is GPU-first (unless explicitly disabled) and
    // decided before the first chrome producer. A requested-but-unavailable
    // rail records a reason and keeps every surface on the T185 producer.
    m_gpuChromeRequested = GpuChromeFrameProducer::environmentRequested();
    const QString gpuSurfaces = qEnvironmentVariable("DRAGONFRUIT_CHROME_GPU_SURFACES");
    if (!gpuSurfaces.isEmpty()) {
        const QStringList labels = gpuSurfaces.split(QLatin1Char(','), Qt::SkipEmptyParts);
        for (const QString &label : labels)
            m_chromeDmabufAllowed.insert(label.trimmed());
    }
    if (!m_gpuChromeRequested) {
        ChromeStats::instance().setGpuPath(QStringLiteral("off"));
    }
    if (ChromeStats::instance().enabled()) {
        // The signal handler writes one byte to a self-pipe; the notifier
        // emits from the Qt thread so the stats line cannot race rendering.
        ChromeStats::installSignalHandler();
        const int fd = ChromeStats::signalFd();
        if (fd >= 0) {
            m_chromeStatsNotifier = new QSocketNotifier(fd, QSocketNotifier::Read, this);
            connect(m_chromeStatsNotifier, &QSocketNotifier::activated, this, []() {
                if (ChromeStats::takeSignal())
                    ChromeStats::instance().emitSignalStats();
            });
        }
    }
    m_protocol = new ShellProtocol(this);
    // T230: focus and window-state events are part of the initial scene
    // replay the compositor sends during `connectToCompositor` (the first
    // window is already focused before the shell binds). Connect them before
    // the roundtrip so the menu bar starts on the real focused app instead of
    // the empty desktop; the handlers tolerate the QML items not existing yet.
    connect(m_protocol, &ShellProtocol::focusedAppChanged, this,
            &ShellController::onFocusedAppChanged);
    connect(m_protocol, &ShellProtocol::dockStateChanged, this,
            &ShellController::onDockStateChanged);
    connect(m_protocol, &ShellProtocol::fatal, this, [this](const QString &message) {
        qWarning() << "shell:" << message;
        // A dead compositor connection is not recoverable in-process: every
        // later request would be marshalled onto a broken display and the
        // QSocketNotifier would spin on a readable-but-dead fd. Exit with a
        // failure so systemd (Restart=always) starts a fresh shell against
        // the compositor's reissued launch token. Disable the notifier first
        // so the fatal signal cannot be re-entered while the exit is queued.
        if (m_notifier)
            m_notifier->setEnabled(false);
        QTimer::singleShot(0, this, []() { QCoreApplication::exit(1); });
    });
    // The observed clipboard selection feeds the history store; the store
    // never writes the selection except when the user picks an entry (T-13.5b).
    connect(m_protocol, &ShellProtocol::clipboardObserved, this,
            &ShellController::onClipboardObserved);

    // Application identity (T-14.1a): `org.dragonfruit.AppIndex1` owns the
    // `.desktop` corpus, themed icons, and resolution. T-14.7 retired the
    // shell's local `.desktop` scan, so the service is the only source; a
    // session without it shows an empty Dock rather than a divergent corpus.
    // The shell and app-index start in parallel from the session target, so
    // the service may not own its bus name yet when this process starts. The
    // initial load and the restart/reappearance resync both go through
    // `connectAppIndex()` once the settings client exists.
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
    // persisted pinned set is empty. Mark the snapshot applied first so an
    // app-index resync that lands during startup does not seed before the
    // daemon's persisted `dock.pinned` has been read.
    connect(m_settingsClient, &SettingsClient::refreshed, this, [this]() {
        m_settingsSnapshotReceived = true;
        seedDefaultDockPins();
    });
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
    // T-18.1b: the provider's resolved defaults are a second wallpaper input;
    // subscribing here means the shipped default reaches the compositor even
    // when settingsd is absent, and a provider restart re-syncs it.
    connectWallpaperProvider();
    // `auto` follows the host scheme for the compositor too; ThemeBinding owns
    // the Theme side, this keeps the forwarded scheme live.
    if (QStyleHints *hints = QGuiApplication::styleHints()) {
        connect(hints, &QStyleHints::colorSchemeChanged, this,
                &ShellController::applyCompositorPolicy);
        // `auto` also changes the Control Center's effective dark state (the
        // Dark Mode tile), so refresh an open panel too.
        connect(hints, &QStyleHints::colorSchemeChanged, this, [this]() {
            if (m_controlCenterOpen)
                applyControlCenterData();
        });
    }
    m_dockConfig = dockConfigFromValues(m_settingsClient->values());
    if (!m_settingsClient->isAvailable()) {
        // No daemon: seed now so the first frame already shows the default
        // pins; when the daemon later appears, its `dock.pinned` wins.
        seedDefaultDockPins();
    }
    // Load the app-index corpus now if it is already up, and watch the
    // well-known name so a later start (or a restart) re-syncs the Dock,
    // picker, and drawer instead of leaving the startup race visible.
    connectAppIndex();

    if (!m_protocol->connectToCompositor(socketName))
        return false;
    if (!m_protocol->authenticate(tokenHex))
        return false;
    // Dispatch compositor events from the main loop. Created before the
    // startup readiness waits so they can observe the compositor's replies
    // (output replay/scale) instead of waiting for a notifier that is only
    // installed at the end of start().
    const int fd = m_protocol->displayFd();
    m_notifier = new QSocketNotifier(fd, QSocketNotifier::Read, this);
    connect(m_notifier, &QSocketNotifier::activated, this, [this]() { m_protocol->dispatch(); });
    // The first paint must already use the persisted display scale and the
    // persisted Dock/menu policy. The client's GetAll is asynchronous, so wait
    // for its snapshot here, after the compositor connection exists but before
    // any QML scene or chrome surface, instead of applying schema defaults and
    // replacing them a frame later. The notifier is installed first so the
    // policies the snapshot triggers can reach the compositor and its replies
    // are processed by this wait.
    waitForInitialSettings();
    // Seed the physical scale before the first scene is created. The persisted
    // display policy (below) is authoritative once settingsd has answered; the
    // live per-surface preferred scale still corrects any later change.
    m_chromeScale = m_protocol->outputScale();
    // T-21.2c: the role table may already be populated (initial output
    // events dispatch during authentication). Seed the policy before the
    // first scene so the menu bar and Dock start on their own role state.
    seedMaterialPolicy();

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
    // T195: one internal rollout switch for the end-to-end material
    // replacement. The shipped default is the replacement look; nested and
    // native fixtures can select either rail with
    // `DRAGONFRUIT_GLASS_ROLLOUT=legacy|replacement`. The compositor reads the
    // same variable through its material-preset resolver.
    if (QObject *glass = m_engine->singletonInstance<QObject *>(
                QStringLiteral("Dragonfruit"), QStringLiteral("Glass"))) {
        const QString requested =
            qEnvironmentVariable("DRAGONFRUIT_GLASS_ROLLOUT").trimmed().toLower();
        const QString shipped = glass->property("rollout").toString();
        const QString selected =
            (requested == QLatin1String("legacy")
             || requested == QLatin1String("replacement"))
            ? requested
            : (shipped == QLatin1String("legacy")
               ? QStringLiteral("legacy") : QStringLiteral("replacement"));
        glass->setProperty("rollout", selected);
        qInfo() << "shell: glass rollout:" << selected;
    }
    // T-08.2c: forward the initial motion/input policy now that the protocol
    // is authenticated (the compositor is the sole applier).
    applyCompositorPolicy();
    // T-09.3: forward the persisted wallpaper selection. The protocol retries
    // once the manager has announced its Spaces.
    applyWallpaperPolicy();
    // T-09.5: forward the persisted display selection. The protocol retries
    // once the manager has announced its outputs. When the snapshot was read,
    // seed the scale from it too so the very first configure is already at the
    // persisted scale even if the compositor's scale event is still in flight.
    applyDisplayPolicy();
    if (m_settingsSnapshotReceived && m_settingsClient
        && m_settingsClient->isAvailable()) {
        m_chromeScale = ChromeScale::sanitize(m_displaySettings.scale);
    }
    waitForDisplayPolicy();
    m_window = createChromeProducer()->window();
    m_window->setColor(Qt::transparent);
    ChromeFrameProducer *menuProducer = m_producers.value(m_window);
    if (menuProducer && menuProducer->renderControlActive()) {
        if (m_gpuChromeState == GpuChromeState::Active) {
            ChromeStats::instance().setTransportPath(
                QStringLiteral("render-control+dmabuf"));
            qInfo() << "shell: chrome transport: QQuickRenderControl + GPU"
                    << "(dmabuf chrome, shm fallback)";
        } else {
            ChromeStats::instance().setTransportPath(
                QStringLiteral("render-control+shm"));
            qInfo() << "shell: chrome transport: QQuickRenderControl + pooled shm";
        }
    } else {
        ChromeStats::instance().setTransportPath(QStringLiteral("offscreen-qpa+shm"));
        qWarning() << "shell: chrome transport: render control unavailable; using the"
                   << "grabWindow fallback";
    }
    m_chromeDmabufEnabled = (m_gpuChromeState == GpuChromeState::Active);

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
    updateMenuBarLuminance();
    // T200: the QML token owns the veil extent; the layer surface is sized to
    // it while the exclusive zone stays the bar strip. The surface itself
    // still commits only the strip, so the area below it is visual-only.
    m_menuBarVeilExtent = qMax(m_barHeight,
        qRound(m_item->property("veilExtent").toReal()));
    m_menuBarSurfaceHeight = m_menuBarVeilExtent;
    m_item->setProperty("reduceTransparency", m_compositorPolicy.reduceTransparency);
    m_item->setProperty("increaseContrast", m_compositorPolicy.increaseContrast);
    // T-21.2c: one role-keyed application policy for the persistent chrome.
    // The Dock and menu bar read the same decision; QML never re-derives it.
    const AppliedMaterial menuMaterial =
        m_materialPolicy.applied(MaterialPolicy::Chrome);
    m_item->setProperty("materialBlurActive", menuMaterial.blurActive);
    m_item->setProperty("materialTier", static_cast<int>(menuMaterial.tier));
    m_item->setProperty("materialFallback", static_cast<int>(menuMaterial.fallback));
    // The bar scene also hosts every menu dropdown/status popover; seed the
    // transient Popup role before the first commit (T-21.2c).
    setMaterialProperties(m_item, m_materialPolicy.applied(MaterialPolicy::Popup),
                          "popupMaterial");

    connect(m_item, SIGNAL(controlCenterRequested()), this, SLOT(onControlCenterRequested()));
    // T231: the clock opens the Date & Time panel in QML; Notifications is the
    // menu-bar entry point for the Notification Center, routed through the
    // registry's `statusItemActivated`.
    connect(m_item, SIGNAL(missionControlRequested()), this, SLOT(onMissionControlRequested()));
    connect(m_item, SIGNAL(statusItemActivated(QString)), this,
            SLOT(onStatusItemActivated(QString)));
    connect(m_item, SIGNAL(appMenuOpened(int)), this, SLOT(onAppMenuOpened(int)));
    connect(m_item, SIGNAL(appMenuClosed()), this, SLOT(onAppMenuClosed()));
    connect(m_item, SIGNAL(dropdownSurfaceRegionsChanged()),
            this, SLOT(schedulePopupGeometry()));
    connect(m_item, SIGNAL(appMenuTriggered(int,int,QVariant)), this,
            SLOT(onAppMenuTriggered(int,int,QVariant)));
    if (QObject *clock = m_item->findChild<QObject *>(QStringLiteral("clock")))
        connect(clock, SIGNAL(nowChanged()), this, SLOT(onClockTick()));
    // T-21.1c: the menu-bar producer renders once per requested frame and the
    // compositor's wl_surface.frame callback paces the next one.
    bindChromePacing(m_producers.value(m_window), QStringLiteral("menubar"),
                     [this]() { render(); });

    connect(m_protocol, &ShellProtocol::configured, this, &ShellController::onConfigured);
    // T-21.1b: chrome renders at the output's physical scale. The output
    // scale seeds the first frame; the per-surface preferred scale keeps it
    // live across Displays changes and hotplug.
    connect(m_protocol, &ShellProtocol::preferredScaleChanged, this,
            [this](const QString &, qreal scale) { onChromeScaleChanged(scale); });
    connect(m_protocol, &ShellProtocol::outputScaleChanged, this,
            &ShellController::onChromeScaleChanged);
    connect(m_protocol, &ShellProtocol::framePresented, this,
            &ShellController::onChromeFramePresented);
    connect(m_protocol, &ShellProtocol::bufferReleased, this,
            &ShellController::onChromeBufferReleased);
    connect(m_protocol, &ShellProtocol::dmabufReleased, this,
            &ShellController::onChromeDmabufReleased);
    connect(m_protocol, &ShellProtocol::commitDeferred, this,
            &ShellController::onChromeCommitDeferred);
    connect(m_protocol, &ShellProtocol::materialStateChanged, this,
            &ShellController::onMaterialStateChanged);
    connect(m_protocol, &ShellProtocol::materialRoleStateChanged, this,
            &ShellController::onMaterialRoleStateChanged);
    connect(m_protocol, &ShellProtocol::appAccelerator, this,
            &ShellController::onAppAccelerator);
    connect(m_protocol, &ShellProtocol::pointerMoved, this, &ShellController::onPointerMoved);
    connect(m_protocol, &ShellProtocol::pointerButton, this, &ShellController::onPointerButton);
    connect(m_protocol, &ShellProtocol::pointerLeft, this, &ShellController::onPointerLeft);
    connect(m_protocol, &ShellProtocol::keyboardFocused, this,
            &ShellController::onKeyboardFocused);
    connect(m_protocol, &ShellProtocol::keyEvent, this, &ShellController::onKeyEvent);
    connect(m_protocol, &ShellProtocol::lockKeyEvent, this, &ShellController::onLockKeyEvent);
    connect(m_protocol, &ShellProtocol::pointerAxis, this,
            &ShellController::onChromePointerAxis);
    connect(m_protocol, &ShellProtocol::dockConfigured, this,
            &ShellController::onDockConfigured);
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
    connect(m_protocol, &ShellProtocol::activationResult, this,
            &ShellController::onActivationResult);
    connect(m_protocol, &ShellProtocol::dockExternalDragEntered, this,
            &ShellController::onDockExternalDragEntered);
    connect(m_protocol, &ShellProtocol::dockExternalDragMoved, this,
            &ShellController::onDockExternalDragMoved);
    connect(m_protocol, &ShellProtocol::dockExternalDragLeft, this,
            &ShellController::onDockExternalDragLeft);
    connect(m_protocol, &ShellProtocol::dockExternalDropped, this,
            &ShellController::onDockExternalDropped);
    connect(m_protocol, &ShellProtocol::dockExternalDragPayload, this,
            &ShellController::onDockExternalDragPayload);

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
    connect(m_statusClient, &SystemStatusClient::bluetoothState, this,
            &ShellController::onBluetoothState);
    connect(m_statusClient, &SystemStatusClient::storageState, this,
            &ShellController::onStorageState);
    connect(m_statusClient, &SystemStatusClient::inputState, this,
            &ShellController::onInputState);
    connect(m_statusClient, &SystemStatusClient::updatesState, this,
            &ShellController::onUpdatesState);
    connect(m_statusClient, &SystemStatusClient::accountsState, this,
            &ShellController::onAccountsState);
    connect(m_statusClient, &SystemStatusClient::printersState, this,
            &ShellController::onPrintersState);
    connect(m_statusClient, &SystemStatusClient::privacyState, this,
            &ShellController::onPrivacyState);
    connect(m_statusClient, &SystemStatusClient::accessibilityState, this,
            &ShellController::onAccessibilityState);
    connect(m_statusClient, &SystemStatusClient::vpnState, this,
            &ShellController::onVpnState);
    connect(m_statusClient, &SystemStatusClient::joinReport, this,
            &ShellController::onStatusReport);
    connect(m_statusClient, &SystemStatusClient::writeReport, this,
            &ShellController::onStatusReport);
    // A bridge host that starts after the shell (the session target starts
    // services in parallel) must fill the bar without waiting for the user to
    // open a menu or restart the shell.
    connect(m_statusClient, &SystemStatusClient::availableChanged, this,
            [this](bool available) {
                if (available)
                    refreshStatusViews();
            });
    connect(m_item, SIGNAL(wifiJoinRequested(QString,QString)), this,
            SLOT(onWifiJoinRequested(QString,QString)));
    connect(m_item, SIGNAL(wifiToggleRequested(bool)), this,
            SLOT(onWifiToggleRequested(bool)));
    connect(m_item, SIGNAL(volumeSetRequested(double)), this,
            SLOT(onVolumeSetRequested(double)));
    connect(m_item, SIGNAL(muteToggleRequested()), this, SLOT(onMuteToggleRequested()));
    connect(m_item, SIGNAL(statusMenuRefreshRequested(QString)), this,
            SLOT(onStatusMenuRefreshRequested(QString)));
    connect(m_item, SIGNAL(statusMenuOpened()), this, SLOT(onStatusMenuOpened()));
    connect(m_item, SIGNAL(statusMenuClosed()), this, SLOT(onStatusMenuClosed()));
    // T202: the Bluetooth status menu shares the Control Center tile's one
    // action policy and bridge-host slots.
    connect(m_item, SIGNAL(bluetoothPowerToggleRequested(bool)), this,
            SLOT(onBluetoothToggleRequested(bool)));
    connect(m_item, SIGNAL(bluetoothDeviceToggled(QString,bool)), this,
            SLOT(onBluetoothDeviceToggled(QString,bool)));
    connect(m_item, SIGNAL(bluetoothSettingsRequested()), this,
            SLOT(onBluetoothSettingsRequested()));
    // T232: the Bluetooth status menu's discovery and nearby-device pair
    // actions, the Sound menu's output routing, and the Battery menu's power
    // mode. Each is one explicit bridge-host write.
    connect(m_item, SIGNAL(bluetoothDiscoveringToggled(bool)), this,
            SLOT(onBluetoothDiscoveringToggleRequested(bool)));
    connect(m_item, SIGNAL(bluetoothPairRequested(QString)), this,
            SLOT(onBluetoothPairRequested(QString)));
    connect(m_item, SIGNAL(sinkSelected(int)), this, SLOT(onSinkSelected(int)));
    connect(m_item, SIGNAL(powerProfileSelected(QString)), this,
            SLOT(onPowerProfileSelected(QString)));
    connect(m_item, SIGNAL(wifiSettingsRequested()), this,
            SLOT(onWifiSettingsRequested()));
    connect(m_item, SIGNAL(volumeSettingsRequested()), this,
            SLOT(onSoundSettingsRequested()));
    connect(m_item, SIGNAL(batterySettingsRequested()), this,
            SLOT(onBatterySettingsRequested()));
    applyStatusMenuData();
    // Hold the first menu-bar commit until the three visible first-party
    // status reads have landed. The host answers asynchronously; without this
    // gate the first committed frame has an empty Wi-Fi/volume/battery row.
    // The timer is the failure bound for a host that never answers; a late
    // registration still fills the row through `availableChanged`.
    if (m_statusClient->isAvailable()) {
        m_initialStatusPending =
            kInitialStatusWifi | kInitialStatusAudio | kInitialStatusBattery;
        m_initialStatusTimer = new QTimer(this);
        m_initialStatusTimer->setSingleShot(true);
        m_initialStatusTimer->setInterval(kInitialStatusWaitMs);
        connect(m_initialStatusTimer, &QTimer::timeout, this, [this]() {
            m_initialStatusPending = 0;
            commitInitialChrome();
        });
        m_initialStatusTimer->start();
    }
    // Seed every view once; the mock answers synchronously, the host is asked
    // asynchronously and answers with its signal.
    refreshStatusViews();

    // T-14.3: StatusNotifier tray items. app-index owns the watcher; the shell
    // reads its live item view and re-reads on a short timer (a tray app can
    // appear or leave at any time and there is no private-protocol tray
    // event). The whole path is a no-op when app-index is absent.
    connect(m_item, SIGNAL(trayMenuTriggered(QString,int)), this,
            SLOT(onTrayMenuTriggered(QString,int)));
    connect(m_item, SIGNAL(trayMenuClosed()), this, SLOT(onTrayMenuClosed()));
    refreshTrayItems();
    m_trayTimer = new QTimer(this);
    m_trayTimer->setInterval(2000);
    connect(m_trayTimer, &QTimer::timeout, this, &ShellController::refreshTrayItems);
    m_trayTimer->start();

    applyStatusItems();
    // The system menu (dragonfruit mark) is fixed for the session; the Log Out
    // item is personalized with the account name, the T-12.6b "Quit to
    // <previous desktop>" row appears only for a harness-armed session
    // (DRAGONFRUIT_DEV_RETURN set), and Recent Items follows the live recency
    // list (T230).
    m_systemUserName = qEnvironmentVariable("USER");
    if (m_systemUserName.isEmpty())
        m_systemUserName = QStringLiteral("user");
    m_devReturnDesktop = ShellMenus::devReturnDesktop();
    refreshSystemMenu();
    applyFocusedApp();
    // T-14.2b: the global application-menu toggle from Settings. It is a
    // standalone key (not part of DockConfig), so it gets its own live view of
    // the same client and re-publishes the bar when it flips.
    connect(m_settingsClient, &SettingsClient::changed, this,
            &ShellController::applyMenuBarPolicy);
    connect(m_settingsClient, &SettingsClient::refreshed, this,
            &ShellController::applyMenuBarPolicy);
    applyMenuBarPolicy();

    // The Dock is a second offscreen QML scene and its own `top` chrome
    // surface (T-10). It is created before the surfaces so the shell can read
    // the bar thickness and magnified band out of the QML tokens.
    m_dockWindow = createChromeProducer()->window();
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
    // P1/T-21.2c: the Dock's own role state, applied before its first commit
    // so the fallback never flashes the wrong plate.
    const AppliedMaterial dockMaterial =
        m_materialPolicy.applied(MaterialPolicy::Dock);
    m_dockItem->setProperty("materialBlurActive", dockMaterial.blurActive);
    m_dockItem->setProperty("materialTier", static_cast<int>(dockMaterial.tier));
    m_dockItem->setProperty("materialFallback",
                            static_cast<int>(dockMaterial.fallback));
    // The Dock scene hosts its popovers; seed the transient Popup role before
    // the first commit (T-21.2c).
    setMaterialProperties(m_dockItem, m_materialPolicy.applied(MaterialPolicy::Popup),
                          "popupMaterial");
    // Share the overlay budget with QML so placement fits the expanded target.
    m_dockItem->setProperty("popoverHeadroom", kDockPopoverHeadroom);
    m_dockItem->setProperty("popoverGutter", kDockPopoverGutter);
    connect(m_dockItem, SIGNAL(entryActivated(QVariant)), this,
            SLOT(onDockEntryActivated(QVariant)));
    connect(m_dockItem, SIGNAL(entryTileRect(QString,qreal,qreal,qreal,qreal)), this,
            SLOT(onDockEntryTileRect(QString,qreal,qreal,qreal,qreal)));
    connect(m_dockItem, SIGNAL(entryContextMenuRequested(QVariant,qreal,qreal)), this,
            SLOT(onDockEntryContextMenu(QVariant,qreal,qreal)));
    connect(m_dockItem, SIGNAL(dividerContextMenuRequested(qreal,qreal)), this,
            SLOT(onDockDividerContextMenu(qreal,qreal)));
    connect(m_dockItem, SIGNAL(menuActionRequested(QString,QVariant)), this,
            SLOT(onDockEntryMenuAction(QString,QVariant)));
    connect(m_dockItem, SIGNAL(appPickerRequested()), this,
            SLOT(onDockAppPickerRequested()));
    connect(m_dockItem, SIGNAL(appsDrawerRequested()), this,
            SLOT(onAppsDrawerRequested()));
    connect(m_dockItem, SIGNAL(appPinToggled(QString,bool)), this,
            SLOT(onDockAppPinToggled(QString,bool)));
    connect(m_dockItem, SIGNAL(windowActivated(QString)), this,
            SLOT(onDockWindowActivated(QString)));
    connect(m_dockItem, SIGNAL(windowCloseRequested(QString)), this,
            SLOT(onDockWindowCloseRequested(QString)));
    connect(m_dockItem, SIGNAL(windowMinimizeRequested(QString,bool)), this,
            SLOT(onDockWindowMinimizeRequested(QString,bool)));
    connect(m_dockItem, SIGNAL(pinnedOrderChanged(QVariant)), this,
            SLOT(onDockPinnedOrderChanged(QVariant)));
    connect(m_dockItem, SIGNAL(popoverChanged()), this, SLOT(onDockPopoverChanged()));
    // Column sizes can settle during polish, after this frame's crop was
    // chosen. Ordinary re-entrant sceneChanged signals are suppressed by the
    // pacer, so queue layout changes explicitly for the next presentation.
    connect(m_dockItem, SIGNAL(popoverRectChanged()), this, SLOT(scheduleDockRender()),
            Qt::QueuedConnection);
    connect(m_dockItem, SIGNAL(activePopoverSurfaceRegionsChanged()),
            this, SLOT(scheduleDockRender()), Qt::QueuedConnection);
    connect(m_dockItem, SIGNAL(inputRectsChanged()), this, SLOT(scheduleDockRender()));
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
    connect(m_dockItem, SIGNAL(folderOpenRequested(QString)), this,
            SLOT(onDockFolderOpenRequested(QString)));
    connect(m_dockItem, SIGNAL(folderViewed(QString)), this,
            SLOT(onDockFolderViewed(QString)));
    connect(m_dockItem, SIGNAL(folderPinRemoved(QString)), this,
            SLOT(onDockFolderPinRemoved(QString)));
    connect(m_dockItem, SIGNAL(dockSizePreview(qreal)), this,
            SLOT(onDockSizePreview(qreal)));
    connect(m_dockItem, SIGNAL(dockSizeChanged(qreal)), this,
            SLOT(onDockSizeChanged(qreal)));
    // T-21.1c: the Dock producer renders once per requested frame; the
    // compositor's wl_surface.frame callback paces the next one.
    bindChromePacing(m_producers.value(m_dockWindow), QStringLiteral("dock"),
                     [this]() { renderDock(); });
    m_dockTrackingTimer = new QChronoTimer(this);
    m_dockTrackingTimer->setTimerType(Qt::PreciseTimer);
    auto setDockTrackingInterval = [this]() {
        const auto refresh = m_protocol->outputRefreshMilliHz();
        m_dockTrackingTimer->setInterval(std::chrono::nanoseconds(1'000'000'000'000LL / refresh));
    };
    setDockTrackingInterval();
    connect(m_protocol, &ShellProtocol::outputRefreshChanged, this, setDockTrackingInterval);
    // T204: a detached output closes the per-output Center instead of leaving
    // a stale panel anchored to a display that no longer exists.
    connect(m_protocol, &ShellProtocol::outputRemoved, this, [this]() {
        if (m_notificationCenterOpen)
            hideNotificationCenter(QStringLiteral("output"));
    });
    connect(m_dockTrackingTimer, &QChronoTimer::timeout, this, &ShellController::scheduleDockRender);
    connect(m_dockItem, SIGNAL(pointerTrackingActiveChanged()),
            this, SLOT(onDockPointerTrackingChanged()));
    m_dockItem->setProperty("pointerTrackingFromShell", true);
    // The Trash entry's state comes from files-core/GVfs (section 16); a
    // deletion by any application updates the full/count state, and the shell
    // and Files share the one Trash store (T-10.6a).
    m_trash = new TrashBridge(this);
    connect(m_trash, &TrashBridge::changed, this, &ShellController::onTrashChanged);
    connect(m_trash, &TrashBridge::emptyFinished, this,
            &ShellController::onTrashEmptyFinished);
    m_trash->start();
    m_dockItem->setProperty("trashFull", m_trash->isFull());
    m_dockItem->setProperty("trashCount", m_trash->itemCount());
    m_dockItem->setProperty("trashAvailable", m_trash->isAvailable());
    // Folder stacks (T-14.7k): the Downloads default member plus every
    // `dock.pinnedFolders` path, all listed through files-core and watched for
    // the popover listings and new-items badges. The Downloads stack stays the
    // default member; user pins are additional stacks.
    m_folderStacks = new FolderStacks(this);
    connect(m_folderStacks, &FolderStacks::changed, this, &ShellController::onDownloadsChanged);
    refreshFolderStackData();
    // Apply `dock.*` before the surfaces exist (no reconfigure yet) so the
    // baseline bar thickness and magnified band reflect the saved size.
    applyDockSettings(false);
    // The surface is the floating plate + interior magnify band + edge gap;
    // the reserved zone is the plate + edge gap (ADR 0089). Both come from
    // Dock.qml so the plate math has one source (T-14.7a).
    m_dockThickness = qCeil(m_dockItem->property("surfaceThickness").toReal());
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
    m_overviewWindow = createChromeProducer()->window();
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
    bindChromePacing(m_producers.value(m_overviewWindow), QStringLiteral("overview"),
                     [this]() { renderOverview(); });
    connect(m_protocol, &ShellProtocol::overviewConfigured, this,
            &ShellController::onOverviewConfigured);
    connect(m_protocol, &ShellProtocol::overviewChanged, this,
            &ShellController::onOverviewChanged);
    connect(m_protocol, &ShellProtocol::overviewProgress, this,
            &ShellController::onOverviewProgress);
    connect(m_protocol, &ShellProtocol::overviewDataChanged, this,
            &ShellController::onOverviewDataChanged);
    connect(m_protocol, &ShellProtocol::activeWallpaperChanged, this,
            &ShellController::updateMenuBarLuminance);
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
    m_switcherWindow = createChromeProducer()->window();
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
    bindChromePacing(m_producers.value(m_switcherWindow), QStringLiteral("switcher"),
                     [this]() { renderSwitcher(); });
    connect(m_protocol, &ShellProtocol::switcherConfigured, this,
            &ShellController::onSwitcherConfigured);
    connect(m_protocol, &ShellProtocol::appSwitcherChanged, this,
            &ShellController::onAppSwitcherChanged);

    // Applications drawer overlay (T-19.2): a full-output launcher scene. The
    // list is a pure app-index projection (`buildAppsDrawerList`); the scene is
    // mapped only while the drawer is open.
    m_appsDrawerWindow = createChromeProducer()->window();
    m_appsDrawerWindow->setColor(Qt::transparent);
    QQmlComponent appsDrawerComponent(m_engine);
    appsDrawerComponent.loadFromModule(QStringLiteral("Dragonfruit.AppsDrawer"),
                                       QStringLiteral("AppsDrawer"));
    if (appsDrawerComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: AppsDrawer QML error: %s\n",
                qPrintable(appsDrawerComponent.errorString()));
        return false;
    }
    QObject *appsDrawerObject = appsDrawerComponent.create();
    m_appsDrawerItem = qobject_cast<QQuickItem *>(appsDrawerObject);
    if (!m_appsDrawerItem) {
        fprintf(stderr, "dragonfruit-shell: AppsDrawer QML did not produce an item\n");
        return false;
    }
    m_appsDrawerItem->setParentItem(m_appsDrawerWindow->contentItem());
    // T-21.2c: the drawer is the transient Drawer role; seed it before the
    // first commit so it never flashes the wrong plate.
    setMaterialProperties(m_appsDrawerItem,
                          m_materialPolicy.applied(MaterialPolicy::Drawer));
    connect(appsDrawerObject, SIGNAL(appLaunched(QString,qreal,qreal,qreal,qreal)), this,
            SLOT(onAppsDrawerActivated(QString,qreal,qreal,qreal,qreal)));
    connect(appsDrawerObject, SIGNAL(dismissRequested()), this,
            SLOT(onAppsDrawerDismissed()));
    bindChromePacing(m_producers.value(m_appsDrawerWindow),
                     QStringLiteral("apps-drawer"),
                     [this]() { renderAppsDrawer(); });
    connect(m_protocol, &ShellProtocol::appsDrawerConfigured, this,
            &ShellController::onAppsDrawerConfigured);
    connect(m_protocol, &ShellProtocol::appsDrawerPointerMoved, this,
            &ShellController::onAppsDrawerPointerMoved);
    connect(m_protocol, &ShellProtocol::appsDrawerPointerButton, this,
            &ShellController::onAppsDrawerPointerButton);
    connect(m_protocol, &ShellProtocol::appsDrawerPointerLeft, this,
            &ShellController::onAppsDrawerPointerLeft);
    connect(m_protocol, &ShellProtocol::appsDrawerKeyboardFocused, this,
            &ShellController::onAppsDrawerKeyboardFocused);
    connect(m_protocol, &ShellProtocol::appsDrawerKeyEvent, this,
            &ShellController::onAppsDrawerKeyEvent);
    refreshAppsDrawer(false);

    // Notification banners and history (T-11.1a): the model decodes the
    // service's `Banners()`/`History()` views; the client is the live
    // session-bus client, or the fixture under `DF_NOTIFY_FIXTURE`
    // (headless/capture only). A banner maps the top-right `notification`
    // overlay; an empty queue unmaps it.
    m_notificationModel = new NotificationModel(this);
    // T204: the Center's own state validates active rows and owns the keyboard
    // selection. The service remains the single owner of history, expiry,
    // Focus/DND, banner replacement and urgency.
    m_notificationCenter = new NotificationCenter(this);
    connect(m_notificationCenter, &NotificationCenter::changed, this, [this]() {
        if (m_notificationCenterOpen)
            applyNotificationCenterData();
    });
    connect(m_notificationCenter, &NotificationCenter::closed, this,
            &ShellController::onNotificationCenterStateClosed);
    connect(m_notificationCenter, &NotificationCenter::invokeRequested, this,
            [this](quint32 id, const QString &key) {
                if (m_notificationClient)
                    m_notificationClient->invoke(id, key);
            });
    connect(m_notificationCenter, &NotificationCenter::dismissRequested, this,
            [this](quint32 id) {
                if (m_notificationClient)
                    m_notificationClient->dismiss(id);
            });
    if (qEnvironmentVariableIsSet("DF_NOTIFY_FIXTURE")) {
        m_notificationClient = new MockNotificationClient(this);
        // Capture/demo seam (T-11.2b): start the fixture in a Focus/DND mode
        // so the bar reflection can be captured deterministically. Never set
        // in a normal session.
        const QString focusFixture = qEnvironmentVariable("DF_FOCUS_FIXTURE");
        if (!focusFixture.isEmpty())
            m_notificationClient->setFocusMode(focusFixture);
    } else {
        m_notificationClient = new DbusNotificationClient(this);
    }
    // A service that was already absent at startup does not emit a change;
    // seed the Center's unavailable state from the client once.
    m_notificationCenter->setAvailable(m_notificationClient->isAvailable());
    connect(m_notificationModel, &NotificationModel::changed, this, [this]() {
        if (m_notificationModel->hasBanner())
            showCurrentBanner();
        else
            hideCurrentBanner();
    });
    connect(m_notificationClient, &NotificationClient::bannersChanged, this,
            &ShellController::onNotificationBanners);
    connect(m_notificationClient, &NotificationClient::historyChanged, this,
            &ShellController::onNotificationHistory);
    connect(m_notificationClient, &NotificationClient::availableChanged, this,
            &ShellController::onNotificationAvailable);
    connect(m_notificationClient, &NotificationClient::focusPolicyChanged, this,
            &ShellController::onNotificationFocusPolicy);
    // T229a: the schedule applies the notification service's Focus mode at a
    // window boundary. It never enforces the lock or writes the service's
    // queue; it is a small consumer of the settingsd keys.
    m_focusSchedule = new FocusSchedule(m_settingsClient, m_notificationClient, this);
    m_focusSchedule->start();

    m_bannerWindow = createChromeProducer()->window();
    m_bannerWindow->setColor(Qt::transparent);
    QQmlComponent bannerComponent(m_engine);
    bannerComponent.loadFromModule(QStringLiteral("Dragonfruit.Notifications"),
                                   QStringLiteral("NotificationBanner"));
    if (bannerComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: NotificationBanner QML error: %s\n",
                qPrintable(bannerComponent.errorString()));
        return false;
    }
    QObject *bannerObject = bannerComponent.create();
    m_bannerItem = qobject_cast<QQuickItem *>(bannerObject);
    if (!m_bannerItem) {
        fprintf(stderr, "dragonfruit-shell: NotificationBanner QML did not produce an item\n");
        return false;
    }
    m_bannerItem->setParentItem(m_bannerWindow->contentItem());
    // T-21.2c: the banner is the transient Popup role.
    setMaterialProperties(m_bannerItem,
                          m_materialPolicy.applied(MaterialPolicy::Popup));
    connect(bannerObject, SIGNAL(actionInvoked(QString)), this,
            SLOT(onBannerActionInvoked(QString)));
    connect(bannerObject, SIGNAL(activated()), this, SLOT(onBannerActivated()));
    // T205: the card region follows the action-row height, so re-render when
    // the QML layout re-declares it.
    connect(bannerObject, SIGNAL(materialRegionsChanged()), this,
            SLOT(scheduleBannerRender()));
    bindChromePacing(m_producers.value(m_bannerWindow), QStringLiteral("banner"),
                     [this]() { renderBanner(); });
    connect(m_protocol, &ShellProtocol::bannerConfigured, this,
            &ShellController::onBannerConfigured);
    connect(m_protocol, &ShellProtocol::bannerPointerMoved, this,
            &ShellController::onBannerPointerMoved);
    connect(m_protocol, &ShellProtocol::bannerPointerButton, this,
            &ShellController::onBannerPointerButton);
    connect(m_protocol, &ShellProtocol::bannerPointerLeft, this,
            &ShellController::onBannerPointerLeft);
    m_notificationClient->refresh();
    // The Focus item exists now that the notification model is wired; the
    // first apply above ran before it.
    applyStatusItems();

    // Notification Center (T204): a top-right offscreen scene rendered into
    // its own `notification-center` overlay surface. It is a pure view of the
    // normalized service history; actions and dismissal go back through the
    // same `NotificationClient` the banner uses.
    m_notificationCenterWindow = createChromeProducer()->window();
    m_notificationCenterWindow->setColor(Qt::transparent);
    QQmlComponent notificationCenterComponent(m_engine);
    notificationCenterComponent.loadFromModule(QStringLiteral("Dragonfruit.Notifications"),
                                               QStringLiteral("NotificationCenter"));
    if (notificationCenterComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: NotificationCenter QML error: %s\n",
                qPrintable(notificationCenterComponent.errorString()));
        return false;
    }
    QObject *notificationCenterObject = notificationCenterComponent.create();
    m_notificationCenterItem = qobject_cast<QQuickItem *>(notificationCenterObject);
    if (!m_notificationCenterItem) {
        fprintf(stderr, "dragonfruit-shell: NotificationCenter QML did not produce an item\n");
        return false;
    }
    m_notificationCenterItem->setParentItem(m_notificationCenterWindow->contentItem());
    setMaterialProperties(m_notificationCenterItem,
                          m_materialPolicy.applied(MaterialPolicy::Popup));
    connect(notificationCenterObject, SIGNAL(closed()), this,
            SLOT(onNotificationCenterDismissRequested()));
    connect(notificationCenterObject, SIGNAL(moveRequested(int)), this,
            SLOT(onNotificationCenterMoveRequested(int)));
    connect(notificationCenterObject, SIGNAL(selectRequested(int)), this,
            SLOT(onNotificationCenterSelectRequested(int)));
    connect(notificationCenterObject, SIGNAL(activateRequested()), this,
            SLOT(onNotificationCenterActivateRequested()));
    connect(notificationCenterObject, SIGNAL(dismissSelectedRequested()), this,
            SLOT(onNotificationCenterDismissSelectedRequested()));
    connect(notificationCenterObject, SIGNAL(actionInvoked(int,QString)), this,
            SLOT(onNotificationCenterActionRequested(int,QString)));
    connect(notificationCenterObject, SIGNAL(dismissEntryRequested(int)), this,
            SLOT(onNotificationCenterEntryDismissRequested(int)));
    // T205: the stack/lower regions follow the QML layout and the banner
    // inset; re-render when the binding settles so the compositor never keeps
    // the pre-layout region list.
    connect(notificationCenterObject, SIGNAL(materialRegionsChanged()), this,
            SLOT(scheduleNotificationCenterRender()));
    bindChromePacing(m_producers.value(m_notificationCenterWindow),
                     QStringLiteral("notification-center"),
                     [this]() { renderNotificationCenter(); });
    connect(m_protocol, &ShellProtocol::notificationCenterConfigured, this,
            &ShellController::onNotificationCenterConfigured);
    connect(m_protocol, &ShellProtocol::notificationCenterPointerMoved, this,
            &ShellController::onNotificationCenterPointerMoved);
    connect(m_protocol, &ShellProtocol::notificationCenterPointerButton, this,
            &ShellController::onNotificationCenterPointerButton);
    connect(m_protocol, &ShellProtocol::notificationCenterPointerLeft, this,
            &ShellController::onNotificationCenterPointerLeft);
    connect(m_protocol, &ShellProtocol::notificationCenterKeyboardFocused, this,
            &ShellController::onNotificationCenterKeyboardFocused);
    connect(m_protocol, &ShellProtocol::notificationCenterKeyEvent, this,
            &ShellController::onNotificationCenterKeyEvent);
    connect(m_protocol, &ShellProtocol::notificationCenterSurfaceClosed, this,
            &ShellController::onNotificationCenterSurfaceClosed);
    m_notificationCenterWidth = kNotificationCenterWidth;
    m_notificationCenterHeight = kNotificationCenterHeight;
    applyNotificationCenterData();
    // Capture/demo seam (T205): open the Center deterministically for the
    // material captures without injecting the Control-Shift-N chord. Never
    // set in a normal session.
    if (qEnvironmentVariable("DF_NOTIFY_CENTER_FIXTURE") == QLatin1String("open")) {
        QTimer::singleShot(0, this, [this]() { showNotificationCenter(); });
    }

    // Control Center panel (T-11.3a): a top-right offscreen scene rendered
    // into its own overlay surface. It is a pure view; the taps/gestures it
    // raises are forwarded to the bridge host or written to settingsd here.
    m_controlCenterWindow = createChromeProducer()->window();
    m_controlCenterWindow->setColor(Qt::transparent);
    QQmlComponent controlCenterComponent(m_engine);
    controlCenterComponent.loadFromModule(QStringLiteral("Dragonfruit.ControlCenter"),
                                          QStringLiteral("ControlCenter"));
    if (controlCenterComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: ControlCenter QML error: %s\n",
                qPrintable(controlCenterComponent.errorString()));
        return false;
    }
    QObject *controlCenterObject = controlCenterComponent.create();
    m_controlCenterItem = qobject_cast<QQuickItem *>(controlCenterObject);
    if (!m_controlCenterItem) {
        fprintf(stderr, "dragonfruit-shell: ControlCenter QML did not produce an item\n");
        return false;
    }
    m_controlCenterItem->setParentItem(m_controlCenterWindow->contentItem());
    // T-21.2c: Control Center is the transient Popup role.
    setMaterialProperties(m_controlCenterItem,
                          m_materialPolicy.applied(MaterialPolicy::Popup));
    connect(controlCenterObject, SIGNAL(closed()), this, SLOT(onControlCenterClosed()));
    // T203: the module/list regions follow the QML layout and scroll state.
    // Re-render when the binding settles so the compositor never keeps the
    // pre-layout region list.
    connect(controlCenterObject, SIGNAL(materialRegionsChanged()), this,
            SLOT(scheduleControlCenterRender()));
    connect(controlCenterObject, SIGNAL(volumeSetRequested(double)), this,
            SLOT(onVolumeSetRequested(double)));
    connect(controlCenterObject, SIGNAL(muteToggleRequested()), this,
            SLOT(onMuteToggleRequested()));
    connect(controlCenterObject, SIGNAL(brightnessSetRequested(double)), this,
            SLOT(onBrightnessSetRequested(double)));
    connect(controlCenterObject, SIGNAL(wifiToggleRequested(bool)), this,
            SLOT(onWifiToggleRequested(bool)));
    connect(controlCenterObject, SIGNAL(wifiSettingsRequested()), this,
            SLOT(onWifiSettingsRequested()));
    connect(controlCenterObject, SIGNAL(bluetoothToggleRequested(bool)), this,
            SLOT(onBluetoothToggleRequested(bool)));
    connect(controlCenterObject, SIGNAL(bluetoothDeviceToggled(QString,bool)), this,
            SLOT(onBluetoothDeviceToggled(QString,bool)));
    connect(controlCenterObject, SIGNAL(bluetoothSettingsRequested()), this,
            SLOT(onBluetoothSettingsRequested()));
    connect(controlCenterObject, SIGNAL(storageMountRequested(QString)), this,
            SLOT(onStorageMountRequested(QString)));
    connect(controlCenterObject, SIGNAL(storageUnmountRequested(QString)), this,
            SLOT(onStorageUnmountRequested(QString)));
    connect(controlCenterObject, SIGNAL(storageEjectRequested(QString)), this,
            SLOT(onStorageEjectRequested(QString)));
    connect(controlCenterObject, SIGNAL(storageSettingsRequested()), this,
            SLOT(onStorageSettingsRequested()));
    connect(controlCenterObject, SIGNAL(soundSettingsRequested()), this,
            SLOT(onSoundSettingsRequested()));
    connect(controlCenterObject, SIGNAL(keyboardSettingsRequested()), this,
            SLOT(onKeyboardSettingsRequested()));
    connect(controlCenterObject, SIGNAL(missionControlSettingsRequested()), this,
            SLOT(onMissionControlSettingsRequested()));
    connect(controlCenterObject, SIGNAL(batterySettingsRequested()), this,
            SLOT(onBatterySettingsRequested()));
    connect(controlCenterObject, SIGNAL(lockScreenSettingsRequested()), this,
            SLOT(onLockScreenSettingsRequested()));
    connect(controlCenterObject, SIGNAL(menuBarSettingsRequested()), this,
            SLOT(onMenuBarSettingsRequested()));
    connect(controlCenterObject, SIGNAL(updatesCheckRequested()), this,
            SLOT(onUpdatesCheckRequested()));
    connect(controlCenterObject, SIGNAL(updatesInstallRequested()), this,
            SLOT(onUpdatesInstallRequested()));
    connect(controlCenterObject, SIGNAL(updatesRebootRequested()), this,
            SLOT(onUpdatesRebootRequested()));
    connect(controlCenterObject, SIGNAL(updatesSettingsRequested()), this,
            SLOT(onUpdatesSettingsRequested()));
    connect(controlCenterObject, SIGNAL(usersSettingsRequested()), this,
            SLOT(onUsersSettingsRequested()));
    connect(controlCenterObject, SIGNAL(printersSettingsRequested()), this,
            SLOT(onPrintersSettingsRequested()));
    connect(controlCenterObject, SIGNAL(privacySettingsRequested()), this,
            SLOT(onPrivacySettingsRequested()));
    connect(controlCenterObject, SIGNAL(accessibilitySettingsRequested()), this,
            SLOT(onAccessibilitySettingsRequested()));
    connect(controlCenterObject, SIGNAL(vpnSettingsRequested()), this,
            SLOT(onVpnSettingsRequested()));
    connect(controlCenterObject, SIGNAL(quickSettingsSettingsRequested()), this,
            SLOT(onQuickSettingsSettingsRequested()));
    connect(controlCenterObject, SIGNAL(focusToggleRequested(bool)), this,
            SLOT(onFocusToggleRequested(bool)));
    connect(controlCenterObject, SIGNAL(focusSettingsRequested()), this,
            SLOT(onFocusSettingsRequested()));
    connect(controlCenterObject, SIGNAL(darkModeToggleRequested(bool)), this,
            SLOT(onDarkModeToggleRequested(bool)));
    connect(controlCenterObject, SIGNAL(appearanceSettingsRequested()), this,
            SLOT(onAppearanceSettingsRequested()));
    // Clipboard history (T-13.5b): the panel's list is a pure view of the
    // store; copying/pinning/clearing come back here.
    connect(controlCenterObject, SIGNAL(clipboardCopyRequested(int)), this,
            SLOT(onClipboardCopyRequested(int)));
    connect(controlCenterObject, SIGNAL(clipboardPinToggled(int, bool)), this,
            SLOT(onClipboardPinToggled(int, bool)));
    connect(controlCenterObject, SIGNAL(clipboardClearRequested()), this,
            SLOT(onClipboardClearRequested()));
    bindChromePacing(m_producers.value(m_controlCenterWindow),
                     QStringLiteral("control-center"),
                     [this]() { renderControlCenter(); });
    connect(m_protocol, &ShellProtocol::controlCenterConfigured, this,
            &ShellController::onControlCenterConfigured);
    connect(m_protocol, &ShellProtocol::controlCenterPointerMoved, this,
            &ShellController::onControlCenterPointerMoved);
    connect(m_protocol, &ShellProtocol::controlCenterPointerButton, this,
            &ShellController::onControlCenterPointerButton);
    connect(m_protocol, &ShellProtocol::controlCenterPointerLeft, this,
            &ShellController::onControlCenterPointerLeft);
    connect(m_protocol, &ShellProtocol::controlCenterKeyboardFocused, this,
            &ShellController::onControlCenterKeyboardFocused);
    // Keep the panel in sync with settingsd while it is open (brightness) and
    // with the bridge host (Wi-Fi/volume) through `applyStatusMenuData`.
    connect(m_settingsClient, &SettingsClient::changed, this, [this]() {
        if (m_controlCenterOpen)
            applyControlCenterData();
        if (m_lockActive)
            applyLockData();
    });
    connect(m_settingsClient, &SettingsClient::refreshed, this, [this]() {
        if (m_controlCenterOpen)
            applyControlCenterData();
        if (m_lockActive)
            applyLockData();
    });

    // OSD overlay (T-11.4a): a centered offscreen scene rendered into its own
    // `overlay` surface while a volume/brightness change is presented. It is a
    // pure view; the controller drives it from the pure OSD model.
    m_osdWindow = createChromeProducer()->window();
    m_osdWindow->setColor(Qt::transparent);
    QQmlComponent osdComponent(m_engine);
    osdComponent.loadFromModule(QStringLiteral("Dragonfruit.Osd"), QStringLiteral("Osd"));
    if (osdComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: Osd QML error: %s\n",
                qPrintable(osdComponent.errorString()));
        return false;
    }
    QObject *osdObject = osdComponent.create();
    m_osdItem = qobject_cast<QQuickItem *>(osdObject);
    if (!m_osdItem) {
        fprintf(stderr, "dragonfruit-shell: Osd QML did not produce an item\n");
        return false;
    }
    m_osdItem->setParentItem(m_osdWindow->contentItem());
    // T-21.2c: the OSD is the transient Popup role.
    setMaterialProperties(m_osdItem,
                          m_materialPolicy.applied(MaterialPolicy::Popup));
    bindChromePacing(m_producers.value(m_osdWindow), QStringLiteral("osd"),
                     [this]() { renderOsd(); });
    connect(m_protocol, &ShellProtocol::osdConfigured, this,
            &ShellController::onOsdConfigured);
    // T-11.4b: an AT-SPI press action (or the view's own Escape handler) clears
    // the transient alert through the same hide path as the auto-dismiss tick.
    connect(osdObject, SIGNAL(dismissed()), this, SLOT(onOsdDismissed()));
    m_osdTimer = new QTimer(this);
    m_osdTimer->setInterval(16);
    connect(m_osdTimer, &QTimer::timeout, this, &ShellController::onOsdTick);

    // FileChooser picker (T-13.2b): a centered offscreen scene rendered into
    // its own `file-chooser` overlay surface while a portal request waits for
    // a presenter. The bridge is the D-Bus half; the QML is a pure view.
    m_chooser = new ChooserBridge(this);
    m_chooser->connectService();
    connect(m_chooser, &ChooserBridge::started, this, &ShellController::onChooserStarted);
    connect(m_chooser, &ChooserBridge::finished, this, &ShellController::onChooserFinished);
    connect(m_chooser, &ChooserBridge::changed, this, &ShellController::onChooserChanged);

    m_chooserWindow = createChromeProducer()->window();
    m_chooserWindow->setColor(Qt::transparent);
    QQmlComponent chooserComponent(m_engine);
    chooserComponent.loadFromModule(QStringLiteral("Dragonfruit.Screenshot"),
                                    QStringLiteral("FileChooser"));
    if (chooserComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: FileChooser QML error: %s\n",
                qPrintable(chooserComponent.errorString()));
        return false;
    }
    QObject *chooserObject = chooserComponent.create();
    m_chooserItem = qobject_cast<QQuickItem *>(chooserObject);
    if (!m_chooserItem) {
        fprintf(stderr, "dragonfruit-shell: FileChooser QML did not produce an item\n");
        return false;
    }
    m_chooserItem->setParentItem(m_chooserWindow->contentItem());
    connect(chooserObject, SIGNAL(selectionChanged(int)), this,
            SLOT(onChooserSelectionChanged(int)));
    connect(chooserObject, SIGNAL(entryActivated(int)), this,
            SLOT(onChooserEntryActivated(int)));
    connect(chooserObject, SIGNAL(browseRequested(QString)), this,
            SLOT(onChooserBrowseRequested(QString)));
    connect(chooserObject, SIGNAL(upRequested()), this, SLOT(onChooserUpRequested()));
    connect(chooserObject, SIGNAL(nameEdited(QString)), this,
            SLOT(onChooserNameEdited(QString)));
    connect(chooserObject, SIGNAL(accepted()), this, SLOT(onChooserAccepted()));
    connect(chooserObject, SIGNAL(cancelled()), this, SLOT(onChooserCancelled()));
    bindChromePacing(m_producers.value(m_chooserWindow), QStringLiteral("chooser"),
                     [this]() { renderChooser(); });
    connect(m_protocol, &ShellProtocol::chooserConfigured, this,
            &ShellController::onChooserConfigured);
    connect(m_protocol, &ShellProtocol::chooserPointerMoved, this,
            &ShellController::onChooserPointerMoved);
    connect(m_protocol, &ShellProtocol::chooserPointerButton, this,
            &ShellController::onChooserPointerButton);
    connect(m_protocol, &ShellProtocol::chooserPointerLeft, this,
            &ShellController::onChooserPointerLeft);
    connect(m_protocol, &ShellProtocol::chooserKeyboardFocused, this,
            &ShellController::onChooserKeyboardFocused);
    connect(m_protocol, &ShellProtocol::chooserKeyEvent, this,
            &ShellController::onChooserKeyEvent);

    // Screenshot selection overlay (T-13.3a): a full-output offscreen scene
    // rendered into its own `screenshot` overlay surface while a capture is in
    // progress. The bridge is the D-Bus presenter half; the QML is a pure
    // view. The same overlay serves the portal's `Screenshot` requests and the
    // desktop's own Cmd+Shift+3/4 shortcut.
    m_screenshot = new ScreenshotBridge(this);
    m_screenshot->connectService();
    connect(m_screenshot, &ScreenshotBridge::started, this, &ShellController::onScreenshotStarted);
    connect(m_screenshot, &ScreenshotBridge::finished, this, &ShellController::onScreenshotFinished);
    connect(m_screenshot, &ScreenshotBridge::changed, this, &ShellController::onScreenshotChanged);
    connect(m_screenshot, &ScreenshotBridge::captureRequested, this,
            &ShellController::onScreenshotCaptureRequested);
    m_screenshotWriter = new ScreenshotWriter(this);

    m_screenshotWindow = createChromeProducer()->window();
    m_screenshotWindow->setColor(Qt::transparent);
    QQmlComponent screenshotComponent(m_engine);
    screenshotComponent.loadFromModule(QStringLiteral("Dragonfruit.Screenshot"),
                                       QStringLiteral("SelectionOverlay"));
    if (screenshotComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: SelectionOverlay QML error: %s\n",
                qPrintable(screenshotComponent.errorString()));
        return false;
    }
    QObject *screenshotObject = screenshotComponent.create();
    m_screenshotItem = qobject_cast<QQuickItem *>(screenshotObject);
    if (!m_screenshotItem) {
        fprintf(stderr, "dragonfruit-shell: SelectionOverlay QML did not produce an item\n");
        return false;
    }
    m_screenshotItem->setParentItem(m_screenshotWindow->contentItem());
    connect(screenshotObject, SIGNAL(accepted(int, int, int, int)), m_screenshot,
            SLOT(accept(int, int, int, int)));
    connect(screenshotObject, SIGNAL(cancelled()), m_screenshot, SLOT(cancel()));
    bindChromePacing(m_producers.value(m_screenshotWindow),
                     QStringLiteral("screenshot"),
                     [this]() { renderScreenshot(); });
    connect(m_protocol, &ShellProtocol::screenshotConfigured, this,
            &ShellController::onScreenshotConfigured);
    connect(m_protocol, &ShellProtocol::screenshotPointerMoved, this,
            &ShellController::onScreenshotPointerMoved);
    connect(m_protocol, &ShellProtocol::screenshotPointerButton, this,
            &ShellController::onScreenshotPointerButton);
    connect(m_protocol, &ShellProtocol::screenshotPointerLeft, this,
            &ShellController::onScreenshotPointerLeft);
    connect(m_protocol, &ShellProtocol::screenshotKeyboardFocused, this,
            &ShellController::onScreenshotKeyboardFocused);
    connect(m_protocol, &ShellProtocol::screenshotKeyEvent, this,
            &ShellController::onScreenshotKeyEvent);
    connect(m_protocol, &ShellProtocol::screenshotSaved, this,
            &ShellController::onScreenshotSaved);
    connect(m_protocol, &ShellProtocol::screenshotFailed, this,
            &ShellController::onScreenshotFailed);

    // ScreenCast source picker (T-13.4a): a centered offscreen scene rendered
    // into its own `screencast` overlay surface while a portal request waits.
    // The bridge owns the request and the D-Bus presenter call; the source
    // list comes from the compositor projection when the request starts.
    m_screencast = new ScreenCastBridge(this);
    m_screencast->connectService();
    connect(m_screencast, &ScreenCastBridge::started, this,
            &ShellController::onScreenCastStarted);
    connect(m_screencast, &ScreenCastBridge::finished, this,
            &ShellController::onScreenCastFinished);
    connect(m_screencast, &ScreenCastBridge::changed, this,
            &ShellController::onScreenCastChanged);

    m_screencastWindow = createChromeProducer()->window();
    m_screencastWindow->setColor(Qt::transparent);
    QQmlComponent screencastComponent(m_engine);
    screencastComponent.loadFromModule(QStringLiteral("Dragonfruit.Screenshot"),
                                       QStringLiteral("ScreenCastPicker"));
    if (screencastComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: ScreenCastPicker QML error: %s\n",
                qPrintable(screencastComponent.errorString()));
        return false;
    }
    QObject *screencastObject = screencastComponent.create();
    m_screencastItem = qobject_cast<QQuickItem *>(screencastObject);
    if (!m_screencastItem) {
        fprintf(stderr, "dragonfruit-shell: ScreenCastPicker QML did not produce an item\n");
        return false;
    }
    m_screencastItem->setParentItem(m_screencastWindow->contentItem());
    connect(screencastObject, SIGNAL(sourceToggled(QString)), this,
            SLOT(onScreenCastSourceToggled(QString)));
    connect(screencastObject, SIGNAL(accepted()), this, SLOT(onScreenCastAccepted()));
    connect(screencastObject, SIGNAL(cancelled()), this, SLOT(onScreenCastCancelled()));
    bindChromePacing(m_producers.value(m_screencastWindow),
                     QStringLiteral("screencast"),
                     [this]() { renderScreenCast(); });
    connect(m_protocol, &ShellProtocol::screencastConfigured, this,
            &ShellController::onScreenCastConfigured);
    connect(m_protocol, &ShellProtocol::screencastPointerMoved, this,
            &ShellController::onScreenCastPointerMoved);
    connect(m_protocol, &ShellProtocol::screencastPointerButton, this,
            &ShellController::onScreenCastPointerButton);
    connect(m_protocol, &ShellProtocol::screencastPointerLeft, this,
            &ShellController::onScreenCastPointerLeft);
    connect(m_protocol, &ShellProtocol::screencastKeyboardFocused, this,
            &ShellController::onScreenCastKeyboardFocused);
    connect(m_protocol, &ShellProtocol::screencastKeyEvent, this,
            &ShellController::onScreenCastKeyEvent);

    // polkit authentication dialog (T-13.6): a centered offscreen scene
    // rendered into its own `polkit` overlay surface while a privileged request
    // waits. The agent owns the D-Bus agent object and the host helper
    // conversation; the dialog is a pure view driven by the agent's properties.
    m_polkit = new PolkitAgent(this);
    connect(m_polkit, &PolkitAgent::started, this, &ShellController::onPolkitStarted);
    connect(m_polkit, &PolkitAgent::finished, this, &ShellController::onPolkitFinished);
    connect(m_polkit, &PolkitAgent::changed, this, &ShellController::onPolkitChanged);

    m_polkitWindow = createChromeProducer()->window();
    m_polkitWindow->setColor(Qt::transparent);
    QQmlComponent polkitComponent(m_engine);
    polkitComponent.loadFromModule(QStringLiteral("Dragonfruit.Screenshot"),
                                   QStringLiteral("PolkitDialog"));
    if (polkitComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: PolkitDialog QML error: %s\n",
                qPrintable(polkitComponent.errorString()));
        return false;
    }
    QObject *polkitObject = polkitComponent.create();
    m_polkitItem = qobject_cast<QQuickItem *>(polkitObject);
    if (!m_polkitItem) {
        fprintf(stderr, "dragonfruit-shell: PolkitDialog QML did not produce an item\n");
        return false;
    }
    m_polkitItem->setParentItem(m_polkitWindow->contentItem());
    connect(polkitObject, SIGNAL(submitted()), this, SLOT(onPolkitSubmitted()));
    connect(polkitObject, SIGNAL(cancelled()), this, SLOT(onPolkitCancelled()));
    bindChromePacing(m_producers.value(m_polkitWindow), QStringLiteral("polkit"),
                     [this]() { renderPolkit(); });
    connect(m_protocol, &ShellProtocol::polkitConfigured, this,
            &ShellController::onPolkitConfigured);
    connect(m_protocol, &ShellProtocol::polkitPointerMoved, this,
            &ShellController::onPolkitPointerMoved);
    connect(m_protocol, &ShellProtocol::polkitPointerButton, this,
            &ShellController::onPolkitPointerButton);
    connect(m_protocol, &ShellProtocol::polkitPointerLeft, this,
            &ShellController::onPolkitPointerLeft);
    connect(m_protocol, &ShellProtocol::polkitKeyboardFocused, this,
            &ShellController::onPolkitKeyboardFocused);
    connect(m_protocol, &ShellProtocol::polkitKeyEvent, this,
            &ShellController::onPolkitKeyEvent);
    // Register with the host polkit authority. A missing authority or an
    // existing session agent is a normal degraded state: the dialog simply
    // never opens on its own.
    m_polkit->start();

    // Session lock (T-12.3a): a full-output offscreen scene rendered into one
    // `ext-session-lock-v1` surface per output. Authentication and input
    // capture are T-12.3b/T-12.3c; this is the fail-secure presentation.
    m_lockWindow = createChromeProducer()->window();
    m_lockWindow->setColor(Qt::transparent);
    QQmlComponent lockComponent(m_engine);
    lockComponent.loadFromModule(QStringLiteral("Dragonfruit.Lock"), QStringLiteral("LockScreen"));
    if (lockComponent.isError()) {
        fprintf(stderr, "dragonfruit-shell: LockScreen QML error: %s\n",
                qPrintable(lockComponent.errorString()));
        return false;
    }
    QObject *lockObject = lockComponent.create();
    m_lockItem = qobject_cast<QQuickItem *>(lockObject);
    if (!m_lockItem) {
        fprintf(stderr, "dragonfruit-shell: LockScreen QML did not produce an item\n");
        return false;
    }
    m_lockItem->setParentItem(m_lockWindow->contentItem());
    connect(lockObject, SIGNAL(powerActionRequested(QString)), this,
            SLOT(onLockPowerAction(QString)));
    bindChromePacing(m_producers.value(m_lockWindow), QStringLiteral("lock"),
                     [this]() { repaintLockSurfaces(); });
    connect(m_protocol, &ShellProtocol::sessionLocked, this, &ShellController::onSessionLocked);
    connect(m_protocol, &ShellProtocol::sessionFinished, this, &ShellController::onSessionFinished);
    connect(m_protocol, &ShellProtocol::lockSurfaceConfigured, this,
            &ShellController::onLockSurfaceConfigured);
    // Refresh the clock while locked; stopped on unlock so the lock scene is
    // frame-quiet once settled (FR-2).
    m_lockTimer = new QTimer(this);
    m_lockTimer->setInterval(1000);
    connect(m_lockTimer, &QTimer::timeout, this, [this]() {
        if (!m_lockActive)
            return;
        applyLockData();
        const auto ids = m_lockSurfaceSizes.keys();
        for (quintptr id : ids)
            renderLockSurface(id);
    });
    // Authentication (T-12.3b): the shell's half of the PAM helper boundary.
    // `DF_PAM_SERVICE` selects the PAM service; empty keeps the helper's
    // default. The helper binary is resolved by LockAuthenticator.
    m_lockAuth = new LockAuthenticator(this);
    m_lockAuth->setService(QString::fromLocal8Bit(qgetenv("DF_PAM_SERVICE")));
    connect(m_lockAuth, &LockAuthenticator::succeeded, this,
            &ShellController::onLockAuthSucceeded);
    connect(m_lockAuth, &LockAuthenticator::failed, this, &ShellController::onLockAuthFailed);

    // T200: the surface is tall enough for the compositor veil (95 pt) while
    // the exclusive zone remains the 28 pt bar strip. The shell commits only
    // the strip buffer, so the extra area is visual-only and cannot take
    // input or change the usable area.
    if (!m_protocol->createMenuBarSurface(m_menuBarSurfaceHeight, barHeight))
        return false;
    // The dropdown rides a separate `overlay` chrome surface so transient
    // menus composite above fullscreen windows while the bar keeps its own
    // `top` surface and reserved zone (T-09).
    if (!m_protocol->createPopupSurface())
        return false;
    // The Dock reserves the resting plate plus its edge margin; with
    // auto-hide on it reserves nothing (section 2, ADR 0089).
    const int dockExclusive = m_dockItem->property("autoHide").toBool()
            ? 0
            : qRound(m_dockItem->property("reservedThickness").toReal());
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
    // The notification banner is a top-right `overlay` surface, unmapped until
    // a banner is active (T-11.1a).
    if (!m_protocol->createBannerSurface(kBannerWidth, kBannerSurfaceHeight,
                                         barHeight + kBannerTopGap))
        return false;
    // The Notification Center is a top-right `overlay` surface, unmapped until
    // the clock, keyboard action or configured hot corner opens it (T204).
    if (!m_protocol->createNotificationCenterSurface(
            kNotificationCenterWidth, kNotificationCenterHeight,
            barHeight + kNotificationCenterTopGap))
        return false;
    // The Control Center panel is a top-right `overlay` surface, unmapped until
    // the menu-bar item (or its shortcut) opens it (T-11.3a).
    if (!m_protocol->createControlCenterSurface(kControlCenterWidth, kControlCenterHeight,
                                                barHeight + kControlCenterTopGap))
        return false;
    // The OSD is a centered `overlay` surface, unmapped until a volume or
    // brightness change presents it (T-11.4a).
    if (!m_protocol->createOsdSurface(kOsdSurfaceWidth, kOsdSurfaceHeight))
        return false;
    // The FileChooser picker is a centered `overlay` surface, unmapped until a
    // portal request arrives (T-13.2b).
    if (!m_protocol->createChooserSurface(kChooserWidth, kChooserHeight))
        return false;
    // The screenshot selection overlay is a full-output `overlay` surface,
    // unmapped until a capture starts (T-13.3a).
    if (!m_protocol->createScreenshotSurface())
        return false;
    // The ScreenCast source picker is a centered `overlay` surface, unmapped
    // until a portal request arrives (T-13.4a).
    if (!m_protocol->createScreenCastSurface(kScreenCastWidth, kScreenCastHeight))
        return false;
    // The polkit authentication dialog is a centered `overlay` surface,
    // unmapped until a privileged request is presented (T-13.6).
    if (!m_protocol->createPolkitSurface(kPolkitWidth, kPolkitHeight))
        return false;
    // The Applications drawer is a full-output `overlay` surface, unmapped
    // until the launcher opens (T-19.2).
    if (!m_protocol->createAppsDrawerSurface())
        return false;

    // Capture/demo seam (T-19.2): seed a synthetic corpus and open the drawer
    // once the chrome is up so the live visual check can inspect the grid
    // without app-index. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_APPS_DRAWER_FIXTURE")) {
        startAppsDrawerFixture();
        QTimer::singleShot(1500, this, [this]() { showAppsDrawer(); });
    }

    // Capture/demo seam (T-20.3): open the menu-bar system dropdown once the
    // chrome is up, so the live visual check can capture the `menubar-popup`
    // overlay surface without relying on a synthetic click on the status row
    // (the row is right-anchored and its x shifts with the clock width). The
    // value is the top-level menu index (0 = the fixed system menu). Never set
    // in a normal session.
    if (qEnvironmentVariableIsSet("DF_MENUBAR_MENU_FIXTURE")) {
        bool ok = false;
        const int index = qEnvironmentVariable("DF_MENUBAR_MENU_FIXTURE").toInt(&ok);
        // T230: a capture that must focus an app first can push the open later.
        int delay = qEnvironmentVariableIntValue("DF_MENUBAR_MENU_FIXTURE_DELAY");
        if (delay <= 0)
            delay = 1500;
        QTimer::singleShot(delay, this, [this, ok, index]() {
            if (m_item)
                QMetaObject::invokeMethod(m_item, "openMenu",
                                          Q_ARG(QVariant, ok ? index : 0));
        });
    }
    // T230 capture seam: activate one open menu row through the normal
    // `appMenuTriggered` path, so a DBusMenu click and a Hide Others action can
    // be exercised without a synthetic pointer landing on a pixel. The value
    // is `menuIndex,itemIndex`. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_MENUBAR_ACTIVATE_FIXTURE")) {
        const QStringList parts =
            qEnvironmentVariable("DF_MENUBAR_ACTIVATE_FIXTURE").split(QLatin1Char(','));
        bool menuOk = false;
        bool itemOk = false;
        const int menuIndex = parts.value(0).toInt(&menuOk);
        const int itemIndex = parts.value(1).toInt(&itemOk);
        int delay = qEnvironmentVariableIntValue("DF_MENUBAR_ACTIVATE_DELAY");
        if (delay <= 0)
            delay = 2200;
        QTimer::singleShot(delay, this, [this, menuOk, itemOk, menuIndex, itemIndex]() {
            if (m_item && menuOk && itemOk)
                QMetaObject::invokeMethod(m_item, "activateMenuRow",
                                          Q_ARG(QVariant, menuIndex),
                                          Q_ARG(QVariant, itemIndex));
        });
    }

    // T230 capture seam: focus a named app through the compositor's normal
    // `activate_app` request so the live check exercises a real focus change
    // (the first window is already focused before the shell connects, so a
    // click on it produces no event). Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_FOCUS_APP_FIXTURE")) {
        const QStringList appIds =
            qEnvironmentVariable("DF_FOCUS_APP_FIXTURE").split(QLatin1Char(','));
        int delay = qEnvironmentVariableIntValue("DF_FOCUS_APP_FIXTURE_DELAY");
        if (delay <= 0)
            delay = 2000;
        // The app window can map several seconds after the shell; retry the
        // activation until the compositor reports it found, then move to the
        // next id. Two ids (an existing window, then the target) force a real
        // focus change even when the target was already focused before the
        // shell connected.
        auto *timer = new QTimer(this);
        timer->setInterval(1000);
        auto index = std::make_shared<int>(0);
        auto attempts = std::make_shared<int>(0);
        QObject::connect(timer, &QTimer::timeout, this,
                         [this, appIds, index, attempts, timer]() {
                             if (*index >= appIds.size()) {
                                 timer->stop();
                                 return;
                             }
                             if (++*attempts > 20) {
                                 *attempts = 0;
                                 ++*index;
                                 return;
                             }
                             if (m_protocol)
                                 m_protocol->activateApp(appIds.at(*index));
                         });
        QObject::connect(
            m_protocol, &ShellProtocol::activationResult, timer,
            [appIds, index, attempts, timer](const QString &appId, bool found) {
                if (!found || *index >= appIds.size()
                    || appId != appIds.at(*index))
                    return;
                ++*index;
                *attempts = 0;
                if (*index >= appIds.size())
                    timer->stop();
            });
        QTimer::singleShot(delay, timer, [timer]() { timer->start(); });
    }
    // Capture/demo seam (T202): open one status menu by id so the menu-family
    // capture does not depend on the right-anchored row's pixel positions.
    if (qEnvironmentVariableIsSet("DF_STATUS_MENU_FIXTURE")) {
        const QString id = qEnvironmentVariable("DF_STATUS_MENU_FIXTURE");
        QTimer::singleShot(1500, this, [this, id]() {
            if (m_item)
                QMetaObject::invokeMethod(m_item, "openStatusMenu",
                                          Q_ARG(QVariant, id));
        });
    }

    // Capture/demo seam (T-21.4a): enter Mission Control once the chrome is up
    // so the native visual sweep can capture the live-surface grid without a
    // synthetic keyboard gesture. The compositor owns the overview; this only
    // forwards the same request the menu-bar item sends. Never set in a normal
    // session.
    if (qEnvironmentVariableIsSet("DF_MISSION_CONTROL_FIXTURE")) {
        QTimer::singleShot(1500, this, &ShellController::onMissionControlRequested);
    }

    // Capture/demo seam (T-21.2c): open the Control Center once the chrome is
    // up so the forced-tier overlay capture can show its material without a
    // synthetic chord. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_CONTROL_CENTER_FIXTURE")) {
        const QString fixture = qEnvironmentVariable("DF_CONTROL_CENTER_FIXTURE");
        auto applyExpansion = [this, fixture]() {
            if (!m_controlCenterItem)
                return;
            // T203 capture seam: `wifi`/`bluetooth` open one expanded list and
            // `expanded` opens both. The QML owns the visible state; the shell
            // only seeds it once the panel exists.
            m_controlCenterItem->setProperty(
                "wifiExpanded",
                fixture == QLatin1String("wifi") || fixture == QLatin1String("expanded"));
            m_controlCenterItem->setProperty(
                "bluetoothExpanded",
                fixture == QLatin1String("bluetooth")
                    || fixture == QLatin1String("expanded"));
        };
        auto open = [this, applyExpansion]() {
            if (!m_controlCenterOpen)
                showControlCenter();
            applyExpansion();
            // The QML layout settles on the next event-loop turn; re-render so
            // the declared material regions include the expanded list.
            QTimer::singleShot(120, this, [this]() {
                if (m_controlCenterOpen)
                    scheduleControlCenterRender();
            });
        };
        QTimer::singleShot(1500, this, open);
        // `hold` (or `DF_CONTROL_CENTER_HOLD`) keeps the panel open while a
        // capture script cycles the forced material tiers; a focus change may
        // dismiss it between captures. Never set in a normal session.
        if (fixture == QLatin1String("hold")
                || qEnvironmentVariableIsSet("DF_CONTROL_CENTER_HOLD")) {
            auto *timer = new QTimer(this);
            timer->setInterval(500);
            connect(timer, &QTimer::timeout, this, open);
            QTimer::singleShot(1500, timer, [timer]() { timer->start(); });
        }
    }

    // Capture/demo seam (T-11.1b): raise the Dock's launch-failure
    // notification once the chrome is up, so the live check can exercise the
    // real failure path deterministically. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_FAIL_FIXTURE")) {
        QTimer::singleShot(1500, this, [this]() {
            failDockLaunch(QStringLiteral("org.dragonfruit.Files.desktop"),
                           QStringLiteral("The app did not start."));
        });
    }

    // Capture/demo seam (T-11.4a): present the OSD once the chrome is up so the
    // live visual check can exercise it with no hardware key wiring. Values are
    // `volume` (default) or `brightness`. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_OSD_FIXTURE")) {
        const QString fixture = qEnvironmentVariable("DF_OSD_FIXTURE");
        auto present = [this, fixture]() {
            if (fixture == QLatin1String("brightness"))
                showOsd(OsdModel::Kind::Brightness, 0.7, false);
            else
                showOsd(OsdModel::Kind::Volume, 0.6, false);
        };
        QTimer::singleShot(1500, this, present);
        // `hold` re-presents the brief overlay while a capture script cycles
        // the forced material tiers; never set in a normal session.
        if (fixture == QLatin1String("hold")) {
            auto *timer = new QTimer(this);
            timer->setInterval(1000);
            connect(timer, &QTimer::timeout, this, present);
            QTimer::singleShot(1500, timer, [timer]() { timer->start(); });
        }
    }

    // Capture/demo seam (T-13.2b): present the FileChooser picker once the
    // chrome is up so the live visual check can capture it with no portal
    // caller. `DF_CHOOSER_FIXTURE` is the folder to browse (empty/`1` =
    // home); the request uses an open panel. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_CHOOSER_FIXTURE")) {
        QTimer::singleShot(1500, this, &ShellController::startChooserFixture);
    }

    // Capture/demo seam (T-13.3a): present the screenshot selection overlay
    // once the chrome is up so the live visual check can capture it with no
    // portal caller or hardware shortcut. `DF_SCREENSHOT_FIXTURE` is
    // `fullscreen`/`region`/`window` (empty/`1` = region). Never set in a
    // normal session.
    if (qEnvironmentVariableIsSet("DF_SCREENSHOT_FIXTURE")) {
        QTimer::singleShot(1500, this, &ShellController::startScreenshotFixture);
    }

    // Capture/demo seam (T-21.1c close-out): ask the compositor to render the
    // output straight to a PNG after a delay, without the selection overlay.
    // The value is `<delay-ms>:<path>`; the capture uses the primary output's
    // logical geometry. This is how the transport close-out records a live
    // frame while the Dock is moving on a host that throttles the nested
    // window. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_SCREENSHOT_CAPTURE_FIXTURE")) {
        const QString spec = qEnvironmentVariable("DF_SCREENSHOT_CAPTURE_FIXTURE");
        const qsizetype separator = spec.indexOf(QLatin1Char(':'));
        const int delay = (separator > 0 ? spec.left(separator) : QStringLiteral("3000"))
                              .toInt();
        const QString path = separator > 0 ? spec.mid(separator + 1) : spec;
        const int interval = qEnvironmentVariableIntValue(
            "DF_SCREENSHOT_CAPTURE_INTERVAL_MS");
        auto capture = [this, path]() {
            if (!m_protocol || path.isEmpty())
                return;
            const QRect rect = m_protocol->primaryOutputGeometry();
            if (rect.isEmpty())
                return;
            // The fixture is the same capture path as the user shortcut: the
            // output geometry is logical, the compositor crops physical output
            // pixels. Convert, or a 1.5x/2x output captures only the top-left
            // logical-sized corner (T-21.1b).
            const QRect physical =
                ChromeScale::bufferRect(rect, m_protocol->outputScale());
            m_protocol->captureScreenshot(path, physical.x(), physical.y(),
                                          physical.width(), physical.height(),
                                          QStringLiteral("fullscreen"));
        };
        if (interval > 0) {
            auto *timer = new QTimer(this);
            timer->setInterval(interval);
            connect(timer, &QTimer::timeout, this, capture);
            QTimer::singleShot(qMax(0, delay), this, [timer]() { timer->start(); });
        } else {
            QTimer::singleShot(qMax(0, delay), this, capture);
        }
    }

    // Capture/demo seam (T-13.4a): present the ScreenCast source picker once
    // the chrome is up so the live visual check can capture it with no portal
    // caller. `DF_SCREENCAST_FIXTURE` is `monitors`/`windows`/`1` (empty/`1` =
    // both); never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_SCREENCAST_FIXTURE")) {
        QTimer::singleShot(1500, this, &ShellController::startScreenCastFixture);
    }

    // Capture/demo seam (T-13.6): present the polkit authentication dialog
    // once the chrome is up so the live visual check can capture it with no
    // real privileged request. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_POLKIT_FIXTURE")) {
        QTimer::singleShot(1500, this, &ShellController::startPolkitFixture);
    }

    // Capture/demo seam (T-14.7e): open the Add Application picker once the
    // chrome is up so the live visual check can capture it. The value is the
    // filter text (empty/`1` = unfiltered); the corpus is the real app-index
    // one, so this is the production path minus a synthetic divider click.
    // Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_APP_PICKER_FIXTURE")) {
        QTimer::singleShot(1500, this, [this]() {
            if (m_dockItem)
                QMetaObject::invokeMethod(m_dockItem, "openAppPicker");
            const QString query = qEnvironmentVariable("DF_APP_PICKER_FIXTURE");
            if (m_dockItem && !query.isEmpty() && query != QLatin1String("1")) {
                QMetaObject::invokeMethod(m_dockItem, "setAppPickerQuery",
                                          Q_ARG(QVariant, query));
            }
        });
    }

    // Capture/demo seam (T-14.7f): present an in-flight external drag once the
    // chrome is up, so the live visual check can capture the identity ghost and
    // the per-target affordances without a real drag source. Values: `app` (an
    // app-alias ghost over the app region), `file-app` (files over an app), and
    // `file-trash` (files over the Trash). The identity comes from the real
    // app-index corpus; the Dock presentation is the production path. Never set
    // in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_DROP_FIXTURE")) {
        QTimer::singleShot(1500, this, [this]() {
            if (!m_dockItem)
                return;
            const QString mode = qEnvironmentVariable("DF_DOCK_DROP_FIXTURE");
            const bool isApp = mode == QLatin1String("app");
            const QVariantList apps = m_dockItem->property("appEntries").toList();
            QString appDesktopId;
            if (!apps.isEmpty())
                appDesktopId = apps.first().toMap().value(QStringLiteral("desktopId")).toString();
            DesktopEntry identity = m_index.byId(appDesktopId);
            if (!identity.valid)
                identity = m_index.resolve(appDesktopId);
            if (!identity.valid) {
                for (const DesktopEntry &entry : m_index.entries()) {
                    if (entry.valid) {
                        identity = entry;
                        break;
                    }
                }
            }
            QMetaObject::invokeMethod(m_dockItem, "reveal");
            QMetaObject::invokeMethod(m_dockItem, "beginExternalDrag",
                                      Q_ARG(QVariant, isApp), Q_ARG(QVariant, 1));
            const QString firstName =
                isApp ? QString() : QStringLiteral("report.pdf");
            QMetaObject::invokeMethod(
                m_dockItem, "setExternalPayload", Q_ARG(QVariant, isApp),
                Q_ARG(QVariant, isApp ? identity.name : QString()),
                Q_ARG(QVariant, isApp ? identity.iconPath : QString()),
                Q_ARG(QVariant, isApp ? 1 : 2), Q_ARG(QVariant, firstName));
            QString target = QStringLiteral("__trash__");
            if (mode != QLatin1String("file-trash")) {
                if (!apps.isEmpty())
                    target = apps.first().toMap().value(QStringLiteral("id")).toString();
            }
            QMetaObject::invokeMethod(m_dockItem, "externalHoverEntry",
                                      Q_ARG(QVariant, target));
        });
    }

    // Capture/demo seam (T-14.7h): open the folder stack popover once the
    // chrome is up, so the live visual check can capture the open/empty/long
    // states without a synthetic pointer click. The listing itself comes from
    // `XDG_DOWNLOAD_DIR`, which the capture script points at a scratch folder;
    // values `open`/`empty`/`long` all open the stack, `resting` leaves it
    // closed. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_STACK_FIXTURE")) {
        const QString mode = qEnvironmentVariable("DF_DOCK_STACK_FIXTURE");
        if (mode == QLatin1String("open") || mode == QLatin1String("empty")
            || mode == QLatin1String("long")) {
            QTimer::singleShot(1500, this, [this]() {
                if (m_dockItem)
                    QMetaObject::invokeMethod(m_dockItem, "openStack");
            });
        }
    }

    // Capture/demo seam (T-14.7k): pin a folder so the live visual check can
    // capture the pin, the open stack, and the drag-out removal without a real
    // drag. The value is `<absolute path>` or `<absolute path>:open`; the pin
    // is written through settingsd (the only writer) like a real drop. Never
    // set in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_FOLDER_PIN_FIXTURE")) {
        QString spec = qEnvironmentVariable("DF_DOCK_FOLDER_PIN_FIXTURE");
        bool openIt = false;
        if (spec.endsWith(QLatin1String(":open"))) {
            openIt = true;
            spec.chop(5);
        }
        const QString folder = QDir::cleanPath(spec);
        if (!folder.isEmpty()) {
            QTimer::singleShot(1500, this, [this, folder, openIt]() {
                QStringList next = m_dockConfig.pinnedFolders;
                if (!next.contains(folder))
                    next.append(folder);
                writeDockSetting(QStringLiteral("dock.pinnedFolders"), next);
                refreshFolderStackData();
                if (openIt && m_dockItem)
                    QMetaObject::invokeMethod(m_dockItem, "openStackById",
                                              Q_ARG(QVariant, QStringLiteral("folder:") + folder));
            });
        }
    }

    // Capture/demo seam (T-14.7i): show the Dock's hover name label for the
    // first entry of a kind once the chrome is up, so the live visual check can
    // capture it without a synthetic pointer. Values: `app`/`folder`/`trash`
    // (also `reveal`-then-hover when the Dock is hidden). Never set in a normal
    // session.
    if (qEnvironmentVariableIsSet("DF_DOCK_TOOLTIP_FIXTURE")) {
        QTimer::singleShot(1500, this, [this]() {
            if (!m_dockItem)
                return;
            QMetaObject::invokeMethod(m_dockItem, "reveal");
            QMetaObject::invokeMethod(
                m_dockItem, "showTooltipFor",
                Q_ARG(QVariant, qEnvironmentVariable("DF_DOCK_TOOLTIP_FIXTURE")));
        });
    }

    // Capture/demo seam (T202): open the first Dock entry's context menu.
    if (qEnvironmentVariableIsSet("DF_DOCK_MENU_FIXTURE")) {
        QTimer::singleShot(1500, this, [this]() {
            if (m_dockItem)
                QMetaObject::invokeMethod(m_dockItem, "openEntryMenuFixture");
        });
    }

    // Capture/demo seam (T-14.7m): open the window chooser for the first
    // multi-window app once it has mapped, with the first row's per-window
    // actions revealed, so the live visual check captures them without a
    // synthetic pointer. Retries until a grouped app exists. The value `scroll`
    // (T-14.7n) instead scrolls the bounded viewport mid-list and highlights a
    // visible row, for the long-list capture. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_CHOOSER_FIXTURE")) {
        const QString mode = qEnvironmentVariable("DF_DOCK_CHOOSER_FIXTURE");
        auto *timer = new QTimer(this);
        timer->setInterval(500);
        connect(timer, &QTimer::timeout, this, [this, timer, mode]() {
            if (!m_dockItem)
                return;
            QMetaObject::invokeMethod(m_dockItem, "reveal");
            QMetaObject::invokeMethod(m_dockItem, "openChooserFixture");
            if (m_dockItem->property("chooserOpen").toBool()) {
                timer->stop();
                timer->deleteLater();
                if (mode == QLatin1String("scroll")) {
                    // Let the capped popover lay out, then scroll it.
                    QTimer::singleShot(300, this, [this]() {
                        if (m_dockItem)
                            QMetaObject::invokeMethod(m_dockItem, "scrollChooserFixture");
                    });
                }
            }
        });
        timer->start();
    }

    // Capture/demo seam (T-14.7p): open the hover chooser once the Dock is up,
    // without a synthetic pointer dwell. `DF_DOCK_HOVER_FIXTURE=retarget` also
    // places the magnification pointer between two grouped entries and
    // retargets the popover to the second. The `dock.chooserOnHover` key must
    // already be on (the capture script sets it through settingsd). Never set
    // in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_HOVER_FIXTURE")) {
        const QString mode = qEnvironmentVariable("DF_DOCK_HOVER_FIXTURE");
        auto *timer = new QTimer(this);
        timer->setInterval(500);
        connect(timer, &QTimer::timeout, this, [this, timer, mode]() {
            if (!m_dockItem)
                return;
            QMetaObject::invokeMethod(m_dockItem, "reveal");
            QMetaObject::invokeMethod(m_dockItem, "hoverChooserFixture",
                                      Q_ARG(QVariant, mode));
            if (m_dockItem->property("chooserOpen").toBool())
                timer->stop();
        });
        timer->start();
    }

    // Capture/demo seam (T-14.7q): a full-size nested output fits dozens of
    // running groups, so the overflow cell cannot be produced by launching a
    // realistic number of apps. `DF_DOCK_OVERFLOW_FIXTURE=1` injects synthetic
    // running groups and caps the effective axis length so a genuine overflow
    // — and only a genuine one — appears; the timer then opens the cell's
    // "More Windows" list for the capture. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_OVERFLOW_FIXTURE")) {
        m_dockOverflowFixture = true;
        m_dockOverflowFixtureWidth = 480;
        auto *timer = new QTimer(this);
        timer->setInterval(500);
        connect(timer, &QTimer::timeout, this, [this, timer]() {
            if (!m_dockItem)
                return;
            QMetaObject::invokeMethod(m_dockItem, "reveal");
            QMetaObject::invokeMethod(m_dockItem, "openOverflowFixture");
            if (m_dockItem->property("overflowOpen").toBool()) {
                timer->stop();
                timer->deleteLater();
            }
        });
        timer->start();
    }

    // Capture/demo seam (T-14.7r): open the Trash empty progress/result popover
    // in a chosen state (`busy`, `success`, or `failed`) without running a real
    // empty, so the live visual check captures it deterministically. Never set
    // in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_TRASH_EMPTY_FIXTURE")) {
        const QString mode = qEnvironmentVariable("DF_DOCK_TRASH_EMPTY_FIXTURE");
        auto *timer = new QTimer(this);
        timer->setInterval(500);
        connect(timer, &QTimer::timeout, this, [this, timer, mode]() {
            if (!m_dockItem)
                return;
            QMetaObject::invokeMethod(m_dockItem, "reveal");
            QMetaObject::invokeMethod(m_dockItem, "trashEmptyFixture",
                                      Q_ARG(QVariant, mode));
            if (m_dockItem->property("trashEmptyOpen").toBool()) {
                timer->stop();
                timer->deleteLater();
            }
        });
        timer->start();
    }

    // Capture/demo seam (T-14.7s): pin a constant minimize-reaction phase on the
    // first running entry so the live visual check captures a mid-bounce frame
    // deterministically. `DF_DOCK_MINIMIZE_REACTION_FIXTURE` is the phase
    // (default 0.5, the peak). Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_DOCK_MINIMIZE_REACTION_FIXTURE")) {
        m_minimizeReactionFixture = true;
        bool ok = false;
        const double phase =
            qEnvironmentVariable("DF_DOCK_MINIMIZE_REACTION_FIXTURE").toDouble(&ok);
        if (ok)
            m_minimizeReactionFixturePhase = qBound(0.0, phase, 1.0);
        if (m_dockItem)
            m_dockItem->setProperty("minimizeReaction", true);
    }

    // Capture/demo seam (T-12.3a): lock the session once the chrome is up so
    // the live visual check can capture the lock screen with no hardware key
    // wiring. `DF_LOCK_FIXTURE` is the lock delay in ms (or `1` for the
    // default); `DF_LOCK_UNLOCK_MS` optionally unlocks after that delay so a
    // scripted run ends on a usable desktop. Never set in a normal session.
    if (qEnvironmentVariableIsSet("DF_LOCK_FIXTURE")) {
        const int lockDelay = qMax(1, qEnvironmentVariableIntValue("DF_LOCK_FIXTURE"));
        const int unlockDelay = qEnvironmentVariableIntValue("DF_LOCK_UNLOCK_MS");
        qInfo() << "shell: lock fixture lock-ms=" << lockDelay
                << "unlock-ms=" << unlockDelay;
        QTimer::singleShot(lockDelay, this, [this]() {
            if (m_protocol)
                m_protocol->lockSession();
        });
        if (unlockDelay > 0) {
            // The lock event can arrive after the requested unlock delay while
            // the lock UI is still configuring (the nested host throttles the
            // first commits). Retry until the protocol reports the locked
            // state, then unlock once; the seam stays inert unless set.
            auto *unlockTimer = new QTimer(this);
            unlockTimer->setInterval(250);
            connect(unlockTimer, &QTimer::timeout, this, [this, unlockTimer]() {
                if (m_protocol && m_protocol->isSessionLocked()) {
                    qInfo() << "shell: lock fixture unlocking";
                    m_protocol->unlockSession();
                    unlockTimer->stop();
                }
            });
            QTimer::singleShot(unlockDelay, this, [unlockTimer]() {
                unlockTimer->start();
            });
        }
    }

    // Launch timeout: a launch that produces no window within the bounded
    // window returns to not-running and raises a one-shot notice (T-10
    // section 8.5). This is the interim stand-in for the app-index
    // activation/attention signal (FR-4).
    m_launchTimer = new QTimer(this);
    m_launchTimer->setInterval(500);
    connect(m_launchTimer, &QTimer::timeout, this, &ShellController::onDockLaunchTick);

    // Dock animation clock (T-10 section 8.1): 16 ms while a launch or
    // attention bounce is in flight, stopped otherwise so the idle Dock
    // contributes zero wakeups (FR-8). The tick is only a sampling wake: the
    // phase is a pure function of the wall clock (`dockLaunchBouncePhase` /
    // `dockAttentionBouncePhase`), so it cannot drift from the committed
    // frames, and the tick now advances only the phase map rather than
    // rebuilding the entry model (T-14.7c). The resulting QML change renders
    // through the producer's frame-callback pacing (T-21.1c), not a render
    // timer.
    m_dockAnimTimer = new QTimer(this);
    m_dockAnimTimer->setInterval(16);
    connect(m_dockAnimTimer, &QTimer::timeout, this, &ShellController::onDockAnimationTick);

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
    sendChromePointer(m_window, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton, Qt::NoButton);
    render();
}

void ShellController::applyStatusItems()
{
    if (!m_item)
        return;
    QVariantList items;
    const QVariantMap wifi = m_statusModel ? m_statusModel->wifi() : QVariantMap();
    const QVariantMap audio = m_statusModel ? m_statusModel->audio() : QVariantMap();
    const QVariantMap battery = m_statusModel ? m_statusModel->battery() : QVariantMap();
    const QVariantMap bluetooth =
        m_statusModel ? m_statusModel->bluetooth() : QVariantMap();

    // T-15.9b: the Menu Bar pane's per-control visibility keys gate the status
    // row. The key is additive with the live adapter availability: a control
    // shows only when its daemon is present *and* the user has not hidden it.
    const QVariantMap menuControls =
        m_settingsClient ? m_settingsClient->values() : QVariantMap();
    const auto controlEnabled = [&menuControls](const char *id) {
        return menuControls
            .value(QStringLiteral("menu.control.") + QLatin1String(id), true)
            .toBool();
    };

    // Wi-Fi, volume, and battery are live from the bridge host (T-07.5a/b);
    // an absent daemon hides the slot, an error shows it visible but inert
    // (FR-4). `--placeholders` is gone, so no item is faked. Every entry now
    // carries the registry fields (priority, tooltip, menu, state, kind).
    QString wifiGlyph = wifi.value(QStringLiteral("glyph")).toString();
    if (wifiGlyph.isEmpty())
        wifiGlyph = QStringLiteral("wifi");
    items << statusItem(QStringLiteral("wifi"), wifiGlyph, QString(), tr("Wi-Fi"),
                        wifi.value(QStringLiteral("visible")).toBool()
                            && controlEnabled("wifi"),
                        0.8, wifi.value(QStringLiteral("enabled")).toBool(),
                        StatusPriority::Wifi, tr("Wi-Fi"), QStringLiteral("wifi"),
                        statusState(wifi), QStringLiteral("status"),
                        controlEnabled("wifi"));

    // T202: Bluetooth is live through the same bridge host the Control Center
    // tile uses. An absent daemon hides the slot; an error stays visible but
    // inert; the Menu Bar pane's `menu.control.bluetooth` key remains additive.
    QString bluetoothGlyph = bluetooth.value(QStringLiteral("glyph")).toString();
    if (bluetoothGlyph.isEmpty())
        bluetoothGlyph = bluetooth.value(QStringLiteral("powered")).toBool()
            ? QStringLiteral("bluetooth") : QStringLiteral("bluetooth-disabled");
    items << statusItem(QStringLiteral("bluetooth"), bluetoothGlyph, QString(),
                        tr("Bluetooth"),
                        bluetooth.value(QStringLiteral("visible")).toBool()
                            && controlEnabled("bluetooth"),
                        0.8, bluetooth.value(QStringLiteral("enabled")).toBool(),
                        StatusPriority::Bluetooth, tr("Bluetooth"),
                        QStringLiteral("bluetooth"), statusState(bluetooth),
                        QStringLiteral("status"), controlEnabled("bluetooth"));

    QString audioGlyph = audio.value(QStringLiteral("glyph")).toString();
    if (audioGlyph.isEmpty())
        audioGlyph = QStringLiteral("volume");
    items << statusItem(QStringLiteral("volume"), audioGlyph, QString(), tr("Volume"),
                        audio.value(QStringLiteral("visible")).toBool()
                            && controlEnabled("volume"),
                        audio.value(QStringLiteral("volume")).toReal(),
                        audio.value(QStringLiteral("enabled")).toBool(),
                        StatusPriority::Volume, tr("Volume"), QStringLiteral("volume"),
                        statusState(audio), QStringLiteral("status"),
                        controlEnabled("volume"));

    // The battery is read-only; the model hides it both when UPower is absent
    // and when the machine has no present battery. Charging swaps the glyph.
    QString batteryGlyph = battery.value(QStringLiteral("glyph")).toString();
    if (batteryGlyph.isEmpty())
        batteryGlyph = QStringLiteral("battery");
    if (battery.value(QStringLiteral("charging")).toBool())
        batteryGlyph = QStringLiteral("battery-charging");
    items << statusItem(QStringLiteral("battery"), batteryGlyph, QString(), tr("Battery"),
                        battery.value(QStringLiteral("visible")).toBool()
                            && controlEnabled("battery"),
                        battery.value(QStringLiteral("level")).toReal(),
                        battery.value(QStringLiteral("enabled")).toBool(),
                        StatusPriority::Battery, tr("Battery"), QStringLiteral("battery"),
                        statusState(battery), QStringLiteral("status"),
                        controlEnabled("battery"));

    QVariantMap focusItem = focusStatusItem(m_notificationModel
                                                ? m_notificationModel->focusPolicy()
                                                : QVariantMap());
    focusItem.insert(QStringLiteral("priority"), StatusPriority::Focus);
    focusItem.insert(QStringLiteral("tooltip"), tr("Focus"));
    focusItem.insert(QStringLiteral("menu"), QString());
    focusItem.insert(QStringLiteral("state"), QStringLiteral("ok"));
    focusItem.insert(QStringLiteral("kind"), QStringLiteral("status"));
    focusItem.insert(QStringLiteral("thirdParty"), false);
    focusItem.insert(QStringLiteral("essential"), false);
    if (!controlEnabled("focus")) {
        focusItem.insert(QStringLiteral("available"), false);
        focusItem.insert(QStringLiteral("enabled"), false);
        focusItem.insert(QStringLiteral("state"), QStringLiteral("unavailable"));
    }
    items << focusItem;

    items << statusItem(QStringLiteral("accessibility"), QStringLiteral("accessibility"),
                        QString(), tr("Accessibility"), false,
                        0.8, false, 65, tr("Accessibility"), QString(),
                        QStringLiteral("unavailable"), QStringLiteral("status"),
                        controlEnabled("accessibility"));

    // T231: Search and Notifications join the registry as action extras. The
    // shell routes their activation to the Applications drawer and the
    // Notification Center; the clock no longer doubles as the Center's entry.
    items << statusItem(QStringLiteral("search"), QStringLiteral("search"), QString(),
                        tr("Search"), true, 0.8, true, StatusPriority::Search,
                        tr("Search"), QString(), QStringLiteral("ok"),
                        QStringLiteral("action"), controlEnabled("search"));

    const bool notificationsAvailable =
        m_notificationClient && m_notificationClient->isAvailable();
    const int activeNotifications =
        m_notificationCenter ? m_notificationCenter->activeCount() : 0;
    // `icon` is a StatusGlyph state name, not a raw Tabler glyph name:
    // "notifications" maps to the bell in StatusGlyph.
    QVariantMap notificationItem =
        statusItem(QStringLiteral("notifications"), QStringLiteral("notifications"),
                   activeNotifications > 0 ? QString::number(activeNotifications)
                                           : QString(),
                   tr("Notifications"), notificationsAvailable, 0.8, true,
                   StatusPriority::Notifications, tr("Notifications"), QString(),
                   activeNotifications > 0 ? QStringLiteral("attention")
                                           : QStringLiteral("ok"),
                   QStringLiteral("action"), controlEnabled("notifications"));
    notificationItem.insert(QStringLiteral("selected"), activeNotifications > 0);
    items << notificationItem;

    // The clock is a first-class registry extra: the panel formatting lives in
    // MenuBarClock, but order, visibility, keyboard focus and overflow are the
    // registry's.
    items << statusItem(QStringLiteral("clock"), QString(), QString(),
                        tr("Date & Time"), true, 0.8, true, StatusPriority::Clock,
                        tr("Date & Time"), QStringLiteral("clock"),
                        QStringLiteral("ok"), QStringLiteral("clock"),
                        controlEnabled("clock"));

    items << statusItem(QStringLiteral("control-center"),
                        QStringLiteral("control-center"), QString(),
                        tr("Quick Settings"), true, 0.8, true,
                        StatusPriority::ControlCenter, tr("Quick Settings"),
                        QString(), QStringLiteral("ok"), QStringLiteral("action"),
                        controlEnabled("controlCenter"));

    items << statusItem(QStringLiteral("mission-control"),
                        QStringLiteral("mission-control"), QString(),
                        tr("Overview"), true, 0.8, true,
                        StatusPriority::MissionControl, tr("Overview"), QString(),
                        QStringLiteral("ok"), QStringLiteral("action"), true);

    // Third-party StatusNotifier/AppIndicator items (T-14.3/T231). They join
    // the same registry, so sizing/hover/dark-light treatment is identical and
    // the overflow policy can push them out before any system item.
    int trayPriority = StatusPriority::ThirdParty;
    for (const TrayItem &tray : m_trayItems) {
        QVariantMap item =
            statusItem(QStringLiteral("tray:") + tray.name,
                       QStringLiteral("application"), QString(),
                       !tray.title.isEmpty() ? tray.title
                                             : (!tray.id.isEmpty() ? tray.id : tray.name),
                       true, 0.8, true, trayPriority++,
                       tray.tooltip.isEmpty() ? tray.title : tray.tooltip,
                       QString(),
                       tray.needsAttention ? QStringLiteral("attention")
                                           : QStringLiteral("ok"),
                       QStringLiteral("status"), true);
        item.insert(QStringLiteral("iconSource"),
                    tray.iconPath.isEmpty() ? QUrl()
                                            : QUrl::fromLocalFile(tray.iconPath));
        item.insert(QStringLiteral("selected"), tray.needsAttention);
        item.insert(QStringLiteral("thirdParty"), true);
        items << item;
    }

    m_item->setProperty("statusItems", items);
}

void ShellController::refreshStatusViews()
{
    if (!m_statusClient)
        return;
    m_statusClient->refreshWifi();
    m_statusClient->refreshAudio();
    m_statusClient->refreshBattery();
    m_statusClient->refreshBluetooth();
    m_statusClient->refreshStorage();
    m_statusClient->refreshInput();
    m_statusClient->refreshUpdates();
    m_statusClient->refreshAccounts();
    m_statusClient->refreshPrinters();
    m_statusClient->refreshPrivacy();
    m_statusClient->refreshAccessibility();
    m_statusClient->refreshVpn();
}

void ShellController::applyStatusMenuData()
{
    if (!m_item || !m_statusModel)
        return;
    m_item->setProperty("wifiMenu", m_statusModel->wifi());
    m_item->setProperty("volumeMenu", m_statusModel->audio());
    m_item->setProperty("batteryMenu", m_statusModel->battery());
    m_item->setProperty("bluetoothMenu", m_statusModel->bluetooth());
    applyStatusItems();
    // A live Wi-Fi/volume change reaches an open Control Center too.
    if (m_controlCenterOpen)
        applyControlCenterData();
}

void ShellController::refreshTrayItems()
{
    if (!m_trayClient.available()) {
        if (!m_trayItems.isEmpty()) {
            m_trayItems.clear();
            if (m_item)
                applyStatusItems();
        }
        return;
    }
    const QList<TrayItem> items = m_trayClient.items();
    bool changed = items.size() != m_trayItems.size();
    if (!changed) {
        for (int i = 0; i < items.size(); ++i) {
            const TrayItem &next = items.at(i);
            const TrayItem &prev = m_trayItems.at(i);
            if (next.name != prev.name || next.title != prev.title
                || next.iconPath != prev.iconPath || next.needsAttention != prev.needsAttention) {
                changed = true;
                break;
            }
        }
    }
    if (!changed)
        return;
    m_trayItems = items;
    if (m_item)
        applyStatusItems();
}

void ShellController::openTrayMenu(const QString &name)
{
    if (!m_item)
        return;
    // The menu rows come from the item's DBusMenu (fetched and projected by
    // app-index). The QML owns the popup geometry and reports the trigger.
    m_item->setProperty("trayMenu", m_trayClient.menu(name));
    QMetaObject::invokeMethod(m_item, "openTrayMenu", Q_ARG(QVariant, name));
    m_menuOpen = true;
    schedulePopupGeometry();

}

void ShellController::onTrayMenuTriggered(const QString &service, int id)
{
    if (id >= 0)
        m_trayClient.menuEvent(service, id);
    m_menuOpen = false;
    updatePopupGeometry();
}

void ShellController::onTrayMenuClosed()
{
    m_menuOpen = false;
    // The logical menu is closed immediately: stop intercepting clicks and
    // drop the decorative material while the visual exit plays.
    if (m_protocol) {
        m_protocol->setPopupInputActive(false);
        m_protocol->clearPopupPanelRegions();
    }
    QTimer::singleShot(210, this, [this]() {
        if (!m_menuOpen)
            updatePopupGeometry();
    });
}

void ShellController::onStatusMenuOpened()
{
    m_menuOpen = true;
    schedulePopupGeometry();
}

void ShellController::onStatusMenuClosed()
{
    m_menuOpen = false;
    if (m_protocol) {
        m_protocol->setPopupInputActive(false);
        m_protocol->clearPopupPanelRegions();
    }
    // The measured menu exit is 183 ms; keep the overlay mapped for the
    // visual tail, but the logical interaction and input are already closed.
    QTimer::singleShot(210, this, [this]() {
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
    else if (itemId == QLatin1String("bluetooth"))
        m_statusClient->refreshBluetooth();
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
    // A volume change presents the OSD on the active output (T-11.4a).
    const bool muted = m_statusModel
                           ? m_statusModel->audio().value(QStringLiteral("muted")).toBool()
                           : false;
    showOsd(OsdModel::Kind::Volume, volume, muted);
}

void ShellController::onMuteToggleRequested()
{
    if (!m_statusClient)
        return;
    const QVariantMap audio = m_statusModel ? m_statusModel->audio() : QVariantMap();
    const bool muted = audio.value(QStringLiteral("muted")).toBool();
    const double volume = audio.value(QStringLiteral("volume")).toDouble();
    m_statusClient->setMute(!muted);
    // Muting is a volume change: show the OSD with the resulting state.
    showOsd(OsdModel::Kind::Volume, volume, !muted);
}

void ShellController::onWifiState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyWifiJson(json);
    noteInitialStatusReply(kInitialStatusWifi);
}

void ShellController::onAudioState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyAudioJson(json);
    noteInitialStatusReply(kInitialStatusAudio);
}

void ShellController::onBatteryState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyBatteryJson(json);
    noteInitialStatusReply(kInitialStatusBattery);
}

void ShellController::onBluetoothState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyBluetoothJson(json);
}

void ShellController::onStorageState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyStorageJson(json);
}

void ShellController::onInputState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyInputJson(json);
}

void ShellController::onUpdatesState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyUpdatesJson(json);
}

void ShellController::onAccountsState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyAccountsJson(json);
}

void ShellController::onPrintersState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyPrintersJson(json);
}

void ShellController::onPrivacyState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyPrivacyJson(json);
}

void ShellController::onAccessibilityState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyAccessibilityJson(json);
}

void ShellController::onVpnState(const QByteArray &json)
{
    if (m_statusModel)
        m_statusModel->applyVpnJson(json);
}

void ShellController::onStatusReport(const QByteArray &json)
{
    qInfo() << "shell: system-status action:" << SystemStatusModel::outcomeOf(json);
    // The action reply is not the new state: re-read the host, whose adapter
    // state is the single source of truth.
    refreshStatusViews();
}

void ShellController::refreshSystemMenu()
{
    if (!m_item)
        return;
    QVariantList recent;
    QSet<QString> seen;
    const int limit = qBound(0, m_menuRecentItems, 50);
    for (const QString &appId : std::as_const(m_recentAppIds)) {
        if (recent.size() >= limit)
            break;
        const DesktopEntry entry = m_index.resolve(appId);
        if (!entry.valid || entry.id.isEmpty() || seen.contains(entry.id))
            continue;
        seen.insert(entry.id);
        recent.append(QVariantMap{{QStringLiteral("desktopId"), entry.id},
                                  {QStringLiteral("name"), entry.name}});
    }
    m_item->setProperty(
        "systemMenuItems",
        ShellMenus::systemMenu(m_systemUserName, m_devReturnDesktop, recent));
}

void ShellController::applyFocusedApp()
{
    if (!m_item)
        return;
    if (m_protocol)
        m_focusedWindowId = m_protocol->focusedWindowId();
    const QString rawId = m_appId;
    const DesktopEntry entry = m_index.resolve(rawId);
    const bool hasDisplayName = entry.valid && !entry.name.isEmpty();
    QString name = rawId.isEmpty() ? QStringLiteral("Files")
        : hasDisplayName ? entry.name : displayNameForIdentity(rawId);
    const QString canonicalId = entry.valid && !entry.id.isEmpty() ? entry.id : rawId;
    const QString brokerId = canonicalId;
    const bool focusChanged = m_item->property("appId").toString() != rawId;
    if (!focusChanged && !hasDisplayName && !rawId.isEmpty()
        && !m_item->property("appName").toString().isEmpty())
        name = m_item->property("appName").toString();
    if (focusChanged)
        QMetaObject::invokeMethod(m_item, "closeMenus");
    m_item->setProperty("appId", rawId);
    m_item->setProperty("appName", name);
    if (focusChanged || m_appliedApplicationMenu.isEmpty()) {
        m_dbusMenuWindowId = 0;
        const QVariantList localMenu =
            fixedApplicationMenu(name, appMenuLiveState(rawId, m_runningEntries));
        m_appliedApplicationMenu = localMenu;
        m_appliedAppMenu.clear();
        m_item->setProperty("applicationMenuItems", localMenu);
        m_item->setProperty("appMenuModel", QVariantList{});
        if (m_protocol)
            m_protocol->setAppAccelerators(rawId, acceleratorWireTable(
                publishedAccelerators(localMenu, {})));
    }
    refreshSystemMenu();

    // D-Bus exporters may take seconds to answer AboutToShow. Keep them off
    // the scene/input thread shared by the Dock. One bounded worker coalesces
    // bursts of title/state/focus events; only the newest snapshot can publish.
    const quint64 generation = ++m_menuRefreshGeneration;
    if (m_menuRefreshInFlight)
        return;
    m_menuRefreshInFlight = true;
    m_menuWorker.setMaxThreadCount(1);
    const QVariantList runningEntries = m_runningEntries;
    const quintptr focusedWindowId = m_focusedWindowId;
    m_menuWorker.start([this, generation, rawId, canonicalId, brokerId,
                        name, hasDisplayName, runningEntries, focusedWindowId]() mutable {
        const AppIndexClient indexClient;
        const MenuBrokerClient broker;
        QVariantList applicationMenu;
        QVariantList appMenu;
        QString tier;
        quint32 dbusWindow = 0;

        // T230: focus → app identity → AppMenu.Registrar registration. A
        // registration can arrive under the app's bus name (the standard
        // `RegisterWindow` call); focus is the first moment the shell knows the
        // canonical desktop id, so it re-keys the bridge model to that identity
        // before resolving it. The registration's own window id is kept for click
        // routing.
        if (indexClient.available()) {
            const QList<AppMenuWindow> windows = indexClient.appMenuWindows();
            const AppMenuWindow *registration = findAppMenuWindow(windows, rawId, canonicalId);
            // A standard `RegisterWindow` keys the bridge by the caller's bus name,
            // so the app-id lookup cannot see it. The compositor's focused window
            // id is the second, authoritative match; `FocusWindow` then re-keys the
            // registration under the canonical desktop id.
            if (!registration && focusedWindowId != 0) {
                for (const AppMenuWindow &window : windows) {
                    if (window.windowId == focusedWindowId) {
                        registration = &window;
                        break;
                    }
                }
            }
            if (registration) {
                dbusWindow = registration->windowId;
                const QString key = canonicalId.isEmpty() ? registration->appId : canonicalId;
                indexClient.focusWindow(QString::number(registration->windowId), key);
                if (broker.available()) {
                    broker.setFocusedApp(key);
                    broker.setWindowStates(windowStatesJson(runningEntries));
                    ResolvedMenu resolved = broker.resolve(key);
                    if (!resolved.valid)
                        resolved = broker.resolveFocused();
                    if (resolved.valid) {
                        if (!hasDisplayName && !resolved.appName.isEmpty()
                        && resolved.appName != rawId && resolved.appName != canonicalId
                        && !resolved.appName.endsWith(QLatin1String(".desktop")))
                            name = resolved.appName;
                        applicationMenu = resolved.applicationMenuItems;
                        appMenu = resolved.menus;
                        tier = resolved.tier;
                    }
                }
                // A fresh `WindowMenu` read runs `AboutToShow` first, so a dynamic
                // menu (recent files, window lists) reflects the state at open
                // time instead of the registration snapshot. A native publication
                // still wins the tier priority (native → DBusMenu → none).
                if (tier != QLatin1String("native")) {
                    const QVariantList fresh =
                        indexClient.windowMenu(QString::number(registration->windowId));
                    if (!fresh.isEmpty()) {
                        appMenu = fresh;
                        tier = QStringLiteral("dbusmenu");
                    } else if (tier.isEmpty()) {
                        tier = QStringLiteral("dbusmenu");
                    }
                }
                if (applicationMenu.isEmpty() || tier != QLatin1String("native")) {
                    const AppMenuLiveState live = appMenuLiveState(rawId, runningEntries);
                    applicationMenu = fixedApplicationMenu(name, live);
                }

            }
        }

        if (applicationMenu.isEmpty() && broker.available()) {
            broker.setFocusedApp(brokerId);
            broker.setWindowStates(windowStatesJson(runningEntries));
            ResolvedMenu resolved = broker.resolve(brokerId);
            if (!resolved.valid)
                resolved = broker.resolveFocused();
            if (resolved.valid) {

                if (!hasDisplayName && !resolved.appName.isEmpty()
                        && resolved.appName != rawId && resolved.appName != canonicalId
                        && !resolved.appName.endsWith(QLatin1String(".desktop")))
                    name = resolved.appName;
                applicationMenu = resolved.applicationMenuItems;
                appMenu = resolved.menus;
                tier = resolved.tier;
            }
        }
        if (applicationMenu.isEmpty() || tier != QLatin1String("native")) {
            const AppMenuLiveState live = appMenuLiveState(rawId, runningEntries);
            applicationMenu = fixedApplicationMenu(name, live);
            if (tier.isEmpty())
                tier = QStringLiteral("none");
        }
        // T230: no-export and DBusMenu apps get the dynamic Window menu from the
        // live compositor projection. A published Window menu wins. The broker
        // already appends its synthesized menu when it owns the model, so never
        // duplicate it.
        if (!rawId.isEmpty() && !hasMenuTitle(appMenu, QStringLiteral("Window"))) {
            const QVariantList dynamicWindow = windowMenu(runningEntries, rawId, name);
            if (!dynamicWindow.isEmpty()) {
                QVariantMap menu;
                menu.insert(QStringLiteral("title"), QStringLiteral("Window"));
                menu.insert(QStringLiteral("items"), dynamicWindow);
                appMenu.append(menu);
            }
        }
        QMetaObject::invokeMethod(this,
            [this, generation, rawId, name, dbusWindow, applicationMenu, appMenu]() {
                m_menuRefreshInFlight = false;
                if (generation != m_menuRefreshGeneration) {
                    applyFocusedApp();
                    return;
                }
                if (!m_item)
                    return;
                if (m_menuOpen) {
                    m_menuRefreshDeferred = true;
                    return;
                }
                m_dbusMenuWindowId = dbusWindow;
                m_item->setProperty("appName", rawId.isEmpty() ? QStringLiteral("Files") : name);
                if (m_appliedApplicationMenu != applicationMenu) {
                    m_appliedApplicationMenu = applicationMenu;
                    m_item->setProperty("applicationMenuItems", applicationMenu);
                }
                if (m_appliedAppMenu != appMenu) {
                    m_appliedAppMenu = appMenu;
                    m_item->setProperty("appMenuModel", appMenu);
                }
                if (m_protocol) {
                    m_protocol->setAppAccelerators(rawId, acceleratorWireTable(
                        publishedAccelerators(applicationMenu, appMenu)));
                }
                render();
            }, Qt::QueuedConnection);
    });
}

void ShellController::hideAppWindows(const QString &appId)
{
    if (!m_protocol || appId.isEmpty())
        return;
    const QStringList windows = visibleWindowIds(m_runningEntries, appId);
    for (const QString &windowId : windows) {
        m_protocol->setToplevelMinimized(windowId, true);
        m_hiddenWindowIds.insert(windowId);
    }
    qInfo() << "shell: Hide" << appId << "minimized" << windows.size() << "window(s)";
    if (!windows.isEmpty())
        applyFocusedApp();
}

void ShellController::hideOtherAppWindows(const QString &appId)
{
    if (!m_protocol || appId.isEmpty())
        return;
    const QStringList windows = visibleWindowIds(m_runningEntries, appId, true);
    for (const QString &windowId : windows) {
        m_protocol->setToplevelMinimized(windowId, true);
        m_hiddenWindowIds.insert(windowId);
    }
    qInfo() << "shell: Hide Others minimized" << windows.size() << "window(s)";
    // The focused app stays in front (macOS Hide Others raises it).
    if (!windows.isEmpty()) {
        m_protocol->activateApp(appId);
        applyFocusedApp();
    }
}

void ShellController::showHiddenWindows()
{
    if (!m_protocol)
        return;
    QStringList windows(m_hiddenWindowIds.begin(), m_hiddenWindowIds.end());
    if (windows.isEmpty())
        windows = minimizedWindowIds(m_runningEntries);
    if (windows.isEmpty())
        return;
    for (const QString &windowId : windows)
        m_protocol->setToplevelMinimized(windowId, false);
    qInfo() << "shell: Show All restored" << windows.size() << "window(s)";
    m_hiddenWindowIds.clear();
    applyFocusedApp();
}

bool ShellController::routeDbusMenuAction(const QString &action)
{
    constexpr auto prefix = "dbusmenu:";
    if (!action.startsWith(QLatin1String(prefix)))
        return false;
    bool ok = false;
    const int id = action.mid(int(qstrlen(prefix))).toInt(&ok);
    // Prefer the registration matched at menu-open time; fall back to the
    // focused window id when the broker supplied the model directly.
    const quint32 target = m_dbusMenuWindowId != 0
                               ? m_dbusMenuWindowId
                               : static_cast<quint32>(m_focusedWindowId);
    if (!ok || target == 0) {
        qWarning() << "shell: cannot route DBusMenu action" << action
                   << "(registration" << target << ")";
        return false;
    }
    const bool delivered = m_appIndexClient.windowMenuEvent(QString::number(target), id);
    if (!delivered) {
        qWarning() << "shell: DBusMenu click was not accepted for window"
                   << target << "item" << id;
        return false;
    }
    qInfo() << "shell: DBusMenu click" << id << "delivered to window" << target;
    // A click can change the menu (checked rows, dynamic recent items), so
    // re-read the focused model on the next event-loop turn.
    QTimer::singleShot(0, this, &ShellController::applyFocusedApp);
    return true;
}

void ShellController::onConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's anchor/size; skip it. The bar
    // surface is the veil extent tall (T200); the QML item and the committed
    // buffer remain the bar strip, and the dropdown is a separate `overlay`
    // surface.
    if (height != m_menuBarSurfaceHeight) {
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
    qInfo() << "shell: focused app" << appId << title;
    // T230: focus is the route from the compositor's app identity to the
    // DBusMenu registration; `applyFocusedApp` resolves the registration and
    // re-keys its bridge model under the canonical desktop id.
    m_focusedWindowId = m_protocol ? m_protocol->focusedWindowId() : 0;
    qInfo() << "shell: focus" << appId << title << "window" << m_focusedWindowId;
    // FR-4: attention stops when the app's window gains focus.
    clearAttention(appId);
    // Track recency for the suggested entries (`dock.showRecentApps`, T-10
    // section 17) and the system menu's Recent Items (T230). The list is kept
    // even when a consumer is off so toggling it on is immediate.
    if (!appId.isEmpty()) {
        m_recentAppIds.removeAll(appId);
        m_recentAppIds.prepend(appId);
        while (m_recentAppIds.size() > 12)
            m_recentAppIds.removeLast();
        if (m_dockConfig.showRecentApps)
            rebuildDockEntries();
    }
    refreshSystemMenu();
    applyFocusedApp();
    render();
}

// --- Notification Center (T204) --------------------------------------------

void ShellController::toggleNotificationCenter()
{
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("toggle"));
    else
        showNotificationCenter();
}

void ShellController::showNotificationCenter()
{
    if (!m_notificationCenterItem || !m_notificationCenter)
        return;
    // One ambient panel at a time: opening the Center retires an open menu,
    // the Control Center, the Applications drawer and Mission Control rather
    // than stacking transient overlays.
    if (m_item) {
        QMetaObject::invokeMethod(m_item, "closeMenus");
        QMetaObject::invokeMethod(m_item, "closeStatusMenu");
    }
    if (m_controlCenterOpen)
        hideControlCenter();
    if (m_appsDrawerOpen)
        hideAppsDrawer();
    if (m_overviewActive && m_protocol)
        m_protocol->exitMissionControl();

    m_notificationCenterOpen = true;
    m_notificationCenterPending = true;
    const bool available = m_notificationClient && m_notificationClient->isAvailable();
    m_notificationCenter->setAvailable(available);
    if (m_notificationClient)
        m_notificationClient->refresh();
    m_notificationCenter->open();
    updateNotificationCenterGeometry();
    applyNotificationCenterData();
    if (m_notificationCenterWidth > 0 && m_notificationCenterHeight > 0)
        renderNotificationCenter();
    // Take active focus so Escape and Up/Down/Return reach the list; the
    // compositor hands the keyboard to the OnDemand layer surface.
    m_notificationCenterItem->forceActiveFocus();
    if (m_protocol) {
        m_protocol->setNotificationCenterInputRegion(m_notificationCenterWidth,
                                                     m_notificationCenterHeight);
        m_protocol->setNotificationCenterKeyboardGrab(true);
    }
}

void ShellController::hideNotificationCenter(const QString &reason)
{
    // `NotificationCenter::close` emits `closed`, which lands in
    // `onNotificationCenterStateClosed` and owns the surface teardown. If the
    // state was already closed (a compositor-driven surface close), run the
    // teardown directly.
    if (m_notificationCenter && m_notificationCenter->isOpen()) {
        m_notificationCenter->close(reason);
        return;
    }
    onNotificationCenterStateClosed(reason);
}

void ShellController::onNotificationCenterStateClosed(const QString &reason)
{
    m_notificationCenterOpen = false;
    m_notificationCenterPending = false;
    m_notificationCenterButtons = Qt::NoButton;
    if (m_protocol) {
        // Clear the input region and the material regions before unmapping so
        // a stale clickable or frosted panel never survives the close, then
        // hand focus back to the active window.
        m_protocol->setNotificationCenterInputRegion(0, 0);
        m_protocol->clearNotificationCenterPanelRegions();
        m_protocol->setNotificationCenterKeyboardGrab(false);
        m_protocol->hideNotificationCenter();
        if (m_notificationCenterKeyboardFocused)
            m_protocol->releaseKeyboardFocus();
    }
    m_notificationCenterKeyboardFocused = false;
    if (!reason.isEmpty())
        qInfo() << "shell: notification center closed:" << reason;
}

void ShellController::onNotificationCenterDismissRequested()
{
    hideNotificationCenter(QStringLiteral("escape"));
}

void ShellController::onNotificationCenterMoveRequested(int delta)
{
    if (m_notificationCenter)
        m_notificationCenter->moveSelection(delta);
}

void ShellController::onNotificationCenterSelectRequested(int index)
{
    if (m_notificationCenter)
        m_notificationCenter->selectIndex(index);
}

void ShellController::onNotificationCenterActivateRequested()
{
    if (m_notificationCenter)
        m_notificationCenter->activateSelected();
}

void ShellController::onNotificationCenterDismissSelectedRequested()
{
    if (m_notificationCenter)
        m_notificationCenter->dismissSelected();
}

void ShellController::onNotificationCenterActionRequested(int id, const QString &actionKey)
{
    if (m_notificationCenter)
        m_notificationCenter->invoke(static_cast<quint32>(id), actionKey);
}

void ShellController::onNotificationCenterEntryDismissRequested(int id)
{
    if (m_notificationCenter)
        m_notificationCenter->dismiss(static_cast<quint32>(id));
}

void ShellController::onNotificationCenterConfigured(int width, int height, quint32)
{
    // The compositor's pre-layout configure is the full output; the panel's
    // own size follows (the same rule as the Control Center).
    updateNotificationCenterGeometry();
    if (width != kNotificationCenterWidth || height != m_notificationCenterHeight)
        return;
    m_notificationCenterWidth = width;
    if (m_protocol)
        m_protocol->setNotificationCenterInputRegion(m_notificationCenterWidth,
                                                     m_notificationCenterHeight);
    if (m_notificationCenterPending)
        renderNotificationCenter();
}

void ShellController::onNotificationCenterPointerMoved(qreal x, qreal y)
{
    if (!m_notificationCenterWindow)
        return;
    sendChromePointer(m_notificationCenterWindow, QEvent::MouseMove, QPointF(x, y),
                      Qt::NoButton, m_notificationCenterButtons);
    scheduleNotificationCenterRender();
}

void ShellController::onNotificationCenterPointerButton(qreal x, qreal y, quint32 button,
                                                        bool pressed)
{
    if (!m_notificationCenterWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    if (pressed)
        m_notificationCenterButtons |= qtButton;
    else
        m_notificationCenterButtons &= ~qtButton;
    sendChromePointer(m_notificationCenterWindow,
                      pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease,
                      QPointF(x, y), qtButton, m_notificationCenterButtons);
    scheduleNotificationCenterRender();
}

void ShellController::onNotificationCenterPointerLeft()
{
    if (!m_notificationCenterWindow)
        return;
    sendChromePointer(m_notificationCenterWindow, QEvent::MouseMove, QPointF(-1, -1),
                      Qt::NoButton, m_notificationCenterButtons);
    scheduleNotificationCenterRender();
}

void ShellController::onNotificationCenterKeyboardFocused(bool focused)
{
    const bool wasFocused = m_notificationCenterKeyboardFocused;
    m_notificationCenterKeyboardFocused = focused;
    if (!focused && wasFocused && m_notificationCenterOpen) {
        // Losing the keyboard is the click-away dismissal. A focus hand-off
        // during the opening click arrives before the surface ever holds it,
        // so only a real loss closes.
        hideNotificationCenter(QStringLiteral("outside"));
    } else if (focused && m_notificationCenterOpen && m_notificationCenterItem) {
        m_notificationCenterItem->forceActiveFocus();
    }
}

void ShellController::onNotificationCenterKeyEvent(quint32 key, bool pressed)
{
    if (!m_notificationCenterWindow || !m_notificationCenterOpen)
        return;
    sendChromeKeyEvent(m_notificationCenterWindow, key, pressed,
                       m_protocol ? m_protocol->keyboardModifiers() : 0);
    scheduleNotificationCenterRender();
}

void ShellController::onNotificationCenterSurfaceClosed()
{
    // The compositor closed the layer (output teardown). Drop the panel state
    // without trying to unmap a surface that no longer exists.
    if (m_notificationCenter)
        m_notificationCenter->close(QStringLiteral("output"));
    m_notificationCenterOpen = false;
    m_notificationCenterPending = false;
    m_notificationCenterButtons = Qt::NoButton;
    m_notificationCenterKeyboardFocused = false;
}

void ShellController::applyNotificationCenterData()
{
    if (!m_notificationCenterItem || !m_notificationCenter)
        return;
    m_notificationCenterItem->setProperty("history", m_notificationCenter->history());
    m_notificationCenterItem->setProperty("selectedIndex",
                                          m_notificationCenter->selectedIndex());
    m_notificationCenterItem->setProperty("available", m_notificationCenter->available());
    // T205: a co-visible banner owns the top strip; inset the Center's stack
    // below the card so the two trusted surfaces never overlap or sample one
    // another. The inset follows the banner's action-row height.
    const bool hasBanner = m_notificationModel && m_notificationModel->hasBanner();
    int bannerCardHeight = 0;
    if (hasBanner) {
        const QVariantList actions =
            m_notificationModel->banner().value(QStringLiteral("actions")).toList();
        bannerCardHeight = actions.isEmpty() ? kBannerCardHeight
                                             : kBannerCardActionsHeight;
    }
    m_notificationCenterItem->setProperty(
        "bannerInset", hasBanner ? bannerCardHeight + kBannerTopGap : 0);
    if (m_notificationCenterOpen)
        scheduleNotificationCenterRender();
}

void ShellController::renderNotificationCenter()
{
    if (!m_notificationCenterOpen || !m_notificationCenterPending
        || !m_notificationCenterWindow || !m_notificationCenterItem) {
        return;
    }
    if (m_notificationCenterWidth <= 0 || m_notificationCenterHeight <= 0)
        return;
    prepareChromeScene(m_notificationCenterWindow, m_notificationCenterItem,
                       QSize(m_notificationCenterWidth, m_notificationCenterHeight),
                       QSize(m_notificationCenterWidth, m_notificationCenterHeight),
                       m_chromeScale);
    // T205: the stack and lower panel are disjoint, independently keyed
    // regions; the list is commit-latched with the image below.
    if (m_protocol) {
        m_protocol->setNotificationCenterPanelRegions(
            m_notificationCenterItem->property("materialRegions").toList());
    }
    const QSize logical(m_notificationCenterWidth, m_notificationCenterHeight);
    commitChromeScene(m_notificationCenterWindow,
                      {{QStringLiteral("notification-center"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitNotificationCenterImage(image, logical);
                        }}});
}

void ShellController::scheduleNotificationCenterRender()
{
    if (m_notificationCenterRenderPending)
        return;
    m_notificationCenterRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_notificationCenterRenderPending = false;
        renderNotificationCenter();
    });
}

void ShellController::updateNotificationCenterGeometry()
{
    if (!m_protocol)
        return;
    const QSize output = m_protocol->outputLogicalSize();
    if (output.isEmpty())
        return;
    const int available = output.height() - m_barHeight - kNotificationCenterTopGap;
    const int target = qMin(kNotificationCenterHeight, available);
    if (target <= 0 || target == m_notificationCenterHeight)
        return;
    m_notificationCenterHeight = target;
    m_protocol->setNotificationCenterSize(kNotificationCenterWidth, target);
}

void ShellController::onControlCenterRequested()
{
    toggleControlCenter();
}

void ShellController::toggleControlCenter()
{
    if (m_controlCenterOpen)
        hideControlCenter();
    else
        showControlCenter();
}

void ShellController::showControlCenter()
{
    if (!m_controlCenterItem)
        return;
    // The two top-right ambient panels are mutually exclusive.
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    if (m_appsDrawerOpen)
        hideAppsDrawer();
    m_controlCenterOpen = true;
    m_controlCenterPending = true;
    updateControlCenterGeometry();
    applyControlCenterData();
    if (m_controlCenterWidth > 0 && m_controlCenterHeight > 0)
        renderControlCenter();
    // Take active focus so Escape is delivered to the panel and a later focus
    // loss (a click on a window) reads as dismissal.
    m_controlCenterItem->forceActiveFocus();
    if (m_protocol)
        m_protocol->setControlCenterInputRegion(m_controlCenterWidth, m_controlCenterHeight);
}

void ShellController::hideControlCenter()
{
    m_controlCenterOpen = false;
    m_controlCenterPending = false;
    if (m_protocol) {
        m_protocol->setControlCenterInputRegion(0, 0);
        m_protocol->clearControlCenterPanelRegions();
        m_protocol->hideControlCenter();
    }
}

void ShellController::onControlCenterClosed()
{
    hideControlCenter();
}

void ShellController::applyControlCenterData()
{
    if (!m_controlCenterItem)
        return;
    const QVariantMap wifi = m_statusModel ? m_statusModel->wifi() : QVariantMap();
    const QVariantMap audio = m_statusModel ? m_statusModel->audio() : QVariantMap();
    const QVariantMap settingsValues =
        m_settingsClient ? m_settingsClient->values() : QVariantMap();
    const double brightness = m_settingsClient
        ? displaySettingsFromValues(settingsValues).brightness
        : 1.0;
    // The Focus/DND policy is owned by the notification service; the model
    // holds the latest decoded view (T-11.2b).
    const QVariantMap focus = m_notificationModel ? m_notificationModel->focusPolicy()
                                                  : QVariantMap();
    // The panel shows the effective scheme: `auto` follows the host, exactly
    // as ThemeBinding resolves it for the design-system Theme (T-08.2b).
    const QString scheme = m_settingsClient
        ? m_settingsClient->values().value(QStringLiteral("appearance.colorScheme")).toString()
        : QStringLiteral("auto");
    const bool dark = ThemeBinding::darkForScheme(scheme, ThemeBinding::hostDark());
    const QVariantMap bluetooth = m_statusModel ? m_statusModel->bluetooth() : QVariantMap();
    const QVariantMap storage = m_statusModel ? m_statusModel->storage() : QVariantMap();
    const QVariantMap input = m_statusModel ? m_statusModel->input() : QVariantMap();
    // Battery / power profiles (T-15.6b): the tile reflects the bridge host's
    // battery view (the same view the Settings pane and menu bar read).
    const QVariantMap battery = m_statusModel ? m_statusModel->battery() : QVariantMap();
    // General/About/Updates (T-15.10b): the tile reflects the bridge host's
    // host-stack view (the same view the Settings General pane reads).
    const QVariantMap updates = m_statusModel ? m_statusModel->updates() : QVariantMap();
    // Users and Groups (T-15.11b): the tile reflects the bridge host's
    // AccountsService/distro-group view (the same view the Settings pane reads).
    const QVariantMap accounts = m_statusModel ? m_statusModel->accounts() : QVariantMap();
    // Printers and Scanners (T-15.12b): the tile reflects the bridge host's
    // CUPS/SANE view (the same view the Settings pane reads).
    const QVariantMap printers = m_statusModel ? m_statusModel->printers() : QVariantMap();
    // Privacy and Security (T-15.13b): the tile reflects the bridge host's
    // portal PermissionStore view (the same view the Settings pane reads).
    const QVariantMap privacy = m_statusModel ? m_statusModel->privacy() : QVariantMap();
    // Accessibility (T-15.14b): the tile reflects the bridge host's live AT-SPI
    // view (the same view the Settings pane reads).
    const QVariantMap accessibility =
        m_statusModel ? m_statusModel->accessibility() : QVariantMap();
    // Network advanced (VPN) (T-15.15b): the tile reflects the bridge host's
    // NetworkManager VPN view (the same view the Settings pane reads).
    const QVariantMap vpn = m_statusModel ? m_statusModel->vpn() : QVariantMap();
    // Mission Control and hot corners are compositor-native: the shell owns the
    // compositor mirror and the settingsd values, so it projects the tile's
    // summary locally (T-15.5b) instead of reading a services-layer host.
    const QVariantMap missionControl =
        m_settingsClient ? missionControlView(m_settingsClient->values()) : QVariantMap();
    // Lock Screen policy is session/compositor-native: the shell owns the
    // durable settingsd keys, so it projects the tile's summary locally
    // (T-15.8b) instead of reading a services-layer host.
    const QVariantMap lockPolicy =
        m_settingsClient ? lockPolicyView(m_settingsClient->values()) : QVariantMap();
    // Menu Bar configuration is shell-native: the shell owns the settingsd
    // keys and the bar, so it projects the tile's summary locally (T-15.9b).
    const QVariantMap menuBar =
        m_settingsClient ? menuBarView(m_settingsClient->values()) : QVariantMap();
    // Capture-only absence seam (T203): a deterministic list of adapter names
    // whose views are replaced with `unavailable`, so the visual matrix can
    // capture the absent-adapter state without stopping a real daemon. Never
    // set in a normal session.
    const QStringList absentAdapters = qEnvironmentVariable("DF_CONTROL_CENTER_ABSENT")
        .split(QLatin1Char(','), Qt::SkipEmptyParts);
    const auto absentView = [&absentAdapters](const QString &kind,
                                              const QVariantMap &view) {
        for (const QString &name : absentAdapters) {
            if (name.trimmed().compare(kind, Qt::CaseInsensitive) == 0) {
                return QVariantMap{{QStringLiteral("kind"), kind},
                                   {QStringLiteral("state"),
                                    QStringLiteral("unavailable")}};
            }
        }
        return view;
    };
    m_controlCenterItem->setProperty("wifi", absentView(QStringLiteral("wifi"), wifi));
    m_controlCenterItem->setProperty("audio", absentView(QStringLiteral("audio"), audio));
    m_controlCenterItem->setProperty("bluetooth",
                                     absentView(QStringLiteral("bluetooth"), bluetooth));
    m_controlCenterItem->setProperty("storage",
                                     absentView(QStringLiteral("storage"), storage));
    m_controlCenterItem->setProperty("input", absentView(QStringLiteral("input"), input));
    m_controlCenterItem->setProperty("battery",
                                     absentView(QStringLiteral("battery"), battery));
    m_controlCenterItem->setProperty("updates",
                                     absentView(QStringLiteral("updates"), updates));
    m_controlCenterItem->setProperty("accounts",
                                     absentView(QStringLiteral("accounts"), accounts));
    m_controlCenterItem->setProperty("printers",
                                     absentView(QStringLiteral("printers"), printers));
    m_controlCenterItem->setProperty("privacy",
                                     absentView(QStringLiteral("privacy"), privacy));
    m_controlCenterItem->setProperty("accessibility",
                                     absentView(QStringLiteral("accessibility"),
                                                accessibility));
    m_controlCenterItem->setProperty("vpn", absentView(QStringLiteral("vpn"), vpn));
    m_controlCenterItem->setProperty("missionControl", missionControl);
    m_controlCenterItem->setProperty("lockPolicy", lockPolicy);
    m_controlCenterItem->setProperty("menuBar", menuBar);
    m_controlCenterItem->setProperty("brightness", brightness);
    m_controlCenterItem->setProperty("focusPolicy", focus);
    m_controlCenterItem->setProperty("dark", dark);
    // T227: the Quick Settings pane's ordered module list and the
    // menu-bar-only set. The panel filters/orders its live modules from these
    // two settingsd keys; an empty list keeps the built-in order.
    m_controlCenterItem->setProperty(
        "quickSettingsModules",
        settingsValues.value(QStringLiteral("quickSettings.modules")));
    m_controlCenterItem->setProperty(
        "quickSettingsMenuBarOnly",
        settingsValues.value(QStringLiteral("quickSettings.menuBarOnly")));
    // The clipboard history list (T-13.5b); the panel gates it on
    // `clipboardAvailable`.
    m_controlCenterItem->setProperty("clipboardEntries",
                                     m_clipboard ? m_clipboard->entries() : QVariantList());
    // T225: the Wi-Fi radio write is live through the bridge host. Bluetooth
    // writes are live too (T-15.1b).
    m_controlCenterItem->setProperty("wifiWritable", true);
    m_controlCenterItem->setProperty("bluetoothWritable", true);
    // Storage writes (mount/unmount/eject) are live through the T-15.2b bridge
    // host, so its tile is live.
    m_controlCenterItem->setProperty("storageWritable", true);
}

void ShellController::updateControlCenterGeometry()
{
    if (!m_protocol)
        return;
    const QSize output = m_protocol->outputLogicalSize();
    if (output.isEmpty())
        return;
    const int available = output.height() - m_barHeight - kControlCenterTopGap;
    const int target = qMin(kControlCenterHeight, available);
    if (target <= 0 || target == m_controlCenterHeight)
        return;
    m_controlCenterHeight = target;
    m_protocol->setControlCenterSize(kControlCenterWidth, target);
}

void ShellController::onControlCenterConfigured(int width, int height, quint32)
{
    // The compositor's pre-layout configure is the full output; the panel's
    // own size follows. At 1.5x/2x the fixed 1164 px panel would run past the
    // bottom edge, so fit it below the menu bar and wait for the matching
    // configure (T-21.1b).
    updateControlCenterGeometry();
    if (width != kControlCenterWidth || height != m_controlCenterHeight)
        return;
    m_controlCenterWidth = width;
    // The surface exists now; apply the panel-sized input region for a pending
    // open and render the first frame.
    if (m_protocol)
        m_protocol->setControlCenterInputRegion(m_controlCenterWidth, m_controlCenterHeight);
    if (m_controlCenterPending)
        renderControlCenter();
}

void ShellController::onControlCenterPointerMoved(qreal x, qreal y)
{
    if (!m_controlCenterWindow)
        return;
    const QPointF p(x, y);
    sendChromePointer(m_controlCenterWindow, QEvent::MouseMove, p, Qt::NoButton,
                        m_controlCenterButtons);
    scheduleControlCenterRender();
}

void ShellController::onControlCenterPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_controlCenterWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    if (pressed)
        m_controlCenterButtons |= qtButton;
    else
        m_controlCenterButtons &= ~qtButton;
    const QPointF p(x, y);
    sendChromePointer(m_controlCenterWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p,
                        qtButton, m_controlCenterButtons);
    scheduleControlCenterRender();
}

void ShellController::onControlCenterPointerLeft()
{
    if (!m_controlCenterWindow)
        return;
    sendChromePointer(m_controlCenterWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_controlCenterButtons);
    scheduleControlCenterRender();
}

void ShellController::onControlCenterKeyboardFocused(bool focused)
{
    m_controlCenterKeyboardFocused = focused;
    // Losing the keyboard is the click-away dismissal: a click on a window or
    // the desktop moves chrome keyboard focus away from the panel.
    if (!focused && m_controlCenterOpen)
        hideControlCenter();
}

void ShellController::onBrightnessSetRequested(double level)
{
    if (!m_settingsClient)
        return;
    const double clamped = qBound(0.0, level, 1.0);
    // settingsd is the single owner (T-08); the `changed` connection forwards
    // the new value to the compositor via `applyDisplayPolicy`. The OSD
    // presents the change on the active output (T-11.4a).
    m_settingsClient->set(QStringLiteral("display.brightness"), clamped);
    showOsd(OsdModel::Kind::Brightness, clamped, false);
}

void ShellController::onWifiToggleRequested(bool enabled)
{
    // T225: the bridge host exposes the radio write; the host re-reads and
    // pushes the new view, so the shell keeps no local state.
    if (m_statusClient)
        m_statusClient->setWifiEnabled(enabled);
}

void ShellController::onWifiSettingsRequested()
{
    // T232: the menu-bar extra's Settings link lands on the Wi-Fi pane.
    openSettingsDeepLink(QStringLiteral("settings://pane/wifi"));
}

void ShellController::onBluetoothToggleRequested(bool enabled)
{
    // One explicit BlueZ write through the bridge host (T-15.1b). The host
    // re-reads and the pushed `PropertiesChanged` updates the tile; no local
    // state is kept.
    if (!m_statusClient)
        return;
    m_statusClient->setBluetoothPowered(enabled);
}

void ShellController::onBluetoothDeviceToggled(const QString &address, bool connected)
{
    if (!m_statusClient || address.isEmpty())
        return;
    m_statusClient->setBluetoothConnected(address, connected);
}

void ShellController::onBluetoothSettingsRequested()
{
    // T232: the menu-bar extra's Settings link lands on the Bluetooth pane.
    openSettingsDeepLink(QStringLiteral("settings://pane/bluetooth"));
}

void ShellController::onBluetoothDiscoveringToggleRequested(bool discovering)
{
    // One explicit StartDiscovery/StopDiscovery through the bridge host.
    if (m_statusClient)
        m_statusClient->setBluetoothDiscovering(discovering);
}

void ShellController::onBluetoothPairRequested(const QString &address)
{
    // Pairing is an explicit BlueZ action; the host re-reads after it.
    if (m_statusClient && !address.isEmpty())
        m_statusClient->pairBluetooth(address);
}

void ShellController::onStorageMountRequested(const QString &volumePath)
{
    // One explicit UDisks2 write through the bridge host (T-15.2b). The host
    // re-reads and the pushed property change updates the tile; no local state
    // is kept.
    if (m_statusClient && !volumePath.isEmpty())
        m_statusClient->mountStorage(volumePath);
}

void ShellController::onStorageUnmountRequested(const QString &volumePath)
{
    if (m_statusClient && !volumePath.isEmpty())
        m_statusClient->unmountStorage(volumePath);
}

void ShellController::onStorageEjectRequested(const QString &drivePath)
{
    if (m_statusClient && !drivePath.isEmpty())
        m_statusClient->ejectStorage(drivePath);
}

void ShellController::onStorageSettingsRequested()
{
    // Launching Settings on the Storage pane is T-16; the entry point is wired
    // and logs until then.
    qInfo() << "shell: Storage Settings requested (T-16)";
}

void ShellController::onSoundSettingsRequested()
{
    // T232: the menu-bar extra's Settings link lands on the Sound pane.
    openSettingsDeepLink(QStringLiteral("settings://pane/sound"));
}

void ShellController::onSinkSelected(int id)
{
    // T232: selecting an output device makes it the default sink through the
    // one bridge-host routing write; the host re-reads and pushes the view.
    if (m_statusClient)
        m_statusClient->setDefaultSink(id);
}

void ShellController::onKeyboardSettingsRequested()
{
    // Launching Settings on the Keyboard pane is T-16; the entry point is wired
    // and logs until then.
    qInfo() << "shell: Keyboard Settings requested (T-16)";
}

void ShellController::onMissionControlSettingsRequested()
{
    // Launching Settings on the Mission Control pane is T-16; the entry point
    // is wired and logs until then.
    qInfo() << "shell: Mission Control Settings requested (T-16)";
}

void ShellController::onBatterySettingsRequested()
{
    // T232: the menu-bar extra's Settings link lands on the Battery pane.
    openSettingsDeepLink(QStringLiteral("settings://pane/battery"));
}

void ShellController::onPowerProfileSelected(const QString &profile)
{
    // T232: one explicit power-profiles-daemon write through the bridge host.
    if (m_statusClient && !profile.isEmpty())
        m_statusClient->setPowerProfile(profile);
}

void ShellController::onLockScreenSettingsRequested()
{
    // Launching Settings on the Lock Screen pane is T-16; the entry point is
    // wired and logs until then.
    qInfo() << "shell: Lock Screen Settings requested (T-16)";
}

void ShellController::onMenuBarSettingsRequested()
{
    // Launching Settings on the Menu Bar pane is T-16; the entry point is
    // wired and logs until then.
    qInfo() << "shell: Menu Bar Settings requested (T-16)";
}

void ShellController::onUpdatesCheckRequested()
{
    // One explicit host-stack write through the bridge host (T-15.10b). The
    // host re-reads and the pushed view updates the tile; no local state is
    // kept.
    if (m_statusClient)
        m_statusClient->checkUpdates();
}

void ShellController::onUpdatesInstallRequested()
{
    if (m_statusClient)
        m_statusClient->installUpdates();
}

void ShellController::onUpdatesRebootRequested()
{
    if (m_statusClient)
        m_statusClient->rebootUpdates();
}

void ShellController::onUpdatesSettingsRequested()
{
    // Launching Settings on the General pane is T-16; the entry point is wired
    // and logs until then.
    qInfo() << "shell: General Settings requested (T-16)";
}

void ShellController::onUsersSettingsRequested()
{
    // Launching Settings on the Users & Groups pane is T-16; the entry point is
    // wired and logs until then.
    qInfo() << "shell: Users & Groups Settings requested (T-16)";
}

void ShellController::onPrintersSettingsRequested()
{
    // Launching Settings on the Printers & Scanners pane is T-16; the entry
    // point is wired and logs until then.
    qInfo() << "shell: Printers & Scanners Settings requested (T-16)";
}

void ShellController::onPrivacySettingsRequested()
{
    // Launching Settings on the Privacy & Security pane is T-16; the entry
    // point is wired and logs until then.
    qInfo() << "shell: Privacy & Security Settings requested (T-16)";
}

void ShellController::onAccessibilitySettingsRequested()
{
    // Launching Settings on the Accessibility pane is T-16; the entry point is
    // wired and logs until then.
    qInfo() << "shell: Accessibility Settings requested (T-16)";
}

void ShellController::onVpnSettingsRequested()
{
    // Launching Settings on the Network pane is T-16; the entry point is wired
    // and logs until then.
    qInfo() << "shell: Network Settings requested (T-16)";
}

void ShellController::onQuickSettingsSettingsRequested()
{
    // T232: the Quick Settings panel footer lands on the customization pane.
    openSettingsDeepLink(QStringLiteral("settings://pane/control-center"));
}

void ShellController::onFocusToggleRequested(bool enabled)
{
    // The notification service owns the policy (T-11.2a); the panel writes the
    // mode through the same seam as the menu-bar reflection. The service
    // echoes `Changed`, which re-reads `FocusPolicy()` and updates the tile.
    if (!m_notificationClient)
        return;
    m_notificationClient->setFocusMode(focusModeForToggle(enabled));
}

void ShellController::onFocusSettingsRequested()
{
    // Launching Settings on the Focus pane is T-16; the entry point is wired
    // and logs until then.
    qInfo() << "shell: Focus Settings requested (T-16)";
}

void ShellController::onDarkModeToggleRequested(bool dark)
{
    if (!m_settingsClient)
        return;
    // settingsd is the single owner (T-08); ThemeBinding consumes the same
    // client and flips the design-system Theme. The toggle writes an absolute
    // scheme, never `auto`.
    m_settingsClient->set(QStringLiteral("appearance.colorScheme"),
                          colorSchemeForDarkToggle(dark));
}

void ShellController::onAppearanceSettingsRequested()
{
    // Launching Settings on the Appearance pane is T-16; the entry point is
    // wired and logs until then.
    qInfo() << "shell: Appearance Settings requested (T-16)";
}

void ShellController::renderControlCenter()
{
    if (!m_controlCenterPending || !m_controlCenterWindow || !m_controlCenterItem)
        return;
    if (m_controlCenterWidth <= 0 || m_controlCenterHeight <= 0)
        return;
    prepareChromeScene(m_controlCenterWindow, m_controlCenterItem,
                       QSize(m_controlCenterWidth, m_controlCenterHeight),
                       QSize(m_controlCenterWidth, m_controlCenterHeight), m_chromeScale);
    // T203: the panel declares its outer plate, module capsules and expanded
    // lists as independently keyed material regions. Send them after the
    // layout pass so the rects follow the current scroll/expansion state; the
    // region list is commit-latched with the image below.
    if (m_protocol) {
        m_protocol->setControlCenterPanelRegions(
            m_controlCenterItem->property("materialRegions").toList());
    }
    const QSize logical(m_controlCenterWidth, m_controlCenterHeight);
    commitChromeScene(m_controlCenterWindow,
                      {{QStringLiteral("control-center"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitControlCenterImage(image, logical);
                        }}});
}

void ShellController::scheduleControlCenterRender()
{
    if (m_controlCenterRenderPending)
        return;
    m_controlCenterRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_controlCenterRenderPending = false;
        renderControlCenter();
    });
}

// --- OSD overlay (T-11.4a) --------------------------------------------------

void ShellController::showOsd(OsdModel::Kind kind, double value, bool muted)
{
    if (!m_osdItem || !m_osdWindow || !m_protocol)
        return;
    // A fullscreen surface owns the output: suppress the brief overlay so it
    // never disturbs a presentation. The model keeps the suppression policy so
    // it is testable without the protocol.
    m_osd.setFullscreen(m_protocol->fullscreenOverlayActive());
    m_osd.present(kind, value, muted, QDateTime::currentMSecsSinceEpoch());
    if (!m_osd.visible()) {
        hideOsd();
        return;
    }
    applyOsdData();
    m_osdActive = true;
    if (m_osdWidth > 0 && m_osdHeight > 0)
        renderOsd();
    if (m_osdTimer && !m_osdTimer->isActive())
        m_osdTimer->start();
}

void ShellController::applyOsdData()
{
    if (!m_osdItem)
        return;
    m_osdItem->setProperty("kind", m_osd.kindName());
    m_osdItem->setProperty("value", m_osd.value());
    m_osdItem->setProperty("muted", m_osd.muted());
    m_osdItem->setProperty("fade", m_osd.fade(QDateTime::currentMSecsSinceEpoch(),
                                              m_compositorPolicy.reducedMotion));
}

void ShellController::hideOsd()
{
    m_osdActive = false;
    if (m_osdTimer)
        m_osdTimer->stop();
    if (m_protocol)
        m_protocol->hideOsd();
}

void ShellController::onOsdTick()
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    if (!m_osd.tick(now)) {
        hideOsd();
        return;
    }
    m_osdItem->setProperty("fade", m_osd.fade(now, m_compositorPolicy.reducedMotion));
    renderOsd();
}

void ShellController::onOsdDismissed()
{
    // T-11.4b: a keyboard/AT-SPI dismissal, routed through the model's `hide`
    // so it shares the auto-dismiss path (and stops the visible-only timer).
    if (m_osd.visible())
        m_osd.hide();
    hideOsd();
}

void ShellController::onOsdConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's requested size; skip it (the same
    // rule as the menu bar, banner, and panel).
    if (width != kOsdSurfaceWidth || height != kOsdSurfaceHeight) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout OSD configure %dx%d\n", width,
                height);
        return;
    }
    m_osdWidth = width;
    m_osdHeight = height;
    fprintf(stderr, "dragonfruit-shell: OSD configured %dx%d\n", width, height);
    if (m_osdActive && !m_osdMapped)
        renderOsd();
}

void ShellController::renderOsd()
{
    if (!m_osdActive || !m_osdWindow || !m_osdItem)
        return;
    if (m_osdWidth <= 0 || m_osdHeight <= 0)
        return;
    prepareChromeScene(m_osdWindow, m_osdItem, QSize(m_osdWidth, m_osdHeight),
                       QSize(m_osdWidth, m_osdHeight), m_chromeScale);
    // Declare the visible card so the compositor's backdrop frosts the card,
    // not the transparent shadow margin (T-20.3). The card never moves, so
    // only a changed integer rect is sent.
    const QVariantMap card = m_osdItem->property("panelRect").toMap();
    const QRect panel(qFloor(card.value(QStringLiteral("x")).toReal()),
                      qFloor(card.value(QStringLiteral("y")).toReal()),
                      qCeil(card.value(QStringLiteral("w")).toReal()),
                      qCeil(card.value(QStringLiteral("h")).toReal()));
    if (panel.width() > 0 && panel.height() > 0
            && (!m_osdPanelRectValid || panel != m_osdPanelRect) && m_protocol) {
        if (m_protocol->setOsdPanelRect(panel.x(), panel.y(),
                                        panel.width(), panel.height())) {
            m_osdPanelRect = panel;
            m_osdPanelRectValid = true;
        }
    }
    const QSize logical(m_osdWidth, m_osdHeight);
    const bool committed =
        commitChromeScene(m_osdWindow,
                          {{QStringLiteral("osd"), logical, QRect(),
                            [this, logical](const QImage &image) {
                                return m_protocol->commitOsdImage(image, logical);
                            }}});
    if (committed)
        m_osdMapped = true;
}

// --- FileChooser picker (T-13.2b) -------------------------------------------

void ShellController::startChooserFixture()
{
    if (!m_chooser)
        return;
    const QString value = qEnvironmentVariable("DF_CHOOSER_FIXTURE");
    QString folder = value;
    if (folder.isEmpty() || folder == QLatin1String("1"))
        folder = QDir::homePath();
    m_chooser->begin(QStringLiteral("/org/freedesktop/portal/desktop/request/fixture"),
                     QStringLiteral("open"), tr("Open File"), QString(), false, false,
                     QUrl::fromLocalFile(folder).toString());
}

void ShellController::onChooserStarted()
{
    showChooser();
}

void ShellController::onChooserFinished(bool completed)
{
    Q_UNUSED(completed);
    hideChooser();
}

void ShellController::onChooserChanged()
{
    if (!m_chooserActive)
        return;
    applyChooserData();
    scheduleChooserRender();
}

void ShellController::showChooser()
{
    if (!m_chooserItem)
        return;
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    m_chooserActive = true;
    m_chooserPending = true;
    applyChooserData();
    if (m_chooserWidth > 0 && m_chooserHeight > 0)
        renderChooser();
    // Take active focus so Escape/Return reach the dialog.
    m_chooserItem->forceActiveFocus();
    if (m_protocol)
        m_protocol->setChooserInputRegion(m_chooserWidth, m_chooserHeight);
}

void ShellController::hideChooser()
{
    m_chooserActive = false;
    m_chooserPending = false;
    m_chooserButtons = Qt::NoButton;
    if (m_protocol) {
        m_protocol->setChooserInputRegion(0, 0);
        m_protocol->hideChooser();
    }
}

void ShellController::applyChooserData()
{
    if (!m_chooserItem || !m_chooser)
        return;
    m_chooserItem->setProperty("title", m_chooser->title());
    m_chooserItem->setProperty("kind", m_chooser->kind());
    m_chooserItem->setProperty("acceptLabel", m_chooser->acceptLabel());
    m_chooserItem->setProperty("currentUri", m_chooser->currentUri());
    m_chooserItem->setProperty("currentLabel", m_chooser->currentLabel());
    m_chooserItem->setProperty("entries", m_chooser->entries());
    m_chooserItem->setProperty("selectedIndex", m_chooser->selectedIndex());
    m_chooserItem->setProperty("saveName", m_chooser->saveName());
    m_chooserItem->setProperty("directoryMode", m_chooser->directoryMode());
    m_chooserItem->setProperty("multiple", m_chooser->multiple());
    m_chooserItem->setProperty("errorText", m_chooser->error());
}

void ShellController::renderChooser()
{
    if (!m_chooserActive || !m_chooserPending || !m_chooserWindow || !m_chooserItem)
        return;
    if (m_chooserWidth <= 0 || m_chooserHeight <= 0)
        return;
    prepareChromeScene(m_chooserWindow, m_chooserItem, QSize(m_chooserWidth, m_chooserHeight),
                       QSize(m_chooserWidth, m_chooserHeight), m_chromeScale);
    const QSize logical(m_chooserWidth, m_chooserHeight);
    commitChromeScene(m_chooserWindow,
                      {{QStringLiteral("chooser"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitChooserImage(image, logical);
                        }}});
}

void ShellController::scheduleChooserRender()
{
    if (m_chooserRenderPending)
        return;
    m_chooserRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_chooserRenderPending = false;
        renderChooser();
    });
}

void ShellController::onChooserConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's requested size; skip it (the same
    // rule as the menu bar, banner, panel, and OSD).
    if (width != kChooserWidth || height != kChooserHeight) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout chooser configure %dx%d\n", width,
                height);
        return;
    }
    m_chooserWidth = width;
    m_chooserHeight = height;
    if (m_protocol)
        m_protocol->setChooserInputRegion(m_chooserWidth, m_chooserHeight);
    if (m_chooserPending)
        renderChooser();
}

void ShellController::onChooserPointerMoved(qreal x, qreal y)
{
    if (!m_chooserWindow)
        return;
    const QPointF p(x, y);
    sendChromePointer(m_chooserWindow, QEvent::MouseMove, p, Qt::NoButton, m_chooserButtons);
    scheduleChooserRender();
}

void ShellController::onChooserPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_chooserWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    if (pressed)
        m_chooserButtons |= qtButton;
    else
        m_chooserButtons &= ~qtButton;
    const QPointF p(x, y);
    sendChromePointer(m_chooserWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p,
                        qtButton, m_chooserButtons);
    scheduleChooserRender();
}

void ShellController::onChooserPointerLeft()
{
    if (!m_chooserWindow)
        return;
    sendChromePointer(m_chooserWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_chooserButtons);
    scheduleChooserRender();
}

void ShellController::onChooserKeyboardFocused(bool focused)
{
    // Losing the keyboard is the click-away dismissal: a click on a window or
    // the desktop moves chrome keyboard focus away from the dialog. Cancelling
    // tells the portal the caller got no selection.
    if (!focused && m_chooserActive && m_chooser)
        m_chooser->cancel();
}

void ShellController::onChooserKeyEvent(quint32 key, bool pressed)
{
    if (!m_chooserWindow || !m_chooserActive)
        return;
    sendChromeKeyEvent(m_chooserWindow, key, pressed,
                       m_protocol ? m_protocol->keyboardModifiers() : 0);
    scheduleChooserRender();
}

void ShellController::onChooserSelectionChanged(int index)
{
    if (m_chooser)
        m_chooser->select(index);
}

void ShellController::onChooserEntryActivated(int index)
{
    if (m_chooser)
        m_chooser->activate(index);
}

void ShellController::onChooserBrowseRequested(const QString &uri)
{
    if (m_chooser)
        m_chooser->browse(uri);
}

void ShellController::onChooserUpRequested()
{
    if (m_chooser)
        m_chooser->goUp();
}

void ShellController::onChooserNameEdited(const QString &name)
{
    if (m_chooser)
        m_chooser->setSaveName(name);
}

void ShellController::onChooserAccepted()
{
    if (m_chooser)
        m_chooser->accept();
}

void ShellController::onChooserCancelled()
{
    if (m_chooser)
        m_chooser->cancel();
}

// --- screenshot selection overlay (T-13.3a) ---------------------------------

void ShellController::startScreenshot(const QString &mode)
{
    if (!m_screenshot)
        return;
    // The selection overlay is modal; retire the ambient Center first.
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    m_screenshot->beginLocal(mode);
}

void ShellController::startScreenshotFixture()
{
    const QString value = qEnvironmentVariable("DF_SCREENSHOT_FIXTURE");
    QString mode = value;
    if (mode.isEmpty() || mode == QLatin1String("1"))
        mode = QStringLiteral("region");
    startScreenshot(mode);
}

void ShellController::onScreenshotStarted()
{
    showScreenshot();
}

void ShellController::onScreenshotFinished(bool completed)
{
    Q_UNUSED(completed);
    hideScreenshot();
}

void ShellController::onScreenshotChanged()
{
    if (!m_screenshotActive)
        return;
    applyScreenshotData();
    scheduleScreenshotRender();
}

void ShellController::onScreenshotCaptureRequested(const QString &mode, int x, int y, int width,
                                                   int height)
{
    // The selection is real: ask the compositor to render the frame itself
    // (portal-presenter-only capture; T-13.3b) into a temporary PNG. The
    // overlay is unmapped first so it is not part of the capture.
    hideScreenshot();
    if (!m_protocol) {
        onScreenshotFailed(tr("The compositor capture path is unavailable."));
        return;
    }
    m_capturePath = QDir::tempPath() + QStringLiteral("/dragonfruit-screenshot-")
                    + QString::number(QCoreApplication::applicationPid()) + QLatin1Char('-')
                    + QString::number(QDateTime::currentMSecsSinceEpoch())
                    + QStringLiteral(".png");
    // The overlay selects in logical surface coordinates; the compositor
    // renders and crops in physical output pixels. Convert before asking for
    // the frame, or a fullscreen capture at 1.5x would crop the top-left
    // 1280x800 of the 1920x1200 output (T-21.1b).
    const QRect physical =
        ChromeScale::bufferRect(QRect(x, y, width, height), m_protocol->outputScale());
    if (!m_protocol->captureScreenshot(m_capturePath, physical.x(), physical.y(),
                                       physical.width(), physical.height(), mode))
        onScreenshotFailed(tr("The compositor capture path is unavailable."));
}

void ShellController::onScreenshotSaved(const QString &path)
{
    const QImage image(path);
    if (image.isNull()) {
        onScreenshotFailed(tr("The captured image could not be read."));
        return;
    }
    // Save into Pictures/Screenshots and copy to the clipboard (T-13.3b).
    const ScreenshotWriter::Result result = m_screenshotWriter->deliver(image);
    if (!path.isEmpty())
        QFile::remove(path);
    m_capturePath.clear();
    if (!result.saved) {
        onScreenshotFailed(tr("The captured image could not be saved."));
        return;
    }
    // A portal request is answered with the saved file URI; the desktop
    // shortcut has no request (its bridge already finished).
    if (m_screenshot && m_screenshot->active())
        m_screenshot->complete(QUrl::fromLocalFile(result.path).toString());
    fprintf(stderr, "dragonfruit-shell: screenshot saved %s (copy=%s)\n",
            qPrintable(result.path), result.copied ? "yes" : "no");
}

void ShellController::onScreenshotFailed(const QString &reason)
{
    if (!reason.isEmpty())
        qWarning("dragonfruit-shell: screenshot failed: %s", qPrintable(reason));
    if (!m_capturePath.isEmpty()) {
        QFile::remove(m_capturePath);
        m_capturePath.clear();
    }
    if (m_screenshot && m_screenshot->active())
        m_screenshot->cancel();
}

void ShellController::showScreenshot()
{
    if (!m_screenshotItem)
        return;
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    m_screenshotActive = true;
    m_screenshotPending = true;
    applyScreenshotData();
    if (m_screenshotWidth > 0 && m_screenshotHeight > 0)
        renderScreenshot();
    // Take active focus so Escape/Return reach the overlay.
    m_screenshotItem->forceActiveFocus();
    if (m_protocol)
        m_protocol->setScreenshotInputRegion(m_screenshotWidth, m_screenshotHeight);
}

void ShellController::hideScreenshot()
{
    m_screenshotActive = false;
    m_screenshotPending = false;
    m_screenshotButtons = Qt::NoButton;
    if (m_protocol) {
        m_protocol->setScreenshotInputRegion(0, 0);
        m_protocol->hideScreenshot();
    }
}

void ShellController::applyScreenshotData()
{
    if (!m_screenshotItem || !m_screenshot)
        return;
    m_screenshotItem->setProperty("mode", m_screenshot->mode());
}

void ShellController::renderScreenshot()
{
    if (!m_screenshotActive || !m_screenshotPending || !m_screenshotWindow
        || !m_screenshotItem)
        return;
    if (m_screenshotWidth <= 0 || m_screenshotHeight <= 0)
        return;
    prepareChromeScene(m_screenshotWindow, m_screenshotItem, QSize(m_screenshotWidth, m_screenshotHeight),
                       QSize(m_screenshotWidth, m_screenshotHeight), m_chromeScale);
    const QSize logical(m_screenshotWidth, m_screenshotHeight);
    commitChromeScene(m_screenshotWindow,
                      {{QStringLiteral("screenshot"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitScreenshotImage(image, logical);
                        }}});
}

void ShellController::scheduleScreenshotRender()
{
    if (m_screenshotRenderPending)
        return;
    m_screenshotRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_screenshotRenderPending = false;
        renderScreenshot();
    });
}

void ShellController::onScreenshotConfigured(int width, int height, quint32)
{
    // A full-output overlay is configured to the output size; a transient
    // pre-layout configure may arrive first and is ignored until it is
    // non-degenerate.
    if (width <= 0 || height <= 0)
        return;
    m_screenshotWidth = width;
    m_screenshotHeight = height;
    if (m_protocol)
        m_protocol->setScreenshotInputRegion(m_screenshotWidth, m_screenshotHeight);
    if (m_screenshotPending)
        renderScreenshot();
}

void ShellController::onScreenshotPointerMoved(qreal x, qreal y)
{
    if (!m_screenshotWindow)
        return;
    const QPointF p(x, y);
    sendChromePointer(m_screenshotWindow, QEvent::MouseMove, p, Qt::NoButton, m_screenshotButtons);
    scheduleScreenshotRender();
}

void ShellController::onScreenshotPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_screenshotWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    if (pressed)
        m_screenshotButtons |= qtButton;
    else
        m_screenshotButtons &= ~qtButton;
    const QPointF p(x, y);
    sendChromePointer(m_screenshotWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p,
                        qtButton, m_screenshotButtons);
    scheduleScreenshotRender();
}

void ShellController::onScreenshotPointerLeft()
{
    if (!m_screenshotWindow)
        return;
    sendChromePointer(m_screenshotWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_screenshotButtons);
    scheduleScreenshotRender();
}

void ShellController::onScreenshotKeyboardFocused(bool focused)
{
    // Losing the keyboard is the click-away dismissal: cancelling leaves the
    // desktop untouched.
    if (!focused && m_screenshotActive && m_screenshot)
        m_screenshot->cancel();
}

void ShellController::onScreenshotKeyEvent(quint32 key, bool pressed)
{
    if (!m_screenshotWindow || !m_screenshotActive)
        return;
    sendChromeKeyEvent(m_screenshotWindow, key, pressed,
                       m_protocol ? m_protocol->keyboardModifiers() : 0);
    scheduleScreenshotRender();
}

// --- ScreenCast source picker (T-13.4a) -------------------------------------

void ShellController::startScreenCastFixture()
{
    if (!m_screencast)
        return;
    const QString value = qEnvironmentVariable("DF_SCREENCAST_FIXTURE");
    const bool monitors = value.isEmpty() || value == QLatin1String("1")
            || value == QLatin1String("both") || value == QLatin1String("monitors");
    const bool windows = value.isEmpty() || value == QLatin1String("1")
            || value == QLatin1String("both") || value == QLatin1String("windows");
    uint types = 0;
    if (monitors)
        types |= 1;
    if (windows)
        types |= 2;
    m_screencast->begin(QStringLiteral("/org/freedesktop/portal/desktop/request/fixture"),
                        QStringLiteral("/org/freedesktop/portal/desktop/session/fixture"),
                        QStringLiteral("org.example.App"), types, windows, 1);

    // A deterministic source list so the capture is stable regardless of the
    // live session's windows (the fixture is capture-only).
    QVariantList sources;
    if (monitors) {
        sources.append(QVariantMap{
            {QStringLiteral("id"), QStringLiteral("monitor:DP-1")},
            {QStringLiteral("kind"), QStringLiteral("monitor")},
            {QStringLiteral("label"), tr("Built-in Display")},
            {QStringLiteral("detail"), QStringLiteral("1920 × 1080")},
        });
    }
    if (windows) {
        sources.append(QVariantMap{
            {QStringLiteral("id"), QStringLiteral("window:42")},
            {QStringLiteral("kind"), QStringLiteral("window")},
            {QStringLiteral("label"), tr("Settings")},
            {QStringLiteral("detail"), QStringLiteral("org.dragonfruit.Settings")},
        });
        sources.append(QVariantMap{
            {QStringLiteral("id"), QStringLiteral("window:43")},
            {QStringLiteral("kind"), QStringLiteral("window")},
            {QStringLiteral("label"), tr("Files")},
            {QStringLiteral("detail"), QStringLiteral("org.dragonfruit.Files")},
        });
    }
    m_screencast->setSources(sources);
}

void ShellController::onScreenCastStarted()
{
    if (!m_screencast)
        return;
    // Feed the picker the compositor's monitor/window projection, filtered by
    // the request's source-type bitmask. The fixture overwrites this with a
    // deterministic list immediately after `begin` returns.
    if (m_screencast->active() && m_protocol) {
        const uint types = m_screencast->types();
        QVariantList filtered;
        for (const QVariant &entry : m_protocol->screencastSources()) {
            const QVariantMap source = entry.toMap();
            const bool monitor =
                source.value(QStringLiteral("kind")).toString() == QLatin1String("monitor");
            if ((monitor && (types & 1u)) || (!monitor && (types & 2u)))
                filtered.append(source);
        }
        m_screencast->setSources(filtered);
    }
    showScreenCast();
}

void ShellController::onScreenCastFinished(bool completed)
{
    Q_UNUSED(completed);
    hideScreenCast();
}

void ShellController::onScreenCastChanged()
{
    if (!m_screencastActive)
        return;
    applyScreenCastData();
    scheduleScreenCastRender();
}

void ShellController::onScreenCastSourceToggled(const QString &id)
{
    if (m_screencast)
        m_screencast->select(id);
}

void ShellController::onScreenCastAccepted()
{
    if (m_screencast)
        m_screencast->accept();
}

void ShellController::onScreenCastCancelled()
{
    if (m_screencast)
        m_screencast->cancel();
}

void ShellController::showScreenCast()
{
    if (!m_screencastItem)
        return;
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    m_screencastActive = true;
    m_screencastPending = true;
    applyScreenCastData();
    if (m_screencastWidth > 0 && m_screencastHeight > 0)
        renderScreenCast();
    // Take active focus so Escape/Return reach the picker.
    m_screencastItem->forceActiveFocus();
    if (m_protocol)
        m_protocol->setScreenCastInputRegion(m_screencastWidth, m_screencastHeight);
}

void ShellController::hideScreenCast()
{
    m_screencastActive = false;
    m_screencastPending = false;
    m_screencastButtons = Qt::NoButton;
    if (m_protocol) {
        m_protocol->setScreenCastInputRegion(0, 0);
        m_protocol->hideScreenCast();
    }
}

void ShellController::applyScreenCastData()
{
    if (!m_screencastItem || !m_screencast)
        return;
    m_screencastItem->setProperty("sources", m_screencast->sources());
    m_screencastItem->setProperty("multiple", m_screencast->multiple());
    m_screencastItem->setProperty("appId", m_screencast->appId());
    m_screencastItem->setProperty("errorText", m_screencast->error());
    // The portal's negotiated stream mode: the stills fallback is named in the
    // picker rather than discovered only when no frames arrive (T-13.4b).
    m_screencastItem->setProperty("streamNote", m_screencast->streamNote());
}

void ShellController::renderScreenCast()
{
    if (!m_screencastActive || !m_screencastPending || !m_screencastWindow
        || !m_screencastItem)
        return;
    if (m_screencastWidth <= 0 || m_screencastHeight <= 0)
        return;
    prepareChromeScene(m_screencastWindow, m_screencastItem, QSize(m_screencastWidth, m_screencastHeight),
                       QSize(m_screencastWidth, m_screencastHeight), m_chromeScale);
    const QSize logical(m_screencastWidth, m_screencastHeight);
    commitChromeScene(m_screencastWindow,
                      {{QStringLiteral("screencast"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitScreenCastImage(image, logical);
                        }}});
}

void ShellController::scheduleScreenCastRender()
{
    if (m_screencastRenderPending)
        return;
    m_screencastRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_screencastRenderPending = false;
        renderScreenCast();
    });
}

void ShellController::onScreenCastConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's requested size; skip it (the same
    // rule as the menu bar, banner, panel, chooser, and OSD).
    if (width != kScreenCastWidth || height != kScreenCastHeight) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout screencast configure %dx%d\n",
                width, height);
        return;
    }
    m_screencastWidth = width;
    m_screencastHeight = height;
    if (m_protocol)
        m_protocol->setScreenCastInputRegion(m_screencastWidth, m_screencastHeight);
    if (m_screencastPending)
        renderScreenCast();
}

void ShellController::onScreenCastPointerMoved(qreal x, qreal y)
{
    if (!m_screencastWindow)
        return;
    const QPointF p(x, y);
    sendChromePointer(m_screencastWindow, QEvent::MouseMove, p, Qt::NoButton, m_screencastButtons);
    scheduleScreenCastRender();
}

void ShellController::onScreenCastPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_screencastWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    if (pressed)
        m_screencastButtons |= qtButton;
    else
        m_screencastButtons &= ~qtButton;
    const QPointF p(x, y);
    sendChromePointer(m_screencastWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p,
                        qtButton, m_screencastButtons);
    scheduleScreenCastRender();
}

void ShellController::onScreenCastPointerLeft()
{
    if (!m_screencastWindow)
        return;
    sendChromePointer(m_screencastWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_screencastButtons);
    scheduleScreenCastRender();
}

void ShellController::onScreenCastKeyboardFocused(bool focused)
{
    // Losing the keyboard is the click-away dismissal: a click on a window or
    // the desktop moves chrome keyboard focus away from the picker. Cancelling
    // tells the portal the caller got no selection.
    if (!focused && m_screencastActive && m_screencast)
        m_screencast->cancel();
}

void ShellController::onScreenCastKeyEvent(quint32 key, bool pressed)
{
    if (!m_screencastWindow || !m_screencastActive)
        return;
    sendChromeKeyEvent(m_screencastWindow, key, pressed,
                       m_protocol ? m_protocol->keyboardModifiers() : 0);
    scheduleScreenCastRender();
}

// --- polkit authentication dialog (T-13.6) ----------------------------------

void ShellController::startPolkitFixture()
{
    if (!m_polkit)
        return;
    const QString value = qEnvironmentVariable("DF_POLKIT_FIXTURE");
    QString message = value;
    if (message.isEmpty() || message == QLatin1String("1")) {
        message = tr("Authentication is required to manage system services or other units.");
    }
    QVariantList details;
    details.append(QVariantMap{ { QStringLiteral("key"), QStringLiteral("polkit.subject-pid") },
                                { QStringLiteral("value"),
                                  QString::number(QCoreApplication::applicationPid()) } });
    details.append(QVariantMap{ { QStringLiteral("key"), QStringLiteral("polkit.caller-pid") },
                                { QStringLiteral("value"),
                                  QString::number(QCoreApplication::applicationPid()) } });
    m_polkit->presentLocal(QStringLiteral("org.freedesktop.systemd1.manage-units"), message,
                           qEnvironmentVariable("USER", QStringLiteral("user")), details);
}

void ShellController::onPolkitStarted()
{
    showPolkit();
}

void ShellController::onPolkitFinished(bool success)
{
    Q_UNUSED(success);
    hidePolkit();
}

void ShellController::onPolkitChanged()
{
    if (!m_polkitActive)
        return;
    applyPolkitData();
    schedulePolkitRender();
}

void ShellController::onPolkitSubmitted()
{
    if (!m_polkit || m_polkitResponse.isEmpty())
        return;
    const QString response = m_polkitResponse;
    m_polkitResponse.clear();
    if (m_polkitItem) {
        m_polkitItem->setProperty("responseText", QString());
        m_polkitItem->setProperty("responseLength", 0);
    }
    m_polkit->respond(response);
    schedulePolkitRender();
}

void ShellController::onPolkitCancelled()
{
    if (m_polkit)
        m_polkit->cancel();
}

void ShellController::showPolkit()
{
    if (!m_polkitItem)
        return;
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    m_polkitActive = true;
    m_polkitPending = true;
    m_polkitResponse.clear();
    applyPolkitData();
    if (m_polkitWidth > 0 && m_polkitHeight > 0)
        renderPolkit();
    // Take active focus so Escape and the password keys reach the dialog.
    m_polkitItem->forceActiveFocus();
    if (m_protocol)
        m_protocol->setPolkitInputRegion(m_polkitWidth, m_polkitHeight);
}

void ShellController::hidePolkit()
{
    m_polkitActive = false;
    m_polkitPending = false;
    m_polkitButtons = Qt::NoButton;
    m_polkitResponse.clear();
    if (m_polkitItem) {
        m_polkitItem->setProperty("responseText", QString());
        m_polkitItem->setProperty("responseLength", 0);
    }
    if (m_protocol) {
        m_protocol->setPolkitInputRegion(0, 0);
        m_protocol->hidePolkit();
    }
}

void ShellController::applyPolkitData()
{
    if (!m_polkitItem || !m_polkit)
        return;
    m_polkitItem->setProperty("message", m_polkit->message());
    m_polkitItem->setProperty("actionId", m_polkit->actionId());
    m_polkitItem->setProperty("identityLabel", m_polkit->identityLabel());
    m_polkitItem->setProperty("details", m_polkit->details());
    m_polkitItem->setProperty("prompt", m_polkit->prompt());
    m_polkitItem->setProperty("promptEcho", m_polkit->promptEcho());
    m_polkitItem->setProperty("info", m_polkit->info());
    m_polkitItem->setProperty("errorText", m_polkit->errorText());
    m_polkitItem->setProperty("busy", m_polkit->isBusy());
    m_polkitItem->setProperty("responseText", m_polkitResponse);
    m_polkitItem->setProperty("responseLength", m_polkitResponse.size());
}

void ShellController::renderPolkit()
{
    if (!m_polkitActive || !m_polkitPending || !m_polkitWindow || !m_polkitItem)
        return;
    if (m_polkitWidth <= 0 || m_polkitHeight <= 0)
        return;
    prepareChromeScene(m_polkitWindow, m_polkitItem, QSize(m_polkitWidth, m_polkitHeight),
                       QSize(m_polkitWidth, m_polkitHeight), m_chromeScale);
    const QSize logical(m_polkitWidth, m_polkitHeight);
    commitChromeScene(m_polkitWindow,
                      {{QStringLiteral("polkit"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitPolkitImage(image, logical);
                        }}});
}

void ShellController::schedulePolkitRender()
{
    if (m_polkitRenderPending)
        return;
    m_polkitRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_polkitRenderPending = false;
        renderPolkit();
    });
}

void ShellController::onPolkitConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's requested size; skip it (the same
    // rule as the menu bar, banner, panel, chooser, screencast, and OSD).
    if (width != kPolkitWidth || height != kPolkitHeight) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout polkit configure %dx%d\n",
                width, height);
        return;
    }
    m_polkitWidth = width;
    m_polkitHeight = height;
    if (m_protocol)
        m_protocol->setPolkitInputRegion(m_polkitWidth, m_polkitHeight);
    if (m_polkitPending)
        renderPolkit();
}

void ShellController::onPolkitPointerMoved(qreal x, qreal y)
{
    if (!m_polkitWindow)
        return;
    const QPointF p(x, y);
    sendChromePointer(m_polkitWindow, QEvent::MouseMove, p, Qt::NoButton, m_polkitButtons);
    schedulePolkitRender();
}

void ShellController::onPolkitPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_polkitWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    if (pressed)
        m_polkitButtons |= qtButton;
    else
        m_polkitButtons &= ~qtButton;
    const QPointF p(x, y);
    sendChromePointer(m_polkitWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p,
                        qtButton, m_polkitButtons);
    schedulePolkitRender();
}

void ShellController::onPolkitPointerLeft()
{
    if (!m_polkitWindow)
        return;
    sendChromePointer(m_polkitWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_polkitButtons);
    schedulePolkitRender();
}

void ShellController::onPolkitKeyboardFocused(bool focused)
{
    // Losing the keyboard is the click-away dismissal: declining is the
    // fail-closed answer, so a privileged request never sits unattended.
    if (!focused && m_polkitActive && m_polkit)
        m_polkit->cancel();
}

void ShellController::onPolkitKeyEvent(quint32 key, bool pressed, bool shift)
{
    if (!m_polkitActive || !m_polkitItem || !pressed)
        return;
    // A PAM attempt in flight owns the field: keys are ignored until it
    // returns, so a password cannot be edited mid-authentication.
    if (m_polkitItem->property("busy").toBool())
        return;

    const dragonfruit::LockKey mapped = dragonfruit::lockKeyFromEvdev(key, shift);
    switch (mapped.kind) {
    case dragonfruit::LockKeyKind::Escape:
        if (m_polkit)
            m_polkit->cancel();
        return;
    case dragonfruit::LockKeyKind::Backspace:
        m_polkitResponse.chop(1);
        break;
    case dragonfruit::LockKeyKind::Return: {
        if (m_polkitResponse.isEmpty())
            return;
        onPolkitSubmitted();
        return;
    }
    case dragonfruit::LockKeyKind::Character:
        m_polkitResponse.append(mapped.character);
        break;
    case dragonfruit::LockKeyKind::Ignore:
        return;
    }
    m_polkitItem->setProperty("responseText", m_polkitResponse);
    m_polkitItem->setProperty("responseLength", m_polkitResponse.size());
    schedulePolkitRender();
}

// --- session lock (T-12.3a protocol/UI, T-12.3b authentication) -------------

void ShellController::onSessionLocked()
{
    // Lock is a hard close for ambient panels: no notification history or
    // input region may remain above the fail-secure lock surface.
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("lock"));
    // A lock clears the clipboard history by default; pinned entries survive
    // (T-13.5b, ADR 0082).
    if (m_clipboard && m_clipboard->clearOnLock())
        m_clipboard->clearUnpinned();
    showLockScreen();
}

// --- clipboard history (T-13.5b) -------------------------------------------

QString ShellController::clipboardHistoryPath() const
{
    const QString base = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    if (base.isEmpty())
        return QString();
    return base + QStringLiteral("/clipboard-history.json");
}

void ShellController::loadClipboardHistory()
{
    const QString path = clipboardHistoryPath();
    if (path.isEmpty())
        return;
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        return;
    m_clipboard->fromJson(file.readAll());
}

void ShellController::saveClipboardHistory()
{
    const QString path = clipboardHistoryPath();
    if (path.isEmpty() || !m_clipboard)
        return;
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate))
        return;
    file.write(m_clipboard->toJson());
}

void ShellController::onClipboardObserved(const QStringList &mimes,
                                          const QMap<QString, QByteArray> &payloads)
{
    if (!m_clipboard)
        return;
    const bool stored = m_clipboard->observe(mimes, payloads);
    fprintf(stderr, "dragonfruit-shell: clipboard selection observed (%d mime(s), %s)\n",
            int(payloads.size()), stored ? "stored" : "skipped");
}

void ShellController::onClipboardHistoryChanged()
{
    saveClipboardHistory();
    if (m_controlCenterOpen)
        applyControlCenterData();
}

void ShellController::onClipboardCopyRequested(int index)
{
    if (!m_clipboard || !m_protocol)
        return;
    QStringList mimes;
    QMap<QString, QByteArray> payloads;
    if (!m_clipboard->restore(index, &mimes, &payloads))
        return;
    // Re-serve the observed entry through the data-control device; this is the
    // only time the shell takes selection ownership (ADR 0082).
    m_protocol->offerClipboard(mimes, payloads);
}

void ShellController::onClipboardPinToggled(int index, bool pinned)
{
    if (m_clipboard)
        m_clipboard->setPinned(index, pinned);
}

void ShellController::onClipboardClearRequested()
{
    if (m_clipboard)
        m_clipboard->clear();
}

void ShellController::onSessionFinished()
{
    // The compositor dropped the lock on its own (it should not while locked).
    teardownLockScreen();
}

void ShellController::onLockSurfaceConfigured(quintptr lockSurfaceId, int width, int height,
                                              quint32)
{
    if (width <= 0 || height <= 0)
        return;
    m_lockSurfaceSizes.insert(lockSurfaceId, QSize(width, height));
    // Render now if the lock is already confirmed; otherwise `showLockScreen`
    // paints every surface the moment it is.
    if (m_lockActive)
        renderLockSurface(lockSurfaceId);
}

void ShellController::lockScreen()
{
    // T-12.3a: one entry point for the Cmd+Ctrl+Q shortcut and the system
    // menu's Lock Screen; the shell owns the lock UI and drives
    // `ext-session-lock-v1`.
    if (m_protocol)
        m_protocol->lockSession();
}

void ShellController::performSessionAction(const QString &action)
{
    QString method;
    if (action == QLatin1String("sleep"))
        method = QStringLiteral("Suspend");
    else if (action == QLatin1String("restart"))
        method = QStringLiteral("Reboot");
    else if (action == QLatin1String("shut-down"))
        method = QStringLiteral("PowerOff");
    else
        return;

    QDBusInterface login1(QStringLiteral("org.freedesktop.login1"),
                          QStringLiteral("/org/freedesktop/login1"),
                          QStringLiteral("org.freedesktop.login1.Manager"),
                          QDBusConnection::systemBus(), this);
    if (!login1.isValid()) {
        qWarning() << "shell: logind is not reachable; cannot" << action;
        return;
    }
    // logind's power methods take `interactive`; true lets polkit prompt when
    // the session is allowed to ask.
    auto *watcher = new QDBusPendingCallWatcher(login1.asyncCall(method, true), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [method](QDBusPendingCallWatcher *call) {
                const QDBusPendingReply<> reply = *call;
                if (reply.isError()) {
                    qWarning() << "shell: logind" << method << "failed:"
                               << reply.error().message();
                }
                call->deleteLater();
            });
}

void ShellController::onLockPowerAction(const QString &action)
{
    performSessionAction(action);
}

void ShellController::requestDevReturn(){
    // T-12.6b: the "Quit to <previous desktop>" row runs the dev harness's
    // return, which restores the previous default session, disarms autologin,
    // and asks the session manager to log out. Prefer the absolute dev-tool
    // path the armed session exported; fall back to `dragonfruit` on PATH.
    const QString configured = ShellMenus::devBin();
    const QString program =
        configured.isEmpty() ? QStringLiteral("dragonfruit") : configured;
    qInfo() << "shell: dev round-trip return requested via" << program;
    const bool started = QProcess::startDetached(
        program, {QStringLiteral("dev"), QStringLiteral("--real"), QStringLiteral("--return")});
    if (!started) {
        qWarning() << "shell: could not start" << program
                   << "— the dev harness is unavailable; log out from the system menu";
    }
}

void ShellController::showLockScreen()
{
    if (!m_lockItem || !m_lockWindow || !m_protocol)
        return;
    m_lockActive = true;
    m_lockPassword.clear();
    applyLockData();
    m_lockItem->setProperty("authEnabled", true);
    m_lockItem->setProperty("authBusy", false);
    m_lockItem->setProperty("message", QString());
    m_lockItem->setProperty("passwordLength", 0);
    const QString user = lockUserName();
    if (!user.isEmpty())
        m_lockItem->setProperty("userName", user);
    if (m_lockTimer && !m_lockTimer->isActive())
        m_lockTimer->start();
    const auto ids = m_lockSurfaceSizes.keys();
    for (quintptr id : ids)
        renderLockSurface(id);
}

void ShellController::applyLockData()
{
    if (!m_lockItem)
        return;
    const QVariantMap values = m_settingsClient ? m_settingsClient->values() : QVariantMap();
    const QString style =
        values.value(QStringLiteral("screenSaver.style"), QStringLiteral("clock")).toString();
    const bool showClock = style != QLatin1String("none")
        && values.value(QStringLiteral("screenSaver.clock"), true).toBool();
    const bool showDate =
        values.value(QStringLiteral("screenSaver.showDate"), true).toBool();
    const bool showSeconds =
        values.value(QStringLiteral("screenSaver.showSeconds"), false).toBool();
    const bool showUserName =
        values.value(QStringLiteral("lock.showUserNameAndPhoto"), true).toBool();
    const bool showPasswordHints =
        values.value(QStringLiteral("lock.showPasswordHints"), false).toBool();
    const QString passwordHint =
        values.value(QStringLiteral("lock.passwordHint")).toString();
    const bool showLockMessage =
        values.value(QStringLiteral("lock.showMessageWhenLocked"), false).toBool();
    const QString lockMessage = values.value(QStringLiteral("lock.message")).toString();
    const bool showPowerButtons =
        values.value(QStringLiteral("lock.showPowerButtons"), true).toBool();
    const QDateTime now = QDateTime::currentDateTime();
    m_lockItem->setProperty("showClock", showClock);
    m_lockItem->setProperty("showDate", showDate);
    m_lockItem->setProperty("showSeconds", showSeconds);
    m_lockItem->setProperty("showUserNameAndPhoto", showUserName);
    m_lockItem->setProperty("showPasswordHints", showPasswordHints);
    m_lockItem->setProperty("passwordHint", passwordHint);
    m_lockItem->setProperty("showLockMessage", showLockMessage);
    m_lockItem->setProperty("lockMessageText", lockMessage);
    m_lockItem->setProperty("showPowerButtons", showPowerButtons);
    m_lockItem->setProperty(
        "timeText",
        now.toString(showSeconds ? QStringLiteral("HH:mm:ss") : QStringLiteral("HH:mm")));
    m_lockItem->setProperty("dateText", now.toString(QStringLiteral("dddd d MMMM")));
}

void ShellController::renderLockSurface(quintptr lockSurfaceId)
{
    if (!m_lockWindow || !m_lockItem || !m_protocol)
        return;
    const auto it = m_lockSurfaceSizes.constFind(lockSurfaceId);
    if (it == m_lockSurfaceSizes.constEnd() || it.value().isEmpty())
        return;
    const QSize size = it.value();
    // Lock surfaces are not layer surfaces, so they do not receive a
    // wp_fractional_scale event; they follow the output scale directly.
    const qreal scale = m_protocol ? m_protocol->outputScale() : m_chromeScale;
    prepareChromeScene(m_lockWindow, m_lockItem, size, size, scale);
    const bool committed =
        commitChromeScene(m_lockWindow,
                          {{QStringLiteral("lock"), size, QRect(),
                            [this, lockSurfaceId, size](const QImage &image) {
                                return m_protocol->commitLockImage(lockSurfaceId,
                                                                   image, size);
                            },
                            lockSurfaceId}});
    if (committed)
        m_lockCommitted.insert(lockSurfaceId);
}

void ShellController::repaintLockSurfaces()
{
    if (!m_lockActive)
        return;
    const auto ids = m_lockSurfaceSizes.keys();
    for (quintptr id : ids)
        renderLockSurface(id);
}

void ShellController::teardownLockScreen()
{
    if (m_lockAuth)
        m_lockAuth->cancel();
    m_lockActive = false;
    m_lockPassword.clear();
    if (m_lockItem)
        m_lockItem->setProperty("passwordLength", 0);
    if (m_lockTimer)
        m_lockTimer->stop();
    m_lockSurfaceSizes.clear();
    m_lockCommitted.clear();
}

QString ShellController::lockUserName() const
{
    const QString user = qEnvironmentVariable("USER");
    if (!user.isEmpty())
        return user;
    return qEnvironmentVariable("LOGNAME");
}

void ShellController::submitLockPassword(const QString &password)
{
    if (!m_lockActive || !m_lockAuth || !m_protocol || !m_protocol->isSessionLocked())
        return;
    const QString user = lockUserName();
    if (user.isEmpty())
        return;
    if (m_lockItem) {
        m_lockItem->setProperty("authEnabled", true);
        m_lockItem->setProperty("authBusy", true);
        m_lockItem->setProperty("message", QString());
    }
    repaintLockSurfaces();
    if (!m_lockAuth->authenticate(user, password))
        onLockAuthFailed(QStringLiteral("Authentication unavailable"));
}

void ShellController::onLockAuthSucceeded()
{
    m_lockPassword.clear();
    if (m_lockItem) {
        m_lockItem->setProperty("authBusy", false);
        m_lockItem->setProperty("message", QString());
        m_lockItem->setProperty("passwordLength", 0);
    }
    repaintLockSurfaces();
    // PAM accepted the password: release the fail-secure lock. The compositor
    // clears its state and destroys the lock surfaces; the shell resets its
    // scene. No credential is retained anywhere.
    if (m_protocol && m_protocol->isSessionLocked())
        m_protocol->unlockSession();
    teardownLockScreen();
}

void ShellController::onLockAuthFailed(const QString &message)
{
    m_lockPassword.clear();
    if (m_lockItem) {
        m_lockItem->setProperty("authBusy", false);
        m_lockItem->setProperty("message", message);
        m_lockItem->setProperty("passwordLength", 0);
    }
    repaintLockSurfaces();
}

void ShellController::onLockKeyEvent(quint32 key, bool pressed, bool shift)
{
    if (!m_lockActive || !m_lockItem || !pressed)
        return;
    // A PAM attempt in flight owns the field: keys are ignored until it
    // returns, so a password cannot be edited mid-authentication.
    if (m_lockItem->property("authBusy").toBool())
        return;

    const dragonfruit::LockKey mapped = dragonfruit::lockKeyFromEvdev(key, shift);
    switch (mapped.kind) {
    case dragonfruit::LockKeyKind::Escape:
        m_lockPassword.clear();
        break;
    case dragonfruit::LockKeyKind::Backspace:
        m_lockPassword.chop(1);
        break;
    case dragonfruit::LockKeyKind::Return: {
        if (m_lockPassword.isEmpty())
            return;
        const QString password = m_lockPassword;
        m_lockPassword.clear();
        m_lockItem->setProperty("passwordLength", 0);
        // `submitLockPassword` clears the field, flips `authBusy`, and
        // repaints; the buffer is already gone from here.
        submitLockPassword(password);
        return;
    }
    case dragonfruit::LockKeyKind::Character:
        m_lockPassword.append(mapped.character);
        break;
    case dragonfruit::LockKeyKind::Ignore:
        return;
    }
    m_lockItem->setProperty("passwordLength", m_lockPassword.size());
    repaintLockSurfaces();
}

void ShellController::onMissionControlRequested()
{
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    m_protocol->enterMissionControl();
}

void ShellController::onStatusItemActivated(const QString &itemId)
{
    // A tray item opens its owner's DBusMenu (T-14.3); the two action extras
    // added in T231 open the shell's own surfaces; every other status slot is
    // a first-party popup or an inert placeholder.
    if (itemId.startsWith(QLatin1String("tray:"))) {
        openTrayMenu(itemId.mid(5));
        return;
    }
    if (itemId == QLatin1String("search")) {
        showAppsDrawer();
        return;
    }
    if (itemId == QLatin1String("notifications")) {
        toggleNotificationCenter();
        return;
    }
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
    schedulePopupGeometry();
    // The popup's fade/scale-in is QML-driven; the scene-graph hook commits
    // every frame it renders (FR-14).
}

void ShellController::onAppMenuClosed()
{
    m_menuOpen = false;
    if (m_protocol) {
        m_protocol->setPopupInputActive(false);
        m_protocol->clearPopupPanelRegions();
    }
    // The measured menu exit is 183 ms. The logical menu and input close now;
    // the overlay keeps rendering the visual tail so a reopen/retarget can
    // take over before the timer expires.
    QTimer::singleShot(210, this, [this]() {
        if (!m_menuOpen) {
            updatePopupGeometry();
            if (m_menuRefreshDeferred) {
                m_menuRefreshDeferred = false;
                applyFocusedApp();
            }
        }
    });
}

void ShellController::onAppMenuTriggered(int menuIndex, int itemIndex, const QVariant &item)
{
    const QVariantMap map = item.toMap();
    const QString action = map.value(QStringLiteral("action")).toString();
    dispatchAppAction(action, menuIndex, itemIndex, map);
    scheduleRender();
}

void ShellController::dispatchAppAction(const QString &action, int menuIndex, int itemIndex,
                                        const QVariantMap &map)
{
    if (action.startsWith(QLatin1String("dbusmenu:"))) {
        // T230: a bridged third-party row routes back through the window's
        // registered DBusMenu object (`Event(id, "clicked")`), never a shell
        // guess at the app's internal action.
        routeDbusMenuAction(action);
    } else if (action.startsWith(QLatin1String("recent-app:"))) {
        openApp(action.mid(int(qstrlen("recent-app:"))));
    } else if (action.startsWith(QLatin1String("window.activate:"))) {
        if (m_protocol)
            m_protocol->selectToplevel(action.mid(int(qstrlen("window.activate:"))));
    } else if (action == QLatin1String("window.minimize")
               || action == QLatin1String("minimize")) {
        if (m_protocol && m_focusedWindowId != 0)
            m_protocol->setToplevelMinimized(QString::number(m_focusedWindowId), true);
    } else if (action == QLatin1String("window.zoom") || action == QLatin1String("zoom")) {
        if (m_protocol && m_focusedWindowId != 0) {
            m_protocol->setToplevelZoomed(QString::number(m_focusedWindowId),
                                          !m_protocol->focusedWindowZoomed());
        }
    } else if (action == QLatin1String("window.bring-all-to-front")
               || action == QLatin1String("bring-all-to-front")) {
        // macOS' Bring All to Front restores every window of the app and
        // raises it; the compositor's activate_app picks the most recent.
        if (m_protocol && !m_appId.isEmpty()) {
            for (const QVariant &value : std::as_const(m_runningEntries)) {
                const QVariantMap row = value.toMap();
                if (row.value(QStringLiteral("kind")).toString()
                    != QLatin1String("temporary"))
                    continue;
                if (row.value(QStringLiteral("appId")).toString() != m_appId)
                    continue;
                for (const QVariant &windowValue :
                     row.value(QStringLiteral("windowList")).toList()) {
                    const QString windowId = windowValue.toMap()
                                                 .value(QStringLiteral("windowId"))
                                                 .toString();
                    if (!windowId.isEmpty())
                        m_protocol->setToplevelMinimized(windowId, false);
                }
            }
            m_protocol->activateApp(m_appId);
        }
    } else if (action == QLatin1String("close")) {
        if (m_protocol && m_focusedWindowId != 0)
            m_protocol->closeToplevel(QString::number(m_focusedWindowId));
    } else if (action == QLatin1String("quit")) {
        // The application menu's Quit closes the focused app; on the desktop
        // (Files menu) there is no app to close yet.
        if (!m_appId.isEmpty())
            m_protocol->closeApp(m_appId);
    } else if (action == QLatin1String("settings")) {
        // The system menu's "System Settings…" and the application menu's
        // "Settings…" open the first-party Settings app (T-09); if it is
        // already running its window is focused instead of a second launch.
        openApp(QStringLiteral("org.dragonfruit.Settings.desktop"));
    } else if (action == QLatin1String("about-system")) {
        // The Settings executable owns both About surfaces. Launch its
        // utility-window entry point; More Info reveals the canonical page.
        const DesktopEntry entry = m_index.byId(QStringLiteral("org.dragonfruit.Settings.desktop"));
        QStringList argv = DesktopEntryIndex::buildLaunchCommand(entry, QStringList());
        if (!argv.isEmpty()) {
            argv << QStringLiteral("--about-system");
            if (!launchDetachedFromShell(argv, m_waylandDisplay, nullptr))
                qWarning() << "shell: could not open About This System";
        } else {
            qWarning() << "shell: Settings launcher unavailable for About This System";
        }
    } else if (action == QLatin1String("about")) {
        openApp(QStringLiteral("org.dragonfruit.Settings.desktop"));
    } else if (action == QLatin1String("hide")) {
        hideAppWindows(m_appId);
    } else if (action == QLatin1String("hide-others")) {
        hideOtherAppWindows(m_appId);
    } else if (action == QLatin1String("show-all")) {
        showHiddenWindows();
    } else if (action == QLatin1String("sleep") || action == QLatin1String("restart")
               || action == QLatin1String("shut-down")) {
        // T229a: the lock screen's power row and the system menu share one
        // logind path.
        performSessionAction(action);
    } else if (action == QLatin1String("log-out")) {
        // The logind session action has no shell implementation yet; it lands
        // with the real-session work (T-12.6, T-16.4).
        qInfo() << "shell: session action requested (T-12.6):" << action;
    } else if (action == QLatin1String("quit-to-return")) {
        // T-12.6b: the dev harness's return. It restores the previous default
        // session, disarms autologin, and logs out; the shell only triggers it.
        requestDevReturn();
    } else if (action == QLatin1String("lock-screen")) {
        // Same path as the Cmd+Ctrl+Q shortcut (T-12.3a).
        lockScreen();
    } else if (action == QLatin1String("app-store")) {
        // No app-store equivalent exists yet; the item is disabled (T-09
        // open question in design/04-shell.md).
        qInfo() << "shell: Software Center requested (no equivalent yet)";
    } else {
        // T-14 dispatches the resolved action back to the owning app; until
        // that channel exists the placeholder logs so the demo has feedback.
        qInfo() << "shell: app menu item activated:" << menuIndex << itemIndex << map;
    }
}

void ShellController::onAppAccelerator(const QString &appId, const QString &action,
                                       const QString &source, quint32 serial)
{
    // T-14.2b: the compositor matched a focus-scoped accelerator and hands the
    // owning action back. The shell routes it through the same dispatch seam
    // the menu rows use, so a keyboard shortcut and a click cannot diverge.
    // Ignore a delivery for an app that is no longer focused (a stale match).
    if (appId != m_appId)
        return;
    qInfo() << "shell: app accelerator" << action << "for" << appId
            << "via" << source << "serial" << serial;
    dispatchAppAction(action, -1, -1, {});
    scheduleRender();
}

void ShellController::noteInitialStatusReply(int bit)
{
    if (m_initialStatusPending == 0 || (m_initialStatusPending & bit) == 0)
        return;
    m_initialStatusPending &= ~bit;
    if (m_initialStatusPending == 0)
        commitInitialChrome();
}

void ShellController::commitInitialChrome()
{
    if (m_initialStatusPending != 0)
        return;
    if (m_initialStatusTimer) {
        m_initialStatusTimer->stop();
        m_initialStatusTimer->deleteLater();
        m_initialStatusTimer = nullptr;
    }
    render();
    // Match `onConfigured`'s post-first-frame pass: any scene-graph-only
    // content paints on the next event-loop turn, not in the first grab.
    QTimer::singleShot(200, this, &ShellController::render);
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


void ShellController::schedulePopupGeometry()
{
    if (!m_menuOpen || m_popupGeometryPending)
        return;
    m_popupGeometryPending = true;
    // opened() is delivered before dependent QML region bindings settle.
    // Read once after that transaction, including submenu/layout changes.
    QTimer::singleShot(0, this, [this]() {
        m_popupGeometryPending = false;
        if (m_menuOpen)
            updatePopupGeometry();
    });
}

void ShellController::updatePopupGeometry()
{
    if (!m_item) {
        return;
    }
    if (!m_menuOpen) {
        m_popupWidth = 0;
        m_popupHeight = 0;
        m_protocol->setPopupInputActive(false);
        m_protocol->clearPopupPanelRegions();
        m_protocol->hidePopup();
        render();
        return;
    }
    // The open dropdown's rectangle in window coordinates. The popup QML
    // overflows the bar item; the shell reveals it through the overlay
    // surface placed at exactly this rectangle.
    const int x = qMax(0, qFloor(m_item->property("dropdownX").toReal()));
    const int y = qMax(0, qFloor(m_item->property("dropdownY").toReal()));
    const int width = qCeil(m_item->property("dropdownX").toReal()
                            + m_item->property("dropdownWidth").toReal()) - x;
    const int height = qCeil(m_item->property("dropdownY").toReal()
                             + m_item->property("dropdownHeight").toReal()) - y;
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
    // T202: declare the exact attached/detached/status regions for this open
    // menu. The popup surface accepts input only while it is logically open;
    // the close path clears both before the visual exit.
    m_protocol->setPopupPanelRegions(
        m_item->property("dropdownSurfaceRegions").toList());
    m_protocol->setPopupInputActive(true);
    render();
}

void ShellController::onPointerMoved(qreal x, qreal y)
{
    if (!m_window)
        return;
    sendChromePointer(m_window, QEvent::MouseMove, QPointF(x, y), Qt::NoButton, m_buttons);
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
    sendChromePointer(m_window, pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease,
                        QPointF(x, y), qtButton, m_buttons);
    scheduleRender();
}

void ShellController::onPointerLeft()
{
    if (!m_window)
        return;
    // Move the pointer off-screen so hover states clear when the compositor
    // takes the pointer away (e.g. it moved onto a window).
    sendChromePointer(m_window, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton, m_buttons);
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
    // The Control Center panel is dismissed by a click-away. Focus may move
    // from the bar to the panel in the same dispatch (a click on a tile), so
    // defer the check one event-loop turn: if the panel then holds keyboard,
    // this was not a dismissal.
    if (!focused && m_controlCenterOpen) {
        QTimer::singleShot(0, this, [this]() {
            if (m_controlCenterOpen && !m_controlCenterKeyboardFocused)
                hideControlCenter();
        });
    }
    // Same click-away rule for the Notification Center. Its dedicated focus
    // signal normally handles this; the bar's focus loss is the fallback for
    // an open triggered by the clock while the bar held the keyboard.
    if (!focused && m_notificationCenterOpen) {
        QTimer::singleShot(0, this, [this]() {
            if (m_notificationCenterOpen && !m_notificationCenterKeyboardFocused)
                hideNotificationCenter(QStringLiteral("outside"));
        });
    }
}

void ShellController::onChromePointerAxis(ShellProtocol::PointerSurface surface, qreal x,
                                          qreal y, quint32 axis, qreal value)
{
    QQuickWindow *window = nullptr;
    switch (surface) {
    case ShellProtocol::PointerSurface::AppsDrawer:
        window = m_appsDrawerWindow;
        break;
    case ShellProtocol::PointerSurface::Chooser:
        window = m_chooserWindow;
        break;
    case ShellProtocol::PointerSurface::Screenshot:
        window = m_screenshotWindow;
        break;
    case ShellProtocol::PointerSurface::ScreenCast:
        window = m_screencastWindow;
        break;
    case ShellProtocol::PointerSurface::Polkit:
        window = m_polkitWindow;
        break;
    case ShellProtocol::PointerSurface::NotificationCenter:
        window = m_notificationCenterWindow;
        break;
    case ShellProtocol::PointerSurface::ControlCenter:
        window = m_controlCenterWindow;
        break;
    case ShellProtocol::PointerSurface::Overview:
        window = m_overviewWindow;
        break;
    case ShellProtocol::PointerSurface::Banner:
        window = m_bannerWindow;
        break;
    case ShellProtocol::PointerSurface::Dock:
    case ShellProtocol::PointerSurface::DockPopup:
        window = m_dockWindow;
        break;
    case ShellProtocol::PointerSurface::MenuBar:
    case ShellProtocol::PointerSurface::Popup:
        window = m_window;
        break;
    case ShellProtocol::PointerSurface::Lock:
    case ShellProtocol::PointerSurface::None:
        return;
    }
    if (!window)
        return;

    // wl_pointer.axis value is in surface-local scroll units, positive
    // down/right. Qt wheel deltas are 1/8 degree (120 per notch) for
    // angleDelta and device pixels for pixelDelta; provide both so a
    // Flickable scrolls smoothly and a discrete wheel still steps a notch.
    const qreal delta = -value;
    int pixelX = 0;
    int pixelY = 0;
    int angleX = 0;
    int angleY = 0;
    if (axis == 0) {
        pixelY = qRound(delta * 2);
        angleY = qRound(delta * 12);
    } else if (axis == 1) {
        pixelX = qRound(delta * 2);
        angleX = qRound(delta * 12);
    } else {
        return;
    }
    QPointF logical(x, y);
    if (surface == ShellProtocol::PointerSurface::DockPopup)
        logical += QPointF(m_dockPopoverX, m_dockPopoverY);
    if (surface == ShellProtocol::PointerSurface::Dock
        || surface == ShellProtocol::PointerSurface::DockPopup) {
        logical += QPointF(m_dockItemOffsetX, m_dockItemOffsetY);
    }
    // T-21.1d: the GPU producer keeps its QQuickWindow in logical units while
    // its FBO is physical; the software producer's window is the physical
    // buffer. Only scale the injected coordinate for the latter.
    const bool logicalWindow =
        qobject_cast<GpuChromeFrameProducer *>(m_producers.value(window)) != nullptr;
    const QPointF pos = logicalWindow
        ? logical
        : ChromeScale::logicalToBuffer(logical, m_chromeScale);
    QWheelEvent event(pos, pos, QPoint(pixelX, pixelY),
                      QPoint(angleX, angleY), Qt::NoButton, Qt::NoModifier,
                      Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(window, &event);

    switch (surface) {
    case ShellProtocol::PointerSurface::AppsDrawer:
        scheduleAppsDrawerRender();
        break;
    case ShellProtocol::PointerSurface::Chooser:
        scheduleChooserRender();
        break;
    case ShellProtocol::PointerSurface::Screenshot:
        scheduleScreenshotRender();
        break;
    case ShellProtocol::PointerSurface::ScreenCast:
        scheduleScreenCastRender();
        break;
    case ShellProtocol::PointerSurface::Polkit:
        schedulePolkitRender();
        break;
    case ShellProtocol::PointerSurface::NotificationCenter:
        scheduleNotificationCenterRender();
        break;
    case ShellProtocol::PointerSurface::ControlCenter:
        scheduleControlCenterRender();
        break;
    case ShellProtocol::PointerSurface::Overview:
        scheduleOverviewRender();
        break;
    case ShellProtocol::PointerSurface::Banner:
        scheduleBannerRender();
        break;
    case ShellProtocol::PointerSurface::Dock:
    case ShellProtocol::PointerSurface::DockPopup:
        scheduleDockRender();
        break;
    case ShellProtocol::PointerSurface::MenuBar:
    case ShellProtocol::PointerSurface::Popup:
        scheduleRender();
        break;
    case ShellProtocol::PointerSurface::Lock:
    case ShellProtocol::PointerSurface::None:
        break;
    }
}

void ShellController::onKeyEvent(quint32 key, bool pressed, quint32 modifiers)
{
    if (!m_window)
        return;
    const Qt::Key qtKey = chromeKeyFromEvdev(key, qtModifiersFromXkb(modifiers)).first;
    // T-11.4b: Escape dismisses a visible OSD. The OSD surface never takes
    // keyboard focus, so its dismissal is owned here, at the shell level: it
    // lets a keyboard user clear the alert without stealing focus from the
    // active window. A visible OSD clears first; the panel check follows.
    if (pressed && qtKey == Qt::Key_Escape && m_osdActive) {
        hideOsd();
        return;
    }
    // T-11.3a: Escape dismisses an open Control Center panel no matter which
    // chrome surface currently holds the keyboard.
    if (pressed && qtKey == Qt::Key_Escape && m_controlCenterOpen) {
        hideControlCenter();
        return;
    }
    // T204: the Center's own scene owns traversal while it holds the keyboard
    // (`notificationCenterKeyEvent`); this is the fallback for a Center opened
    // from the bar before the compositor hands it focus.
    if (pressed && qtKey == Qt::Key_Escape && m_notificationCenterOpen) {
        hideNotificationCenter(QStringLiteral("escape"));
        return;
    }
    // An open Dock popover or Dock keyboard focus owns the keys (navigation,
    // Escape; T-10 sections 13/20); otherwise the menu bar does.
    if (m_dockWindow && m_dockItem
            && (m_dockKeyboardFocused
                || m_dockItem->property("popoverOpen").toBool())) {
        sendChromeKeyEvent(m_dockWindow, key, pressed, modifiers);
        scheduleDockRender();
    } else {
        sendChromeKeyEvent(m_window, key, pressed, modifiers);
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
    // The Dock needs the kind to choose the highlight or the live gap; the
    // resolved identity and file count arrive separately once the enter-time
    // payload read completes (`onDockExternalDragPayload`).
    m_externalPayloadIsFolder = false;
    QMetaObject::invokeMethod(m_dockItem, "beginExternalDrag", Q_ARG(QVariant, payloadIsApp),
                              Q_ARG(QVariant, 0));
    onDockExternalDragMoved(x, y);
}

void ShellController::onDockExternalDragPayload(bool payloadIsApp, const QString &desktopId,
                                                const QStringList &paths)
{
    if (!m_dockItem)
        return;
    QString name;
    QString iconPath;
    int count = 0;
    QString firstName;
    const bool isFolder = !payloadIsApp && uriListIsFolder(paths);
    m_externalPayloadIsFolder = isFolder;
    if (payloadIsApp && !desktopId.isEmpty()) {
        // Resolve the app alias through app-index so the ghost shows the real
        // name and themed icon; a failure degrades to the raw identity
        // (legacy/10-dock.md FR-7), never a crash.
        DesktopEntry entry = m_index.byId(desktopId);
        if (!entry.valid)
            entry = m_index.resolve(desktopId);
        name = entry.valid ? entry.name : desktopId;
        iconPath = entry.valid ? entry.iconPath : QString();
        count = 1;
    } else if (isFolder) {
        // A single folder shows its own name in the drop ghost (T-14.7k).
        name = folderDisplayName(paths.first());
        count = 1;
        firstName = name;
    } else {
        count = paths.size();
        if (!paths.isEmpty())
            firstName = QFileInfo(paths.first()).fileName();
    }
    QMetaObject::invokeMethod(m_dockItem, "setExternalPayload", Q_ARG(QVariant, payloadIsApp),
                              Q_ARG(QVariant, name), Q_ARG(QVariant, iconPath),
                              Q_ARG(QVariant, count), Q_ARG(QVariant, firstName),
                              Q_ARG(QVariant, isFolder));
    scheduleDockRender();
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
    // A single `file://` directory is a folder pin candidate (T-14.7k); the
    // classification is filesystem-backed and independent of the app flag.
    m_externalPayloadIsFolder = !payloadIsApp && uriListIsFolder(paths);
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
    Q_UNUSED(payloadIsApp)
    // Use the drop-time classification, not the QML's enter-time kind: a
    // single `.desktop` URI is an app alias even though it arrived as files,
    // and a single `file://` directory is a folder pin (T-14.7k).
    const DockDropPayload payload = m_externalPayloadIsApp ? DockDropPayload::Application
                                    : m_externalPayloadIsFolder ? DockDropPayload::Folder
                                                                : DockDropPayload::Files;
    const DockDropAction action = dockDropActionFor(targetKind, payload);
    // The folder a stack target represents: the built-in Downloads member or
    // the path encoded in a `folder:` pin id.
    const auto stackTargetPath = [this, &targetId]() -> QString {
        if (targetId == QLatin1String("__downloads__"))
            return downloadsDirectory();
        if (targetId.startsWith(QLatin1String("folder:")))
            return targetId.mid(7);
        return QString();
    };
    switch (action) {
    case DockDropAction::PinFolder: {
        const bool hasFolder = !m_externalPaths.isEmpty();
        const QString folder = hasFolder ? QDir::cleanPath(m_externalPaths.first()) : QString();
        const QString downloads = QDir::cleanPath(downloadsDirectory());
        if (!hasFolder || folder.isEmpty()) {
            qWarning() << "shell: Dock folder drop had no path";
        } else if (!QFileInfo(folder).isDir()) {
            // Only an absolute, existing directory is persisted (ADR 0092).
            raiseDockNotice(m_notificationClient, QStringLiteral("Could not pin folder"),
                            QStringLiteral("Only an existing folder can be pinned to the Dock."));
        } else if (folder == downloads) {
            qInfo() << "shell: Dock folder" << folder << "is the default Downloads stack";
        } else {
            QStringList next = m_dockConfig.pinnedFolders;
            bool duplicate = false;
            for (const QString &existing : next) {
                if (QDir::cleanPath(existing) == folder)
                    duplicate = true;
            }
            if (duplicate) {
                qInfo() << "shell: Dock folder" << folder << "is already pinned (no-op)";
            } else {
                next.append(folder);
                writeDockSetting(QStringLiteral("dock.pinnedFolders"), next);
                refreshFolderStackData();
                rebuildDockEntries();
                qInfo() << "shell: Dock pinned folder" << folder << "from an external drop";
            }
        }
        break;
    }
    case DockDropAction::PinApp: {
        const QString id = m_externalDesktopId;
        if (id.isEmpty()) {
            qWarning() << "shell: Dock external app drop had no desktop id";
        } else {
            // Duplicate detection is by identity, not raw string: a raw app
            // alias may resolve to an already-pinned `.desktop` id.
            DesktopEntry resolved = m_index.byId(id);
            if (!resolved.valid)
                resolved = m_index.resolve(id);
            const QString resolvedId = resolved.valid ? resolved.id : QString();
            if (dockPinnedContains(m_dockConfig.pinned, id, resolvedId)) {
                flashDuplicatePin(resolvedId.isEmpty() ? id : resolvedId);
                qInfo() << "shell: Dock app" << id << "is already pinned (visible no-op)";
            } else {
                QStringList ids = m_dockConfig.pinned;
                ids.append(id);
                writeDockSetting(QStringLiteral("dock.pinned"), ids);
                rebuildDockEntries();
                qInfo() << "shell: Dock pinned" << id << "from an external drop";
            }
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
        if (!m_trash || !m_trash->isAvailable()) {
            raiseDockNotice(m_notificationClient, QStringLiteral("Could not move to Trash"),
                            QStringLiteral("The Trash is not available right now."));
            qWarning() << "shell: Dock drop on Trash while the backend is unavailable";
            break;
        }
        const int trashed = m_trash->trash(m_externalPaths);
        const int failed = m_externalPaths.size() - trashed;
        if (failed > 0) {
            raiseDockNotice(
                m_notificationClient, QStringLiteral("Could not move to Trash"),
                failed == 1
                    ? QStringLiteral("1 item could not be moved to the Trash.")
                    : QStringLiteral("%1 items could not be moved to the Trash.").arg(failed));
        }
        qInfo() << "shell: Dock trashed" << trashed << "of" << m_externalPaths.size()
                << "dropped items";
        break;
    }
    case DockDropAction::MoveToFolder: {
        const QString folder = stackTargetPath();
        if (folder.isEmpty()) {
            qWarning() << "shell: Dock file drop on an unresolved folder stack";
            break;
        }
        const QString name = folderDisplayName(folder);
        QString error;
        const int moved = moveFilesIntoFolder(folder, m_externalPaths, &error);
        const int failed = m_externalPaths.size() - moved;
        if (failed > 0) {
            raiseDockNotice(
                m_notificationClient, QStringLiteral("Could not move to %1").arg(name),
                failed == 1
                    ? QStringLiteral("1 item could not be moved to %1.").arg(name)
                    : QStringLiteral("%1 items could not be moved to %2.").arg(failed).arg(name));
            qWarning() << "shell: Dock could not move" << failed << "items into" << folder
                       << error;
        } else if (moved > 0) {
            qInfo() << "shell: Dock moved" << moved << "items into" << folder;
        }
        break;
    }
    case DockDropAction::None:
    default:
        break;
    }
    m_externalPayloadIsApp = false;
    m_externalPayloadIsFolder = false;
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
    } else if (action == QLatin1String("control-center")) {
        // T-11.3a: the Control Center keyboard shortcut toggles the panel.
        toggleControlCenter();
    } else if (action == QLatin1String("notification-center")) {
        // T204: the same action comes from the declared keyboard shortcut and
        // any configured hot corner. The shell owns the Center; the
        // compositor only routes the trigger (and picks the invoking output).
        toggleNotificationCenter();
    } else if (action == QLatin1String("show-apps")) {
        // T-19.2: the Applications drawer shortcut (F4) toggles the launcher.
        toggleAppsDrawer();
    } else if (action == QLatin1String("lock-screen")) {
        // T-12.3a: the compositor routes Cmd+Ctrl+Q here; the system menu
        // routes the same action (see onAppMenuTriggered). The shell owns the
        // lock UI and drives `ext-session-lock-v1` (authentication is
        // T-12.3b).
        lockScreen();
    } else if (action == QLatin1String("screenshot")) {
        // T-13.3a: Cmd+Shift+3 captures the whole output.
        startScreenshot(QStringLiteral("fullscreen"));
    } else if (action == QLatin1String("screenshot-region")) {
        // T-13.3a: Cmd+Shift+4 opens the region selection.
        startScreenshot(QStringLiteral("region"));
    }
}

// --- Dock (T-10) ------------------------------------------------------------

void ShellController::onMaterialStateChanged(quint32 tier, bool blurActive, quint32 fallback)
{
    // The legacy v3 event is the compatibility source for a compositor that
    // does not send role state. Once a role has a v4 state, `applied()` keeps
    // it and this snapshot cannot pull the persistent chrome down.
    m_materialPolicy.setGlobal(tier, blurActive, fallback);
    applyMaterialState();
}

void ShellController::onMaterialRoleStateChanged(quint32 role, quint32 tier,
                                                 bool blurActive, quint32 fallback)
{
    m_materialPolicy.setRole(role, tier, blurActive, fallback);
    // Change-only state broadcast; record the reason alongside the tier so a
    // capture log can name the milky fallback without inferring it from pixels.
    qInfo().nospace() << "shell: material role=" << materialRoleName(role)
                      << " tier=" << tier << " blur=" << (blurActive ? 1 : 0)
                      << " fallback=" << fallback;
    applyMaterialState();
}

void ShellController::seedMaterialPolicy()
{
    if (!m_protocol)
        return;
    m_materialPolicy.setGlobal(m_protocol->materialTier(),
                               m_protocol->materialBlurActive(),
                               m_protocol->materialFallback());
    for (quint32 role = 0; role < MaterialPolicy::RoleCount; ++role) {
        if (!m_protocol->materialRoleKnown(role))
            continue;
        m_materialPolicy.setRole(role, m_protocol->materialRoleTier(role),
                                 m_protocol->materialRoleBlurActive(role),
                                 m_protocol->materialRoleFallback(role));
    }
}

void ShellController::applyMaterialState()
{
    const AppliedMaterial popup = m_materialPolicy.applied(MaterialPolicy::Popup);
    const AppliedMaterial drawer = m_materialPolicy.applied(MaterialPolicy::Drawer);

    const bool menuChanged =
        setMaterialProperties(m_item, m_materialPolicy.applied(MaterialPolicy::Chrome));
    const bool dockChanged =
        setMaterialProperties(m_dockItem, m_materialPolicy.applied(MaterialPolicy::Dock));
    // The bar and Dock scenes host their popovers, so the transient role is
    // pushed into the same two scenes under the `popupMaterial*` names.
    const bool menuPopupChanged = setMaterialProperties(m_item, popup, "popupMaterial");
    const bool dockPopupChanged = setMaterialProperties(m_dockItem, popup, "popupMaterial");
    const bool drawerChanged = setMaterialProperties(m_appsDrawerItem, drawer);
    const bool bannerChanged = setMaterialProperties(m_bannerItem, popup);
    const bool notificationCenterChanged =
        setMaterialProperties(m_notificationCenterItem, popup);
    const bool controlCenterChanged = setMaterialProperties(m_controlCenterItem, popup);
    const bool osdChanged = setMaterialProperties(m_osdItem, popup);

    // The event arrives from the Wayland dispatch, outside the render path;
    // repaint the affected surface so the new plate is committed immediately
    // rather than waiting for an unrelated scene change.
    if (dockChanged || dockPopupChanged)
        scheduleDockRender();
    if ((menuChanged || menuPopupChanged) && m_window)
        render();
    if (drawerChanged && m_appsDrawerOpen)
        scheduleAppsDrawerRender();
    if (bannerChanged && m_bannerPending)
        scheduleBannerRender();
    if (notificationCenterChanged && m_notificationCenterOpen)
        scheduleNotificationCenterRender();
    if (controlCenterChanged && m_controlCenterOpen)
        scheduleControlCenterRender();
    if (osdChanged && m_osdActive)
        renderOsd();
}

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
    if (width == m_dockWidth && height == m_dockHeight)
        return;
    m_dockWidth = width;
    m_dockHeight = height;
    fprintf(stderr, "dragonfruit-shell: Dock configured %dx%d\n", width, height);
    // The output length along the Dock axis is only known now, so re-run the
    // section 5.1 overflow clamp. This never resets the Repeater model unless
    // the hidden set changed (a divider resize reconfigures the surface).
    applyDockOverflowResult(computeDockOverflow(), true);
    scheduleDockRender();
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
    m_dockItem->setProperty("chooserOnHover", m_dockConfig.chooserOnHover);
    // The minimize reaction is opt-in; the capture seam forces the property on
    // regardless of the persisted key (T-14.7s). Turning the key off drops any
    // in-flight pulse so a stale hop cannot outlive the setting.
    m_dockItem->setProperty("minimizeReaction",
                            m_dockConfig.minimizeReaction || m_minimizeReactionFixture);
    if (!m_dockConfig.minimizeReaction && !m_minimizeReactionFixture) {
        m_minimizeReactionStart.clear();
        publishDockBouncePhases();
    }
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
    m_dockThickness = qCeil(m_dockItem->property("surfaceThickness").toReal());
    const int exclusive = m_dockConfig.autohide
            ? 0
            : qRound(m_dockItem->property("reservedThickness").toReal());
    if (!m_protocol->configureDockSurface(position, m_dockThickness, exclusive))
        qWarning() << "shell: cannot reconfigure the Dock surface:"
                   << m_protocol->lastError();
    scheduleDockRender();
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
    // When settingsd is up, wait for its first snapshot: seeding before the
    // persisted `dock.pinned` is read would overwrite the user's pins.
    if (m_settingsClient && m_settingsClient->isAvailable()
        && !m_settingsSnapshotReceived) {
        return;
    }
    if (!m_dockConfig.pinned.isEmpty()) {
        m_defaultPinsSeeded = true;
        return;
    }
    const QStringList defaults = resolveDefaultDockPins(m_index);
    if (defaults.isEmpty()) {
        // An empty corpus means app-index has not answered yet (the shell and
        // the service start in parallel); leave the seed armed so the
        // registration resync can resolve the defaults. Once a real corpus
        // has no default apps, there is nothing to retry.
        if (!m_index.isEmpty())
            m_defaultPinsSeeded = true;
        return;
    }
    m_defaultPinsSeeded = true;
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
    const int exclusive = m_dockConfig.autohide
            ? 0
            : qRound(m_dockItem->property("reservedThickness").toReal());
    if (!m_protocol->configureDockSurface(m_dockPosition, m_dockThickness, exclusive))
        qWarning() << "shell: cannot resize the Dock surface:" << m_protocol->lastError();
    scheduleDockRender();
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
    // T196: the glass preferences travel in one snapshot alongside the
    // resolved scheme; the v5 motion request above stays for older
    // compositors.
    m_protocol->setAppearancePolicy(policy.colorScheme, policy.transparency,
                                    policy.reduceTransparency, policy.increaseContrast);
    m_protocol->setInputPolicy(policy.repeatDelayMs, policy.repeatRateHz,
                               policy.gesturesEnabled, policy.gestureSpaceSwitch,
                               policy.gestureMissionControl);
    m_protocol->setPointerPolicy(policy.pointerSpeed, policy.naturalScroll,
                                 policy.tapToClick, policy.leftHanded,
                                 policy.scrollMethod, policy.secondaryClick,
                                 policy.scrollSpeed, policy.doubleClickMs, policy.haptics);
    m_protocol->setKeyboardLayout(policy.keyboardLayout, policy.keyboardVariant);
    // T200: the veil's accessibility variants are decided in the same policy
    // snapshot. Push them to QML so Reduce Transparency draws the opaque
    // scheme plate instead of leaving the compositor's skipped veil bare.
    if (m_item) {
        const bool changed =
            m_item->property("reduceTransparency").toBool() != policy.reduceTransparency
            || m_item->property("increaseContrast").toBool() != policy.increaseContrast;
        m_item->setProperty("reduceTransparency", policy.reduceTransparency);
        m_item->setProperty("increaseContrast", policy.increaseContrast);
        if (changed && m_protocol)
            render();
    }
    fprintf(stderr,
            "dragonfruit-shell: compositor motion/input policy applied "
            "(scheme=%s reducedMotion=%d transparency=%.4f reduceTransparency=%d "
            "increaseContrast=%d repeat=%d/%d gestures=%d)\n",
            qPrintable(policy.colorScheme), policy.reducedMotion ? 1 : 0,
            policy.transparency, policy.reduceTransparency ? 1 : 0,
            policy.increaseContrast ? 1 : 0, policy.repeatDelayMs, policy.repeatRateHz,
            policy.gesturesEnabled ? 1 : 0);
}

void ShellController::applyMenuBarPolicy()
{
    // T-14.2b: `menu.global` is the global application-menu toggle. When it is
    // off the bar stops showing the focused app's exported menus, so first-party
    // apps present their own local menus (the never-break-apps rule). The fixed
    // system and application menus stay.
    if (!m_settingsClient || !m_item)
        return;
    const QVariantMap values = m_settingsClient->values();
    const bool enabled = values.value(QStringLiteral("menu.global"), true).toBool();
    if (m_globalMenuEnabled != enabled) {
        m_globalMenuEnabled = enabled;
        m_item->setProperty("globalMenuEnabled", enabled);
        // Re-publish the focused menu so the bar reflects the toggle immediately.
        applyFocusedApp();
    }

    // T-15.9b: the Menu Bar pane's keys. The clock options and the background
    // material apply live; the auto-hide mode is forwarded to the compositor
    // (T201), which owns the per-output conceal/reveal, reservation and measured
    // transition (T233), and is mirrored on the bar for the scene's own rule.
    const bool showDate = values.value(QStringLiteral("menu.clock.showDate"), true).toBool();
    const bool showSeconds = values.value(QStringLiteral("menu.clock.showSeconds"), false).toBool();
    const bool use24Hour = values.value(QStringLiteral("clock.use24Hour"), false).toBool();
    const bool showBackground =
        values.value(QStringLiteral("menu.showBackground"), true).toBool();
    const QString autoHide =
        values.value(QStringLiteral("menu.autoHide"), QStringLiteral("full-screen")).toString();
    // T230: `menu.recentItems` now has a consumer: the system menu's Recent
    // Items submenu. Clamp to the schema's 0..50 range.
    const int recentItems =
        qBound(0, values.value(QStringLiteral("menu.recentItems"), 10).toInt(), 50);
    if (recentItems != m_menuRecentItems) {
        m_menuRecentItems = recentItems;
        refreshSystemMenu();
    }
    const bool changed = !m_menuBarPolicySent || showDate != m_menuBarShowDate
        || showSeconds != m_menuBarShowSeconds
        || use24Hour != m_menuBarUse24Hour
        || showBackground != m_menuBarShowBackground || autoHide != m_menuBarAutoHide;
    if (changed) {
        m_menuBarShowDate = showDate;
        m_menuBarShowSeconds = showSeconds;
        m_menuBarUse24Hour = use24Hour;
        m_menuBarShowBackground = showBackground;
        m_menuBarAutoHide = autoHide;
        m_menuBarPolicySent = true;
        m_item->setProperty("showDate", showDate);
        m_item->setProperty("showSeconds", showSeconds);
        m_item->setProperty("use24Hour", use24Hour);
        m_item->setProperty("showBackground", showBackground);
        m_item->setProperty("autoHideMode", autoHide);
        // T201: the compositor owns the per-output reveal. Forward the policy
        // through the same Settings refresh path as the rest of the pane.
        if (m_protocol)
            m_protocol->setMenuBarAutoHide(autoHide);
        render();
        fprintf(stderr,
                "dragonfruit-shell: menu bar policy applied "
                "(globalMenu=%d clockDate=%d clockSeconds=%d clock24h=%d background=%d "
                "autoHide=%s)\n",
                m_globalMenuEnabled ? 1 : 0, showDate ? 1 : 0, showSeconds ? 1 : 0,
                use24Hour ? 1 : 0, showBackground ? 1 : 0, qPrintable(autoHide));
    }
    // The per-control keys (`menu.control.*`, including T231's clock,
    // notifications and Quick Settings rows) gate the registry live.
    applyStatusItems();
}

void ShellController::applyWallpaperPolicy()
{
    // T-09.3 / T-18.1b: one view of the settingsd wallpaper keys plus the
    // provider's resolved defaults, forwarded to the compositor as per-Space
    // `df_workspace.set_wallpaper`. The compositor is the sole applier; the
    // shell keeps no second settings source. The effective source is the user
    // choice, then the shipped default, then the fetched fallback (ADR 0094).
    if (!m_protocol)
        return;
    // The client starts on the schema defaults. When settingsd is present its
    // first `GetAll` snapshot is authoritative, so do not publish a default
    // wallpaper (and decode it in the compositor) only to replace it a moment
    // later with the persisted choice: wait for the snapshot, then send the
    // effective source exactly once. A missing daemon never blocks this path.
    if (m_settingsClient && m_settingsClient->isAvailable()
        && !m_settingsSnapshotReceived) {
        return;
    }
    WallpaperSettings settings = wallpaperSettingsFromValues(
        m_settingsClient ? m_settingsClient->values() : QVariantMap());
    // The settingsd keys persist, but the provider's live properties are
    // authoritative when a key is empty (settingsd absent, or not yet written).
    if (settings.builtinDefault.isEmpty() && !m_providerBuiltinDefault.isEmpty())
        settings.builtinDefault = m_providerBuiltinDefault;
    if (settings.providerSource.isEmpty() && !m_providerDefault.isEmpty())
        settings.providerSource = m_providerDefault;
    // Last resort (provider absent too): resolve the shipped asset locally so
    // the out-of-box background never depends on a service being up.
    if (settings.builtinDefault.isEmpty())
        settings.builtinDefault = shippedDefaultWallpaperPath();

    const QString effective = settings.effectiveSource();
    if (m_wallpaperSent && settings == m_wallpaperSettings)
        return;
    m_wallpaperSettings = settings;
    m_wallpaperSent = true;
    m_protocol->setWallpaper(effective, wallpaperFitFromName(settings.fit),
                             settings.showOnAllSpaces);
    fprintf(stderr,
            "dragonfruit-shell: wallpaper applied (fit=%s allSpaces=%d source=%s)\n",
            qPrintable(settings.fit), settings.showOnAllSpaces ? 1 : 0,
            effective.isEmpty() ? "(solid)" : qPrintable(effective));
    updateMenuBarLuminance();
}

void ShellController::updateMenuBarLuminance()
{
    if (!m_item || !m_protocol)
        return;
    QString source = m_protocol->activeWallpaperSource();
    QColor color = m_protocol->activeWallpaperColor();
    if (source.isEmpty() && !color.isValid())
        source = m_wallpaperSettings.effectiveSource();
    const QString key = source + QLatin1Char('|')
            + (color.isValid() ? color.name(QColor::HexArgb) : QString());
    auto cached = m_menuBarLumaCache.constFind(key);
    if (cached == m_menuBarLumaCache.constEnd())
        cached = m_menuBarLumaCache.insert(key, wallpaperTopBandLuminance(source, color));
    m_item->setProperty("backgroundLuma", cached.value());
}

void ShellController::connectAppIndex()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    // The coalesced `Changed` signal is registered once and follows the
    // well-known name across service restarts (the settingsd client uses the
    // same pattern). It is safe to register before app-index owns the name.
    bus.connect(kAppIndexService, kAppIndexPath, kAppIndexInterface,
                QStringLiteral("Changed"), this, SLOT(onAppIndexChanged(QString)));
    auto *watcher = new QDBusServiceWatcher(
        kAppIndexService, bus,
        QDBusServiceWatcher::WatchForRegistration
            | QDBusServiceWatcher::WatchForUnregistration,
        this);
    connect(watcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) { syncAppIndex(); });
    connect(watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                // Drop the subscription so the next owner gets a fresh one.
                // Keep the last corpus while the service restarts so the Dock
                // does not lose its icons for the gap.
                m_appIndexSubscribed = false;
                syncAppIndex();
            });
    syncAppIndex();
}

void ShellController::syncAppIndex()
{
    const bool available = m_appIndexClient.available();
    if (available) {
        // Subscribe before the snapshot so a change that lands between the
        // two cannot be missed; the reload then has the latest corpus.
        if (!m_appIndexSubscribed)
            m_appIndexSubscribed =
                !m_appIndexClient.subscribe(QStringLiteral("identity")).isEmpty();
        m_index.loadFromAppIndex(m_appIndexClient);
        seedDefaultDockPins();
    } else {
        m_appIndexSubscribed = false;
    }
    // T230: identity is now available, so resolve the recent app ids to names
    // for the system menu's Recent Items submenu.
    refreshSystemMenu();
    if (m_dockItem) {
        rebuildDockEntries();
        refreshAppPicker(false);
        if (m_appsDrawerOpen)
            refreshAppsDrawer(false);
    }
    if (available)
        refreshTrayItems();
}

void ShellController::connectWallpaperProvider()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_wallpaperWatcher = new QDBusServiceWatcher(
        kWallpaperService, bus,
        QDBusServiceWatcher::WatchForRegistration | QDBusServiceWatcher::WatchForUnregistration,
        this);
    // A provider (re)start re-reads its resolved defaults; an unregistration
    // keeps the last known values so a restart leaves the desktop unchanged.
    connect(m_wallpaperWatcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) { refreshWallpaperProvider(); });
    // The standard property notification covers `DefaultSource` and
    // `BuiltinDefaultSource`; the contract signals are a belt-and-braces
    // refresh for a first fill.
    bus.connect(kWallpaperService, kWallpaperPath,
                QStringLiteral("org.freedesktop.DBus.Properties"),
                QStringLiteral("PropertiesChanged"), this,
                SLOT(onWallpaperProviderPropertiesChanged(QString, QVariantMap, QStringList)));
    bus.connect(kWallpaperService, kWallpaperPath, kWallpaperInterface,
                QStringLiteral("ItemsChanged"), this,
                SLOT(onWallpaperProviderItemsChanged()));
    refreshWallpaperProvider();
}

void ShellController::refreshWallpaperProvider()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    const auto readProperty = [&bus](const QString &name) -> QString {
        QDBusMessage call = QDBusMessage::createMethodCall(
            kWallpaperService, kWallpaperPath,
            QStringLiteral("org.freedesktop.DBus.Properties"), QStringLiteral("Get"));
        call << kWallpaperInterface << name;
        const QDBusMessage reply = bus.call(call);
        if (reply.type() != QDBusMessage::ReplyMessage || reply.arguments().isEmpty())
            return QString();
        QVariant value = reply.arguments().constFirst();
        if (value.userType() == qMetaTypeId<QDBusVariant>())
            value = value.value<QDBusVariant>().variant();
        return value.toString();
    };
    const QString providerDefault = readProperty(QStringLiteral("DefaultSource"));
    const QString builtinDefault = readProperty(QStringLiteral("BuiltinDefaultSource"));
    if (providerDefault == m_providerDefault && builtinDefault == m_providerBuiltinDefault)
        return;
    m_providerDefault = providerDefault;
    m_providerBuiltinDefault = builtinDefault;
    applyWallpaperPolicy();
}

void ShellController::onWallpaperProviderItemsChanged()
{
    // A first fill or a refreshed catalogue may have moved DefaultSource; the
    // property read is local and cheap.
    refreshWallpaperProvider();
}

void ShellController::onWallpaperProviderPropertiesChanged(const QString &interface,
                                                           const QVariantMap &changed,
                                                           const QStringList &invalidated)
{
    Q_UNUSED(interface);
    Q_UNUSED(invalidated);
    bool moved = false;
    if (changed.contains(QStringLiteral("DefaultSource"))) {
        m_providerDefault = changed.value(QStringLiteral("DefaultSource")).toString();
        moved = true;
    }
    if (changed.contains(QStringLiteral("BuiltinDefaultSource"))) {
        m_providerBuiltinDefault =
            changed.value(QStringLiteral("BuiltinDefaultSource")).toString();
        moved = true;
    }
    if (moved)
        applyWallpaperPolicy();
}

void ShellController::waitForInitialSettings()
{
    // The shell's first chrome configure is also the compositor's first
    // display-scale use. settingsd is a session dependency (Type=dbus), but its
    // `GetAll` reply is asynchronous; without this wait the shell forwards the
    // schema default (1.0), renders at that scale, and only then receives the
    // persisted value and reconfigures. Waiting here, before the QML engine and
    // surfaces exist, makes the first configure use the persisted scale.
    if (!m_settingsClient || !m_settingsClient->isAvailable()
        || m_settingsSnapshotReceived) {
        return;
    }

    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    const QMetaObject::Connection refreshed = connect(
        m_settingsClient, &SettingsClient::refreshed, &loop, &QEventLoop::quit);
    connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    timeout.start(kInitialSettingsWaitMs);
    loop.exec();
    disconnect(refreshed);

    if (!m_settingsSnapshotReceived) {
        qWarning() << "shell: settingsd did not answer the initial GetAll;"
                   << "starting on schema defaults";
    }
}

void ShellController::waitForDisplayPolicy()
{
    if (!m_protocol || !m_displaySent || !m_settingsSnapshotReceived)
        return;
    const qreal desired = ChromeScale::sanitize(m_displaySettings.scale);
    const auto applied = [this, desired]() {
        return qFuzzyCompare(m_protocol->outputScale(), desired);
    };
    if (applied())
        return;

    // The compositor answers `df_output.set_scale` with the new output
    // geometry followed by `df_output.scale`; when the scale event arrives the
    // logical geometry is already current. Creating a layer before it would
    // configure against the old size (the oversized first menu-bar frame).
    QEventLoop loop;
    QTimer timeout;
    timeout.setSingleShot(true);
    const QMetaObject::Connection scaleChanged = connect(
        m_protocol, &ShellProtocol::outputScaleChanged, &loop, [&]() {
            if (applied())
                loop.quit();
        });
    connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
    timeout.start(kInitialDisplayWaitMs);
    loop.exec();
    disconnect(scaleChanged);
    if (!applied()) {
        qWarning() << "shell: compositor did not apply display scale" << desired
                   << "before the first chrome configure; continuing";
    }
}

void ShellController::applyDisplayPolicy()
{
    // T-09.5: one view of the settingsd display keys, forwarded to the
    // compositor as `df_output.set_scale` / `df_output.set_transform`. The
    // compositor is the sole applier; the shell keeps no second settings
    // source. The protocol retries once the manager has announced its outputs.
    if (!m_settingsClient || !m_protocol)
        return;
    // Never publish the schema default scale while the daemon's first snapshot
    // is still in flight; the refreshed signal re-runs this with the persisted
    // value (the same rule the wallpaper forwarder follows).
    if (m_settingsClient->isAvailable() && !m_settingsSnapshotReceived)
        return;
    const DisplaySettings settings = displaySettingsFromValues(m_settingsClient->values());
    if (m_displaySent && settings == m_displaySettings)
        return;
    m_displaySettings = settings;
    m_displaySent = true;
    m_protocol->setDisplayPolicy(settings.scale, outputTransformFromName(settings.rotation),
                                 settings.brightness);
    fprintf(stderr, "dragonfruit-shell: display applied (scale=%.3f rotation=%s brightness=%.3f)\n",
            settings.scale, qPrintable(settings.rotation), settings.brightness);
}

void ShellController::onDockStateChanged(const QVariantList &entries)
{
    if (entries == m_runningEntries)
        return;
    m_runningEntries = entries;
    // T230: a window that closed while hidden must not stay in the Show All
    // set (the compositor handle is gone).
    if (!m_hiddenWindowIds.isEmpty()) {
        const QStringList retained = retainWindowIds(entries, m_hiddenWindowIds);
        QSet<QString> pruned;
        for (const QString &windowId : retained)
            pruned.insert(windowId);
        m_hiddenWindowIds = pruned;
    }
    // The fixed application menu's Hide/Hide Others/Show All reflect the live
    // window state, so a minimize/restore/open/close re-publishes it (T-14.2a).
    applyFocusedApp();
    // A window appeared: a pending launch for that app has succeeded, so its
    // transient launching/failed state clears (T-10 section 8.4). Focus the
    // first window the launch produced: a Dock/launcher click is an explicit
    // activation, and a newly launched terminal or Settings must accept
    // typing without a second click on its content.
    QStringList launchedApps;
    for (const QVariant &value : entries) {
        const QVariantMap map = value.toMap();
        if (map.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        const DesktopEntry resolved =
            m_index.resolve(map.value(QStringLiteral("appId")).toString());
        if (resolved.valid) {
            if (m_launchStates.remove(resolved.id))
                launchedApps.append(map.value(QStringLiteral("appId")).toString());
            m_launchDeadlines.remove(resolved.id);
        }
    }
    if (m_launchDeadlines.isEmpty() && m_launchTimer && m_launchTimer->isActive())
        m_launchTimer->stop();
    for (const QString &appId : launchedApps) {
        // The compositor just reported the window, so it has the app; ask it
        // to focus the launch's window. No fallback launch here: a race would
        // otherwise start a duplicate process.
        if (!appId.isEmpty() && m_protocol)
            m_protocol->activateApp(appId);
    }
    rebuildDockEntries();
}

void ShellController::rebuildDockEntries()
{
    if (!m_dockItem)
        return;
    // The entries are the stable model: they never carry a launch/attention
    // phase, so a running bounce does not reset the Repeater. The phases are
    // published separately (T-14.7c).
    QVariantList running = m_runningEntries;
    if (m_dockOverflowFixture) {
        // Synthetic unpinned running groups so the overflow planner has more
        // groups than the capped fixture width fits. Distinct names make the
        // "More Windows" rows legible in the capture.
        static const char *const kNames[] = {"Calendar", "Music", "Photos",
                                             "Maps", "Weather", "Notes",
                                             "Podcasts", "Contacts"};
        for (int i = 0; i < 8; ++i) {
            const QString id = QStringLiteral("overflow-fixture-%1").arg(i + 1);
            QVariantList windows;
            windows.append(QVariantMap{
                {QStringLiteral("windowId"), id + QStringLiteral("-w1")},
                {QStringLiteral("title"), QString::fromLatin1(kNames[i])},
                {QStringLiteral("focused"), i == 0}});
            QVariantMap group;
            group.insert(QStringLiteral("id"), id);
            group.insert(QStringLiteral("appId"), id);
            group.insert(QStringLiteral("name"), QString::fromLatin1(kNames[i]));
            group.insert(QStringLiteral("kind"), QStringLiteral("temporary"));
            group.insert(QStringLiteral("running"), true);
            group.insert(QStringLiteral("windows"), 1);
            group.insert(QStringLiteral("windowList"), windows);
            running.append(group);
        }
    }
    m_dockAllEntries = buildDockEntries(
        m_dockConfig.pinned, m_index, running, m_launchStates,
        m_dockConfig.showRecentApps ? m_recentAppIds : QStringList());
    // The Applications launcher (T-19.2 follow-up) is a permanent Dock tile:
    // it is not part of the user-orderable `dock.pinned` set and cannot be
    // removed. Inject it immediately after the first pinned tile (Files by
    // default), matching the reference placement, or at the front when nothing
    // is pinned. The Dock renders `appsLauncher: true` as the grid tile and
    // toggles the drawer on click.
    {
        QVariantMap launcher;
        launcher.insert(QStringLiteral("id"), QStringLiteral("__apps__"));
        launcher.insert(QStringLiteral("appId"), QString());
        launcher.insert(QStringLiteral("desktopId"), QString());
        launcher.insert(QStringLiteral("name"), tr("Applications"));
        launcher.insert(QStringLiteral("icon"), QString());
        launcher.insert(QStringLiteral("iconPath"), QString());
        launcher.insert(QStringLiteral("kind"), QStringLiteral("pinned"));
        launcher.insert(QStringLiteral("pinned"), true);
        launcher.insert(QStringLiteral("appsLauncher"), true);
        launcher.insert(QStringLiteral("missing"), false);
        launcher.insert(QStringLiteral("running"), false);
        launcher.insert(QStringLiteral("windows"), 0);
        launcher.insert(QStringLiteral("windowList"), QVariantList());
        launcher.insert(QStringLiteral("windowCount"), 0);
        int insertAt = 0;
        for (int i = 0; i < m_dockAllEntries.size(); ++i) {
            if (m_dockAllEntries.at(i).toMap().value(QStringLiteral("kind")).toString()
                == QLatin1String("pinned")) {
                insertAt = i + 1;
                break;
            }
        }
        m_dockAllEntries.insert(insertAt, launcher);
    }
    // A window that just minimized starts its app's one-shot reaction pulse
    // (T-14.7s) before the entries are clamped/published. The pulse rides the
    // phase map below, never the entry model.
    detectDockMinimizePulses();
    // A full rebuild always re-publishes the entries; the clamp only decides
    // which ones survive and at what icon size.
    applyDockOverflowResult(computeDockOverflow(), true, true);
    publishDockBouncePhases();
    scheduleDockRender();
}

// The Add Application picker's data (T-14.7e). The pure `buildAppPickerList`
// helper owns dedupe/filter/sort/tag; the shell pushes its result as plain
// rows. `reloadCorpus` is the one-shot re-enumerate when the picker opens and
// no live subscription is held.
void ShellController::refreshAppPicker(bool reloadCorpus)
{
    if (!m_dockItem)
        return;
    if (reloadCorpus && !m_appIndexSubscribed)
        m_index.loadFromAppIndex(m_appIndexClient);

    const QList<AppPickerRow> rows = buildAppPickerList(m_index.entries(), m_dockConfig.pinned);
    QVariantList list;
    list.reserve(rows.size());
    for (const AppPickerRow &row : rows) {
        list.append(QVariantMap{
            {QStringLiteral("desktopId"), row.desktopId},
            {QStringLiteral("name"), row.name},
            {QStringLiteral("iconPath"), row.iconPath},
            {QStringLiteral("pinned"), row.pinned},
        });
    }
    m_dockItem->setProperty("appPickerItems", list);
    m_dockItem->setProperty("appIndexAvailable", m_appIndexClient.available());
    scheduleDockRender();
}

void ShellController::onDockAppPickerRequested()
{
    refreshAppPicker(true);
}

void ShellController::onDockAppPinToggled(const QString &desktopId, bool pinned)
{
    if (desktopId.isEmpty())
        return;
    const QStringList ids = toggleAppPickerPin(m_dockConfig.pinned, desktopId, pinned);
    if (ids == m_dockConfig.pinned)
        return;
    // The single writer path: settingsd owns `dock.pinned`; the picker and the
    // Dock both re-read it, so they cannot disagree (ADR 0090).
    writeDockSetting(QStringLiteral("dock.pinned"), ids);
    // The pinned set changed, so every recorded tile may describe a moved or
    // removed entry; drop them and let the next click repopulate (T-14.7l).
    m_dockTiles.clear();
    rebuildDockEntries();
    refreshAppPicker(false);
}

void ShellController::onAppIndexChanged(const QString &interests)
{
    Q_UNUSED(interests);
    // An install/uninstall/update (T-14.1c) can change the picker's corpus and
    // the Dock's identity resolution; reload both.
    m_index.loadFromAppIndex(m_appIndexClient);
    rebuildDockEntries();
    refreshAppPicker(false);
    // An install/uninstall while the drawer is open refreshes its grid live
    // (T-19.2); when it is closed the next open re-reads anyway.
    if (m_appsDrawerOpen)
        refreshAppsDrawer(false);
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
    int available = m_dockPosition == ShellProtocol::DockPosition::Bottom
            ? m_dockWidth
            : m_dockHeight;
    // The capture seam caps the effective axis so the overflow cell appears on
    // a full-size nested output (T-14.7q); a normal session never sets it.
    if (m_dockOverflowFixture && m_dockOverflowFixtureWidth > 0)
        available = qMin(available, m_dockOverflowFixtureWidth);
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
    m_dockThickness = qCeil(m_dockItem->property("surfaceThickness").toReal());
    warnDockOverflow(overflow);
    if (!allowEntries)
        return;
    const bool hiddenChanged = overflow.hiddenTemporary != m_dockHiddenTemporary
            || overflow.hiddenRecent != m_dockHiddenRecent
            || overflow.overflowShown != m_dockOverflowShown;
    // A geometry change during a divider resize must not destroy the delegate
    // that holds the pointer, so only reset the Repeater model when the hidden
    // set actually changed (the internal-reorder lesson); a full rebuild
    // forces the fresh entries through.
    if (!hiddenChanged && !forceEntries)
        return;
    m_dockHiddenTemporary = overflow.hiddenTemporary;
    m_dockHiddenRecent = overflow.hiddenRecent;
    m_dockOverflowShown = overflow.overflowShown;
    m_dockItem->setProperty("entries", overflow.entries);
}

void ShellController::warnDockOverflow(const DockOverflowResult &overflow)
{
    if (!overflow.clamped || m_dockOverflowWarned)
        return;
    m_dockOverflowWarned = true;
    qWarning() << "shell: Dock content exceeds the output; clamped the icon size to"
               << overflow.iconSize << "px, folded"
               << overflow.hiddenTemporary << "running groups into the overflow cell and hid"
               << overflow.hiddenRecent << "recent entries"
               << (overflow.overflowed ? "(non-droppable content still overflows)" : "");
}

void ShellController::detectDockMinimizePulses()
{
    const QHash<QString, int> counts = dockMinimizedCounts(m_dockAllEntries);
    if (m_dockMinimizedCountsSeeded && m_dockConfig.minimizeReaction) {
        const QStringList pulses =
            dockMinimizedPulses(m_dockAllEntries, m_dockMinimizedCounts);
        if (!pulses.isEmpty()) {
            const qint64 now = QDateTime::currentMSecsSinceEpoch();
            for (const QString &id : pulses)
                m_minimizeReactionStart.insert(id, now);
            ensureDockAnimation();
        }
    }
    m_dockMinimizedCounts = counts;
    m_dockMinimizedCountsSeeded = true;
}

void ShellController::publishDockBouncePhases()
{
    if (!m_dockItem)
        return;
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    QVariantMap phases;
    bool fixtureAssigned = false;
    for (const QVariant &value : m_dockAllEntries) {
        const QVariantMap entry = value.toMap();
        const QString id = entry.value(QStringLiteral("id")).toString();
        if (id.isEmpty())
            continue;
        const QString appId = entry.value(QStringLiteral("appId")).toString();
        const QString desktopId = entry.value(QStringLiteral("desktopId")).toString();
        const QString kind = entry.value(QStringLiteral("kind")).toString();
        double phase = -1.0;
        bool attention = false;
        if (!appId.isEmpty() && now < m_attentionUntil.value(appId)) {
            // Attention bounce wins over a launch bounce for the same entry
            // and is taller/repeating (FR-4).
            attention = true;
            phase = dockAttentionBouncePhase(now - m_attentionStart.value(appId));
        } else if (!desktopId.isEmpty() && m_launchStart.contains(desktopId)) {
            phase = dockLaunchBouncePhase(now - m_launchStart.value(desktopId));
        }
        // The one-shot minimize reaction (T-14.7s) is its own field: it is
        // opt-in and rendered separately so a launch/attention hop and a
        // minimize pulse never fight for the same geometry. The capture seam
        // pins a constant phase on the first running entry.
        double minimizePhase = -1.0;
        if (m_minimizeReactionFixture && !fixtureAssigned
            && entry.value(QStringLiteral("running")).toBool()
            && (kind == QLatin1String("pinned") || kind == QLatin1String("temporary"))) {
            minimizePhase = m_minimizeReactionFixturePhase;
            fixtureAssigned = true;
        } else if (m_dockConfig.minimizeReaction && m_minimizeReactionStart.contains(id)) {
            minimizePhase = dockMinimizeReactionPhase(now - m_minimizeReactionStart.value(id));
        }
        if (phase < 0.0 && !attention && minimizePhase < 0.0)
            continue;
        QVariantMap info;
        info.insert(QStringLiteral("phase"), phase);
        info.insert(QStringLiteral("attention"), attention);
        if (minimizePhase >= 0.0) {
            info.insert(QStringLiteral("minimize"), true);
            info.insert(QStringLiteral("minimizePhase"), minimizePhase);
        }
        phases.insert(id, info);
    }
    // The idle Dock must not churn QML bindings: only push a real change.
    if (phases == m_dockBouncePhases)
        return;
    m_dockBouncePhases = phases;
    m_dockItem->setProperty("bouncePhases", phases);
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
    // Only the phase map changes; the entry model (and its delegates) stays.
    publishDockBouncePhases();
    scheduleDockRender();
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
    // The attention state rides the phase map, so the entry model is untouched
    // and any live hover/press on the entry survives the bounce (T-14.7c).
    publishDockBouncePhases();
    scheduleDockRender();
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
    for (auto it = m_minimizeReactionStart.begin(); it != m_minimizeReactionStart.end();) {
        if (now - it.value() >= kMinimizeReactionMs)
            it = m_minimizeReactionStart.erase(it);
        else
            ++it;
    }
    // Advance only the phase map; the Repeater model is not touched, so the
    // delegates are not recreated and the hover/press state survives
    // (T-14.7c). A settled clock stops the timer entirely (zero idle wakeups).
    publishDockBouncePhases();
    scheduleDockRender();
    if (m_attentionUntil.isEmpty() && m_launchStart.isEmpty()
        && m_minimizeReactionStart.isEmpty()) {
        if (m_dockAnimTimer)
            m_dockAnimTimer->stop();
    }
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
        activateAppOrFallback(map.value(QStringLiteral("appId")).toString(),
                              map.value(QStringLiteral("desktopId")).toString());
        return;
    }

    if (map.value(QStringLiteral("running")).toBool()) {
        activateAppOrFallback(map.value(QStringLiteral("appId")).toString(),
                              map.value(QStringLiteral("desktopId")).toString());
        return;
    }

    if (map.value(QStringLiteral("missing")).toBool()) {
        // A pinned identity the app-index cannot resolve is not silent: it is
        // a real notice, and the tile keeps its not-found mark (T-14.7g).
        const QString name = map.value(QStringLiteral("name")).toString().isEmpty()
                                 ? displayNameForIdentity(
                                       map.value(QStringLiteral("desktopId")).toString())
                                 : map.value(QStringLiteral("name")).toString();
        qWarning() << "shell: Dock entry is not installed (app-index):"
                   << map.value(QStringLiteral("desktopId")).toString();
        raiseDockNotice(m_notificationClient, tr("%1 is not installed").arg(name),
                        tr("The application could not be found."));
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

void ShellController::onDockEntryTileRect(const QString &desktopId, qreal x, qreal y, qreal w,
                                          qreal h)
{
    // The Dock owns the tile geometry; the shell only remembers the last
    // acted-on rect and hands it to the compositor at launch (T-14.7l). Clamp
    // to the output so a stale or off-surface rect can never be recorded.
    if (desktopId.isEmpty() || w <= 0 || h <= 0)
        return;
    const QRect rect(qRound(x), qRound(y), qRound(w), qRound(h));
    const QRect output = m_protocol ? m_protocol->primaryOutputGeometry() : QRect();
    m_dockTiles.record(desktopId, clampDockTileRect(rect, output));
}

QPoint ShellController::dockSurfaceOutputOrigin() const
{
    const QRect output = m_protocol ? m_protocol->primaryOutputGeometry() : QRect();
    if (!output.isValid())
        return QPoint(0, 0);
    switch (m_dockPosition) {
    case ShellProtocol::DockPosition::Left:
        return QPoint(output.x(), output.y());
    case ShellProtocol::DockPosition::Right:
        return QPoint(output.x() + output.width() - m_dockThickness, output.y());
    case ShellProtocol::DockPosition::Bottom:
    default:
        return QPoint(output.x(), output.y() + output.height() - m_dockThickness);
    }
}

void ShellController::sendLaunchOrigin(const QString &desktopId)
{
    if (!m_protocol || desktopId.isEmpty())
        return;
    const QRect tile = m_dockTiles.rectFor(desktopId);
    if (!tile.isValid() || tile.width() <= 0 || tile.height() <= 0)
        return;
    const DesktopEntry entry = m_index.byId(desktopId);
    const QString appId = dockLaunchAppId(entry);
    if (appId.isEmpty()) {
        // No compositor-visible key could be resolved: keep the centered
        // fallback rather than send a guess (T-14.7l).
        return;
    }
    m_protocol->setLaunchOrigin(appId, tile.x(), tile.y(), tile.width(), tile.height());
}

void ShellController::activateAppOrFallback(const QString &appId, const QString &desktopId)
{
    // The compositor is the sole activation authority; the shell only asks and
    // (T-14.7g) reacts to the result. An empty app id has nothing to activate,
    // so it is a launch; a non-empty one is remembered so a not-found reply
    // can fall back to a launch or a notice instead of a dead click.
    if (appId.isEmpty()) {
        if (!desktopId.isEmpty()) {
            launchDockApp(desktopId);
        } else {
            raiseDockNotice(m_notificationClient, tr("Cannot open application"),
                            tr("The application could not be identified."));
        }
        return;
    }
    if (!m_protocol)
        return;
    m_pendingActivations.insert(appId, desktopId);
    m_protocol->activateApp(appId);
}

void ShellController::onActivationResult(const QString &appId, bool found)
{
    const QString desktopId = m_pendingActivations.take(appId);
    if (found)
        return;
    // The compositor found no window for the app. If the Dock knows the
    // installed entry, launch it (the app exited between the projection and
    // the click); otherwise make the failure visible.
    if (!desktopId.isEmpty()) {
        const DesktopEntry entry = m_index.byId(desktopId);
        if (DesktopEntryIndex::isLaunchable(entry)) {
            qInfo() << "shell: Dock activation found no window for" << appId
                    << "- launching" << entry.id;
            launchDockApp(entry.id);
            return;
        }
    }
    qWarning() << "shell: Dock activation found no window for" << appId;
    raiseDockNotice(m_notificationClient, tr("Could not switch to application"),
                    tr("It is no longer running and no launch command was found."));
}

void ShellController::launchFiles(const QString &argument)
{
    // Resolve Files through the interim index and launch it at `argument`
    // (T-10.6c). The design's `org.dragonfruit.Files1` service name is the
    // D-Bus activation target; the installed `.desktop` carries its `%U`
    // argument seam until the running-instance API (T-18) lands.
    DesktopEntry files = m_index.byId(QStringLiteral("org.dragonfruit.Files.desktop"));
    if (!files.valid)
        files = m_index.resolve(QStringLiteral("org.dragonfruit.Files"));
    if (!DesktopEntryIndex::isLaunchable(files)) {
        qWarning() << "shell: Files is not installed (T-10.6c):" << argument;
        return;
    }
    const QStringList argv = DesktopEntryIndex::buildLaunchCommand(files, {argument});
    if (argv.isEmpty()) {
        qWarning() << "shell: Files .desktop has no usable Exec for" << argument;
        return;
    }
    qint64 pid = 0;
    if (!launchDetachedFromShell(argv, m_waylandDisplay, &pid))
        qWarning() << "shell: cannot open Files at" << argument << ":" << argv.first();
    else
        qInfo() << "shell: opened Files at" << argument << "(pid" << pid << ")";
}

void ShellController::openTrashInFiles()
{
    launchFiles(QStringLiteral("trash://"));
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
    const QString target = revealExecutable(app);
    if (target.isEmpty()) {
        qWarning() << "shell: Show in Files: no executable to reveal for" << app.id;
        return;
    }
    launchFiles(target);
    qInfo() << "shell: revealing" << app.name << "at" << target;
}

void ShellController::launchDockApp(const QString &desktopId)
{
    launchDockAppWithFiles(desktopId, QStringList());
}

void ShellController::openApp(const QString &desktopId)
{
    // One entry point for the fixed menus (system "System Settings…",
    // application "Settings…", and any future first-party item): focus the
    // running window if the app is already open, otherwise launch it from its
    // cached app-index record.
    const AppOpenPlan plan = planAppOpen(m_index, m_runningEntries, desktopId);
    if (!plan.resolved) {
        qWarning() << "shell: cannot open app; no usable .desktop entry:" << desktopId;
        return;
    }
    if (plan.running && m_protocol) {
        activateAppOrFallback(plan.appId, plan.desktopId);
        return;
    }
    launchDockApp(plan.desktopId);
}

void ShellController::openSettingsDeepLink(const QString &url)
{
    // T232: the one Settings entry point for the extras' "… Settings…" rows.
    // A running Settings app owns `org.dragonfruit.SettingsApp1`; deliver the
    // link there and raise its window. Otherwise launch it with the link so the
    // app lands on the pane (SettingsShell::deepLink is the consumer).
    if (url.isEmpty())
        return;
    const QString service = QStringLiteral("org.dragonfruit.SettingsApp1");
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (bus.isConnected() && bus.interface()
            && bus.interface()->isServiceRegistered(service)) {
        QDBusMessage message = QDBusMessage::createMethodCall(
            service, QStringLiteral("/org/dragonfruit/SettingsApp1"), service,
            QStringLiteral("Open"));
        message << url;
        bus.send(message);
        const AppOpenPlan plan =
            planAppOpen(m_index, m_runningEntries,
                        QStringLiteral("org.dragonfruit.Settings.desktop"));
        if (plan.resolved && plan.running && m_protocol)
            activateAppOrFallback(plan.appId, plan.desktopId);
        return;
    }
    const DesktopEntry entry =
        m_index.byId(QStringLiteral("org.dragonfruit.Settings.desktop"));
    if (!DesktopEntryIndex::isLaunchable(entry)) {
        qWarning() << "shell: cannot open Settings deep link; no usable entry";
        return;
    }
    QStringList argv = DesktopEntryIndex::buildLaunchCommand(entry, QStringList());
    if (argv.isEmpty()) {
        qWarning() << "shell: cannot open Settings deep link; empty launch command";
        return;
    }
    argv << QStringLiteral("--deep-link") << url;
    if (!launchDetachedFromShell(argv, m_waylandDisplay, nullptr))
        qWarning() << "shell: could not open Settings deep link" << url;
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
    // Hand the acted-on entry's tile to the compositor before the child is
    // spawned, so its first mapped window plays the appear from the real icon
    // (T-14.7l). A missing rect or key is a no-op (centered fallback).
    sendLaunchOrigin(desktopId);
    qint64 pid = 0;
    if (!launchDetachedFromShell(argv, m_waylandDisplay, &pid)) {
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
    // The failed mark is transient; the durable record is the notification
    // (T-11.1b replaces T-10's badge-only notice).
    raiseDockLaunchFailure(m_notificationClient, m_index.byId(desktopId).name, reason);
    rebuildDockEntries();
    scheduleLaunchStateClear(desktopId);
}

void ShellController::scheduleLaunchStateClear(const QString &desktopId)
{
    // The transient Dock mark is the only "badge" left; the durable record
    // is the notification raised in `failDockLaunch` (T-11.1b).
    QTimer::singleShot(4000, this, [this, desktopId]() {
        if (m_launchStates.value(desktopId) == QLatin1String("failed")) {
            m_launchStates.remove(desktopId);
            rebuildDockEntries();
        }
    });
}

void ShellController::flashDuplicatePin(const QString &desktopId)
{
    if (desktopId.isEmpty() || !m_dockItem)
        return;
    // The Dock highlights the pinned entry it already has, so a duplicate
    // alias drop reads as a no-op rather than silence (T-14.7f).
    QMetaObject::invokeMethod(m_dockItem, "flashPin", Q_ARG(QVariant, desktopId));
    scheduleDockRender();
}

void ShellController::onDockLaunchTick()
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    bool any = false;
    bool changed = false;
    for (auto it = m_launchDeadlines.begin(); it != m_launchDeadlines.end();) {
        if (it.value() <= now) {
            const QString desktopId = it.key();
            it = m_launchDeadlines.erase(it);
            if (m_launchStates.value(desktopId) == QLatin1String("launching")) {
                m_launchStates.insert(desktopId, QStringLiteral("failed"));
                changed = true;
                qWarning() << "shell: Dock launch timed out for" << desktopId
                           << "(no window mapped)";
                raiseDockLaunchFailure(m_notificationClient, m_index.byId(desktopId).name,
                                       QStringLiteral("The app did not open in time."));
                scheduleLaunchStateClear(desktopId);
            }
        } else {
            any = true;
            ++it;
        }
    }
    if (!any && m_launchTimer)
        m_launchTimer->stop();
    // Only a real launch-state change resets the entry model; a quiet tick
    // during a bounce must not recreate the delegates (T-14.7c).
    if (changed) {
        rebuildDockEntries();
    } else {
        publishDockBouncePhases();
    }
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
    m_dockPointerPosition = QPointF(x, y);
    // The compositor delivers Dock-surface coordinates in scene space; the
    // scene is offset inside the offscreen window when a popover needs
    // headroom (above a bottom Dock) or a side gutter (beside a vertical
    // Dock).
    const QPointF p(x + m_dockItemOffsetX, y + m_dockItemOffsetY);
    sendChromePointer(m_dockWindow, QEvent::MouseMove, p, Qt::NoButton, m_dockButtons);
}

void ShellController::onDockPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_dockWindow)
        return;
    m_dockPointerPosition = QPointF(x, y);
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
    sendChromePointer(m_dockWindow, pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease,
                      p, qtButton, m_dockButtons);
}

void ShellController::onDockPointerLeft()
{
    m_dockPointerPosition.reset();
    if (!m_dockWindow)
        return;
    sendChromePointer(m_dockWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                      m_dockButtons);
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
    if (active && m_appsDrawerOpen)
        hideAppsDrawer();
    if (active && m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
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
    sendChromePointer(m_overviewWindow, QEvent::MouseMove, p, Qt::NoButton, m_overviewButtons);
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
    sendChromePointer(m_overviewWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p,
                        qtButton, m_overviewButtons);
    scheduleOverviewRender();
}

void ShellController::onOverviewPointerLeft()
{
    if (!m_overviewWindow)
        return;
    sendChromePointer(m_overviewWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_overviewButtons);
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
    prepareChromeScene(m_overviewWindow, m_overviewItem, QSize(m_overviewWidth, m_overviewHeight),
                       QSize(m_overviewWidth, m_overviewHeight), m_chromeScale);
    const QSize logical(m_overviewWidth, m_overviewHeight);
    commitChromeScene(m_overviewWindow,
                      {{QStringLiteral("overview"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitOverviewImage(image, logical);
                        }}});
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
    // The Cmd-Tab overlay is a competing transient surface.
    if (active && m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
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

// --- Notification banners and history (T-11.1a) ----------------------------

void ShellController::onNotificationAvailable(bool available)
{
    if (!m_notificationModel)
        return;
    // The Center stays mapped through a service restart: absence shows its
    // accessible unavailable state and the next `History()` repopulates it.
    if (m_notificationCenter)
        m_notificationCenter->setAvailable(available);
    applyStatusItems();
    if (available) {
        m_notificationClient->refresh();
    } else {
        // The service went away: clear all views and unmap the banner. The
        // Focus item hides with the empty policy.
        m_notificationModel->applyBannersJson(QByteArrayLiteral("[]"));
        m_notificationModel->applyHistoryJson(QByteArrayLiteral("[]"));
        m_notificationModel->applyFocusPolicyJson(QByteArrayLiteral("{}"));
    }
}

void ShellController::onNotificationBanners(const QByteArray &json)
{
    if (m_notificationModel)
        m_notificationModel->applyBannersJson(json);
}

void ShellController::onNotificationHistory(const QByteArray &json)
{
    if (!m_notificationModel)
        return;
    m_notificationModel->applyHistoryJson(json);
    // The Center's active rows are derived only from the service's history
    // fields (`reason`/`closedAt`/`suppressed`), never from the banner view.
    if (m_notificationCenter) {
        const bool available = m_notificationClient && m_notificationClient->isAvailable();
        m_notificationCenter->setAvailable(available);
        if (available)
            m_notificationCenter->setHistory(m_notificationModel->history());
    }
    // The bell's attention mark/count follows the active notification set.
    applyStatusItems();
}

void ShellController::onNotificationFocusPolicy(const QByteArray &json)
{
    if (!m_notificationModel)
        return;
    m_notificationModel->applyFocusPolicyJson(json);
    applyStatusItems();
    // A live Focus/DND change reaches an open Control Center too (the tile
    // reflects the mode and the suppression count).
    if (m_controlCenterOpen)
        applyControlCenterData();
}

void ShellController::showCurrentBanner()
{
    if (!m_bannerItem || !m_notificationModel)
        return;
    const QVariantMap banner = m_notificationModel->banner();
    if (banner.isEmpty()) {
        hideCurrentBanner();
        return;
    }
    const QVariantList actions = banner.value(QStringLiteral("actions")).toList();
    m_bannerId = banner.value(QStringLiteral("id")).toUInt();
    m_bannerItem->setProperty("appName", banner.value(QStringLiteral("appName")));
    m_bannerItem->setProperty("summary", banner.value(QStringLiteral("summary")));
    m_bannerItem->setProperty("body", banner.value(QStringLiteral("body")));
    m_bannerItem->setProperty("urgency", banner.value(QStringLiteral("urgency")));
    m_bannerItem->setProperty("actions", actions);
    // The card is taller when it carries action buttons; only the card is
    // clickable, so the transparent strip below a no-action banner passes
    // clicks through (T-11.1b).
    applyBannerInputRegion();
    m_bannerPending = true;
    if (m_bannerWidth > 0 && m_bannerHeight > 0)
        renderBanner();
    // T205: a banner appearing while the Center is open moves the stack down.
    if (m_notificationCenterOpen)
        applyNotificationCenterData();
}

void ShellController::applyBannerInputRegion()
{
    if (!m_protocol)
        return;
    // Before the surface is created the call is a no-op and the region is
    // re-applied from `onBannerConfigured` once it exists.
    if (!m_notificationModel || !m_notificationModel->hasBanner()) {
        m_protocol->setBannerInputRegion(0, 0);
        return;
    }
    const QVariantList actions =
        m_notificationModel->banner().value(QStringLiteral("actions")).toList();
    const int cardHeight = actions.isEmpty() ? kBannerCardHeight : kBannerCardActionsHeight;
    m_protocol->setBannerInputRegion(kBannerWidth, cardHeight);
}

void ShellController::hideCurrentBanner()
{
    m_bannerPending = false;
    m_bannerId = 0;
    if (m_protocol) {
        m_protocol->setBannerInputRegion(0, 0);
        m_protocol->clearBannerPanelRegions();
        m_protocol->hideBanner();
    }
    // T205: the Center reclaims the top strip once the banner is gone.
    if (m_notificationCenterOpen)
        applyNotificationCenterData();
}

void ShellController::onBannerConfigured(int width, int height, quint32)
{
    // The compositor sends a pre-layout configure at the full output size
    // before applying the layer surface's anchor/size; skip it (the same rule
    // as the menu bar and Dock).
    if (width != kBannerWidth || height != kBannerSurfaceHeight) {
        fprintf(stderr, "dragonfruit-shell: ignoring pre-layout notification configure %dx%d\n",
                width, height);
        return;
    }
    m_bannerWidth = width;
    m_bannerHeight = height;
    fprintf(stderr, "dragonfruit-shell: notification banner configured %dx%d\n", width, height);
    // The surface exists now; apply the card-sized input region for the
    // banner that may already be pending.
    applyBannerInputRegion();
    if (m_bannerPending)
        renderBanner();
}

void ShellController::onBannerActionInvoked(const QString &actionKey)
{
    if (!m_notificationClient || m_bannerId == 0)
        return;
    qInfo() << "shell: notification action" << actionKey << "on" << m_bannerId;
    m_notificationClient->invoke(m_bannerId, actionKey);
}

void ShellController::onBannerActivated()
{
    // A click on the banner body fires the app's "default" action when it
    // registered one; otherwise the banner is dismissed (there is nothing to
    // open).
    if (m_notificationModel) {
        const QVariantList actions =
            m_notificationModel->banner().value(QStringLiteral("actions")).toList();
        for (const QVariant &action : actions) {
            if (action.toMap().value(QStringLiteral("key")).toString()
                == QLatin1String("default")) {
                onBannerActionInvoked(QStringLiteral("default"));
                return;
            }
        }
    }
    if (m_notificationClient && m_bannerId != 0)
        m_notificationClient->dismiss(m_bannerId);
}

void ShellController::onBannerPointerMoved(qreal x, qreal y)
{
    if (!m_bannerWindow)
        return;
    const QPointF p(x, y);
    sendChromePointer(m_bannerWindow, QEvent::MouseMove, p, Qt::NoButton, m_bannerButtons);
    scheduleBannerRender();
}

void ShellController::onBannerPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_bannerWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    if (pressed)
        m_bannerButtons |= qtButton;
    else
        m_bannerButtons &= ~qtButton;
    const QPointF p(x, y);
    sendChromePointer(m_bannerWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, p,
                        qtButton, m_bannerButtons);
    scheduleBannerRender();
}

void ShellController::onBannerPointerLeft()
{
    if (!m_bannerWindow)
        return;
    sendChromePointer(m_bannerWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_bannerButtons);
    scheduleBannerRender();
}

void ShellController::scheduleBannerRender()
{
    if (m_bannerRenderPending)
        return;
    m_bannerRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_bannerRenderPending = false;
        renderBanner();
    });
}

void ShellController::renderBanner()
{
    if (!m_bannerPending || !m_bannerWindow || !m_bannerItem)
        return;
    if (m_bannerWidth <= 0 || m_bannerHeight <= 0)
        return;
    prepareChromeScene(m_bannerWindow, m_bannerItem, QSize(m_bannerWidth, m_bannerHeight),
                       QSize(m_bannerWidth, m_bannerHeight), m_chromeScale);
    // T205: declare the visible card, not the transparent strip below a
    // no-action banner, before the image commit latches the region list.
    if (m_protocol) {
        m_protocol->setBannerPanelRegions(
            m_bannerItem->property("materialRegions").toList());
    }
    const QSize logical(m_bannerWidth, m_bannerHeight);
    commitChromeScene(m_bannerWindow,
                      {{QStringLiteral("banner"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitBannerImage(image, logical);
                        }}});
}

void ShellController::renderSwitcher()
{
    if (!m_switcherActive || !m_switcherWindow || !m_switcherItem)
        return;
    if (m_switcherWidth <= 0 || m_switcherHeight <= 0)
        return;
    prepareChromeScene(m_switcherWindow, m_switcherItem, QSize(m_switcherWidth, m_switcherHeight),
                       QSize(m_switcherWidth, m_switcherHeight), m_chromeScale);
    const QSize logical(m_switcherWidth, m_switcherHeight);
    commitChromeScene(m_switcherWindow,
                      {{QStringLiteral("switcher"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitSwitcherImage(image, logical);
                        }}});
}

// --- Applications drawer (T-19.2) -------------------------------------------

void ShellController::onAppsDrawerRequested()
{
    toggleAppsDrawer();
}

void ShellController::toggleAppsDrawer()
{
    if (m_appsDrawerOpen)
        hideAppsDrawer();
    else
        showAppsDrawer();
}

void ShellController::showAppsDrawer()
{
    if (!m_appsDrawerItem)
        return;
    if (m_notificationCenterOpen)
        hideNotificationCenter(QStringLiteral("competing"));
    if (m_controlCenterOpen)
        hideControlCenter();
    if (m_item) {
        QMetaObject::invokeMethod(m_item, "closeMenus");
        QMetaObject::invokeMethod(m_item, "closeStatusMenu");
    }
    if (m_overviewActive && m_protocol)
        m_protocol->exitMissionControl();
    m_appsDrawerOpen = true;
    m_appsDrawerPending = true;
    refreshAppsDrawer(true);
    m_appsDrawerItem->setProperty("active", true);
    if (m_appsDrawerWidth > 0 && m_appsDrawerHeight > 0)
        renderAppsDrawer();
    // Put the caret in the search field before the compositor hands us the
    // keyboard, not just on the root item: the drawer is a typing surface.
    QMetaObject::invokeMethod(m_appsDrawerItem, "focusSearch");
    if (m_protocol) {
        m_protocol->setAppsDrawerInputRegion(m_appsDrawerWidth, m_appsDrawerHeight);
        // The drawer is a full-output modal surface opened by a Dock click;
        // take the keyboard from the Dock so typing reaches the search field
        // immediately, not after a second click on the field.
        m_protocol->setAppsDrawerKeyboardGrab(true);
    }
    // Capture seam: report the Files tile's scene rect so the live check can
    // click a known app instead of guessing; never set in a normal session.
    if (m_appsDrawerFixture) {
        QTimer::singleShot(1200, this, [this]() {
            logAppsDrawerTile(QStringLiteral("org.dragonfruit.Files.desktop"));
        });
    }
}

void ShellController::hideAppsDrawer()
{
    const bool heldKeyboard = m_appsDrawerKeyboardFocused;
    m_appsDrawerKeyboardFocused = false;
    m_appsDrawerOpen = false;
    m_appsDrawerPending = false;
    m_appsDrawerButtons = Qt::NoButton;
    if (m_appsDrawerItem)
        m_appsDrawerItem->setProperty("active", false);
    if (m_protocol) {
        m_protocol->setAppsDrawerInputRegion(0, 0);
        m_protocol->setAppsDrawerKeyboardGrab(false);
        // Unmapping the layer does not itself move keyboard focus. Release
        // the chrome focus we took on open so the active window gets the
        // keyboard back instead of the keys landing on a hidden surface.
        if (heldKeyboard)
            m_protocol->releaseKeyboardFocus();
        m_protocol->hideAppsDrawer();
    }
}

void ShellController::onAppsDrawerDismissed()
{
    if (m_appsDrawerOpen)
        hideAppsDrawer();
}

void ShellController::refreshAppsDrawer(bool reloadCorpus)
{
    if (!m_appsDrawerItem)
        return;
    if (reloadCorpus && !m_appIndexSubscribed)
        m_index.loadFromAppIndex(m_appIndexClient);

    QList<DesktopEntry> entries = m_index.entries();
    if (m_appsDrawerFixture) {
        // Capture-only synthetic corpus: a broad mix of names and categories
        // so the live check exercises sorting, pills, and the icon fallback.
        struct Fixture {
            const char *id;
            const char *name;
            const char *categories;
        };
        static const Fixture fixtures[] = {
            {"org.example.Calendar.desktop", "Calendar", "Office;Utility"},
            {"org.example.Browser.desktop", "Browser", "Network"},
            {"org.example.Calculator.desktop", "Calculator", "Utility"},
            {"org.example.Editor.desktop", "Editor", "Development;Utility"},
            {"org.example.Music.desktop", "Music", "Audio;AudioVideo"},
            {"org.example.Photos.desktop", "Photos", "Graphics;Photography"},
            {"org.example.Terminal.desktop", "Terminal", "System;Utility"},
            {"org.example.Mail.desktop", "Mail", "Network;Email"},
            {"org.example.Game.desktop", "Arcade", "Game"},
        };
        for (const Fixture &fixture : fixtures) {
            DesktopEntry entry;
            entry.id = QString::fromLatin1(fixture.id);
            entry.name = QString::fromLatin1(fixture.name);
            entry.exec = QString::fromLatin1(fixture.id);
            entry.categories = QString::fromLatin1(fixture.categories)
                                   .split(QLatin1Char(';'), Qt::SkipEmptyParts);
            entry.valid = true;
            entries.append(entry);
        }
    }

    const QList<AppsDrawerRow> rows = buildAppsDrawerList(entries, QStringLiteral("all"));
    QVariantList list;
    list.reserve(rows.size());
    for (const AppsDrawerRow &row : rows) {
        // A QUrl, not a hand-built `file://` string: icon paths may contain
        // spaces or other characters that need percent-encoding.
        const QString iconUrl = row.iconPath.isEmpty()
                ? QString()
                : QUrl::fromLocalFile(row.iconPath).toString();
        list.append(QVariantMap{
            {QStringLiteral("desktopId"), row.desktopId},
            {QStringLiteral("name"), row.name},
            {QStringLiteral("iconPath"), row.iconPath},
            {QStringLiteral("iconUrl"), iconUrl},
            {QStringLiteral("categories"), row.categories},
        });
    }
    m_appsDrawerItem->setProperty("apps", list);
    m_appsDrawerItem->setProperty("available", m_appsDrawerFixture
                                                    || m_appIndexClient.available());
    m_appsDrawerItem->setProperty("active", m_appsDrawerOpen);
}

void ShellController::startAppsDrawerFixture()
{
    m_appsDrawerFixture = true;
}

void ShellController::logAppsDrawerTile(const QString &desktopId)
{
    if (!m_appsDrawerItem || !m_appsDrawerOpen)
        return;
    QVariant rect;
    const bool ok = QMetaObject::invokeMethod(m_appsDrawerItem, "tileRectFor",
                                              Q_RETURN_ARG(QVariant, rect),
                                              Q_ARG(QVariant, desktopId));
    const QVariantMap map = ok ? rect.toMap() : QVariantMap();
    if (!ok || map.isEmpty()) {
        qInfo() << "apps-drawer tile" << desktopId << "not found";
        return;
    }
    qInfo() << "apps-drawer tile" << desktopId
            << "x" << map.value(QStringLiteral("x")).toDouble()
            << "y" << map.value(QStringLiteral("y")).toDouble()
            << "w" << map.value(QStringLiteral("width")).toDouble()
            << "h" << map.value(QStringLiteral("height")).toDouble();
}

void ShellController::onAppsDrawerConfigured(int width, int height, quint32)
{
    if (width <= 0 || height <= 0)
        return;
    m_appsDrawerWidth = width;
    m_appsDrawerHeight = height;
    fprintf(stderr, "dragonfruit-shell: apps-drawer configured %dx%d\n", width, height);
    if (m_appsDrawerOpen)
        renderAppsDrawer();
}

void ShellController::onAppsDrawerPointerMoved(qreal x, qreal y)
{
    if (!m_appsDrawerWindow)
        return;
    sendChromePointer(m_appsDrawerWindow, QEvent::MouseMove, QPointF(x, y), Qt::NoButton,
                        m_appsDrawerButtons);
    scheduleAppsDrawerRender();
}

void ShellController::onAppsDrawerPointerButton(qreal x, qreal y, quint32 button, bool pressed)
{
    if (!m_appsDrawerWindow)
        return;
    Qt::MouseButton qtButton = Qt::NoButton;
    if (button == 0x110)
        qtButton = Qt::LeftButton;
    else if (button == 0x111)
        qtButton = Qt::RightButton;
    if (pressed)
        m_appsDrawerButtons |= qtButton;
    else
        m_appsDrawerButtons &= ~qtButton;
    sendChromePointer(m_appsDrawerWindow,
                        pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease,
                        QPointF(x, y), qtButton, m_appsDrawerButtons);
    scheduleAppsDrawerRender();
}

void ShellController::onAppsDrawerPointerLeft()
{
    if (!m_appsDrawerWindow)
        return;
    sendChromePointer(m_appsDrawerWindow, QEvent::MouseMove, QPointF(-1, -1), Qt::NoButton,
                        m_appsDrawerButtons);
    scheduleAppsDrawerRender();
}

void ShellController::onAppsDrawerKeyboardFocused(bool focused)
{
    m_appsDrawerKeyboardFocused = focused;
    if (!focused) {
        QTimer::singleShot(0, this, [this]() {
            if (m_appsDrawerOpen && !m_appsDrawerKeyboardFocused)
                hideAppsDrawer();
        });
        return;
    }
    if (m_appsDrawerItem && m_appsDrawerOpen && m_appsDrawerWindow)
        QMetaObject::invokeMethod(m_appsDrawerItem, "focusSearch");
}

void ShellController::onAppsDrawerKeyEvent(quint32 key, bool pressed)
{
    if (!m_appsDrawerWindow || !m_appsDrawerOpen)
        return;
    // Forward the full key (text included) to the offscreen scene; the search
    // field receives printable characters and the QML Keys handler owns
    // navigation/dismissal.
    sendChromeKeyEvent(m_appsDrawerWindow, key, pressed,
                       m_protocol ? m_protocol->keyboardModifiers() : 0);
    scheduleAppsDrawerRender();
}

void ShellController::onAppsDrawerActivated(const QString &desktopId, qreal x, qreal y, qreal w,
                                            qreal h)
{
    if (desktopId.isEmpty())
        return;
    // Record the acted-on tile so the launch path hands the compositor the real
    // icon rect exactly as the Dock does (T-14.7l). The drawer surface maps 1:1
    // to the output, so its scene coordinates are output coordinates.
    if (w > 0 && h > 0)
        m_dockTiles.record(desktopId, QRect(qRound(x), qRound(y), qRound(w), qRound(h)));
    hideAppsDrawer();
    // A launcher click never dies silently: `openApp` activates a running
    // window or launches the installed entry; a failure raises the shell's
    // existing launch-failure notice.
    openApp(desktopId);
}

void ShellController::scheduleAppsDrawerRender()
{
    if (m_appsDrawerRenderPending)
        return;
    m_appsDrawerRenderPending = true;
    QTimer::singleShot(0, this, [this]() {
        m_appsDrawerRenderPending = false;
        renderAppsDrawer();
    });
}

void ShellController::renderAppsDrawer()
{
    if (!m_appsDrawerOpen || !m_appsDrawerWindow || !m_appsDrawerItem)
        return;
    if (m_appsDrawerWidth <= 0 || m_appsDrawerHeight <= 0)
        return;
    prepareChromeScene(m_appsDrawerWindow, m_appsDrawerItem, QSize(m_appsDrawerWidth, m_appsDrawerHeight),
                       QSize(m_appsDrawerWidth, m_appsDrawerHeight), m_chromeScale);
    if (m_protocol) {
        // Declare the card's live rect before the buffer commit, so the
        // compositor's backdrop blur follows it (T-20.1). The card is centred
        // and changes size with the corpus, so it is re-sent every commit.
        QVariant rect;
        if (QMetaObject::invokeMethod(m_appsDrawerItem, "panelRect",
                                      Q_RETURN_ARG(QVariant, rect))) {
            const QVariantMap map = rect.toMap();
            m_protocol->setAppsDrawerPanelRect(
                qRound(map.value(QStringLiteral("x")).toDouble()),
                qRound(map.value(QStringLiteral("y")).toDouble()),
                qMax(0, qRound(map.value(QStringLiteral("width")).toDouble())),
                qMax(0, qRound(map.value(QStringLiteral("height")).toDouble())));
        }
    }
    const QSize logical(m_appsDrawerWidth, m_appsDrawerHeight);
    commitChromeScene(m_appsDrawerWindow,
                      {{QStringLiteral("apps-drawer"), logical, QRect(),
                        [this, logical](const QImage &image) {
                            return m_protocol->commitAppsDrawerImage(image, logical);
                        }}});
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
        // The QML confirmation already ran; start the destructive operation
        // asynchronously on the Trash bridge worker so the shell UI thread
        // never blocks (T-14.7r). A second request while emptying, or an
        // unreachable backend, is reported to the Dock rather than hanging.
        if (m_trash) {
            if (!m_trash->emptyAsync() && !m_trash->isEmptying()) {
                // The operation could not start (no monitor). emptyAsync
                // already emitted the failure result; nothing more to do.
                qWarning() << "shell: cannot empty the Trash:"
                           << m_trash->emptyError();
            }
        } else if (m_dockItem) {
            QMetaObject::invokeMethod(
                m_dockItem, "handleTrashEmptyResult",
                Q_ARG(QVariant, false), Q_ARG(QVariant, -1),
                Q_ARG(QVariant, QStringLiteral("no trash monitor")));
        }
    }
    scheduleDockRender();
}

void ShellController::onDockWindowActivated(const QString &windowId)
{
    m_protocol->selectToplevel(windowId);
    scheduleDockRender();
}

void ShellController::onDockWindowCloseRequested(const QString &windowId)
{
    // One window, not the whole app (T-14.7m). The chooser stays open; when
    // the compositor retires the window the next projection drops its row, and
    // the Dock dismisses the chooser if that was the app's last window.
    m_protocol->closeToplevel(windowId);
    scheduleDockRender();
}

void ShellController::onDockWindowMinimizeRequested(const QString &windowId, bool minimized)
{
    // The row sent the state it wants; the compositor flips the toplevel and
    // the projection flips the button label (T-14.7m). The chooser stays open.
    m_protocol->setToplevelMinimized(windowId, minimized);
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
    // The pinned set changed; drop the recorded tiles (T-14.7l).
    m_dockTiles.clear();
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

void ShellController::onTrashEmptyFinished(bool ok, int removed, const QString &error)
{
    // The worker reported on the UI thread; hand the one-shot result to the
    // Dock's progress/result popover (T-14.7r). The bridge's `changed` signal
    // separately updates the icon/badge from the real store.
    if (!m_dockItem)
        return;
    QMetaObject::invokeMethod(
        m_dockItem, "handleTrashEmptyResult",
        Q_ARG(QVariant, ok), Q_ARG(QVariant, removed), Q_ARG(QVariant, error));
    if (!ok)
        qWarning() << "shell: cannot empty the Trash:" << error;
    scheduleDockRender();
}

void ShellController::refreshFolderStackData()
{
    if (!m_folderStacks)
        return;
    // The Downloads default member is always first; the user's
    // `dock.pinnedFolders` follow. The whole set is listed through the one
    // files-core owner, never a second directory reader (ADR 0092).
    const QString downloads = QDir::cleanPath(downloadsDirectory());
    QStringList paths;
    paths.append(downloads);
    for (const QString &path : m_dockConfig.pinnedFolders) {
        const QString clean = QDir::cleanPath(path);
        if (!clean.isEmpty() && !paths.contains(clean))
            paths.append(clean);
    }
    m_folderStacks->setFolders(paths);
}

void ShellController::onDownloadsChanged()
{
    if (!m_dockItem || !m_folderStacks)
        return;
    const QVariantList stacks = m_folderStacks->stacks();
    const QString downloads = QDir::cleanPath(downloadsDirectory());

    // The Downloads stack keeps its dedicated QML properties (the default
    // member, ADR 0092); the remaining stacks are the user's folder pins.
    QVariantList pins;
    bool sawDownloads = false;
    for (const QVariant &value : stacks) {
        const QVariantMap map = value.toMap();
        const QString path = map.value(QStringLiteral("path")).toString();
        if (!sawDownloads && path == downloads) {
            sawDownloads = true;
            m_dockItem->setProperty("downloadsItems", map.value(QStringLiteral("items")));
            m_dockItem->setProperty("downloadsCount", map.value(QStringLiteral("count")));
            m_dockItem->setProperty("downloadsBadge", map.value(QStringLiteral("badge")));
            m_dockItem->setProperty("downloadsName", map.value(QStringLiteral("name")));
            m_dockItem->setProperty("downloadsPath", downloads);
            continue;
        }
        QVariantMap pin = map;
        pin.insert(QStringLiteral("id"), QStringLiteral("folder:") + path);
        pin.insert(QStringLiteral("kind"), QStringLiteral("stack"));
        pins.append(pin);
    }
    if (!sawDownloads) {
        // The Downloads folder is unavailable; degrade with an empty listing
        // rather than a stale one.
        m_dockItem->setProperty("downloadsItems", QVariantList());
        m_dockItem->setProperty("downloadsCount", 0);
        m_dockItem->setProperty("downloadsBadge", 0);
        m_dockItem->setProperty("downloadsName", folderDisplayName(downloads));
        m_dockItem->setProperty("downloadsPath", downloads);
    }
    m_dockItem->setProperty("folderPins", pins);
    scheduleDockRender();
}

void ShellController::onDockDownloadsViewed()
{
    if (m_folderStacks)
        m_folderStacks->markSeen(downloadsDirectory());
}

void ShellController::onDockFolderOpenRequested(const QString &path)
{
    if (path.isEmpty())
        return;
    // A pinned folder opens in Files through the one reveal path (T-14.7h).
    launchFiles(path);
}

void ShellController::onDockFolderViewed(const QString &path)
{
    if (m_folderStacks && !path.isEmpty())
        m_folderStacks->markSeen(path);
}

void ShellController::onDockFolderPinRemoved(const QString &path)
{
    if (path.isEmpty())
        return;
    const QString clean = QDir::cleanPath(path);
    QStringList next;
    for (const QString &pinned : m_dockConfig.pinnedFolders) {
        if (QDir::cleanPath(pinned) != clean)
            next.append(pinned);
    }
    if (next == m_dockConfig.pinnedFolders)
        return;
    // Settingsd remains the only writer; the Dock never persists locally.
    writeDockSetting(QStringLiteral("dock.pinnedFolders"), next);
    refreshFolderStackData();
    qInfo() << "shell: Dock folder pin removed" << clean;
}

void ShellController::onDockDownloadActivated(const QString &path)
{
    if (path.isEmpty())
        return;
    // Downloads-stack rows open through Files (T-10.6c): Files opens the
    // containing folder and reveals the item, so the Dock and the file
    // manager agree on where the download lives.
    const QFileInfo info(path);
    if (!info.exists()) {
        qWarning() << "shell: downloaded file is gone:" << path;
        return;
    }
    launchFiles(info.absoluteFilePath());
}

void ShellController::onDockDownloadsFolderRequested()
{
    const QString folder = downloadsDirectory();
    QDir().mkpath(folder);
    // A stack folder opens in Files (T-14.7h, T-10.6c): the single reveal path
    // the Dock and the file manager share, so the folder appears where the
    // file manager expects it.
    launchFiles(folder);
}


void ShellController::scheduleDockRender()
{
    markChromeDirty(m_producers.value(m_dockWindow));
}

void ShellController::onDockPointerTrackingChanged()
{
    if (!m_dockItem->property("pointerTrackingActive").toBool()) {
        m_dockTrackingTimer->stop();
        m_dockTrackingTime.invalidate();
    } else if (!m_dockTrackingTimer->isActive()) {
        m_dockTrackingTime.start();
        m_dockTrackingTimer->start();
        scheduleDockRender();
    }
}

void ShellController::prepareChromeScene(QQuickWindow *window, QQuickItem *item,
                                         const QSize &itemLogical,
                                         const QSize &windowLogical, qreal scale)
{
    if (!window || !item || itemLogical.isEmpty() || windowLogical.isEmpty())
        return;
    // Setting a QQuickItem property to its current value can still mark a text
    // item for relayout, which feeds the scene-changed signal back into the
    // pacer. Only touch geometry/scale when it actually changes.
    if (!qFuzzyCompare(item->width(), qreal(itemLogical.width())))
        item->setWidth(itemLogical.width());
    if (!qFuzzyCompare(item->height(), qreal(itemLogical.height())))
        item->setHeight(itemLogical.height());
    // Scale the root item, not the layout: QML anchors, popup rects, and
    // pointer handlers keep their logical coordinates while the scene graph
    // rasterizes at the physical resolution. The GPU producer's render target
    // already carries the scale as its device pixel ratio, so the item stays
    // at 1.0 there; scaling it twice would double the physical geometry.
    item->setTransformOrigin(QQuickItem::TopLeft);
    ChromeFrameProducer *producer = m_producers.value(window);
    auto *gpu = qobject_cast<GpuChromeFrameProducer *>(producer);
    const qreal itemScale = gpu ? 1.0 : scale;
    if (!qFuzzyCompare(item->scale(), itemScale))
        item->setScale(itemScale);
    const QSize buffer = ChromeScale::bufferSize(windowLogical, scale);
    if (producer) {
        // T-21.1d: the GPU producer renders a physical FBO but keeps the QML
        // scene in logical units; it needs the same scale the software path
        // bakes into the QImage DPR.
        if (gpu)
            gpu->setDevicePixelRatio(scale);
        // Render-control windows are never shown; the producer owns the
        // offscreen target. Fallback windows still need their render loop.
        producer->resize(buffer);
    } else {
        if (window->width() != buffer.width() || window->height() != buffer.height())
            window->resize(buffer);
        if (!window->isVisible())
            window->show();
    }
}

QRect ShellController::chromeBufferRect(const QRect &logical) const
{
    return ChromeScale::bufferRect(logical, m_chromeScale);
}

ChromeFrameProducer *ShellController::createChromeProducer()
{
    // T-21.1d: one GPU-first attempt at the GPU producer. The scene-graph API
    // is process-global, so this must happen before any other QQuickWindow is
    // constructed. If the context/export prerequisites are missing the whole
    // shell stays on the T185 software producer; a failed attempt that already
    // created a window falls back to the producer's grabWindow rail. The
    // producer rejects software GL renderers before creating that window, so
    // a no-GPU host gets the software scene graph rather than llvmpipe.
    if (m_gpuChromeRequested && m_gpuChromeState != GpuChromeState::Failed) {
        auto *gpu = new GpuChromeFrameProducer(this);
        gpu->setDmabufFormats(m_protocol ? m_protocol->gpuCapabilities().formats
                                         : QVector<GpuChromeCapabilities::FormatModifier>());
        QString reason;
        if (gpu->initialize(&reason)) {
            m_gpuChromeState = GpuChromeState::Active;
            m_gpuChromeReason.clear();
            ChromeStats::instance().setGpuPath(QStringLiteral("shm"));
            m_producers.insert(gpu->window(), gpu);
            if (!m_chromeFallbackTimer) {
                m_chromeFallbackTimer = new QTimer(this);
                m_chromeFallbackTimer->setInterval(100);
                connect(m_chromeFallbackTimer, &QTimer::timeout, this,
                        &ShellController::onChromeFallbackTick);
            }
            return gpu;
        }
        m_gpuChromeState = GpuChromeState::Failed;
        m_gpuChromeReason = reason;
        ChromeStats::instance().setGpuPath(QStringLiteral("shm"));
        ChromeStats::instance().setGpuFallbackReason(reason);
        // No window was created before the failure: restore the software
        // scene graph so the T185 producer can still render. If a window did
        // exist, the producer's non-software grabWindow fallback takes over.
        if (!gpu->hasWindow())
            QQuickWindow::setGraphicsApi(QSGRendererInterface::Software);
        delete gpu;
    }

    auto *producer = new ChromeFrameProducer(this);
    m_producers.insert(producer->window(), producer);
    if (!m_chromeFallbackTimer) {
        // The safety tick is not an animation source: it only runs while a
        // dirty producer is waiting on a frame callback the compositor never
        // answered, and every render it drives is counted.
        m_chromeFallbackTimer = new QTimer(this);
        m_chromeFallbackTimer->setInterval(100);
        connect(m_chromeFallbackTimer, &QTimer::timeout, this,
                &ShellController::onChromeFallbackTick);
    }
    return producer;
}

void ShellController::bindChromePacing(ChromeFrameProducer *producer,
                                       const QString &primaryLabel,
                                       std::function<void()> render)
{
    if (!producer)
        return;
    ChromePace pace;
    pace.producer = producer;
    pace.primaryLabel = primaryLabel;
    pace.render = std::move(render);
    m_chromePaces.insert(producer, pace);
    producer->window()->setObjectName(primaryLabel);
    if (primaryLabel == QLatin1String("dock"))
        qInfo() << "shell: Dock scene-graph commit path active (FR-14)";
    connect(producer, &ChromeFrameProducer::sceneChanged, this,
            [this, producer]() { markChromeDirty(producer); });
}

void ShellController::markChromeDirty(ChromeFrameProducer *producer)
{
    auto it = m_chromePaces.find(producer);
    if (it == m_chromePaces.end())
        return;
    ChromePace &pace = it.value();
    if (pace.rendering) {
        // A scene change raised synchronously inside polish/sync/render is a
        // re-entrant update, not a new presentation request. Ignoring it is
        // what keeps the render loop from feeding itself; the QML animation
        // driver raises the next real change on its next tick.
        return;
    }
    pace.dirty = true;
    if (pace.scheduled)
        return;
    if (!m_protocol->hasPendingFrame(pace.primaryLabel))
        scheduleChromeRender(producer);
    else if (m_chromeFallbackTimer && !m_chromeFallbackTimer->isActive())
        m_chromeFallbackTimer->start();
}

void ShellController::scheduleChromeRender(ChromeFrameProducer *producer)
{
    auto it = m_chromePaces.find(producer);
    if (it == m_chromePaces.end() || it.value().scheduled)
        return;
    it.value().scheduled = true;
    QTimer::singleShot(0, this, [this, producer]() {
        auto pace = m_chromePaces.find(producer);
        if (pace == m_chromePaces.end())
            return;
        pace.value().scheduled = false;
        if (!pace.value().dirty)
            return;
        if (m_protocol->hasPendingFrame(pace.value().primaryLabel)) {
            if (m_chromeFallbackTimer && !m_chromeFallbackTimer->isActive())
                m_chromeFallbackTimer->start();
            return;
        }
        renderChromeProducer(producer, false);
    });
}

void ShellController::renderChromeProducer(ChromeFrameProducer *producer, bool fallback)
{
    auto it = m_chromePaces.find(producer);
    if (it == m_chromePaces.end())
        return;
    ChromePace &pace = it.value();
    if (!pace.dirty)
        return;
    pace.dirty = false;
    pace.rendering = true;
    if (pace.render)
        pace.render();
    pace.rendering = false;
    if (fallback)
        ChromeStats::instance().recordFallbackRender();
    if (pace.dirty) {
        if (!m_protocol->hasPendingFrame(pace.primaryLabel))
            scheduleChromeRender(producer);
        else if (m_chromeFallbackTimer && !m_chromeFallbackTimer->isActive())
            m_chromeFallbackTimer->start();
    }
}

void ShellController::onChromeFramePresented(const QString &surface, uint32_t time)
{
    Q_UNUSED(time);
    for (auto it = m_chromePaces.begin(); it != m_chromePaces.end(); ++it) {
        ChromePace &pace = it.value();
        if (pace.primaryLabel != surface || !pace.dirty || pace.scheduled)
            continue;
        // The compositor presented the frame we asked to be paced. Render the
        // next QML state only now, not on a free-running timer.
        scheduleChromeRender(it.key());
    }
}

void ShellController::onChromeBufferReleased(const QString &surface)
{
    for (auto it = m_chromePaces.begin(); it != m_chromePaces.end(); ++it) {
        ChromePace &pace = it.value();
        if (pace.primaryLabel != surface || !pace.dirty || pace.scheduled
            || m_protocol->hasPendingFrame(surface))
            continue;
        scheduleChromeRender(it.key());
    }
}

void ShellController::onChromeDmabufReleased(const QString &surface, int slot,
                                             quint64 serial)
{
    const QString paceLabel = chromePaceLabel(surface);
    for (auto it = m_chromePaces.begin(); it != m_chromePaces.end(); ++it) {
        ChromePace &pace = it.value();
        if (pace.primaryLabel != paceLabel)
            continue;
        // A shared scene has one wl_buffer per surface. Only the last release
        // returns the producer's ring slot; an earlier popover release must
        // not let the next frame overwrite the still-visible bar buffer.
        if (!pace.dmabufUsers.release(serial))
            return;
        if (auto *gpu = qobject_cast<GpuChromeFrameProducer *>(pace.producer))
            gpu->releaseDmabufFrame(slot, serial);
        if (pace.dirty && !pace.scheduled
            && !m_protocol->hasPendingFrame(pace.primaryLabel))
            scheduleChromeRender(pace.producer);
        return;
    }
}

void ShellController::onChromeCommitDeferred(const QString &surface)
{
    for (auto it = m_chromePaces.begin(); it != m_chromePaces.end(); ++it) {
        ChromePace &pace = it.value();
        if (pace.primaryLabel != surface)
            continue;
        pace.dirty = true;
        if (!pace.scheduled && !m_protocol->hasPendingFrame(pace.primaryLabel))
            scheduleChromeRender(it.key());
        else if (m_chromeFallbackTimer && !m_chromeFallbackTimer->isActive())
            m_chromeFallbackTimer->start();
    }
}

void ShellController::onChromeFallbackTick()
{
    const QList<ChromeFrameProducer *> producers = m_chromePaces.keys();
    for (ChromeFrameProducer *producer : producers) {
        auto it = m_chromePaces.find(producer);
        if (it == m_chromePaces.end() || !it.value().dirty)
            continue;
        if (!m_protocol->hasPendingFrame(it.value().primaryLabel))
            scheduleChromeRender(producer);
        else if (m_protocol->hasPendingFrame(it.value().primaryLabel,
                                            m_chromeFallbackTimer->interval()))
            renderChromeProducer(producer, true);
    }
    bool waiting = false;
    for (auto it = m_chromePaces.constBegin(); it != m_chromePaces.constEnd(); ++it) {
        if (it.value().dirty
                && m_protocol->hasPendingFrame(it.value().primaryLabel)) {
            waiting = true;
            break;
        }
    }
    if (!waiting && m_chromeFallbackTimer)
        m_chromeFallbackTimer->stop();
}

QString ShellController::chromePaceLabel(const QString &surface) const
{
    // A popover is a sub-rect of its owner's scene, not a second render. Its
    // wl_buffer release must return the owner producer's ring slot.
    if (surface == QLatin1String("menubar-popup"))
        return QStringLiteral("menubar");
    if (surface == QLatin1String("dock-popup"))
        return QStringLiteral("dock");
    return surface;
}

void ShellController::noteSurfacePath(const QString &label, const QString &path,
                                      const QString &reason)
{
    ChromeStats::instance().setSurfacePath(label, path, reason);
}

void ShellController::deferChromeProducer(ChromeFrameProducer *producer)
{
    auto it = m_chromePaces.find(producer);
    if (it == m_chromePaces.end())
        return;
    it.value().dirty = true;
    // A transient export failure means the GPU is still finishing the frame.
    // Retry on the bounded safety tick instead of spinning a new render
    // immediately; a wl_buffer release still wakes the producer directly.
    if (m_chromeFallbackTimer && !m_chromeFallbackTimer->isActive())
        m_chromeFallbackTimer->start();
    else if (!m_chromeFallbackTimer && !it.value().scheduled
             && !m_protocol->hasPendingFrame(it.value().primaryLabel))
        scheduleChromeRender(producer);
}

ShellProtocol::DmabufCommitResult ShellController::commitChromeDmabufTarget(
    GpuChromeFrameProducer *gpu, const QString &label,
    const ChromeDmabufFrame &frame, const QSize &logicalSize,
    const QRect &sourceRect, quintptr lockSurfaceId, QString *reason)
{
    const ShellProtocol::DmabufCommitResult result =
        lockSurfaceId != 0
            ? m_protocol->commitLockDmabuf(lockSurfaceId, frame, logicalSize, reason)
            : m_protocol->commitDmabufFor(label, frame, logicalSize, sourceRect,
                                          reason);
    if (result == ShellProtocol::DmabufCommitResult::Committed) {
        noteSurfacePath(label, QStringLiteral("dmabuf"));
        ChromeStats::instance().setGpuPath(QStringLiteral("dmabuf"));
        ChromeStats::instance().setTransportPath(QStringLiteral("render-control+dmabuf"));
        auto it = m_chromePaces.find(gpu);
        if (it != m_chromePaces.end())
            it.value().dmabufUsers.acquire(frame.serial);
        return result;
    }

    const QString why = reason && !reason->isEmpty() ? *reason
                                                     : QStringLiteral("import-failed");
    if (result == ShellProtocol::DmabufCommitResult::Deferred) {
        ChromeStats::instance().recordDmabufDeferred(label);
        noteSurfacePath(label, QStringLiteral("shm"), why);
        return result;
    }

    ChromeStats::instance().recordDmabufFallback(label, why);
    noteSurfacePath(label, QStringLiteral("shm"), why);
    ChromeStats::instance().setGpuPath(QStringLiteral("shm"));
    // A surface that cannot express this commit (no viewporter, no format,
    // import failure) stays on shm for the session; the rest of the chrome
    // keeps using dmabuf. A missing linux-dmabuf binding is producer-wide.
    if (why == QLatin1String("no-dmabuf")) {
        m_chromeDmabufEnabled = false;
        ChromeStats::instance().setTransportPath(QStringLiteral("render-control+shm"));
    } else {
        m_chromeDmabufDisabled.insert(label);
    }
    return result;
}

bool ShellController::commitChromeScene(QQuickWindow *window,
                                        const QList<ChromeSceneTarget> &targets)
{
    if (!window || !m_protocol || targets.isEmpty())
        return false;

    ChromeFrameProducer *producer = m_producers.value(window);
    auto *gpu = qobject_cast<GpuChromeFrameProducer *>(producer);

    QList<int> deferredTargets;
    bool anyCommitted = false;
    ChromeDmabufFrame frame;
    QString exportReason;

    // A shared scene has one path: if any of its surfaces cannot use dmabuf,
    // the whole scene renders through T185 shm. That is the only way to avoid
    // reading back a frame that the compositor is already sampling through a
    // committed dmabuf (which can block on the GPU fence).
    bool anyTargetDisabled = false;
    for (const ChromeSceneTarget &target : targets) {
        if (m_chromeDmabufDisabled.contains(target.label)
            || (!m_chromeDmabufAllowed.isEmpty()
                && !m_chromeDmabufAllowed.contains(target.label))) {
            anyTargetDisabled = true;
            break;
        }
    }
    const bool useGpu = gpu && gpu->gpuActive() && m_chromeDmabufEnabled
        && !anyTargetDisabled;

    if (useGpu) {
        // One scene render for every target. The first target owns the stats
        // label; a shared popover does not add a second render.
        ChromeStats::instance().recordRender(targets.first().label);
        if (gpu->exportFrame(&frame, &exportReason)) {
            int successCount = 0;
            bool anyHardFailure = false;
            for (int index = 0; index < targets.size(); ++index) {
                const ChromeSceneTarget &target = targets.at(index);
                if (!m_chromeDmabufAllowed.isEmpty()
                    && !m_chromeDmabufAllowed.contains(target.label)) {
                    noteSurfacePath(target.label, QStringLiteral("shm"),
                                    QStringLiteral("not-selected"));
                    continue;
                }
                QString reason;
                const ShellProtocol::DmabufCommitResult result =
                    commitChromeDmabufTarget(gpu, target.label, frame,
                                             target.logicalSize, target.sourceRect,
                                             target.lockSurfaceId, &reason);
                if (result == ShellProtocol::DmabufCommitResult::Committed) {
                    anyCommitted = true;
                    ++successCount;
                } else if (result == ShellProtocol::DmabufCommitResult::Deferred) {
                    deferredTargets.append(index);
                } else {
                    anyHardFailure = true;
                }
            }
            if (successCount == 0) {
                // Nothing holds the exported frame. The FBO is safe to read
                // once for the shm fallback of this frame.
                gpu->abandonDmabufFrame(frame.slot);
                const QImage image = gpu->readbackCurrentFrame();
                if (!image.isNull()) {
                    for (const ChromeSceneTarget &target : targets) {
                        if (target.commitShm && target.commitShm(image)) {
                            anyCommitted = true;
                            noteSurfacePath(target.label, QStringLiteral("shm"));
                        }
                    }
                } else {
                    deferChromeProducer(producer);
                }
            } else if (anyHardFailure || !deferredTargets.isEmpty()) {
                // A dmabuf commit already handed this scene to the compositor.
                // Do not read it back; a hard-failed surface is now disabled,
                // so the next frame renders the whole scene through shm. A
                // deferred surface retries on the release signal.
                deferChromeProducer(producer);
            }
        } else {
            // The producer could not export. A transient ring/fence failure
            // keeps the rail enabled and retries; a hard export/format failure
            // abandons it for the session so the next frame takes the shm
            // path. Never read back a slot the compositor may still hold.
            const bool transient = exportReason == QLatin1String("ring-exhausted")
                || exportReason == QLatin1String("sync-timeout");
            if (!transient) {
                m_chromeDmabufEnabled = false;
                ChromeStats::instance().setGpuPath(QStringLiteral("shm"));
                ChromeStats::instance().setTransportPath(
                    QStringLiteral("render-control+shm"));
            }
            for (const ChromeSceneTarget &target : targets) {
                ChromeStats::instance().recordDmabufFallback(target.label,
                                                             exportReason);
                noteSurfacePath(target.label, QStringLiteral("shm"), exportReason);
            }
            deferChromeProducer(producer);
        }
    } else {
        QString reason;
        if (!gpu || !gpu->gpuActive())
            reason = m_gpuChromeRequested && !m_gpuChromeReason.isEmpty()
                ? m_gpuChromeReason
                : QStringLiteral("flag-off");
        else
            reason = QStringLiteral("disabled");
        for (const ChromeSceneTarget &target : targets) {
            const QString targetReason =
                (!m_chromeDmabufAllowed.isEmpty()
                 && !m_chromeDmabufAllowed.contains(target.label))
                    ? QStringLiteral("not-selected")
                    : reason;
            noteSurfacePath(target.label, QStringLiteral("shm"), targetReason);
        }

        const QImage image = renderChromeFrame(window);
        if (image.isNull()) {
            deferChromeProducer(producer);
        } else {
            for (const ChromeSceneTarget &target : targets) {
                if (target.commitShm && target.commitShm(image)) {
                    anyCommitted = true;
                    noteSurfacePath(target.label, QStringLiteral("shm"));
                }
            }
        }
    }

    return anyCommitted && deferredTargets.isEmpty();
}

QImage ShellController::renderChromeFrame(QQuickWindow *window)
{
    ChromeFrameProducer *producer = m_producers.value(window);
    if (producer) {
        ChromeStats::instance().recordRender(window ? window->objectName() : QString());
        return producer->renderFrame();
    }
    return window ? window->grabWindow() : QImage();
}

void ShellController::sendChromePointer(QQuickWindow *window, QEvent::Type type,
                                        const QPointF &logical, Qt::MouseButton button,
                                        Qt::MouseButtons buttons,
                                        Qt::KeyboardModifiers modifiers)
{
    // The software producer's offscreen window is the physical buffer, so its
    // coordinate space is physical; the GPU producer keeps the QQuickWindow
    // logical (the FBO is physical). Negative sentinels (pointer left) stay
    // off-screen.
    QPointF pos = logical;
    if (logical.x() >= 0.0 && logical.y() >= 0.0) {
        const bool logicalWindow =
            qobject_cast<GpuChromeFrameProducer *>(m_producers.value(window)) != nullptr;
        if (!logicalWindow)
            pos = ChromeScale::logicalToBuffer(logical, m_chromeScale);
    }
    ChromePointer::send(window, type, pos, button, buttons, modifiers);
}

void ShellController::onChromeScaleChanged(qreal scale)
{
    const qreal safe = ChromeScale::sanitize(scale);
    if (qFuzzyCompare(m_chromeScale, safe))
        return;
    m_chromeScale = safe;
    updateNotificationCenterGeometry();
    updateControlCenterGeometry();
    rerenderChrome();
}

void ShellController::rerenderChrome()
{
    if (m_item && m_width > 0)
        render();
    if (m_dockItem && m_dockWidth > 0 && m_dockHeight > 0)
        scheduleDockRender();
    if (m_overviewActive)
        renderOverview();
    if (m_switcherActive)
        renderSwitcher();
    if (m_appsDrawerOpen && m_appsDrawerPending)
        renderAppsDrawer();
    if (m_bannerPending)
        renderBanner();
    if (m_notificationCenterPending)
        renderNotificationCenter();
    if (m_controlCenterPending)
        renderControlCenter();
    if (m_osdActive)
        renderOsd();
    if (m_chooserActive && m_chooserPending)
        renderChooser();
    if (m_screenshotActive && m_screenshotPending)
        renderScreenshot();
    if (m_screencastActive && m_screencastPending)
        renderScreenCast();
    if (m_polkitActive && m_polkitPending)
        renderPolkit();
    if (m_lockActive)
        repaintLockSurfaces();
}

void ShellController::renderDock()
{
    if (!m_dockItem || !m_dockWindow || m_dockWidth <= 0 || m_dockHeight <= 0)
        return;
    if (m_dockTrackingTime.isValid()) {
        const qreal elapsedMs = m_dockTrackingTime.nsecsElapsed() / 1'000'000.0;
        m_dockTrackingTime.start();
        QMetaObject::invokeMethod(m_dockItem, "advancePointerTracking", Q_ARG(qreal, elapsedMs));
    }
    // Tell the Dock where its surface-local (0, 0) sits in the compositor's
    // output coordinates, so the tile rect it reports for the launch-origin
    // hand-off is already in that space (T-14.7l).
    const QPoint origin = dockSurfaceOutputOrigin();
    if (qRound(m_dockItem->property("outputOriginX").toReal()) != origin.x())
        m_dockItem->setProperty("outputOriginX", origin.x());
    if (qRound(m_dockItem->property("outputOriginY").toReal()) != origin.y())
        m_dockItem->setProperty("outputOriginY", origin.y());

    // The Dock popover (context menu / window chooser) in Dock-scene
    // coordinates; empty when nothing is open. `popoverRect` keys on the
    // popover's `open || visible` so the open/close fade is still captured.
    const auto dockRectProperty = [this](const char *name) {
        const QVariantMap rect = m_dockItem->property(name).toMap();
        const qreal x = rect.value(QStringLiteral("x")).toReal();
        const qreal y = rect.value(QStringLiteral("y")).toReal();
        const int left = qFloor(x);
        const int top = qFloor(y);
        return QRect(left, top,
                     qCeil(x + rect.value(QStringLiteral("w")).toReal()) - left,
                     qCeil(y + rect.value(QStringLiteral("h")).toReal()) - top);
    };
    const QRect popover = dockRectProperty("popoverRect");
    const int px = popover.x();
    const int py = popover.y();
    const int pw = popover.width();
    const int ph = popover.height();
    const bool hasPopover = !popover.isEmpty();

    // Use a strip for ordinary Dock frames. Give the shared scene its full
    // popover budget for the lifetime of an overlay, including its fade out.
    const int headroom = hasPopover ? kDockPopoverHeadroom : 0;
    const int leftGutter = hasPopover ? kDockPopoverGutter : 0;
    const bool originChanged = leftGutter != m_dockItemOffsetX || headroom != m_dockItemOffsetY;
    m_dockItemOffsetX = leftGutter;
    m_dockItemOffsetY = headroom;
    const int windowWidth = m_dockWidth + 2 * leftGutter;
    const int windowHeight = m_dockHeight + headroom;
    prepareChromeScene(m_dockWindow, m_dockItem, QSize(m_dockWidth, m_dockHeight),
                       QSize(windowWidth, windowHeight), m_chromeScale);
    auto *gpuProducer =
        qobject_cast<GpuChromeFrameProducer *>(m_producers.value(m_dockWindow));
    // The software producer's window is the physical buffer, so the scene
    // offset is physical too. The GPU producer keeps the QQuickWindow in
    // logical units (the FBO is physical), so the same offset stays logical.
    const QPointF itemOrigin =
        gpuProducer ? QPointF(leftGutter, headroom)
                    : ChromeScale::logicalToBuffer(QPointF(leftGutter, headroom),
                                                   m_chromeScale);
    m_dockItem->setX(itemOrigin.x());
    m_dockItem->setY(itemOrigin.y());
    // Qt remembers the last injected pointer in window coordinates. When
    // the overlay changes the scene origin, rebase that position before
    // Qt reevaluates hover during polish. Otherwise opening a label makes
    // the pointer appear to leave its tile and closes the label again.
    if (originChanged && m_dockPointerPosition && m_dockButtons == Qt::NoButton) {
        const QPointF pointer = *m_dockPointerPosition + QPointF(leftGutter, headroom);
        sendChromePointer(m_dockWindow, QEvent::MouseMove, pointer,
                          Qt::NoButton, Qt::NoButton);
    }

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
    // The live plate the Dock paints, in surface-local coordinates: the
    // compositor's frosted backdrop follows it as the plate grows around the
    // magnified row (T-14.7b). The Dock reports `panelRect`, the integer edge
    // it draws, so the declared panel and the QML fill are the same rounded
    // rect (T-14.7z). Submit the input and blur geometry with this frame.
    m_protocol->setDockFrameGeometry(dockRectProperty("panelRect"), inputRects);
    const QRect dockRect(leftGutter, headroom, m_dockWidth, m_dockHeight);
    const QRect dockSource = chromeBufferRect(dockRect);
    QList<ChromeSceneTarget> targets;
    targets.append({QStringLiteral("dock"), QSize(m_dockWidth, m_dockHeight), dockSource,
                    [this, dockSource](const QImage &image) {
                        return m_protocol->commitDockImage(
                            image, QSize(m_dockWidth, m_dockHeight), dockSource);
                    }});

    // Place the popover on the Dock's overlay surface. The popover is anchored
    // to the Dock's edge, so the margins are measured from that edge: scene
    // y=0 is the Dock surface top and scene x=0 its left. A bottom popover
    // sits above the bar (bottom margin); a vertical Dock's popover sits beside
    // the bar (left/right margin) at the entry's scene y (T-10 section 5).
    m_dockPopoverX = px;
    m_dockPopoverY = py;
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
        // Each menu card owns a material region in overlay-local coordinates.
        // The transparent gap and tooltip remain outside those regions.
        const bool popoverInputActive =
            m_dockItem->property("popoverInputActive").toBool();
        m_protocol->setDockPopupPanelRegions(
            m_dockItem->property("activePopoverSurfaceRegions").toList());
        m_protocol->setDockPopupInputActive(popoverInputActive);
        m_protocol->setDockPopupGeometry(top, right, bottom, left, pw, ph, QRect());
        const QRect popupRect(leftGutter + px, headroom + py, pw, ph);
        const QRect popupSource = chromeBufferRect(popupRect);
        targets.append({QStringLiteral("dock-popup"), QSize(pw, ph), popupSource,
                        [this, pw, ph, popupSource](const QImage &image) {
                            return m_protocol->commitDockPopupImage(
                                image, QSize(pw, ph), popupSource);
                        }});
    } else {
        m_protocol->clearDockPopupPanelRegions();
        m_protocol->setDockPopupInputActive(false);
        m_protocol->hideDockPopup();
    }

    if (!commitChromeScene(m_dockWindow, targets))
        qWarning() << "shell: failed to commit the Dock:" << m_protocol->lastError();
}

void ShellController::render()
{
    if (!m_item || m_width <= 0 || m_barHeight <= 0)
        return;
    // Startup readiness: do not map the bar until the initial status reads
    // have landed (or their bounded timeout fired). Committing an empty row
    // first is exactly the half-populated launch frame this gate prevents.
    if (m_initialStatusPending != 0)
        return;
    // The bar item itself stays bar-height; an open dropdown overflows it and
    // is committed to the separate overlay surface.
    const bool popupShown = m_popupWidth > 0 && m_popupHeight > 0;
    // T200: the scene is tall enough for the veil (and any dropdown), but the
    // menubar surface still commits only the bar strip. The compositor draws
    // the veil from its own material region below the strip.
    const int windowHeight = popupShown
        ? qMax(m_menuBarSurfaceHeight, m_popupY + m_popupHeight)
        : m_menuBarSurfaceHeight;
    prepareChromeScene(m_window, m_item, QSize(m_width, m_barHeight),
                       QSize(m_width, windowHeight), m_chromeScale);
    // Declare the material panel with this frame's content commit. The QML
    // resolution decides whether this is the 95 pt veil, the legacy 28 pt
    // sampled panel, or an explicit empty set for the fallback.
    const bool panelActive =
        m_item->property("compositorPanelActive").toBool();
    const int panelHeight = qMax(
        m_barHeight, m_item->property("compositorPanelHeight").toInt());
    const int panelRadius =
        m_item->property("compositorPanelRadius").toInt();
    m_protocol->setMenuBarFrameGeometry(panelActive, m_width, panelHeight,
                                        panelRadius,
                                        m_item->property("veilScheme").toString()
                                            == QLatin1String("light"));
    const QRect barRect(0, 0, m_width, m_barHeight);
    const QRect barSource = chromeBufferRect(barRect);
    QList<ChromeSceneTarget> targets;
    targets.append({QStringLiteral("menubar"), QSize(m_width, m_barHeight), barSource,
                    [this, barSource](const QImage &image) {
                        return m_protocol->commitImage(image,
                                                       QSize(m_width, m_barHeight),
                                                       barSource);
                    }});
    if (popupShown) {
        const QRect popupRect(m_popupX, m_popupY, m_popupWidth, m_popupHeight);
        const QRect popupSource = chromeBufferRect(popupRect);
        targets.append({QStringLiteral("menubar-popup"),
                        QSize(m_popupWidth, m_popupHeight), popupSource,
                        [this, popupSource](const QImage &image) {
                            return m_protocol->commitPopupImage(
                                image, QSize(m_popupWidth, m_popupHeight), popupSource);
                        }});
    }
    commitChromeScene(m_window, targets);
}
