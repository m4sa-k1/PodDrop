#include "dialogs.hpp"

#include <QDialogButtonBox>
#include <QFileDialog>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QIntValidator>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QVBoxLayout>

#include "poddrop.h"

namespace {

QLabel *makeCoverPreviewLabel(QWidget *parent, UiLang lang) {
    auto *lbl = new QLabel(parent);
    lbl->setFixedSize(120, 120);
    lbl->setAlignment(Qt::AlignCenter);
    lbl->setFrameStyle(QFrame::Box | QFrame::Plain);
    lbl->setText(trText(lang, "lbl.no_art"));
    return lbl;
}

void setCoverPreview(QLabel *lbl, const QPixmap &pm, UiLang lang) {
    if (pm.isNull()) {
        lbl->setPixmap(QPixmap());
        lbl->setText(trText(lang, "lbl.no_art"));
    } else {
        lbl->setText(QString());
        lbl->setPixmap(pm.scaled(116, 116, Qt::KeepAspectRatio, Qt::SmoothTransformation));
    }
}

} // namespace

QPixmap loadTrackCoverPixmap(const QString &mountPath, uint64_t trackId) {
    if (mountPath.isEmpty() || trackId == 0) {
        return QPixmap();
    }
    const QByteArray mp = mountPath.toUtf8();
    uint8_t *data = nullptr;
    size_t len = 0;
    char *err = nullptr;
    if (poddrop_get_track_artwork_png(mp.constData(), trackId, &data, &len, &err) != 0) {
        if (err) poddrop_free_string(err);
        return QPixmap();
    }
    if (err) poddrop_free_string(err);

    QPixmap pm;
    if (data && len > 0) {
        pm.loadFromData(data, static_cast<uint>(len), "PNG");
        poddrop_free_bytes(data, len);
    }
    return pm;
}

std::optional<AlbumBatchInput> runCreateAlbumDialog(
    QWidget *parent,
    UiLang lang,
    const QString &initialAlbum,
    const QString &initialArtist
) {
    QDialog dlg(parent);
    dlg.setWindowTitle(trText(lang, "act.create_album"));
    dlg.setMinimumWidth(520);

    auto *previewLbl = makeCoverPreviewLabel(&dlg, lang);
    auto *albumEdit = new QLineEdit(initialAlbum, &dlg);
    auto *artistEdit = new QLineEdit(initialArtist, &dlg);

    auto *coverEdit = new QLineEdit(&dlg);
    coverEdit->setReadOnly(true);
    auto *browseBtn = new QPushButton(trText(lang, "dlg.browse"), &dlg);
    auto *clearBtn = new QPushButton(trText(lang, "dlg.clear"), &dlg);

    auto *coverRow = new QHBoxLayout();
    coverRow->addWidget(coverEdit, 1);
    coverRow->addWidget(browseBtn);
    coverRow->addWidget(clearBtn);

    auto *form = new QFormLayout();
    form->addRow(trText(lang, "dlg.album_name"), albumEdit);
    form->addRow(trText(lang, "dlg.artist_name"), artistEdit);
    form->addRow(trText(lang, "dlg.cover"), coverRow);

    auto *topRow = new QHBoxLayout();
    topRow->addWidget(previewLbl, 0, Qt::AlignTop);
    topRow->addLayout(form, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Cancel, &dlg);
    auto *selectBtn = buttons->addButton(trText(lang, "dlg.select_files"), QDialogButtonBox::AcceptRole);
    selectBtn->setDefault(true);

    auto *root = new QVBoxLayout(&dlg);
    root->addLayout(topRow);
    root->addWidget(buttons);

    QObject::connect(browseBtn, &QPushButton::clicked, &dlg, [&]() {
        const QString file = QFileDialog::getOpenFileName(
            &dlg,
            trText(lang, "dlg.cover"),
            QString(),
            "Images (*.png *.jpg *.jpeg)"
        );
        if (!file.isEmpty()) {
            coverEdit->setText(file);
            setCoverPreview(previewLbl, QPixmap(file), lang);
        }
    });

    QObject::connect(clearBtn, &QPushButton::clicked, &dlg, [&]() {
        coverEdit->clear();
        setCoverPreview(previewLbl, QPixmap(), lang);
    });

    QStringList chosenFiles;
    QObject::connect(selectBtn, &QPushButton::clicked, &dlg, [&]() {
        const QStringList files = QFileDialog::getOpenFileNames(
            &dlg,
            trText(lang, "dlg.select_files"),
            QString(),
            "Audio (*.mp3 *.m4a *.mp4 *.flac *.ogg *.opus *.wav)"
        );
        if (!files.isEmpty()) {
            chosenFiles = files;
            dlg.accept();
        }
    });
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);

    if (dlg.exec() != QDialog::Accepted || chosenFiles.isEmpty()) {
        return std::nullopt;
    }

    return AlbumBatchInput{
        albumEdit->text().trimmed(),
        artistEdit->text().trimmed(),
        coverEdit->text().trimmed(),
        chosenFiles,
    };
}

