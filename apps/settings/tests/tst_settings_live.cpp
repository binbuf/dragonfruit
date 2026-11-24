// SPDX-License-Identifier: MIT
// Settings live-apply plumbing tests (T-09.1b).
//
// The acceptance path, headless: a representative design-system control is
// bound to a settingsd key, the user changes it, and the consumer (the same
// control, and the fake/live `org.dragonfruit.Settings1` owner) reflects the
// new value without a restart. Three layers are exercised:
//
//   * `DF_SETTINGS_FIXTURE` — the deterministic in-process mock, no bus.
//   * a fake `org.dragonfruit.Settings1` object on the private session bus
//     (`dbus-run-session`, provided by ctest) — the live D-Bus path.
//   * the **real `dragonfruit-settingsd` binary**, when CMake found it, over
//     the same private bus with a scratch config home — the true round trip.
//
// The QML `Settings` singleton (apps/settings/SettingsBridge) is the plumbing
// under test; the control is a stock design-system `Toggle`.
#include "settingsclient.h"
#include "MenuPublisher.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusVariant>
#include <QProcess>
#include <QProcessEnvironment>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>
#include <QVariantList>
#include <QVariantMap>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.Settings1");
const QString kPath = QStringLiteral("/org/dragonfruit/Settings1");
const QString kInterface = QStringLiteral("org.dragonfruit.Settings1");
const QString kKey = QStringLiteral("accessibility.reduceMotion");

// The wallpaper content provider (T-18.1b), a minimal stand-in with the same
// property surface the real `dragonfruit-wallpaperd` exports.
const QString kWallpaperService = QStringLiteral("org.dragonfruit.Wallpaper1");
const QString kWallpaperPath = QStringLiteral("/org/dragonfruit/Wallpaper1");
const QString kWallpaperInterface = QStringLiteral("org.dragonfruit.Wallpaper1");

class FakeWallpaperService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Wallpaper1")
    Q_PROPERTY(QString Status READ status CONSTANT)
    Q_PROPERTY(QString DefaultSource READ defaultSource CONSTANT)
    Q_PROPERTY(QString BuiltinDefaultSource READ builtinDefaultSource CONSTANT)
    Q_PROPERTY(QString Items READ items CONSTANT)

public:
    QString status() const { return QStringLiteral("ready"); }
    QString defaultSource() const { return QStringLiteral("/cache/nature/1.jpg"); }
    QString builtinDefaultSource() const
    {
        return QStringLiteral("/usr/share/dragonfruit/wallpapers/Default.jpg");
    }
    QString items() const
    {
        return QStringLiteral(
            "[{\"pageid\":1,\"title\":\"File:Nature.jpg\",\"artist\":\"A\","
            "\"licenseShortName\":\"CC BY 4.0\",\"licenseUrl\":\"https://lic\","
            "\"pageUrl\":\"https://page\",\"description\":\"d\",\"category\":\"nature\","
            "\"width\":3840,\"height\":2160,\"mime\":\"image/jpeg\","
            "\"sourceUrl\":\"https://src\",\"localPath\":\"/cache/nature/1.jpg\","
            "\"fetchedAt\":1}]");
    }

public slots:
    QString Preload() { return items(); }
};

// A minimal stand-in for settingsd's object, with the same method/signal
// signatures the real daemon exports (the same fake `tst_settingsclient` uses).
class FakeSettingsService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Settings1")

public:
    explicit FakeSettingsService(QObject *parent = nullptr)
        : QObject(parent)
        , m_values(settingsSchemaDefaults())
    {
    }

    QVariant value(const QString &key) const { return m_values.value(key); }
    // T268: how many `Set` calls the client actually made. A slider drag must
    // produce one per commit, not one per pixel.
    int setCalls() const { return m_setCalls; }

    void setValue(const QString &key, const QVariant &value)
    {
        m_values.insert(key, value);
        emit Changed(key, QDBusVariant(value));
    }

public slots:
    QDBusVariant Get(const QString &key) const
    {
        return QDBusVariant(m_values.value(key));
    }

    void Set(const QString &key, const QDBusVariant &value)
    {
        ++m_setCalls;
        setValue(key, value.variant());
    }

    QVariantMap GetAll() const { return m_values; }

    QStringList ListKeys() const { return m_values.keys(); }

