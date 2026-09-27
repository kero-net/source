#include "kero_window.h"
#include "kero_home.h"

#include <QApplication>
#include <QDir>
#include <QFile>
#include <QIcon>
#include <QLabel>
#include <QMainWindow>
#include <QStandardPaths>
#include <QVBoxLayout>

namespace {
void loadStyle(QApplication& app) {
    QFile style(QCoreApplication::applicationDirPath() + "/kero.qss");
    if (style.open(QIODevice::ReadOnly | QIODevice::Text)) app.setStyleSheet(style.readAll());
}
}

int runKeroWindow(int argc, char* argv[], const QString& role) {
    QApplication app(argc, argv);
    app.setWindowIcon(QIcon(":/images/kero-icon.png"));
    loadStyle(app);
    const auto home = configuredKeroHome();
    if (role != "uninstall") {
        QDir().mkpath(home + "/data");
        QDir().mkpath(home + "/mnt");
    }
    QMainWindow window;
    window.setWindowTitle(role == "app" ? "KERO" : "KERO " + role);
    auto* body = new QWidget(&window);
    auto* layout = new QVBoxLayout(body);
    auto* title = new QLabel(role == "app" ? "KERO" : "KERO " + role, body);
    title->setObjectName("title");
    layout->addWidget(title);
    layout->addWidget(new QLabel("KERO home\n" + home, body));
    layout->addStretch();
    window.setCentralWidget(body);
    window.resize(720, 420);
    window.show();
    return app.exec();
}
