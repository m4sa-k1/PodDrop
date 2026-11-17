#include "mainwindow.hpp"
#include "mainwindow_helpers.hpp"

#include <QApplication>
#include <QFileDialog>
#include <QMessageBox>
#include <thread>

void MainWindow::addMusicFiles() {
    const QStringList files = QFileDialog::getOpenFileNames(
        this,
        trText(m_lang, "act.add_music"),
        QString(),
        "Audio (*.mp3 *.m4a *.mp4 *.flac *.ogg *.opus *.wav)"
    );
    if (!files.isEmpty()) {
        startTransferBatch(files, QString(), QString(), QString());
    }
}

void MainWindow::createAlbum(const QString &initialAlbum, const QString &initialArtist) {
    if (auto res = runCreateAlbumDialog(this, m_lang, initialAlbum, initialArtist)) {
        startTransferBatch(res->audioFiles, res->coverPath, res->album, res->artist);
    }
}

void MainWindow::addToSelectedAlbum() {
    QString artist;
    if (!m_selectedAlbum.isEmpty()) {
        for (const auto &t : m_tracks) {
            if (t.album == m_selectedAlbum && !t.artist.isEmpty()) {
                artist = t.artist;
                break;
            }
        }
    }
    createAlbum(m_selectedAlbum, artist);
}

