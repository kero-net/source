#include "kero_app.h"
#include "kero_automation_server.h"
#include "kero_service_client.h"

#include "ui_kero-app.h"

#include <QApplication>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QEvent>
#include <QDateTime>
#include <QDesktopServices>
#include <QDialog>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QIcon>
#include <QListWidget>
#include <QLocale>
#include <QJsonArray>
#include <QLabel>
#include <QLineEdit>
#include <QInputDialog>
#include <QMenu>
#include <QMessageBox>
#include <QPushButton>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QPainter>
#include <QPalette>
#include <QSettings>
#include <QVBoxLayout>
#include <QStackedWidget>
#include <QTreeWidget>
#include <QTreeWidgetItem>
#include <QUrl>

namespace {
void loadStyle(QApplication& app) {
    QFile style(QCoreApplication::applicationDirPath() + "/kero.qss");
    if (style.open(QIODevice::ReadOnly | QIODevice::Text)) {
        app.setStyleSheet(style.readAll());
    }
}

QString displayPath(const QString& path) {
    return QDir::toNativeSeparators(QDir::cleanPath(path));
}

class StatusDot final : public QWidget {
public:
    explicit StatusDot(QWidget* parent = nullptr) : QWidget(parent) { setFixedSize(10, 10); }
    void setColor(const QColor& color) { color_ = color; update(); }
protected:
    void paintEvent(QPaintEvent*) override { QPainter p(this); p.setRenderHint(QPainter::Antialiasing); p.setPen(Qt::NoPen); p.setBrush(color_); p.drawEllipse(rect()); }
private: QColor color_;
};

enum class StatusTone { Neutral, Info, Success, Warning, Error };

QColor statusColor(StatusTone tone, const QPalette& palette) {
    switch (tone) {
    case StatusTone::Neutral:
        return palette.color(QPalette::Disabled, QPalette::WindowText);
    case StatusTone::Info:
        return palette.color(QPalette::Active, QPalette::Accent);
    case StatusTone::Success:
    case StatusTone::Warning:
    case StatusTone::Error:
        break;
    }

    // Qt exposes native surface, text, disabled, and accent roles on every
    // supported desktop, but no portable success/warning/error palette roles.
    // Keep only the semantic hue stable and adapt its lightness to the active
    // platform surface so the indicator remains legible in dark and light
    // Windows, macOS, and Linux themes.
    const bool darkSurface = palette.color(QPalette::Window).lightnessF() < 0.5;
    const qreal lightness = darkSurface ? 0.62 : 0.36;
    switch (tone) {
    case StatusTone::Success: return QColor::fromHslF(0.39, 0.48, lightness);
    case StatusTone::Warning: return QColor::fromHslF(0.12, 0.72, lightness);
    case StatusTone::Error: return QColor::fromHslF(0.01, 0.62, lightness);
    default: return palette.color(QPalette::WindowText);
    }
}

void setStatus(QLabel* label, const QString& text, StatusTone tone) {
    label->setText(text); label->setAccessibleName(text); label->setContentsMargins(16, 0, 0, 0);
    StatusDot* dot = nullptr;
    for (QObject* child : label->children()) {
        if (auto* candidate = dynamic_cast<StatusDot*>(child)) { dot = candidate; break; }
    }
    if (!dot) { dot = new StatusDot(label); dot->move(0, 3); }
    dot->setColor(statusColor(tone, label->palette()));
}
} // namespace

