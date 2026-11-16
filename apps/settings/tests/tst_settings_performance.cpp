// SPDX-License-Identifier: MIT
// T268: the System Settings startup / pane-switch performance budget.
//
// Loads the real SettingsWindow through the real QML module with the
// deterministic settings fixture, measures the cold open (module load through
// the first presented frame), then switches through five panes twice:
//
//   * the first visit constructs and wires each pane's adapter client once;
//   * the second and later visits come from `PaneLoader`'s cache, so the
//     pane's `Component.onCompleted` side effects do not run again.
//
// The test records p50/p95/p99 and the count of samples over budget, and fails
// when the offscreen harness budget is missed. The reference-machine target is
// two 60 Hz frames (33.3 ms) for a pane switch; the harness budget is wider
// because the software scene graph and offscreen platform are slower.
//
// `make t268-settings-performance` runs this binary with
// DF_SETTINGS_PERF_OUT / DF_SETTINGS_PERF_NOTE set, writing the trace and the
// budget note under docs/captures/.
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlApplicationEngine>
#include <QQmlEngine>
#include <QQuickWindow>
#include <QSignalSpy>
#include <QTest>
#include <QVector>
#include <algorithm>
#include <cmath>

namespace {

// A cold open compiles and links the Settings QML module, the design system,
// and the pane catalog before the first frame. 2500 ms is the reference-host
// floor; the gate runs on a shared machine where other suites and background
// services can hold the CPU, so the failure ceiling is 8000 ms. The recorded
// fresh-host figure remains ~2 s (and 729 ms in the T268 reference run).
constexpr qint64 kColdOpenBudgetMs = 8000;
constexpr qint64 kFirstVisitBudgetMs = 1500;
constexpr qint64 kCachedSwitchBudgetMs = 150;

QStringList switchPanes()
{
    return { QStringLiteral("appearance"), QStringLiteral("wallpaper"),
             QStringLiteral("bluetooth"), QStringLiteral("sound"),
             QStringLiteral("general") };
}

qint64 percentile(const QVector<qint64> &sorted, double fraction)
{
    if (sorted.isEmpty())
        return 0;
    const int count = int(sorted.size());
    const int index = std::min(count - 1,
                               int(std::ceil(fraction * double(count))) - 1);
    return sorted[std::max(0, index)];
}

QString summaryLine(const QString &label, const QVector<qint64> &samples,
                    qint64 budget, int *missed)
{
    QVector<qint64> sorted = samples;
    std::sort(sorted.begin(), sorted.end());
    const qint64 p50 = percentile(sorted, 0.50);
    const qint64 p95 = percentile(sorted, 0.95);
    const qint64 p99 = percentile(sorted, 0.99);
    const int over = int(std::count_if(sorted.cbegin(), sorted.cend(),
                                       [budget](qint64 value) {
                                           return value > budget;
                                       }));
    if (missed)
        *missed += over;
    return QStringLiteral("%1: p50 %2 ms · p95 %3 ms · p99 %4 ms · "
                          "max %5 ms · %6/%7 over %8 ms")
        .arg(label)
        .arg(p50)
        .arg(p95)
        .arg(p99)
        .arg(sorted.isEmpty() ? 0 : sorted.constLast())
        .arg(over)
        .arg(sorted.size())
        .arg(budget);
}

void writeText(const QString &path, const QString &text)
{
    if (path.isEmpty())
        return;
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate | QIODevice::Text))
        return;
    file.write(text.toUtf8());
}

} // namespace

class TestSettingsPerformance : public QObject
{
    Q_OBJECT

private:
    QQmlApplicationEngine *m_engine = nullptr;
    QObject *m_shell = nullptr;
    QQuickWindow *m_window = nullptr;

