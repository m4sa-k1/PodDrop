#include "mainwindow.hpp"

#include <QAction>
#include <QActionGroup>
#include <QApplication>
#include <QComboBox>
#include <QDir>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QFileDialog>
#include <QFileInfo>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QMimeData>
#include <QPalette>
#include <QProgressBar>
#include <QPushButton>
#include <QSplitter>
#include <QStatusBar>
#include <QStyle>
#include <QTableWidget>
#include <QToolBar>
#include <QToolButton>
#include <QVBoxLayout>
#include <map>
#include <thread>

#include "poddrop.h"

namespace {

QString takeCString(char *p) {
    if (!p) return QString();
    QString s = QString::fromUtf8(p);
    poddrop_free_string(p);
    return s;
}

bool isSupportedAudioExt(const QString &path) {
    const QString ext = QFileInfo(path).suffix().toLower();
    return ext == "mp3" || ext == "m4a" || ext == "mp4" || ext == "flac" || ext == "ogg" ||
           ext == "opus" || ext == "wav";
}

} // namespace

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent), m_lang(detectSystemLang()) {
    setAcceptDrops(true);
    resize(1020, 660);
    setWindowTitle(QString("PodDrop v%1").arg(QString::fromUtf8(poddrop_app_version())));

    setupUi();
    applyPalette();
    applyTranslations();

    if (char *detected = poddrop_discover_first_ipod()) {
        m_mountPath = takeCString(detected);
    }
    reloadDevice();
}

