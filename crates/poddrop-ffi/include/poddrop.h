#ifndef PODDROP_H
#define PODDROP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    char *mount_path;
    char *model_name;
    char *model_number;
    double capacity_gb;
    char *filesystem;
    int32_t is_read_only;
    int32_t has_sysinfo_extended;
    char *firewire_guid;
    int32_t supports_artwork;
    uint32_t track_count;
    uint32_t playlist_count;
    uint64_t storage_total_bytes;
    uint64_t storage_free_bytes;
    int32_t has_storage_stats;
} PodDropDeviceInfo;

typedef struct {
    uint64_t id;
    char *title;
    char *artist;
    char *album_artist;
    char *album;
    char *genre;
    uint32_t track_number;
    uint32_t disc_number;
    uint32_t year;
    uint32_t duration_ms;
    uint32_t bitrate_kbps;
    uint32_t size_bytes;
    int32_t has_artwork;
} PodDropTrack;

/* format: 0 = AAC, 1 = ALAC, 2 = MP3 */
typedef struct {
    const char *audio_path;
    const char *artwork_path;
    int32_t transcode_format;
    const char *title_override;
    const char *artist_override;
    const char *album_override;
} PodDropTransferRequest;

typedef struct {
    const char *title;
    const char *artist;
    const char *album;
    const char *genre;
    int32_t track_number; /* -1 to skip */
    int32_t disc_number;  /* -1 to skip */
    int32_t year;         /* -1 to skip */
    const char *artwork_path;
    int32_t clear_artwork;
} PodDropUpdateRequest;

typedef struct {
    const char *old_album;
    const char *new_album;
    const char *new_artist;
    const char *new_genre;
    int32_t year; /* -1 to skip */
    const char *artwork_path;
    int32_t clear_artwork;
    int32_t apply_artist_to_tracks;
    int32_t apply_genre_to_tracks;
    int32_t apply_year_to_tracks;
    int32_t apply_artwork_to_tracks;
} PodDropUpdateAlbumRequest;

typedef struct {
    char *app_name;
    char *app_version;
    char *ffmpeg_version;
    char *ffprobe_version;
    char *udisksctl_status;
    char *backup_dir;
    char *last_backup_time;
    uint64_t last_backup_size_bytes;
} PodDropSystemInfo;

typedef struct {
    uint64_t id;
    char *name;
    int32_t is_smart;
    uint32_t *track_ids;
    size_t track_count;
} PodDropPlaylist;

const char *poddrop_app_version(void);
char *poddrop_discover_first_ipod(void);
int32_t poddrop_inspect_device(const char *mount_path, PodDropDeviceInfo *out_info, char **out_err);
int32_t poddrop_list_tracks(const char *mount_path, PodDropTrack **out_tracks, size_t *out_count, char **out_err);
int32_t poddrop_transfer_track(
    const char *mount_path,
    const PodDropTransferRequest *req,
    int32_t create_backup,
    uint64_t *out_track_id,
    char **out_err
);
int32_t poddrop_update_track(
    const char *mount_path,
    uint64_t track_id,
    const PodDropUpdateRequest *req,
    char **out_err
);
int32_t poddrop_update_album(
    const char *mount_path,
    const PodDropUpdateAlbumRequest *req,
    uint32_t *out_updated_count,
    char **out_err
);
int32_t poddrop_remove_album(
    const char *mount_path,
    const char *album_name,
    int32_t delete_tracks,
    uint32_t *out_affected_count,
    char **out_err
);
int32_t poddrop_remove_track(const char *mount_path, uint64_t track_id, char **out_err);
int32_t poddrop_get_track_artwork_png(
    const char *mount_path,
    uint64_t track_id,
    uint8_t **out_data,
    size_t *out_len,
    char **out_err
);
int32_t poddrop_eject_device(const char *mount_path, char **out_err);
int32_t poddrop_validate_audio(
    const char *audio_path,
    uint64_t *out_size_bytes,
    uint32_t *out_duration_ms,
    char **out_err
);
void poddrop_get_system_info(PodDropSystemInfo *out_info);
void poddrop_free_system_info(PodDropSystemInfo *info);
int32_t poddrop_create_backup(const char *mount_path, char **out_backup_time, char **out_err);
int32_t poddrop_restore_latest_backup(const char *mount_path, char **out_restored_time, char **out_err);

void poddrop_free_string(char *s);
void poddrop_free_bytes(uint8_t *data, size_t len);
void poddrop_free_device_info(PodDropDeviceInfo *info);
void poddrop_free_tracks(PodDropTrack *tracks, size_t count);

int32_t poddrop_list_playlists(
    const char *mount_path,
    PodDropPlaylist **out_playlists,
    size_t *out_count,
    char **out_err
);
void poddrop_free_playlists(PodDropPlaylist *playlists, size_t count);
int32_t poddrop_create_playlist(
    const char *mount_path,
    const char *name,
    const uint32_t *track_ids,
    size_t track_count,
    uint64_t *out_playlist_id,
    char **out_err
);
int32_t poddrop_update_playlist(
    const char *mount_path,
    uint64_t playlist_id,
    const char *new_name,
    const uint32_t *track_ids,
    size_t track_count,
    int32_t replace_tracks,
    char **out_err
);
int32_t poddrop_add_tracks_to_playlist(
    const char *mount_path,
    uint64_t playlist_id,
    const uint32_t *track_ids,
    size_t track_count,
    uint32_t *out_added_count,
    char **out_err
);
int32_t poddrop_remove_playlist(const char *mount_path, uint64_t playlist_id, char **out_err);

#ifdef __cplusplus
}
#endif

#endif
