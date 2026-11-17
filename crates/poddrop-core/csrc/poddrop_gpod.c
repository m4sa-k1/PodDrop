#include <gpod/itdb.h>
#include <gdk-pixbuf/gdk-pixbuf.h>
#include <glib.h>
#include <errno.h>
#include <fcntl.h>
#include <scsi/sg.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/statvfs.h>
#include <unistd.h>

typedef struct {
    char *model_number;
    double capacity_gb;
    int32_t generation;
    int32_t supports_artwork;
    uint32_t track_count;
    uint32_t playlist_count;
    char *firewire_guid;
} PodDropGpodDeviceInfo;

typedef struct {
    uint64_t id;
    char *title;
    char *artist;
    char *album;
    char *genre;
    int32_t track_number;
    int32_t disc_number;
    int32_t year;
    int32_t duration_ms;
    int32_t bitrate_kbps;
    uint32_t size_bytes;
    int32_t has_artwork;
} PodDropGpodTrack;

static char *dup_cstr_or_empty(const char *s) {
    return g_strdup(s ? s : "");
}

void poddrop_gpod_free_string(char *s) {
    g_free(s);
}

void poddrop_gpod_free_device_info(PodDropGpodDeviceInfo *info) {
    if (!info) return;
    g_free(info->model_number);
    g_free(info->firewire_guid);
    info->model_number = NULL;
    info->firewire_guid = NULL;
}

void poddrop_gpod_free_tracks(PodDropGpodTrack *tracks, size_t count) {
    if (!tracks) return;
    for (size_t i = 0; i < count; ++i) {
        g_free(tracks[i].title);
        g_free(tracks[i].artist);
        g_free(tracks[i].album);
        g_free(tracks[i].genre);
    }
    g_free(tracks);
}

int poddrop_gpod_inspect(
    const char *mount_path,
    PodDropGpodDeviceInfo *out_info,
    char **out_err
) {
    if (!mount_path || !out_info) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }

    memset(out_info, 0, sizeof(*out_info));
    GError *err = NULL;
    Itdb_iTunesDB *itdb = itdb_parse(mount_path, &err);
    if (!itdb) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to parse iTunesDB");
        }
        if (err) g_error_free(err);
        return -1;
    }

    out_info->track_count = itdb_tracks_number(itdb);
    out_info->playlist_count = itdb_playlists_number(itdb);

    if (itdb->device) {
        const Itdb_IpodInfo *ipod_info = itdb_device_get_ipod_info(itdb->device);
        if (ipod_info) {
            out_info->model_number = dup_cstr_or_empty(ipod_info->model_number);
            out_info->capacity_gb = ipod_info->capacity;
            out_info->generation = (int32_t)ipod_info->ipod_generation;
        } else {
            out_info->model_number = g_strdup("");
            out_info->generation = 0;
        }
        out_info->supports_artwork = itdb_device_supports_artwork(itdb->device) ? 1 : 0;
        char *guid = itdb_device_get_sysinfo(itdb->device, "FirewireGuid");
        out_info->firewire_guid = guid;
    } else {
        out_info->model_number = g_strdup("");
    }

    itdb_free(itdb);
    return 0;
}

int poddrop_gpod_list_tracks(
    const char *mount_path,
    PodDropGpodTrack **out_tracks,
    size_t *out_count,
    char **out_err
) {
    if (!mount_path || !out_tracks || !out_count) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }

    *out_tracks = NULL;
    *out_count = 0;

    GError *err = NULL;
    Itdb_iTunesDB *itdb = itdb_parse(mount_path, &err);
    if (!itdb) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to parse iTunesDB");
        }
        if (err) g_error_free(err);
        return -1;
    }

    guint32 total = itdb_tracks_number(itdb);
    if (total == 0) {
        itdb_free(itdb);
        return 0;
    }

    PodDropGpodTrack *arr = g_new0(PodDropGpodTrack, total);
    size_t idx = 0;
    for (GList *l = itdb->tracks; l != NULL && idx < total; l = l->next) {
        Itdb_Track *tr = (Itdb_Track *)l->data;
        if (!tr) continue;
        arr[idx].id = (uint64_t)tr->id;
        arr[idx].title = dup_cstr_or_empty(tr->title);
        arr[idx].artist = dup_cstr_or_empty(tr->artist);
        arr[idx].album = dup_cstr_or_empty(tr->album);
        arr[idx].genre = dup_cstr_or_empty(tr->genre);
        arr[idx].track_number = tr->track_nr;
        arr[idx].disc_number = tr->cd_nr;
        arr[idx].year = tr->year;
        arr[idx].duration_ms = tr->tracklen;
        arr[idx].bitrate_kbps = tr->bitrate;
        arr[idx].size_bytes = tr->size;
        arr[idx].has_artwork = (tr->artwork && tr->artwork->artwork_size > 0) ? 1 : 0;
        idx++;
    }

    *out_tracks = arr;
    *out_count = idx;
    itdb_free(itdb);
    return 0;
}

