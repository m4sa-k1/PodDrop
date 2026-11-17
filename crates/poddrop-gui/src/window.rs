use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};

use poddrop_core::backend::{IpodBackend, TrackMetadata, TransferTrackRequest, UpdateTrackRequest};
use poddrop_core::backup::default_backup_root;
use poddrop_core::device::DeviceInfo;
use poddrop_core::gpod::{discover_mounted_ipods, LibgpodBackend};
use poddrop_core::i18n::{Language, Translator};
use poddrop_core::media::TranscodeFormat;
use poddrop_core::{APP_NAME, APP_VERSION};

use crate::dialogs::{show_confirm_dialog, show_create_album_dialog, show_track_editor_dialog};

struct AppState {
    lang: Language,
    mount_path: Option<PathBuf>,
    transcode_format: TranscodeFormat,
    device_info: Option<DeviceInfo>,
    tracks: Vec<TrackMetadata>,
    selected_album: Option<String>,
    selected_track_id: Option<u64>,
    search_query: String,
    status_message: String,
    is_busy: bool,
}

impl AppState {
    fn new() -> Self {
        let detected = discover_mounted_ipods().into_iter().next();
        Self {
            lang: Language::from_env(),
            mount_path: detected,
            transcode_format: TranscodeFormat::Aac,
            device_info: None,
            tracks: Vec::new(),
            selected_album: None,
            selected_track_id: None,
            search_query: String::new(),
            status_message: String::new(),
            is_busy: false,
        }
    }

    fn translator(&self) -> Translator {
        Translator::new(self.lang)
    }

    fn album_summaries(&self) -> Vec<AlbumSummary> {
        let mut map: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for t in &self.tracks {
            let album = if t.album.is_empty() {
                "Unknown Album".to_string()
            } else {
                t.album.clone()
            };
            let artist = if t.artist.is_empty() {
                "Unknown Artist".to_string()
            } else {
                t.artist.clone()
            };
            let entry = map.entry(album).or_insert((artist, 0));
            entry.1 += 1;
        }
        map.into_iter()
            .map(|(album, (artist, count))| AlbumSummary {
                album,
                artist,
                track_count: count,
            })
            .collect()
    }

    fn filtered_tracks(&self) -> Vec<TrackMetadata> {
        let q = self.search_query.trim().to_ascii_lowercase();
        self.tracks
            .iter()
            .filter(|t| {
                let album_match = match &self.selected_album {
                    Some(alb) => {
                        let name = if t.album.is_empty() {
                            "Unknown Album"
                        } else {
                            t.album.as_str()
                        };
                        name == alb
                    }
                    None => true,
                };
                let search_match = q.is_empty()
                    || t.title.to_ascii_lowercase().contains(&q)
                    || t.artist.to_ascii_lowercase().contains(&q)
                    || t.album.to_ascii_lowercase().contains(&q);
                album_match && search_match
            })
            .cloned()
            .collect()
    }
}

#[derive(Clone)]
struct AlbumSummary {
    album: String,
    artist: String,
    track_count: usize,
}

#[derive(Clone)]
struct UiRefs {
    window: gtk4::ApplicationWindow,
    // Toolbar
    open_btn: gtk4::Button,
    refresh_btn: gtk4::Button,
    add_btn: gtk4::Button,
    create_album_btn: gtk4::Button,
    format_dropdown: gtk4::DropDown,
    lang_btn: gtk4::Button,
    // Read-only bar
    ro_bar: gtk4::Box,
    ro_label: gtk4::Label,
    // Left pane: Device & Albums
    device_panel_title: gtk4::Label,
    device_info_label: gtk4::Label,
    storage_label: gtk4::Label,
    storage_bar: gtk4::ProgressBar,
    albums_panel_title: gtk4::Label,
    album_list: gtk4::ListBox,
    add_to_album_btn: gtk4::Button,
    // Right pane: Tracks table
    tracks_panel_title: gtk4::Label,
    search_entry: gtk4::SearchEntry,
    edit_btn: gtk4::Button,
    remove_btn: gtk4::Button,
    col_title: gtk4::Label,
    col_artist: gtk4::Label,
    col_album: gtk4::Label,
    col_time: gtk4::Label,
    col_art: gtk4::Label,
    sg_title: gtk4::SizeGroup,
    sg_artist: gtk4::SizeGroup,
    sg_album: gtk4::SizeGroup,
    track_list: gtk4::ListBox,
    // Status bar
    status_left: gtk4::Label,
    status_right: gtk4::Label,
}