void MainWindow::setupUi() {
    auto *st = style();

    // Menus
    m_fileMenu = menuBar()->addMenu(QString());
    m_actOpen = m_fileMenu->addAction(st->standardIcon(QStyle::SP_DirOpenIcon), QString());
    m_actRefresh = m_fileMenu->addAction(st->standardIcon(QStyle::SP_BrowserReload), QString());
    m_actEject = m_fileMenu->addAction(
        QIcon::fromTheme("media-eject", st->standardIcon(QStyle::SP_DriveHDIcon)),
        QString()
    );
    m_fileMenu->addSeparator();
    m_actExit = m_fileMenu->addAction(QString());

    m_editMenu = menuBar()->addMenu(QString());
    m_actAddMusic = m_editMenu->addAction(st->standardIcon(QStyle::SP_FileDialogNewFolder), QString());
    m_actCreateAlbum = m_editMenu->addAction(st->standardIcon(QStyle::SP_DirIcon), QString());
    m_actEditAlbum = m_editMenu->addAction(QString());
    m_editMenu->addSeparator();
    m_actEditTrack = m_editMenu->addAction(QString());
    m_actRemoveTrack = m_editMenu->addAction(st->standardIcon(QStyle::SP_TrashIcon), QString());

    m_viewMenu = menuBar()->addMenu(QString());
    auto *langGroup = new QActionGroup(this);
    m_actLangEn = m_viewMenu->addAction(QString());
    m_actLangEn->setCheckable(true);
    m_actLangJa = m_viewMenu->addAction(QString());
    m_actLangJa->setCheckable(true);
    langGroup->addAction(m_actLangEn);
    langGroup->addAction(m_actLangJa);
    m_actLangEn->setChecked(m_lang == UiLang::En);
    m_actLangJa->setChecked(m_lang == UiLang::Ja);

    m_viewMenu->addSeparator();
    auto *themeGroup = new QActionGroup(this);
    m_actThemeLight = m_viewMenu->addAction(QString());
    m_actThemeLight->setCheckable(true);
    m_actThemeDark = m_viewMenu->addAction(QString());
    m_actThemeDark->setCheckable(true);
    themeGroup->addAction(m_actThemeLight);
    themeGroup->addAction(m_actThemeDark);
    m_actThemeLight->setChecked(!m_darkTheme);
    m_actThemeDark->setChecked(m_darkTheme);

    // Top-right close button on menu bar
    m_closeBtn = new QToolButton(this);
    m_closeBtn->setAutoRaise(true);
    m_closeBtn->setIcon(st->standardIcon(QStyle::SP_TitleBarCloseButton));
    m_closeBtn->setFixedSize(26, 22);
    menuBar()->setCornerWidget(m_closeBtn, Qt::TopRightCorner);
    connect(m_closeBtn, &QToolButton::clicked, this, &QWidget::close);

    // Toolbar
    m_toolbar = addToolBar("MainToolbar");
    m_toolbar->setMovable(false);
    m_toolbar->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_toolbar->addAction(m_actOpen);
    m_toolbar->addAction(m_actRefresh);
    m_toolbar->addAction(m_actEject);
    m_toolbar->addSeparator();
    m_toolbar->addAction(m_actAddMusic);
    m_toolbar->addAction(m_actCreateAlbum);
    m_toolbar->addSeparator();

    m_formatCombo = new QComboBox(this);
    m_formatCombo->addItems({"AAC (.m4a)", "ALAC (.m4a)", "MP3"});
    m_toolbar->addWidget(m_formatCombo);

    // Central widget
    auto *central = new QWidget(this);
    auto *centralLayout = new QVBoxLayout(central);
    centralLayout->setContentsMargins(6, 6, 6, 6);
    centralLayout->setSpacing(6);

    m_roLabel = new QLabel(central);
    m_roLabel->setFrameStyle(QFrame::Panel | QFrame::Sunken);
    m_roLabel->setMargin(5);
    m_roLabel->setVisible(false);
    centralLayout->addWidget(m_roLabel);

    auto *splitter = new QSplitter(Qt::Horizontal, central);

    // Left Pane
    auto *leftWidget = new QWidget(splitter);
    auto *leftLayout = new QVBoxLayout(leftWidget);
    leftLayout->setContentsMargins(0, 0, 0, 0);
    leftLayout->setSpacing(6);

    m_deviceGroup = new QGroupBox(leftWidget);
    auto *devLayout = new QVBoxLayout(m_deviceGroup);
    m_deviceInfoLabel = new QLabel(m_deviceGroup);
    m_deviceInfoLabel->setWordWrap(true);
    m_deviceInfoLabel->setTextInteractionFlags(Qt::TextSelectableByMouse);
    m_storageLabel = new QLabel(m_deviceGroup);
    m_storageBar = new QProgressBar(m_deviceGroup);
    m_storageBar->setTextVisible(false);
    m_storageBar->setFixedHeight(14);
    devLayout->addWidget(m_deviceInfoLabel);
    devLayout->addWidget(m_storageLabel);
    devLayout->addWidget(m_storageBar);

    m_albumsGroup = new QGroupBox(leftWidget);
    auto *albLayout = new QVBoxLayout(m_albumsGroup);
    m_albumList = new QListWidget(m_albumsGroup);
    m_albumList->setContextMenuPolicy(Qt::CustomContextMenu);
    m_addToAlbumBtn = new QPushButton(m_albumsGroup);
    m_editAlbumBtn = new QPushButton(m_albumsGroup);
    auto *albBtnRow = new QHBoxLayout();
    albBtnRow->addWidget(m_addToAlbumBtn);
    albBtnRow->addWidget(m_editAlbumBtn);
    albLayout->addWidget(m_albumList, 1);
    albLayout->addLayout(albBtnRow);

    leftLayout->addWidget(m_deviceGroup);
    leftLayout->addWidget(m_albumsGroup, 1);

    auto *previewRow = new QHBoxLayout();
    m_coverPreview = new QLabel(leftWidget);
    m_coverPreview->setFixedSize(84, 84);
    m_coverPreview->setAlignment(Qt::AlignCenter);
    m_coverPreview->setFrameStyle(QFrame::Box | QFrame::Plain);
    m_coverCaption = new QLabel(leftWidget);
    m_coverCaption->setWordWrap(true);
    previewRow->addWidget(m_coverPreview);
    previewRow->addWidget(m_coverCaption, 1);
    leftLayout->addLayout(previewRow);

    // Right Pane
    m_libraryGroup = new QGroupBox(splitter);
    auto *rightLayout = new QVBoxLayout(m_libraryGroup);

    m_searchEdit = new QLineEdit(m_libraryGroup);
    m_searchEdit->setClearButtonEnabled(true);
    m_editBtn = new QPushButton(m_libraryGroup);
    m_removeBtn = new QPushButton(m_libraryGroup);

    auto *filterRow = new QHBoxLayout();
    filterRow->addWidget(m_searchEdit, 1);
    filterRow->addWidget(m_editBtn);
    filterRow->addWidget(m_removeBtn);

    m_trackTable = new QTableWidget(0, 8, m_libraryGroup);
    m_trackTable->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_trackTable->setSelectionMode(QAbstractItemView::SingleSelection);
    m_trackTable->setEditTriggers(QAbstractItemView::NoEditTriggers);
    m_trackTable->setAlternatingRowColors(true);
    m_trackTable->setSortingEnabled(true);
    m_trackTable->setContextMenuPolicy(Qt::CustomContextMenu);
    m_trackTable->verticalHeader()->setVisible(false);
    m_trackTable->horizontalHeader()->setSectionResizeMode(QHeaderView::Interactive);
    m_trackTable->horizontalHeader()->setStretchLastSection(true);
    m_trackTable->horizontalHeader()->setDefaultAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    m_trackTable->setColumnWidth(0, 42);
    m_trackTable->setColumnWidth(1, 200);
    m_trackTable->setColumnWidth(2, 135);
    m_trackTable->setColumnWidth(3, 145);
    m_trackTable->setColumnWidth(4, 90);
    m_trackTable->setColumnWidth(5, 74);
    m_trackTable->setColumnWidth(6, 115);

    rightLayout->addLayout(filterRow);
    rightLayout->addWidget(m_trackTable, 1);

    splitter->addWidget(leftWidget);
    splitter->addWidget(m_libraryGroup);
    splitter->setStretchFactor(0, 0);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({270, 740});

    centralLayout->addWidget(splitter, 1);
    setCentralWidget(central);

    // Status bar
    m_statusLeft = new QLabel(this);
    m_statusRight = new QLabel(this);
    statusBar()->addWidget(m_statusLeft, 1);
    statusBar()->addPermanentWidget(m_statusRight);

    // Connections
    connect(m_actOpen, &QAction::triggered, this, &MainWindow::openMountFolder);
    connect(m_actRefresh, &QAction::triggered, this, [this]() {
        if (m_mountPath.isEmpty() || !QDir(m_mountPath).exists()) {
            m_mountPath.clear();
            if (char *detected = poddrop_discover_first_ipod()) {
                m_mountPath = takeCString(detected);
            }
        }
        reloadDevice();
    });
    connect(m_actEject, &QAction::triggered, this, &MainWindow::ejectDevice);
    connect(m_actExit, &QAction::triggered, this, &QWidget::close);
    connect(m_actAddMusic, &QAction::triggered, this, &MainWindow::addMusicFiles);
    connect(m_actCreateAlbum, &QAction::triggered, this, [this]() { createAlbum(); });
    connect(m_actEditAlbum, &QAction::triggered, this, &MainWindow::editSelectedAlbum);
    connect(m_actEditTrack, &QAction::triggered, this, &MainWindow::editSelectedTrack);
    connect(m_actRemoveTrack, &QAction::triggered, this, &MainWindow::removeSelectedTrack);

    connect(m_actLangEn, &QAction::triggered, this, [this]() {
        m_lang = UiLang::En;
        applyTranslations();
        populateAlbums();
        populateTracks();
    });
    connect(m_actLangJa, &QAction::triggered, this, [this]() {
        m_lang = UiLang::Ja;
        applyTranslations();
        populateAlbums();
        populateTracks();
    });
    connect(m_actThemeLight, &QAction::triggered, this, [this]() {
        m_darkTheme = false;
        applyPalette();
    });
    connect(m_actThemeDark, &QAction::triggered, this, [this]() {
        m_darkTheme = true;
        applyPalette();
    });

    connect(m_addToAlbumBtn, &QPushButton::clicked, this, &MainWindow::addToSelectedAlbum);
    connect(m_editAlbumBtn, &QPushButton::clicked, this, &MainWindow::editSelectedAlbum);
    connect(m_editBtn, &QPushButton::clicked, this, &MainWindow::editSelectedTrack);
    connect(m_removeBtn, &QPushButton::clicked, this, &MainWindow::removeSelectedTrack);

    connect(m_searchEdit, &QLineEdit::textChanged, this, [this]() { populateTracks(); });
    connect(m_albumList, &QListWidget::currentRowChanged, this, [this](int row) {
        if (row <= 0) {
            m_selectedAlbum.clear();
        } else if (auto *item = m_albumList->item(row)) {
            m_selectedAlbum = item->data(Qt::UserRole).toString();
        }
        populateTracks();
    });
    connect(m_albumList, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *item) {
        if (item && !item->data(Qt::UserRole).toString().isEmpty()) {
            editSelectedAlbum();
        }
    });
    connect(m_albumList, &QListWidget::customContextMenuRequested, this, [this](const QPoint &pos) {
        if (m_selectedAlbum.isEmpty()) return;
        QMenu menu(this);
        menu.addAction(m_actEditAlbum);
        menu.exec(m_albumList->viewport()->mapToGlobal(pos));
    });

    connect(m_trackTable, &QTableWidget::itemSelectionChanged, this, [this]() {
        updateActionStates();
        updateCoverPreview();
    });
    connect(m_trackTable, &QTableWidget::cellDoubleClicked, this, [this](int, int) {
        editSelectedTrack();
    });
    connect(m_trackTable, &QTableWidget::customContextMenuRequested, this, [this](const QPoint &pos) {
        if (!selectedTrack().has_value()) return;
        QMenu menu(this);
        menu.addAction(m_actEditTrack);
        menu.addAction(m_actRemoveTrack);
        menu.exec(m_trackTable->viewport()->mapToGlobal(pos));
    });
}

