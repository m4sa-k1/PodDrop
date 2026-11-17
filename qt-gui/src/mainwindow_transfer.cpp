#include "mainwindow.hpp"
#include "mainwindow_helpers.hpp"

#include <QApplication>
#include <QComboBox>
#include <QDir>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QFileInfo>
#include <QMessageBox>
#include <QMimeData>
#include <thread>

void MainWindow::startTransferBatch(
    const QStringList &files,
    const QString &coverPath,
    const QString &albumOverride,
    const QString &artistOverride
) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy || files.isEmpty()) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const int32_t fmt = m_formatCombo->currentIndex();

    std::thread([this, mp, files, coverPath, albumOverride, artistOverride, fmt]() {
        const QByteArray artB = coverPath.toUtf8();
        const QByteArray albB = albumOverride.toUtf8();
        const QByteArray arB = artistOverride.toUtf8();

        int succeeded = 0;
        QString firstError;

        for (int i = 0; i < files.size(); ++i) {
            const QByteArray audioB = files[i].toUtf8();
            PodDropTransferRequest req{};
            req.audio_path = audioB.constData();
            req.artwork_path = artB.isEmpty() ? nullptr : artB.constData();
            req.transcode_format = fmt;
            req.title_override = nullptr;
            req.artist_override = arB.isEmpty() ? nullptr : arB.constData();
            req.album_override = albB.isEmpty() ? nullptr : albB.constData();

            uint64_t outId = 0;
            char *err = nullptr;
            if (poddrop_transfer_track(mp.constData(), &req, i == 0 ? 1 : 0, &outId, &err) == 0) {
                succeeded++;
            } else if (firstError.isEmpty()) {
                firstError = takeCString(err);
            } else {
                poddrop_free_string(err);
            }
        }

        QMetaObject::invokeMethod(
            this,
            [this, succeeded, total = files.size(), firstError]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (firstError.isEmpty()) {
                    m_statusMsg = QString("%1 (%2/%3)")
                                      .arg(trText(m_lang, "lbl.transfer_done"))
                                      .arg(succeeded)
                                      .arg(total);
                } else {
                    m_statusMsg = QString("%1 (%2/%3) - %4")
                                      .arg(trText(m_lang, "lbl.transfer_done"))
                                      .arg(succeeded)
                                      .arg(total)
                                      .arg(firstError);
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::dragEnterEvent(QDragEnterEvent *event) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy) return;
    if (event->mimeData()->hasUrls()) {
        for (const QUrl &url : event->mimeData()->urls()) {
            if (!url.isLocalFile()) continue;
            const QString p = url.toLocalFile();
            if (QFileInfo(p).isDir() || isSupportedAudioExt(p)) {
                event->acceptProposedAction();
                return;
            }
        }
    }
}

void MainWindow::dropEvent(QDropEvent *event) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy) return;
    QStringList files;
    QStringList folders;
    for (const QUrl &url : event->mimeData()->urls()) {
        if (!url.isLocalFile()) continue;
        const QString path = url.toLocalFile();
        if (QFileInfo(path).isDir()) {
            folders.push_back(path);
        } else if (isSupportedAudioExt(path)) {
            files.push_back(path);
        }
    }
    if (!folders.isEmpty()) {
        handleDroppedFolder(folders.first());
        event->acceptProposedAction();
        return;
    }
    if (!files.isEmpty()) {
        startTransferBatch(files, QString(), m_selectedAlbum, QString());
        event->acceptProposedAction();
    }
}

void MainWindow::handleDroppedFolder(const QString &folderPath) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy) return;

    const QFileInfo dirInfo(folderPath);
    const QString albumName = dirInfo.fileName().trimmed();
    if (!dirInfo.isDir() || albumName.isEmpty()) return;

    m_busy = true;
    m_statusMsg = trText(m_lang, "lbl.validating");
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const bool hasStorageStats = m_device.hasStorageStats;
    const uint64_t freeBytes = m_device.storageFreeBytes;

    std::thread([this, folderPath, albumName, hasStorageStats, freeBytes]() {
        QStringList audioFiles;
        QString coverPath;

        QDir dir(folderPath);
        const QStringList coverCandidates = {
            "cover.jpg", "cover.jpeg", "cover.png",
            "folder.jpg", "folder.jpeg", "folder.png",
            "front.jpg", "front.jpeg", "front.png",
            "artwork.jpg", "artwork.jpeg", "artwork.png"
        };
        for (const QString &name : coverCandidates) {
            const QString candidate = dir.filePath(name);
            if (QFileInfo::exists(candidate) && QFileInfo(candidate).isFile()) {
                coverPath = candidate;
                break;
            }
        }

        const QFileInfoList entries = dir.entryInfoList(
            QDir::Files | QDir::NoDotAndDotDot,
            QDir::Name | QDir::LocaleAware
        );
        for (const QFileInfo &fi : entries) {
            if (fi.fileName().startsWith('.')) continue;
            if (isSupportedAudioExt(fi.absoluteFilePath())) {
                audioFiles.push_back(fi.absoluteFilePath());
            }
        }

        QString errorKey;
        QString errorDetail;
        uint64_t totalSize = 0;

        if (audioFiles.isEmpty()) {
            errorKey = QStringLiteral("err.no_audio_in_folder");
        } else {
            for (const QString &f : audioFiles) {
                const QByteArray b = f.toUtf8();
                uint64_t sz = 0;
                uint32_t dur = 0;
                char *err = nullptr;
                if (poddrop_validate_audio(b.constData(), &sz, &dur, &err) != 0) {
                    errorKey = QStringLiteral("err.validation_failed");
                    errorDetail = takeCString(err);
                    break;
                }
                if (err) poddrop_free_string(err);
                totalSize += sz;
            }
        }

        if (errorKey.isEmpty() && hasStorageStats && totalSize > freeBytes) {
            errorKey = QStringLiteral("err.insufficient_space");
        }

        QMetaObject::invokeMethod(
            this,
            [this, albumName, audioFiles, coverPath, totalSize, errorKey, errorDetail]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;

                if (!errorKey.isEmpty()) {
                    m_statusMsg = trText(m_lang, "lbl.ready");
                    applyTranslations();
                    updateActionStates();
                    QString msg = trText(m_lang, errorKey.toUtf8().constData());
                    if (!errorDetail.isEmpty()) {
                        msg += QString("\n\n%1").arg(errorDetail);
                    }
                    QMessageBox::warning(this, trText(m_lang, "dlg.folder_album_title"), msg);
                    return;
                }

                applyTranslations();
                updateActionStates();

                const double mb = static_cast<double>(totalSize) / (1024.0 * 1024.0);
                const QString coverLabel = coverPath.isEmpty()
                                               ? QStringLiteral("-")
                                               : QFileInfo(coverPath).fileName();
                const QString confirmMsg = trText(m_lang, "dlg.folder_confirm")
                                               .arg(albumName)
                                               .arg(audioFiles.size())
                                               .arg(mb, 0, 'f', 2)
                                               .arg(coverLabel);

                const auto reply = QMessageBox::question(
                    this,
                    trText(m_lang, "dlg.folder_album_title"),
                    confirmMsg
                );
                if (reply == QMessageBox::Yes) {
                    startTransferBatch(audioFiles, coverPath, albumName, QString());
                } else {
                    m_statusMsg = trText(m_lang, "lbl.ready");
                    applyTranslations();
                }
            },
            Qt::QueuedConnection
        );
    }).detach();
}