enum WorkerEvent {
    TransferBatch {
        succeeded: usize,
        failed: Vec<String>,
    },
    Update(std::result::Result<(), String>),
    Remove(std::result::Result<(), String>),
}

pub fn build_main_window(app: &gtk4::Application) {
    let state = Rc::new(RefCell::new(AppState::new()));

    // --- Top Classic Toolbar ---
    let open_btn = gtk4::Button::new();
    let refresh_btn = gtk4::Button::new();
    let add_btn = gtk4::Button::builder().css_classes(["primary-btn"]).build();
    let create_album_btn = gtk4::Button::new();
    let format_items = gtk4::StringList::new(&["AAC (.m4a)", "ALAC (.m4a)", "MP3"]);
    let format_dropdown = gtk4::DropDown::new(Some(format_items), gtk4::Expression::NONE);
    let lang_btn = gtk4::Button::builder().label("EN / 日本語").build();

    let spacer = gtk4::Box::builder().hexpand(true).build();
    let toolbar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .css_classes(["toolbar-bar"])
        .build();
    toolbar.append(&open_btn);
    toolbar.append(&refresh_btn);
    toolbar.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
    toolbar.append(&add_btn);
    toolbar.append(&create_album_btn);
    toolbar.append(&format_dropdown);
    toolbar.append(&spacer);
    toolbar.append(&lang_btn);

    // --- Read-Only Notice Bar ---
    let ro_label = gtk4::Label::builder().xalign(0.0).build();
    let ro_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .css_classes(["ro-bar"])
        .visible(false)
        .build();
    ro_bar.append(&ro_label);

    // --- Left Pane: Device Summary + Album Browser ---
    let device_panel_title = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["panel-header"])
        .build();
    let device_info_label = gtk4::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    let storage_label = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["muted"])
        .margin_top(4)
        .build();
    let storage_bar = gtk4::ProgressBar::new();

    let device_body = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();
    device_body.append(&device_info_label);
    device_body.append(&storage_label);
    device_body.append(&storage_bar);

    let device_panel = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["panel-box"])
        .build();
    device_panel.append(&device_panel_title);
    device_panel.append(&device_body);

    let albums_panel_title = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["panel-header"])
        .build();
    let album_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["classic-list"])
        .build();
    let albums_scroll = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .kinetic_scrolling(false)
        .child(&album_list)
        .build();
    let add_to_album_btn = gtk4::Button::builder()
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    let albums_panel = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(true)
        .css_classes(["panel-box"])
        .build();
    albums_panel.append(&albums_panel_title);
    albums_panel.append(&albums_scroll);
    albums_panel.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    albums_panel.append(&add_to_album_btn);

    let left_pane = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .width_request(250)
        .build();
    left_pane.append(&device_panel);
    left_pane.append(&albums_panel);

    // --- Right Pane: Filter Bar + Classic Table ---
    let tracks_panel_title = gtk4::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .css_classes(["panel-header"])
        .build();
    let search_entry = gtk4::SearchEntry::builder().hexpand(true).build();
    let edit_btn = gtk4::Button::new();
    let remove_btn = gtk4::Button::new();

    let filter_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(8)
        .margin_end(8)
        .build();
    filter_bar.append(&search_entry);
    filter_bar.append(&edit_btn);
    filter_bar.append(&remove_btn);

    let col_num = gtk4::Label::builder()
        .label("#")
        .xalign(0.0)
        .width_request(36)
        .build();
    let col_title = gtk4::Label::builder().xalign(0.0).hexpand(true).build();
    let col_artist = gtk4::Label::builder().xalign(0.0).hexpand(true).build();
    let col_album = gtk4::Label::builder().xalign(0.0).hexpand(true).build();
    let col_time = gtk4::Label::builder().xalign(1.0).width_request(56).build();
    let col_art = gtk4::Label::builder().xalign(0.5).width_request(48).build();

    let sg_title = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
    let sg_artist = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
    let sg_album = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
    sg_title.add_widget(&col_title);
    sg_artist.add_widget(&col_artist);
    sg_album.add_widget(&col_album);

    let table_header = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .css_classes(["table-header"])
        .build();
    table_header.append(&col_num);
    table_header.append(&col_title);
    table_header.append(&col_artist);
    table_header.append(&col_album);
    table_header.append(&col_time);
    table_header.append(&col_art);

    let track_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["classic-list"])
        .build();
    let tracks_scroll = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .kinetic_scrolling(false)
        .child(&track_list)
        .build();

    let right_pane = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .css_classes(["panel-box"])
        .build();
    right_pane.append(&tracks_panel_title);
    right_pane.append(&filter_bar);
    right_pane.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    right_pane.append(&table_header);
    right_pane.append(&tracks_scroll);

    let paned = gtk4::Paned::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .position(270)
        .start_child(&left_pane)
        .end_child(&right_pane)
        .resize_start_child(false)
        .shrink_start_child(false)
        .vexpand(true)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    // --- Bottom Classic Status Bar ---
    let status_left = gtk4::Label::builder().xalign(0.0).hexpand(true).build();
    let status_right = gtk4::Label::builder().xalign(1.0).build();
    let status_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .css_classes(["status-bar"])
        .build();
    status_bar.append(&status_left);
    status_bar.append(&status_right);

    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    root.append(&toolbar);
    root.append(&ro_bar);
    root.append(&paned);
    root.append(&status_bar);

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title(format!("{APP_NAME} v{APP_VERSION}"))
        .default_width(980)
        .default_height(640)
        .child(&root)
        .build();

    let ui = UiRefs {
        window: window.clone(),
        open_btn: open_btn.clone(),
        refresh_btn: refresh_btn.clone(),
        add_btn: add_btn.clone(),
        create_album_btn: create_album_btn.clone(),
        format_dropdown: format_dropdown.clone(),
        lang_btn: lang_btn.clone(),
        ro_bar,
        ro_label,
        device_panel_title,
        device_info_label,
        storage_label,
        storage_bar,
        albums_panel_title,
        album_list: album_list.clone(),
        add_to_album_btn: add_to_album_btn.clone(),
        tracks_panel_title,
        search_entry: search_entry.clone(),
        edit_btn: edit_btn.clone(),
        remove_btn: remove_btn.clone(),
        col_title,
        col_artist,
        col_album,
        col_time,
        col_art,
        sg_title,
        sg_artist,
        sg_album,
        track_list: track_list.clone(),
        status_left,
        status_right,
    };

    // Language switch
    {
        let state = state.clone();
        let ui = ui.clone();
        lang_btn.connect_clicked(move |_| {
            {
                let mut st = state.borrow_mut();
                st.lang = match st.lang {
                    Language::En => Language::Ja,
                    Language::Ja => Language::En,
                };
            }
            refresh_ui(&state, &ui);
        });
    }

    // Format switch
    {
        let state = state.clone();
        format_dropdown.connect_selected_notify(move |dd| {
            let fmt = match dd.selected() {
                1 => TranscodeFormat::Alac,
                2 => TranscodeFormat::Mp3,
                _ => TranscodeFormat::Aac,
            };
            state.borrow_mut().transcode_format = fmt;
        });
    }

    // Search filter
    {
        let state = state.clone();
        let ui = ui.clone();
        search_entry.connect_search_changed(move |entry| {
            state.borrow_mut().search_query = entry.text().to_string();
            refresh_tracks_only(&state, &ui);
        });
    }

    // Refresh
    {
        let state = state.clone();
        let ui = ui.clone();
        refresh_btn.connect_clicked(move |_| {
            if state.borrow().mount_path.is_none() {
                state.borrow_mut().mount_path = discover_mounted_ipods().into_iter().next();
            }
            reload_device(&state, &ui);
        });
    }

    // Open iPod mount folder
    {
        let state = state.clone();
        let ui = ui.clone();
        open_btn.connect_clicked(move |_| {
            let tr = state.borrow().translator();
            let dialog = gtk4::FileDialog::builder()
                .title(tr.t("gui.open_folder"))
                .modal(true)
                .build();
            let state_cb = state.clone();
            let ui_cb = ui.clone();
            dialog.select_folder(Some(&ui.window), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(path) = file.path() {
                        state_cb.borrow_mut().mount_path = Some(path);
                        reload_device(&state_cb, &ui_cb);
                    }
                }
            });
        });
    }

    // Add music files
    {
        let state = state.clone();
        let ui = ui.clone();
        add_btn.connect_clicked(move |_| {
            open_audio_picker(&state, &ui);
        });
    }

    // Create album
    {
        let state = state.clone();
        let ui = ui.clone();
        create_album_btn.connect_clicked(move |_| {
            open_create_album_flow(&state, &ui, None, None);
        });
    }

    // Add to selected album
    {
        let state = state.clone();
        let ui = ui.clone();
        add_to_album_btn.connect_clicked(move |_| {
            let alb = state.borrow().selected_album.clone();
            let artist = alb.as_ref().and_then(|a| {
                state
                    .borrow()
                    .tracks
                    .iter()
                    .find(|t| t.album == *a)
                    .map(|t| t.artist.clone())
            });
            open_create_album_flow(&state, &ui, alb.as_deref(), artist.as_deref());
        });
    }

    // Album list selection
    {
        let state = state.clone();
        let ui = ui.clone();
        album_list.connect_row_selected(move |_, row_opt| {
            let Some(row) = row_opt else { return };
            let idx = row.index();
            let selected_album = if idx <= 0 {
                None
            } else {
                let albums = state.borrow().album_summaries();
                albums.get((idx - 1) as usize).map(|a| a.album.clone())
            };
            state.borrow_mut().selected_album = selected_album;
            state.borrow_mut().selected_track_id = None;
            refresh_tracks_only(&state, &ui);
        });
    }

    // Track selection
    {
        let state = state.clone();
        let ui = ui.clone();
        track_list.connect_row_selected(move |_, row_opt| {
            let tid = row_opt.and_then(|row| {
                let idx = row.index();
                if idx < 0 {
                    return None;
                }
                state
                    .borrow()
                    .filtered_tracks()
                    .get(idx as usize)
                    .map(|t| t.id)
            });
            state.borrow_mut().selected_track_id = tid;
            update_action_buttons(&state, &ui);
        });
    }

    // Double click track row to open editor
    {
        let gesture = gtk4::GestureClick::new();
        let state = state.clone();
        let ui = ui.clone();
        gesture.connect_pressed(move |_, n_press, _, _| {
            if n_press == 2 {
                open_selected_track_editor(&state, &ui);
            }
        });
        track_list.add_controller(gesture);
    }

    // Edit selected track button
    {
        let state = state.clone();
        let ui = ui.clone();
        edit_btn.connect_clicked(move |_| {
            open_selected_track_editor(&state, &ui);
        });
    }

    // Remove selected track button
    {
        let state = state.clone();
        let ui = ui.clone();
        remove_btn.connect_clicked(move |_| {
            let (mount_opt, track_opt) = {
                let st = state.borrow();
                let trk = st
                    .selected_track_id
                    .and_then(|id| st.tracks.iter().find(|t| t.id == id).cloned());
                (st.mount_path.clone(), trk)
            };
            if let (Some(mp), Some(track)) = (mount_opt, track_opt) {
                let tr = state.borrow().translator();
                let state_c = state.clone();
                let ui_c = ui.clone();
                let tid = track.id;
                show_confirm_dialog(
                    &ui.window,
                    tr.t("gui.remove"),
                    &format!("{}\n\n{}", tr.t("gui.remove_confirm"), track.title),
                    tr.t("gui.remove"),
                    tr.t("gui.cancel"),
                    Rc::new(move || {
                        start_remove_track(&mp, tid, &state_c, &ui_c);
                    }),
                );
            }
        });
    }

    // Drag & drop audio files onto right pane
    {
        let drop_target =
            gtk4::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let state_cb = state.clone();
        let ui_cb = ui.clone();
        drop_target.connect_drop(move |_, value, _, _| {
            if let Ok(file_list) = value.get::<gdk::FileList>() {
                let paths: Vec<PathBuf> = file_list
                    .files()
                    .into_iter()
                    .filter_map(|f| f.path())
                    .filter(|p| p.is_file())
                    .collect();
                if !paths.is_empty() {
                    let album_opt = state_cb.borrow().selected_album.clone();
                    start_transfer_batch(paths, None, album_opt, None, &state_cb, &ui_cb);
                    return true;
                }
            }
            false
        });
        right_pane.add_controller(drop_target);
    }

    reload_device(&state, &ui);
    window.present();
}