signals:
    void Changed(const QString &key, const QDBusVariant &value);

private:
    QVariantMap m_values;
    int m_setCalls = 0;
};

// T268: the General pane's update policy owner. `SetAutomaticPolicy` is the
// write that opening General must not make.
class FakeUpdatesService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.SystemStatus1.Updates")

public:
    int policyWrites() const { return m_policyWrites; }

public slots:
    QString State() const
    {
        return QStringLiteral(
            "{\"kind\":\"updates\",\"state\":\"available\",\"phase\":\"available\","
            "\"updatesAvailable\":true,\"busy\":false,\"updateCount\":1,"
            "\"securityCount\":1,\"lastCheckedMs\":1,\"label\":\"1 Update Available\","
            "\"updates\":[]}");
    }
    // T282: the pane's open path is a real re-read, not the cached State.
    QString Refresh() const { return State(); }
    void SetAutomaticPolicy(bool autoCheck, bool autoInstall)
    {
        ++m_policyWrites;
        m_autoCheck = autoCheck;
        m_autoInstall = autoInstall;
    }
    void Check() {}
    void Install() {}
    void Reboot() {}

private:
    int m_policyWrites = 0;
    bool m_autoCheck = false;
    bool m_autoInstall = false;
};

// A QML control bound to one key through the `Settings` singleton: the exact
// two-way pattern panes use (write on toggle, bind back to `Settings.values`).
const char *kControlQml = R"(
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

Item {
    id: stage
    property alias toggle: reduceMotion
    property alias current: reduceMotion.checked
    property var settings: Settings
    width: 240
    height: 60

    // The Lock Screen pane's `Never` path in miniature: an integral JavaScript
    // literal written through the same `Settings` singleton.
    function writeNever() { Settings.set("idle.dim", 0) }

    Toggle {
        id: reduceMotion
        text: "Reduce motion"
        onToggled: (checked) => Settings.set("accessibility.reduceMotion", checked)
        Binding {
            target: reduceMotion
            property: "checked"
            value: Settings.values["accessibility.reduceMotion"] === true
        }
    }
}
)";

} // namespace

// A minimal stand-in for the menu broker. `tst_settings_live` runs on a private
// session bus with no real broker, so this object can own the well-known name
// for the publisher-restart test and count the publications it receives.
class FakeMenuBroker : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.MenuBroker1")

public:
    int publishCount() const { return m_publishCount; }
    QString lastAppId() const { return m_lastAppId; }
    QString lastModel() const { return m_lastModel; }

public slots:
    bool Publish(const QString &appId, const QString &model)
    {
        ++m_publishCount;
        m_lastAppId = appId;
        m_lastModel = model;
        return true;
    }

    bool Withdraw(const QString &appId)
    {
        m_withdrawn = appId;
        return true;
    }

private:
    int m_publishCount = 0;
    QString m_lastAppId;
    QString m_lastModel;
    QString m_withdrawn;
};

class TestSettingsLive : public QObject
{
    Q_OBJECT

private:
    // Build the QML control with `DF_SETTINGS_FIXTURE` in the requested state.
    // The singleton reads the environment once, when the engine creates it.
    QObject *loadControl(QQmlEngine &engine, bool fixture)
    {
        if (fixture)
            qputenv("DF_SETTINGS_FIXTURE", "1");
        else
            qunsetenv("DF_SETTINGS_FIXTURE");

        engine.addImportPath(QStringLiteral(DF_QML_IMPORT_DIR));
        QQmlComponent component(&engine);
        component.setData(kControlQml, QUrl(QStringLiteral("qrc:/tst_settings_live.qml")));
        QObject *root = component.create();
        if (!root)
            qWarning() << "QML load failed:" << component.errorString();
        return root;
    }

