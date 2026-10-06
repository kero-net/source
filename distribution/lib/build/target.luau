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

function Target.PrepareHostTools(root: string, configuration: Settings.Configuration)
	local version = configuration.tools.aqtinstall
	local abi, message = Runtime.Capture(root, "python3 -c 'import platform,sys; print(platform.machine()+\"-py\"+str(sys.version_info.major)+\".\"+str(sys.version_info.minor))'")
	assert(abi, "cannot identify Python host ABI: " .. tostring(message))
	local cacheRoot = Runtime.Environment("KERO_WSL_WORK_ROOT") or (root .. "/.heap")
	local packages = cacheRoot .. "/cache/toolchains/python/aqtinstall-" .. version .. "-" .. abi
	if Runtime.IsDirectory(packages .. "/aqt") then return end
	local cache = cacheRoot .. "/cache/pip"
	local passed, failure = Runtime.Run(root, "mkdir -p " .. Runtime.Quote(packages) .. " " .. Runtime.Quote(cache)
		.. " && python3 -m pip install --disable-pip-version-check --cache-dir " .. Runtime.Quote(cache)
		.. " --target " .. Runtime.Quote(packages) .. " aqtinstall==" .. version, true, "host-tool-preparation")
	assert(passed, failure)
end

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
	local job = "toolchain preflight"
	local windows = configuration.targets["windows-arm64"]
	if not windows or not windows.enabled then
		Runtime.Step(job, "Skipping", "Windows ARM64 compiler check")
		Runtime.StepComplete(job)
		Runtime.Step(job, "Skipping", "Windows ARM64 build tools")
		Runtime.StepComplete(job)
		return
	end

	Runtime.Step(job, "Checking", "Windows ARM64 compiler")
	local compiler = windowsArmCompiler(root)
	Runtime.StepComplete(job)
	if compiler ~= "" then
		Runtime.Step(job, "Checking", "Windows ARM64 build tools")
		Runtime.StepComplete(job)
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
	Runtime.Step(job, "Installing", "Windows ARM64 build tools")
	local line = "winget.exe install --exact --source winget --accept-package-agreements --accept-source-agreements --id "
		.. Runtime.Quote(package) .. " --override " .. Runtime.Quote(table.concat(override, " "))
	local passed, message = Runtime.Run(root, line, true, "toolchain-preflight")
	assert(passed, "Visual Studio Build Tools installation failed: " .. message)
	assert(windowsArmCompiler(root) ~= "", "Visual Studio installation completed without the required ARM64 compiler")
	Runtime.StepComplete(job)
end

local function environmentScope(values: { [string]: string }, callback: () -> ())
	local previous: { [string]: string? } = {}
	for name, value in pairs(values) do
		previous[name] = Runtime.Environment(name)
		Runtime.SetEnvironment(name, value)
	end
	local succeeded, failure = pcall(callback)
	for name in pairs(values) do
		Runtime.SetEnvironment(name, previous[name] or "")
	end
	if not succeeded then
		error(tostring(failure), 0)
	end
end

function Target.PreparationSteps(target: Settings.Target, configuration: Settings.Configuration): number
	if target.builder.kind == "windows-msvc" then
		return if target.builder.qtHostArchitecture then 3 else 2
	end
	if target.builder.kind == "windows-llvm" then
		return 2
	end
	if target.builder.kind == "wsl-linux" then
		local total = 3
		if target.name == "linux-x64" then
			total += 4 + #configuration.tools.ubuntuX64Libraries
		end
		return total
	end
	error("unsupported builder kind: " .. target.builder.kind)
end