fn open_selected_track_editor(state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let (mount_opt, track_opt, can_write, tr) = {
        let st = state.borrow();
        let trk = st
            .selected_track_id
            .and_then(|id| st.tracks.iter().find(|t| t.id == id).cloned());
        let writable = st.device_info.as_ref().is_some_and(|d| !d.is_read_only) && !st.is_busy;
        (st.mount_path.clone(), trk, writable, st.translator())
    };
    if let (Some(mp), Some(track)) = (mount_opt, track_opt) {
        let st_c = state.clone();
        let ui_c = ui.clone();
        show_track_editor_dialog(
            &ui.window,
            &tr,
            &track,
            can_write,
            Rc::new(move |tid, req| {
                start_update_track(&mp, tid, req, &st_c, &ui_c);
            }),
        );
    }
}

fn open_audio_picker(state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let tr = state.borrow().translator();
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("Audio (MP3, M4A, FLAC, OGG, WAV)"));
    for pat in [
        "*.mp3", "*.m4a", "*.mp4", "*.flac", "*.ogg", "*.opus", "*.wav",
    ] {
        filter.add_pattern(pat);
    }
    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);

    let dialog = gtk4::FileDialog::builder()
        .title(tr.t("gui.add_music"))
        .filters(&filters)
        .modal(true)
        .build();

    let state_cb = state.clone();
    let ui_cb = ui.clone();
    dialog.open_multiple(Some(&ui.window), gio::Cancellable::NONE, move |res| {
        if let Ok(list) = res {
            let mut paths = Vec::new();
            for idx in 0..list.n_items() {
                if let Some(obj) = list.item(idx) {
                    if let Ok(file) = obj.downcast::<gio::File>() {
                        if let Some(p) = file.path() {
                            paths.push(p);
                        }
                    }
                }
            }
            if !paths.is_empty() {
                start_transfer_batch(paths, None, None, None, &state_cb, &ui_cb);
            }
        }
    });
}

