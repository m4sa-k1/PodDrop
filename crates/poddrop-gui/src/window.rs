use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use adw::prelude::*;
use gtk4::{gdk, gio, glib};
use libadwaita as adw;

use poddrop_core::backend::{IpodBackend, TrackMetadata, TransferTrackRequest, UpdateTrackRequest};
use poddrop_core::backup::default_backup_root;
use poddrop_core::device::DeviceInfo;
use poddrop_core::gpod::{discover_mounted_ipods, LibgpodBackend};
use poddrop_core::i18n::{Language, Translator};
use poddrop_core::media::TranscodeFormat;
use poddrop_core::{APP_NAME, APP_VERSION};

use crate::dialogs::{show_create_album_dialog, show_track_editor_dialog};

struct AppState {
    lang: Language,
    mount_path: Option<PathBuf>,
    transcode_format: TranscodeFormat,
    device_info: Option<DeviceInfo>,
    tracks: Vec<TrackMetadata>,
    search_query: String,
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
            search_query: String::new(),
            is_busy: false,
        }
    }

    fn translator(&self) -> Translator {
        Translator::new(self.lang)
    }

    fn album_summaries(&self) -> Vec<AlbumSummary> {
        let mut map: BTreeMap<(String, String), usize> = BTreeMap::new();
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
            *map.entry((album, artist)).or_insert(0) += 1;
        }
        map.into_iter()
            .map(|((album, artist), count)| AlbumSummary {
                album,
                artist,
                track_count: count,
            })
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
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    view_stack: adw::ViewStack,
    dashboard_page: adw::ViewStackPage,
    albums_page: adw::ViewStackPage,
    tracks_page: adw::ViewStackPage,
    open_btn: gtk4::Button,
    refresh_btn: gtk4::Button,
    ro_banner: adw::Banner,
    spinner: gtk4::Spinner,
    // Dashboard widgets
    device_heading: gtk4::Label,
    mount_label: gtk4::Label,
    device_meta_label: gtk4::Label,
    storage_label: gtk4::Label,
    storage_bar: gtk4::ProgressBar,
    stat_tracks_val: gtk4::Label,
    stat_tracks_lbl: gtk4::Label,
    stat_albums_val: gtk4::Label,
    stat_albums_lbl: gtk4::Label,
    stat_playlists_val: gtk4::Label,
    stat_playlists_lbl: gtk4::Label,
    dash_add_btn: gtk4::Button,
    dash_album_btn: gtk4::Button,
    format_dropdown: gtk4::DropDown,
    drop_hint_label: gtk4::Label,
    // Albums widgets
    albums_header_label: gtk4::Label,
    create_album_btn: gtk4::Button,
    album_list: gtk4::ListBox,
    // Tracks widgets
    tracks_header_label: gtk4::Label,
    search_entry: gtk4::SearchEntry,
    track_list: gtk4::ListBox,
}

enum WorkerEvent {
    TransferBatch {
        succeeded: usize,
        failed: Vec<String>,
    },
    Update(std::result::Result<(), String>),
    Remove(std::result::Result<(), String>),
}

