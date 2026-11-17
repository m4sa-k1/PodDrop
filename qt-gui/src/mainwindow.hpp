#pragma once

#include <QMainWindow>
#include <QString>
#include <QVector>

#include "dialogs.hpp"
#include "i18n.hpp"

class QAction;
class QCheckBox;
class QComboBox;
class QGroupBox;
class QLabel;
class QLineEdit;
class QListWidget;
class QProgressBar;
class QPushButton;
class QTabWidget;
class QTableWidget;
class QToolBar;
class QToolButton;
class QMenu;

struct DeviceSummary {
    QString mountPath;
    QString modelName;
    QString modelNumber;
    double capacityGb = 0.0;
    QString filesystem;
    bool isReadOnly = false;
    bool hasSysinfoExtended = false;
    QString firewireGuid;
    bool supportsArtwork = false;
    uint32_t trackCount = 0;
    uint32_t playlistCount = 0;
    uint64_t storageTotalBytes = 0;
    uint64_t storageFreeBytes = 0;
    bool hasStorageStats = false;
};

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);

protected:
    void closeEvent(QCloseEvent *event) override;
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dropEvent(QDropEvent *event) override;

private:
    void loadSettings();
    void saveSettings() const;
    void setupUi();
    void applyPalette();
    void applyTranslations();
    void reloadDevice();
    void populateAlbums();
    void populatePlaylists();
    void populateTracks();
    void updateActionStates();
    void updateCoverPreview();

    void openMountFolder();
    void ejectDevice();
    void exportSelectedTracks();
    void restoreFromBackup();
    void addMusicFiles();
    void createAlbum(const QString &initialAlbum = QString(), const QString &initialArtist = QString());
    void addToSelectedAlbum();
    void editSelectedAlbum();
    void removeSelectedAlbum();
    void createPlaylist(const QVector<uint32_t> &initialTrackIds = {});
    void editSelectedPlaylist();
    void removeSelectedPlaylist();
    void addTracksToPlaylistPrompt(const QVector<uint32_t> &trackIds);
    void editSelectedTrack();
    void removeSelectedTrack();

    void startTransferBatch(
        const QStringList &files,
        const QString &coverPath,
        const QString &albumOverride,
        const QString &artistOverride
    );
    void handleDroppedFolder(const QString &folderPath);

    std::optional<TrackItem> selectedTrack() const;
    QVector<TrackItem> selectedTracks() const;
    std::optional<PlaylistItem> selectedPlaylist() const;

    UiLang m_lang;
    bool m_darkTheme = false;
    bool m_busy = false;
    QString m_mountPath;
    QString m_backedUpMountPath;
    bool m_hasDevice = false;
    DeviceSummary m_device;
    QVector<TrackItem> m_tracks;
    QVector<PlaylistItem> m_playlists;
    QString m_selectedAlbum;
    uint64_t m_selectedPlaylistId = 0;
    QString m_statusMsg;

    // Menus & Actions
    QMenu *m_fileMenu = nullptr;
    QMenu *m_editMenu = nullptr;
    QMenu *m_viewMenu = nullptr;
    QMenu *m_helpMenu = nullptr;
    QAction *m_actOpen = nullptr;
    QAction *m_actRefresh = nullptr;
    QAction *m_actExport = nullptr;
    QAction *m_actRestoreBackup = nullptr;
    QAction *m_actEject = nullptr;
    QAction *m_actExit = nullptr;
    QAction *m_actAddMusic = nullptr;
    QAction *m_actCreateAlbum = nullptr;
    QAction *m_actEditAlbum = nullptr;
    QAction *m_actRemoveAlbum = nullptr;
    QAction *m_actNewPlaylist = nullptr;
    QAction *m_actEditPlaylist = nullptr;
    QAction *m_actRemovePlaylist = nullptr;
    QAction *m_actAddToPlaylist = nullptr;
    QAction *m_actEditTrack = nullptr;
    QAction *m_actRemoveTrack = nullptr;
    QAction *m_actLangEn = nullptr;
    QAction *m_actLangJa = nullptr;
    QAction *m_actThemeLight = nullptr;
    QAction *m_actThemeDark = nullptr;
    QAction *m_actSysInfo = nullptr;
    QAction *m_actTerms = nullptr;

    QToolBar *m_toolbar = nullptr;
    QToolButton *m_closeBtn = nullptr;
    QComboBox *m_formatCombo = nullptr;

    // Read-only notice bar
    QLabel *m_roLabel = nullptr;

    // Left pane
    QGroupBox *m_deviceGroup = nullptr;
    QLabel *m_deviceInfoLabel = nullptr;
    QLabel *m_storageLabel = nullptr;
    QProgressBar *m_storageBar = nullptr;
    QTabWidget *m_navTabs = nullptr;
    QWidget *m_albumsTab = nullptr;
    QCheckBox *m_hideSinglesChk = nullptr;
    QListWidget *m_albumList = nullptr;
    QPushButton *m_addToAlbumBtn = nullptr;
    QPushButton *m_editAlbumBtn = nullptr;
    QPushButton *m_removeAlbumBtn = nullptr;
    QWidget *m_playlistsTab = nullptr;
    QListWidget *m_playlistList = nullptr;
    QPushButton *m_newPlaylistBtn = nullptr;
    QPushButton *m_editPlaylistBtn = nullptr;
    QPushButton *m_removePlaylistBtn = nullptr;
    QLabel *m_coverPreview = nullptr;
    QLabel *m_coverCaption = nullptr;

    // Right pane
    QGroupBox *m_libraryGroup = nullptr;
    QLineEdit *m_searchEdit = nullptr;
    QComboBox *m_libraryFilterCombo = nullptr;
    QPushButton *m_editBtn = nullptr;
    QPushButton *m_removeBtn = nullptr;
    QTableWidget *m_trackTable = nullptr;

    // Status bar
    QLabel *m_statusLeft = nullptr;
    QLabel *m_statusRight = nullptr;
};
