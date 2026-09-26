// SPDX-License-Identifier: MIT
#include "clipboardhistory.h"

#include <QCryptographicHash>
#include <QFileInfo>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QUrl>

namespace {

bool isTextMime(const QString &mime)
{
    return mime == QLatin1String("text/plain;charset=utf-8")
        || mime == QLatin1String("text/plain")
        || mime == QLatin1String("UTF8_STRING")
        || mime == QLatin1String("TEXT");
}

bool isImageMime(const QString &mime)
{
    return mime.startsWith(QLatin1String("image/"));
}

bool isSupportedMime(const QString &mime)
{
    return isTextMime(mime) || isImageMime(mime)
        || mime == QLatin1String("text/uri-list");
}

QStringList parseUriList(const QByteArray &payload)
{
    QStringList uris;
    const QStringList lines =
        QString::fromUtf8(payload).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    for (const QString &raw : lines) {
        const QString line = raw.trimmed();
        if (line.isEmpty() || line.startsWith(QLatin1Char('#')))
            continue;
        uris.append(line);
    }
    return uris;
}

} // namespace

ClipboardHistory::ClipboardHistory(QObject *parent)
    : QObject(parent)
{
}

ClipboardHistory::Kind ClipboardHistory::classify(const QStringList &mimes)
{
    for (const QString &mime : mimes) {
        if (mime == QLatin1String("text/uri-list"))
            return Kind::Files;
    }
    for (const QString &mime : mimes) {
        if (isImageMime(mime))
            return Kind::Image;
    }
    for (const QString &mime : mimes) {
        if (isTextMime(mime))
            return Kind::Text;
    }
    return Kind::Other;
}

QString ClipboardHistory::kindName(Kind kind)
{
    switch (kind) {
    case Kind::Text:
        return QStringLiteral("text");
    case Kind::Image:
        return QStringLiteral("image");
    case Kind::Files:
        return QStringLiteral("files");
    case Kind::Other:
        break;
    }
    return QStringLiteral("other");
}

bool ClipboardHistory::isSensitive(const QStringList &mimes,
                                   const QMap<QString, QByteArray> &payloads)
{
    for (const QString &mime : mimes) {
        const QString lower = mime.toLower();
        if (lower.contains(QLatin1String("password"))
            || lower.contains(QLatin1String("secret"))) {
            // The password-manager convention marks a secret without naming
            // it in the type; a marker with no payload still forbids storing.
            if (lower == QLatin1String("x-kde-passwordmanagerhint")) { // df-allow-desktop-name
                const QByteArray value = payloads.value(mime, "secret");
                if (value.trimmed().toLower() != "onlymimes"
                    && !value.trimmed().isEmpty())
                    return true;
                continue;
            }
            return true;
        }
    }
    return false;
}

int ClipboardHistory::entryBytes(const QMap<QString, QByteArray> &payloads)
{
    int bytes = 0;
    for (auto it = payloads.constBegin(); it != payloads.constEnd(); ++it)
        bytes += it.value().size();
    return bytes;
}

QString ClipboardHistory::computeHash(const QMap<QString, QByteArray> &payloads)
{
    QCryptographicHash hash(QCryptographicHash::Sha1);
    QStringList sorted = payloads.keys();
    sorted.sort();
    for (const QString &mime : sorted) {
        hash.addData(mime.toUtf8());
        hash.addData(QByteArray(1, '\0'));
        const QByteArray payload = payloads.value(mime);
        hash.addData(QByteArray::number(payload.size()));
        hash.addData(QByteArray(1, '\0'));
        hash.addData(payload);
    }
    return QString::fromLatin1(hash.result().toHex());
}

