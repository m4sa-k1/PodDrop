use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use adw::prelude::*;
use gtk4::{gdk, gio, glib};
use libadwaita as adw;

use poddrop_core::backend::{IpodBackend, TrackMetadata, TransferTrackRequest};
use poddrop_core::backup::default_backup_root;
use poddrop_core::device::DeviceInfo;
use poddrop_core::gpod::{discover_mounted_ipods, LibgpodBackend};
use poddrop_core::i18n::{Language, Translator};
use poddrop_core::media::TranscodeFormat;
use poddrop_core::{APP_NAME, APP_VERSION};

struct AppState {
    lang: Language,
    mount_path: Option<PathBuf>,
    selected_artwork: Option<PathBuf>,
    transcode_format: TranscodeFormat,
    device_info: Option<DeviceInfo>,
    tracks: Vec<TrackMetadata>,
    is_busy: bool,
}

impl AppState {
    fn new() -> Self {
        let detected = discover_mounted_ipods().into_iter().next();
        Self {
            lang: Language::from_env(),
            mount_path: detected,
            selected_artwork: None,
            transcode_format: TranscodeFormat::Aac,
            device_info: None,
            tracks: Vec::new(),
            is_busy: false,
        }
    }

    fn translator(&self) -> Translator {
        Translator::new(self.lang)
    }
}

#[derive(Clone)]
struct UiRefs {
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    header_title: adw::WindowTitle,
    open_btn: gtk4::Button,
    refresh_btn: gtk4::Button,
    ro_banner: adw::Banner,
    status_label: gtk4::Label,
    device_summary_label: gtk4::Label,
    add_music_btn: gtk4::Button,
    artwork_btn: gtk4::Button,
    clear_artwork_btn: gtk4::Button,
    artwork_label: gtk4::Label,
    format_dropdown: gtk4::DropDown,
    drop_hint_label: gtk4::Label,
    spinner: gtk4::Spinner,
    tracks_header_label: gtk4::Label,
    track_list: gtk4::ListBox,
}

enum WorkerEvent {
    TransferBatchDone {
        succeeded: usize,
        failed: Vec<String>,
    },
    RemoveDone(std::result::Result<(), String>),
}

