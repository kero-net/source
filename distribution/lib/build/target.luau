-- distribution/lib/build/target.luau
--!strict

local Runtime = require("../Runtime")
local Settings = require("../configuration/settings")
local Package = require("./package")
local Task = require("@lune/task")

export type Prepared = {
	name: string,
	build: () -> (),
}

local Target = {}

type DownloadState = "active" | "complete" | "failed"
local downloadStates: { [string]: DownloadState } = {}

local function wslEnvironment(names: { string }): string
	local inherited = Runtime.Environment("WSLENV")
	if inherited and inherited ~= "" then
		table.insert(names, 1, inherited)
	end
	return table.concat(names, ":")
end

local function windowsArmCompiler(root: string): string
	local output, message = Runtime.Capture(root, [[find '/mnt/c/Program Files/Microsoft Visual Studio/2022' '/mnt/c/Program Files (x86)/Microsoft Visual Studio/2022' -type f -path '*/bin/Hostarm64/arm64/cl.exe' -print 2>/dev/null | sort | tail -n 1]])
	assert(output, "cannot inspect the Windows ARM64 compiler: " .. tostring(message))
	return output
end

function Target.Preflight(root: string, configuration: Settings.Configuration)
	local windows = configuration.targets["windows-arm64"]
	if not windows or not windows.enabled or windowsArmCompiler(root) ~= "" then
		return
	end

	local package = assert(windows.builder.installerPackage, "windows-arm64 requires builder.installer-package")
	assert(#windows.builder.installComponents > 0, "windows-arm64 requires builder.install-components")
	if not Runtime.Confirm("Windows ARM64 requires Visual Studio Build Tools and the Windows SDK. Install them now?") then
		error("Windows ARM64 toolchain installation was declined")
	end

	local override = { "--wait", "--passive", "--norestart" }
	for _, component in windows.builder.installComponents do
		table.insert(override, "--add")
		table.insert(override, component)
	end
	Runtime.Job("toolchain preflight", "run", "installing Windows ARM64 build tools")
	local line = "winget.exe install --exact --source winget --accept-package-agreements --accept-source-agreements --id "
		.. Runtime.Quote(package) .. " --override " .. Runtime.Quote(table.concat(override, " "))
	local passed, message = Runtime.Run(root, line, true, "toolchain-preflight")
	assert(passed, "Visual Studio Build Tools installation failed: " .. message)
	assert(windowsArmCompiler(root) ~= "", "Visual Studio installation completed without the required ARM64 compiler")
end

local function environmentScope(values: { [string]: string }, callback: () -> ())
	local previous: { [string]: string? } = {}
	for name, value in pairs(values) do
		previous[name] = Runtime.Environment(name)
		Runtime.SetEnvironment(name, value)
	end
	local succeeded, failure = pcall(callback)
	for name, value in pairs(previous) do
		Runtime.SetEnvironment(name, value or "")
	end
	if not succeeded then
		error(tostring(failure), 0)
	end
end

function Target.Prepare(root: string, name: string, configuration: Settings.Configuration?): Prepared
	local activeConfiguration = configuration or Settings.Read(root)
	local target = assert(activeConfiguration.targets[name], "unknown distribution target: " .. name)
	assert(target.enabled, name .. " is " .. target.lifecycle)
	local builder = target.builder
	local dependencies = root .. "/.heap/artifacts/dependencies"
	local tools = root .. "/.heap/build/toolchains"

	local function run(line: string)
		local passed, message = Runtime.Run(root, line, true, name .. "-prepare")
		assert(passed, message)
	end

	local function capture(line: string): string
		local output, message = Runtime.Capture(root, line)
		assert(output, "probe failed: " .. line .. ": " .. tostring(message))
		return output
	end

	local function download(url: string, path: string, checksum: string)
		while downloadStates[path] == "active" do
			Task.wait(0.05)
		end
		assert(downloadStates[path] ~= "failed", "shared dependency acquisition failed: " .. path)

		local directory = assert(path:match("^(.+)/[^/]+$"), "dependency path has no parent: " .. path)
		local packageName = assert(path:match("([^/]+)$"), "dependency path has no file name: " .. path)
		run("mkdir -p " .. Runtime.Quote(directory))
		downloadStates[path] = "active"
		local succeeded, failure = pcall(function()
			if not Runtime.IsFile(path) then
				local temporary = path .. ".partial"
				Runtime.Log(":: Retrieving package: " .. packageName)
				run("rm -f " .. Runtime.Quote(temporary)
					.. " && curl --fail --location --retry 3 " .. Runtime.Quote(url)
					.. " --output " .. Runtime.Quote(temporary))
				local temporaryOutput = capture("sha256sum " .. Runtime.Quote(temporary))
				local temporaryDigest = assert(temporaryOutput:match("^([0-9a-fA-F]+)"), "checksum output is invalid: " .. temporary)
				assert(temporaryDigest:lower() == checksum, "download checksum mismatch: " .. path)
				run("mv " .. Runtime.Quote(temporary) .. " " .. Runtime.Quote(path))
				Runtime.Log(":: Retrieved package: " .. packageName)
			end
			local output = capture("sha256sum " .. Runtime.Quote(path))
			local digest = assert(output:match("^([0-9a-fA-F]+)"), "checksum output is invalid: " .. path)
			assert(digest:lower() == checksum, "download checksum mismatch: " .. path)
		end)
		if not succeeded then
			downloadStates[path] = "failed"
			error(tostring(failure), 0)
		end
		downloadStates[path] = "complete"
	end

	local function ensureQt(item: Settings.Builder, targetName: string): string
		local prefix = tools .. "/qt/" .. targetName
		local kit = prefix .. "/" .. item.qtVersion .. "/" .. item.qtDirectory
		if Runtime.IsFile(kit .. "/lib/cmake/Qt6/Qt6Config.cmake") then
			return kit
		end

		local packages = tools .. "/python/" .. targetName .. "/site-packages"
		local cache = dependencies .. "/pip/" .. targetName
		local archives = dependencies .. "/qt/" .. targetName
		local logs = root .. "/.heap/logs/" .. targetName
		run("mkdir -p " .. Runtime.Quote(packages) .. " " .. Runtime.Quote(cache) .. " " .. Runtime.Quote(archives) .. " " .. Runtime.Quote(logs))
		if not Runtime.IsDirectory(packages .. "/aqt") then
			run("python3 -m pip install --disable-pip-version-check --cache-dir " .. Runtime.Quote(cache)
				.. " --target " .. Runtime.Quote(packages) .. " aqtinstall==" .. activeConfiguration.tools.aqtinstall)
		end
		Runtime.Log(":: Retrieving package: Qt " .. item.qtVersion .. " for " .. targetName)
		local invocation = "cd " .. Runtime.Quote(logs)
			.. " && PYTHONPATH=" .. Runtime.Quote(packages)
			.. " python3 -m aqt install-qt " .. item.qtHost .. " desktop " .. item.qtVersion .. " " .. item.qtArchitecture
			.. " -k -d " .. Runtime.Quote(archives) .. " -O " .. Runtime.Quote(prefix)
		run(invocation)
		assert(Runtime.IsFile(kit .. "/lib/cmake/Qt6/Qt6Config.cmake"), "Qt installation is incomplete for " .. targetName)
		Runtime.Log(":: Retrieved package: Qt " .. item.qtVersion .. " for " .. targetName)
		return kit
	end

	local drivers: { [string]: () -> (() -> ()) } = {}

	drivers["windows-msvc"] = function()
		assert(Runtime.IsWsl(), "windows-msvc requires the WSL controller")
		local qt = ensureQt(builder, name)
		local compiler = windowsArmCompiler(root)
		local sdkInclude = capture([[find '/mnt/c/Program Files (x86)/Windows Kits/10/Include' -type f -path '*/um/windows.h' -print 2>/dev/null | sort | tail -n 1]])
		assert(compiler ~= "" and sdkInclude ~= "", "Windows ARM64 requires Visual Studio Build Tools with ARM64 C++ tools and the Windows SDK")
		local msvc = assert(compiler:match("^(.+)/bin/Hostarm64/arm64/cl%.exe$"))
		local sdk = assert(sdkInclude:match("/Include/([^/]+)/um/windows%.h$"))
		local kits = "/mnt/c/Program Files (x86)/Windows Kits/10"
		local bin = msvc .. "/bin/Hostarm64/arm64"
		local environment = {
			KERO_QT_PREFIX = qt,
			KERO_MSVC_ROOT = msvc,
			KERO_WINDOWS_SDK = kits,
			KERO_WINDOWS_SDK_VERSION = sdk,
			KERO_MSVC_LINKER = bin .. "/link.exe",
			KERO_MSVC_LINKER_POSIX = bin .. "/link.exe",
			KERO_MSVC_ARCHIVER = bin .. "/lib.exe",
			KERO_CXX_COMPILER = bin .. "/cl.exe",
			CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER = bin .. "/link.exe",
			VCToolsInstallDir = msvc .. "/",
			VCINSTALLDIR = msvc .. "/../../",
			WindowsSdkDir = kits .. "/",
			WindowsSDKVersion = sdk .. "/",
			UniversalCRTSdkDir = kits .. "/",
			UCRTVersion = sdk .. "/",
			INCLUDE = table.concat({ msvc .. "/include", kits .. "/Include/" .. sdk .. "/ucrt", kits .. "/Include/" .. sdk .. "/shared", kits .. "/Include/" .. sdk .. "/um", kits .. "/Include/" .. sdk .. "/winrt", kits .. "/Include/" .. sdk .. "/cppwinrt" }, ":"),
			LIB = table.concat({ msvc .. "/lib/arm64", kits .. "/Lib/" .. sdk .. "/ucrt/arm64", kits .. "/Lib/" .. sdk .. "/um/arm64" }, ":"),
			PATH = bin .. ":" .. kits .. "/bin/" .. sdk .. "/arm64:" .. (Runtime.Environment("PATH") or ""),
			KERO_CARGO = "cargo.exe",
			KERO_RUSTUP = "rustup.exe",
			KERO_CMAKE = "cmake.exe",
			KERO_DEPLOY_TOOL = qt .. "/bin/windeployqt.exe",
			WSLENV = wslEnvironment({ "KERO_QT_PREFIX/p", "KERO_MSVC_ROOT/p", "KERO_WINDOWS_SDK/p", "KERO_MSVC_LINKER/p", "KERO_MSVC_ARCHIVER/p", "KERO_CXX_COMPILER/p", "CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER/p", "VCToolsInstallDir/p", "VCINSTALLDIR/p", "WindowsSdkDir/p", "UniversalCRTSdkDir/p", "KERO_WINDOWS_SDK_VERSION", "WindowsSDKVersion", "UCRTVersion", "INCLUDE/lp", "LIB/lp", "CARGO_TARGET_DIR/p", "KERO_ICON/p", "KERO_WASM/p", "KERO_HOST/p", "KERO_DEPLOY_TOOL/p" }),
		}
		return function()
			environmentScope(environment, function()
				Package.Run(root, name)
			end)
		end
	end

	drivers["windows-llvm"] = function()
		assert(Runtime.IsWsl(), "windows-llvm requires the WSL controller")
		local qt = ensureQt(builder, name)
		local definition = activeConfiguration.tools.llvmMingw
		local archive = dependencies .. "/llvm-mingw/" .. definition.archive
		download("https://github.com/mstorsjo/llvm-mingw/releases/download/" .. definition.version .. "/" .. definition.archive, archive, definition.sha256)
		local extracted = tools .. "/llvm-mingw/llvm-mingw-" .. definition.version .. "-ucrt-aarch64"
		if not Runtime.IsFile(extracted .. "/bin/x86_64-w64-mingw32-clang++.exe") then
			run("mkdir -p " .. Runtime.Quote(tools .. "/llvm-mingw") .. " && unzip -q " .. Runtime.Quote(archive) .. " -d " .. Runtime.Quote(tools .. "/llvm-mingw"))
		end
		local bin = extracted .. "/bin"
		local environment = {
			KERO_QT_PREFIX = qt,
			KERO_CXX_COMPILER = bin .. "/x86_64-w64-mingw32-clang++.exe",
			KERO_WINDRES = bin .. "/x86_64-w64-mingw32-windres.exe",
			KERO_RUNTIME_DIR = extracted .. "/x86_64-w64-mingw32/bin",
			KERO_CARGO = "cargo.exe +stable-aarch64-pc-windows-gnullvm",
			KERO_RUSTUP = "rustup.exe",
			KERO_RUST_TOOLCHAIN = "stable-aarch64-pc-windows-gnullvm",
			KERO_CMAKE = "cmake.exe",
			KERO_DEPLOY_TOOL = qt .. "/bin/windeployqt.exe",
			PATH = bin .. ":" .. (Runtime.Environment("PATH") or ""),
			WSLENV = wslEnvironment({ "KERO_QT_PREFIX/p", "KERO_CXX_COMPILER/p", "KERO_WINDRES/p", "KERO_RUNTIME_DIR/p", "CARGO_TARGET_DIR/p", "KERO_ICON/p", "KERO_WASM/p", "KERO_HOST/p", "KERO_DEPLOY_TOOL/p" }),
		}
		return function()
			environmentScope(environment, function()
				Package.Run(root, name)
			end)
		end
	end

	drivers["wsl-linux"] = function()
		assert(Runtime.OperatingSystem() == "linux", "wsl-linux must run inside the WSL controller")
		ensureQt(builder, name)
		local deploy = activeConfiguration.tools.linuxdeploy
		local architecture = assert(builder.linuxdeployArchitecture, name .. " requires builder.linuxdeploy-architecture")
		local deployName = "linuxdeploy-" .. architecture .. ".AppImage"
		local deployChecksum = if architecture == "aarch64" then deploy.aarch64Sha256 else deploy.x86_64Sha256
		download(deploy.baseUrl .. "/" .. deployName, dependencies .. "/linuxdeploy/" .. deployName, deployChecksum)
		if name == "linux-x64" then
			download(deploy.baseUrl .. "/linuxdeploy-aarch64.AppImage", dependencies .. "/linuxdeploy/linuxdeploy-aarch64.AppImage", deploy.aarch64Sha256)
		end
		local runtime = activeConfiguration.tools.appImageRuntime
		local runtimeName = "runtime-" .. architecture
		local runtimeChecksum = if architecture == "aarch64" then runtime.aarch64Sha256 else runtime.x86_64Sha256
		download(runtime.baseUrl .. "/" .. runtimeName, dependencies .. "/appimage/" .. runtimeName, runtimeChecksum)

		if name == "linux-x64" then
			local zig = activeConfiguration.tools.zig
			local zigArchive = dependencies .. "/zig/" .. zig.archive
			download(zig.url, zigArchive, zig.sha256)
			local zigRoot = tools .. "/zig/zig-aarch64-linux-" .. zig.version
			if not Runtime.IsFile(zigRoot .. "/zig") then
				run("mkdir -p " .. Runtime.Quote(tools .. "/zig") .. " && tar -xf " .. Runtime.Quote(zigArchive) .. " -C " .. Runtime.Quote(tools .. "/zig"))
			end
			local qemu = activeConfiguration.tools.qemuX64
			local qemuArchive = dependencies .. "/qemu/" .. qemu.archive
			download(qemu.url, qemuArchive, qemu.sha256)
			if not Runtime.IsFile(tools .. "/qemu/usr/bin/qemu-x86_64-static") then
				run("dpkg-deb -x " .. Runtime.Quote(qemuArchive) .. " " .. Runtime.Quote(tools .. "/qemu"))
			end
			local image = activeConfiguration.tools.ubuntuX64Rootfs
			local imageArchive = dependencies .. "/ubuntu-rootfs/" .. image.archive
			download(image.url, imageArchive, image.sha256)
			local rootfs = tools .. "/linux-x64/rootfs"
			if not Runtime.IsFile(rootfs .. "/bin/bash") then
				run("mkdir -p " .. Runtime.Quote(rootfs) .. " && tar -xzf " .. Runtime.Quote(imageArchive) .. " -C " .. Runtime.Quote(rootfs))
			end
			for _, library in activeConfiguration.tools.ubuntuX64Libraries do
				local libraryArchive = dependencies .. "/ubuntu-libraries/" .. library.archive
				download(library.url, libraryArchive, library.sha256)
				run("dpkg-deb -x " .. Runtime.Quote(libraryArchive) .. " " .. Runtime.Quote(rootfs))
			end
		end

		local invocation = "/bin/bash --noprofile --norc " .. Runtime.Quote(root .. "/distribution/toolchains/private/wsl-build.sh")
			.. " " .. Runtime.Quote(root) .. " " .. name .. " " .. builder.qtVersion .. " " .. builder.qtDirectory
			.. " " .. architecture .. " " .. Runtime.Quote(target.rustTarget)
		return function()
			run(invocation)
		end
	end

	local build = assert(drivers[builder.kind], "unsupported builder kind: " .. builder.kind)()
	return { name = name, build = build }
end

function Target.Build(prepared: Prepared)
	prepared.build()
end

function Target.Run(root: string, target: string, configuration: Settings.Configuration?)
	Target.Build(Target.Prepare(root, target, configuration))
end

return Target