KeroAppWindow::KeroAppWindow(QWidget* parent)
    : QMainWindow(parent), ui_(std::make_unique<Ui::KeroMainWindow>()) {
    ui_->setupUi(this);
    setWindowIcon(QIcon(":/images/kero-icon.png"));

    auto* contextPanel = new QWidget(this);
    auto* contextLayout = new QVBoxLayout(contextPanel);
    contextLayout->setContentsMargins(0, 0, 0, 0);
    contextLayout->setSpacing(6);
    auto* selectionRow = new QHBoxLayout;
    contextPathEdit_ = new QLineEdit(contextPanel);
    contextPathEdit_->setReadOnly(true);
    selectionRow->addWidget(new QLabel("Selected folder:", contextPanel));
    selectionRow->addWidget(contextPathEdit_, 1);
    contextStateLabel_ = new QLabel("Selected folder status: checking", contextPanel);
    enrollContextButton_ = new QPushButton("Create KERO environment...", contextPanel);
    enrollContextButton_->hide();
    contextLayout->addLayout(selectionRow);
    contextLayout->addWidget(contextStateLabel_);
    contextLayout->addWidget(enrollContextButton_, 0, Qt::AlignLeft);
    ui_->overviewLayout->insertWidget(2, contextPanel);
    auto* browse = new QPushButton("Browse...", contextPanel);
    auto* recent = new QPushButton("Recent", contextPanel);
    auto* refresh = new QPushButton("Refresh", contextPanel);
    selectionRow->addWidget(browse);
    selectionRow->addWidget(recent);
    selectionRow->addWidget(refresh);
    connect(browse, &QPushButton::clicked, this, [this] {
        const QString path = QFileDialog::getExistingDirectory(this, "Choose KERO context", selectedPath_);
        if (!path.isEmpty()) selectContext(path);
    });
    connect(recent, &QPushButton::clicked, this, [this, recent] {
        QMenu menu(this);
        for (const auto& path : QSettings().value("contexts/recent").toStringList()) {
            auto* action = menu.addAction(displayPath(path));
            connect(action, &QAction::triggered, this, [this, path] { selectContext(path); });
        }
        if (menu.isEmpty()) menu.addAction("No recent contexts")->setEnabled(false);
        menu.exec(recent->mapToGlobal(QPoint(0, recent->height())));
    });
    connect(refresh, &QPushButton::clicked, this, &KeroAppWindow::refreshEnvironment);
    connect(enrollContextButton_, &QPushButton::clicked, this, [this] {
        if (context_.value("repository").toObject().value("state").toString() != "eligible") return;
        if (QMessageBox::question(this, "Create KERO environment",
                "Create a KERO environment in this eligible Git repository?\n\n"
                "This creates .kero/config, .kero/data, and .kero/mnt using the current global defaults. "
                "It does not mount another environment.")
            != QMessageBox::Yes) return;
        const auto enrolled = KeroServiceClient().invoke(
            {"repository", "enroll", "--policy", "automatic", selectedPath_});
        if (!enrolled.ok) QMessageBox::warning(this, "KERO", enrolled.error);
        refreshEnvironment();
    });

    connectUi();
    ui_->navigationList->setCurrentRow(0);
    ui_->pageStack->setCurrentIndex(0);
    const auto recentContexts = QSettings().value("contexts/recent").toStringList();
    if (recentContexts.isEmpty()) {
        // The first-run default is deliberately Home, rather than treating the
        // application's launch directory as an implicit repository context.
        selectedPath_ = QStringLiteral(".");
        refreshEnvironment();
        selectContext(homePath_);
    } else {
        selectContext(recentContexts.front());
    }
}

KeroAppWindow::~KeroAppWindow() = default;

void KeroAppWindow::changeEvent(QEvent* event) {
    QMainWindow::changeEvent(event);
    if (event->type() == QEvent::PaletteChange || event->type() == QEvent::ApplicationPaletteChange) {
        refreshEnvironment();
    }
}

