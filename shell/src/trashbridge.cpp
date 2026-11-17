// SPDX-License-Identifier: MIT
#include "trashbridge.h"

#include "files_core_trash.h"

#include <QMetaObject>
#include <QUrl>
#include <QVariantMap>

namespace {

// How long the watch worker blocks before re-checking cancellation. An idle
// Trash wakes only to check this flag; a change arrives immediately.
constexpr uint32_t kWaitMillis = 250;

} // namespace

TrashBridge::TrashBridge(QObject *parent)
    : QObject(parent)
{
}

TrashBridge::~TrashBridge()
{
    stop();
    joinEmptyWorker();
    joinTrashWorker();
    if (m_monitor) {
        df_files_trash_monitor_free(m_monitor);
        m_monitor = nullptr;
    }
}

void TrashBridge::start()
{
    if (m_monitor)
        return;
    m_monitor = df_files_trash_monitor_new();
    m_watching = m_monitor != nullptr;
    applyState(true);
    if (!m_monitor)
        return;

    m_stop.store(false);
    m_worker = std::thread([this]() {
        while (!m_stop.load()) {
            if (df_files_trash_monitor_wait(m_monitor, kWaitMillis) == 1) {
                QMetaObject::invokeMethod(
                    this, [this]() { applyState(); }, Qt::QueuedConnection);
            }
        }
    });
}

void TrashBridge::stop()
{
    if (!m_watching)
        return;
    m_stop.store(true);
    if (m_worker.joinable())
        m_worker.join();
    m_watching = false;
}

void TrashBridge::refresh()
{
    if (m_monitor)
        df_files_trash_monitor_refresh(m_monitor);
    applyState();
}

int TrashBridge::empty()
{
    if (!m_monitor) {
        m_error = QStringLiteral("no trash monitor");
        return -1;
    }
    int removed = -1;
    {
        std::lock_guard<std::mutex> lock(m_mutationMutex);
        removed = df_files_trash_monitor_empty(m_monitor);
    }
    if (removed < 0)
        takeError();
    applyState();
    return removed;
}

bool TrashBridge::emptyAsync(quint64 requestId)
{
    // One operation at a time (T-14.7r): a second Empty Trash while emptying
    // is ignored.
    if (m_emptyState == EmptyState::Emptying)
        return false;

    const quint64 resolvedId =
        requestId != 0 ? requestId : m_nextEmptyRequestId++;

    resetEmptyState();
    m_emptyState = EmptyState::Emptying;
    m_emptyRequestId = resolvedId;
    emit emptyStarted();

    if (!m_monitor) {
        // The backend is unreachable (service down). Report a failure
        // immediately on the UI thread instead of hanging.
        m_emptyState = EmptyState::Failed;
        m_emptyError = QStringLiteral("no trash monitor");
        m_error = m_emptyError;
        emit emptyFinished(false, -1, m_emptyError, resolvedId);
        return true;
    }

    joinEmptyWorker();
    m_emptyWorker = std::thread([this, resolvedId]() {
        int removed = -1;
        {
            std::lock_guard<std::mutex> lock(m_mutationMutex);
            removed = df_files_trash_monitor_empty(m_monitor);
        }
        QMetaObject::invokeMethod(
            this, [this, removed, resolvedId]() {
                finishEmpty(removed, resolvedId);
            },
            Qt::QueuedConnection);
    });
    return true;
}

void TrashBridge::resetEmptyState()
{
    m_emptyState = EmptyState::Idle;
    m_emptyRemoved = 0;
    m_emptyError.clear();
    m_emptyRequestId = 0;
}

void TrashBridge::finishEmpty(int removed, quint64 requestId)
{
    // A result from a superseded operation must not overwrite the current one.
    if (m_emptyState != EmptyState::Emptying || requestId != m_emptyRequestId)
        return;
    if (removed >= 0) {
        applyState();
        m_emptyState = EmptyState::Succeeded;
        m_emptyRemoved = removed;
        m_emptyError.clear();
        emit emptyFinished(true, removed, QString(), requestId);
        return;
    }
    takeError();
    m_emptyState = EmptyState::Failed;
    m_emptyRemoved = 0;
    m_emptyError = m_error.isEmpty() ? QStringLiteral("failed to empty the trash")
                                     : m_error;
    emit emptyFinished(false, -1, m_emptyError, requestId);
}

void TrashBridge::joinEmptyWorker()
{
    if (m_emptyWorker.joinable())
        m_emptyWorker.join();
}

int TrashBridge::trash(const QStringList &paths)
{
    if (!m_monitor) {
        m_error = QStringLiteral("no trash monitor");
        return 0;
    }
    m_error.clear();
    int trashed = 0;
    for (const QString &path : paths) {
        if (path.isEmpty())
            continue;
        const QByteArray uri =
            QUrl::fromLocalFile(path).toString(QUrl::FullyEncoded).toUtf8();
        int result = 0;
        {
            std::lock_guard<std::mutex> lock(m_mutationMutex);
            result = df_files_trash_monitor_trash(m_monitor, uri.constData());
        }
        if (result == 1)
            ++trashed;
        else
            takeError();
    }
    applyState();
    return trashed;
}

