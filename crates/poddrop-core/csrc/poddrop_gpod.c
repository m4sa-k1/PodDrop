#include <gpod/itdb.h>
#include <glib.h>
#include <errno.h>
#include <fcntl.h>
#include <scsi/sg.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
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
    int32_t track_number;
    int32_t disc_number;
    int32_t duration_ms;
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
        arr[idx].track_number = tr->track_nr;
        arr[idx].disc_number = tr->cd_nr;
        arr[idx].duration_ms = tr->tracklen;
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