fn open_create_album_flow(
    state: &Rc<RefCell<AppState>>,
    ui: &UiRefs,
    initial_album: Option<&str>,
    initial_artist: Option<&str>,
) {
    let tr = state.borrow().translator();
    let state_cb = state.clone();
    let ui_cb = ui.clone();
    show_create_album_dialog(
        &ui.window,
        &tr,
        initial_album,
        initial_artist,
        Rc::new(move |album, artist, cover, paths| {
            let album_opt = (!album.is_empty()).then_some(album);
            let artist_opt = (!artist.is_empty()).then_some(artist);
            start_transfer_batch(paths, cover, album_opt, artist_opt, &state_cb, &ui_cb);
        }),
    );
}

fn reload_device(state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let mount_opt = state.borrow().mount_path.clone();
    if let Some(mount) = mount_opt {
        let backend = LibgpodBackend::new();
        match backend.inspect_device(&mount) {
            Ok(info) => {
                let tracks = backend.list_tracks(&mount).unwrap_or_default();
                let mut st = state.borrow_mut();
                st.device_info = Some(info);
                st.tracks = tracks;
                if st.status_message.is_empty() {
                    st.status_message = mount.display().to_string();
                }
            }
            Err(e) => {
                let mut st = state.borrow_mut();
                st.device_info = None;
                st.tracks.clear();
                st.status_message = e.to_string();
            }
        }
    } else {
        let mut st = state.borrow_mut();
        st.device_info = None;
        st.tracks.clear();
        st.status_message.clear();
    }
    refresh_ui(state, ui);
}