pub fn build_main_window(app: &adw::Application) {
    let state = Rc::new(RefCell::new(AppState::new()));

    let open_btn = gtk4::Button::builder()
        .icon_name("folder-open-symbolic")
        .build();
    let refresh_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .build();
    let lang_btn = gtk4::Button::builder().label("EN / 日本語").build();
    let spinner = gtk4::Spinner::new();

    let view_stack = adw::ViewStack::new();
    let view_switcher = adw::ViewSwitcher::builder()
        .stack(&view_stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();

    let header_bar = adw::HeaderBar::builder()
        .title_widget(&view_switcher)
        .build();
    header_bar.pack_start(&open_btn);
    header_bar.pack_start(&refresh_btn);
    header_bar.pack_start(&spinner);
    header_bar.pack_end(&lang_btn);

    let ro_banner = adw::Banner::new("");

    // --- Tab 1: Dashboard ---
    let device_heading = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["title-3"])
        .build();
    let mount_label = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["dim-label"])
        .selectable(true)
        .build();
    let device_meta_label = gtk4::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    let storage_label = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["caption", "dim-label"])
        .margin_top(6)
        .build();
    let storage_bar = gtk4::ProgressBar::new();

    let device_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .css_classes(["card-box"])
        .build();
    let device_inner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(18)
        .margin_end(18)
        .build();
    device_inner.append(&device_heading);
    device_inner.append(&mount_label);
    device_inner.append(&device_meta_label);
    device_inner.append(&storage_label);
    device_inner.append(&storage_bar);
    device_box.append(&device_inner);

    // Stats row (centered number + label per card)
    let (stat_tracks_card, stat_tracks_val, stat_tracks_lbl) = make_stat_card();
    let (stat_albums_card, stat_albums_val, stat_albums_lbl) = make_stat_card();
    let (stat_playlists_card, stat_playlists_val, stat_playlists_lbl) = make_stat_card();

    let stats_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .homogeneous(true)
        .build();
    stats_row.append(&stat_tracks_card);
    stats_row.append(&stat_albums_card);
    stats_row.append(&stat_playlists_card);

    // Quick Actions + Drop Zone
    let dash_add_btn = gtk4::Button::builder().css_classes(["primary-btn"]).build();
    let dash_album_btn = gtk4::Button::new();
    let format_items = gtk4::StringList::new(&["AAC (.m4a)", "ALAC (.m4a)", "MP3"]);
    let format_dropdown = gtk4::DropDown::new(Some(format_items), gtk4::Expression::NONE);

    let quick_actions = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk4::Align::Center)
        .build();
    quick_actions.append(&dash_add_btn);
    quick_actions.append(&dash_album_btn);
    quick_actions.append(&format_dropdown);

    let drop_hint_label = gtk4::Label::builder()
        .halign(gtk4::Align::Center)
        .css_classes(["muted-text"])
        .build();

    let drop_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(14)
        .css_classes(["drop-zone"])
        .build();
    let drop_inner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(26)
        .margin_bottom(26)
        .margin_start(18)
        .margin_end(18)
        .build();
    drop_inner.append(&quick_actions);
    drop_inner.append(&drop_hint_label);
    drop_box.append(&drop_inner);

    let dashboard_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(18)
        .margin_end(18)
        .build();
    dashboard_box.append(&device_box);
    dashboard_box.append(&stats_row);
    dashboard_box.append(&drop_box);
    let dashboard_clamp = adw::Clamp::builder()
        .maximum_size(820)
        .child(&dashboard_box)
        .build();

    // --- Tab 2: Albums ---
    let albums_header_label = gtk4::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .css_classes(["heading"])
        .build();
    let create_album_btn = gtk4::Button::builder().css_classes(["primary-btn"]).build();
    let albums_top = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    albums_top.append(&albums_header_label);
    albums_top.append(&create_album_btn);

    let album_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let albums_scroll = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .kinetic_scrolling(false)
        .child(&album_list)
        .build();
    let albums_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(18)
        .margin_end(18)
        .build();
    albums_box.append(&albums_top);
    albums_box.append(&albums_scroll);
    let albums_clamp = adw::Clamp::builder()
        .maximum_size(820)
        .child(&albums_box)
        .build();

    // --- Tab 3: All Tracks ---
    let tracks_header_label = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["heading"])
        .build();
    let search_entry = gtk4::SearchEntry::builder().hexpand(true).build();
    let tracks_top = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .build();
    tracks_top.append(&tracks_header_label);
    tracks_top.append(&search_entry);

    let track_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let tracks_scroll = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .kinetic_scrolling(false)
        .child(&track_list)
        .build();
    let tracks_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(18)
        .margin_end(18)
        .build();
    tracks_box.append(&tracks_top);
    tracks_box.append(&tracks_scroll);
    let tracks_clamp = adw::Clamp::builder()
        .maximum_size(820)
        .child(&tracks_box)
        .build();

    let dashboard_page = view_stack.add_titled_with_icon(
        &dashboard_clamp,
        Some("dashboard"),
        "Dashboard",
        "go-home-symbolic",
    );
    let albums_page = view_stack.add_titled_with_icon(
        &albums_clamp,
        Some("albums"),
        "Albums",
        "media-optical-symbolic",
    );
    let tracks_page = view_stack.add_titled_with_icon(
        &tracks_clamp,
        Some("tracks"),
        "Tracks",
        "audio-x-generic-symbolic",
    );

    let root_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    root_box.append(&ro_banner);
    root_box.append(&view_stack);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&root_box));

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.set_top_bar_style(adw::ToolbarStyle::Raised);
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&toast_overlay));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(format!("{APP_NAME} v{APP_VERSION}"))
        .default_width(920)
        .default_height(640)
        .content(&toolbar_view)
        .build();

    let ui = UiRefs {
        window: window.clone(),
        toast_overlay,
        view_stack,
        dashboard_page,
        albums_page,
        tracks_page,
        open_btn: open_btn.clone(),
        refresh_btn: refresh_btn.clone(),
        ro_banner,
        spinner,
        device_heading,
        mount_label,
        device_meta_label,
        storage_label,
        storage_bar,
        stat_tracks_val,
        stat_tracks_lbl,
        stat_albums_val,
        stat_albums_lbl,
        stat_playlists_val,
        stat_playlists_lbl,
        dash_add_btn: dash_add_btn.clone(),
        dash_album_btn: dash_album_btn.clone(),
        format_dropdown: format_dropdown.clone(),
        drop_hint_label,
        albums_header_label,
        create_album_btn: create_album_btn.clone(),
        album_list,
        tracks_header_label,
        search_entry: search_entry.clone(),
        track_list,
    };

    // Language switcher
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

    // Format selector
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
            refresh_ui(&state, &ui);
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

    // Open folder
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

    // Add music files button
    {
        let state = state.clone();
        let ui = ui.clone();
        dash_add_btn.connect_clicked(move |_| {
            open_audio_picker(&state, &ui);
        });
    }

    // Create Album buttons (Dashboard + Albums tab)
    for btn in [&dash_album_btn, &create_album_btn] {
        let state = state.clone();
        let ui = ui.clone();
        btn.connect_clicked(move |_| {
            open_create_album_flow(&state, &ui, None, None);
        });
    }

    // Drag & drop on Dashboard
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
                    start_transfer_batch(paths, None, None, None, &state_cb, &ui_cb);
                    return true;
                }
            }
            false
        });
        dashboard_box.add_controller(drop_target);
    }

    reload_device(&state, &ui);
    window.present();
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
            }
            Err(e) => {
                let mut st = state.borrow_mut();
                st.device_info = None;
                st.tracks.clear();
                ui.toast_overlay.add_toast(adw::Toast::new(&e.to_string()));
            }
        }
    } else {
        let mut st = state.borrow_mut();
        st.device_info = None;
        st.tracks.clear();
    }
    refresh_ui(state, ui);
}

