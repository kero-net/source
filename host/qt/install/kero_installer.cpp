#include "kero_installer.h"
#include "../app/kero_automation_server.h"
#include "../common/kero_home.h"

#include "ui_kero-install.h"

#include <QApplication>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QDesktopServices>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QIcon>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProcess>
#include <QProgressBar>
#include <QPushButton>
#include <QRadioButton>
#include <QSaveFile>
#include <QStandardPaths>
#include <QTimer>
#include <QUrl>
#include <QWizard>

#ifdef Q_OS_WIN
#include <windows.h>
#include <shobjidl.h>
#include <shlobj.h>
#endif

#include <memory>

namespace {
void loadStyle(QApplication& app) {
    QFile style(QCoreApplication::applicationDirPath() + "/kero.qss");
    if (style.open(QIODevice::ReadOnly | QIODevice::Text)) app.setStyleSheet(style.readAll());
}

QString userHome() {
    return defaultKeroHomeLocation();
}

QString systemHome() {
    return QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation) + "/KERO";
}

QString selectedHome(const Ui::KeroInstallWizard& form) {
    return QDir::cleanPath(form.customHomePathEdit->text());
}

QString enrollment(const Ui::KeroInstallWizard& form) {
    if (form.automaticEnrollmentRadio->isChecked()) return "automatic";
    if (form.manualEnrollmentRadio->isChecked()) return "manual";
    return "ask";
}

QString identity(const Ui::KeroInstallWizard& form) {
    return form.configureIdentityLaterRadio->isChecked() ? "configure-later" : "reuse";
}

QString verification(const Ui::KeroInstallWizard& form) {
    if (form.warnVerificationRadio->isChecked()) return "warn";
    if (form.strictVerificationRadio->isChecked()) return "strict";
    if (form.permissiveVerificationRadio->isChecked()) return "permissive";
    return "automatic";
}

QByteArray homeConfig(const Ui::KeroInstallWizard& form) {
    return "# KERO global-home preferences. These defaults never change repository data.\n"
           "# Select one value in each section: uncomment an alternative, then comment\n"
           "# the currently active line. KERO_HOME selects this home; it is not set here.\n"
           "#\n"
           "# Repository enrollment default: ask | automatic | manual\n"
           "enrollment " + enrollment(form).toUtf8() + "\n"
           "# enrollment ask\n# enrollment automatic\n# enrollment manual\n"
           "#\n"
           "# Identity default: reuse | configure-later\n"
           "identity " + identity(form).toUtf8() + "\n"
           "# identity reuse\n# identity configure-later\n"
           "#\n"
           "# Verification default: automatic | warn | strict | permissive\n"
           "verification " + verification(form).toUtf8() + "\n"
           "# verification automatic\n# verification warn\n# verification strict\n# verification permissive\n"
           "#\n"
           "# Mount defaults. Repository mount entries override refresh only.\n"
           "mountDefaults\n"
           "\t# Refresh: manual | event\n\trefresh manual\n"
           "\t# refresh event\n\t# Event debounce is a positive number of seconds.\n\teventDebounceSeconds 1\n"
           "\t# Missed-event audit: disabled (no periodic full-tree scan).\n\tmissedEventAudit disabled\n"
           "\t# Access request: readOnly | readWrite. readWrite still requires a grant.\n\taccess readOnly\n"
           "\t# access readWrite\n\t# Conflict policy: blockAndAsk.\n\tconflictPolicy blockAndAsk\n"
           "#\n"
           "# Service endpoint, bearer token, installation paths, and mount sources are\n"
           "# intentionally not settings: they are generated or supplied explicitly.\n";
}

bool copyFile(const QString& from, const QString& to, QString* error) {
    const auto sourceInfo = QFileInfo(from);
    if (!sourceInfo.exists()) { *error = "The installer bundle is missing " + sourceInfo.fileName() + "."; return false; }
    if (sourceInfo.canonicalFilePath() == QFileInfo(to).canonicalFilePath()) return true;
    if (!QDir().mkpath(QFileInfo(to).dir().absolutePath())) { *error = "Could not create " + QFileInfo(to).dir().absolutePath(); return false; }
    QFile::remove(to);
    if (!QFile::copy(from, to)) { *error = "Could not copy " + sourceInfo.fileName() + "."; return false; }
    return true;
}