std::optional<TrackEditResult> runTrackEditorDialog(
    QWidget *parent,
    UiLang lang,
    const TrackItem &track,
    const QPixmap &currentCover,
    bool canWrite
) {
    QDialog dlg(parent);
    dlg.setWindowTitle(track.title);
    dlg.setMinimumWidth(560);

    auto *previewLbl = makeCoverPreviewLabel(&dlg, lang);
    setCoverPreview(previewLbl, currentCover, lang);

    auto *titleEdit = new QLineEdit(track.title, &dlg);
    auto *artistEdit = new QLineEdit(track.artist, &dlg);
    auto *albumEdit = new QLineEdit(track.album, &dlg);
    auto *genreEdit = new QLineEdit(track.genre, &dlg);

    auto *trackNrEdit = new QLineEdit(track.trackNumber > 0 ? QString::number(track.trackNumber) : QString(), &dlg);
    auto *discNrEdit = new QLineEdit(track.discNumber > 0 ? QString::number(track.discNumber) : QString(), &dlg);
    auto *yearEdit = new QLineEdit(track.year > 0 ? QString::number(track.year) : QString(), &dlg);
    trackNrEdit->setValidator(new QIntValidator(0, 9999, trackNrEdit));
    discNrEdit->setValidator(new QIntValidator(0, 999, discNrEdit));
    yearEdit->setValidator(new QIntValidator(0, 9999, yearEdit));

    for (auto *e : {titleEdit, artistEdit, albumEdit, genreEdit, trackNrEdit, discNrEdit, yearEdit}) {
        e->setReadOnly(!canWrite);
    }

    auto *numRow = new QHBoxLayout();
    numRow->addWidget(trackNrEdit);
    numRow->addWidget(discNrEdit);
    numRow->addWidget(yearEdit);

    bool clearArtwork = false;
    QString selectedCover;
    auto *coverEdit = new QLineEdit(
        track.hasArtwork ? trText(lang, "dlg.cover_keep") : QStringLiteral("-"),
        &dlg
    );
    coverEdit->setReadOnly(true);
    auto *browseBtn = new QPushButton(trText(lang, "dlg.browse"), &dlg);
    auto *removeCoverBtn = new QPushButton(trText(lang, "dlg.cover_remove"), &dlg);
    browseBtn->setEnabled(canWrite);
    removeCoverBtn->setEnabled(canWrite);

    auto *coverRow = new QHBoxLayout();
    coverRow->addWidget(coverEdit, 1);
    coverRow->addWidget(browseBtn);
    coverRow->addWidget(removeCoverBtn);

    const uint32_t sec = track.durationMs / 1000;
    const double sizeMb = static_cast<double>(track.sizeBytes) / (1024.0 * 1024.0);
    auto *detailsLbl = new QLabel(
        QString("%1:%2  |  %3 kbps  |  %4 MB  |  ID %5")
            .arg(sec / 60)
            .arg(sec % 60, 2, 10, QChar('0'))
            .arg(track.bitrateKbps)
            .arg(sizeMb, 0, 'f', 2)
            .arg(track.id),
        &dlg
    );

    auto *form = new QFormLayout();
    form->addRow(trText(lang, "dlg.title"), titleEdit);
    form->addRow(trText(lang, "dlg.artist_name"), artistEdit);
    form->addRow(trText(lang, "dlg.album_name"), albumEdit);
    form->addRow(trText(lang, "dlg.genre"), genreEdit);
    form->addRow(
        QString("%1 / %2 / %3")
            .arg(trText(lang, "dlg.track_num"), trText(lang, "dlg.disc_num"), trText(lang, "dlg.year")),
        numRow
    );
    form->addRow(trText(lang, "dlg.cover"), coverRow);
    form->addRow(trText(lang, "dlg.details"), detailsLbl);

    auto *topRow = new QHBoxLayout();
    topRow->addWidget(previewLbl, 0, Qt::AlignTop);
    topRow->addLayout(form, 1);

    auto *buttons = new QDialogButtonBox(
        canWrite ? (QDialogButtonBox::Ok | QDialogButtonBox::Cancel) : QDialogButtonBox::Close,
        &dlg
    );

    auto *root = new QVBoxLayout(&dlg);
    root->addLayout(topRow);
    root->addWidget(buttons);

    QObject::connect(browseBtn, &QPushButton::clicked, &dlg, [&]() {
        const QString file = QFileDialog::getOpenFileName(
            &dlg,
            trText(lang, "dlg.cover"),
            QString(),
            "Images (*.png *.jpg *.jpeg)"
        );
        if (!file.isEmpty()) {
            selectedCover = file;
            clearArtwork = false;
            coverEdit->setText(file);
            setCoverPreview(previewLbl, QPixmap(file), lang);
        }
    });

    QObject::connect(removeCoverBtn, &QPushButton::clicked, &dlg, [&]() {
        selectedCover.clear();
        clearArtwork = true;
        coverEdit->setText(trText(lang, "dlg.cover_remove"));
        setCoverPreview(previewLbl, QPixmap(), lang);
    });

    QObject::connect(buttons, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);

    if (dlg.exec() != QDialog::Accepted || !canWrite) {
        return std::nullopt;
    }

    auto parseOrZero = [](const QString &s) -> int32_t {
        bool ok = false;
        const int v = s.trimmed().toInt(&ok);
        return ok ? v : 0;
    };

    return TrackEditResult{
        titleEdit->text(),
        artistEdit->text(),
        albumEdit->text(),
        genreEdit->text(),
        parseOrZero(trackNrEdit->text()),
        parseOrZero(discNrEdit->text()),
        parseOrZero(yearEdit->text()),
        selectedCover,
        clearArtwork,
    };
}