    QObject *loadModuleType(QQmlEngine &engine, const QString &type)
    {
        engine.addImportPath(QStringLiteral(DF_QML_IMPORT_DIR));
        QQmlComponent component(&engine);
        const QByteArray source = QByteArrayLiteral(
            "import QtQuick\nimport Dragonfruit.Settings\n")
            + type.toUtf8() + QByteArrayLiteral(" { }");
        component.setData(source, QUrl(QStringLiteral("qrc:/") + type
                                       + QStringLiteral(".qml")));
        QObject *root = component.create();
        if (!root)
            qWarning() << type << "QML load failed:" << component.errorString();
        return root;
    }

    static QObject *toggleOf(QObject *root)
    {
        return root->property("toggle").value<QObject *>();
    }

private slots:
    void mockFixtureAppliesLiveWithoutABus()
    {
        // No bus, no daemon: the fixture client keeps writes in memory, so the
        // control still round-trips (the absent-provider behavior panes rely
        // on). The store is deterministic, so this test never touches the
        // host session.
        QQmlEngine engine;
        QScopedPointer<QObject> root(loadControl(engine, true));
        QVERIFY(root);
        QObject *toggle = toggleOf(root.data());
        QVERIFY(toggle);
        QCOMPARE(toggle->property("checked").toBool(), false);

        QMetaObject::invokeMethod(toggle, "toggle");
        QTRY_COMPARE(toggle->property("checked").toBool(), true);
    }

    // T268: constructing the bridge must not construct or refresh any adapter
    // client. The old constructor made one read per adapter (~20 D-Bus calls)
    // before the first pane painted.
    void noAdapterClientsAreCreatedBeforeAPaneOpens()
    {
        QQmlEngine engine;
        QScopedPointer<QObject> root(loadControl(engine, true));
        QVERIFY(root);
        QObject *settings = root->property("settings").value<QObject *>();
        QVERIFY(settings);
        QCOMPARE(settings->property("adapterClientCount").toInt(), 0);

        // One pane's explicit refresh creates exactly its own client.
        QMetaObject::invokeMethod(settings, "refreshBluetooth");
        QTRY_COMPARE(settings->property("adapterClientCount").toInt(), 1);
    }

    void controlRoundTripsThroughASettingsService()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakeSettingsService service;
        QVERIFY(bus.registerService(kService));
        QVERIFY(bus.registerObject(kPath, &service,
                                   QDBusConnection::ExportAllSlots
                                       | QDBusConnection::ExportAllSignals));

        QQmlEngine engine;
        QScopedPointer<QObject> root(loadControl(engine, false));
        QVERIFY(root);
        QObject *toggle = toggleOf(root.data());
        QVERIFY(toggle);
        QTRY_COMPARE(toggle->property("checked").toBool(), false);

        // The user changes the control: the write reaches the settings owner.
        QMetaObject::invokeMethod(toggle, "toggle");
        QTRY_VERIFY(service.value(kKey).toBool());

        // A change made by another client (settingsd's `Changed`) reaches the
        // control without a restart or a poll.
        service.setValue(kKey, false);
        QTRY_COMPARE(toggle->property("checked").toBool(), false);

        // A second user change round-trips as well (no stale binding).
        QMetaObject::invokeMethod(toggle, "toggle");
        QTRY_VERIFY(service.value(kKey).toBool());