bool copyDirectory(const QString& from, const QString& to, QString* error) {
    QDir source(from);
    if (!source.exists()) return true;
    if (!QDir().mkpath(to)) { *error = "Could not create " + to; return false; }
    for (const auto& item : source.entryInfoList(QDir::Files | QDir::Dirs | QDir::NoDotAndDotDot)) {
        const auto target = QDir(to).filePath(item.fileName());
        if (item.isDir()) {
            if (!copyDirectory(item.absoluteFilePath(), target, error)) return false;
        } else if (!copyFile(item.absoluteFilePath(), target, error)) return false;
    }
    return true;
}

bool install(const Ui::KeroInstallWizard& form, QString* error) {
    // The installer executable and its Qt loader dependencies remain in the
    // release bundle. Only this private payload becomes the installed KERO.
    QDir source(QCoreApplication::applicationDirPath() + "/payload");
    QDir destination(form.installPathEdit->text());
    if (destination.path().isEmpty()) { *error = "Choose an installation location first."; return false; }
    if (!QDir().mkpath(destination.path())) { *error = "Could not create " + destination.path(); return false; }

    for (const auto& name : { "kero.exe", "kero-host.exe", "kero-uninstall.exe", "kero.wasm", "kero.qss" }) {
        if (!copyFile(source.filePath(name), destination.filePath(name), error)) return false;
    }
    for (const auto& name : source.entryList({ "*.dll" }, QDir::Files)) {
        if (!copyFile(source.filePath(name), destination.filePath(name), error)) return false;
    }
    for (const auto& name : { "platforms", "iconengines", "imageformats", "networkinformation", "styles", "tls", "translations" }) {
        if (!copyDirectory(source.filePath(name), destination.filePath(name), error)) return false;
    }

    const auto home = selectedHome(form);
    if (!QDir().mkpath(home + "/data") || !QDir().mkpath(home + "/mnt")) {
        *error = "Could not create KERO home at " + home; return false;
    }
    QFile config(home + "/config");
    if (!config.open(QIODevice::WriteOnly | QIODevice::Text | QIODevice::Truncate)) {
        *error = "Could not write KERO home configuration."; return false;
    }
    config.write(homeConfig(form));
    config.close();
    if (!QFileInfo(destination.filePath("kero-host.exe")).isFile()
        || !QFileInfo(destination.filePath("kero.wasm")).isFile()) {
        *error = "The installed terminal host payload is incomplete.";
        return false;
    }
    return true;
}

bool createShellLink(const QString& target, const QString& shortcut, const QString& arguments,
                     const QString& workingDirectory, QString* error) {
#ifdef Q_OS_WIN
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    const bool uninitialize = SUCCEEDED(initialized);
    IShellLinkW* link = nullptr;
    const HRESULT created = CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER,
        IID_IShellLinkW, reinterpret_cast<void**>(&link));
    if (FAILED(created)) { if (uninitialize) CoUninitialize(); *error = "Could not create a Windows shortcut."; return false; }
    const std::wstring nativeTarget = QDir::toNativeSeparators(target).toStdWString();
    link->SetPath(nativeTarget.c_str());
    link->SetIconLocation(nativeTarget.c_str(), 0);
    const std::wstring nativeArguments = arguments.toStdWString();
    link->SetArguments(nativeArguments.c_str());
    const std::wstring nativeWorkingDirectory =
        QDir::toNativeSeparators(workingDirectory).toStdWString();
    link->SetWorkingDirectory(nativeWorkingDirectory.c_str());
    IPersistFile* file = nullptr;
    const HRESULT persisted = link->QueryInterface(IID_IPersistFile, reinterpret_cast<void**>(&file));
    const std::wstring nativeShortcut = QDir::toNativeSeparators(shortcut).toStdWString();
    const HRESULT saved = SUCCEEDED(persisted) ? file->Save(nativeShortcut.c_str(), TRUE) : E_FAIL;
    if (file) file->Release();
    link->Release();
    if (uninitialize) CoUninitialize();
    if (FAILED(saved)) { *error = "Could not save the Windows shortcut."; return false; }
    return true;
#else
    Q_UNUSED(target); Q_UNUSED(shortcut); *error = "Windows shortcuts are unavailable on this platform."; return false;
#endif
}