std::optional<AlbumEditResult> runAlbumEditorDialog(
    QWidget *parent,
    UiLang lang,
    const AlbumInfoSummary &albumInfo,
    const QPixmap &currentCover,
    bool canWrite
) {
    QDialog dlg(parent);
    dlg.setWindowTitle(albumInfo.album);
    dlg.setMinimumWidth(540);

    auto *previewLbl = makeCoverPreviewLabel(&dlg, lang);
    setCoverPreview(previewLbl, currentCover, lang);

    auto *albumEdit = new QLineEdit(albumInfo.album, &dlg);
    auto *artistEdit = new QLineEdit(albumInfo.artist, &dlg);
    auto *genreEdit = new QLineEdit(albumInfo.genre, &dlg);
    auto *yearEdit = new QLineEdit(albumInfo.year > 0 ? QString::number(albumInfo.year) : QString(), &dlg);
    yearEdit->setValidator(new QIntValidator(0, 9999, yearEdit));

    for (auto *e : {albumEdit, artistEdit, genreEdit, yearEdit}) {
        e->setReadOnly(!canWrite);
    }

    bool clearArtwork = false;
    QString selectedCover;
    auto *coverEdit = new QLineEdit(
        albumInfo.hasArtwork ? trText(lang, "dlg.cover_keep") : QStringLiteral("-"),
        &dlg
    );
    coverEdit->setReadOnly(true);
    auto *browseBtn = new QPushButton(trText(lang, "dlg.browse"), &dlg);
    auto *removeCoverBtn = new QPushButton(trText(lang, "dlg.cover_remove"), &dlg);
    browseBtn->setEnabled(canWrite);
    removeCoverBtn->setEnabled(canWrite);

    auto *coverRow = new QHBoxLayout();
    coverRow->addWidget(coverEdit, 1);
    coverRow->addWidget(browseBtn);
    coverRow->addWidget(removeCoverBtn);

    const uint64_t sec = albumInfo.totalDurationMs / 1000;
    const double sizeMb = static_cast<double>(albumInfo.totalSizeBytes) / (1024.0 * 1024.0);
    auto *detailsLbl = new QLabel(
        QString("%1 tracks  |  %2:%3  |  %4 MB")
            .arg(albumInfo.trackCount)
            .arg(sec / 60)
            .arg(sec % 60, 2, 10, QChar('0'))
            .arg(sizeMb, 0, 'f', 2),
        &dlg
    );

    auto *form = new QFormLayout();
    form->addRow(trText(lang, "dlg.album_name"), albumEdit);
    form->addRow(trText(lang, "dlg.artist_name"), artistEdit);
    form->addRow(trText(lang, "dlg.genre"), genreEdit);
    form->addRow(trText(lang, "dlg.year"), yearEdit);
    form->addRow(trText(lang, "dlg.cover"), coverRow);
    form->addRow(trText(lang, "dlg.details"), detailsLbl);

    auto *topRow = new QHBoxLayout();
    topRow->addWidget(previewLbl, 0, Qt::AlignTop);
    topRow->addLayout(form, 1);

    auto *buttons = new QDialogButtonBox(
        canWrite ? (QDialogButtonBox::Ok | QDialogButtonBox::Cancel) : QDialogButtonBox::Close,
        &dlg
    );

    auto *root = new QVBoxLayout(&dlg);
    root->addLayout(topRow);
    root->addWidget(buttons);

    QObject::connect(browseBtn, &QPushButton::clicked, &dlg, [&]() {
        const QString file = QFileDialog::getOpenFileName(
            &dlg,
            trText(lang, "dlg.cover"),
            QString(),
            "Images (*.png *.jpg *.jpeg)"
        );
        if (!file.isEmpty()) {
            selectedCover = file;
            clearArtwork = false;
            coverEdit->setText(file);
            setCoverPreview(previewLbl, QPixmap(file), lang);
        }
    });

    QObject::connect(removeCoverBtn, &QPushButton::clicked, &dlg, [&]() {
        selectedCover.clear();
        clearArtwork = true;
        coverEdit->setText(trText(lang, "dlg.cover_remove"));
        setCoverPreview(previewLbl, QPixmap(), lang);
    });

    QObject::connect(buttons, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);

    if (dlg.exec() != QDialog::Accepted || !canWrite) {
        return std::nullopt;
    }

    bool okYear = false;
    const int yr = yearEdit->text().trimmed().toInt(&okYear);

    return AlbumEditResult{
        albumEdit->text(),
        artistEdit->text(),
        genreEdit->text(),
        okYear ? yr : 0,
        selectedCover,
        clearArtwork,
    };
}