void MainWindow::applyPalette() {
    QPalette pal;
    if (m_darkTheme) {
        pal.setColor(QPalette::Window, QColor(0x1b, 0x1b, 0x1b));
        pal.setColor(QPalette::WindowText, QColor(0xee, 0xee, 0xee));
        pal.setColor(QPalette::Base, QColor(0x12, 0x12, 0x12));
        pal.setColor(QPalette::AlternateBase, QColor(0x19, 0x19, 0x19));
        pal.setColor(QPalette::Text, QColor(0xee, 0xee, 0xee));
        pal.setColor(QPalette::Button, QColor(0x26, 0x26, 0x26));
        pal.setColor(QPalette::ButtonText, QColor(0xee, 0xee, 0xee));
        pal.setColor(QPalette::Highlight, QColor(0xe0, 0xe0, 0xe0));
        pal.setColor(QPalette::HighlightedText, QColor(0x11, 0x11, 0x11));
        pal.setColor(QPalette::Mid, QColor(0x3a, 0x3a, 0x3a));
        pal.setColor(QPalette::Dark, QColor(0x0d, 0x0d, 0x0d));
    } else {
        pal.setColor(QPalette::Window, QColor(0xf0, 0xf0, 0xf0));
        pal.setColor(QPalette::WindowText, QColor(0x11, 0x11, 0x11));
        pal.setColor(QPalette::Base, QColor(0xff, 0xff, 0xff));
        pal.setColor(QPalette::AlternateBase, QColor(0xf7, 0xf7, 0xf7));
        pal.setColor(QPalette::Text, QColor(0x11, 0x11, 0x11));
        pal.setColor(QPalette::Button, QColor(0xe4, 0xe4, 0xe4));
        pal.setColor(QPalette::ButtonText, QColor(0x11, 0x11, 0x11));
        pal.setColor(QPalette::Highlight, QColor(0x22, 0x22, 0x22));
        pal.setColor(QPalette::HighlightedText, QColor(0xff, 0xff, 0xff));
        pal.setColor(QPalette::Mid, QColor(0xc0, 0xc0, 0xc0));
        pal.setColor(QPalette::Dark, QColor(0x9a, 0x9a, 0x9a));
    }
    qApp->setPalette(pal);
}

