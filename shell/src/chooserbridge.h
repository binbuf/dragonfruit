// SPDX-License-Identifier: MIT
// The shell's half of the FileChooser presenter seam (T-13.2b).
//
// `ChooserBridge` watches the portal backend's diagnostic
// `org.dragonfruit.Portal1.FileChooserOpened` signal, browses the request's
// folder through the one `files-core` implementation (the listing slice of the
// C ABI, exactly as Files and the portal do), and answers the waiting request
// with `CompleteFileChooser` / `CancelFileChooser`. The portal's method is
// synchronous and the registry (T-13.2a) is the one seam, so the picker only
// has to resolve the handle it was handed.
//
// The dialog itself is pure QML (`FileChooser.qml`); this class owns the state
// the view binds to and the one D-Bus call that returns the selection to the
// portal. `begin()` is the shared entry point for the live signal and the
// headless/capture fixture (`DF_CHOOSER_FIXTURE`), so the view path is
// identical in both.
//
// The four dialog kinds the portal can ask for are all served here:
//
//   * `open`       — pick one or more existing files; `directory` turns it
//                    into a folder picker.
//   * `save`       — pick a folder and a file name (with an overwrite
//                    confirmation), returning the full `file://` URI.
//   * `save-files` — pick a folder; the portal appends the caller's file
//                    names and returns one URI per name.
//
// Requests are queued: a second application asking while a dialog is open is
// shown as soon as the first resolves instead of being dropped.
//
// A live portal handle is never dropped silently (T311): Cancel answers
// `CancelFileChooser`, and startup cancels whatever `PendingFileChoosers` the
// backend still holds. Re-presenting a stale chooser from only its handle and
// kind would lose the caller's options (notably SaveFiles' file names), so a
// stale request is cancelled rather than shown with wrong behavior.
#pragma once

#include "files_core_list.h"

#include <QList>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>

class QDBusPendingCallWatcher;
class QTimer;

class ChooserBridge : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool active READ active NOTIFY changed)
    Q_PROPERTY(QString handle READ handle NOTIFY changed)
    Q_PROPERTY(QString kind READ kind NOTIFY changed)
    Q_PROPERTY(QString appId READ appId NOTIFY changed)
    Q_PROPERTY(QString title READ title NOTIFY changed)
    Q_PROPERTY(QString acceptLabel READ acceptLabel NOTIFY changed)
    Q_PROPERTY(QString currentUri READ currentUri NOTIFY changed)
    Q_PROPERTY(QString currentLabel READ currentLabel NOTIFY changed)
    Q_PROPERTY(QVariantList breadcrumbs READ breadcrumbs NOTIFY changed)
    Q_PROPERTY(QVariantList favorites READ favorites NOTIFY changed)
    Q_PROPERTY(QVariantList entries READ entries NOTIFY changed)
    Q_PROPERTY(int entriesRevision READ entriesRevision NOTIFY changed)
    Q_PROPERTY(bool loading READ loading NOTIFY changed)
    Q_PROPERTY(int selectedIndex READ selectedIndex NOTIFY changed)
    Q_PROPERTY(QVariantList selectedIndices READ selectedIndices NOTIFY changed)
    Q_PROPERTY(QString saveName READ saveName WRITE setSaveName NOTIFY changed)
    Q_PROPERTY(QStringList saveFiles READ saveFiles NOTIFY changed)
    Q_PROPERTY(bool canGoUp READ canGoUp NOTIFY changed)
    Q_PROPERTY(bool directoryMode READ directoryMode NOTIFY changed)
    Q_PROPERTY(bool multiple READ multiple NOTIFY changed)
    Q_PROPERTY(bool saveMode READ saveMode NOTIFY changed)
    Q_PROPERTY(bool saveFilesMode READ saveFilesMode NOTIFY changed)
    Q_PROPERTY(bool confirmReplace READ confirmReplace NOTIFY changed)
    Q_PROPERTY(QString replaceName READ replaceName NOTIFY changed)
    Q_PROPERTY(QString error READ error NOTIFY changed)

public:
    explicit ChooserBridge(QObject *parent = nullptr);
    ~ChooserBridge() override;

    // Subscribe to the portal's presenter signal. A missing portal is a normal
    // state: the picker simply never opens. Reconnect-safe.
    void connectService();
    bool serviceAvailable() const { return m_serviceAvailable; }

    bool active() const { return m_active; }
    QString handle() const { return m_handle; }
    QString kind() const { return m_kind; }
    QString appId() const { return m_appId; }
    QString title() const { return m_title; }
    QString acceptLabel() const;
    QString currentUri() const { return m_currentUri; }
    QString currentLabel() const;
    QVariantList breadcrumbs() const;
    QVariantList favorites() const { return m_favorites; }
    QVariantList entries() const { return m_entries; }
    int entriesRevision() const { return m_entriesRevision; }
    bool loading() const { return m_loading; }
    int selectedIndex() const { return m_current; }
    QVariantList selectedIndices() const;
    QString saveName() const { return m_saveName; }
    void setSaveName(const QString &name);
    QStringList saveFiles() const { return m_saveFiles; }
    bool canGoUp() const;
    bool directoryMode() const { return m_directory; }
    bool multiple() const { return m_multiple; }
    bool saveMode() const;
    bool saveFilesMode() const;
    bool confirmReplace() const { return m_confirmReplace; }
    QString replaceName() const { return m_replaceName; }
    QString error() const { return m_error; }