void KeroAppWindow::connectUi() {
    connect(ui_->navigationList, &QListWidget::currentRowChanged,
            ui_->pageStack, &QStackedWidget::setCurrentIndex);

    connect(ui_->refreshOverviewButton, &QPushButton::clicked,
            this, &KeroAppWindow::refreshEnvironment);

    connect(ui_->openEnvironmentButton, &QPushButton::clicked, this, [this] {
        openPath(activeKeroPath());
    });

    connect(ui_->openHomeButton, &QPushButton::clicked, this, [this] {
        openPath(homePath_);
    });

    connect(ui_->localKnowledgeTree, &QTreeWidget::currentItemChanged,
            this, [this](QTreeWidgetItem* current, QTreeWidgetItem*) {
                updateKnowledgeDetails(current);
            });

    connect(ui_->mountsTree, &QTreeWidget::itemSelectionChanged, this, [this] {
        const bool selected = !ui_->mountsTree->selectedItems().isEmpty();
        ui_->editMountButton->setEnabled(selected);
        ui_->removeMountButton->setEnabled(selected);
        ui_->moveMountUpButton->setEnabled(selected);
        ui_->moveMountDownButton->setEnabled(selected);
    });

    connect(ui_->addMountButton, &QPushButton::clicked, this, [this] {
        if (!isRepositoryContext()) return;
        const QString source = QFileDialog::getExistingDirectory(this, "Choose explicit KERO mount source", projectRoot_);
        if (source.isEmpty()) return;
        const auto inspected = KeroServiceClient().invoke({"mount", "inspect", source});
        if (!inspected.ok) { QMessageBox::warning(this, "KERO", inspected.error); return; }
        const auto sourceState = inspected.result.toObject();
        const QString format = sourceState.value("format").toString();
        if (format != "repository" && format != "globalHome" && format != "global-home") {
            QMessageBox::warning(this, "KERO", "The selected folder is not an in-format KERO repository or global home."); return;
        }
        bool accepted = false;
        const QString name = QInputDialog::getText(this, "Name mount", "Mount name", QLineEdit::Normal, {}, &accepted).trimmed();
        if (!accepted || name.isEmpty()) return;
        QStringList request { "mount", format == "globalHome" || format == "global-home" ? "add-home" : "add", name };
        if (format == "globalHome" || format == "global-home") request += { "--home", source }; else request += source;
        request += { "--repository", projectRoot_ };
        const auto added = KeroServiceClient().invoke(request);
        if (!added.ok) QMessageBox::warning(this, "KERO", added.error);
        refreshEnvironment();
    });
    connect(ui_->editMountButton, &QPushButton::clicked, this, &KeroAppWindow::openMountSyncDialog);
    connect(ui_->removeMountButton, &QPushButton::clicked, this, [this] {
        const auto items = ui_->mountsTree->selectedItems(); if (items.isEmpty() || !isRepositoryContext()) return;
        const QString name = items.front()->text(0);
        if (QMessageBox::question(this, "Remove mount", "Remove mount '" + name + "'?") != QMessageBox::Yes) return;
        const auto result = KeroServiceClient().invoke({"mount", "remove", name, "--repository", projectRoot_});
        if (!result.ok) QMessageBox::warning(this, "KERO", result.error);
        refreshEnvironment();
    });
    ui_->moveMountUpButton->hide();
    ui_->moveMountDownButton->hide();
}

