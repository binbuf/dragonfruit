// SPDX-License-Identifier: MIT
#include "PrintersClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

#include <algorithm>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Printers");

QString stateLabel(const QString &state)
{
    if (state == QStringLiteral("idle"))
        return QStringLiteral("Idle");
    if (state == QStringLiteral("processing"))
        return QStringLiteral("Printing");
    if (state == QStringLiteral("stopped"))
        return QStringLiteral("Stopped");
    return QStringLiteral("Unknown");
}

QString kindLabel(const QString &kind)
{
    if (kind == QStringLiteral("flatbed"))
        return QStringLiteral("Flatbed");
    if (kind == QStringLiteral("sheetfed"))
        return QStringLiteral("Sheet-fed");
    if (kind == QStringLiteral("handheld"))
        return QStringLiteral("Handheld");
    return QStringLiteral("Scanner");
}

} // namespace

// --- DbusPrintersClient ----------------------------------------------------

DbusPrintersClient::DbusPrintersClient(QObject *parent)
    : PrintersClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_available = available();
    m_watcher = new QDBusServiceWatcher(
        m_service, bus, QDBusServiceWatcher::WatchForRegistration
                          | QDBusServiceWatcher::WatchForUnregistration,
        this);
    connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) {
                m_available = true;
                emit availableChanged(true);
                refresh();
            });
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                m_available = false;
                emit availableChanged(false);
                if (!m_view.isEmpty()) {
                    m_view.clear();
                    emit changed(m_view);
                }
            });
    refresh();
}

DbusPrintersClient::~DbusPrintersClient() = default;

bool DbusPrintersClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusPrintersClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusPrintersClient::setDefaultPrinter(const QString &name)
{
    call(QStringLiteral("SetDefaultPrinter"), {name});
}

void DbusPrintersClient::setPrinterAcceptingJobs(const QString &name, bool accepting)
{
    call(QStringLiteral("SetPrinterAcceptingJobs"), {name, accepting});
}

void DbusPrintersClient::cancelJob(int jobId)
{
    call(QStringLiteral("CancelJob"), {jobId});
}

void DbusPrintersClient::call(const QString &method, const QVariantList &arguments)
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        return;
    }
    const bool isState = method == QStringLiteral("State");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isState)
                    applyReply(reply.value().toUtf8());
                else
                    // The write reply is the outcome report; the new state is
                    // the host's re-read, never invented here.
                    refresh();
            });
}

void DbusPrintersClient::applyReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_view)
        return;
    m_view = view;
    emit changed(m_view);
}

// --- MockPrintersClient ----------------------------------------------------

MockPrintersClient::MockPrintersClient(QObject *parent)
    : PrintersClient(parent)
{
    resetForTest();
}

void MockPrintersClient::resetForTest()
{
    m_cupsAvailable = true;
    m_saneAvailable = true;
    m_printers = {
        { QStringLiteral("Canon_MF230"), QStringLiteral("Canon MF230"),
          QStringLiteral("Canon MF230 Series"), QStringLiteral("Office"),
          QStringLiteral("ipp://192.168.0.5/ipp/print"), QStringLiteral("idle"),
          QStringLiteral("Idle, Last Used"), true, true, true,
          { { 12, QStringLiteral("dan"), 2048 } } },
        { QStringLiteral("HP_LaserJet"), QStringLiteral("HP LaserJet"),
          QString(), QString(), QStringLiteral("ipp://192.168.0.9/ipp/print"),
          QStringLiteral("processing"), QStringLiteral("Printing"), true, true,
          false, { { 9, QStringLiteral("sam"), 64 } } },
    };
    m_scanners = {
        { QStringLiteral("epson2:net:192.168.0.7"),
          QStringLiteral("Epson GT-1500 flatbed scanner"),
          QStringLiteral("flatbed") },
    };
    rebuild();
}

void MockPrintersClient::refresh()
{
    rebuild();
}

void MockPrintersClient::setDefaultPrinter(const QString &name)
{
    bool found = false;
    for (Printer &printer : m_printers) {
        printer.isDefault = printer.name == name;
        found = found || printer.name == name;
    }
    if (!found)
        return;
    rebuild();
}

void MockPrintersClient::setPrinterAcceptingJobs(const QString &name, bool accepting)
{
    for (Printer &printer : m_printers)
        if (printer.name == name)
            printer.acceptingJobs = accepting;
    rebuild();
}

void MockPrintersClient::cancelJob(int jobId)
{
    for (Printer &printer : m_printers) {
        for (int i = 0; i < printer.jobs.size(); ++i) {
            if (printer.jobs[i].id == jobId) {
                printer.jobs.removeAt(i);
                break;
            }
        }
    }
    rebuild();
}