bool createDesktopShortcut(const QString& installPath, QString* error) {
    const QString desktop = QStandardPaths::writableLocation(QStandardPaths::DesktopLocation);
    if (desktop.isEmpty()) {
        *error = "Windows did not provide a desktop location.";
        return false;
    }

    const QString application = QDir(installPath).filePath("kero.exe");
    if (!QFileInfo(application).isFile()) {
        *error = "KERO was not found in the selected installation location.";
        return false;
    }

    return createShellLink(application, QDir(desktop).filePath("KERO.lnk"), {}, installPath, error);
}

bool createStartMenuShortcut(const QString& installPath, QString* error) {
    const QString applications = QStandardPaths::writableLocation(QStandardPaths::ApplicationsLocation);
    if (applications.isEmpty()) {
        *error = "Windows did not provide a Start menu applications location.";
        return false;
    }
    const QString application = QDir(installPath).filePath("kero.exe");
    if (!QFileInfo(application).isFile()) {
        *error = "KERO was not found in the selected installation location.";
        return false;
    }
    const QString folder = QDir(applications).filePath("KERO");
    if (!QDir().mkpath(folder)) {
        *error = "Could not create the KERO Start menu folder.";
        return false;
    }
    return createShellLink(application, QDir(folder).filePath("KERO.lnk"), {}, installPath, error);
}

bool createStartupHostShortcut(const QString& host, QString* error) {
#ifdef Q_OS_WIN
    PWSTR startupPath = nullptr;
    const HRESULT located = SHGetKnownFolderPath(FOLDERID_Startup, KF_FLAG_DEFAULT, nullptr, &startupPath);
    if (FAILED(located) || !startupPath) {
        *error = "Windows did not provide the current user's Startup folder.";
        return false;
    }
    const QString startup = QString::fromWCharArray(startupPath);
    CoTaskMemFree(startupPath);
    if (!QDir().mkpath(startup)) {
        *error = "Could not create the current user's Startup folder.";
        return false;
    }
    return createShellLink(host, QDir(startup).filePath("KERO Host.lnk"),
        "service start", QFileInfo(host).absolutePath(), error);
#else
    Q_UNUSED(host); Q_UNUSED(error); return true;
#endif
}

bool persistCustomHomeAndPath(const Ui::KeroInstallWizard& form, QString* error) {
#ifdef Q_OS_WIN
    HKEY key = nullptr;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, L"Environment", 0, nullptr, 0, KEY_READ | KEY_WRITE, nullptr, &key, nullptr) != ERROR_SUCCESS) { *error = "Could not open the user environment settings."; return false; }
    const auto home = QDir::toNativeSeparators(selectedHome(form)).toStdWString();
    if (RegSetValueExW(key, L"KERO_HOME", 0, REG_SZ, reinterpret_cast<const BYTE*>(home.c_str()), DWORD((home.size() + 1) * sizeof(wchar_t))) != ERROR_SUCCESS) { RegCloseKey(key); *error = "Could not persist KERO_HOME."; return false; }
    wchar_t existing[32767]{}; DWORD size = sizeof(existing); DWORD type = 0;
    QString path;
    if (RegQueryValueExW(key, L"Path", nullptr, &type, reinterpret_cast<BYTE*>(existing), &size) == ERROR_SUCCESS && (type == REG_SZ || type == REG_EXPAND_SZ)) path = QString::fromWCharArray(existing);
    const QString install = QDir::toNativeSeparators(form.installPathEdit->text());
    if (!path.split(';', Qt::SkipEmptyParts).contains(install, Qt::CaseInsensitive)) path += (path.isEmpty() ? "" : ";") + install;
    const auto nativePath = path.toStdWString();
    if (RegSetValueExW(key, L"Path", 0, REG_EXPAND_SZ, reinterpret_cast<const BYTE*>(nativePath.c_str()), DWORD((nativePath.size() + 1) * sizeof(wchar_t))) != ERROR_SUCCESS) { RegCloseKey(key); *error = "Could not add KERO to the user PATH."; return false; }
    RegCloseKey(key); SetEnvironmentVariableW(L"KERO_HOME", home.c_str());
    SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, reinterpret_cast<LPARAM>(L"Environment"), SMTO_ABORTIFHUNG, 2000, nullptr);
    return true;
#else
    Q_UNUSED(form); Q_UNUSED(error); return true;
#endif
}