static int scsi_vpd_inquiry(int fd, uint8_t page, uint8_t *buf, size_t buf_len, char **out_err) {
    uint8_t cdb[6] = {0x12, 0x01, page, 0x00, (uint8_t)(buf_len > 255 ? 255 : buf_len), 0x00};
    uint8_t sense[32] = {0};

    sg_io_hdr_t io_hdr;
    memset(&io_hdr, 0, sizeof(io_hdr));
    io_hdr.interface_id = 'S';
    io_hdr.dxfer_direction = SG_DXFER_FROM_DEV;
    io_hdr.cmd_len = sizeof(cdb);
    io_hdr.mx_sb_len = sizeof(sense);
    io_hdr.dxfer_len = (unsigned int)(buf_len > 255 ? 255 : buf_len);
    io_hdr.dxferp = buf;
    io_hdr.cmdp = cdb;
    io_hdr.sbp = sense;
    io_hdr.timeout = 5000;

    if (ioctl(fd, SG_IO, &io_hdr) < 0) {
        if (out_err) *out_err = g_strdup_printf("SG_IO ioctl failed: %s", g_strerror(errno));
        return -1;
    }
    if ((io_hdr.info & SG_INFO_OK_MASK) != SG_INFO_OK) {
        if (out_err) *out_err = g_strdup_printf("SCSI INQUIRY failed (status=%u)", io_hdr.status);
        return -1;
    }
    return 0;
}

int poddrop_read_sysinfo_vpd(const char *dev_path, char **out_xml, char **out_err) {
    if (!dev_path || !out_xml) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }
    *out_xml = NULL;

    int fd = open(dev_path, O_RDONLY | O_NONBLOCK);
    if (fd < 0) {
        if (out_err) *out_err = g_strdup_printf("Failed to open %s: %s", dev_path, g_strerror(errno));
        return -1;
    }

    uint8_t page_list[255] = {0};
    if (scsi_vpd_inquiry(fd, 0xC0, page_list, sizeof(page_list), out_err) != 0) {
        close(fd);
        return -1;
    }

    uint8_t list_len = page_list[3];
    GString *xml = g_string_new(NULL);
    for (size_t i = 0; i < list_len && (4 + i) < sizeof(page_list); ++i) {
        uint8_t page = page_list[4 + i];
        if (page < 0xC2) continue;

        uint8_t chunk[255] = {0};
        if (scsi_vpd_inquiry(fd, page, chunk, sizeof(chunk), out_err) != 0) {
            g_string_free(xml, TRUE);
            close(fd);
            return -1;
        }
        uint8_t chunk_len = chunk[3];
        if (chunk_len > 0 && (size_t)(4 + chunk_len) <= sizeof(chunk)) {
            g_string_append_len(xml, (const gchar *)(chunk + 4), chunk_len);
        }
    }

    close(fd);
    if (xml->len == 0) {
        g_string_free(xml, TRUE);
        if (out_err) *out_err = g_strdup("Empty SysInfoExtended VPD response");
        return -1;
    }

    *out_xml = g_string_free(xml, FALSE);
    return 0;
}

typedef struct {
    const char *audio_path;
    const char *artwork_path;
    const char *title;
    const char *artist;
    const char *album;
    int32_t track_number;
    int32_t disc_number;
    int32_t duration_ms;
    int32_t bitrate_kbps;
    uint16_t samplerate_hz;
} PodDropGpodAddTrackRequest;

