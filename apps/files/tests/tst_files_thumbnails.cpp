// SPDX-License-Identifier: MIT
// The Files thumbnail provider: a local image decodes to a bounded preview and
// lands in the freedesktop cache; non-local and non-image inputs are rejected.
#include <QDir>
#include <QFileInfo>
#include <QImage>
#include <QStandardPaths>
#include <QTemporaryDir>
#include <QUrl>
#include <QtTest>

#include "FilesThumbnailProvider.h"

class FilesThumbnailsTest : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        QVERIFY(m_cache.isValid());
        qputenv("XDG_CACHE_HOME", m_cache.path().toUtf8());
    }
    void a_local_image_gets_a_cached_bounded_thumbnail()
    {
        QTemporaryDir sourceDir;
        QVERIFY(sourceDir.isValid());
        const QString path = sourceDir.path() + QStringLiteral("/wide image #1.png");

        QImage source(512, 256, QImage::Format_ARGB32_Premultiplied);
        source.fill(Qt::blue);
        QVERIFY(source.save(path, "PNG"));

        const QString uri = QUrl::fromLocalFile(path).toString();
        const QString sourceUrl = FilesThumbnailProvider::sourceFor(uri);
        const QString prefix = QStringLiteral("image://files-thumbnail/");
        QVERIFY(sourceUrl.startsWith(prefix));

        FilesThumbnailProvider provider;
        QSize size;
        const QImage thumbnail = provider.requestImage(
                sourceUrl.mid(prefix.size()), &size, QSize(128, 128));
        QVERIFY(!thumbnail.isNull());
        QCOMPARE(size, thumbnail.size());
        // A 2:1 source fits in the 128px normal bucket without distortion.
        QCOMPARE(thumbnail.size(), QSize(128, 64));

        const QString cacheDir =
                QStandardPaths::writableLocation(QStandardPaths::GenericCacheLocation)
                + QStringLiteral("/thumbnails/normal");
        const QStringList cached =
                QDir(cacheDir).entryList({QStringLiteral("*.png")}, QDir::Files);
        QCOMPARE(cached.size(), 1);
        const QImage fromCache(cacheDir + QLatin1Char('/') + cached.first());
        QVERIFY(!fromCache.isNull());
        QCOMPARE(fromCache.size(), thumbnail.size());
    }

    void non_local_sources_have_no_thumbnail_url()
    {
        QCOMPARE(FilesThumbnailProvider::sourceFor(
                         QStringLiteral("https://example.com/a.png")),
                 QString());
        QCOMPARE(FilesThumbnailProvider::sourceFor(QString()), QString());
    }

private:
    QTemporaryDir m_cache;
};

QTEST_GUILESS_MAIN(FilesThumbnailsTest)

#include "tst_files_thumbnails.moc"