bool installPersistentHostService(const QString& installPath, QString* error) {
#ifdef Q_OS_WIN
    const QString host = QDir(installPath).filePath("kero-host.exe");
    if (!QFileInfo(host).isFile()) {
        *error = "KERO Host was not found after installation.";
        return false;
    }
    // The current user's Startup folder is used instead of a scheduled task.
    // It has no cross-user ownership collision and requires no elevation or
    // Task Scheduler policy exception. `start` launches the persistent process
    // from disposable runtime state, so an active host never locks the installed
    // executable during an upgrade.
    if (!createStartupHostShortcut(host, error)) {
        return false;
    }
    QProcess starter;
    starter.start(host, { "service", "start" });
    if (!starter.waitForFinished(5000) || starter.exitCode() != 0) {
        *error = "KERO Host was registered for the current user's Startup folder, but it could not start now: "
            + QString::fromLocal8Bit(starter.readAllStandardError()).trimmed();
        return false;
    }
    return true;
#else
    Q_UNUSED(installPath); Q_UNUSED(error); return true;
#endif
}

bool confirmExistingTargets(QWidget* parent, const Ui::KeroInstallWizard& form) {
    const QFileInfo installInfo(form.installPathEdit->text());
    if (installInfo.exists() && !QDir(form.installPathEdit->text()).entryList(QDir::NoDotAndDotDot | QDir::AllEntries).isEmpty()
        && QMessageBox::question(parent, "KERO Setup", "The installation folder already contains files. Replace KERO's installed files there?") != QMessageBox::Yes) return false;
    const QFileInfo homeInfo(selectedHome(form));
    if (homeInfo.exists() && !QDir(selectedHome(form)).entryList(QDir::NoDotAndDotDot | QDir::AllEntries).isEmpty()
        && QMessageBox::question(parent, "KERO Setup", "The selected KERO home already contains data. Keep that home and update its preferences?") != QMessageBox::Yes) return false;
    return true;
}

bool launchAndVerify(const QString& installPath, QString* error) {
    qint64 processId = 0;
    const QString application = QDir(installPath).filePath("kero.exe");
    if (!QProcess::startDetached(application, {}, installPath, &processId)) {
        *error = "Windows could not start KERO.";
        return false;
    }
#ifdef Q_OS_WIN
    HANDLE process = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE,
        static_cast<DWORD>(processId));
    if (!process) {
        *error = "KERO started, but Windows did not allow startup verification.";
        return false;
    }
    const DWORD state = WaitForSingleObject(process, 1500);
    if (state == WAIT_OBJECT_0) {
        DWORD exitCode = 1;
        GetExitCodeProcess(process, &exitCode);
        CloseHandle(process);
        *error = "KERO exited during startup (exit code " + QString::number(exitCode) + ").";
        return false;
    }
    CloseHandle(process);
    if (state != WAIT_TIMEOUT) {
        *error = "Windows could not verify the KERO startup process.";
        return false;
    }
#endif
    return true;
}
} // namespace