bool TrashBridge::trashAsync(const QStringList &paths, quint64 requestId)
{
    QStringList items;
    items.reserve(paths.size());
    for (const QString &path : paths) {
        if (!path.isEmpty())
            items.append(path);
    }
    if (items.isEmpty())
        return false;

    const quint64 resolvedId =
        requestId != 0 ? requestId : m_nextTrashRequestId++;
    if (requestId != 0 && requestId >= m_nextTrashRequestId)
        m_nextTrashRequestId = requestId + 1;

    if (!m_monitor) {
        // The backend is unreachable (service down). Report the failure on the
        // UI thread instead of leaving the caller waiting for a worker.
        QVariantList outcomes;
        outcomes.reserve(items.size());
        for (const QString &path : items) {
            QVariantMap outcome;
            outcome.insert(QStringLiteral("path"), path);
            outcome.insert(QStringLiteral("ok"), false);
            outcome.insert(QStringLiteral("error"), QStringLiteral("no trash monitor"));
            outcomes.append(outcome);
        }
        m_error = QStringLiteral("no trash monitor");
        m_lastTrashRequestId = resolvedId;
        recordTrashOutcomes(resolvedId, outcomes);
        emit trashStarted(resolvedId);
        emit trashFinished(false, 0, items.size(), m_error, outcomes, resolvedId);
        return true;
    }

    PendingTrash pending;
    pending.requestId = resolvedId;
    pending.paths = items;
    m_trashQueue.append(pending);
    pumpTrash();
    return true;
}

void TrashBridge::pumpTrash()
{
    if (m_trashRunning || m_trashQueue.isEmpty() || !m_monitor)
        return;

    const PendingTrash request = m_trashQueue.takeFirst();
    m_trashRunning = true;
    m_trashRequestId = request.requestId;
    emit trashStarted(request.requestId);

    joinTrashWorker();
    m_trashWorker = std::thread([this, request]() {
        QVariantList outcomes;
        outcomes.reserve(request.paths.size());
        int trashed = 0;
        QString lastError;
        for (const QString &path : request.paths) {
            const QByteArray uri =
                QUrl::fromLocalFile(path).toString(QUrl::FullyEncoded).toUtf8();
            int result = 0;
            QString itemError;
            {
                std::lock_guard<std::mutex> lock(m_mutationMutex);
                result = df_files_trash_monitor_trash(m_monitor, uri.constData());
                if (result != 1)
                    itemError = takeErrorText();
            }
            QVariantMap outcome;
            outcome.insert(QStringLiteral("path"), path);
            outcome.insert(QStringLiteral("ok"), result == 1);
            if (result == 1) {
                ++trashed;
            } else {
                if (!itemError.isEmpty()) {
                    outcome.insert(QStringLiteral("error"), itemError);
                    lastError = itemError;
                }
            }
            outcomes.append(outcome);
        }
        const int failed = request.paths.size() - trashed;
        QMetaObject::invokeMethod(
            this,
            [this, request, outcomes, trashed, failed, lastError]() {
                finishTrash(request.requestId, outcomes, trashed, failed,
                            lastError);
            },
            Qt::QueuedConnection);
    });
}

void TrashBridge::finishTrash(quint64 requestId, const QVariantList &outcomes,
                              int trashed, int failed, const QString &error)
{
    // A result for a superseded request (or a repeated completion) must not
    // overwrite the current operation or update the UI twice.
    if (!m_trashRunning || requestId != m_trashRequestId)
        return;

    applyState();
    m_trashRunning = false;
    m_trashRequestId = 0;
    m_lastTrashRequestId = requestId;
    recordTrashOutcomes(requestId, outcomes);

    const QString resultError =
        failed == 0 ? QString()
                    : error.isEmpty()
                        ? QStringLiteral("%1 item(s) could not be moved to the Trash")
                              .arg(failed)
                        : error;
    if (failed > 0)
        m_error = resultError;
    emit trashFinished(failed == 0, trashed, failed, resultError, outcomes,
                       requestId);
    pumpTrash();
}

void TrashBridge::recordTrashOutcomes(quint64 requestId,
                                      const QVariantList &outcomes)
{
    if (!m_trashHistory.contains(requestId))
        m_trashHistoryOrder.append(requestId);
    m_trashHistory.insert(requestId, outcomes);
    trimTrashHistory();
}

void TrashBridge::trimTrashHistory()
{
    constexpr int kRetainedTrashResults = 32;
    while (m_trashHistoryOrder.size() > kRetainedTrashResults)
        m_trashHistory.remove(m_trashHistoryOrder.takeFirst());
}

void TrashBridge::joinTrashWorker()
{
    if (m_trashWorker.joinable())
        m_trashWorker.join();
}

QVariantList TrashBridge::trashOutcomes(quint64 requestId) const
{
    return m_trashHistory.value(requestId);
}

void TrashBridge::applyState(bool force)
{
    if (!m_monitor)
        return;
    const df_files_trash_state state = df_files_trash_monitor_state(m_monitor);
    const int count = state.count > 0 ? state.count : 0;
    const bool available = state.available != 0;
    if (!force && count == m_count && available == m_available)
        return;
    m_count = count;
    m_available = available;
    emit changed();
}

void TrashBridge::takeError()
{
    const QString message = takeErrorText();
    if (!message.isEmpty())
        m_error = message;
}

QString TrashBridge::takeErrorText()
{
    char *message = df_files_trash_monitor_take_error(m_monitor);
    if (!message)
        return {};
    const QString text = QString::fromUtf8(message);
    df_files_string_free(message);
    return text;
}