#include <QApplication>
#include "mainwindow.hpp"

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);
    app.setStyle("fusion");
    app.setApplicationName("PodDrop");

    MainWindow w;
    w.show();
    return app.exec();
}
