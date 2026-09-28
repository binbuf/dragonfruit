// SPDX-License-Identifier: MIT
// The Settings app's Printers and Scanners seam (T-15.12b).
//
// The Printers & Scanners pane never touches D-Bus: it binds the `Settings`
// singleton, and the bridge forwards reads and writes here. The live
// `DbusPrintersClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Printers` interface (the same adapter the
// Control Center tile reads), and `MockPrintersClient` serves a deterministic
// fixture for the headless pane tests (`DF_PRINTERS_FIXTURE`). Absence is a
// normal state: `available()` is false and the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class QDBusServiceWatcher;

class PrintersClient : public QObject
{
    Q_OBJECT

public:
    explicit PrintersClient(QObject *parent = nullptr) : QObject(parent) {}
    ~PrintersClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `printers` view (`{state, glyph, label, present,
    // printersAvailable, scannersAvailable, printerCount, scannerCount,
    // queuedJobCount, defaultPrinter, printers, scanners}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host stack once (the pane calls this on open).
    virtual void refresh() = 0;
    // The three explicit queue writes; the host re-reads and pushes the new
    // view.
    virtual void setDefaultPrinter(const QString &name) = 0;
    virtual void setPrinterAcceptingJobs(const QString &name, bool accepting) = 0;
    virtual void cancelJob(int jobId) = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusPrintersClient : public PrintersClient
{
    Q_OBJECT

public:
    explicit DbusPrintersClient(QObject *parent = nullptr);
    ~DbusPrintersClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setDefaultPrinter(const QString &name) override;
    void setPrinterAcceptingJobs(const QString &name, bool accepting) override;
    void cancelJob(int jobId) override;

private:
    void call(const QString &method, const QVariantList &arguments);
    void applyReply(const QByteArray &json);
    QString m_service;
    QString m_path;
    QString m_interface;
    QVariantMap m_view;
    bool m_available = false;
    QDBusServiceWatcher *m_watcher = nullptr;
};

// The fixture client used by `DF_PRINTERS_FIXTURE`: a Dragonfruit workstation
// with two queued printers (an idle default and a processing one with a job)
// and one scanner. The writes mutate the simulated CUPS queue in place and
// re-emit, so every pane control's round-trip is observable with no bus.
class MockPrintersClient : public PrintersClient
{
    Q_OBJECT

public:
    explicit MockPrintersClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setDefaultPrinter(const QString &name) override;
    void setPrinterAcceptingJobs(const QString &name, bool accepting) override;
    void cancelJob(int jobId) override;
    void resetForTest() override;

private:
    void rebuild();
    // The simulated CUPS queue: the T-15.12a `PrinterData`/`PrintJobData`
    // shape as Qt values.
    struct Job {
        int id = 0;
        QString user;
        qulonglong size = 0;
    };
    struct Printer {
        QString name;
        QString displayName;
        QString makeAndModel;
        QString location;
        QString uri;
        QString state; // idle / processing / stopped / unknown
        QString stateMessage;
        bool acceptingJobs = true;
        bool enabled = true;
        bool isDefault = false;
        QList<Job> jobs;
    };
    struct Scanner {
        QString device;
        QString description;
        QString kind; // flatbed / sheetfed / handheld / unknown
    };
    QList<Printer> m_printers;
    QList<Scanner> m_scanners;
    bool m_cupsAvailable = true;
    bool m_saneAvailable = true;
    QVariantMap m_view;
};