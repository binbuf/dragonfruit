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
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>

class ChooserBridge : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool active READ active NOTIFY changed)
    Q_PROPERTY(QString handle READ handle NOTIFY changed)
    Q_PROPERTY(QString kind READ kind NOTIFY changed)
    Q_PROPERTY(QString title READ title NOTIFY changed)
    Q_PROPERTY(QString acceptLabel READ acceptLabel NOTIFY changed)
    Q_PROPERTY(QString currentUri READ currentUri NOTIFY changed)
    Q_PROPERTY(QString currentLabel READ currentLabel NOTIFY changed)
    Q_PROPERTY(QVariantList entries READ entries NOTIFY changed)
    Q_PROPERTY(int selectedIndex READ selectedIndex WRITE setSelectedIndex NOTIFY changed)
    Q_PROPERTY(QString saveName READ saveName WRITE setSaveName NOTIFY changed)
    Q_PROPERTY(bool canGoUp READ canGoUp NOTIFY changed)
    Q_PROPERTY(bool directoryMode READ directoryMode NOTIFY changed)
    Q_PROPERTY(bool multiple READ multiple NOTIFY changed)
    Q_PROPERTY(bool saveMode READ saveMode NOTIFY changed)
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
    QString title() const { return m_title; }
    QString acceptLabel() const;
    QString currentUri() const { return m_currentUri; }
    QString currentLabel() const;
    QVariantList entries() const { return m_entries; }
    int selectedIndex() const { return m_selectedIndex; }
    void setSelectedIndex(int index);
    QString saveName() const { return m_saveName; }
    void setSaveName(const QString &name);
    bool canGoUp() const;
    bool directoryMode() const { return m_directory; }
    bool multiple() const { return m_multiple; }
    bool saveMode() const;
    QString error() const { return m_error; }

public slots:
    // Present one request. `kind` is `open`/`save`/`save-files`; `startUri` is
    // the folder to browse first (empty falls back to the home folder).
    void begin(const QString &handle, const QString &kind, const QString &title,
               const QString &acceptLabel, bool multiple, bool directory,
               const QString &startUri);
    // Re-list the current folder.
    void refresh();
    // Browse an explicit folder (a breadcrumb jump).
    void browse(const QString &uri);
    // Row activation: a folder navigates into it, a file accepts (open) or
    // fills the save name (save).
    void activate(int index);
    // Single-click selection.
    void select(int index);
    // Navigate to the parent folder.
    void goUp();
    // Return the current selection to the portal.
    void accept();
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

private:
    // Clear every per-request field.
    void reset();
    // List `uri` through files-core and publish the rows.
    void list(const QString &uri);
    // Re-evaluate the portal service presence.
    void checkService();
    // Deliver a success response and close the request.
    void completeWith(const QStringList &selections);
    // The entry at `index`, or an empty map.
    QVariantMap entryAt(int index) const;
    // Resolve a request's suggested folder from the raw options.
    QString suggestedUri(const QVariantMap &options) const;
    // Local absolute path of the current folder (for SaveFile joins).
    QString currentDirectoryPath() const;

    bool m_active = false;
    QString m_handle;
    QString m_kind;
    QString m_title;
    QString m_acceptLabel;
    QString m_appId;
    bool m_multiple = false;
    bool m_directory = false;
    QString m_startUri;
    QString m_currentUri;
    QVariantList m_entries;
    int m_selectedIndex = -1;
    QString m_saveName;
    QString m_error;
    bool m_serviceAvailable = false;
    bool m_connected = false;
};