fn refresh_ui(state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let st = state.borrow();
    let tr = st.translator();

    ui.dashboard_page.set_title(Some(tr.t("gui.tab.dashboard")));
    ui.albums_page.set_title(Some(tr.t("gui.tab.albums")));
    ui.tracks_page.set_title(Some(tr.t("gui.tab.tracks")));

    ui.open_btn.set_tooltip_text(Some(tr.t("gui.open_folder")));
    ui.refresh_btn.set_tooltip_text(Some(tr.t("gui.refresh")));
    ui.ro_banner.set_title(tr.t("gui.readonly_banner"));
    ui.dash_add_btn.set_label(tr.t("gui.add_music"));
    ui.dash_album_btn.set_label(tr.t("gui.create_album"));
    ui.create_album_btn.set_label(tr.t("gui.create_album"));
    ui.drop_hint_label.set_label(tr.t("gui.drop_hint"));
    ui.search_entry
        .set_placeholder_text(Some(tr.t("gui.search_placeholder")));

    if st.is_busy {
        ui.spinner.start();
    } else {
        ui.spinner.stop();
    }

    let albums = st.album_summaries();
    ui.albums_header_label
        .set_label(&format!("{} ({})", tr.t("gui.albums_header"), albums.len()));

    let query = st.search_query.trim().to_ascii_lowercase();
    let filtered_tracks: Vec<TrackMetadata> = st
        .tracks
        .iter()
        .filter(|t| {
            query.is_empty()
                || t.title.to_ascii_lowercase().contains(&query)
                || t.artist.to_ascii_lowercase().contains(&query)
                || t.album.to_ascii_lowercase().contains(&query)
        })
        .cloned()
        .collect();
    ui.tracks_header_label.set_label(&format!(
        "{} ({})",
        tr.t("gui.tracks_header"),
        filtered_tracks.len()
    ));

    let can_write = match &st.device_info {
        Some(info) => {
            ui.ro_banner.set_revealed(info.is_read_only);
            let model_num = info.model_number.as_deref().unwrap_or("-");
            ui.device_heading
                .set_label(&format!("{} ({model_num})", info.model.display_name()));
            ui.mount_label.set_label(&format!(
                "{}: {}",
                tr.t("inspect.mount"),
                info.mount_path.display()
            ));
            let guid = info.firewire_guid.as_deref().unwrap_or("-");
            ui.device_meta_label.set_label(&format!(
                "{}: {}  |  {}: {guid}",
                tr.t("inspect.filesystem"),
                info.filesystem.as_str(),
                tr.t("inspect.guid"),
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
                    "{}: {:.2} GB / {:.2} GB",
                    tr.t("gui.storage"),
                    used as f64 / gb,
                    total as f64 / gb
                ));
                ui.storage_bar.set_fraction(ratio.clamp(0.0, 1.0));
            } else {
                ui.storage_label.set_label("");
                ui.storage_bar.set_fraction(0.0);
            }

            ui.stat_tracks_val.set_label(&info.track_count.to_string());
            ui.stat_tracks_lbl.set_label(tr.t("inspect.tracks"));
            ui.stat_albums_val.set_label(&albums.len().to_string());
            ui.stat_albums_lbl.set_label(tr.t("gui.albums_count"));
            ui.stat_playlists_val
                .set_label(&info.playlist_count.to_string());
            ui.stat_playlists_lbl.set_label(tr.t("inspect.playlists"));

            !info.is_read_only && !st.is_busy
        }
        None => {
            ui.ro_banner.set_revealed(false);
            ui.device_heading.set_label(tr.t("gui.device_header"));
            ui.mount_label.set_label(tr.t("gui.no_device"));
            ui.device_meta_label.set_label("");
            ui.storage_label.set_label("");
            ui.storage_bar.set_fraction(0.0);
            ui.stat_tracks_val.set_label("0");
            ui.stat_tracks_lbl.set_label(tr.t("inspect.tracks"));
            ui.stat_albums_val.set_label("0");
            ui.stat_albums_lbl.set_label(tr.t("gui.albums_count"));
            ui.stat_playlists_val.set_label("0");
            ui.stat_playlists_lbl.set_label(tr.t("inspect.playlists"));
            false
        }
    };

    ui.dash_add_btn.set_sensitive(can_write);
    ui.dash_album_btn.set_sensitive(can_write);
    ui.create_album_btn.set_sensitive(can_write);
    ui.format_dropdown.set_sensitive(!st.is_busy);
    ui.open_btn.set_sensitive(!st.is_busy);
    ui.refresh_btn.set_sensitive(!st.is_busy);

    let mount_path = st.mount_path.clone();
    let is_busy = st.is_busy;
    drop(st);

    // Populate Albums list
    while let Some(child) = ui.album_list.first_child() {
        ui.album_list.remove(&child);
    }
    for alb in albums {
        let row = adw::ActionRow::builder()
            .title(&alb.album)
            .subtitle(format!("{}  •  {} tracks", alb.artist, alb.track_count))
            .activatable(true)
            .build();

        let add_btn = gtk4::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(tr.t("gui.add_to_album"))
            .valign(gtk4::Align::Center)
            .css_classes(["flat"])
            .sensitive(can_write)
            .build();
        {
            let state_c = state.clone();
            let ui_c = ui.clone();
            let a_name = alb.album.clone();
            let ar_name = alb.artist.clone();
            add_btn.connect_clicked(move |_| {
                open_create_album_flow(&state_c, &ui_c, Some(&a_name), Some(&ar_name));
            });
        }
        row.add_suffix(&add_btn);

        {
            let state_c = state.clone();
            let ui_c = ui.clone();
            let a_name = alb.album.clone();
            row.connect_activated(move |_| {
                state_c.borrow_mut().search_query = a_name.clone();
                ui_c.search_entry.set_text(&a_name);
                ui_c.view_stack.set_visible_child_name("tracks");
                refresh_ui(&state_c, &ui_c);
            });
        }

        ui.album_list.append(&row);
    }

    // Populate Tracks list
    while let Some(child) = ui.track_list.first_child() {
        ui.track_list.remove(&child);
    }
    for track in filtered_tracks {
        let duration_sec = track.duration_ms / 1000;
        let row = adw::ActionRow::builder()
            .title(&track.title)
            .subtitle(format!(
                "{} — {}  ({}:{:02})",
                track.artist,
                track.album,
                duration_sec / 60,
                duration_sec % 60
            ))
            .activatable(true)
            .build();

        if track.has_artwork {
            let art_icon = gtk4::Image::from_icon_name("emblem-photos-symbolic");
            row.add_suffix(&art_icon);
        }

        let edit_btn = gtk4::Button::builder()
            .icon_name("document-edit-symbolic")
            .tooltip_text(tr.t("gui.edit"))
            .valign(gtk4::Align::Center)
            .css_classes(["flat"])
            .sensitive(!is_busy)
            .build();

        let rm_btn = gtk4::Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text(tr.t("gui.remove"))
            .valign(gtk4::Align::Center)
            .css_classes(["flat"])
            .sensitive(can_write)
            .build();

        if let Some(mp) = mount_path.clone() {
            let track_clone = track.clone();
            let state_ed = state.clone();
            let ui_ed = ui.clone();
            let mp_ed = mp.clone();
            let open_editor = Rc::new(move || {
                let tr_cur = state_ed.borrow().translator();
                let st_save = state_ed.clone();
                let ui_save = ui_ed.clone();
                let mp_save = mp_ed.clone();
                show_track_editor_dialog(
                    &ui_ed.window,
                    &tr_cur,
                    &track_clone,
                    can_write,
                    Rc::new(move |tid, req| {
                        start_update_track(&mp_save, tid, req, &st_save, &ui_save);
                    }),
                );
            });

            {
                let cb = open_editor.clone();
                edit_btn.connect_clicked(move |_| cb());
            }
            {
                let cb = open_editor.clone();
                row.connect_activated(move |_| cb());
            }

            let state_rm = state.clone();
            let ui_rm = ui.clone();
            let track_id = track.id;
            let track_title = track.title.clone();
            rm_btn.connect_clicked(move |_| {
                confirm_and_remove(&mp, track_id, &track_title, &state_rm, &ui_rm);
            });
        }

        row.add_suffix(&edit_btn);
        row.add_suffix(&rm_btn);
        ui.track_list.append(&row);
    }
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

    let Some(mount_path) = mount else {
        return;
    };
    if !can_write {
        return;
    }

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