fn refresh_ui(state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let st = state.borrow();
    let tr = st.translator();

    ui.open_btn.set_label(tr.t("gui.open_folder"));
    ui.refresh_btn.set_label(tr.t("gui.refresh"));
    ui.add_btn.set_label(tr.t("gui.add_music"));
    ui.create_album_btn.set_label(tr.t("gui.create_album"));
    ui.add_to_album_btn.set_label(tr.t("gui.add_to_album"));
    ui.edit_btn.set_label(tr.t("gui.edit"));
    ui.remove_btn.set_label(tr.t("gui.remove"));
    ui.ro_label.set_label(tr.t("gui.readonly_banner"));
    ui.device_panel_title.set_label(tr.t("gui.device_header"));
    ui.search_entry
        .set_placeholder_text(Some(tr.t("gui.search_placeholder")));

    ui.col_title.set_label(tr.t("gui.track_title"));
    ui.col_artist.set_label(tr.t("gui.artist_name"));
    ui.col_album.set_label(tr.t("gui.album_name"));
    ui.col_time.set_label("Time");
    ui.col_art.set_label("Art");

    let albums = st.album_summaries();
    ui.albums_panel_title
        .set_label(&format!("{} ({})", tr.t("gui.albums_header"), albums.len()));

    match &st.device_info {
        Some(info) => {
            ui.ro_bar.set_visible(info.is_read_only);
            let model_num = info.model_number.as_deref().unwrap_or("-");
            let guid = info.firewire_guid.as_deref().unwrap_or("-");
            ui.device_info_label.set_label(&format!(
                "{} ({model_num})\n{}: {}\nGUID: {guid}",
                info.model.display_name(),
                tr.t("inspect.filesystem"),
                info.filesystem.as_str(),
            ));

            if let (Some(total), Some(free)) = (info.storage_total_bytes, info.storage_free_bytes) {
                let used = total.saturating_sub(free);
                let ratio = if total > 0 {
                    used as f64 / total as f64
                } else {
                    0.0
                };
                let gb = 1024.0 * 1024.0 * 1024.0;
                ui.storage_label.set_label(&format!(
                    "{}: {:.2} / {:.2} GB",
                    tr.t("gui.storage"),
                    used as f64 / gb,
                    total as f64 / gb
                ));
                ui.storage_bar.set_fraction(ratio.clamp(0.0, 1.0));
            } else {
                ui.storage_label.set_label("");
                ui.storage_bar.set_fraction(0.0);
            }

            ui.status_right.set_label(&format!(
                "{}: {}  |  {}: {}  |  {}: {}",
                tr.t("inspect.tracks"),
                info.track_count,
                tr.t("gui.albums_count"),
                albums.len(),
                tr.t("inspect.playlists"),
                info.playlist_count,
            ));
        }
        None => {
            ui.ro_bar.set_visible(false);
            ui.device_info_label.set_label(tr.t("gui.no_device"));
            ui.storage_label.set_label("");
            ui.storage_bar.set_fraction(0.0);
            ui.status_right.set_label("");
        }
    }

    if st.is_busy {
        ui.status_left.set_label(tr.t("gui.transferring"));
    } else if !st.status_message.is_empty() {
        ui.status_left.set_label(&st.status_message);
    } else {
        ui.status_left.set_label(tr.t("gui.drop_hint"));
    }

    let selected_album = st.selected_album.clone();
    let total_tracks = st.tracks.len();
    drop(st);

    // Populate Album sidebar
    while let Some(child) = ui.album_list.first_child() {
        ui.album_list.remove(&child);
    }
    let all_label = gtk4::Label::builder()
        .xalign(0.0)
        .label(format!("All ({total_tracks})"))
        .build();
    ui.album_list.append(&all_label);

    let mut row_to_select = 0i32;
    for (idx, alb) in albums.iter().enumerate() {
        let lbl = gtk4::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .label(format!("{} ({})", alb.album, alb.track_count))
            .tooltip_text(format!("{} — {}", alb.album, alb.artist))
            .build();
        ui.album_list.append(&lbl);
        if selected_album.as_deref() == Some(alb.album.as_str()) {
            row_to_select = (idx + 1) as i32;
        }
    }
    if let Some(row) = ui.album_list.row_at_index(row_to_select) {
        ui.album_list.select_row(Some(&row));
    }

    refresh_tracks_only(state, ui);
}

