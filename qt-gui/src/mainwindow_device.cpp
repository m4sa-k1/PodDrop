#include "mainwindow.hpp"
#include "mainwindow_helpers.hpp"

#include <QApplication>
#include <QFileDialog>
#include <QMessageBox>
#include <thread>

void MainWindow::openMountFolder() {
    const QString dir = QFileDialog::getExistingDirectory(this, trText(m_lang, "act.open"), m_mountPath);
    if (!dir.isEmpty()) {
        m_mountPath = dir;
        m_statusMsg.clear();
        reloadDevice();
    }
}

void MainWindow::ejectDevice() {
    if (!m_hasDevice || m_busy || m_mountPath.isEmpty()) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    std::thread([this, mp]() {
        char *err = nullptr;
        const int32_t rc = poddrop_eject_device(mp.constData(), &err);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    m_mountPath.clear();
                    m_backedUpMountPath.clear();
                    m_selectedAlbum.clear();
                    m_statusMsg = trText(m_lang, "lbl.ejected");
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::restoreFromBackup() {
    if (!m_hasDevice || m_device.isReadOnly || m_busy || m_mountPath.isEmpty()) return;

    const QByteArray mp = m_mountPath.toUtf8();
    char *rawTime = nullptr;
    uint64_t sizeBytes = 0;
    uint32_t devCount = 0;
    poddrop_get_device_backup_info(mp.constData(), &rawTime, &sizeBytes, &devCount);
    const QString lastTime = takeCString(rawTime);

    if (lastTime.isEmpty()) {
        QMessageBox::information(
            this,
            trText(m_lang, "act.restore_backup"),
            trText(m_lang, "sys.no_backup")
        );
        return;
    }

    const double mb = static_cast<double>(sizeBytes) / (1024.0 * 1024.0);
    const auto reply = QMessageBox::question(
        this,
        trText(m_lang, "act.restore_backup"),
        trText(m_lang, "dlg.restore_confirm").arg(lastTime).arg(mb, 0, 'f', 2)
    );
    if (reply != QMessageBox::Yes) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    std::thread([this, mp]() {
        char *restoredTime = nullptr;
        char *err = nullptr;
        const int32_t rc = poddrop_restore_latest_backup(mp.constData(), &restoredTime, &err);
        const QString ts = takeCString(restoredTime);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, ts, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                m_statusMsg = (rc == 0)
                    ? trText(m_lang, "lbl.restore_done").arg(ts)
                    : errMsg;
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::exportSelectedTracks() {
    if (!m_hasDevice || m_busy || m_tracks.isEmpty() || m_mountPath.isEmpty()) return;

    const QString destDir = QFileDialog::getExistingDirectory(
        this,
        trText(m_lang, "dlg.export_title")
    );
    if (destDir.isEmpty()) return;

    QVector<uint32_t> ids;
    const auto sel = selectedTracks();
    for (const auto &t : sel) {
        ids.push_back(static_cast<uint32_t>(t.id));
    }

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const QByteArray destB = destDir.toUtf8();

    std::thread([this, mp, ids, destB]() {
        uint32_t exported = 0;
        char *err = nullptr;
        const int32_t rc = poddrop_export_tracks(
            mp.constData(),
            ids.isEmpty() ? nullptr : ids.constData(),
            static_cast<size_t>(ids.size()),
            destB.constData(),
            &exported,
            &err
        );
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, exported, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                m_statusMsg = (rc == 0)
                    ? trText(m_lang, "lbl.export_done").arg(exported)
                    : errMsg;
                applyTranslations();
                updateActionStates();
            },
            Qt::QueuedConnection
        );
    }).detach();
}
