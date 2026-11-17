use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::gio;
use libadwaita as adw;

use poddrop_core::backend::{TrackMetadata, UpdateTrackRequest};
use poddrop_core::i18n::Translator;

/// Callback invoked when the user confirms creating an album and transferring tracks.
pub type CreateAlbumCallback = Rc<dyn Fn(String, String, Option<PathBuf>, Vec<PathBuf>)>;

/// Callback invoked when the user saves edited track metadata.
pub type UpdateTrackCallback = Rc<dyn Fn(u64, UpdateTrackRequest)>;

/// Opens an intuitive modal dialog to create/populate an album with optional cover art and audio files.
pub fn show_create_album_dialog(
    parent: &adw::ApplicationWindow,
    tr: &Translator,
    initial_album: Option<&str>,
    initial_artist: Option<&str>,
    on_submit: CreateAlbumCallback,
) {
    let win = gtk4::Window::builder()
        .title(tr.t("gui.create_album"))
        .transient_for(parent)
        .modal(true)
        .default_width(460)
        .resizable(false)
        .build();

    let header = adw::HeaderBar::new();
    let cancel_btn = gtk4::Button::with_label(tr.t("gui.cancel"));
    let submit_btn = gtk4::Button::with_label(tr.t("gui.select_files"));
    submit_btn.add_css_class("primary-btn");
    header.pack_start(&cancel_btn);
    header.pack_end(&submit_btn);

    let album_entry = adw::EntryRow::builder()
        .title(tr.t("gui.album_name"))
        .build();
    if let Some(a) = initial_album {
        album_entry.set_text(a);
    }
    let artist_entry = adw::EntryRow::builder()
        .title(tr.t("gui.artist_name"))
        .build();
    if let Some(ar) = initial_artist {
        artist_entry.set_text(ar);
    }

    let form_group = adw::PreferencesGroup::new();
    form_group.add(&album_entry);
    form_group.add(&artist_entry);

    let cover_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let cover_label = gtk4::Label::builder()
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .label(tr.t("gui.cover_image"))
        .hexpand(true)
        .css_classes(["dim-label"])
        .build();
    let cover_btn = gtk4::Button::with_label(tr.t("gui.select_cover"));
    let clear_btn = gtk4::Button::with_label(tr.t("gui.clear"));

    let cover_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    cover_row.append(&cover_btn);
    cover_row.append(&clear_btn);

    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    content.append(&form_group);
    content.append(&cover_row);
    content.append(&cover_label);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&content));
    win.set_child(Some(&toolbar));

    {
        let win_c = win.clone();
        cancel_btn.connect_clicked(move |_| win_c.close());
    }

    {
        let win_c = win.clone();
        let cover_c = cover_path.clone();
        let lbl_c = cover_label.clone();
        let dlg_title = tr.t("gui.select_cover").to_string();
        cover_btn.connect_clicked(move |_| {
            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Images (PNG, JPEG)"));
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/jpeg");
            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);

            let dialog = gtk4::FileDialog::builder()
                .title(&dlg_title)
                .filters(&filters)
                .modal(true)
                .build();
            let cover_cb = cover_c.clone();
            let lbl_cb = lbl_c.clone();
            dialog.open(Some(&win_c), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(p) = file.path() {
                        lbl_cb.set_label(&p.display().to_string());
                        *cover_cb.borrow_mut() = Some(p);
                    }
                }
            });
        });
    }

    {
        let cover_c = cover_path.clone();
        let lbl_c = cover_label.clone();
        let default_lbl = tr.t("gui.cover_image").to_string();
        clear_btn.connect_clicked(move |_| {
            *cover_c.borrow_mut() = None;
            lbl_c.set_label(&default_lbl);
        });
    }

    {
        let win_c = win.clone();
        let dlg_title = tr.t("gui.add_music").to_string();
        submit_btn.connect_clicked(move |_| {
            let album = album_entry.text().trim().to_string();
            let artist = artist_entry.text().trim().to_string();
            let art = cover_path.borrow().clone();

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
                .title(&dlg_title)
                .filters(&filters)
                .modal(true)
                .build();
            let win_close = win_c.clone();
            let cb = on_submit.clone();
            dialog.open_multiple(Some(&win_c), gio::Cancellable::NONE, move |res| {
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
                        win_close.close();
                        cb(album, artist, art, paths);
                    }
                }
            });
        });
    }

    win.present();
}

