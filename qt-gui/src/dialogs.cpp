#include "dialogs.hpp"

#include <QCheckBox>
#include <QDialogButtonBox>
#include <QFileDialog>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QIntValidator>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QRadioButton>
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
    auto *applyArtistChk = new QCheckBox(trText(lang, "dlg.apply_artist_tracks"), &dlg);
    applyArtistChk->setChecked(false);
    applyArtistChk->setEnabled(canWrite);

    auto *genreEdit = new QLineEdit(albumInfo.genre, &dlg);
    auto *applyGenreChk = new QCheckBox(trText(lang, "dlg.apply_genre_tracks"), &dlg);
    applyGenreChk->setChecked(false);
    applyGenreChk->setEnabled(canWrite);

    auto *yearEdit = new QLineEdit(albumInfo.year > 0 ? QString::number(albumInfo.year) : QString(), &dlg);
    yearEdit->setValidator(new QIntValidator(0, 9999, yearEdit));
    auto *applyYearChk = new QCheckBox(trText(lang, "dlg.apply_year_tracks"), &dlg);
    applyYearChk->setChecked(false);
    applyYearChk->setEnabled(canWrite);

    for (auto *e : {albumEdit, artistEdit, genreEdit, yearEdit}) {
        e->setReadOnly(!canWrite);
    }

    auto *artistBox = new QVBoxLayout();
    artistBox->setSpacing(3);
    artistBox->addWidget(artistEdit);
    artistBox->addWidget(applyArtistChk);

    auto *genreRow = new QHBoxLayout();
    genreRow->addWidget(genreEdit, 1);
    genreRow->addWidget(applyGenreChk);

    auto *yearRow = new QHBoxLayout();
    yearRow->addWidget(yearEdit, 1);
    yearRow->addWidget(applyYearChk);

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

    auto *applyCoverChk = new QCheckBox(trText(lang, "dlg.apply_cover_tracks"), &dlg);
    applyCoverChk->setChecked(true);
    applyCoverChk->setEnabled(canWrite);

    auto *coverRow = new QHBoxLayout();
    coverRow->addWidget(coverEdit, 1);
    coverRow->addWidget(browseBtn);
    coverRow->addWidget(removeCoverBtn);

    auto *coverBox = new QVBoxLayout();
    coverBox->setSpacing(3);
    coverBox->addLayout(coverRow);
    coverBox->addWidget(applyCoverChk);

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
    form->addRow(trText(lang, "dlg.artist_name"), artistBox);
    form->addRow(trText(lang, "dlg.genre"), genreRow);
    form->addRow(trText(lang, "dlg.year"), yearRow);
    form->addRow(trText(lang, "dlg.cover"), coverBox);
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
        applyArtistChk->isChecked(),
        applyGenreChk->isChecked(),
        applyYearChk->isChecked(),
        applyCoverChk->isChecked(),
    };
}

void showSystemInfoDialog(QWidget *parent, UiLang lang) {
    PodDropSystemInfo info{};
    poddrop_get_system_info(&info);

    auto take = [](char *p) -> QString {
        if (!p) return QStringLiteral("-");
        return QString::fromUtf8(p);
    };

    QDialog dlg(parent);
    dlg.setWindowTitle(trText(lang, "act.sysinfo"));
    dlg.setMinimumWidth(520);

    auto *form = new QFormLayout();
    auto addReadOnlyRow = [&](const QString &label, const QString &val) {
        auto *lbl = new QLabel(val, &dlg);
        lbl->setTextInteractionFlags(Qt::TextSelectableByMouse);
        lbl->setWordWrap(true);
        form->addRow(label, lbl);
    };

    addReadOnlyRow(
        trText(lang, "sys.version"),
        QString("%1 v%2").arg(take(info.app_name), take(info.app_version))
    );
    addReadOnlyRow(trText(lang, "sys.license"), QStringLiteral("GPL-3.0-or-later"));
    addReadOnlyRow(trText(lang, "sys.qt"), QString::fromUtf8(qVersion()));
    addReadOnlyRow(QStringLiteral("ffmpeg:"), take(info.ffmpeg_version));
    addReadOnlyRow(QStringLiteral("ffprobe:"), take(info.ffprobe_version));
    addReadOnlyRow(QStringLiteral("udisksctl:"), take(info.udisksctl_status));
    addReadOnlyRow(trText(lang, "sys.backup_dir"), take(info.backup_dir));
    addReadOnlyRow(trText(lang, "sys.repo"), QStringLiteral("https://github.com/m4sa-k1/PodDrop"));

    poddrop_free_system_info(&info);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dlg);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);

    auto *root = new QVBoxLayout(&dlg);
    root->addLayout(form);
    root->addWidget(buttons);
    dlg.exec();
}

std::optional<bool> runRemoveAlbumDialog(
    QWidget *parent,
    UiLang lang,
    const QString &albumName,
    int trackCount
) {
    QDialog dlg(parent);
    dlg.setWindowTitle(trText(lang, "act.remove_album"));
    dlg.setMinimumWidth(420);

    auto *msgLbl = new QLabel(
        trText(lang, "dlg.remove_album_msg").arg(albumName).arg(trackCount),
        &dlg
    );
    msgLbl->setWordWrap(true);

    auto *rbAlbumOnly = new QRadioButton(trText(lang, "dlg.remove_album_only"), &dlg);
    auto *rbWithTracks = new QRadioButton(trText(lang, "dlg.remove_album_with_tracks"), &dlg);
    rbAlbumOnly->setChecked(true);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dlg);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);

    auto *root = new QVBoxLayout(&dlg);
    root->addWidget(msgLbl);
    root->addWidget(rbAlbumOnly);
    root->addWidget(rbWithTracks);
    root->addWidget(buttons);

    if (dlg.exec() != QDialog::Accepted) {
        return std::nullopt;
    }
    return rbWithTracks->isChecked();
}
