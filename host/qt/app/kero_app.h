#pragma once

#include <QMainWindow>
#include <QString>
#include <QJsonObject>

#include <memory>

QT_BEGIN_NAMESPACE
class QTreeWidgetItem;
class QLineEdit;
class QLabel;
class QPushButton;
class QEvent;
QT_END_NAMESPACE

namespace Ui {
class KeroMainWindow;
}

class KeroAppWindow final : public QMainWindow {
    Q_OBJECT

public:
    explicit KeroAppWindow(QWidget* parent = nullptr);
    ~KeroAppWindow() override;

protected:
    void changeEvent(QEvent* event) override;

private:
    void connectUi();
    void refreshEnvironment();
    void populateKnowledge();
    void populateMounts();
    void populatePolicy();
    void updateKnowledgeDetails(QTreeWidgetItem* item);
    void openPath(const QString& path);
    void openMountSyncDialog();

    void selectContext(const QString& path);
    bool isRepositoryContext() const;
    QString activeKeroPath() const;

    std::unique_ptr<Ui::KeroMainWindow> ui_;
    QString homePath_;
    QString projectRoot_;
    QString selectedPath_;
    QJsonObject context_;
    bool homeContext_ = false;
    QLineEdit* contextPathEdit_ = nullptr;
    QLabel* contextStateLabel_ = nullptr;
    QPushButton* enrollContextButton_ = nullptr;
};

/// Starts the normal KERO desktop application.
int runKeroApp(int argc, char* argv[]);
