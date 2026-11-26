// SPDX-License-Identifier: GPL-3.0-only
// Panel-brightness tests (T319): the backlight resolver, the 0..1 -> device
// range mapping, the logind `Session.SetBrightness` request, the sysfs
// fallback, and the absent no-op. A fake logind session runs on a private bus
// and the sysfs tree is a temporary directory, so the host logind and the
// real `/sys/class/backlight` are never touched. `DBUS_SYSTEM_BUS_ADDRESS`
// points at a dead path so a bug cannot reach the developer's session.
#include "backlight.h"
#include "brightnessbackend.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusContext>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QTemporaryDir>
#include <QThread>
#include <QtTest>

namespace {

const QString kLogindService = QStringLiteral("org.freedesktop.login1");
const QString kSessionPath =
    QStringLiteral("/org/freedesktop/login1/session/auto");

class FakeLogind : public QObject, protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.freedesktop.login1.Session")

public:
    int calls = 0;
    QString subsystem;
    QString deviceName;
    uint brightness = 0;
    int delayMs = 0;

public slots:
    void SetBrightness(const QString &subsystem, const QString &name,
                       uint brightness)
    {
        ++calls;
        this->subsystem = subsystem;
        this->deviceName = name;
        this->brightness = brightness;
        if (delayMs > 0)
            QThread::msleep(static_cast<unsigned long>(delayMs));
    }
};

bool registerLogind(QDBusConnection &bus, FakeLogind &service)
{
    if (!bus.registerService(kLogindService))
        return false;
    return bus.registerObject(kSessionPath, &service,
                              QDBusConnection::ExportAllSlots);
}

QString busAddress()
{
    return QString::fromLocal8Bit(qgetenv("DBUS_SESSION_BUS_ADDRESS"));
}

bool writeFile(const QString &path, const QByteArray &data)
{
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    return file.write(data) == data.size();
}

// A deterministic backlight tree: the firmware device is present but the raw
// hardware device must win.
bool makeSysfs(const QString &root, int maxBrightness = 1200)
{
    const QString raw = root + QStringLiteral("/intel_backlight");
    if (!QDir().mkpath(raw))
        return false;
    if (!writeFile(raw + QStringLiteral("/max_brightness"),
                   QByteArray::number(maxBrightness)))
        return false;
    if (!writeFile(raw + QStringLiteral("/type"), "raw"))
        return false;
    if (!writeFile(raw + QStringLiteral("/brightness"),
                   QByteArray::number(maxBrightness)))
        return false;

    const QString firmware = root + QStringLiteral("/acpi_video0");
    if (!QDir().mkpath(firmware))
        return false;
    if (!writeFile(firmware + QStringLiteral("/max_brightness"), "100"))
        return false;
    if (!writeFile(firmware + QStringLiteral("/type"), "firmware"))
        return false;
    return writeFile(firmware + QStringLiteral("/brightness"), "100");
}

} // namespace

