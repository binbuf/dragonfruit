// SPDX-License-Identifier: GPL-3.0-or-later
// Panel brightness through logind (T319).
//
// `/sys/class/backlight/*/brightness` is normally root-owned, so the shell
// does not write it directly. It resolves the device and asks logind's
// `Session.SetBrightness("backlight", name, value)` for the unprivileged
// write. The call is asynchronous with a short timeout: a slow or absent
// logind never blocks the GUI thread. When logind is absent the backend falls
// back to the sysfs file only if it is writable; otherwise it logs and reports
// a visible absence reason instead of claiming success.
//
// The class is Wayland-free and the bus/sysfs root are injectable, so
// `tst_brightness` drives a fake logind session and a temporary sysfs tree.
#pragma once

#include <QDBusConnection>
#include <QObject>
#include <QString>

#include "backlight.h"

class BrightnessBackend : public QObject
{
    Q_OBJECT

public:
    explicit BrightnessBackend(QObject *parent = nullptr);
    ~BrightnessBackend() override;

    // Test seams. `setBusAddress` selects a private bus instead of the system
    // bus; `setSysfsRoot` selects a fixture tree instead of
    // `/sys/class/backlight`; `setSessionPath` selects the logind session
    // object; `setSysfsWriteEnabled(false)` simulates a read-only sysfs.
    void setBusAddress(const QString &address);
    void setSysfsRoot(const QString &root);
    void setSessionPath(const QString &path);
    void setTimeoutMs(int milliseconds);
    void setSysfsWriteEnabled(bool enabled);

    // Resolve the device and decide whether a real write path exists. Called
    // lazily by `setLevel`, and explicitly at startup so the idle dim effect
    // knows whether the panel can be dimmed without the overlay fallback.
    bool probe();

    // True when a backlight device is present and logind (or a writable sysfs
    // fallback) can actually change it.
    bool available() const { return m_available; }
    bool logindAvailable();
    const BacklightDevice &device() const { return m_device; }
    QString deviceName() const { return m_device.name; }
    int maxBrightness() const { return m_device.maxBrightness; }
    // The user-facing reason no write path exists (empty when available).
    QString absenceReason() const { return m_absenceReason; }

    // Apply one 0..1 level. Returns true when the request was dispatched; an
    // asynchronous logind error is reported through `failed`. The mapping is
    // available immediately in `lastMapped`.
    bool setLevel(double level);

    double lastLevel() const { return m_lastLevel; }
    int lastMapped() const { return m_lastMapped; }
    QString lastMethod() const { return m_lastMethod; }

signals:
    // A logind or sysfs write was dispatched/completed successfully.
    void applied(double level, int mapped, const QString &method);
    // No write path exists or the async logind call failed. The reason is
    // already logged; a caller may surface it.
    void failed(const QString &reason);

private:
    QDBusConnection bus();
    bool writeSysfs(int value);

    QString m_busAddress;
    QString m_connectionName;
    QString m_sysfsRoot = QStringLiteral("/sys/class/backlight");
    QString m_sessionPath =
        QStringLiteral("/org/freedesktop/login1/session/auto");
    int m_timeoutMs = 1500;
    bool m_sysfsWriteEnabled = true;
    bool m_available = false;
    bool m_logind = false;
    bool m_sysfsWritable = false;
    BacklightDevice m_device;
    QString m_absenceReason;
    double m_lastLevel = -1.0;
    int m_lastMapped = -1;
    QString m_lastMethod;
};