fn refresh_tracks_only(state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let (filtered, tr, selected_id) = {
        let st = state.borrow();
        (st.filtered_tracks(), st.translator(), st.selected_track_id)
    };

    ui.tracks_panel_title.set_label(&format!(
        "{} ({})",
        tr.t("gui.tracks_header"),
        filtered.len()
    ));

    while let Some(child) = ui.track_list.first_child() {
        ui.track_list.remove(&child);
    }

    let mut row_to_select: Option<i32> = None;
    for (idx, track) in filtered.iter().enumerate() {
        let dur_sec = track.duration_ms / 1000;
        let num_str = track
            .track_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| (idx + 1).to_string());

        let row_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let c_num = gtk4::Label::builder()
            .label(&num_str)
            .xalign(0.0)
            .width_request(36)
            .build();
        let c_title = gtk4::Label::builder()
            .label(&track.title)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        let c_artist = gtk4::Label::builder()
            .label(&track.artist)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        let c_album = gtk4::Label::builder()
            .label(&track.album)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        let c_time = gtk4::Label::builder()
            .label(format!("{}:{:02}", dur_sec / 60, dur_sec % 60))
            .xalign(1.0)
            .width_request(56)
            .build();
        let c_art = gtk4::Label::builder()
            .label(if track.has_artwork { "Yes" } else { "-" })
            .xalign(0.5)
            .width_request(48)
            .build();

        row_box.append(&c_num);
        row_box.append(&c_title);
        row_box.append(&c_artist);
        row_box.append(&c_album);
        row_box.append(&c_time);
        row_box.append(&c_art);

        ui.sg_title.add_widget(&c_title);
        ui.sg_artist.add_widget(&c_artist);
        ui.sg_album.add_widget(&c_album);

        ui.track_list.append(&row_box);
        if selected_id == Some(track.id) {
            row_to_select = Some(idx as i32);
        }
    }

    if let Some(i) = row_to_select {
        if let Some(row) = ui.track_list.row_at_index(i) {
            ui.track_list.select_row(Some(&row));
        }
    }

    update_action_buttons(state, ui);
}