void MainWindow::applyTranslations() {
    m_fileMenu->setTitle(trText(m_lang, "menu.file"));
    m_editMenu->setTitle(trText(m_lang, "menu.edit"));
    m_viewMenu->setTitle(trText(m_lang, "menu.view"));

    m_actOpen->setText(trText(m_lang, "act.open"));
    m_actRefresh->setText(trText(m_lang, "act.refresh"));
    m_actEject->setText(trText(m_lang, "act.eject"));
    m_actExit->setText(trText(m_lang, "act.exit"));
    if (m_closeBtn) {
        m_closeBtn->setToolTip(trText(m_lang, "act.exit"));
    }
    m_actAddMusic->setText(trText(m_lang, "act.add_music"));
    m_actCreateAlbum->setText(trText(m_lang, "act.create_album"));
    m_actEditAlbum->setText(trText(m_lang, "act.edit_album"));
    m_actEditTrack->setText(trText(m_lang, "act.edit_track"));
    m_actRemoveTrack->setText(trText(m_lang, "act.remove_track"));
    m_actLangEn->setText(trText(m_lang, "act.lang_en"));
    m_actLangJa->setText(trText(m_lang, "act.lang_ja"));
    m_actThemeLight->setText(trText(m_lang, "act.theme_light"));
    m_actThemeDark->setText(trText(m_lang, "act.theme_dark"));

    m_roLabel->setText(trText(m_lang, "lbl.readonly"));
    m_deviceGroup->setTitle(trText(m_lang, "grp.device"));
    m_albumsGroup->setTitle(trText(m_lang, "grp.albums"));
    m_libraryGroup->setTitle(trText(m_lang, "grp.library"));
    m_addToAlbumBtn->setText(trText(m_lang, "act.add_to_album"));
    m_editAlbumBtn->setText(trText(m_lang, "act.edit_album"));

    m_searchEdit->setPlaceholderText(trText(m_lang, "lbl.search_hint"));
    m_editBtn->setText(trText(m_lang, "act.edit_track"));
    m_removeBtn->setText(trText(m_lang, "act.remove_track"));

    m_trackTable->setHorizontalHeaderLabels({
        trText(m_lang, "col.num"),
        trText(m_lang, "col.title"),
        trText(m_lang, "col.artist"),
        trText(m_lang, "col.album"),
        trText(m_lang, "col.genre"),
        trText(m_lang, "col.time"),
        trText(m_lang, "col.bitrate"),
        trText(m_lang, "col.art"),
    });

    if (m_hasDevice) {
        const QString modelNum = m_device.modelNumber.isEmpty() ? "-" : m_device.modelNumber;
        const QString guid = m_device.firewireGuid.isEmpty() ? "-" : m_device.firewireGuid;
        m_deviceInfoLabel->setText(
            QString("%1 (%2)\nFS: %3\nGUID: %4\n%5")
                .arg(m_device.modelName, modelNum, m_device.filesystem, guid, m_device.mountPath)
        );
        if (m_device.hasStorageStats && m_device.storageTotalBytes > 0) {
            const uint64_t used = m_device.storageTotalBytes - m_device.storageFreeBytes;
            const double gb = 1024.0 * 1024.0 * 1024.0;
            const int pct = static_cast<int>((used * 100) / m_device.storageTotalBytes);
            m_storageLabel->setText(
                QString("%1: %2 / %3 GB (%4%)")
                    .arg(trText(m_lang, "lbl.storage"))
                    .arg(static_cast<double>(used) / gb, 0, 'f', 2)
                    .arg(static_cast<double>(m_device.storageTotalBytes) / gb, 0, 'f', 2)
                    .arg(pct)
            );
            m_storageBar->setValue(pct);
        } else {
            m_storageLabel->clear();
            m_storageBar->setValue(0);
        }
    } else {
        m_deviceInfoLabel->setText(trText(m_lang, "lbl.no_device"));
        m_storageLabel->clear();
        m_storageBar->setValue(0);
    }

    m_statusLeft->setText(m_busy ? trText(m_lang, "lbl.working")
                                 : (m_statusMsg.isEmpty() ? trText(m_lang, "lbl.ready") : m_statusMsg));
}