class TestBrightness : public QObject
{
    Q_OBJECT

private slots:
    void cleanup()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (bus.isConnected()) {
            bus.unregisterObject(kSessionPath);
            bus.unregisterService(kLogindService);
        }
    }

    void mapsTheSettingOntoTheDeviceRange()
    {
        QCOMPARE(mapBrightnessLevel(0.0, 1200), 0);
        QCOMPARE(mapBrightnessLevel(0.5, 1200), 600);
        QCOMPARE(mapBrightnessLevel(1.0, 1200), 1200);
        QCOMPARE(mapBrightnessLevel(0.333, 100), 33);
        QCOMPARE(mapBrightnessLevel(-1.0, 1200), 0);
        QCOMPARE(mapBrightnessLevel(2.0, 1200), 1200);
        QCOMPARE(mapBrightnessLevel(0.5, 0), 0);
    }

    void resolvesThePreferredDevice()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QVERIFY(makeSysfs(directory.path()));

        // A directory with no max_brightness is not a device.
        const QString broken = directory.filePath(QStringLiteral("broken"));
        QVERIFY(QDir().mkpath(broken));

        const BacklightDevice device = resolveBacklightDevice(directory.path());
        QVERIFY(device.isValid());
        QCOMPARE(device.name, QStringLiteral("intel_backlight"));
        QCOMPARE(device.maxBrightness, 1200);
        QCOMPARE(device.type, QStringLiteral("raw"));
        QVERIFY(device.path.endsWith(QStringLiteral("/intel_backlight")));

        const BacklightDevice empty =
            resolveBacklightDevice(directory.filePath(QStringLiteral("missing")));
        QVERIFY(!empty.isValid());
    }

    void writesTheMappedValueThroughLogind()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        QVERIFY(registerLogind(bus, service));

        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QVERIFY(makeSysfs(directory.path()));

        BrightnessBackend backend;
        backend.setBusAddress(busAddress());
        backend.setSysfsRoot(directory.path());
        QVERIFY(backend.probe());
        QVERIFY(backend.available());
        QCOMPARE(backend.deviceName(), QStringLiteral("intel_backlight"));
        QCOMPARE(backend.maxBrightness(), 1200);

        QVERIFY(backend.setLevel(0.4));
        QCOMPARE(backend.lastLevel(), 0.4);
        QCOMPARE(backend.lastMapped(), 480);
        QCOMPARE(backend.lastMethod(), QStringLiteral("logind"));

        QTRY_COMPARE(service.calls, 1);
        QCOMPARE(service.subsystem, QStringLiteral("backlight"));
        QCOMPARE(service.deviceName, QStringLiteral("intel_backlight"));
        QCOMPARE(service.brightness, 480u);

        // The next level maps through the same device range.
        QVERIFY(backend.setLevel(1.0));
        QCOMPARE(backend.lastMapped(), 1200);
        QTRY_COMPARE(service.calls, 2);
        QCOMPARE(service.brightness, 1200u);

        bus.unregisterObject(kSessionPath);
        bus.unregisterService(kLogindService);
    }

    void absentDeviceIsALoggedNoOp()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());

        BrightnessBackend backend;
        backend.setBusAddress(busAddress());
        backend.setSysfsRoot(directory.path());
        QSignalSpy failed(&backend, &BrightnessBackend::failed);

        QVERIFY(!backend.probe());
        QVERIFY(!backend.available());
        QVERIFY(backend.absenceReason().contains(QStringLiteral("backlight")));
        QVERIFY(!backend.setLevel(0.5));
        QCOMPARE(failed.count(), 1);
        QVERIFY(backend.lastMethod().isEmpty());
    }

    void absentLogindFallsBackToWritableSysfs()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        if (bus.interface()->isServiceRegistered(kLogindService))
            QSKIP("the private bus unexpectedly owns the logind name");

        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QVERIFY(makeSysfs(directory.path()));

        BrightnessBackend backend;
        backend.setBusAddress(busAddress());
        backend.setSysfsRoot(directory.path());
        QVERIFY(backend.probe());
        QVERIFY(backend.available());
        QVERIFY(backend.setLevel(0.25));
        QCOMPARE(backend.lastMapped(), 300);
        QCOMPARE(backend.lastMethod(), QStringLiteral("sysfs"));

        QFile file(directory.filePath(
            QStringLiteral("intel_backlight/brightness")));
        QVERIFY(file.open(QIODevice::ReadOnly | QIODevice::Text));
        QCOMPARE(QString::fromUtf8(file.readAll()).trimmed(),
                 QStringLiteral("300"));
    }

    void absentLogindAndReadOnlySysfsIsALoggedNoOp()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        if (bus.interface()->isServiceRegistered(kLogindService))
            QSKIP("the private bus unexpectedly owns the logind name");

        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QVERIFY(makeSysfs(directory.path()));

        BrightnessBackend backend;
        backend.setBusAddress(busAddress());
        backend.setSysfsRoot(directory.path());
        backend.setSysfsWriteEnabled(false);
        QSignalSpy failed(&backend, &BrightnessBackend::failed);

        QVERIFY(!backend.probe());
        QVERIFY(!backend.available());
        QVERIFY(backend.absenceReason().contains(QStringLiteral("logind")));
        QVERIFY(!backend.setLevel(0.5));
        QCOMPARE(failed.count(), 1);
    }

    void doesNotBlockTheGuiThread()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        FakeLogind service;
        service.delayMs = 250;
        QVERIFY(registerLogind(bus, service));

        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QVERIFY(makeSysfs(directory.path()));

        BrightnessBackend backend;
        backend.setBusAddress(busAddress());
        backend.setSysfsRoot(directory.path());
        backend.setTimeoutMs(2000);
        QVERIFY(backend.probe());

        QElapsedTimer timer;
        timer.start();
        QVERIFY(backend.setLevel(0.5));
        const qint64 elapsed = timer.elapsed();
        QVERIFY2(elapsed < 150,
                 qPrintable(QStringLiteral("setLevel blocked for %1 ms")
                                .arg(elapsed)));

        // The slow call still completes asynchronously.
        QTRY_COMPARE(service.calls, 1);
        QCOMPARE(service.brightness, 600u);

        bus.unregisterObject(kSessionPath);
        bus.unregisterService(kLogindService);
    }
};

QTEST_GUILESS_MAIN(TestBrightness)
#include "tst_brightness.moc"