void MockPrintersClient::rebuild()
{
    QList<Printer> printers = m_printers;
    // The default queue first, then by name (mirrors `PrintSnapshot`).
    std::stable_sort(printers.begin(), printers.end(), [](const Printer &a, const Printer &b) {
        if (a.isDefault != b.isDefault)
            return a.isDefault;
        return a.name < b.name;
    });
    QList<Scanner> scanners = m_scanners;
    std::stable_sort(scanners.begin(), scanners.end(), [](const Scanner &a, const Scanner &b) {
        return a.kind < b.kind;
    });

    int queuedJobs = 0;
    QJsonArray printerArray;
    for (const Printer &printer : printers) {
        queuedJobs += printer.jobs.size();
        QJsonArray jobArray;
        for (const Job &job : printer.jobs) {
            QJsonObject entry;
            entry.insert(QStringLiteral("id"), job.id);
            entry.insert(QStringLiteral("user"), job.user);
            entry.insert(QStringLiteral("size"), static_cast<double>(job.size));
            jobArray.append(entry);
        }
        QJsonObject entry;
        entry.insert(QStringLiteral("name"), printer.name);
        entry.insert(QStringLiteral("displayName"),
                     printer.displayName.isEmpty() ? printer.name : printer.displayName);
        entry.insert(QStringLiteral("makeAndModel"), printer.makeAndModel);
        entry.insert(QStringLiteral("location"), printer.location);
        entry.insert(QStringLiteral("uri"), printer.uri);
        entry.insert(QStringLiteral("state"), printer.state);
        entry.insert(QStringLiteral("stateLabel"), stateLabel(printer.state));
        entry.insert(QStringLiteral("stateMessage"), printer.stateMessage);
        entry.insert(QStringLiteral("acceptingJobs"), printer.acceptingJobs);
        entry.insert(QStringLiteral("enabled"), printer.enabled);
        entry.insert(QStringLiteral("isDefault"), printer.isDefault);
        entry.insert(QStringLiteral("jobCount"), jobArray.size());
        entry.insert(QStringLiteral("jobs"), jobArray);
        printerArray.append(entry);
    }

    QJsonArray scannerArray;
    for (const Scanner &scanner : scanners) {
        QJsonObject entry;
        entry.insert(QStringLiteral("device"), scanner.device);
        entry.insert(QStringLiteral("description"), scanner.description);
        entry.insert(QStringLiteral("displayName"),
                     scanner.description.isEmpty() ? scanner.device : scanner.description);
        entry.insert(QStringLiteral("kind"), scanner.kind);
        entry.insert(QStringLiteral("kindLabel"), kindLabel(scanner.kind));
        scannerArray.append(entry);
    }

    QString defaultPrinter;
    for (const Printer &printer : printers) {
        if (printer.isDefault) {
            defaultPrinter = printer.name;
            break;
        }
    }

    const int printerCount = printerArray.size();
    const int scannerCount = scannerArray.size();
    QString label;
    if (printerCount == 0 && scannerCount == 0)
        label = QStringLiteral("No Printers or Scanners");
    else if (printerCount == 0)
        label = scannerCount == 1 ? QStringLiteral("1 Scanner")
                                  : QStringLiteral("%1 Scanners").arg(scannerCount);
    else if (scannerCount == 0)
        label = printerCount == 1 ? QStringLiteral("1 Printer")
                                  : QStringLiteral("%1 Printers").arg(printerCount);
    else
        label = QStringLiteral("%1 %2, %3 %4")
                    .arg(printerCount)
                    .arg(printerCount == 1 ? QStringLiteral("Printer")
                                           : QStringLiteral("Printers"))
                    .arg(scannerCount)
                    .arg(scannerCount == 1 ? QStringLiteral("Scanner")
                                           : QStringLiteral("Scanners"));

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("printers"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("printer"));
    view.insert(QStringLiteral("label"), label);
    view.insert(QStringLiteral("present"), printerCount > 0 || scannerCount > 0);
    view.insert(QStringLiteral("printersAvailable"), m_cupsAvailable);
    view.insert(QStringLiteral("scannersAvailable"), m_saneAvailable);
    view.insert(QStringLiteral("printerCount"), printerCount);
    view.insert(QStringLiteral("scannerCount"), scannerCount);
    view.insert(QStringLiteral("queuedJobCount"), queuedJobs);
    view.insert(QStringLiteral("defaultPrinter"), defaultPrinter);
    view.insert(QStringLiteral("printers"), printerArray.toVariantList());
    view.insert(QStringLiteral("scanners"), scannerArray.toVariantList());
    m_view = view;
    emit changed(m_view);
}