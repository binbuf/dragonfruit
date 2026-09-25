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

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusVariant>
#include <QProcess>
#include <QProcessEnvironment>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QTemporaryDir>
#include <QTest>
#include <QVariantMap>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.Settings1");
const QString kPath = QStringLiteral("/org/dragonfruit/Settings1");
const QString kInterface = QStringLiteral("org.dragonfruit.Settings1");
const QString kKey = QStringLiteral("accessibility.reduceMotion");

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
        setValue(key, value.variant());
    }

    QVariantMap GetAll() const { return m_values; }

    QStringList ListKeys() const { return m_values.keys(); }

signals:
    void Changed(const QString &key, const QDBusVariant &value);

private:
    QVariantMap m_values;
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
    width: 240
    height: 60

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

        daemon.terminate();
        QVERIFY(daemon.waitForFinished(5000));
    }
};

QTEST_MAIN(TestSettingsLive)
#include "tst_settings_live.moc"