void KeroAppWindow::openMountSyncDialog() {
    const auto selected = ui_->mountsTree->selectedItems();
    if (selected.isEmpty() || projectRoot_.isEmpty()) return;
    const QString name = selected.front()->text(0);
    if (name == "Local (repository)") return;
    QDialog dialog(this);
    dialog.setWindowTitle("Mount Sync — " + name);
    auto* layout = new QVBoxLayout(&dialog);
    auto* status = new QLabel("Refreshes publish a validated snapshot. Writable sync blocks on conflicts.", &dialog);
    status->setWordWrap(true);
    auto* refresh = new QPushButton("Refresh now", &dialog);
    auto* sync = new QPushButton("Synchronize", &dialog);
    auto* useSource = new QPushButton("Resolve: use source", &dialog);
    auto* useLocal = new QPushButton("Resolve: use local", &dialog);
    auto* exportBoth = new QPushButton("Resolve: export both…", &dialog);
    auto* close = new QPushButton("Close", &dialog);
    layout->addWidget(status); layout->addWidget(refresh); layout->addWidget(sync); layout->addWidget(useSource); layout->addWidget(useLocal); layout->addWidget(exportBoth); layout->addWidget(close);
    const auto run = [this, name, status](const QStringList& arguments) {
        const auto result = KeroServiceClient().invoke(arguments);
        status->setText(result.ok ? "Completed." : "Needs attention.\n" + result.error);
        refreshEnvironment();
    };
    connect(refresh, &QPushButton::clicked, &dialog, [run, name, this] { run({"mount", "refresh", name, "--repository", projectRoot_}); });
    connect(sync, &QPushButton::clicked, &dialog, [run, name, this] { run({"mount", "sync", name, "--repository", projectRoot_}); });
    connect(useSource, &QPushButton::clicked, &dialog, [run, name, this] { run({"mount", "conflict", "use-source", name, "--repository", projectRoot_}); });
    connect(useLocal, &QPushButton::clicked, &dialog, [run, name, this] { run({"mount", "conflict", "use-local", name, "--repository", projectRoot_}); });
    connect(exportBoth, &QPushButton::clicked, &dialog, [run, name, this, &dialog] {
        const QString output = QFileDialog::getExistingDirectory(&dialog, "Choose an empty parent for the conflict export", projectRoot_);
        if (output.isEmpty()) return;
        run({"mount", "conflict", "export", name, "--output", QDir(output).filePath(name + "-conflict"), "--repository", projectRoot_});
    });
    connect(close, &QPushButton::clicked, &dialog, &QDialog::accept);
    dialog.exec();
}

QString KeroAppWindow::activeKeroPath() const {
    if (isRepositoryContext()) return QDir(projectRoot_).filePath(".kero");
    return homeContext_ ? homePath_ : QString();
}

bool KeroAppWindow::isRepositoryContext() const { return !projectRoot_.isEmpty(); }

void KeroAppWindow::selectContext(const QString& path) {
    selectedPath_ = path;
    refreshEnvironment();
    if (context_.isEmpty()) return;
    QSettings settings;
    QStringList recent = settings.value("contexts/recent").toStringList();
    recent.removeAll(path); recent.prepend(path);
    while (recent.size() > 10) recent.removeLast();
    settings.setValue("contexts/recent", recent);
}

