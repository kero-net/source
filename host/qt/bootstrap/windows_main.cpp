#include <windows.h>

#include <filesystem>
#include <fstream>
#include <string>

namespace {
constexpr wchar_t kPayloadName[] = L"KERO_PAYLOAD";

void showError(const std::wstring& message) {
    MessageBoxW(nullptr, message.c_str(), L"KERO Setup", MB_ICONERROR | MB_OK);
}

bool runHidden(const std::wstring& command, DWORD* exitCode) {
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    startup.dwFlags = STARTF_USESHOWWINDOW;
    startup.wShowWindow = SW_HIDE;
    PROCESS_INFORMATION process{};
    auto mutableCommand = command;
    if (!CreateProcessW(nullptr, mutableCommand.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) return false;
    WaitForSingleObject(process.hProcess, INFINITE);
    GetExitCodeProcess(process.hProcess, exitCode);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    return true;
}

bool writePayload(const std::filesystem::path& zipPath) {
    const auto resource = FindResourceW(nullptr, kPayloadName, MAKEINTRESOURCEW(10));
    if (!resource) return false;
    const auto handle = LoadResource(nullptr, resource);
    const auto bytes = LockResource(handle);
    const auto length = SizeofResource(nullptr, resource);
    if (!bytes || length == 0) return false;
    std::ofstream output(zipPath, std::ios::binary);
    output.write(static_cast<const char*>(bytes), length);
    return output.good();
}

std::wstring quoted(const std::filesystem::path& path) {
    return L"\"" + path.wstring() + L"\"";
}
} // namespace

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR commandLine, int) {
    const bool verifyPayload = std::wstring(commandLine).find(L"--verify-payload") != std::wstring::npos;
    const auto root = std::filesystem::temp_directory_path() / L"KERO-setup";
    const auto bundle = root / L"bundle";
    const auto zip = root / L"bundle.zip";
    std::error_code error;
    std::filesystem::remove_all(root, error);
    std::filesystem::create_directories(bundle, error);
    if (error || !writePayload(zip)) {
        showError(L"KERO Setup could not prepare its embedded installation files.");
        return 1;
    }

    DWORD exitCode = 1;
    // tar.exe is included with supported Windows versions. Keeping extraction
    // here avoids making the signed bootstrap depend on a shell runtime.
    const auto extract = L"tar.exe -xf " + quoted(zip) + L" -C " + quoted(bundle);
    if (!runHidden(extract, &exitCode) || exitCode != 0) {
        showError(L"KERO Setup could not extract its installation files.");
        return 1;
    }

    const auto installer = bundle / L"kero-install.exe";
    const auto application = bundle / L"payload" / L"kero.exe";
    const auto host = bundle / L"payload" / L"kero-host.exe";
    const auto runtime = bundle / L"payload" / L"kero.wasm";
    if (!std::filesystem::is_regular_file(installer) || !std::filesystem::is_regular_file(application)
        || !std::filesystem::is_regular_file(host) || !std::filesystem::is_regular_file(runtime)) {
        showError(L"KERO Setup is missing its embedded installer.");
        return 1;
    }
    if (verifyPayload) {
        std::filesystem::remove_all(root, error);
        return 0;
    }
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    auto command = L"\"" + installer.wstring() + L"\"";
    if (!CreateProcessW(nullptr, command.data(), nullptr, nullptr, FALSE, 0, nullptr, bundle.c_str(), &startup, &process)) {
        showError(L"KERO Setup could not start its installer.");
        return 1;
    }
    WaitForSingleObject(process.hProcess, INFINITE);
    GetExitCodeProcess(process.hProcess, &exitCode);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    std::filesystem::remove_all(root, error);
    return static_cast<int>(exitCode);
}
