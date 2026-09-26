// SPDX-License-Identifier: MIT
// Shell clipboard history (T-13.5b).
//
// The compositor's `wl_data_device` is the one clipboard owner; the shell is
// an *observer* over the `wlr-data-control` manager surface (ADR 0082). This
// class is the pure store the observer feeds: it classifies a selection
// (text / image / file list), deduplicates, caps the history by entry count
// and total bytes, honours pinned entries and the common password-manager
// secret hints, clears on lock, and serializes for best-effort persistence.
//
// It names no Wayland or D-Bus, so it is unit-testable with plain values.
#pragma once

#include <QByteArray>
#include <QMap>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>

class ClipboardHistory : public QObject
{
    Q_OBJECT

public:
    // A selection's payloads keyed by MIME type.
    using EntryMap = QMap<QString, QByteArray>;

    // The one classification of a stored selection.
    enum class Kind { Text, Image, Files, Other };

    explicit ClipboardHistory(QObject *parent = nullptr);

    // The history, most recent first. Each map has `kind`, `preview`,
    // `mimes`, `pinned`, `bytes` and `index`.
    QVariantList entries() const;
    int count() const { return m_entries.size(); }
    bool isEmpty() const { return m_entries.isEmpty(); }
    int totalBytes() const { return m_totalBytes; }

    // Record an observed selection. `payloads` maps the MIME types the
    // observer actually read to their bytes; `mimes` is the full offered set
    // (for classification). A sensitive or unusable selection is ignored.
    // Returns true when an entry was recorded.
    bool observe(const QStringList &mimes, const QMap<QString, QByteArray> &payloads);

    // The entries whose preview or MIME types contain `query` (case
    // insensitive), most recent first.
    QVariantList search(const QString &query) const;

    // The observed MIME types and payloads of an entry, for re-serving it to
    // the data-control device (forwarding, never a second owner).
    bool restore(int index, QStringList *mimes, QMap<QString, QByteArray> *payloads) const;

    bool setPinned(int index, bool pinned);
    bool remove(int index);

    // Drop every entry.
    void clear();
    // Drop only the unpinned entries (the clear-on-lock default).
    void clearUnpinned();

    // Whether a lock clears the history (default true). Pinned entries
    // survive.
    bool clearOnLock() const { return m_clearOnLock; }
    void setClearOnLock(bool enabled);

    // Best-effort persistence. `fromJson` is tolerant: a malformed or
    // oversized payload is dropped, never an error.
    QByteArray toJson() const;
    bool fromJson(const QByteArray &json);

    // The policy limits (public so tests and docs share one number).
    static constexpr int kMaxEntries = 50;
    static constexpr int kMaxEntryBytes = 1 << 20; // 1 MiB
    static constexpr int kMaxTotalBytes = 8 << 20; // 8 MiB
    static constexpr int kPreviewChars = 180;

    // Classify a selection by its offered MIME types. Public for tests.
    static Kind classify(const QStringList &mimes);
    static QString kindName(Kind kind);
    // Whether the selection carries a common "do not store" secret hint.
    static bool isSensitive(const QStringList &mimes,
                            const QMap<QString, QByteArray> &payloads);

signals:
    void changed();

private:
    struct Entry {
        QStringList mimes;
        QMap<QString, QByteArray> payloads;
        Kind kind = Kind::Other;
        QString preview;
        bool pinned = false;
        QString hash;
        int bytes = 0;
    };

    // Insert (or promote) a fully-formed entry, enforcing the caps.
    void addEntry(Entry entry);
    static QString computeHash(const EntryMap &payloads);
    static QString previewFor(Kind kind, const QStringList &mimes,
                              const QMap<QString, QByteArray> &payloads);
    static QVariantMap toVariant(const Entry &entry, int index);
    static int entryBytes(const QMap<QString, QByteArray> &payloads);
    void enforceCaps();

    QList<Entry> m_entries;
    int m_totalBytes = 0;
    bool m_clearOnLock = true;
};