fn update_action_buttons(state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let st = state.borrow();
    let has_device = st.device_info.is_some();
    let can_write = st.device_info.as_ref().is_some_and(|d| !d.is_read_only) && !st.is_busy;
    let has_selection = st.selected_track_id.is_some();

    ui.open_btn.set_sensitive(!st.is_busy);
    ui.refresh_btn.set_sensitive(!st.is_busy);
    ui.format_dropdown.set_sensitive(!st.is_busy);
    ui.lang_btn.set_sensitive(!st.is_busy);
    ui.add_btn.set_sensitive(can_write);
    ui.create_album_btn.set_sensitive(can_write);
    ui.add_to_album_btn.set_sensitive(can_write);
    ui.edit_btn
        .set_sensitive(has_device && has_selection && !st.is_busy);
    ui.remove_btn.set_sensitive(can_write && has_selection);
}

fn start_transfer_batch(
    paths: Vec<PathBuf>,
    artwork: Option<PathBuf>,
    album_override: Option<String>,
    artist_override: Option<String>,
    state: &Rc<RefCell<AppState>>,
    ui: &UiRefs,
) {
    let (mount, format, can_write) = {
        let st = state.borrow();
        let writable = st.device_info.as_ref().is_some_and(|d| !d.is_read_only);
        (
            st.mount_path.clone(),
            st.transcode_format,
            writable && !st.is_busy,
        )
    };

    let (Some(mount_path), true) = (mount, can_write) else {
        return;
    };

    state.borrow_mut().is_busy = true;
    refresh_ui(state, ui);

    let (tx, rx) = mpsc::channel::<WorkerEvent>();
    std::thread::spawn(move || {
        let backend = LibgpodBackend::new();
        let backup_root = default_backup_root();
        let mut succeeded = 0usize;
        let mut failed = Vec::new();

        for (idx, audio_path) in paths.into_iter().enumerate() {
            let req = TransferTrackRequest {
                audio_path: audio_path.clone(),
                artwork_path: artwork.clone(),
                transcode_format: format,
                title_override: None,
                artist_override: artist_override.clone(),
                album_override: album_override.clone(),
            };
            let backup_arg = (idx == 0).then_some(backup_root.as_path());
            match backend.transfer_track(&mount_path, &req, backup_arg) {
                Ok(_) => succeeded += 1,
                Err(e) => failed.push(format!("{}: {e}", audio_path.display())),
            }
        }

        let _ = tx.send(WorkerEvent::TransferBatch { succeeded, failed });
    });

    poll_worker(rx, state, ui);
}

