-- distribution/lib/hosted/create-github-build-matrix.luau
--!strict

--# Modules

local Settings = require("../configuration/settings")

--# Helpers

local function quote(value: string): string
	return '"' .. value:gsub("\\", "\\\\"):gsub('"', '\\"') .. '"'
end

local function field(name: string, value: string?): string?
	if value == nil then
		return nil
	end
	return quote(name) .. ":" .. quote(value)
end

--# Functions

local GitHubBuildMatrix = {}

function GitHubBuildMatrix.Create(root: string): string
	local include: { string } = {}
	local configuration = Settings.Read(root)

	for _, target in pairs(configuration.targets) do
		if target.enabled and target.hosted then
			local hosted = target.hosted
			local fields: { string } = {
				assert(field("target", target.name)),
				assert(field("os", hosted.runner)),
				assert(field("builder_kind", target.builder.kind)),
				assert(field("rust_target", target.rustTarget)),
				assert(field("qt_version", target.builder.qtVersion)),
				assert(field("qt_host", target.builder.qtHost)),
				assert(field("qt_arch", target.builder.qtArchitecture)),
			}

			local msvcArchitecture = field("msvc_arch", target.builder.msvcArchitecture)
			if msvcArchitecture then
				table.insert(fields, msvcArchitecture)
			end

			local qtHostArchitecture = field("qt_host_arch", target.builder.qtHostArchitecture)
			if qtHostArchitecture then
				table.insert(fields, qtHostArchitecture)
			end

			if target.builder.kind == "windows-llvm" then
				local llvm = configuration.tools.llvmMingwX64
				table.insert(fields, assert(field("llvm_mingw_version", llvm.version)))
				table.insert(fields, assert(field("llvm_mingw_archive", llvm.archive)))
				table.insert(fields, assert(field("llvm_mingw_sha256", llvm.sha256)))
			end

			if target.builder.kind == "wsl-linux" then
				local architecture = assert(target.builder.linuxdeployArchitecture, target.name .. " requires linuxdeploy architecture")
				local deploy = configuration.tools.linuxdeploy
				local runtime = configuration.tools.appImageRuntime
				local deployChecksum = if architecture == "aarch64" then deploy.aarch64Sha256 else deploy.x86_64Sha256
				local runtimeChecksum = if architecture == "aarch64" then runtime.aarch64Sha256 else runtime.x86_64Sha256
				table.insert(fields, assert(field("linuxdeploy_arch", architecture)))
				table.insert(fields, assert(field("linuxdeploy_version", deploy.version)))
				table.insert(fields, assert(field("linuxdeploy_base_url", deploy.baseUrl)))
				table.insert(fields, assert(field("linuxdeploy_sha256", deployChecksum)))
				table.insert(fields, assert(field("appimage_runtime_version", runtime.version)))
				table.insert(fields, assert(field("appimage_runtime_base_url", runtime.baseUrl)))
				table.insert(fields, assert(field("appimage_runtime_sha256", runtimeChecksum)))
			end

			table.sort(fields)
			table.insert(include, "{" .. table.concat(fields, ",") .. "}")
		end
	end

	table.sort(include)
	return '{"include":[' .. table.concat(include, ",") .. "]}"
end

return GitHubBuildMatrix
