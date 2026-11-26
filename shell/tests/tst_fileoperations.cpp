// SPDX-License-Identifier: GPL-3.0-or-later
// Filesystem move tests (T312): same-filesystem rename, cross-filesystem
// copy-to-sibling + rename, mid-copy rollback, metadata preservation, name
// collisions, and permission-denied cleanup. No Wayland, QML, or compositor.
#include "fileoperations.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
#include <QTest>

#include <chrono>
#include <filesystem>
#include <sys/stat.h>
#include <system_error>

namespace {

namespace fs = std::filesystem;

fs::path nativePath(const QString &path)
{
    return fs::path(path.toStdString());
}

void writeFile(const QString &path, const QByteArray &contents)
{
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    QCOMPARE(file.write(contents), contents.size());
    file.close();
}

QStringList entries(const QString &dir)
{
    return QDir(dir).entryList(QDir::AllEntries | QDir::Hidden
                               | QDir::NoDotAndDotDot);
}

bool hasMoveTemp(const QString &dir)
{
    const QStringList names = entries(dir);
    for (const QString &name : names) {
        if (name.startsWith(QStringLiteral(".dragonfruit-move-")))
            return true;
    }
    return false;
}

fs::file_time_type mtimeOf(const QString &path)
{
    std::error_code error;
    const fs::file_time_type value = fs::last_write_time(nativePath(path), error);
    return error ? fs::file_time_type::min() : value;
}

bool setMtime(const QString &path, fs::file_time_type value)
{
    std::error_code error;
    fs::last_write_time(nativePath(path), value, error);
    return !error;
}

fs::file_time_type testMtime()
{
    return fs::file_time_type(std::chrono::seconds(1600000000));
}

} // namespace

class TestFileOperations : public QObject
{
    Q_OBJECT

private slots:
    // The common path stays one atomic rename; no temp is involved.
    void sameFilesystemDirectoryMoveIsOneRename()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString target = dir.path() + QStringLiteral("/Target");
        const QString source = dir.path() + QStringLiteral("/Dropped");
        QVERIFY(QDir().mkpath(source));
        QVERIFY(QDir().mkpath(target));
        writeFile(source + QStringLiteral("/a.txt"), "a");