int poddrop_gpod_add_track(
    const char *mount_path,
    const PodDropGpodAddTrackRequest *req,
    uint64_t *out_track_id,
    char **out_err
) {
    if (!mount_path || !req || !req->audio_path) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }

    GError *err = NULL;
    Itdb_iTunesDB *itdb = itdb_parse(mount_path, &err);
    if (!itdb) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to parse iTunesDB");
        }
        if (err) g_error_free(err);
        return -1;
    }

    Itdb_Track *track = itdb_track_new();
    track->title = dup_cstr_or_empty(req->title);
    track->artist = dup_cstr_or_empty(req->artist);
    track->album = dup_cstr_or_empty(req->album);
    track->track_nr = req->track_number;
    track->cd_nr = req->disc_number;
    track->tracklen = req->duration_ms;
    track->bitrate = req->bitrate_kbps;
    track->samplerate = req->samplerate_hz;
    track->mediatype = ITDB_MEDIATYPE_AUDIO;

    itdb_track_add(itdb, track, -1);
    Itdb_Playlist *mpl = itdb_playlist_mpl(itdb);
    if (mpl) {
        itdb_playlist_add_track(mpl, track, -1);
    }

    if (req->artwork_path && req->artwork_path[0] != '\0') {
        if (itdb->device && itdb_device_supports_artwork(itdb->device)) {
            if (!itdb_track_set_thumbnails(track, req->artwork_path)) {
                if (out_err) *out_err = g_strdup("Failed to set track artwork thumbnail");
                itdb_free(itdb);
                return -1;
            }
        }
    }

    if (!itdb_cp_track_to_ipod(track, req->audio_path, &err)) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to copy audio file to iPod");
        }
        if (err) g_error_free(err);
        itdb_free(itdb);
        return -1;
    }

    if (!itdb_write(itdb, &err)) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to write iTunesDB/ArtworkDB");
        }
        if (err) g_error_free(err);
        itdb_free(itdb);
        return -1;
    }

    if (out_track_id) {
        *out_track_id = (uint64_t)track->id;
    }
    itdb_free(itdb);
    return 0;
}

int poddrop_gpod_remove_track(
    const char *mount_path,
    uint64_t track_id,
    char **out_err
) {
    if (!mount_path) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }

    GError *err = NULL;
    Itdb_iTunesDB *itdb = itdb_parse(mount_path, &err);
    if (!itdb) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to parse iTunesDB");
        }
        if (err) g_error_free(err);
        return -1;
    }

    Itdb_Track *track = itdb_track_by_id(itdb, (guint32)track_id);
    if (!track) {
        if (out_err) *out_err = g_strdup_printf("Track ID %" G_GUINT64_FORMAT " not found", track_id);
        itdb_free(itdb);
        return -1;
    }

    gchar *ipod_file = itdb_filename_on_ipod(track);
    for (GList *l = itdb->playlists; l != NULL; l = l->next) {
        Itdb_Playlist *pl = (Itdb_Playlist *)l->data;
        if (pl && itdb_playlist_contains_track(pl, track)) {
            itdb_playlist_remove_track(pl, track);
        }
    }
    itdb_track_remove(track);

    if (!itdb_write(itdb, &err)) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to write iTunesDB after removal");
        }
        if (err) g_error_free(err);
        g_free(ipod_file);
        itdb_free(itdb);
        return -1;
    }

    if (ipod_file) {
        unlink(ipod_file);
        g_free(ipod_file);
    }

    itdb_free(itdb);
    return 0;
}

typedef struct {
    const char *title;
    const char *artist;
    const char *album;
    const char *genre;
    int32_t track_number;
    int32_t disc_number;
    int32_t year;
    const char *artwork_path;
    int32_t clear_artwork;
} PodDropGpodUpdateTrackRequest;

int poddrop_gpod_update_track(
    const char *mount_path,
    uint64_t track_id,
    const PodDropGpodUpdateTrackRequest *req,
    char **out_err
) {
    if (!mount_path || !req) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }

    GError *err = NULL;
    Itdb_iTunesDB *itdb = itdb_parse(mount_path, &err);
    if (!itdb) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to parse iTunesDB");
        }
        if (err) g_error_free(err);
        return -1;
    }

    Itdb_Track *track = itdb_track_by_id(itdb, (guint32)track_id);
    if (!track) {
        if (out_err) *out_err = g_strdup_printf("Track ID %" G_GUINT64_FORMAT " not found", track_id);
        itdb_free(itdb);
        return -1;
    }

    if (req->title) {
        g_free(track->title);
        track->title = g_strdup(req->title);
    }
    if (req->artist) {
        g_free(track->artist);
        track->artist = g_strdup(req->artist);
    }
    if (req->album) {
        g_free(track->album);
        track->album = g_strdup(req->album);
    }
    if (req->genre) {
        g_free(track->genre);
        track->genre = g_strdup(req->genre);
    }
    if (req->track_number >= 0) track->track_nr = req->track_number;
    if (req->disc_number >= 0) track->cd_nr = req->disc_number;
    if (req->year >= 0) track->year = req->year;

    if (req->clear_artwork) {
        itdb_track_remove_thumbnails(track);
    } else if (req->artwork_path && req->artwork_path[0] != '\0') {
        if (itdb->device && itdb_device_supports_artwork(itdb->device)) {
            itdb_track_remove_thumbnails(track);
            if (!itdb_track_set_thumbnails(track, req->artwork_path)) {
                if (out_err) *out_err = g_strdup("Failed to update track cover art");
                itdb_free(itdb);
                return -1;
            }
        }
    }

    if (!itdb_write(itdb, &err)) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to write iTunesDB");
        }
        if (err) g_error_free(err);
        itdb_free(itdb);
        return -1;
    }

    itdb_free(itdb);
    return 0;
}