QString ClipboardHistory::previewFor(Kind kind, const QStringList &mimes,
                                     const QMap<QString, QByteArray> &payloads)
{
    Q_UNUSED(mimes);
    if (kind == Kind::Text) {
        for (const QString &mime : payloads.keys()) {
            if (!isTextMime(mime))
                continue;
            const QString text = QString::fromUtf8(payloads.value(mime)).simplified();
            if (!text.isEmpty())
                return text.left(kPreviewChars);
        }
        return QStringLiteral("(empty text)");
    }
    if (kind == Kind::Files) {
        const QStringList uris = parseUriList(payloads.value(QStringLiteral("text/uri-list")));
        QStringList names;
        for (const QString &uri : uris) {
            const QString name = QUrl(uri).fileName();
            names.append(name.isEmpty() ? uri : name);
        }
        if (names.isEmpty())
            return QStringLiteral("(empty file list)");
        if (names.size() > 3) {
            return names.mid(0, 3).join(QStringLiteral(", "))
                + QStringLiteral(" +%1 more").arg(names.size() - 3);
        }
        return names.join(QStringLiteral(", ")).left(kPreviewChars);
    }
    if (kind == Kind::Image) {
        for (const QString &mime : payloads.keys()) {
            if (!isImageMime(mime))
                continue;
            const QImage image = QImage::fromData(payloads.value(mime));
            if (!image.isNull()) {
                return QStringLiteral("%1 \u00d7 %2 image")
                    .arg(image.width())
                    .arg(image.height());
            }
            break;
        }
        return QStringLiteral("Image");
    }
    return QStringLiteral("Clipboard data");
}

void ClipboardHistory::enforceCaps()
{
    while (m_entries.size() > kMaxEntries
           || (m_totalBytes > kMaxTotalBytes && !m_entries.isEmpty())) {
        // Evict the oldest unpinned entry; fall back to the oldest entry when
        // every entry is pinned so the caps are still hard limits.
        int victim = -1;
        for (int i = m_entries.size() - 1; i >= 0; --i) {
            if (!m_entries.at(i).pinned) {
                victim = i;
                break;
            }
        }
        if (victim < 0)
            victim = m_entries.size() - 1;
        m_totalBytes -= m_entries.at(victim).bytes;
        m_entries.removeAt(victim);
    }
}

void ClipboardHistory::addEntry(Entry entry)
{
    for (int i = 0; i < m_entries.size(); ++i) {
        if (m_entries.at(i).hash == entry.hash) {
            // The same content copied again is promoted, not duplicated; its
            // pin survives.
            const bool pinned = m_entries.at(i).pinned;
            m_totalBytes -= m_entries.at(i).bytes;
            m_entries.removeAt(i);
            entry.pinned = pinned;
            break;
        }
    }
    m_totalBytes += entry.bytes;
    m_entries.prepend(entry);
    enforceCaps();
    emit changed();
}

bool ClipboardHistory::observe(const QStringList &mimes,
                               const QMap<QString, QByteArray> &payloads)
{
    if (isSensitive(mimes, payloads))
        return false;

    QMap<QString, QByteArray> stored;
    for (auto it = payloads.constBegin(); it != payloads.constEnd(); ++it) {
        if (isSupportedMime(it.key()))
            stored.insert(it.key(), it.value());
    }
    if (stored.isEmpty())
        return false;
    if (entryBytes(stored) > kMaxEntryBytes)
        return false;

    Entry entry;
    entry.mimes = mimes;
    entry.payloads = stored;
    entry.kind = classify(mimes);
    entry.preview = previewFor(entry.kind, mimes, stored);
    entry.hash = computeHash(stored);
    entry.bytes = entryBytes(stored);
    addEntry(entry);
    return true;
}

bool ClipboardHistory::restore(int index, QStringList *mimes,
                               QMap<QString, QByteArray> *payloads) const
{
    if (index < 0 || index >= m_entries.size())
        return false;
    const Entry &entry = m_entries.at(index);
    if (mimes)
        *mimes = entry.mimes;
    if (payloads)
        *payloads = entry.payloads;
    return true;
}

QVariantMap ClipboardHistory::toVariant(const Entry &entry, int index)
{
    QVariantMap map;
    map.insert(QStringLiteral("kind"), kindName(entry.kind));
    map.insert(QStringLiteral("preview"), entry.preview);
    map.insert(QStringLiteral("mimes"), entry.mimes);
    map.insert(QStringLiteral("pinned"), entry.pinned);
    map.insert(QStringLiteral("bytes"), entry.bytes);
    map.insert(QStringLiteral("index"), index);
    return map;
}

QVariantList ClipboardHistory::entries() const
{
    QVariantList list;
    list.reserve(m_entries.size());
    for (int i = 0; i < m_entries.size(); ++i)
        list.append(toVariant(m_entries.at(i), i));
    return list;
}