        QString error;
        QCOMPARE(moveFilesIntoFolder(target, {source}, &error), 1);
        QVERIFY(error.isEmpty());
        QVERIFY(!QFileInfo::exists(source));
        QVERIFY(QFileInfo::exists(target + QStringLiteral("/Dropped/a.txt")));
        QVERIFY(!hasMoveTemp(target));
    }

    // The cross-filesystem path is a complete copy into a hidden sibling, a
    // rename into place, and then source removal. The tree, modes, mtime, and
    // symlinks survive; no temp is left behind.
    void crossFilesystemDirectoryMovePreservesTheTree()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString target = dir.path() + QStringLiteral("/Target");
        const QString source = dir.path() + QStringLiteral("/Dropped");
        const QString nested = source + QStringLiteral("/nested");
        QVERIFY(QDir().mkpath(nested));
        QVERIFY(QDir().mkpath(target));

        const QString top = source + QStringLiteral("/top.txt");
        const QString child = nested + QStringLiteral("/child.txt");
        writeFile(top, "top");
        writeFile(child, "child");

        std::error_code error;
        fs::create_symlink("top.txt",
                           nativePath(source + QStringLiteral("/link.txt")),
                           error);
        QVERIFY2(!error, error.message().c_str());

        QVERIFY(QFile::setPermissions(
            top, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                     | QFileDevice::ReadGroup));
        const QFileDevice::Permissions filePermissions = QFileInfo(top).permissions();
        const QFileDevice::Permissions rawDirPermissions =
            QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner
            | QFileDevice::ReadGroup | QFileDevice::ExeGroup;
        QVERIFY(QFile::setPermissions(nested, rawDirPermissions));
        QVERIFY(QFile::setPermissions(source, rawDirPermissions));
        const QFileDevice::Permissions dirPermissions = QFileInfo(nested).permissions();

        const fs::file_time_type sourceMtime = testMtime();
        QVERIFY(setMtime(top, sourceMtime));
        QVERIFY(setMtime(nested, sourceMtime));
        QVERIFY(setMtime(source, sourceMtime));

        FileMoveOptions options;
        options.forceCrossFilesystem = true;
        QString moveError;
        QCOMPARE(moveFilesIntoFolder(target, {source}, &moveError, options), 1);
        QVERIFY(moveError.isEmpty());

        const QString moved = target + QStringLiteral("/Dropped");
        QVERIFY(QFileInfo(moved).isDir());
        QVERIFY(!QFileInfo::exists(source));
        QVERIFY(!hasMoveTemp(target));

        QFile topFile(moved + QStringLiteral("/top.txt"));
        QVERIFY(topFile.open(QIODevice::ReadOnly));
        QCOMPARE(topFile.readAll(), QByteArray("top"));
        topFile.close();
        QFile childFile(moved + QStringLiteral("/nested/child.txt"));
        QVERIFY(childFile.open(QIODevice::ReadOnly));
        QCOMPARE(childFile.readAll(), QByteArray("child"));
        childFile.close();

        QCOMPARE(QFileInfo(moved + QStringLiteral("/top.txt")).permissions(),
                 filePermissions);
        QCOMPARE(QFileInfo(moved).permissions(), dirPermissions);
        QCOMPARE(QFileInfo(moved + QStringLiteral("/nested")).permissions(),
                 dirPermissions);
        QCOMPARE(mtimeOf(moved + QStringLiteral("/top.txt")), sourceMtime);
        QCOMPARE(mtimeOf(moved), sourceMtime);
        QCOMPARE(mtimeOf(moved + QStringLiteral("/nested")), sourceMtime);

        const fs::path link = nativePath(moved + QStringLiteral("/link.txt"));
        QVERIFY(fs::is_symlink(fs::symlink_status(link)));
        error.clear();
        QCOMPARE(fs::read_symlink(link, error), fs::path("top.txt"));
        QVERIFY2(!error, error.message().c_str());
    }

    // When the host has a second filesystem, exercise the real EXDEV path
    // instead of the forced-copy seam. Skipped on single-filesystem hosts.
    void realCrossDeviceMoveWhenAvailable()
    {
        QTemporaryDir sourceRoot(QStringLiteral("/dev/shm/t312-XXXXXX"));
        QTemporaryDir targetRoot;
        if (!sourceRoot.isValid() || !targetRoot.isValid())
            QSKIP("no second filesystem available for a real cross-device move");

        struct stat sourceStat {};
        struct stat targetStat {};
        if (::stat(sourceRoot.path().toLocal8Bit().constData(), &sourceStat) != 0
            || ::stat(targetRoot.path().toLocal8Bit().constData(), &targetStat) != 0
            || sourceStat.st_dev == targetStat.st_dev) {
            QSKIP("the test paths share one filesystem");
        }

        const QString source = sourceRoot.path() + QStringLiteral("/Dropped");
        const QString target = targetRoot.path() + QStringLiteral("/Target");
        QVERIFY(QDir().mkpath(source));
        QVERIFY(QDir().mkpath(target));
        writeFile(source + QStringLiteral("/a.txt"), "a");

        QString error;
        QCOMPARE(moveFilesIntoFolder(target, {source}, &error), 1);
        QVERIFY(error.isEmpty());
        QVERIFY(!QFileInfo::exists(source));
        QVERIFY(QFileInfo::exists(target + QStringLiteral("/Dropped/a.txt")));
        QVERIFY(!hasMoveTemp(target));
    }

    // A top-level file also goes through the temp-file + rename path.
    void crossFilesystemFileMovePreservesMetadata()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString target = dir.path() + QStringLiteral("/Target");
        const QString file = dir.path() + QStringLiteral("/note.txt");
        QVERIFY(QDir().mkpath(target));
        writeFile(file, "hello");

        QVERIFY(QFile::setPermissions(
            file, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                      | QFileDevice::ReadGroup));
        const QFileDevice::Permissions permissions = QFileInfo(file).permissions();
        const fs::file_time_type sourceMtime = testMtime();
        QVERIFY(setMtime(file, sourceMtime));

        FileMoveOptions options;
        options.forceCrossFilesystem = true;
        QString moveError;
        QCOMPARE(moveFilesIntoFolder(target, {file}, &moveError, options), 1);
        QVERIFY(moveError.isEmpty());

        const QString moved = target + QStringLiteral("/note.txt");
        QVERIFY(!QFileInfo::exists(file));
        QVERIFY(QFileInfo::exists(moved));
        QVERIFY(!hasMoveTemp(target));
        QCOMPARE(QFileInfo(moved).permissions(), permissions);
        QCOMPARE(mtimeOf(moved), sourceMtime);

        QFile movedFile(moved);
        QVERIFY(movedFile.open(QIODevice::ReadOnly));
        QCOMPARE(movedFile.readAll(), QByteArray("hello"));
    }

    // A read-only source directory still completes: the copy reads it, the
    // destination keeps its mode, and source removal makes the source tree
    // writable just long enough to delete it.
    void readOnlySourceDirectoryIsRemovedAfterCopy()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString target = dir.path() + QStringLiteral("/Target");
        const QString source = dir.path() + QStringLiteral("/Dropped");
        const QString file = source + QStringLiteral("/readonly.txt");
        QVERIFY(QDir().mkpath(target));
        QVERIFY(QDir().mkpath(source));
        writeFile(file, "readonly");

        const QFileDevice::Permissions sourceDirPermissions =
            QFileDevice::ReadOwner | QFileDevice::ExeOwner | QFileDevice::ReadGroup
            | QFileDevice::ExeGroup;
        QVERIFY(QFile::setPermissions(file, QFileDevice::ReadOwner));
        QVERIFY(QFile::setPermissions(source, sourceDirPermissions));
        const QFileDevice::Permissions expectedDirPermissions =
            QFileInfo(source).permissions();
        const QFileDevice::Permissions expectedFilePermissions =
            QFileInfo(file).permissions();

        FileMoveOptions options;
        options.forceCrossFilesystem = true;
        QString error;
        QCOMPARE(moveFilesIntoFolder(target, {source}, &error, options), 1);
        QVERIFY(error.isEmpty());
        QVERIFY(!QFileInfo::exists(source));
        QCOMPARE(QFileInfo(target + QStringLiteral("/Dropped")).permissions(),
                 expectedDirPermissions);
        QCOMPARE(QFileInfo(target + QStringLiteral("/Dropped/readonly.txt")).permissions(),
                 expectedFilePermissions);
        QVERIFY(QFileInfo::exists(target + QStringLiteral("/Dropped/readonly.txt")));

        QFile::setPermissions(target + QStringLiteral("/Dropped/readonly.txt"),
                              QFileDevice::ReadOwner | QFileDevice::WriteOwner);
        QFile::setPermissions(target + QStringLiteral("/Dropped"),
                              QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                  | QFileDevice::ExeOwner);
    }

    // A failure after some entries were copied removes the partial temp and
    // leaves every source byte in place. The retry lands on the original name,
    // never `Dropped.1` beside a half-moved `Dropped`.
    void crossFilesystemMidCopyFailureLeavesSourceAndNoTemp()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString target = dir.path() + QStringLiteral("/Target");
        const QString source = dir.path() + QStringLiteral("/Dropped");
        QVERIFY(QDir().mkpath(target));
        QVERIFY(QDir().mkpath(source));
        writeFile(source + QStringLiteral("/a.txt"), "a");
        writeFile(source + QStringLiteral("/b.txt"), "b");
        writeFile(source + QStringLiteral("/c.txt"), "c");

        FileMoveOptions options;
        options.forceCrossFilesystem = true;
        options.failAfterEntries = 2;
        QString error;
        const QList<FileOperationItem> outcomes =
            moveFilesIntoFolderDetailed(target, {source}, &error, options);

        QCOMPARE(outcomes.size(), 1);
        QVERIFY(!outcomes.first().ok);
        QVERIFY(!error.isEmpty());
        QVERIFY(!outcomes.first().error.isEmpty());
        QVERIFY(QFileInfo(source).isDir());
        QCOMPARE(QFileInfo(source + QStringLiteral("/a.txt")).size(), qint64(1));
        QCOMPARE(QFileInfo(source + QStringLiteral("/b.txt")).size(), qint64(1));
        QCOMPARE(QFileInfo(source + QStringLiteral("/c.txt")).size(), qint64(1));
        QVERIFY(!QFileInfo::exists(target + QStringLiteral("/Dropped")));
        QVERIFY(!QFileInfo::exists(target + QStringLiteral("/Dropped.1")));
        QVERIFY(!hasMoveTemp(target));

        FileMoveOptions retry;
        retry.forceCrossFilesystem = true;
        QString retryError;
        QCOMPARE(moveFilesIntoFolder(target, {source}, &retryError, retry), 1);
        QVERIFY(retryError.isEmpty());
        QVERIFY(QFileInfo::exists(target + QStringLiteral("/Dropped/a.txt")));
        QVERIFY(!QFileInfo::exists(target + QStringLiteral("/Dropped.1")));
        QVERIFY(!hasMoveTemp(target));
    }

    // A source file that cannot be read fails the copy. The source directory
    // is complete, the temp is gone, and the destination never appears.
    void permissionDeniedCopyLeavesSourceIntact()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString target = dir.path() + QStringLiteral("/Target");
        const QString source = dir.path() + QStringLiteral("/Dropped");
        const QString secret = source + QStringLiteral("/secret.txt");
        QVERIFY(QDir().mkpath(target));
        QVERIFY(QDir().mkpath(source));
        writeFile(secret, "secret");
        QVERIFY(QFile::setPermissions(secret, QFileDevice::Permissions()));
        if (QFileInfo(secret).isReadable()) {
            QFile::setPermissions(secret,
                                  QFileDevice::ReadOwner | QFileDevice::WriteOwner);
            QSKIP("file permissions are not enforced in this environment");
        }

        FileMoveOptions options;
        options.forceCrossFilesystem = true;
        QString error;
        const QList<FileOperationItem> outcomes =
            moveFilesIntoFolderDetailed(target, {source}, &error, options);
        QCOMPARE(outcomes.size(), 1);
        QVERIFY(!outcomes.first().ok);
        QVERIFY(!error.isEmpty());
        QVERIFY(QFileInfo::exists(source));
        QVERIFY(QFileInfo::exists(secret));
        QVERIFY(!QFileInfo::exists(target + QStringLiteral("/Dropped")));
        QVERIFY(!hasMoveTemp(target));

        QFile::setPermissions(secret,
                              QFileDevice::ReadOwner | QFileDevice::WriteOwner);
    }

    // An unwritable destination cannot create the temp sibling; the source is
    // untouched and no partial destination appears.
    void unwritableDestinationLeavesSourceIntact()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString target = dir.path() + QStringLiteral("/Target");
        const QString source = dir.path() + QStringLiteral("/Dropped");
        QVERIFY(QDir().mkpath(target));
        QVERIFY(QDir().mkpath(source));
        writeFile(source + QStringLiteral("/a.txt"), "a");
        QVERIFY(QFile::setPermissions(target,
                                      QFileDevice::ReadOwner | QFileDevice::ExeOwner));
        if (QFileInfo(target).isWritable()) {
            QFile::setPermissions(target,
                                  QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                      | QFileDevice::ExeOwner);
            QSKIP("file permissions are not enforced in this environment");
        }

        FileMoveOptions options;
        options.forceCrossFilesystem = true;
        QString error;
        const QList<FileOperationItem> outcomes =
            moveFilesIntoFolderDetailed(target, {source}, &error, options);
        QCOMPARE(outcomes.size(), 1);
        QVERIFY(!outcomes.first().ok);
        QVERIFY(!error.isEmpty());
        QVERIFY(QFileInfo::exists(source + QStringLiteral("/a.txt")));
        QVERIFY(!QFileInfo::exists(target + QStringLiteral("/Dropped")));
        QVERIFY(!hasMoveTemp(target));

        QFile::setPermissions(target,
                              QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                  | QFileDevice::ExeOwner);
    }

    // Copying a directory into its own subtree must be refused before the
    // temp copy starts, otherwise it would recurse into itself.
    void moveIntoOwnSubtreeIsRefused()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString source = dir.path() + QStringLiteral("/Outer");
        const QString target = source + QStringLiteral("/Inner");
        QVERIFY(QDir().mkpath(target));
        writeFile(source + QStringLiteral("/a.txt"), "a");

        FileMoveOptions options;
        options.forceCrossFilesystem = true;
        QString error;
        const QList<FileOperationItem> outcomes =
            moveFilesIntoFolderDetailed(target, {source}, &error, options);
        QCOMPARE(outcomes.size(), 1);
        QVERIFY(!outcomes.first().ok);
        QVERIFY(!error.isEmpty());
        QVERIFY(QFileInfo::exists(source + QStringLiteral("/a.txt")));
        QVERIFY(entries(target).isEmpty());
    }
};

QTEST_GUILESS_MAIN(TestFileOperations)
#include "tst_fileoperations.moc"