int poddrop_statvfs(const char *path, uint64_t *out_total_bytes, uint64_t *out_free_bytes) {
    if (!path || !out_total_bytes || !out_free_bytes) return -1;
    struct statvfs st;
    if (statvfs(path, &st) != 0) return -1;
    uint64_t bsize = (uint64_t)st.f_frsize;
    *out_total_bytes = (uint64_t)st.f_blocks * bsize;
    *out_free_bytes = (uint64_t)st.f_bavail * bsize;
    return 0;
}

typedef struct {
    const char *old_album;
    const char *new_album;
    const char *new_artist;
    const char *new_genre;
    int32_t year;
    const char *artwork_path;
    int32_t clear_artwork;
} PodDropGpodUpdateAlbumRequest;

int poddrop_gpod_update_album(
    const char *mount_path,
    const PodDropGpodUpdateAlbumRequest *req,
    uint32_t *out_updated_count,
    char **out_err
) {
    if (!mount_path || !req || !req->old_album) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }

    GError *err = NULL;
    Itdb_iTunesDB *itdb = itdb_parse(mount_path, &err);
    if (!itdb) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to parse iTunesDB");
        }
        if (err) g_error_free(err);
        return -1;
    }

    uint32_t updated = 0;
    const gboolean is_unknown_target = (strcmp(req->old_album, "Unknown Album") == 0);

    for (GList *l = itdb->tracks; l != NULL; l = l->next) {
        Itdb_Track *track = (Itdb_Track *)l->data;
        if (!track) continue;

        const char *cur_alb = track->album ? track->album : "";
        const gboolean matched =
            (strcmp(cur_alb, req->old_album) == 0) ||
            (is_unknown_target && cur_alb[0] == '\0');
        if (!matched) continue;

        if (req->new_album) {
            g_free(track->album);
            track->album = g_strdup(req->new_album);
        }
        if (req->new_artist) {
            g_free(track->artist);
            track->artist = g_strdup(req->new_artist);
        }
        if (req->new_genre) {
            g_free(track->genre);
            track->genre = g_strdup(req->new_genre);
        }
        if (req->year >= 0) {
            track->year = req->year;
        }

        if (req->clear_artwork) {
            itdb_track_remove_thumbnails(track);
        } else if (req->artwork_path && req->artwork_path[0] != '\0') {
            if (itdb->device && itdb_device_supports_artwork(itdb->device)) {
                itdb_track_remove_thumbnails(track);
                if (!itdb_track_set_thumbnails(track, req->artwork_path)) {
                    if (out_err) *out_err = g_strdup("Failed to update album cover art");
                    itdb_free(itdb);
                    return -1;
                }
            }
        }
        updated++;
    }

    if (updated > 0 && !itdb_write(itdb, &err)) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to write iTunesDB");
        }
        if (err) g_error_free(err);
        itdb_free(itdb);
        return -1;
    }

    if (out_updated_count) {
        *out_updated_count = updated;
    }
    itdb_free(itdb);
    return 0;
}

void poddrop_gpod_free_bytes(uint8_t *data) {
    g_free(data);
}

int poddrop_gpod_get_track_artwork_png(
    const char *mount_path,
    uint64_t track_id,
    uint8_t **out_data,
    size_t *out_len,
    char **out_err
) {
    if (!mount_path || !out_data || !out_len) {
        if (out_err) *out_err = g_strdup("Invalid arguments");
        return -1;
    }
    *out_data = NULL;
    *out_len = 0;

    GError *err = NULL;
    Itdb_iTunesDB *itdb = itdb_parse(mount_path, &err);
    if (!itdb) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to parse iTunesDB");
        }
        if (err) g_error_free(err);
        return -1;
    }

    Itdb_Track *track = itdb_track_by_id(itdb, (guint32)track_id);
    if (!track) {
        itdb_free(itdb);
        return 0;
    }

    GdkPixbuf *pixbuf = (GdkPixbuf *)itdb_track_get_thumbnail(track, -1, -1);
    if (!pixbuf) {
        itdb_free(itdb);
        return 0;
    }

    gchar *png_buf = NULL;
    gsize png_len = 0;
    const gboolean ok = gdk_pixbuf_save_to_buffer(pixbuf, &png_buf, &png_len, "png", &err, NULL);
    g_object_unref(pixbuf);
    itdb_free(itdb);

    if (!ok) {
        if (out_err) {
            *out_err = g_strdup(err ? err->message : "Failed to encode cover thumbnail to PNG");
        }
        if (err) g_error_free(err);
        g_free(png_buf);
        return -1;
    }

    *out_data = (uint8_t *)png_buf;
    *out_len = (size_t)png_len;
    return 0;
}
