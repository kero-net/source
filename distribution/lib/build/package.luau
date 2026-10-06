-- distribution/lib/build/package.luau
--!strict

--# Modules

local Runtime = require("../Runtime")
local Settings = require("../configuration/settings")
local SigningKey = require("../verification/signing-key")
local Toolchain = require("../verification/toolchain")

--# Helpers

local function run(root: string, line: string, scope: string)
	Runtime.Log(":: " .. line)
	local passed, message = Runtime.Run(root, line, true, scope)
	assert(passed, message)
end

local function runStep(root: string, job: string, action: string, message: string, line: string, scope: string)
	Runtime.Step(job, action, message)
	run(root, line, scope)
	Runtime.StepComplete(job)
end

local function requireEnvironment(name: string): string
	local value = Runtime.Environment(name)
	assert(value ~= nil and value ~= "", "package build requires " .. name)
	return value
end

local function writeWindowsIcon(root: string, target: string): string
	local png = Runtime.ReadFile(root .. "/assets/images/kero-icon.png")
	local icon = root .. "/.heap/cache/build/qt/" .. target .. "/kero-icon.ico"
	local size = #png
	local offset = 22
	local header = string.char(0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 32, 0)
	local directory = string.char(size % 256, math.floor(size / 256) % 256, math.floor(size / 65536) % 256, math.floor(size / 16777216) % 256)
	Runtime.WriteFile(icon, header .. directory .. string.char(offset, 0, 0, 0) .. png)
	return icon
end

local function signArtifact(root: string, artifact: string, scope: string)
	local cmake = Runtime.Environment("KERO_CMAKE") or "cmake"
	run(root, cmake .. " -E sha256sum " .. Runtime.Quote(artifact) .. " > " .. Runtime.Quote(artifact .. ".sha256"), scope)

	if Runtime.Environment("KERO_SIGN_RELEASE") ~= "1" then
		return
	end

	local key = requireEnvironment("KERO_GPG_KEY_ID")
	local gpg = Runtime.Environment("KERO_GPG_COMMAND") or "gpg"
	run(root, Runtime.Quote(gpg) .. " --batch --local-user " .. Runtime.Quote(key) .. " --detach-sign --armor " .. Runtime.Quote(artifact), scope)
	run(root, Runtime.Quote(gpg) .. " --verify " .. Runtime.Quote(artifact .. ".asc") .. " " .. Runtime.Quote(artifact), scope)
end

--# Functions

local Package = {}

