// SPDX-License-Identifier: MIT
#include "trashbridge.h"

#include "files_core_trash.h"

#include <QMetaObject>
#include <QUrl>

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
    const int removed = df_files_trash_monitor_empty(m_monitor);
    if (removed < 0)
        takeError();
    applyState();
    return removed;
}

bool TrashBridge::emptyAsync()
{
    // One operation at a time (T-14.7r): a second Empty Trash while emptying
    // is ignored.
    if (m_emptyState == EmptyState::Emptying)
        return false;

    resetEmptyState();
    m_emptyState = EmptyState::Emptying;
    emit emptyStarted();

    if (!m_monitor) {
        // The backend is unreachable (service down). Report a failure
        // immediately on the UI thread instead of hanging.
        m_emptyState = EmptyState::Failed;
        m_emptyError = QStringLiteral("no trash monitor");
        m_error = m_emptyError;
        emit emptyFinished(false, -1, m_emptyError);
        return true;
    }

    joinEmptyWorker();
    m_emptyWorker = std::thread([this]() {
        const int removed = df_files_trash_monitor_empty(m_monitor);
        QMetaObject::invokeMethod(
            this, [this, removed]() { finishEmpty(removed); },
            Qt::QueuedConnection);
    });
    return true;
}

void TrashBridge::resetEmptyState()
{
    m_emptyState = EmptyState::Idle;
    m_emptyRemoved = 0;
    m_emptyError.clear();
}

void TrashBridge::finishEmpty(int removed)
{
    if (removed >= 0) {
        applyState();
        m_emptyState = EmptyState::Succeeded;
        m_emptyRemoved = removed;
        m_emptyError.clear();
        emit emptyFinished(true, removed, QString());
        return;
    }
    takeError();
    m_emptyState = EmptyState::Failed;
    m_emptyRemoved = 0;
    m_emptyError = m_error.isEmpty() ? QStringLiteral("failed to empty the trash")
                                     : m_error;
    emit emptyFinished(false, -1, m_emptyError);
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
        if (df_files_trash_monitor_trash(m_monitor, uri.constData()) == 1)
            ++trashed;
        else
            takeError();
    }
    applyState();
    return trashed;
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
    char *message = df_files_trash_monitor_take_error(m_monitor);
    if (!message)
        return;
    m_error = QString::fromUtf8(message);
    df_files_string_free(message);
}