public slots:
    // Present one request. `kind` is `open`/`save`/`save-files`; `startUri` is
    // the folder to browse first (empty falls back to the home folder).
    // Requests queue behind a live dialog rather than replacing it.
    void begin(const QString &handle, const QString &kind, const QString &title,
               const QString &acceptLabel, bool multiple, bool directory,
               const QString &startUri);
    // Re-list the current folder.
    void refresh();
    // Browse an explicit folder (a sidebar row or breadcrumb jump).
    void browse(const QString &uri);
    // Row activation: a folder navigates into it, a file accepts (open) or
    // fills the save name (save).
    void activate(int index);
    // Single-click selection. `modifiers` is Qt's keyboard modifier mask:
    // Control/Meta toggles, Shift extends from the anchor.
    void select(int index, int modifiers);
    // Select every row (Ctrl/Cmd+A in a multiple-selection request).
    void selectAll();
    // Navigate to the parent folder.
    void goUp();
    // Create `untitled folder` in the current folder (the save dialog's New
    // Folder affordance; available in every kind).
    void newFolder();
    // Return the current selection to the portal.
    void accept();
    // Confirm replacing an existing file (the second half of a save).
    void acceptReplace();
    // Dismiss the overwrite confirmation and return to the dialog.
    void dismissReplace();
    // Cancel the request.
    void cancel();

signals:
    void changed();
    // A request opened; the shell maps the picker surface.
    void started();
    // The request resolved and the shell unmaps. `completed` is true when the
    // portal was handed a selection, false on cancellation.
    void finished(bool completed);

private slots:
    // The portal backend's presenter signal.
    void onFileChooserOpened(const QString &handle, const QString &kind, const QString &appId,
                             const QString &parentWindow, const QString &title,
                             const QVariantMap &options);
    // Drain ready listing deltas without blocking the shell's event loop.
    void onListTick();
    // The startup `PendingFileChoosers` reply: cancel every stale handle.
    void onPendingReply(QDBusPendingCallWatcher *watcher);

private:
    // One waiting request, decoded from the portal's options.
    struct Request {
        QString handle;
        QString kind;
        QString appId;
        QString title;
        QString acceptLabel;
        bool multiple = false;
        bool directory = false;
        QString startUri;
        QString suggestedName;
        QStringList saveFiles;
        // True when the request came from the portal (not the fixture).
        bool fromPortal = false;
    };

    // Answer `CancelFileChooser` for the live portal handle, if any.
    void cancelCurrent();
    // Ask the backend for the requests a previous shell left pending.
    void queryPending();
    // Clear every per-request field and retire the listing session. Callers
    // must answer a live portal handle first (`cancelCurrent()`/`completeWith`).
    void reset();
    // Show the next queued request, if the dialog is idle.
    void startNext();
    // Finish the live request, unmap the view, and start the next one.
    void finish(bool completed);
    // List `uri` through files-core without blocking the UI thread.
    void list(const QString &uri);
    // Stop the listing timer and free the files-core session.
    void stopListing();
    // Apply one incremental delta to `m_entries`.
    void applyDelta(const df_files_delta *delta);
    // Re-evaluate the portal service presence and query pending requests when
    // the service (re)appears.
    void checkService();
    // Deliver a success response and close the request.
    void completeWith(const QStringList &selections);
    // The entry at `index`, or an empty map.
    QVariantMap entryAt(int index) const;
    // Resolve a request's suggested folder from the raw options.
    QString suggestedUri(const QVariantMap &options) const;
    // The suggested file name from `current_name`/`current_file`.
    QString suggestedName(const QVariantMap &options) const;
    // Decode the `files` names of a SaveFiles request.
    QStringList saveFilesFrom(const QVariantMap &options) const;
    // Local absolute path of the current folder (for SaveFile joins).
    QString currentDirectoryPath() const;
    // Resolve the typed save name to an absolute local path (empty on error).
    QString resolveSavePath();
    // Recompute the click/keyboard selection.
    void setSelection(int index, int modifiers, bool replaceAlways);
    // The selected rows, in display order.
    QList<int> selectedRows() const;
    // Publish the overwrite confirmation for `path`.
    void requestReplace(const QString &path);

    bool m_active = false;
    bool m_fromPortal = false;
    QString m_handle;
    QString m_kind;
    QString m_appId;
    QString m_title;
    QString m_acceptLabel;
    bool m_multiple = false;
    bool m_directory = false;
    QString m_currentUri;
    QVariantList m_entries;
    int m_entriesRevision = 0;
    bool m_loading = false;
    QList<int> m_selected;
    int m_current = -1;
    int m_anchor = -1;
    QString m_saveName;
    QString m_suggestedName;
    QStringList m_saveFiles;
    QString m_error;
    bool m_confirmReplace = false;
    QString m_replaceName;
    QString m_replacePath;
    QVariantList m_favorites;

    QList<Request> m_queue;
    QTimer *m_listTimer = nullptr;
    void *m_session = nullptr;
    bool m_listingComplete = false;

    bool m_serviceAvailable = false;
    bool m_connected = false;
};