function Package.Run(root: string, targetName: string)
	if root == "." then
		root = Runtime.CurrentDirectory()
	end
	local target = assert(Settings.Target(root, targetName), "unknown distribution target: " .. targetName)
	assert(target.enabled, targetName .. " is " .. target.lifecycle)

	local job = "build and publish " .. targetName
	Runtime.Step(job, "Checking", targetName .. " toolchain")
	Toolchain.Verify(root, targetName)
	Runtime.StepComplete(job)

	if Runtime.Environment("KERO_SIGN_RELEASE") == "1" then
		Runtime.Step(job, "Checking", "release signing configuration")
		assert(SigningKey.Verify(root), "release signing requires an available private key")
		Runtime.StepComplete(job)
	end

	local cargo = Runtime.Environment("KERO_CARGO") or "cargo"
	local rustup = Runtime.Environment("KERO_RUSTUP") or "rustup"
	local cmake = Runtime.Environment("KERO_CMAKE") or "cmake"
	local rustToolchain = Runtime.Environment("KERO_RUST_TOOLCHAIN")
	local toolchainOption = if rustToolchain and rustToolchain ~= "" then " --toolchain " .. rustToolchain else ""
	local cargoTarget = root .. "/.heap/cache/build/cargo/" .. targetName
	local wasm = root .. "/.heap/cache/build/cargo/wasm/wasm32-wasip1/release/kero_core.wasm"
	assert(Runtime.IsFile(wasm), "shared WASM core is missing: " .. wasm)
	local hostExtension = if target.platform == "windows" then ".exe" else ""
	local host = cargoTarget .. "/" .. target.rustTarget .. "/release/kero-host" .. hostExtension
	local build = root .. "/.heap/cache/build/qt/" .. targetName
	local release = root .. "/.heap/cache/build/releases/" .. targetName
	local scope = targetName .. "-build"

	local cargoEnvironment = "CARGO_TARGET_DIR=" .. Runtime.Quote(cargoTarget) .. " "
	runStep(root, job, "Installing", "Rust target", rustup .. " target add" .. toolchainOption .. " " .. Runtime.Quote(target.rustTarget), scope)
	runStep(root, job, "Compiling", "kero-cli for " .. targetName, cargoEnvironment .. cargo .. " build --manifest-path src/Cargo.toml --release -p kero-cli --target " .. Runtime.Quote(target.rustTarget), scope)

	local deployTool = Runtime.Environment("KERO_DEPLOY_TOOL") or ""
	if target.platform == "windows" then
		local function windowsPath(path: string): string
			if Runtime.IsWindows() then
				local output, message = Runtime.Capture(root, "cygpath -m " .. Runtime.Quote(path))
				return assert(output, "cannot translate native Windows package path: " .. tostring(message))
			end
			local output, message = Runtime.Capture(root, "wslpath -m " .. Runtime.Quote(path))
			return assert(output, "cannot translate WSL package path: " .. tostring(message))
		end
		Runtime.Step(job, "Configuring", "Qt package for " .. targetName)
		run(root, cmake .. " -E make_directory " .. Runtime.Quote(build), scope)

		local icon = writeWindowsIcon(root, targetName)
		local environment = "KERO_ICON=" .. Runtime.Quote(windowsPath(icon))
			.. " KERO_WASM=" .. Runtime.Quote(windowsPath(wasm))
			.. " KERO_HOST=" .. Runtime.Quote(windowsPath(host))
		if deployTool ~= "" then
			environment ..= " KERO_DEPLOY_TOOL=" .. Runtime.Quote(windowsPath(deployTool))
		elseif targetName ~= "windows-arm64" then
			error("package build requires KERO_DEPLOY_TOOL")
		end
		run(root, environment .. " " .. cmake .. " --preset " .. target.preset .. " -S host/qt -B " .. Runtime.Quote(build), scope)
		Runtime.StepComplete(job)
		run(root, cmake .. " -E rm -rf " .. Runtime.Quote(build .. "/stage") .. " " .. Runtime.Quote(build .. "/bootstrap"), scope)

		runStep(root, job, "Packaging", targetName, cmake .. " --build " .. Runtime.Quote(build) .. " --target kero-package", scope)
		runStep(root, job, "Packaging", "Windows installer", cmake .. " --build " .. Runtime.Quote(build) .. " --target kero-windows-installer", scope)

		Runtime.Step(job, "Checking", "artifact checksum and signature")
		signArtifact(root, release .. "/kero-installer.exe", scope)
		Runtime.StepComplete(job)
		return
	end

	if deployTool == "" then
		error("package build requires KERO_DEPLOY_TOOL")
	end
	local environment = "KERO_WASM=" .. Runtime.Quote(wasm)
		.. " KERO_HOST=" .. Runtime.Quote(host)
		.. " KERO_DEPLOY_TOOL=" .. Runtime.Quote(deployTool)
	runStep(root, job, "Configuring", "Qt package for " .. targetName, environment .. " " .. cmake .. " --preset " .. target.preset .. " -S host/qt -B " .. Runtime.Quote(build), scope)
	runStep(root, job, "Packaging", targetName, cmake .. " --build " .. Runtime.Quote(build) .. " --target kero-package", scope)
	runStep(root, job, "Packaging", "Linux AppImage", cmake .. " --build " .. Runtime.Quote(build) .. " --target kero-linux-appimage", scope)

	Runtime.Step(job, "Checking", "artifact checksum and signature")
	signArtifact(root, release .. "/kero.AppImage", scope)
	Runtime.StepComplete(job)
end

return Package