function Target.Prepare(root: string, name: string, configuration: Settings.Configuration?): Prepared
	local activeConfiguration = configuration or Settings.Read(root)
	local target = assert(activeConfiguration.targets[name], "unknown distribution target: " .. name)
	assert(target.enabled, name .. " is " .. target.lifecycle)
	local builder = target.builder
	local job = "prepare " .. name
	local dependencies = root .. "/.heap/cache/downloads"
	local tools = root .. "/.heap/cache/toolchains"
	if builder.kind == "wsl-linux" then
		local wslRoot = assert(Runtime.Environment("KERO_WSL_WORK_ROOT"), "Linux builds require an ext4 work root")
		tools = wslRoot .. "/cache/toolchains"
	end

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
		local cached = Runtime.IsFile(path)
		Runtime.Step(job, if cached then "Checking" else "Downloading", packageName)
		run("mkdir -p " .. Runtime.Quote(directory))
		downloadStates[path] = "active"
		local succeeded, failure = pcall(function()
			if not cached then
				local temporary = path .. ".partial"
				local dns = if Runtime.Environment("KERO_CURL_DOH") == "1"
					then " --doh-url https://1.1.1.1/dns-query --resolve 1.1.1.1:443:1.1.1.1"
					else ""
				run("rm -f " .. Runtime.Quote(temporary)
					.. " && curl --fail --location --retry 3" .. dns .. " " .. Runtime.Quote(url)
					.. " --output " .. Runtime.Quote(temporary))
				local temporaryOutput = capture("sha256sum " .. Runtime.Quote(temporary))
				local temporaryDigest = assert(temporaryOutput:match("^([0-9a-fA-F]+)"), "checksum output is invalid: " .. temporary)
				assert(temporaryDigest:lower() == checksum, "download checksum mismatch: " .. path)
				run("mv " .. Runtime.Quote(temporary) .. " " .. Runtime.Quote(path))
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
		Runtime.StepComplete(job)
	end

	local function ensureQt(item: Settings.Builder, targetName: string, architecture: string?, directory: string?): string
		local prefix = tools .. "/qt/" .. targetName
		local kit = prefix .. "/" .. item.qtVersion .. "/" .. (directory or item.qtDirectory)
		local needsIcu = item.qtHost:match("^linux") ~= nil
		local installed = Runtime.IsFile(kit .. "/lib/cmake/Qt6/Qt6Config.cmake")
			and (not needsIcu or Runtime.IsFile(kit .. "/lib/libicui18n.so.73"))
		Runtime.Step(job, if installed then "Checking" else "Downloading", "Qt " .. item.qtVersion .. " " .. (directory or item.qtDirectory) .. " for " .. targetName)
		if installed then
			Runtime.StepComplete(job)
			return kit
		end

		local abi = assert(Runtime.Capture(root, "python3 -c 'import platform,sys; print(platform.machine()+\"-py\"+str(sys.version_info.major)+\".\"+str(sys.version_info.minor))'"))
		local cacheRoot = Runtime.Environment("KERO_WSL_WORK_ROOT") or (root .. "/.heap")
		local packages = cacheRoot .. "/cache/toolchains/python/aqtinstall-" .. activeConfiguration.tools.aqtinstall .. "-" .. abi
		local archives = dependencies .. "/qt/" .. targetName
		local logs = root .. "/.heap/cache/logs/" .. targetName
		assert(Runtime.IsDirectory(packages .. "/aqt"), "shared aqtinstall host tool was not prepared")
		run("mkdir -p " .. Runtime.Quote(archives) .. " " .. Runtime.Quote(logs))
		local invocation = "cd " .. Runtime.Quote(logs)
			.. " && PYTHONPATH=" .. Runtime.Quote(packages)
			.. " python3 -m aqt install-qt " .. item.qtHost .. " desktop " .. item.qtVersion .. " " .. (architecture or item.qtArchitecture)
			.. " --archives qtbase" .. (if needsIcu then " icu" else "")
			.. " -k -d " .. Runtime.Quote(archives) .. " -O " .. Runtime.Quote(prefix)
		run(invocation)
		assert(Runtime.IsFile(kit .. "/lib/cmake/Qt6/Qt6Config.cmake")
			and (not needsIcu or Runtime.IsFile(kit .. "/lib/libicui18n.so.73")), "Qt installation is incomplete for " .. targetName)
		Runtime.StepComplete(job)
		return kit
	end

	local drivers: { [string]: () -> (() -> ()) } = {}

	drivers["windows-msvc"] = function()
		assert(Runtime.IsWsl(), "windows-msvc requires the WSL controller")
		local qt = ensureQt(builder, name)
		local qtHost: string? = nil
		if builder.qtHostArchitecture then
			local hostDirectory = assert(builder.qtHostArchitecture:match("^win64_(.+)$"), "unsupported Qt host architecture")
			qtHost = ensureQt(builder, name, builder.qtHostArchitecture, hostDirectory)
		end
		Runtime.Step(job, "Checking", "Windows ARM64 compiler and SDK")
		local compiler = windowsArmCompiler(root)
		local sdkInclude = capture([[find '/mnt/c/Program Files (x86)/Windows Kits/10/Include' -type f -ipath '*/um/windows.h' -print 2>/dev/null | sort | tail -n 1]])
		assert(compiler ~= "" and sdkInclude ~= "", "Windows ARM64 requires Visual Studio Build Tools with ARM64 C++ tools and the Windows SDK")
		Runtime.StepComplete(job)
		local msvc = assert(compiler:match("^(.+)/bin/Hostarm64/arm64/cl%.exe$"))
		local sdk = assert(sdkInclude:match("/Include/([^/]+)/um/[Ww]indows%.h$"))
		local kits = "/mnt/c/Program Files (x86)/Windows Kits/10"
		local bin = msvc .. "/bin/Hostarm64/arm64"
		local environment = {
			KERO_QT_PREFIX = qt,
			KERO_QT_HOST_PATH = qtHost or "",
			KERO_MSVC_ROOT = msvc,
			KERO_WINDOWS_SDK = kits,
			KERO_WINDOWS_SDK_VERSION = sdk,
			KERO_MSVC_LINKER = bin .. "/link.exe",
			KERO_MSVC_LINKER_POSIX = bin .. "/link.exe",
			KERO_MSVC_ARCHIVER = bin .. "/lib.exe",
			KERO_MSVC_RC = capture("wslpath -m " .. Runtime.Quote(kits .. "/bin/" .. sdk .. "/arm64/rc.exe")),
			KERO_MSVC_MT = capture("wslpath -m " .. Runtime.Quote(kits .. "/bin/" .. sdk .. "/arm64/mt.exe")),
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
			KERO_DEPLOY_TOOL = (qtHost or qt) .. "/bin/windeployqt.exe",
			WSLENV = wslEnvironment({ "KERO_QT_PREFIX/p", "KERO_QT_HOST_PATH/p", "KERO_MSVC_ROOT/p", "KERO_WINDOWS_SDK/p", "KERO_MSVC_LINKER/p", "KERO_MSVC_ARCHIVER/p", "KERO_MSVC_RC", "KERO_MSVC_MT", "KERO_CXX_COMPILER/p", "CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER/p", "VCToolsInstallDir/p", "VCINSTALLDIR/p", "WindowsSdkDir/p", "UniversalCRTSdkDir/p", "KERO_WINDOWS_SDK_VERSION", "WindowsSDKVersion", "UCRTVersion", "INCLUDE/lp", "LIB/lp", "CARGO_TARGET_DIR/p", "KERO_ICON", "KERO_WASM", "KERO_HOST", "KERO_DEPLOY_TOOL" }),
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
			Runtime.Status("Extracting", "llvm-mingw " .. definition.version)
			run("mkdir -p " .. Runtime.Quote(tools .. "/llvm-mingw") .. " && unzip -q " .. Runtime.Quote(archive) .. " -d " .. Runtime.Quote(tools .. "/llvm-mingw"))
		end
		local bin = extracted .. "/bin"
		local environment = {
			KERO_QT_PREFIX = qt,
			KERO_CXX_COMPILER = bin .. "/x86_64-w64-mingw32-clang++.exe",
			KERO_WINDRES = capture("wslpath -m " .. Runtime.Quote(bin .. "/x86_64-w64-mingw32-windres.exe")),
			KERO_RUNTIME_DIR = extracted .. "/x86_64-w64-mingw32/bin",
			KERO_CARGO = "cargo.exe +stable-aarch64-pc-windows-gnullvm",
			KERO_RUSTUP = "rustup.exe",
			KERO_RUST_TOOLCHAIN = "stable-aarch64-pc-windows-gnullvm",
			KERO_CMAKE = "cmake.exe",
			KERO_DEPLOY_TOOL = qt .. "/bin/windeployqt.exe",
			PATH = bin .. ":" .. (Runtime.Environment("PATH") or ""),
			WSLENV = wslEnvironment({ "KERO_QT_PREFIX/p", "KERO_CXX_COMPILER/p", "KERO_WINDRES", "KERO_RUNTIME_DIR/p", "CARGO_TARGET_DIR/p", "KERO_ICON", "KERO_WASM", "KERO_HOST", "KERO_DEPLOY_TOOL" }),
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
				Runtime.Status("Extracting", "Zig " .. zig.version)
				run("mkdir -p " .. Runtime.Quote(tools .. "/zig") .. " && tar -xf " .. Runtime.Quote(zigArchive) .. " -C " .. Runtime.Quote(tools .. "/zig"))
			end
			local qemu = activeConfiguration.tools.qemuX64
			local qemuArchive = dependencies .. "/qemu/" .. qemu.archive
			download(qemu.url, qemuArchive, qemu.sha256)
			if not Runtime.IsFile(tools .. "/qemu/usr/bin/qemu-x86_64-static") then
				Runtime.Status("Extracting", "QEMU x86_64")
				run("dpkg-deb -x " .. Runtime.Quote(qemuArchive) .. " " .. Runtime.Quote(tools .. "/qemu"))
			end
			local image = activeConfiguration.tools.ubuntuX64Rootfs
			local imageArchive = dependencies .. "/ubuntu-rootfs/" .. image.archive
			download(image.url, imageArchive, image.sha256)
			local rootfs = tools .. "/linux-x64/rootfs"
			if not Runtime.IsFile(rootfs .. "/bin/bash") then
				Runtime.Status("Extracting", "Ubuntu x86_64 rootfs")
				run("mkdir -p " .. Runtime.Quote(rootfs) .. " && tar -xzf " .. Runtime.Quote(imageArchive) .. " -C " .. Runtime.Quote(rootfs))
			end
			for _, library in activeConfiguration.tools.ubuntuX64Libraries do
				local libraryArchive = dependencies .. "/ubuntu-libraries/" .. library.archive
				download(library.url, libraryArchive, library.sha256)
				Runtime.Status("Extracting", library.archive)
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