void KeroAppWindow::refreshEnvironment() {
    const auto reply = KeroServiceClient().invoke({"context", "status", selectedPath_.isEmpty() ? "." : selectedPath_});
    if (!reply.ok) {
        context_ = {};
        homeContext_ = false;
        projectRoot_.clear();
        contextStateLabel_->setText("Selected folder status: unavailable");
        enrollContextButton_->hide();
        ui_->repositoryGroup->setTitle("Context needs attention");
        ui_->repositoryNameLabel->setText("KERO could not classify this folder");
        ui_->repositoryPathLabel->setText(reply.error);
        setStatus(ui_->environmentStatusLabel, "Needs attention", StatusTone::Error);
        setStatus(ui_->knowledgeReadyLabel, "Unavailable", StatusTone::Error);
        setStatus(ui_->mountReadyLabel, "Unavailable", StatusTone::Error);
        setStatus(ui_->policyReadyLabel, "Unavailable", StatusTone::Error);
        ui_->openEnvironmentButton->setEnabled(false);
        ui_->mountsTree->setEnabled(false);
        ui_->addMountButton->setEnabled(false);
        if (auto* mountsItem = ui_->navigationList->item(2)) mountsItem->setFlags(mountsItem->flags() & ~Qt::ItemIsEnabled);
        if (auto* policyItem = ui_->navigationList->item(3)) policyItem->setFlags(policyItem->flags() & ~Qt::ItemIsEnabled);
        ui_->knowledgeCountLabel->setText("Unavailable");
        ui_->mountCountLabel->setText("Unavailable");
        return;
    }
    context_ = reply.result.toObject();
    homePath_ = context_.value("home").toString();
    const auto repository = context_.value("repository").toObject();
    const auto boundary = repository.value("boundary").toObject();
    const QString state = repository.value("state").toString();
    // RepositoryDiscovery intentionally preserves its stable Rust field names.
    // The context envelope itself is camelCase, but this nested discovery
    // record uses worktree_root.
    projectRoot_ = state == "enrolled" ? repository.value("worktree_root").toString() : QString();
    homeContext_ = !homePath_.isEmpty()
        && QDir::cleanPath(QFileInfo(selectedPath_).absoluteFilePath())
            == QDir::cleanPath(QFileInfo(homePath_).absoluteFilePath());
    contextPathEdit_->setText(displayPath(homeContext_ ? homePath_ : selectedPath_));
    ui_->homePathEdit->setText(displayPath(homePath_));
    ui_->globalContextLabel->setText("Global: " + displayPath(homePath_));
    const bool eligible = state == "eligible";
    const bool dataContext = homeContext_ || isRepositoryContext();
    const bool validContext = dataContext || eligible;
    const QString contextKind = isRepositoryContext() ? "Enrolled repository"
        : (homeContext_ ? "KERO Home" : (eligible ? "Eligible repository" : "No KERO context"));
    contextStateLabel_->setText("Selected folder status: " + contextKind);
    enrollContextButton_->setVisible(eligible);
    ui_->projectContextLabel->setText(contextKind);
    ui_->repositoryNameLabel->setText(isRepositoryContext() ? QFileInfo(projectRoot_).fileName() : (homeContext_ ? "KERO Home" : (eligible ? "Repository can be enrolled" : "No KERO context")));
    ui_->repositoryPathLabel->setText(isRepositoryContext() ? displayPath(projectRoot_) : (homeContext_ ? displayPath(homePath_) : repository.value("message").toString("This folder is neither KERO Home nor an enrolled repository.")));
    ui_->repositoryGroup->setTitle(isRepositoryContext() ? "Repository context" : (homeContext_ ? "Home context" : (eligible ? "Eligible repository" : "Selected folder")));
    setStatus(ui_->environmentStatusLabel, validContext ? "Ready" : "Choose KERO Home or a repository", validContext ? StatusTone::Success : StatusTone::Neutral);
    setStatus(ui_->knowledgeReadyLabel, dataContext ? "Service-backed" : "A KERO data context is required", dataContext ? StatusTone::Success : StatusTone::Neutral);
    setStatus(ui_->mountReadyLabel, isRepositoryContext() ? "Ready" : "Repository context required", isRepositoryContext() ? StatusTone::Success : StatusTone::Neutral);
    setStatus(ui_->policyReadyLabel, isRepositoryContext() ? "Available" : "Repository context required", isRepositoryContext() ? StatusTone::Success : StatusTone::Neutral);
    ui_->openEnvironmentButton->setEnabled(dataContext);
    ui_->mountsTree->setEnabled(isRepositoryContext());
    ui_->addMountButton->setEnabled(isRepositoryContext());
    ui_->editMountButton->setEnabled(false);
    ui_->removeMountButton->setEnabled(false);
    ui_->environmentHomeLabel->setText("KERO Home\n" + displayPath(homePath_));
    const QString dataPath = isRepositoryContext() ? boundary.value("data").toString()
        : (homeContext_ ? context_.value("homeData").toString() : QString());
    const QString mountsPath = isRepositoryContext() ? boundary.value("mounts").toString() : QString();
    ui_->localDataPathLabel->setText(dataPath.isEmpty() ? "Local data\nNot available in this context"
                                                       : "Local data\n" + displayPath(dataPath));
    ui_->mountPathLabel->setText(mountsPath.isEmpty() ? "Mounts\nRepository context required"
                                                       : "Mounts\n" + displayPath(mountsPath));
    if (auto* mountsItem = ui_->navigationList->item(2)) {
        mountsItem->setFlags(isRepositoryContext() ? mountsItem->flags() | Qt::ItemIsEnabled
                                                   : mountsItem->flags() & ~Qt::ItemIsEnabled);
    }
    if (auto* policyItem = ui_->navigationList->item(3)) {
        policyItem->setFlags(isRepositoryContext() ? policyItem->flags() | Qt::ItemIsEnabled
                                                   : policyItem->flags() & ~Qt::ItemIsEnabled);
    }
    populateKnowledge(); populateMounts(); populatePolicy();
    return;
}
#if 0
    const KeroHomeResult home = ensureKeroHome();
    homePath_ = home.path;
    projectRoot_ = findProjectRoot();

    const QString environmentPath = activeKeroPath();
    const QString dataPath = QDir(environmentPath).filePath("data");
    const QString mountPath = QDir(environmentPath).filePath("mnt");

    ui_->homePathEdit->setText(displayPath(homePath_));
    ui_->globalContextLabel->setText("Global: " + displayPath(homePath_));
    ui_->environmentHomeLabel->setText("KERO home\n" + displayPath(environmentPath));
    ui_->localDataPathLabel->setText("Local data\n" + displayPath(dataPath));
    ui_->mountPathLabel->setText("Mounts\n" + displayPath(mountPath));

    if (projectRoot_.isEmpty()) {
        ui_->projectContextLabel->setText("Project: none");
        ui_->repositoryNameLabel->setText("No repository detected");
        ui_->repositoryPathLabel->setText(
            "Open KERO from a repository containing .kero/ to use project-local knowledge.");
        ui_->repositoryGroup->setTitle("Global environment");
    } else {
        const QFileInfo rootInfo(projectRoot_);
        ui_->projectContextLabel->setText("Project: " + rootInfo.fileName());
        ui_->repositoryNameLabel->setText(rootInfo.fileName());
        ui_->repositoryPathLabel->setText(displayPath(projectRoot_));
        ui_->repositoryGroup->setTitle("This repository");
    }

    const qsizetype knowledgeCount = countEntries(dataPath);
    const qsizetype mountCount = countDirectories(mountPath);
    ui_->knowledgeCountLabel->setText(QString::number(knowledgeCount) + " items");
    ui_->mountCountLabel->setText(QString::number(mountCount) + " mounted");

    const bool ready = home.ready && QDir(environmentPath).exists();
    ui_->environmentStatusLabel->setText(ready ? "●  Ready" : "○  Needs attention");
    ui_->knowledgeReadyLabel->setText(QDir(dataPath).exists() ? "●  Ready" : "○  Missing");
    ui_->mountReadyLabel->setText(QDir(mountPath).exists() ? "●  Ready" : "○  Missing");
    ui_->openEnvironmentButton->setEnabled(ready);

    if (!home.ready) {
        ui_->repositoryPathLabel->setText(home.error);
        ui_->repositoryGroup->setTitle("KERO home needs attention");
    }

    populateKnowledge(dataPath);
    populateMounts(mountPath);
    populatePolicy();
}