QVariantList ClipboardHistory::search(const QString &query) const
{
    QVariantList list;
    const QString needle = query.trimmed();
    for (int i = 0; i < m_entries.size(); ++i) {
        const Entry &entry = m_entries.at(i);
        if (!needle.isEmpty()
            && !entry.preview.contains(needle, Qt::CaseInsensitive)
            && !entry.mimes.join(QLatin1Char(' ')).contains(needle, Qt::CaseInsensitive))
            continue;
        list.append(toVariant(entry, i));
    }
    return list;
}

bool ClipboardHistory::setPinned(int index, bool pinned)
{
    if (index < 0 || index >= m_entries.size())
        return false;
    if (m_entries[index].pinned == pinned)
        return true;
    m_entries[index].pinned = pinned;
    emit changed();
    return true;
}

bool ClipboardHistory::remove(int index)
{
    if (index < 0 || index >= m_entries.size())
        return false;
    m_totalBytes -= m_entries.at(index).bytes;
    m_entries.removeAt(index);
    emit changed();
    return true;
}

void ClipboardHistory::clear()
{
    if (m_entries.isEmpty())
        return;
    m_entries.clear();
    m_totalBytes = 0;
    emit changed();
}

void ClipboardHistory::clearUnpinned()
{
    bool removed = false;
    for (int i = m_entries.size() - 1; i >= 0; --i) {
        if (m_entries.at(i).pinned)
            continue;
        m_totalBytes -= m_entries.at(i).bytes;
        m_entries.removeAt(i);
        removed = true;
    }
    if (removed)
        emit changed();
}

void ClipboardHistory::setClearOnLock(bool enabled)
{
    m_clearOnLock = enabled;
}

QByteArray ClipboardHistory::toJson() const
{
    QJsonArray array;
    for (const Entry &entry : m_entries) {
        QJsonObject object;
        object.insert(QStringLiteral("pinned"), entry.pinned);
        object.insert(QStringLiteral("mimes"), QJsonArray::fromStringList(entry.mimes));
        QJsonObject payloads;
        for (auto it = entry.payloads.constBegin(); it != entry.payloads.constEnd(); ++it)
            payloads.insert(it.key(), QString::fromLatin1(it.value().toBase64()));
        object.insert(QStringLiteral("payloads"), payloads);
        array.append(object);
    }
    QJsonObject root;
    root.insert(QStringLiteral("version"), 1);
    root.insert(QStringLiteral("entries"), array);
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

bool ClipboardHistory::fromJson(const QByteArray &json)
{
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject())
        return false;
    const QJsonArray array = document.object().value(QStringLiteral("entries")).toArray();

    m_entries.clear();
    m_totalBytes = 0;
    // Load oldest-first so `addEntry`'s prepend restores the original order.
    for (int i = array.size() - 1; i >= 0; --i) {
        const QJsonObject object = array.at(i).toObject();
        Entry entry;
        entry.mimes = [&object]() {
            QStringList mimes;
            const QJsonArray jsonMimes = object.value(QStringLiteral("mimes")).toArray();
            for (const QJsonValue &value : jsonMimes)
                mimes.append(value.toString());
            return mimes;
        }();
        const QJsonObject jsonPayloads = object.value(QStringLiteral("payloads")).toObject();
        for (auto it = jsonPayloads.constBegin(); it != jsonPayloads.constEnd(); ++it) {
            if (!isSupportedMime(it.key()))
                continue;
            entry.payloads.insert(it.key(), QByteArray::fromBase64(it.value().toString().toLatin1()));
        }
        if (entry.payloads.isEmpty())
            continue;
        entry.bytes = entryBytes(entry.payloads);
        if (entry.bytes > kMaxEntryBytes)
            continue;
        entry.kind = classify(entry.mimes);
        entry.preview = previewFor(entry.kind, entry.mimes, entry.payloads);
        entry.hash = computeHash(entry.payloads);
        entry.pinned = object.value(QStringLiteral("pinned")).toBool();
        m_totalBytes += entry.bytes;
        m_entries.prepend(entry);
    }
    enforceCaps();
    emit changed();
    return true;
}