#include <QApplication>
#include <QIcon>
#include "mainwindow.hpp"

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);
    app.setStyle("fusion");
    app.setApplicationName("PodDrop");
    app.setDesktopFileName("io.github.m4sa_k1.PodDrop");
    app.setWindowIcon(QIcon(":/icons/poddrop.png"));

    MainWindow w;
    w.show();
    return app.exec();
}
