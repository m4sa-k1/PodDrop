#pragma once

#include <QString>
#include <unordered_map>

enum class UiLang {
    En,
    Ja,
};

inline UiLang detectSystemLang() {
    const QByteArray lang = qgetenv("LANG");
    if (lang.toLower().startsWith("ja")) {
        return UiLang::Ja;
    }
    return UiLang::En;
}

inline QString trText(UiLang lang, const char *key) {
    static const std::unordered_map<std::string, const char *> en = {
        {"menu.file", "&File"},
        {"menu.edit", "&Edit"},
        {"menu.view", "&View"},
        {"act.open", "Open iPod Folder..."},
        {"act.refresh", "Refresh"},
        {"act.exit", "Exit"},
        {"act.add_music", "Add Music..."},
        {"act.create_album", "Create Album..."},
        {"act.add_to_album", "Add to Selected Album..."},
        {"act.edit_album", "Edit Album..."},
        {"act.edit_track", "Edit Track..."},
        {"act.remove_track", "Remove Track"},
        {"act.lang_en", "English"},
        {"act.lang_ja", "日本語"},
        {"act.theme_light", "Classic Light (White)"},
        {"act.theme_dark", "Classic Dark (Black)"},
        {"grp.device", "Device"},
        {"grp.albums", "Albums"},
        {"grp.library", "Library"},
        {"lbl.all_albums", "All Tracks"},
        {"lbl.no_device", "No iPod detected.\nUse File -> Open iPod Folder..."},
        {"lbl.readonly", "Mounted read-only: write operations (add, edit, remove) are disabled."},
        {"lbl.storage", "Storage"},
        {"lbl.search_hint", "Filter by title, artist, or album..."},
        {"lbl.ready", "Ready"},
        {"lbl.working", "Working..."},
        {"lbl.transfer_done", "Transfer complete"},
        {"lbl.update_done", "Track updated"},
        {"lbl.album_updated", "Album updated"},
        {"col.num", "#"},
        {"col.title", "Title"},
        {"col.artist", "Artist"},
        {"col.album", "Album"},
        {"col.genre", "Genre"},
        {"col.time", "Time"},
        {"col.bitrate", "Bitrate"},
        {"col.art", "Art"},
        {"dlg.album_name", "Album:"},
        {"dlg.artist_name", "Artist:"},
        {"dlg.cover", "Cover Image:"},
        {"dlg.browse", "Browse..."},
        {"dlg.clear", "Clear"},
        {"dlg.select_files", "Select Audio Files..."},
        {"dlg.title", "Title:"},
        {"dlg.genre", "Genre:"},
        {"dlg.track_num", "Track #:"},
        {"dlg.disc_num", "Disc #:"},
        {"dlg.year", "Year:"},
        {"dlg.details", "Details:"},
        {"dlg.cover_keep", "(Keep existing)"},
        {"dlg.cover_remove", "Remove Cover"},
        {"dlg.remove_confirm", "Remove the selected track from iPod?"},
    };

    static const std::unordered_map<std::string, const char *> ja = {
        {"menu.file", "ファイル(&F)"},
        {"menu.edit", "編集(&E)"},
        {"menu.view", "表示(&V)"},
        {"act.open", "iPodフォルダを開く..."},
        {"act.refresh", "再読み込み"},
        {"act.exit", "終了"},
        {"act.add_music", "曲を追加..."},
        {"act.create_album", "アルバムを作成..."},
        {"act.add_to_album", "選択アルバムへ追加..."},
        {"act.edit_album", "アルバムを編集..."},
        {"act.edit_track", "曲情報を編集..."},
        {"act.remove_track", "曲を削除"},
        {"act.lang_en", "English"},
        {"act.lang_ja", "日本語"},
        {"act.theme_light", "クラシック ライト (白)"},
        {"act.theme_dark", "クラシック ダーク (黒)"},
        {"grp.device", "デバイス情報"},
        {"grp.albums", "アルバム"},
        {"grp.library", "ライブラリ"},
        {"lbl.all_albums", "すべての曲"},
        {"lbl.no_device", "iPod が見つかりません。\n「ファイル -> iPodフォルダを開く」から指定してください。"},
        {"lbl.readonly", "読み取り専用でマウント中のため、追加・編集・削除は無効化されています。"},
        {"lbl.storage", "容量"},
        {"lbl.search_hint", "タイトル・アーティスト・アルバムで絞り込み..."},
        {"lbl.ready", "準備完了"},
        {"lbl.working", "処理中..."},
        {"lbl.transfer_done", "転送完了"},
        {"lbl.update_done", "曲情報を更新しました"},
        {"lbl.album_updated", "アルバム情報を更新しました"},
        {"col.num", "#"},
        {"col.title", "タイトル"},
        {"col.artist", "アーティスト"},
        {"col.album", "アルバム"},
        {"col.genre", "ジャンル"},
        {"col.time", "時間"},
        {"col.bitrate", "ビットレート"},
        {"col.art", "画像"},
        {"dlg.album_name", "アルバム名:"},
        {"dlg.artist_name", "アーティスト:"},
        {"dlg.cover", "カバー画像:"},
        {"dlg.browse", "参照..."},
        {"dlg.clear", "クリア"},
        {"dlg.select_files", "曲ファイルを選択..."},
        {"dlg.title", "タイトル:"},
        {"dlg.genre", "ジャンル:"},
        {"dlg.track_num", "トラック番号:"},
        {"dlg.disc_num", "ディスク番号:"},
        {"dlg.year", "年:"},
        {"dlg.details", "詳細:"},
        {"dlg.cover_keep", "(現在の画像を維持)"},
        {"dlg.cover_remove", "画像を削除"},
        {"dlg.remove_confirm", "選択した曲を iPod から削除しますか？"},
    };

    const auto &table = (lang == UiLang::Ja) ? ja : en;
    if (auto it = table.find(key); it != table.end()) {
        return QString::fromUtf8(it->second);
    }
    if (auto it = en.find(key); it != en.end()) {
        return QString::fromUtf8(it->second);
    }
    return QString::fromUtf8(key);
}