pub fn build_main_window(app: &adw::Application) {
    let state = Rc::new(RefCell::new(AppState::new()));

    let header_title = adw::WindowTitle::new(APP_NAME, &format!("v{APP_VERSION}"));
    let open_btn = gtk4::Button::builder()
        .icon_name("folder-open-symbolic")
        .build();
    let refresh_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .build();
    let lang_btn = gtk4::Button::builder().label("EN / 日本語").build();

    let header_bar = adw::HeaderBar::builder()
        .title_widget(&header_title)
        .build();
    header_bar.pack_start(&open_btn);
    header_bar.pack_start(&refresh_btn);
    header_bar.pack_end(&lang_btn);

    let ro_banner = adw::Banner::new("");
    let status_label = gtk4::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();

    let device_summary_label = gtk4::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();

    let device_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(12)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    device_card.append(&status_label);
    device_card.append(&device_summary_label);

    let device_frame = gtk4::Frame::builder().child(&device_card).build();

    let add_music_btn = gtk4::Button::builder()
        .css_classes(["suggested-action"])
        .build();
    let artwork_btn = gtk4::Button::new();
    let clear_artwork_btn = gtk4::Button::new();
    let artwork_label = gtk4::Label::builder()
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .hexpand(true)
        .css_classes(["dim-label"])
        .build();

    let format_items = gtk4::StringList::new(&["AAC (.m4a)", "ALAC (.m4a)", "MP3"]);
    let format_dropdown = gtk4::DropDown::new(Some(format_items), gtk4::Expression::NONE);
    let spinner = gtk4::Spinner::new();

    let action_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    action_row.append(&add_music_btn);
    action_row.append(&artwork_btn);
    action_row.append(&clear_artwork_btn);
    action_row.append(&format_dropdown);
    action_row.append(&spinner);

    let drop_hint_label = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let tracks_header_label = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["heading"])
        .margin_top(8)
        .build();

    let track_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&track_list)
        .build();

    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(16)
        .margin_end(16)
        .build();
    content_box.append(&device_frame);
    content_box.append(&action_row);
    content_box.append(&artwork_label);
    content_box.append(&drop_hint_label);
    content_box.append(&tracks_header_label);
    content_box.append(&scrolled);

    let root_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    root_box.append(&ro_banner);
    root_box.append(&content_box);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&root_box));

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&toast_overlay));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(880)
        .default_height(620)
        .content(&toolbar_view)
        .build();

    let ui = UiRefs {
        window: window.clone(),
        toast_overlay,
        header_title,
        open_btn: open_btn.clone(),
        refresh_btn: refresh_btn.clone(),
        ro_banner,
        status_label,
        device_summary_label,
        add_music_btn: add_music_btn.clone(),
        artwork_btn: artwork_btn.clone(),
        clear_artwork_btn: clear_artwork_btn.clone(),
        artwork_label,
        format_dropdown: format_dropdown.clone(),
        drop_hint_label,
        spinner,
        tracks_header_label,
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

    // Refresh device
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

    // Open folder dialog
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

    // Select optional cover artwork
    {
        let state = state.clone();
        let ui = ui.clone();
        artwork_btn.connect_clicked(move |_| {
            let tr = state.borrow().translator();
            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Images (PNG, JPEG)"));
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/jpeg");
            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);

            let dialog = gtk4::FileDialog::builder()
                .title(tr.t("gui.select_artwork"))
                .filters(&filters)
                .modal(true)
                .build();

            let state_cb = state.clone();
            let ui_cb = ui.clone();
            dialog.open(Some(&ui.window), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(path) = file.path() {
                        state_cb.borrow_mut().selected_artwork = Some(path);
                        refresh_ui(&state_cb, &ui_cb);
                    }
                }
            });
        });
    }

    // Clear cover artwork
    {
        let state = state.clone();
        let ui = ui.clone();
        clear_artwork_btn.connect_clicked(move |_| {
            state.borrow_mut().selected_artwork = None;
            refresh_ui(&state, &ui);
        });
    }

    // Add music files button
    {
        let state = state.clone();
        let ui = ui.clone();
        add_music_btn.connect_clicked(move |_| {
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
                        start_transfer_batch(paths, &state_cb, &ui_cb);
                    }
                }
            });
        });
    }

    // Drag & drop support for audio files
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
                    start_transfer_batch(paths, &state_cb, &ui_cb);
                    return true;
                }
            }
            false
        });
        content_box.add_controller(drop_target);
    }

    reload_device(&state, &ui);
    window.present();
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
                let toast = adw::Toast::new(&e.to_string());
                ui.toast_overlay.add_toast(toast);
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

    ui.open_btn.set_tooltip_text(Some(tr.t("gui.open_folder")));
    ui.refresh_btn.set_tooltip_text(Some(tr.t("gui.refresh")));
    ui.add_music_btn.set_label(tr.t("gui.add_music"));
    ui.artwork_btn.set_label(tr.t("gui.select_artwork"));
    ui.clear_artwork_btn.set_label(tr.t("gui.clear_artwork"));
    ui.ro_banner.set_title(tr.t("gui.readonly_banner"));
    ui.drop_hint_label.set_label(tr.t("gui.drop_hint"));
    ui.tracks_header_label.set_label(&format!(
        "{} ({})",
        tr.t("gui.tracks_header"),
        st.tracks.len()
    ));

    if let Some(art) = &st.selected_artwork {
        ui.artwork_label
            .set_label(&format!("Artwork: {}", art.display()));
        ui.clear_artwork_btn.set_sensitive(!st.is_busy);
    } else {
        ui.artwork_label.set_label(tr.t("gui.artwork_auto"));
        ui.clear_artwork_btn.set_sensitive(false);
    }

    if st.is_busy {
        ui.spinner.start();
    } else {
        ui.spinner.stop();
    }

    let can_write = match &st.device_info {
        Some(info) => {
            ui.header_title
                .set_subtitle(&format!("{} (v{APP_VERSION})", info.model.display_name()));
            ui.ro_banner.set_revealed(info.is_read_only);
            ui.status_label.set_label(&format!(
                "{}: {}",
                tr.t("inspect.mount"),
                info.mount_path.display()
            ));
            let cap = info
                .capacity_gb
                .map(|c| format!("{c:.1} GB"))
                .unwrap_or_else(|| "-".into());
            let model_num = info.model_number.as_deref().unwrap_or("-");
            let guid = info.firewire_guid.as_deref().unwrap_or("-");
            ui.device_summary_label.set_label(&format!(
                "{}: {} ({model_num})  |  {}: {cap}  |  {}: {}\n{}: {}  |  {}: {guid}  |  {}: {}",
                tr.t("inspect.model"),
                info.model.display_name(),
                tr.t("inspect.capacity"),
                tr.t("inspect.filesystem"),
                info.filesystem.as_str(),
                tr.t("inspect.artwork"),
                info.supports_artwork,
                tr.t("inspect.guid"),
                tr.t("inspect.tracks"),
                info.track_count
            ));
            !info.is_read_only && !st.is_busy
        }
        None => {
            ui.header_title.set_subtitle(&format!("v{APP_VERSION}"));
            ui.ro_banner.set_revealed(false);
            ui.status_label.set_label(tr.t("gui.no_device"));
            ui.device_summary_label.set_label("");
            false
        }
    };

    ui.add_music_btn.set_sensitive(can_write);
    ui.artwork_btn.set_sensitive(!st.is_busy);
    ui.format_dropdown.set_sensitive(!st.is_busy);
    ui.open_btn.set_sensitive(!st.is_busy);
    ui.refresh_btn.set_sensitive(!st.is_busy);

    while let Some(child) = ui.track_list.first_child() {
        ui.track_list.remove(&child);
    }

    let tracks = st.tracks.clone();
    let mount_path = st.mount_path.clone();
    drop(st);

    for track in tracks {
        let row = adw::ActionRow::builder()
            .title(&track.title)
            .subtitle(format!("{} — {}", track.artist, track.album))
            .build();

        if track.has_artwork {
            let art_badge = gtk4::Image::from_icon_name("emblem-photos-symbolic");
            art_badge.set_tooltip_text(Some("Artwork registered"));
            row.add_suffix(&art_badge);
        }

        let rm_btn = gtk4::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk4::Align::Center)
            .css_classes(["flat"])
            .sensitive(can_write)
            .build();

        if let Some(mp) = mount_path.clone() {
            let state_rm = state.clone();
            let ui_rm = ui.clone();
            let track_id = track.id;
            let track_title = track.title.clone();
            rm_btn.connect_clicked(move |_| {
                confirm_and_remove(&mp, track_id, &track_title, &state_rm, &ui_rm);
            });
        }

        row.add_suffix(&rm_btn);
        ui.track_list.append(&row);
    }
}

