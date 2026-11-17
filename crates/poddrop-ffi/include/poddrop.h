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
} PodDropUpdateAlbumRequest;

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
int32_t poddrop_remove_track(const char *mount_path, uint64_t track_id, char **out_err);
int32_t poddrop_get_track_artwork_png(
    const char *mount_path,
    uint64_t track_id,
    uint8_t **out_data,
    size_t *out_len,
    char **out_err
);
int32_t poddrop_eject_device(const char *mount_path, char **out_err);

void poddrop_free_string(char *s);
void poddrop_free_bytes(uint8_t *data, size_t len);
void poddrop_free_device_info(PodDropDeviceInfo *info);
void poddrop_free_tracks(PodDropTrack *tracks, size_t count);

#ifdef __cplusplus
}
#endif

#endif