void MainWindow::reloadDevice() {
    m_hasDevice = false;
    m_tracks.clear();

    if (!m_mountPath.isEmpty()) {
        const QByteArray mpBytes = m_mountPath.toUtf8();
        PodDropDeviceInfo rawInfo{};
        char *err = nullptr;
        if (poddrop_inspect_device(mpBytes.constData(), &rawInfo, &err) == 0) {
            m_hasDevice = true;
            m_device.mountPath = takeCString(rawInfo.mount_path);
            m_device.modelName = takeCString(rawInfo.model_name);
            m_device.modelNumber = takeCString(rawInfo.model_number);
            m_device.capacityGb = rawInfo.capacity_gb;
            m_device.filesystem = takeCString(rawInfo.filesystem);
            m_device.isReadOnly = rawInfo.is_read_only != 0;
            m_device.hasSysinfoExtended = rawInfo.has_sysinfo_extended != 0;
            m_device.firewireGuid = takeCString(rawInfo.firewire_guid);
            m_device.supportsArtwork = rawInfo.supports_artwork != 0;
            m_device.trackCount = rawInfo.track_count;
            m_device.playlistCount = rawInfo.playlist_count;
            m_device.storageTotalBytes = rawInfo.storage_total_bytes;
            m_device.storageFreeBytes = rawInfo.storage_free_bytes;
            m_device.hasStorageStats = rawInfo.has_storage_stats != 0;

            PodDropTrack *rawTracks = nullptr;
            size_t count = 0;
            char *listErr = nullptr;
            if (poddrop_list_tracks(mpBytes.constData(), &rawTracks, &count, &listErr) == 0) {
                for (size_t i = 0; i < count; ++i) {
                    const auto &t = rawTracks[i];
                    m_tracks.push_back(TrackItem{
                        t.id,
                        QString::fromUtf8(t.title ? t.title : ""),
                        QString::fromUtf8(t.artist ? t.artist : ""),
                        QString::fromUtf8(t.album ? t.album : ""),
                        QString::fromUtf8(t.genre ? t.genre : ""),
                        t.track_number,
                        t.disc_number,
                        t.year,
                        t.duration_ms,
                        t.bitrate_kbps,
                        t.size_bytes,
                        t.has_artwork != 0,
                    });
                }
                poddrop_free_tracks(rawTracks, count);
            } else if (listErr) {
                m_statusMsg = takeCString(listErr);
            }
        } else if (err) {
            m_statusMsg = takeCString(err);
        }
    }

    m_roLabel->setVisible(m_hasDevice && m_device.isReadOnly);
    applyTranslations();
    populateAlbums();
    populateTracks();
    updateActionStates();
}

void MainWindow::populateAlbums() {
    std::map<QString, int> counts;
    for (const auto &t : m_tracks) {
        const QString alb = t.album.isEmpty() ? QStringLiteral("Unknown Album") : t.album;
        counts[alb]++;
    }

    m_albumList->blockSignals(true);
    m_albumList->clear();

    auto *allItem = new QListWidgetItem(
        QString("%1 (%2)").arg(trText(m_lang, "lbl.all_albums")).arg(m_tracks.size())
    );
    allItem->setData(Qt::UserRole, QString());
    m_albumList->addItem(allItem);

    int selectRow = 0;
    int rowIdx = 1;
    for (const auto &[alb, cnt] : counts) {
        auto *item = new QListWidgetItem(QString("%1 (%2)").arg(alb).arg(cnt));
        item->setData(Qt::UserRole, alb);
        m_albumList->addItem(item);
        if (alb == m_selectedAlbum) {
            selectRow = rowIdx;
        }
        rowIdx++;
    }
    if (selectRow == 0) {
        m_selectedAlbum.clear();
    }
    m_albumList->setCurrentRow(selectRow);
    m_albumList->blockSignals(false);

    if (m_hasDevice) {
        m_statusRight->setText(
            QString("%1: %2  |  %3: %4")
                .arg(trText(m_lang, "grp.library"))
                .arg(m_tracks.size())
                .arg(trText(m_lang, "grp.albums"))
                .arg(counts.size())
        );
    } else {
        m_statusRight->clear();
    }
}