        bus.unregisterObject(kPath);
        bus.unregisterService(kService);
    }

    void bridgeExposesTheWallpaperProviderProperties()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakeWallpaperService provider;
        QVERIFY(bus.registerService(kWallpaperService));
        QVERIFY(bus.registerObject(kWallpaperPath, &provider,
                                   QDBusConnection::ExportAllProperties
                                       | QDBusConnection::ExportAllSlots));

        QQmlEngine engine;
        QScopedPointer<QObject> root(loadControl(engine, false));
        QVERIFY(root);
        QObject *settings = root->property("settings").value<QObject *>();
        QVERIFY(settings);

        // The bridge read the provider properties on construction (never polls).
        QTRY_COMPARE(settings->property("providerStatus").toString(), QStringLiteral("ready"));
        QCOMPARE(settings->property("providerDefault").toString(),
                 QStringLiteral("/cache/nature/1.jpg"));
        QCOMPARE(settings->property("wallpaperBuiltinDefault").toString(),
                 QStringLiteral("/usr/share/dragonfruit/wallpapers/Default.jpg"));

        const QVariantList items = settings->property("providerItems").toList();
        QCOMPARE(items.size(), 1);
        const QVariantMap entry = items.constFirst().toMap();
        QCOMPARE(entry.value(QStringLiteral("source")).toString(),
                 QStringLiteral("/cache/nature/1.jpg"));
        QCOMPARE(entry.value(QStringLiteral("artist")).toString(), QStringLiteral("A"));
        QCOMPARE(entry.value(QStringLiteral("licenseShortName")).toString(),
                 QStringLiteral("CC BY 4.0"));
        QVERIFY(entry.value(QStringLiteral("url")).toString().startsWith(QStringLiteral("file://")));

        // A live `PropertiesChanged` (the provider's standard notification)
        // reaches the bridge without a re-read of the object.
        QDBusMessage changed = QDBusMessage::createSignal(
            kWallpaperPath, QStringLiteral("org.freedesktop.DBus.Properties"),
            QStringLiteral("PropertiesChanged"));
        QVariantMap props;
        props.insert(QStringLiteral("DefaultSource"), QStringLiteral("/cache/water/2.jpg"));
        changed << kWallpaperInterface << props << QStringList();
        QVERIFY(bus.send(changed));
        QTRY_COMPARE(settings->property("providerDefault").toString(),
                     QStringLiteral("/cache/water/2.jpg"));

        // `Preload` is exposed through the bridge and is a safe round trip.
        QMetaObject::invokeMethod(settings, "preloadWallpapers");
        QTest::qWait(50);
        QCOMPARE(settings->property("providerItems").toList().size(), 1);

        bus.unregisterObject(kWallpaperPath);
        bus.unregisterService(kWallpaperService);
    }

    void controlRoundTripsThroughTheRealSettingsd()
    {
        const QByteArray binary = qgetenv("DF_SETTINGSD_BIN");
        if (binary.isEmpty())
            QSKIP("dragonfruit-settingsd not found at configure time");

        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        // A scratch config home so the run never reads or writes the real one.
        QTemporaryDir scratch;
        QVERIFY(scratch.isValid());

        QProcess daemon;
        QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
        env.insert(QStringLiteral("XDG_CONFIG_HOME"), scratch.path());
        env.insert(QStringLiteral("HOME"), scratch.path());
        daemon.setProcessEnvironment(env);
        daemon.start(QString::fromLocal8Bit(binary), {});
        QVERIFY(daemon.waitForStarted(5000));

        // Wait for the daemon to take the well-known name.
        QDBusConnectionInterface *iface = bus.interface();
        QVERIFY(iface);
        for (int i = 0; i < 400 && !iface->isServiceRegistered(kService); ++i)
            QTest::qWait(25);
        QVERIFY2(iface->isServiceRegistered(kService), "settingsd never took the bus name");

        QQmlEngine engine;
        QScopedPointer<QObject> root(loadControl(engine, false));
        QVERIFY(root);
        QObject *toggle = toggleOf(root.data());
        QVERIFY(toggle);

        QDBusInterface settings(kService, kPath, kInterface, bus);
        QVERIFY(settings.isValid());

        // Seed the durable owner false, then let the control write true.
        settings.call(QStringLiteral("Set"), kKey, QVariant::fromValue(QDBusVariant(false)));
        QTRY_COMPARE(toggle->property("checked").toBool(), false);

        QMetaObject::invokeMethod(toggle, "toggle");
        QTRY_VERIFY(settings.call(QStringLiteral("Get"), kKey)
                        .arguments()
                        .value(0)
                        .value<QDBusVariant>()
                        .variant()
                        .toBool());

        // An external write (as another client would make) updates the control.
        settings.call(QStringLiteral("Set"), kKey, QVariant::fromValue(QDBusVariant(false)));
        QTRY_COMPARE(toggle->property("checked").toBool(), false);

        // An integral QML value must reach the daemon as int64. QML hands the
        // `Settings.set` slot an `int`; before the coercion fix that crossed
        // the bus as `i`, settingsd rejected it, and the Lock Screen's `Never`
        // option (0) silently reverted to its previous value.
        QMetaObject::invokeMethod(root.data(), "writeNever");
        QTRY_COMPARE(settings.call(QStringLiteral("Get"), QStringLiteral("idle.dim"))
                         .arguments().value(0).value<QDBusVariant>().variant()
                         .toLongLong(),
                     qlonglong(0));
        QCOMPARE(settings.call(QStringLiteral("Get"), QStringLiteral("idle.dim"))
                     .arguments().value(0).value<QDBusVariant>().variant()
                     .metaType().id(),
                 static_cast<int>(QMetaType::LongLong));

        daemon.terminate();
        QVERIFY(daemon.waitForFinished(5000));
    }

    // T268: a slider drag writes once on commit, never on every moved() tick.
    // The Dock size slider is the live reference control.
    void sliderDragEmitsOneSetPerCommit()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakeSettingsService service;
        QVERIFY(bus.registerService(kService));
        QVERIFY(bus.registerObject(kPath, &service,
                                   QDBusConnection::ExportAllSlots
                                       | QDBusConnection::ExportAllSignals));

        qunsetenv("DF_SETTINGS_FIXTURE");
        QQmlEngine engine;
        QScopedPointer<QObject> pane(loadModuleType(engine, QStringLiteral("DesktopDockPane")));
        QVERIFY(pane);
        QObject *slider = pane->property("sizeSlider").value<QObject *>();
        QVERIFY(slider);

        // A drag emits moved() per pixel; none of those may reach settingsd.
        for (int i = 0; i < 24; ++i)
            QMetaObject::invokeMethod(slider, "moved", Q_ARG(double, 0.2 + i * 0.02));
        QTest::qWait(120);
        QCOMPARE(service.setCalls(), 0);

        // Releasing commits exactly one durable write.
        QMetaObject::invokeMethod(slider, "committed", Q_ARG(double, 0.75));
        QTRY_COMPARE(service.setCalls(), 1);
        QTRY_VERIFY(qAbs(service.value(QStringLiteral("dock.size")).toDouble() - 0.75)
                    < 0.001);

        // A second commit is a second write, not a debounce that swallows it.
        QMetaObject::invokeMethod(slider, "committed", Q_ARG(double, 0.85));
        QTRY_COMPARE(service.setCalls(), 2);

        bus.unregisterObject(kPath);
        bus.unregisterService(kService);
    }

    // T268: opening General re-reads the host but never pushes the schema
    // defaults as an update policy. Before this fix, the pane's
    // Component.onCompleted called applyUpdatePolicy(false, false), silently
    // turning automatic updates off just by visiting the pane.
    void openingGeneralDoesNotForceTheUpdatePolicyOff()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        const QString statusService = QStringLiteral("org.dragonfruit.SystemStatus1");
        const QString statusPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
        FakeUpdatesService updates;
        QVERIFY(bus.registerService(statusService));
        QVERIFY(bus.registerObject(statusPath, &updates,
                                   QDBusConnection::ExportAllSlots));

        qputenv("DF_SETTINGS_FIXTURE", "1");
        qunsetenv("DF_UPDATES_FIXTURE");
        QQmlEngine engine;
        QScopedPointer<QObject> pane(loadModuleType(engine, QStringLiteral("GeneralPane")));
        QVERIFY(pane);
        QTRY_VERIFY(pane->property("providerAvailable").toBool());

        // Opening the pane is a read, not a policy write.
        QTest::qWait(120);
        QCOMPARE(updates.policyWrites(), 0);

        bus.unregisterObject(statusPath);
        bus.unregisterService(statusService);
    }

    // T-14.2b: the publisher's activation endpoint is a real D-Bus object on
    // the app's unique bus name. The broker addresses it only after validating
    // the owner and generation; this test drives the endpoint directly and
    // proves the synchronous QML handshake returns the app's result.
    void menuPublisherActivationRoundTripsOverTheBus()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        MenuPublisher publisher;
        QSignalSpy spy(&publisher, &MenuPublisher::activationRequested);
        QVERIFY(spy.isValid());
        QObject::connect(
            &publisher, &MenuPublisher::activationRequested, &publisher,
            [&publisher](const QString &action, const QString &, qulonglong, uint) {
                publisher.completeActivation(action == QStringLiteral("edit.copy"));
            });
        // No broker in this test: the publish is a normal miss, but the app id
        // and the activation endpoint are registered before the call.
        publisher.publish(QStringLiteral("org.dragonfruit.Settings.desktop"),
                          QStringLiteral(R"({"appName":"Settings"})"));

        auto activate = [&bus](const QString &appId, const QString &action) {
            QDBusMessage message = QDBusMessage::createMethodCall(
                bus.baseService(), QStringLiteral("/org/dragonfruit/MenuPublisher1"),
                QStringLiteral("org.dragonfruit.MenuPublisher1"),
                QStringLiteral("Activate"));
            message << appId << action
                    << QStringLiteral(R"({"action":"%1","enabled":true})").arg(action)
                    << qulonglong(4) << uint(9);
            return bus.call(message);
        };

        const QDBusMessage accepted =
            activate(QStringLiteral("org.dragonfruit.Settings.desktop"),
                     QStringLiteral("edit.copy"));
        QCOMPARE(accepted.type(), QDBusMessage::ReplyMessage);
        QCOMPARE(accepted.arguments().value(0).toBool(), true);
        QCOMPARE(spy.count(), 1);
        QCOMPARE(spy.at(0).at(0).toString(), QStringLiteral("edit.copy"));
        QCOMPARE(spy.at(0).at(2).toULongLong(), qulonglong(4));
        QCOMPARE(spy.at(0).at(3).toUInt(), uint(9));

        // A request for another app id is rejected without reaching QML.
        const QDBusMessage wrongApp =
            activate(QStringLiteral("org.example.Other"), QStringLiteral("edit.copy"));
        QCOMPARE(wrongApp.arguments().value(0).toBool(), false);
        QCOMPARE(spy.count(), 1);

        // A command the app does not handle is rejected, never reported as
        // accepted just because the endpoint exists.
        const QDBusMessage rejected =
            activate(QStringLiteral("org.dragonfruit.Settings.desktop"),
                     QStringLiteral("no-such-action"));
        QCOMPARE(rejected.arguments().value(0).toBool(), false);
        QCOMPARE(spy.count(), 2);
    }

    // T-14.2b: a broker restart empties its publication table. The publisher
    // watches the well-known name and re-publishes its cached model, so the
    // shell's next resolve sees the native tier again.
    void menuPublisherRePublishesAfterBrokerRestart()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakeMenuBroker broker;
        QVERIFY(bus.registerService(QStringLiteral("org.dragonfruit.MenuBroker1")));
        QVERIFY(bus.registerObject(QStringLiteral("/org/dragonfruit/MenuBroker1"), &broker,
                                   QDBusConnection::ExportAllSlots));

        MenuPublisher publisher;
        const QString model = QStringLiteral(R"({"appName":"Settings"})");
        QVERIFY(publisher.publish(QStringLiteral("org.dragonfruit.Settings.desktop"), model));
        QCOMPARE(broker.publishCount(), 1);
        QCOMPARE(broker.lastAppId(), QStringLiteral("org.dragonfruit.Settings.desktop"));

        // Simulate the broker exiting and a new instance taking the name.
        bus.unregisterObject(QStringLiteral("/org/dragonfruit/MenuBroker1"));
        bus.unregisterService(QStringLiteral("org.dragonfruit.MenuBroker1"));
        QTest::qWait(100);
        QVERIFY(bus.registerService(QStringLiteral("org.dragonfruit.MenuBroker1")));
        QVERIFY(bus.registerObject(QStringLiteral("/org/dragonfruit/MenuBroker1"), &broker,
                                   QDBusConnection::ExportAllSlots));
        QTRY_COMPARE(broker.publishCount(), 2);

        bus.unregisterObject(QStringLiteral("/org/dragonfruit/MenuBroker1"));
        bus.unregisterService(QStringLiteral("org.dragonfruit.MenuBroker1"));
    }
};

QTEST_MAIN(TestSettingsLive)
#include "tst_settings_live.moc"