void MainWindow::editSelectedAlbum() {
    if (!m_hasDevice || m_selectedAlbum.isEmpty()) return;

    AlbumInfoSummary summary{};
    summary.album = m_selectedAlbum;
    uint64_t coverTrackId = 0;
    for (const auto &t : m_tracks) {
        const QString alb = t.album.isEmpty() ? QStringLiteral("Unknown Album") : t.album;
        if (alb != m_selectedAlbum) continue;
        summary.trackCount++;
        summary.totalDurationMs += t.durationMs;
        summary.totalSizeBytes += t.sizeBytes;
        if (summary.artist.isEmpty()) {
            if (!t.albumArtist.isEmpty()) {
                summary.artist = t.albumArtist;
            } else if (!t.artist.isEmpty()) {
                summary.artist = t.artist;
            }
        }
        if (summary.genre.isEmpty() && !t.genre.isEmpty()) summary.genre = t.genre;
        if (summary.year == 0 && t.year > 0) summary.year = t.year;
        if (t.hasArtwork) {
            summary.hasArtwork = true;
            if (coverTrackId == 0) coverTrackId = t.id;
        }
    }
    if (summary.trackCount == 0) return;

    const bool canWrite = !m_device.isReadOnly && !m_busy;
    const QPixmap currentCover = coverTrackId > 0 ? loadTrackCoverPixmap(m_mountPath, coverTrackId) : QPixmap();
    const auto editedOpt = runAlbumEditorDialog(this, m_lang, summary, currentCover, canWrite);
    if (!editedOpt.has_value() || !canWrite) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const QString oldAlbum = m_selectedAlbum;
    const AlbumEditResult edit = *editedOpt;

    std::thread([this, mp, oldAlbum, edit]() {
        const QByteArray oldB = oldAlbum.toUtf8();
        const QByteArray newAlbB = edit.newAlbum.toUtf8();
        const QByteArray newArtB = edit.newArtist.toUtf8();
        const QByteArray newGenB = edit.newGenre.toUtf8();
        const QByteArray coverB = edit.coverPath.toUtf8();

        PodDropUpdateAlbumRequest req{};
        req.old_album = oldB.constData();
        req.new_album = newAlbB.constData();
        req.new_artist = newArtB.constData();
        req.new_genre = newGenB.constData();
        req.year = edit.year;
        req.artwork_path = coverB.isEmpty() ? nullptr : coverB.constData();
        req.clear_artwork = edit.clearArtwork ? 1 : 0;
        req.apply_artist_to_tracks = edit.applyArtistToTracks ? 1 : 0;
        req.apply_genre_to_tracks = edit.applyGenreToTracks ? 1 : 0;
        req.apply_year_to_tracks = edit.applyYearToTracks ? 1 : 0;
        req.apply_artwork_to_tracks = edit.applyArtworkToTracks ? 1 : 0;

        uint32_t updated = 0;
        char *err = nullptr;
        const int32_t rc = poddrop_update_album(mp.constData(), &req, &updated, &err);
        const QString errMsg = takeCString(err);
        const QString nextSelected = edit.newAlbum.trimmed().isEmpty()
                                         ? QStringLiteral("Unknown Album")
                                         : edit.newAlbum.trimmed();

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg, updated, nextSelected]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    m_selectedAlbum = nextSelected;
                    m_statusMsg = QString("%1 (%2)").arg(trText(m_lang, "lbl.album_updated")).arg(updated);
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::removeSelectedAlbum() {
    if (!m_hasDevice || m_device.isReadOnly || m_busy || m_selectedAlbum.isEmpty()) return;

    int count = 0;
    for (const auto &t : m_tracks) {
        const QString alb = t.album.isEmpty() ? QStringLiteral("Unknown Album") : t.album;
        if (alb == m_selectedAlbum) count++;
    }
    if (count == 0) return;

    const auto modeOpt = runRemoveAlbumDialog(this, m_lang, m_selectedAlbum, count);
    if (!modeOpt.has_value()) return;
    const bool deleteTracks = *modeOpt;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const QString albumTarget = m_selectedAlbum;

    std::thread([this, mp, albumTarget, deleteTracks]() {
        const QByteArray albB = albumTarget.toUtf8();
        uint32_t affected = 0;
        char *err = nullptr;
        const int32_t rc = poddrop_remove_album(
            mp.constData(),
            albB.constData(),
            deleteTracks ? 1 : 0,
            &affected,
            &err
        );
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg, affected]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    m_selectedAlbum.clear();
                    m_statusMsg = QString("%1 (%2)").arg(trText(m_lang, "lbl.album_removed")).arg(affected);
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::editSelectedTrack() {
    const auto trackOpt = selectedTrack();
    if (!trackOpt.has_value()) return;
    const bool canWrite = m_hasDevice && !m_device.isReadOnly && !m_busy;
    const QPixmap currentCover = trackOpt->hasArtwork ? loadTrackCoverPixmap(m_mountPath, trackOpt->id) : QPixmap();
    const auto editedOpt = runTrackEditorDialog(this, m_lang, *trackOpt, currentCover, canWrite);
    if (!editedOpt.has_value() || !canWrite) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const uint64_t tid = trackOpt->id;
    const TrackEditResult edit = *editedOpt;

    std::thread([this, mp, tid, edit]() {
        const QByteArray titleB = edit.title.toUtf8();
        const QByteArray artistB = edit.artist.toUtf8();
        const QByteArray albumB = edit.album.toUtf8();
        const QByteArray genreB = edit.genre.toUtf8();
        const QByteArray artB = edit.coverPath.toUtf8();

        PodDropUpdateRequest req{};
        req.title = titleB.constData();
        req.artist = artistB.constData();
        req.album = albumB.constData();
        req.genre = genreB.constData();
        req.track_number = edit.trackNumber;
        req.disc_number = edit.discNumber;
        req.year = edit.year;
        req.artwork_path = artB.isEmpty() ? nullptr : artB.constData();
        req.clear_artwork = edit.clearArtwork ? 1 : 0;

        char *err = nullptr;
        const int32_t rc = poddrop_update_track(mp.constData(), tid, &req, &err);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                m_statusMsg = (rc == 0) ? trText(m_lang, "lbl.update_done") : errMsg;
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::removeSelectedTrack() {
    const auto trackOpt = selectedTrack();
    if (!trackOpt.has_value() || m_device.isReadOnly) return;

    const auto reply = QMessageBox::question(
        this,
        trText(m_lang, "act.remove_track"),
        QString("%1\n\n%2").arg(trText(m_lang, "dlg.remove_confirm"), trackOpt->title)
    );
    if (reply != QMessageBox::Yes) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const uint64_t tid = trackOpt->id;

    std::thread([this, mp, tid]() {
        char *err = nullptr;
        const int32_t rc = poddrop_remove_track(mp.constData(), tid, &err);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                m_statusMsg = (rc == 0) ? trText(m_lang, "lbl.ready") : errMsg;
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}