void KeroAppWindow::populateKnowledge(const QString& dataPath) {
    ui_->localKnowledgeTree->clear();

    const QDir dir(dataPath);
    const QFileInfoList entries = dir.entryInfoList(
        QDir::Files | QDir::Dirs | QDir::NoDotAndDotDot,
        QDir::DirsFirst | QDir::Name | QDir::IgnoreCase);

    for (const QFileInfo& info : entries) {
        auto* item = new QTreeWidgetItem(ui_->localKnowledgeTree);
        item->setText(0, info.fileName());
        item->setText(1, itemType(info));
        item->setText(2, info.lastModified().toString("yyyy-MM-dd HH:mm"));
        item->setData(0, Qt::UserRole, info.absoluteFilePath());
    }

    for (int column = 0; column < ui_->localKnowledgeTree->columnCount(); ++column) {
        ui_->localKnowledgeTree->resizeColumnToContents(column);
    }

    ui_->knowledgeDetailsLabel->setText(
        entries.isEmpty()
            ? "No local knowledge is stored in this environment yet."
            : "Select an item to view details.");
}

void KeroAppWindow::populateMounts(const QString& mountPath) {
    ui_->mountsTree->clear();
    ui_->mountedKnowledgeTree->clear();

    const QString dataPath = QDir(activeKeroPath()).filePath("data");
    if (QDir(dataPath).exists()) {
        auto* local = new QTreeWidgetItem(ui_->mountsTree);
        local->setText(0, "Local (repository)");
        local->setText(1, displayPath(dataPath));
        local->setText(2, "data/");
        local->setText(3, "● Ready");
    }

    const QDir dir(mountPath);
    const QFileInfoList entries = dir.entryInfoList(
        QDir::Dirs | QDir::NoDotAndDotDot,
        QDir::Name | QDir::IgnoreCase);

    for (const QFileInfo& info : entries) {
        const QString source = info.isSymLink() && !info.symLinkTarget().isEmpty()
            ? info.symLinkTarget()
            : info.absoluteFilePath();

        auto* item = new QTreeWidgetItem(ui_->mountsTree);
        item->setText(0, info.fileName());
        item->setText(1, displayPath(source));
        item->setText(2, "mnt/" + info.fileName() + "/");
        item->setText(3, info.exists() ? "● Ready" : "○ Missing");

        auto* mounted = new QTreeWidgetItem(ui_->mountedKnowledgeTree);
        mounted->setText(0, info.fileName());
        mounted->setText(1, displayPath(source));
        mounted->setText(2, info.exists() ? "Ready" : "Missing");
    }

    for (int column = 0; column < ui_->mountsTree->columnCount(); ++column) {
        ui_->mountsTree->resizeColumnToContents(column);
    }
    for (int column = 0; column < ui_->mountedKnowledgeTree->columnCount(); ++column) {
        ui_->mountedKnowledgeTree->resizeColumnToContents(column);
    }
}

