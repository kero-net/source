-- distribution/lib/tools/resolve.luau
--!strict

--# Modules

local ReadManifests = require("../configuration/read-manifests")
local Toml = require("../configuration/toml")

--# Types

export type Download = {
	archive: string,
	url: string,
	sha256: string,
}

export type VersionedDownload = {
	version: string,
	archive: string,
	url: string,
	sha256: string,
}

export type Configuration = {
	aqtinstall: string,
	llvmMingw: { version: string, archive: string, baseUrl: string, sha256: string },
	llvmMingwX64: { version: string, archive: string, baseUrl: string, sha256: string },
	zig: VersionedDownload,
	linuxdeploy: { version: string, baseUrl: string, aarch64Sha256: string, x86_64Sha256: string },
	appImageRuntime: { version: string, baseUrl: string, aarch64Sha256: string, x86_64Sha256: string },
	qemuX64: VersionedDownload,
	ubuntuX64Rootfs: VersionedDownload,
	ubuntuX64Libraries: { Download },
}

--# Helpers

local function tableField(tableValue: Toml.Table, key: string, context: string): Toml.Table
	local value = assert(tableValue[key], context .. " requires " .. key)
	assert(typeof(value) == "table", context .. "." .. key .. " must be a table")
	return value :: Toml.Table
end

local function arrayField(tableValue: Toml.Table, key: string, context: string): Toml.Array
	local value = assert(tableValue[key], context .. " requires " .. key)
	assert(typeof(value) == "table", context .. "." .. key .. " must be an array")
	return value :: Toml.Array
end

local function stringField(tableValue: Toml.Table, key: string, context: string): string
	local value = assert(tableValue[key], context .. " requires " .. key)
	assert(typeof(value) == "string", context .. "." .. key .. " must be a string")
	return value
end

local function requiredTool(tools: ReadManifests.ToolRegistry, name: string): Toml.Table
	return assert(tools[name], "missing distribution/tools/" .. name .. ".toml")
end

local function toolVersion(tool: Toml.Table, name: string, version: string, architecture: string?): Toml.Table
	local versions = tableField(tool, "versions", name)
	local entry = tableField(versions, version, name .. " version")
	if architecture == nil then
		return entry
	end
	return tableField(entry, architecture, name .. " architecture")
end

local function versionedDownload(tool: Toml.Table, name: string, version: string, architecture: string): Download
	local entry = toolVersion(tool, name, version, architecture)
	return {
		archive = stringField(entry, "archive", name),
		url = stringField(entry, "url", name),
		sha256 = stringField(entry, "sha256", name),
	}
end

--# Functions

local Resolve = {}

function Resolve.Read(root: string): Configuration
	local tools = ReadManifests.Tools(root)
	local aqtinstall = requiredTool(tools, "aqtinstall")
	local llvmMingw = requiredTool(tools, "llvm-mingw")
	local zig = requiredTool(tools, "zig")
	local linuxdeploy = requiredTool(tools, "linuxdeploy")
	local appImageRuntime = requiredTool(tools, "appimage-runtime")
	local qemu = requiredTool(tools, "qemu")
	local rootfs = requiredTool(tools, "ubuntu-rootfs")
	local libraries = requiredTool(tools, "ubuntu-libraries")

	local llvmVersion = "20260922"
	local zigVersion = "0.15.2"
	local linuxdeployVersion = "1-alpha-20251107-1"
	local runtimeVersion = "20251108"
	local qemuVersion = "8.2.2-0ubuntu1.18"
	local ubuntuVersion = "24.04.3"

	local llvm = toolVersion(llvmMingw, "llvm-mingw", llvmVersion, "aarch64")
	local llvmX64 = toolVersion(llvmMingw, "llvm-mingw", llvmVersion, "x86_64")
	local deploy = toolVersion(linuxdeploy, "linuxdeploy", linuxdeployVersion)
	local runtime = toolVersion(appImageRuntime, "appimage-runtime", runtimeVersion)
	local libraryVersion = toolVersion(libraries, "ubuntu-libraries", ubuntuVersion)
	local packages: { Download } = {}
	local baseUrl = stringField(libraryVersion, "base-url", "ubuntu-libraries")

	for _, value in arrayField(libraryVersion, "packages", "ubuntu-libraries") do
		assert(typeof(value) == "table", "ubuntu-libraries package must be a table")
		local package = value :: Toml.Table
		local archive = stringField(package, "archive", "ubuntu-libraries package")
		table.insert(packages, {
			archive = archive,
			url = baseUrl .. "/" .. archive,
			sha256 = stringField(package, "sha256", "ubuntu-libraries package"),
		})
	end

	local linuxdeployAarch64 = toolVersion(linuxdeploy, "linuxdeploy", linuxdeployVersion, "aarch64")
	local linuxdeployX64 = toolVersion(linuxdeploy, "linuxdeploy", linuxdeployVersion, "x86_64")
	local runtimeAarch64 = toolVersion(appImageRuntime, "appimage-runtime", runtimeVersion, "aarch64")
	local runtimeX64 = toolVersion(appImageRuntime, "appimage-runtime", runtimeVersion, "x86_64")
	local zigDownload = versionedDownload(zig, "zig", zigVersion, "aarch64")
	local qemuDownload = versionedDownload(qemu, "qemu", qemuVersion, "aarch64")
	local rootfsDownload = versionedDownload(rootfs, "ubuntu-rootfs", ubuntuVersion, "x86_64")

	return {
		aqtinstall = stringField(tableField(aqtinstall, "tool", "aqtinstall"), "version", "aqtinstall"),
		llvmMingw = { version = llvmVersion, archive = stringField(llvm, "archive", "llvm-mingw"), baseUrl = stringField(llvm, "base-url", "llvm-mingw"), sha256 = stringField(llvm, "sha256", "llvm-mingw") },
		llvmMingwX64 = { version = llvmVersion, archive = stringField(llvmX64, "archive", "llvm-mingw"), baseUrl = stringField(llvmX64, "base-url", "llvm-mingw"), sha256 = stringField(llvmX64, "sha256", "llvm-mingw") },
		zig = { version = zigVersion, archive = zigDownload.archive, url = zigDownload.url, sha256 = zigDownload.sha256 },
		linuxdeploy = { version = linuxdeployVersion, baseUrl = stringField(deploy, "base-url", "linuxdeploy"), aarch64Sha256 = stringField(linuxdeployAarch64, "sha256", "linuxdeploy"), x86_64Sha256 = stringField(linuxdeployX64, "sha256", "linuxdeploy") },
		appImageRuntime = { version = runtimeVersion, baseUrl = stringField(runtime, "base-url", "appimage-runtime"), aarch64Sha256 = stringField(runtimeAarch64, "sha256", "appimage-runtime"), x86_64Sha256 = stringField(runtimeX64, "sha256", "appimage-runtime") },
		qemuX64 = { version = qemuVersion, archive = qemuDownload.archive, url = qemuDownload.url, sha256 = qemuDownload.sha256 },
		ubuntuX64Rootfs = { version = ubuntuVersion, archive = rootfsDownload.archive, url = rootfsDownload.url, sha256 = rootfsDownload.sha256 },
		ubuntuX64Libraries = packages,
	}
end

return Resolve