int runKeroInstaller(int argc, char* argv[]) {
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

    QWizard wizard;
    Ui::KeroInstallWizard form;
    form.setupUi(&wizard);
    form.installPathEdit->setText(QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation));
    bool started = false;
    bool confirmed = false;
    std::unique_ptr<KeroAutomationServer> automation;
    if (arguments.isSet(automationPort) || arguments.isSet(automationToken)) {
        bool validPort = false;
        const int requestedPort = arguments.value(automationPort).toInt(&validPort);
        automation = std::make_unique<KeroAutomationServer>(&wizard, &app);
        if (!validPort || requestedPort < 0 || requestedPort > 65535
            || !automation->listen(static_cast<quint16>(requestedPort), arguments.value(automationToken))) {
            QMessageBox::critical(&wizard, "KERO Setup", "Local UI automation could not start.");
            return 1;
        }
    }

    QObject::connect(form.browseInstallPathButton, &QPushButton::clicked, &wizard, [&form, &wizard] {
        const auto path = QFileDialog::getExistingDirectory(&wizard, "Choose KERO installation location", form.installPathEdit->text());
        if (!path.isEmpty()) form.installPathEdit->setText(path);
    });
    const auto setHomePath = [&form](const QString& path) {
        form.customHomePathEdit->setText(QDir::cleanPath(path));
    };
    setHomePath(userHome());
    QObject::connect(form.userLocalHomeRadio, &QRadioButton::toggled, &wizard, [&form, setHomePath](bool checked) {
        if (checked) setHomePath(userHome());
    });
    QObject::connect(form.systemWideHomeRadio, &QRadioButton::toggled, &wizard, [&form, setHomePath](bool checked) {
        if (checked) setHomePath(systemHome());
    });
    QObject::connect(form.customHomeRadio, &QRadioButton::toggled, &wizard, [&form](bool checked) {
        if (checked) form.customHomePathEdit->setFocus();
    });
    QObject::connect(form.browseHomePathButton, &QPushButton::clicked, &wizard, [&form, &wizard] {
        const auto path = QFileDialog::getExistingDirectory(
            &wizard, "Choose KERO home location", form.customHomePathEdit->text());
        if (!path.isEmpty()) {
            form.customHomeRadio->setChecked(true);
            form.customHomePathEdit->setText(QDir::cleanPath(path));
        }
    });
    QObject::connect(&wizard, &QWizard::currentIdChanged, &wizard, [&wizard, &form, &started, &confirmed] {
        wizard.setButtonText(
            QWizard::NextButton,
            wizard.currentPage() == form.identityVerificationPage ? "Install" : "Next >");
        wizard.setButtonText(QWizard::CommitButton, "Install");
        if (wizard.currentPage() != form.installingPage || started) return;
        if (!confirmed) {
            confirmed = true;
            if (!confirmExistingTargets(&wizard, form)) { wizard.reject(); return; }
        }
        started = true;
        // Installation is a one-way transition. Do not leave a dead Back
        // button in the button row while the payload is being committed.
        wizard.button(QWizard::BackButton)->hide();
        wizard.button(QWizard::NextButton)->setEnabled(false);
        wizard.button(QWizard::CancelButton)->setEnabled(false);
        QMetaObject::invokeMethod(&wizard, [&wizard, &form] {
            form.installStatusLabel->setText("Installing KERO...");
            form.installProgressBar->setValue(20);
            form.installDetailsTextEdit->appendPlainText("Copying the KERO runtime bundle...");
            QString error;
            if (!install(form, &error)) {
                form.installStatusLabel->setText("Installation failed.");
                form.installDetailsTextEdit->appendPlainText("Error: " + error);
                wizard.button(QWizard::CancelButton)->setEnabled(true);
                return;
            }
            if (!persistCustomHomeAndPath(form, &error)) {
                form.installStatusLabel->setText("Installation needs attention.");
                form.installDetailsTextEdit->appendPlainText("KERO was copied, but environment setup failed: " + error);
                wizard.button(QWizard::CancelButton)->setEnabled(true);
                return;
            }
            if (!installPersistentHostService(form.installPathEdit->text(), &error)) {
                form.installStatusLabel->setText("Installation needs attention.");
                form.installDetailsTextEdit->appendPlainText("KERO was copied, but its persistent service could not be registered: " + error);
                wizard.button(QWizard::CancelButton)->setEnabled(true);
                return;
            }
            form.installProgressBar->setValue(100);
            form.installStatusLabel->setText("KERO installed successfully.");
            form.installDetailsTextEdit->appendPlainText("KERO home: " + selectedHome(form));
            form.installDetailsTextEdit->appendPlainText("KERO Host is running and will start again when you sign in.");
            form.installDetailsTextEdit->appendPlainText("Installation complete.");
            wizard.button(QWizard::NextButton)->setEnabled(true);
            wizard.button(QWizard::NextButton)->setText("Continue");
            wizard.button(QWizard::CancelButton)->setEnabled(true);
        }, Qt::QueuedConnection);
    });
    QObject::connect(&wizard, &QDialog::finished, &wizard, [&form](int result) {
        if (result != QDialog::Accepted) return;

        if (form.desktopShortcutCheckBox->isChecked()) {
            QString error;
            if (!createDesktopShortcut(form.installPathEdit->text(), &error)) {
                QMessageBox::warning(nullptr, "KERO Setup", error);
            }
        }
        if (form.startMenuShortcutCheckBox->isChecked()) {
            QString error;
            if (!createStartMenuShortcut(form.installPathEdit->text(), &error)) {
                QMessageBox::warning(nullptr, "KERO Setup", error);
            }
        }
        if (form.openHomeCheckBox->isChecked()) {
            const QString home = selectedHome(form);
            QDesktopServices::openUrl(QUrl::fromLocalFile(home));
        }
        if (form.launchKeroCheckBox->isChecked()) {
            QString error;
            if (!launchAndVerify(form.installPathEdit->text(), &error)) {
                QMessageBox::warning(nullptr, "KERO Setup", "KERO was installed, but launch verification failed. " + error);
            }
        }
    });
    wizard.show();
    return app.exec();
}
