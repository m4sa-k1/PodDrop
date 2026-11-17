#pragma once

#include <QDialog>
#include <QPixmap>
#include <QString>
#include <QStringList>
#include <QVector>
#include <cstdint>
#include <optional>

#include "i18n.hpp"

struct TrackItem {
    uint64_t id = 0;
    QString title;
    QString artist;
    QString albumArtist;
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

struct AlbumInfoSummary {
    QString album;
    QString artist;
    QString genre;
    uint32_t year = 0;
    int trackCount = 0;
    uint64_t totalDurationMs = 0;
    uint64_t totalSizeBytes = 0;
    bool hasArtwork = false;
};

struct AlbumEditResult {
    QString newAlbum;
    QString newArtist;
    QString newGenre;
    int32_t year = -1;
    QString coverPath;
    bool clearArtwork = false;
    bool applyArtistToTracks = false;
    bool applyGenreToTracks = false;
    bool applyYearToTracks = false;
    bool applyArtworkToTracks = true;
};

QPixmap loadTrackCoverPixmap(const QString &mountPath, uint64_t trackId);

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
    const QPixmap &currentCover,
    bool canWrite
);

std::optional<AlbumEditResult> runAlbumEditorDialog(
    QWidget *parent,
    UiLang lang,
    const AlbumInfoSummary &albumInfo,
    const QPixmap &currentCover,
    bool canWrite
);

/// Returns std::nullopt if cancelled, false for album-only removal, true for deleting tracks too.
std::optional<bool> runRemoveAlbumDialog(
    QWidget *parent,
    UiLang lang,
    const QString &albumName,
    int trackCount
);

struct PlaylistItem {
    uint64_t id = 0;
    QString name;
    bool isSmart = false;
    QVector<uint32_t> trackIds;
};

struct PlaylistEditResult {
    QString name;
    QVector<uint32_t> trackIds;
    QVector<uint32_t> deleteTrackIdsFromIpod;
};

struct PlaylistPickResult {
    bool createNew = false;
    uint64_t playlistId = 0;
    QString newName;
};

std::optional<PlaylistEditResult> runPlaylistEditorDialog(
    QWidget *parent,
    UiLang lang,
    const QString &initialName,
    const QVector<uint32_t> &initialTrackIds,
    const QVector<TrackItem> &allTracks,
    bool isNew,
    bool canWrite
);

std::optional<PlaylistPickResult> runPickPlaylistDialog(
    QWidget *parent,
    UiLang lang,
    const QVector<PlaylistItem> &playlists
);

void showSystemInfoDialog(QWidget *parent, UiLang lang);
void showTermsDialog(QWidget *parent, UiLang lang);