void KeroAppWindow::populatePolicy() {
    const QMap<QString, QString> config = readSimpleConfig(QDir(homePath_).filePath("config"));

    ui_->enrollmentValueLabel->setText(config.value("enrollment", "ask"));
    ui_->identityValueLabel->setText(config.value("identity", "reuse"));
    ui_->verificationValueLabel->setText(config.value("verification", "automatic"));
    ui_->policyModeLabel->setText(config.value("verification", "automatic"));
}
#endif

void KeroAppWindow::populateKnowledge() {
    ui_->localKnowledgeTree->clear();
    if (!homeContext_ && !isRepositoryContext()) {
        ui_->knowledgeCountLabel->setText("Unavailable");
        ui_->knowledgeDetailsLabel->setText(
            "Choose KERO Home to browse global knowledge, or an enrolled repository to browse its local knowledge.");
        return;
    }
    QStringList request { "knowledge", "browse", "--scope", "local" };
    if (isRepositoryContext()) request += { "--repository", projectRoot_ }; else request += "--home";
    const auto reply = KeroServiceClient().invoke(request);
    if (!reply.ok) {
        ui_->knowledgeCountLabel->setText("Needs attention");
        ui_->knowledgeDetailsLabel->setText(reply.error);
        return;
    }
    const auto entries = reply.result.toObject().value("entries").toArray();
    for (const auto& value : entries) {
        const auto entry = value.toObject();
        auto* item = new QTreeWidgetItem(ui_->localKnowledgeTree);
        item->setText(0, entry.value("path").toString());
        item->setText(1, entry.value("kind").toString());
        item->setText(2, QDateTime::fromSecsSinceEpoch(entry.value("modifiedUnixSeconds").toVariant().toLongLong()).toString("yyyy-MM-dd HH:mm"));
        item->setData(0, Qt::UserRole, entry);
    }
    ui_->knowledgeCountLabel->setText(QString::number(entries.size()) + " items");
    ui_->knowledgeDetailsLabel->setText(entries.isEmpty() ? "No knowledge exists in this context." : "Select an item to view details.");
}

