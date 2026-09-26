// SPDX-License-Identifier: MIT
// Screenshot save/copy tests (T-13.3b): the writer saves a PNG into the
// capture directory and puts the image (and the saved file's URI) on the
// clipboard. Needs a QGuiApplication for the clipboard, but no Wayland,
// compositor, portal, or QML.
#include "screenshotwriter.h"

#include <QClipboard>
#include <QDateTime>
#include <QFileInfo>
#include <QGuiApplication>
#include <QMimeData>
#include <QTemporaryDir>
#include <QTest>
#include <QUrl>

class TestScreenshotWriter : public QObject
{
    Q_OBJECT

private slots:
    void init()
    {
        // Each test gets a fresh capture directory; the writer falls back to
        // the XDG Pictures dir when the override is absent.
        QVERIFY(m_dir.isValid());
        qputenv("DF_SCREENSHOT_DIR", m_dir.path().toUtf8());
    }

    void cleanup() { qunsetenv("DF_SCREENSHOT_DIR"); }

    void savingWritesAPngFile()
    {
        ScreenshotWriter writer;
        const QImage image = sample();
        const QString path = ScreenshotWriter::savePath(QDateTime::fromSecsSinceEpoch(1700000000));
        QVERIFY(!path.isEmpty());
        QVERIFY(ScreenshotWriter::save(image, path));
        QVERIFY(QFileInfo::exists(path));
        const QImage loaded(path);
        QCOMPARE(loaded.size(), image.size());
        QVERIFY(loaded.pixelColor(0, 0) == image.pixelColor(0, 0));
    }

    void deliveringSavesAndCopies()
    {
        ScreenshotWriter writer;
        const QImage image = sample();
        const ScreenshotWriter::Result result = writer.deliver(image);
        QVERIFY(result.saved);
        QVERIFY(result.copied);
        QVERIFY(QFileInfo::exists(result.path));
        QCOMPARE(QGuiApplication::clipboard()->image().size(), image.size());
        const QMimeData *mime = QGuiApplication::clipboard()->mimeData();
        QVERIFY(mime);
        QVERIFY(mime->urls().contains(QUrl::fromLocalFile(result.path)));
    }

    void copyPutssTheImageOnTheClipboard()
    {
        const QImage image = sample();
        QVERIFY(ScreenshotWriter::copy(image));
        QCOMPARE(QGuiApplication::clipboard()->image().size(), image.size());
    }

    void savePathsAreTimestampedAndUnique()
    {
        const QDateTime now = QDateTime::fromSecsSinceEpoch(1700009999);
        const QString first = ScreenshotWriter::savePath(now);
        QVERIFY(first.contains(ScreenshotWriter::baseName(now)));
        QVERIFY(ScreenshotWriter::save(sample(), first));
        const QString second = ScreenshotWriter::savePath(now);
        QVERIFY(second != first);
        QVERIFY(second.endsWith(QStringLiteral("-1.png")));
    }

    void anEmptyImageOrPathIsRefused()
    {
        QVERIFY(!ScreenshotWriter::save(QImage(), m_dir.filePath("x.png")));
        QVERIFY(!ScreenshotWriter::save(sample(), QString()));
        QVERIFY(!ScreenshotWriter::copy(QImage()));
        // A copy without a file path is still a valid image copy.
        QVERIFY(ScreenshotWriter::copy(sample(), QString()));
    }

private:
    static QImage sample()
    {
        QImage image(8, 6, QImage::Format_RGBA8888);
        image.fill(QColor(200, 100, 50));
        return image;
    }

    QTemporaryDir m_dir;
};

QTEST_MAIN(TestScreenshotWriter)
#include "tst_screenshotwriter.moc"