fn start_transfer_batch(paths: Vec<PathBuf>, state: &Rc<RefCell<AppState>>, ui: &UiRefs) {
    let (mount, artwork, format, can_write) = {
        let st = state.borrow();
        let writable = st.device_info.as_ref().is_some_and(|d| !d.is_read_only);
        (
            st.mount_path.clone(),
            st.selected_artwork.clone(),
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
                artist_override: None,
                album_override: None,
            };
            // Create one pre-write backup for the batch on the first track.
            let backup_arg = if idx == 0 {
                Some(backup_root.as_path())
            } else {
                None
            };
            match backend.transfer_track(&mount_path, &req, backup_arg) {
                Ok(_) => succeeded += 1,
                Err(e) => failed.push(format!("{}: {e}", audio_path.display())),
            }
        }

        let _ = tx.send(WorkerEvent::TransferBatchDone { succeeded, failed });
    });

    let state_poll = state.clone();
    let ui_poll = ui.clone();
    glib::timeout_add_local(Duration::from_millis(80), move || match rx.try_recv() {
        Ok(WorkerEvent::TransferBatchDone { succeeded, failed }) => {
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
        Ok(WorkerEvent::RemoveDone(_)) => glib::ControlFlow::Break,
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(mpsc::TryRecvError::Disconnected) => {
            state_poll.borrow_mut().is_busy = false;
            refresh_ui(&state_poll, &ui_poll);
            glib::ControlFlow::Break
        }
    });
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
            let _ = tx.send(WorkerEvent::RemoveDone(res));
        });

        let state_poll = state_cb.clone();
        let ui_poll = ui_cb.clone();
        glib::timeout_add_local(Duration::from_millis(80), move || match rx.try_recv() {
            Ok(WorkerEvent::RemoveDone(res)) => {
                state_poll.borrow_mut().is_busy = false;
                reload_device(&state_poll, &ui_poll);
                if let Err(err) = res {
                    ui_poll.toast_overlay.add_toast(adw::Toast::new(&err));
                }
                glib::ControlFlow::Break
            }
            Ok(WorkerEvent::TransferBatchDone { .. }) => glib::ControlFlow::Break,
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                state_poll.borrow_mut().is_busy = false;
                refresh_ui(&state_poll, &ui_poll);
                glib::ControlFlow::Break
            }
        });
    });
}