fn start_update_track(
    mount_path: &Path,
    track_id: u64,
    request: UpdateTrackRequest,
    state: &Rc<RefCell<AppState>>,
    ui: &UiRefs,
) {
    state.borrow_mut().is_busy = true;
    refresh_ui(state, ui);

    let (tx, rx) = mpsc::channel::<WorkerEvent>();
    let mp = mount_path.to_path_buf();
    std::thread::spawn(move || {
        let res = LibgpodBackend::new()
            .update_track(&mp, track_id, &request)
            .map_err(|e| e.to_string());
        let _ = tx.send(WorkerEvent::Update(res));
    });

    poll_worker(rx, state, ui);
}

fn start_remove_track(
    mount_path: &Path,
    track_id: u64,
    state: &Rc<RefCell<AppState>>,
    ui: &UiRefs,
) {
    state.borrow_mut().is_busy = true;
    refresh_ui(state, ui);

    let (tx, rx) = mpsc::channel::<WorkerEvent>();
    let mp = mount_path.to_path_buf();
    std::thread::spawn(move || {
        let res = LibgpodBackend::new()
            .remove_track(&mp, track_id)
            .map_err(|e| e.to_string());
        let _ = tx.send(WorkerEvent::Remove(res));
    });

    poll_worker(rx, state, ui);
}

fn poll_worker(rx: mpsc::Receiver<WorkerEvent>, state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let state_poll = state.clone();
    let ui_poll = ui.clone();
    glib::timeout_add_local(Duration::from_millis(80), move || match rx.try_recv() {
        Ok(WorkerEvent::TransferBatch { succeeded, failed }) => {
            let tr = state_poll.borrow().translator();
            let msg = if failed.is_empty() {
                format!("{} ({succeeded})", tr.t("gui.transfer_done"))
            } else {
                format!(
                    "{}: {succeeded}, Failed: {} ({})",
                    tr.t("gui.transfer_done"),
                    failed.len(),
                    failed[0]
                )
            };
            {
                let mut st = state_poll.borrow_mut();
                st.is_busy = false;
                st.status_message = msg;
            }
            reload_device(&state_poll, &ui_poll);
            glib::ControlFlow::Break
        }
        Ok(WorkerEvent::Update(res)) => {
            let tr = state_poll.borrow().translator();
            let msg = match res {
                Ok(()) => tr.t("gui.update_done").to_string(),
                Err(e) => e,
            };
            {
                let mut st = state_poll.borrow_mut();
                st.is_busy = false;
                st.status_message = msg;
            }
            reload_device(&state_poll, &ui_poll);
            glib::ControlFlow::Break
        }
        Ok(WorkerEvent::Remove(res)) => {
            {
                let mut st = state_poll.borrow_mut();
                st.is_busy = false;
                st.selected_track_id = None;
                if let Err(err) = res {
                    st.status_message = err;
                }
            }
            reload_device(&state_poll, &ui_poll);
            glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(mpsc::TryRecvError::Disconnected) => {
            state_poll.borrow_mut().is_busy = false;
            refresh_ui(&state_poll, &ui_poll);
            glib::ControlFlow::Break
        }
    });
}
