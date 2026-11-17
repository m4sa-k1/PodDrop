#pragma once

#include <QDialog>
#include <QString>
#include <QStringList>
#include <cstdint>
#include <optional>

#include "i18n.hpp"

struct TrackItem {
    uint64_t id = 0;
    QString title;
    QString artist;
    QString album;
    QString genre;
    uint32_t trackNumber = 0;
    uint32_t discNumber = 0;
    uint32_t year = 0;
    uint32_t durationMs = 0;
    uint32_t bitrateKbps = 0;
    uint32_t sizeBytes = 0;
    bool hasArtwork = false;
};

struct AlbumBatchInput {
    QString album;
    QString artist;
    QString coverPath;
    QStringList audioFiles;
};

struct TrackEditResult {
    QString title;
    QString artist;
    QString album;
    QString genre;
    int32_t trackNumber = -1;
    int32_t discNumber = -1;
    int32_t year = -1;
    QString coverPath;
    bool clearArtwork = false;
};

std::optional<AlbumBatchInput> runCreateAlbumDialog(
    QWidget *parent,
    UiLang lang,
    const QString &initialAlbum = QString(),
    const QString &initialArtist = QString()
);

std::optional<TrackEditResult> runTrackEditorDialog(
    QWidget *parent,
    UiLang lang,
    const TrackItem &track,
    bool canWrite
);