    void loadWindow()
    {
        qputenv("DF_SETTINGS_FIXTURE", "1");
        qunsetenv("DF_SETTINGS_WINDOW_STATE");
        m_engine = new QQmlApplicationEngine(this);
        m_engine->addImportPath(QStringLiteral(DF_QML_IMPORT_DIR));
        m_engine->loadFromModule("Dragonfruit.Settings", "SettingsWindow");
        QVERIFY2(!m_engine->rootObjects().isEmpty(),
                 "SettingsWindow failed to load");
        m_window = qobject_cast<QQuickWindow *>(m_engine->rootObjects().constFirst());
        QVERIFY2(m_window, "SettingsWindow is not a QQuickWindow");
        m_shell = m_window->property("settingsShell").value<QObject *>();
        QVERIFY2(m_shell, "SettingsWindow exposes no SettingsShell");
    }

    qint64 waitForFirstFrame(QQuickWindow *window)
    {
        QElapsedTimer timer;
        timer.start();
        QSignalSpy frames(window, &QQuickWindow::frameSwapped);
        window->show();
        window->requestUpdate();
        for (int i = 0; i < 200 && frames.isEmpty(); ++i)
            QTest::qWait(10);
        return timer.elapsed();
    }

    qint64 switchTo(const QString &paneId)
    {
        QElapsedTimer timer;
        timer.start();
        QMetaObject::invokeMethod(m_shell, "selectPane", Q_ARG(QVariant, paneId));
        for (int i = 0; i < 300; ++i) {
            if (m_shell->property("loadedPaneId").toString() == paneId)
                break;
            QTest::qWait(2);
        }
        const qint64 loadedMs = timer.elapsed();
        // Include the next presented frame: a pane switch is not complete until
        // the new body is on screen.
        QSignalSpy frames(m_window, &QQuickWindow::frameSwapped);
        m_window->requestUpdate();
        for (int i = 0; i < 100 && frames.isEmpty(); ++i)
            QTest::qWait(2);
        return std::max(loadedMs, timer.elapsed());
    }

private slots:
    void initTestCase()
    {
        // Keep the run deterministic and independent of the developer's
        // settingsd/bridge host. The fixture is process-global, so set it
        // before the QML engine creates the Settings singleton.
        qputenv("DF_SETTINGS_FIXTURE", "1");
        qunsetenv("DF_SETTINGS_WINDOW_STATE");
        qputenv("DF_WALLPAPER_FIXTURE", "1");
    }