fn confirm_and_remove(
    mount_path: &Path,
    track_id: u64,
    track_title: &str,
    state: &Rc<RefCell<AppState>>,
    ui: &UiRefs,
) {
    let tr = state.borrow().translator();
    let dialog = adw::AlertDialog::builder()
        .heading(tr.t("gui.remove_confirm"))
        .body(track_title)
        .build();
    dialog.add_response("cancel", tr.t("gui.cancel"));
    dialog.add_response("remove", tr.t("gui.remove"));
    dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));

    let mp = mount_path.to_path_buf();
    let state_cb = state.clone();
    let ui_cb = ui.clone();
    dialog.choose(&ui.window, gio::Cancellable::NONE, move |resp| {
        if resp != "remove" {
            return;
        }
        state_cb.borrow_mut().is_busy = true;
        refresh_ui(&state_cb, &ui_cb);

        let (tx, rx) = mpsc::channel::<WorkerEvent>();
        let mp_bg = mp.clone();
        std::thread::spawn(move || {
            let res = LibgpodBackend::new()
                .remove_track(&mp_bg, track_id)
                .map_err(|e| e.to_string());
            let _ = tx.send(WorkerEvent::Remove(res));
        });

        poll_worker(rx, &state_cb, &ui_cb);
    });
}

fn poll_worker(rx: mpsc::Receiver<WorkerEvent>, state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let state_poll = state.clone();
    let ui_poll = ui.clone();
    glib::timeout_add_local(Duration::from_millis(80), move || match rx.try_recv() {
        Ok(WorkerEvent::TransferBatch { succeeded, failed }) => {
            state_poll.borrow_mut().is_busy = false;
            reload_device(&state_poll, &ui_poll);
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
            ui_poll.toast_overlay.add_toast(adw::Toast::new(&msg));
            glib::ControlFlow::Break
        }
        Ok(WorkerEvent::Update(res)) => {
            state_poll.borrow_mut().is_busy = false;
            reload_device(&state_poll, &ui_poll);
            let tr = state_poll.borrow().translator();
            let msg = match res {
                Ok(()) => tr.t("gui.update_done").to_string(),
                Err(e) => e,
            };
            ui_poll.toast_overlay.add_toast(adw::Toast::new(&msg));
            glib::ControlFlow::Break
        }
        Ok(WorkerEvent::Remove(res)) => {
            state_poll.borrow_mut().is_busy = false;
            reload_device(&state_poll, &ui_poll);
            if let Err(err) = res {
                ui_poll.toast_overlay.add_toast(adw::Toast::new(&err));
            }
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

fn make_stat_card() -> (gtk4::Box, gtk4::Label, gtk4::Label) {
    let val = gtk4::Label::builder()
        .halign(gtk4::Align::Center)
        .justify(gtk4::Justification::Center)
        .css_classes(["stat-value"])
        .build();
    let lbl = gtk4::Label::builder()
        .halign(gtk4::Align::Center)
        .justify(gtk4::Justification::Center)
        .css_classes(["stat-label"])
        .build();
    let inner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(12)
        .margin_end(12)
        .build();
    inner.append(&val);
    inner.append(&lbl);

    let card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card-box"])
        .build();
    card.append(&inner);
    (card, val, lbl)
}
