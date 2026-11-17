use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::*;

use poddrop_core::backend::{TrackMetadata, UpdateTrackRequest};
use poddrop_core::i18n::Translator;

pub type CreateAlbumCallback = Rc<dyn Fn(String, String, Option<PathBuf>, Vec<PathBuf>)>;
pub type UpdateTrackCallback = Rc<dyn Fn(u64, UpdateTrackRequest)>;
pub type ConfirmCallback = Rc<dyn Fn()>;

/// Classic two-column Windows-style modal dialog to create/populate an album.
pub fn show_create_album_dialog(
    parent: &gtk4::ApplicationWindow,
    tr: &Translator,
    initial_album: Option<&str>,
    initial_artist: Option<&str>,
    on_submit: CreateAlbumCallback,
) {
    let win = gtk4::Window::builder()
        .title(tr.t("gui.create_album"))
        .transient_for(parent)
        .modal(true)
        .default_width(440)
        .resizable(false)
        .build();

    let grid = gtk4::Grid::builder()
        .row_spacing(8)
        .column_spacing(12)
        .build();

    let album_entry = gtk4::Entry::builder().hexpand(true).build();
    if let Some(a) = initial_album {
        album_entry.set_text(a);
    }
    let artist_entry = gtk4::Entry::builder().hexpand(true).build();
    if let Some(ar) = initial_artist {
        artist_entry.set_text(ar);
    }

    let cover_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let cover_entry = gtk4::Entry::builder()
        .hexpand(true)
        .editable(false)
        .placeholder_text("-")
        .build();
    let cover_btn = gtk4::Button::with_label(tr.t("gui.select_cover"));
    let clear_btn = gtk4::Button::with_label(tr.t("gui.clear"));

    let cover_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();
    cover_row.append(&cover_entry);
    cover_row.append(&cover_btn);
    cover_row.append(&clear_btn);

    grid.attach(&make_field_label(tr.t("gui.album_name")), 0, 0, 1, 1);
    grid.attach(&album_entry, 1, 0, 1, 1);
    grid.attach(&make_field_label(tr.t("gui.artist_name")), 0, 1, 1, 1);
    grid.attach(&artist_entry, 1, 1, 1, 1);
    grid.attach(&make_field_label(tr.t("gui.cover_image")), 0, 2, 1, 1);
    grid.attach(&cover_row, 1, 2, 1, 1);

    let cancel_btn = gtk4::Button::with_label(tr.t("gui.cancel"));
    let submit_btn = gtk4::Button::with_label(tr.t("gui.select_files"));
    submit_btn.add_css_class("primary-btn");

    let btn_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::End)
        .margin_top(6)
        .build();
    btn_row.append(&submit_btn);
    btn_row.append(&cancel_btn);

    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(14)
        .margin_end(14)
        .build();
    root.append(&grid);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    root.append(&btn_row);
    win.set_child(Some(&root));

    {
        let win_c = win.clone();
        cancel_btn.connect_clicked(move |_| win_c.close());
    }

    {
        let win_c = win.clone();
        let cover_c = cover_path.clone();
        let entry_c = cover_entry.clone();
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
            let entry_cb = entry_c.clone();
            dialog.open(Some(&win_c), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(p) = file.path() {
                        entry_cb.set_text(&p.display().to_string());
                        *cover_cb.borrow_mut() = Some(p);
                    }
                }
            });
        });
    }

    {
        let cover_c = cover_path.clone();
        let entry_c = cover_entry.clone();
        clear_btn.connect_clicked(move |_| {
            *cover_c.borrow_mut() = None;
            entry_c.set_text("");
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

/// Classic two-column track details viewer & metadata editor dialog.
pub fn show_track_editor_dialog(
    parent: &gtk4::ApplicationWindow,
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

    let grid = gtk4::Grid::builder()
        .row_spacing(8)
        .column_spacing(12)
        .build();

    let title_entry = make_entry(&track.title, can_write);
    let artist_entry = make_entry(&track.artist, can_write);
    let album_entry = make_entry(&track.album, can_write);
    let genre_entry = make_entry(&track.genre, can_write);
    let track_nr_entry = make_entry(
        &track
            .track_number
            .map(|v| v.to_string())
            .unwrap_or_default(),
        can_write,
    );
    let disc_nr_entry = make_entry(
        &track.disc_number.map(|v| v.to_string()).unwrap_or_default(),
        can_write,
    );
    let year_entry = make_entry(
        &track.year.map(|v| v.to_string()).unwrap_or_default(),
        can_write,
    );

    let num_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .homogeneous(true)
        .build();
    num_row.append(&track_nr_entry);
    num_row.append(&disc_nr_entry);
    num_row.append(&year_entry);

    let new_cover: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let clear_cover: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let cover_entry = gtk4::Entry::builder()
        .hexpand(true)
        .editable(false)
        .text(if track.has_artwork {
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

    let cover_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();
    cover_row.append(&cover_entry);
    cover_row.append(&pick_cover_btn);
    cover_row.append(&rm_cover_btn);

    let duration_sec = track.duration_ms / 1000;
    let size_mb = track.size_bytes as f64 / (1024.0 * 1024.0);
    let details_lbl = gtk4::Label::builder()
        .xalign(0.0)
        .css_classes(["muted"])
        .label(format!(
            "{}:{:02}  |  {} kbps  |  {:.2} MB  |  ID {}",
            duration_sec / 60,
            duration_sec % 60,
            track.bitrate_kbps,
            size_mb,
            track.id
        ))
        .build();

    grid.attach(&make_field_label(tr.t("gui.track_title")), 0, 0, 1, 1);
    grid.attach(&title_entry, 1, 0, 1, 1);
    grid.attach(&make_field_label(tr.t("gui.artist_name")), 0, 1, 1, 1);
    grid.attach(&artist_entry, 1, 1, 1, 1);
    grid.attach(&make_field_label(tr.t("gui.album_name")), 0, 2, 1, 1);
    grid.attach(&album_entry, 1, 2, 1, 1);
    grid.attach(&make_field_label(tr.t("gui.track_genre")), 0, 3, 1, 1);
    grid.attach(&genre_entry, 1, 3, 1, 1);
    grid.attach(
        &make_field_label(&format!(
            "{} / {} / {}",
            tr.t("gui.track_number"),
            tr.t("gui.track_disc"),
            tr.t("gui.track_year")
        )),
        0,
        4,
        1,
        1,
    );
    grid.attach(&num_row, 1, 4, 1, 1);
    grid.attach(&make_field_label(tr.t("gui.cover_image")), 0, 5, 1, 1);
    grid.attach(&cover_row, 1, 5, 1, 1);
    grid.attach(&make_field_label(tr.t("gui.track_details")), 0, 6, 1, 1);
    grid.attach(&details_lbl, 1, 6, 1, 1);

    let cancel_btn = gtk4::Button::with_label(tr.t("gui.cancel"));
    let save_btn = gtk4::Button::builder()
        .label(tr.t("gui.save"))
        .sensitive(can_write)
        .css_classes(["primary-btn"])
        .build();

    let btn_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::End)
        .margin_top(4)
        .build();
    btn_row.append(&save_btn);
    btn_row.append(&cancel_btn);

    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(14)
        .margin_end(14)
        .build();
    root.append(&grid);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    root.append(&btn_row);
    win.set_child(Some(&root));

    {
        let win_c = win.clone();
        cancel_btn.connect_clicked(move |_| win_c.close());
    }

    {
        let win_c = win.clone();
        let new_c = new_cover.clone();
        let clear_c = clear_cover.clone();
        let entry_c = cover_entry.clone();
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
            let entry_cb = entry_c.clone();
            dialog.open(Some(&win_c), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(p) = file.path() {
                        entry_cb.set_text(&p.display().to_string());
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
        let entry_c = cover_entry.clone();
        let cleared_lbl = tr.t("gui.cover_clear").to_string();
        rm_cover_btn.connect_clicked(move |_| {
            *new_c.borrow_mut() = None;
            *clear_c.borrow_mut() = true;
            entry_c.set_text(&cleared_lbl);
        });
    }

    {
        let win_c = win.clone();
        let track_id = track.id;
        save_btn.connect_clicked(move |_| {
            let req = UpdateTrackRequest {
                title: Some(title_entry.text().to_string()),
                artist: Some(artist_entry.text().to_string()),
                album: Some(album_entry.text().to_string()),
                genre: Some(genre_entry.text().to_string()),
                track_number: track_nr_entry.text().trim().parse::<u32>().ok(),
                disc_number: disc_nr_entry.text().trim().parse::<u32>().ok(),
                year: year_entry.text().trim().parse::<u32>().ok(),
                artwork_path: new_cover.borrow().clone(),
                clear_artwork: *clear_cover.borrow(),
            };
            win_c.close();
            on_save(track_id, req);
        });
    }

    win.present();
}

/// Simple classic yes/no confirmation dialog.
pub fn show_confirm_dialog(
    parent: &gtk4::ApplicationWindow,
    title: &str,
    message: &str,
    confirm_label: &str,
    cancel_label: &str,
    on_confirm: ConfirmCallback,
) {
    let win = gtk4::Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(360)
        .resizable(false)
        .build();

    let msg_lbl = gtk4::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .label(message)
        .build();
    let ok_btn = gtk4::Button::builder()
        .label(confirm_label)
        .css_classes(["primary-btn"])
        .build();
    let cancel_btn = gtk4::Button::with_label(cancel_label);

    let btn_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::End)
        .build();
    btn_row.append(&ok_btn);
    btn_row.append(&cancel_btn);

    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(14)
        .margin_end(14)
        .build();
    root.append(&msg_lbl);
    root.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    root.append(&btn_row);
    win.set_child(Some(&root));

    {
        let win_c = win.clone();
        cancel_btn.connect_clicked(move |_| win_c.close());
    }
    {
        let win_c = win.clone();
        ok_btn.connect_clicked(move |_| {
            win_c.close();
            on_confirm();
        });
    }

    win.present();
}

fn make_field_label(text: &str) -> gtk4::Label {
    gtk4::Label::builder()
        .xalign(1.0)
        .label(text)
        .css_classes(["muted"])
        .build()
}

fn make_entry(text: &str, editable: bool) -> gtk4::Entry {
    gtk4::Entry::builder()
        .hexpand(true)
        .text(text)
        .editable(editable)
        .sensitive(editable)
        .build()
}