void MainWindow::populateTracks() {
    const QString query = m_searchEdit->text().trimmed().toLower();
    m_trackTable->setSortingEnabled(false);
    m_trackTable->setRowCount(0);

    for (const auto &t : m_tracks) {
        const QString alb = t.album.isEmpty() ? QStringLiteral("Unknown Album") : t.album;
        if (!m_selectedAlbum.isEmpty() && alb != m_selectedAlbum) {
            continue;
        }
        if (!query.isEmpty()) {
            const bool matched = t.title.toLower().contains(query) ||
                                 t.artist.toLower().contains(query) ||
                                 t.album.toLower().contains(query);
            if (!matched) continue;
        }

        const int row = m_trackTable->rowCount();
        m_trackTable->insertRow(row);

        const uint32_t sec = t.durationMs / 1000;
        auto *numItem = new QTableWidgetItem();
        numItem->setData(Qt::DisplayRole, t.trackNumber > 0 ? static_cast<int>(t.trackNumber) : row + 1);
        numItem->setData(Qt::UserRole, static_cast<qulonglong>(t.id));
        numItem->setTextAlignment(Qt::AlignRight | Qt::AlignVCenter);

        auto *titleItem = new QTableWidgetItem(t.title);
        auto *artistItem = new QTableWidgetItem(t.artist);
        auto *albumItem = new QTableWidgetItem(t.album);
        auto *genreItem = new QTableWidgetItem(t.genre);
        auto *timeItem = new QTableWidgetItem(
            QString("%1:%2").arg(sec / 60).arg(sec % 60, 2, 10, QChar('0'))
        );
        timeItem->setTextAlignment(Qt::AlignRight | Qt::AlignVCenter);
        auto *bitrateItem = new QTableWidgetItem(QString("%1 kbps").arg(t.bitrateKbps));
        bitrateItem->setTextAlignment(Qt::AlignRight | Qt::AlignVCenter);
        auto *artItem = new QTableWidgetItem(t.hasArtwork ? "Yes" : "-");
        artItem->setTextAlignment(Qt::AlignCenter);

        m_trackTable->setItem(row, 0, numItem);
        m_trackTable->setItem(row, 1, titleItem);
        m_trackTable->setItem(row, 2, artistItem);
        m_trackTable->setItem(row, 3, albumItem);
        m_trackTable->setItem(row, 4, genreItem);
        m_trackTable->setItem(row, 5, timeItem);
        m_trackTable->setItem(row, 6, bitrateItem);
        m_trackTable->setItem(row, 7, artItem);
    }

    m_trackTable->setSortingEnabled(true);
    updateActionStates();
    updateCoverPreview();
}

void MainWindow::updateCoverPreview() {
    if (!m_hasDevice) {
        m_coverPreview->setPixmap(QPixmap());
        m_coverPreview->setText("-");
        m_coverCaption->clear();
        return;
    }

    QPixmap pm;
    QString caption;
    if (const auto trk = selectedTrack(); trk.has_value()) {
        caption = QString("%1\n%2").arg(trk->title, trk->artist);
        if (trk->hasArtwork) {
            pm = loadTrackCoverPixmap(m_mountPath, trk->id);
        }
    } else if (!m_selectedAlbum.isEmpty()) {
        caption = m_selectedAlbum;
        for (const auto &t : m_tracks) {
            const QString alb = t.album.isEmpty() ? QStringLiteral("Unknown Album") : t.album;
            if (alb == m_selectedAlbum && t.hasArtwork) {
                pm = loadTrackCoverPixmap(m_mountPath, t.id);
                break;
            }
        }
    } else if (!m_tracks.isEmpty()) {
        for (const auto &t : m_tracks) {
            if (t.hasArtwork) {
                pm = loadTrackCoverPixmap(m_mountPath, t.id);
                caption = t.album;
                break;
            }
        }
    }

    if (pm.isNull()) {
        m_coverPreview->setPixmap(QPixmap());
        m_coverPreview->setText(trText(m_lang, "lbl.no_art"));
    } else {
        m_coverPreview->setText(QString());
        m_coverPreview->setPixmap(pm.scaled(80, 80, Qt::KeepAspectRatio, Qt::SmoothTransformation));
    }
    m_coverCaption->setText(caption);
}

void MainWindow::updateActionStates() {
    const bool canWrite = m_hasDevice && !m_device.isReadOnly && !m_busy;
    const bool hasSel = selectedTrack().has_value();
    const bool hasAlbumSel = !m_selectedAlbum.isEmpty();

    m_actOpen->setEnabled(!m_busy);
    m_actRefresh->setEnabled(!m_busy);
    m_actEject->setEnabled(m_hasDevice && !m_busy);
    m_formatCombo->setEnabled(!m_busy);
    m_actAddMusic->setEnabled(canWrite);
    m_actCreateAlbum->setEnabled(canWrite);
    m_addToAlbumBtn->setEnabled(canWrite);
    m_actEditAlbum->setEnabled(m_hasDevice && hasAlbumSel && !m_busy);
    m_editAlbumBtn->setEnabled(m_hasDevice && hasAlbumSel && !m_busy);
    m_actEditTrack->setEnabled(m_hasDevice && hasSel && !m_busy);
    m_editBtn->setEnabled(m_hasDevice && hasSel && !m_busy);
    m_actRemoveTrack->setEnabled(canWrite && hasSel);
    m_removeBtn->setEnabled(canWrite && hasSel);
}

std::optional<TrackItem> MainWindow::selectedTrack() const {
    const int row = m_trackTable->currentRow();
    if (row < 0) return std::nullopt;
    const auto *item = m_trackTable->item(row, 0);
    if (!item) return std::nullopt;
    const uint64_t tid = item->data(Qt::UserRole).toULongLong();
    for (const auto &t : m_tracks) {
        if (t.id == tid) return t;
    }
    return std::nullopt;
}

void MainWindow::openMountFolder() {
    const QString dir = QFileDialog::getExistingDirectory(this, trText(m_lang, "act.open"), m_mountPath);
    if (!dir.isEmpty()) {
        m_mountPath = dir;
        m_statusMsg.clear();
        reloadDevice();
    }
}