void KeroAppWindow::populateMounts() {
    ui_->mountsTree->clear(); ui_->mountedKnowledgeTree->clear();
    if (!isRepositoryContext()) { ui_->mountCountLabel->setText("Repository context required"); return; }
    const auto reply = KeroServiceClient().invoke({"mount", "list", "--repository", projectRoot_});
    if (!reply.ok) { ui_->mountCountLabel->setText("Needs attention"); return; }
    const auto entries = reply.result.toArray();
    for (const auto& value : entries) {
        const auto entry = value.toObject();
        auto* item = new QTreeWidgetItem(ui_->mountsTree);
        item->setText(0, entry.value("name").toString());
        item->setText(1, entry.value("sourceFormat").toString());
        item->setText(2, entry.value("access").toString() + ", " + entry.value("refreshMode").toString());
        item->setText(3, entry.value("status").toString());
        item->setData(0, Qt::UserRole, entry);
        auto* mounted = new QTreeWidgetItem(ui_->mountedKnowledgeTree);
        mounted->setText(0, entry.value("name").toString());
        mounted->setText(1, entry.value("sourceFormat").toString());
        mounted->setText(2, entry.value("status").toString());
    }
    ui_->mountCountLabel->setText(QString::number(entries.size()) + " mounted");
}

void KeroAppWindow::populatePolicy() {
    ui_->enrollmentValueLabel->setText(isRepositoryContext() ? "enrolled" : "select a repository");
    ui_->identityValueLabel->setText("managed by KERO Home");
    ui_->verificationValueLabel->setText("managed by KERO Home");
    ui_->policyModeLabel->setText(isRepositoryContext() ? "available" : "Home context");
}

void KeroAppWindow::updateKnowledgeDetails(QTreeWidgetItem* item) {
    if (!item) {
        ui_->knowledgeDetailsLabel->setText("Select an item to view details.");
        return;
    }

    const auto entry = item->data(0, Qt::UserRole).toJsonObject();
    QString details = entry.value("name").toString() + "\n\n";
    details += "Type: " + entry.value("kind").toString() + "\n";
    details += "Scope path: " + entry.value("path").toString() + "\n";
    details += "Modified: " + QLocale().toString(QDateTime::fromSecsSinceEpoch(entry.value("modifiedUnixSeconds").toVariant().toLongLong()), QLocale::ShortFormat);
    if (entry.value("kind").toString() == "file") details += "\nSize: " + QString::number(entry.value("size").toVariant().toLongLong()) + " bytes";
    ui_->knowledgeDetailsLabel->setText(details);
}

void KeroAppWindow::openPath(const QString& path) {
    if (path.isEmpty() || !QFileInfo::exists(path)) return;
    QDesktopServices::openUrl(QUrl::fromLocalFile(path));
}

int runKeroApp(int argc, char* argv[]) {
    QApplication app(argc, argv);
    QCoreApplication::setOrganizationName("Kooraseru");
    QCoreApplication::setApplicationName("KERO");
    app.setWindowIcon(QIcon(":/images/kero-icon.png"));
    loadStyle(app);

    QCommandLineParser arguments;
    arguments.addHelpOption();
    const QCommandLineOption automationPort(
        "automation-port", "Enable local UI automation on this loopback port.", "port");
    const QCommandLineOption automationToken(
        "automation-token", "Required token for local UI automation.", "token");
    arguments.addOption(automationPort);
    arguments.addOption(automationToken);
    arguments.process(app);

    KeroAppWindow window;
    std::unique_ptr<KeroAutomationServer> automation;
    if (arguments.isSet(automationPort) || arguments.isSet(automationToken)) {
        bool validPort = false;
        const int requestedPort = arguments.value(automationPort).toInt(&validPort);
        automation = std::make_unique<KeroAutomationServer>(&window, &app);
        if (!validPort || requestedPort < 0 || requestedPort > 65535
            || !automation->listen(static_cast<quint16>(requestedPort), arguments.value(automationToken))) {
            QMessageBox::critical(&window, "KERO", "Local UI automation could not start.");
            return 1;
        }
    }
    window.show();
    return app.exec();
}
