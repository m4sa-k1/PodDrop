#include "mainwindow.hpp"
#include "mainwindow_helpers.hpp"

#include <QApplication>
#include <QMessageBox>
#include <thread>

void MainWindow::createPlaylist(const QVector<uint32_t> &initialTrackIds) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy) return;
    const auto resOpt = runPlaylistEditorDialog(
        this,
        m_lang,
        QString(),
        initialTrackIds,
        m_tracks,
        true,
        true
    );
    if (!resOpt.has_value()) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const PlaylistEditResult res = *resOpt;

    std::thread([this, mp, res]() {
        const QByteArray nameB = res.name.toUtf8();
        uint64_t newId = 0;
        char *err = nullptr;
        const int32_t rc = poddrop_create_playlist(
            mp.constData(),
            nameB.constData(),
            res.trackIds.isEmpty() ? nullptr : res.trackIds.constData(),
            static_cast<size_t>(res.trackIds.size()),
            &newId,
            &err
        );
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg, newId]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    if (newId != 0) m_selectedPlaylistId = newId;
                    m_statusMsg = trText(m_lang, "lbl.playlist_created");
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::editSelectedPlaylist() {
    const auto plOpt = selectedPlaylist();
    if (!plOpt.has_value()) return;
    const bool canWrite = m_hasDevice && !m_device.isReadOnly && !m_busy && !plOpt->isSmart;

    const auto resOpt = runPlaylistEditorDialog(
        this,
        m_lang,
        plOpt->name,
        plOpt->trackIds,
        m_tracks,
        false,
        canWrite
    );
    if (!resOpt.has_value() || !canWrite) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const uint64_t plId = plOpt->id;
    const PlaylistEditResult res = *resOpt;

    std::thread([this, mp, plId, res]() {
        const QByteArray nameB = res.name.toUtf8();
        char *err = nullptr;
        int32_t rc = poddrop_update_playlist(
            mp.constData(),
            plId,
            nameB.constData(),
            res.trackIds.isEmpty() ? nullptr : res.trackIds.constData(),
            static_cast<size_t>(res.trackIds.size()),
            1,
            &err
        );
        QString errMsg = takeCString(err);

        if (rc == 0 && !res.deleteTrackIdsFromIpod.isEmpty()) {
            for (uint32_t tid : res.deleteTrackIdsFromIpod) {
                char *delErr = nullptr;
                if (poddrop_remove_track(mp.constData(), static_cast<uint64_t>(tid), &delErr) != 0) {
                    rc = -1;
                    if (errMsg.isEmpty()) {
                        errMsg = takeCString(delErr);
                    } else {
                        poddrop_free_string(delErr);
                    }
                }
            }
        }

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                m_statusMsg = (rc == 0) ? trText(m_lang, "lbl.playlist_updated") : errMsg;
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::removeSelectedPlaylist() {
    const auto plOpt = selectedPlaylist();
    if (!plOpt.has_value() || !m_hasDevice || m_device.isReadOnly || m_busy) return;

    const auto reply = QMessageBox::question(
        this,
        trText(m_lang, "act.remove_playlist"),
        trText(m_lang, "dlg.remove_playlist_confirm").arg(plOpt->name)
    );
    if (reply != QMessageBox::Yes) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const uint64_t plId = plOpt->id;

    std::thread([this, mp, plId]() {
        char *err = nullptr;
        const int32_t rc = poddrop_remove_playlist(mp.constData(), plId, &err);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    m_selectedPlaylistId = 0;
                    m_statusMsg = trText(m_lang, "lbl.playlist_removed");
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::addTracksToPlaylistPrompt(const QVector<uint32_t> &trackIds) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy || trackIds.isEmpty()) return;
    const auto pickOpt = runPickPlaylistDialog(this, m_lang, m_playlists);
    if (!pickOpt.has_value()) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const PlaylistPickResult pick = *pickOpt;

    std::thread([this, mp, pick, trackIds]() {
        char *err = nullptr;
        int32_t rc = 0;
        uint32_t added = 0;
        uint64_t targetId = pick.playlistId;

        if (pick.createNew) {
            const QByteArray nameB = pick.newName.toUtf8();
            rc = poddrop_create_playlist(
                mp.constData(),
                nameB.constData(),
                trackIds.constData(),
                static_cast<size_t>(trackIds.size()),
                &targetId,
                &err
            );
            added = static_cast<uint32_t>(trackIds.size());
        } else {
            rc = poddrop_add_tracks_to_playlist(
                mp.constData(),
                targetId,
                trackIds.constData(),
                static_cast<size_t>(trackIds.size()),
                &added,
                &err
            );
        }
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg, added, pick, targetId]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    if (targetId != 0) m_selectedPlaylistId = targetId;
                    m_statusMsg = pick.createNew
                        ? trText(m_lang, "lbl.playlist_created")
                        : trText(m_lang, "lbl.playlist_tracks_added").arg(added);
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}