void MainWindow::ejectDevice() {
    if (!m_hasDevice || m_busy || m_mountPath.isEmpty()) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    std::thread([this, mp]() {
        char *err = nullptr;
        const int32_t rc = poddrop_eject_device(mp.constData(), &err);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    m_mountPath.clear();
                    m_selectedAlbum.clear();
                    m_statusMsg = trText(m_lang, "lbl.ejected");
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::addMusicFiles() {
    const QStringList files = QFileDialog::getOpenFileNames(
        this,
        trText(m_lang, "act.add_music"),
        QString(),
        "Audio (*.mp3 *.m4a *.mp4 *.flac *.ogg *.opus *.wav)"
    );
    if (!files.isEmpty()) {
        startTransferBatch(files, QString(), QString(), QString());
    }
}

void MainWindow::createAlbum(const QString &initialAlbum, const QString &initialArtist) {
    if (auto res = runCreateAlbumDialog(this, m_lang, initialAlbum, initialArtist)) {
        startTransferBatch(res->audioFiles, res->coverPath, res->album, res->artist);
    }
}

void MainWindow::addToSelectedAlbum() {
    QString artist;
    if (!m_selectedAlbum.isEmpty()) {
        for (const auto &t : m_tracks) {
            if (t.album == m_selectedAlbum && !t.artist.isEmpty()) {
                artist = t.artist;
                break;
            }
        }
    }
    createAlbum(m_selectedAlbum, artist);
}

void MainWindow::editSelectedAlbum() {
    if (!m_hasDevice || m_selectedAlbum.isEmpty()) return;

    AlbumInfoSummary summary{};
    summary.album = m_selectedAlbum;
    uint64_t coverTrackId = 0;
    for (const auto &t : m_tracks) {
        const QString alb = t.album.isEmpty() ? QStringLiteral("Unknown Album") : t.album;
        if (alb != m_selectedAlbum) continue;
        summary.trackCount++;
        summary.totalDurationMs += t.durationMs;
        summary.totalSizeBytes += t.sizeBytes;
        if (summary.artist.isEmpty() && !t.artist.isEmpty()) summary.artist = t.artist;
        if (summary.genre.isEmpty() && !t.genre.isEmpty()) summary.genre = t.genre;
        if (summary.year == 0 && t.year > 0) summary.year = t.year;
        if (t.hasArtwork) {
            summary.hasArtwork = true;
            if (coverTrackId == 0) coverTrackId = t.id;
        }
    }
    if (summary.trackCount == 0) return;

    const bool canWrite = !m_device.isReadOnly && !m_busy;
    const QPixmap currentCover = coverTrackId > 0 ? loadTrackCoverPixmap(m_mountPath, coverTrackId) : QPixmap();
    const auto editedOpt = runAlbumEditorDialog(this, m_lang, summary, currentCover, canWrite);
    if (!editedOpt.has_value() || !canWrite) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const QString oldAlbum = m_selectedAlbum;
    const AlbumEditResult edit = *editedOpt;

    std::thread([this, mp, oldAlbum, edit]() {
        const QByteArray oldB = oldAlbum.toUtf8();
        const QByteArray newAlbB = edit.newAlbum.toUtf8();
        const QByteArray newArtB = edit.newArtist.toUtf8();
        const QByteArray newGenB = edit.newGenre.toUtf8();
        const QByteArray coverB = edit.coverPath.toUtf8();

        PodDropUpdateAlbumRequest req{};
        req.old_album = oldB.constData();
        req.new_album = newAlbB.constData();
        req.new_artist = newArtB.constData();
        req.new_genre = newGenB.constData();
        req.year = edit.year;
        req.artwork_path = coverB.isEmpty() ? nullptr : coverB.constData();
        req.clear_artwork = edit.clearArtwork ? 1 : 0;

        uint32_t updated = 0;
        char *err = nullptr;
        const int32_t rc = poddrop_update_album(mp.constData(), &req, &updated, &err);
        const QString errMsg = takeCString(err);
        const QString nextSelected = edit.newAlbum.trimmed().isEmpty()
                                         ? QStringLiteral("Unknown Album")
                                         : edit.newAlbum.trimmed();

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg, updated, nextSelected]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (rc == 0) {
                    m_selectedAlbum = nextSelected;
                    m_statusMsg = QString("%1 (%2)").arg(trText(m_lang, "lbl.album_updated")).arg(updated);
                } else {
                    m_statusMsg = errMsg;
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::editSelectedTrack() {
    const auto trackOpt = selectedTrack();
    if (!trackOpt.has_value()) return;
    const bool canWrite = m_hasDevice && !m_device.isReadOnly && !m_busy;
    const QPixmap currentCover = trackOpt->hasArtwork ? loadTrackCoverPixmap(m_mountPath, trackOpt->id) : QPixmap();
    const auto editedOpt = runTrackEditorDialog(this, m_lang, *trackOpt, currentCover, canWrite);
    if (!editedOpt.has_value() || !canWrite) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const uint64_t tid = trackOpt->id;
    const TrackEditResult edit = *editedOpt;

    std::thread([this, mp, tid, edit]() {
        const QByteArray titleB = edit.title.toUtf8();
        const QByteArray artistB = edit.artist.toUtf8();
        const QByteArray albumB = edit.album.toUtf8();
        const QByteArray genreB = edit.genre.toUtf8();
        const QByteArray artB = edit.coverPath.toUtf8();

        PodDropUpdateRequest req{};
        req.title = titleB.constData();
        req.artist = artistB.constData();
        req.album = albumB.constData();
        req.genre = genreB.constData();
        req.track_number = edit.trackNumber;
        req.disc_number = edit.discNumber;
        req.year = edit.year;
        req.artwork_path = artB.isEmpty() ? nullptr : artB.constData();
        req.clear_artwork = edit.clearArtwork ? 1 : 0;

        char *err = nullptr;
        const int32_t rc = poddrop_update_track(mp.constData(), tid, &req, &err);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                m_statusMsg = (rc == 0) ? trText(m_lang, "lbl.update_done") : errMsg;
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::removeSelectedTrack() {
    const auto trackOpt = selectedTrack();
    if (!trackOpt.has_value() || m_device.isReadOnly) return;

    const auto reply = QMessageBox::question(
        this,
        trText(m_lang, "act.remove_track"),
        QString("%1\n\n%2").arg(trText(m_lang, "dlg.remove_confirm"), trackOpt->title)
    );
    if (reply != QMessageBox::Yes) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const uint64_t tid = trackOpt->id;

    std::thread([this, mp, tid]() {
        char *err = nullptr;
        const int32_t rc = poddrop_remove_track(mp.constData(), tid, &err);
        const QString errMsg = takeCString(err);

        QMetaObject::invokeMethod(
            this,
            [this, rc, errMsg]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                m_statusMsg = (rc == 0) ? trText(m_lang, "lbl.ready") : errMsg;
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::startTransferBatch(
    const QStringList &files,
    const QString &coverPath,
    const QString &albumOverride,
    const QString &artistOverride
) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy || files.isEmpty()) return;

    m_busy = true;
    applyTranslations();
    updateActionStates();
    QApplication::setOverrideCursor(Qt::WaitCursor);

    const QByteArray mp = m_mountPath.toUtf8();
    const int32_t fmt = m_formatCombo->currentIndex();

    std::thread([this, mp, files, coverPath, albumOverride, artistOverride, fmt]() {
        const QByteArray artB = coverPath.toUtf8();
        const QByteArray albB = albumOverride.toUtf8();
        const QByteArray arB = artistOverride.toUtf8();

        int succeeded = 0;
        QString firstError;

        for (int i = 0; i < files.size(); ++i) {
            const QByteArray audioB = files[i].toUtf8();
            PodDropTransferRequest req{};
            req.audio_path = audioB.constData();
            req.artwork_path = artB.isEmpty() ? nullptr : artB.constData();
            req.transcode_format = fmt;
            req.title_override = nullptr;
            req.artist_override = arB.isEmpty() ? nullptr : arB.constData();
            req.album_override = albB.isEmpty() ? nullptr : albB.constData();

            uint64_t outId = 0;
            char *err = nullptr;
            if (poddrop_transfer_track(mp.constData(), &req, i == 0 ? 1 : 0, &outId, &err) == 0) {
                succeeded++;
            } else if (firstError.isEmpty()) {
                firstError = takeCString(err);
            } else {
                poddrop_free_string(err);
            }
        }

        QMetaObject::invokeMethod(
            this,
            [this, succeeded, total = files.size(), firstError]() {
                QApplication::restoreOverrideCursor();
                m_busy = false;
                if (firstError.isEmpty()) {
                    m_statusMsg = QString("%1 (%2/%3)")
                                      .arg(trText(m_lang, "lbl.transfer_done"))
                                      .arg(succeeded)
                                      .arg(total);
                } else {
                    m_statusMsg = QString("%1 (%2/%3) - %4")
                                      .arg(trText(m_lang, "lbl.transfer_done"))
                                      .arg(succeeded)
                                      .arg(total)
                                      .arg(firstError);
                }
                reloadDevice();
            },
            Qt::QueuedConnection
        );
    }).detach();
}

void MainWindow::dragEnterEvent(QDragEnterEvent *event) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy) return;
    if (event->mimeData()->hasUrls()) {
        for (const QUrl &url : event->mimeData()->urls()) {
            if (url.isLocalFile() && isSupportedAudioExt(url.toLocalFile())) {
                event->acceptProposedAction();
                return;
            }
        }
    }
}

void MainWindow::dropEvent(QDropEvent *event) {
    if (!m_hasDevice || m_device.isReadOnly || m_busy) return;
    QStringList files;
    for (const QUrl &url : event->mimeData()->urls()) {
        if (url.isLocalFile()) {
            const QString path = url.toLocalFile();
            if (isSupportedAudioExt(path)) {
                files.push_back(path);
            }
        }
    }
    if (!files.isEmpty()) {
        startTransferBatch(files, QString(), m_selectedAlbum, QString());
        event->acceptProposedAction();
    }
}