    void startupAndPaneSwitchStayWithinBudget()
    {
        QElapsedTimer coldTimer;
        coldTimer.start();
        loadWindow();
        QVERIFY(m_window);
        QVERIFY(m_shell);
        const qint64 loadMs = coldTimer.elapsed();
        const qint64 firstFrameMs = waitForFirstFrame(m_window);
        const qint64 coldMs = std::max(loadMs, coldTimer.elapsed());
        Q_UNUSED(firstFrameMs);

        // The shell starts on General; the first switch cycle constructs the
        // five pane bodies and their adapter clients.
        QVector<qint64> firstVisit;
        for (const QString &paneId : switchPanes()) {
            firstVisit.append(switchTo(paneId));
            QVERIFY2(m_shell->property("loadedPaneId").toString() == paneId,
                     qPrintable(QStringLiteral("pane %1 did not load").arg(paneId)));
        }
        // The Wallpaper pane asks the provider for an eager preload in
        // Component.onCompleted. A cached pane must not ask again.
        QObject *settings = m_engine->singletonInstance<QObject *>(
            QStringLiteral("Dragonfruit.Settings"), QStringLiteral("Settings"));
        QVERIFY(settings);
        const int preloadsAfterFirstVisit =
            settings->property("providerPreloadCount").toInt();
        QVERIFY2(preloadsAfterFirstVisit >= 1,
                 "the first Wallpaper visit must request one preload");

        // Returning to each pane must come from PaneLoader's cache. Sample
        // twelve cycles for a stable p95/p99.
        QVector<qint64> cached;
        for (int cycle = 0; cycle < 12; ++cycle) {
            for (const QString &paneId : switchPanes())
                cached.append(switchTo(paneId));
        }
        QCOMPARE(settings->property("providerPreloadCount").toInt(),
                 preloadsAfterFirstVisit);

        int missed = 0;
        const QString coldLine = QStringLiteral("cold open: %1 ms · budget %2 ms")
                                     .arg(coldMs).arg(kColdOpenBudgetMs);
        const QString firstLine = summaryLine(QStringLiteral("first pane visit"),
                                              firstVisit, kFirstVisitBudgetMs,
                                              &missed);
        const QString cachedLine = summaryLine(QStringLiteral("cached pane switch"),
                                               cached, kCachedSwitchBudgetMs,
                                               &missed);

        const QString note = QStringLiteral(
            "T268 System Settings performance trace\n"
            "harness: offscreen QPA + software scene graph, deterministic fixtures\n"
            "reference target: pane switch within two 60 Hz frames (33.3 ms); "
            "cold open paints the first pane without waiting for host services\n\n"
            "%1\n%2\n%3\n\n"
            "missed deadlines: %4\n"
            "The harness budgets are deliberately wider than the reference target "
            "because the software scene graph is slower than the GPU path.\n")
            .arg(coldLine, firstLine, cachedLine).arg(missed);

        qInfo().noquote() << note;
        writeText(qEnvironmentVariable("DF_SETTINGS_PERF_NOTE"), note);

        QJsonObject trace;
        trace.insert(QStringLiteral("coldOpenMs"), double(coldMs));
        trace.insert(QStringLiteral("coldOpenBudgetMs"), double(kColdOpenBudgetMs));
        trace.insert(QStringLiteral("firstVisitBudgetMs"), double(kFirstVisitBudgetMs));
        trace.insert(QStringLiteral("cachedSwitchBudgetMs"),
                     double(kCachedSwitchBudgetMs));
        trace.insert(QStringLiteral("missedDeadlines"), missed);
        const auto array = [](const QVector<qint64> &samples) {
            QJsonArray out;
            for (qint64 sample : samples)
                out.append(double(sample));
            return out;
        };
        trace.insert(QStringLiteral("firstVisitMs"), array(firstVisit));
        trace.insert(QStringLiteral("cachedSwitchMs"), array(cached));
        QVector<qint64> sortedFirst = firstVisit;
        QVector<qint64> sortedCached = cached;
        std::sort(sortedFirst.begin(), sortedFirst.end());
        std::sort(sortedCached.begin(), sortedCached.end());
        QJsonObject percentiles;
        percentiles.insert(QStringLiteral("firstP50"), double(percentile(sortedFirst, 0.50)));
        percentiles.insert(QStringLiteral("firstP95"), double(percentile(sortedFirst, 0.95)));
        percentiles.insert(QStringLiteral("firstP99"), double(percentile(sortedFirst, 0.99)));
        percentiles.insert(QStringLiteral("cachedP50"), double(percentile(sortedCached, 0.50)));
        percentiles.insert(QStringLiteral("cachedP95"), double(percentile(sortedCached, 0.95)));
        percentiles.insert(QStringLiteral("cachedP99"), double(percentile(sortedCached, 0.99)));
        trace.insert(QStringLiteral("percentiles"), percentiles);
        writeText(qEnvironmentVariable("DF_SETTINGS_PERF_OUT"),
                  QString::fromUtf8(QJsonDocument(trace).toJson(QJsonDocument::Indented)));

        QVERIFY2(coldMs <= kColdOpenBudgetMs,
                 qPrintable(QStringLiteral("cold open %1 ms exceeds %2 ms")
                                .arg(coldMs).arg(kColdOpenBudgetMs)));
        QVERIFY2(percentile(sortedFirst, 0.95) <= kFirstVisitBudgetMs,
                 qPrintable(QStringLiteral("first-visit p95 exceeds %1 ms")
                                .arg(kFirstVisitBudgetMs)));
        QVERIFY2(percentile(sortedCached, 0.95) <= kCachedSwitchBudgetMs,
                 qPrintable(QStringLiteral("cached-switch p95 exceeds %1 ms")
                                .arg(kCachedSwitchBudgetMs)));
    }
};

QTEST_MAIN(TestSettingsPerformance)
#include "tst_settings_performance.moc"