/// Opens a track details viewer & metadata editor dialog.
pub fn show_track_editor_dialog(
    parent: &adw::ApplicationWindow,
    tr: &Translator,
    track: &TrackMetadata,
    can_write: bool,
    on_save: UpdateTrackCallback,
) {
    let win = gtk4::Window::builder()
        .title(&track.title)
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .resizable(false)
        .build();

    let header = adw::HeaderBar::new();
    let cancel_btn = gtk4::Button::with_label(tr.t("gui.cancel"));
    let save_btn = gtk4::Button::with_label(tr.t("gui.save"));
    save_btn.add_css_class("primary-btn");
    save_btn.set_sensitive(can_write);
    header.pack_start(&cancel_btn);
    header.pack_end(&save_btn);

    let title_row = adw::EntryRow::builder()
        .title(tr.t("gui.track_title"))
        .text(&track.title)
        .editable(can_write)
        .build();
    let artist_row = adw::EntryRow::builder()
        .title(tr.t("gui.artist_name"))
        .text(&track.artist)
        .editable(can_write)
        .build();
    let album_row = adw::EntryRow::builder()
        .title(tr.t("gui.album_name"))
        .text(&track.album)
        .editable(can_write)
        .build();
    let genre_row = adw::EntryRow::builder()
        .title(tr.t("gui.track_genre"))
        .text(&track.genre)
        .editable(can_write)
        .build();
    let track_nr_row = adw::EntryRow::builder()
        .title(tr.t("gui.track_number"))
        .text(
            track
                .track_number
                .map(|v| v.to_string())
                .unwrap_or_default(),
        )
        .editable(can_write)
        .build();
    let disc_nr_row = adw::EntryRow::builder()
        .title(tr.t("gui.track_disc"))
        .text(track.disc_number.map(|v| v.to_string()).unwrap_or_default())
        .editable(can_write)
        .build();
    let year_row = adw::EntryRow::builder()
        .title(tr.t("gui.track_year"))
        .text(track.year.map(|v| v.to_string()).unwrap_or_default())
        .editable(can_write)
        .build();

    let meta_group = adw::PreferencesGroup::new();
    meta_group.add(&title_row);
    meta_group.add(&artist_row);
    meta_group.add(&album_row);
    meta_group.add(&genre_row);
    meta_group.add(&track_nr_row);
    meta_group.add(&disc_nr_row);
    meta_group.add(&year_row);

    let duration_sec = track.duration_ms / 1000;
    let size_mb = track.size_bytes as f64 / (1024.0 * 1024.0);
    let details_lbl = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .label(format!(
            "{}: {}:{:02}  |  {} kbps  |  {:.2} MB  |  ID {}",
            tr.t("gui.track_details"),
            duration_sec / 60,
            duration_sec % 60,
            track.bitrate_kbps,
            size_mb,
            track.id
        ))
        .build();

    let new_cover: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let clear_cover: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let cover_status = gtk4::Label::builder()
        .xalign(0.0)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .hexpand(true)
        .css_classes(["dim-label"])
        .label(if track.has_artwork {
            tr.t("gui.cover_keep")
        } else {
            "-"
        })
        .build();

    let pick_cover_btn = gtk4::Button::builder()
        .label(tr.t("gui.select_cover"))
        .sensitive(can_write)
        .build();
    let rm_cover_btn = gtk4::Button::builder()
        .label(tr.t("gui.cover_clear"))
        .sensitive(can_write)
        .build();

    let cover_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    cover_box.append(&pick_cover_btn);
    cover_box.append(&rm_cover_btn);

    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    content.append(&meta_group);
    content.append(&cover_box);
    content.append(&cover_status);
    content.append(&details_lbl);

    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .min_content_height(420)
        .kinetic_scrolling(false)
        .child(&content)
        .build();

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scrolled));
    win.set_child(Some(&toolbar));

    {
        let win_c = win.clone();
        cancel_btn.connect_clicked(move |_| win_c.close());
    }

    {
        let win_c = win.clone();
        let new_c = new_cover.clone();
        let clear_c = clear_cover.clone();
        let lbl_c = cover_status.clone();
        let dlg_title = tr.t("gui.select_cover").to_string();
        pick_cover_btn.connect_clicked(move |_| {
            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Images (PNG, JPEG)"));
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/jpeg");
            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);

            let dialog = gtk4::FileDialog::builder()
                .title(&dlg_title)
                .filters(&filters)
                .modal(true)
                .build();
            let new_cb = new_c.clone();
            let clear_cb = clear_c.clone();
            let lbl_cb = lbl_c.clone();
            dialog.open(Some(&win_c), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(p) = file.path() {
                        lbl_cb.set_label(&p.display().to_string());
                        *new_cb.borrow_mut() = Some(p);
                        *clear_cb.borrow_mut() = false;
                    }
                }
            });
        });
    }

    {
        let new_c = new_cover.clone();
        let clear_c = clear_cover.clone();
        let lbl_c = cover_status.clone();
        let cleared_lbl = tr.t("gui.cover_clear").to_string();
        rm_cover_btn.connect_clicked(move |_| {
            *new_c.borrow_mut() = None;
            *clear_c.borrow_mut() = true;
            lbl_c.set_label(&cleared_lbl);
        });
    }

    {
        let win_c = win.clone();
        let track_id = track.id;
        save_btn.connect_clicked(move |_| {
            let req = UpdateTrackRequest {
                title: Some(title_row.text().to_string()),
                artist: Some(artist_row.text().to_string()),
                album: Some(album_row.text().to_string()),
                genre: Some(genre_row.text().to_string()),
                track_number: track_nr_row.text().trim().parse::<u32>().ok(),
                disc_number: disc_nr_row.text().trim().parse::<u32>().ok(),
                year: year_row.text().trim().parse::<u32>().ok(),
                artwork_path: new_cover.borrow().clone(),
                clear_artwork: *clear_cover.borrow(),
            };
            win_c.close();
            on_save(track_id, req);
        });
    }

    win.present();
}
