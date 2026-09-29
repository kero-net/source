-- distribution/lib/verification/toolchain.luau
--!strict

--# Modules

local Runtime = require("../Runtime")
local Settings = require("../configuration/settings")

--# Helpers

local function requireCommand(root: string, executable: string, arguments: string, scope: string)
	local available = Runtime.Run(root, executable .. " " .. arguments, true, scope)
	assert(available, "required executable is unavailable: " .. executable)
end

--# Functions

local Toolchain = {}

function Toolchain.Verify(root: string, target: string)
	assert(target ~= "", "release target is required")
	local policy = assert(Settings.Target(root, target), "unknown distribution target: " .. target)

	for _, name in policy.requiredEnvironment do
		local value = Runtime.Environment(name)
		assert(value ~= nil and value ~= "", target .. " requires " .. name)
	end

	local cmake = Runtime.Environment("KERO_CMAKE") or "cmake"
	local cargo = Runtime.Environment("KERO_CARGO") or "cargo"
	local rustup = Runtime.Environment("KERO_RUSTUP") or "rustup"
	local scope = target .. "-build"
	requireCommand(root, cmake, "--version", scope)
	requireCommand(root, cargo, "--version", scope)

	local installed, details = Runtime.Capture(root, rustup .. " target list --installed")
	assert(installed and installed:find("wasm32%-wasip1"), target .. " requires Rust target wasm32-wasip1\n" .. tostring(details or ""))

	if target == "windows-arm64" then
		local compiler = Runtime.Environment("KERO_MSVC_LINKER_POSIX")
		assert(compiler and Runtime.IsFile(compiler), "windows-arm64 requires the MSVC ARM64 compiler")
	end

	Runtime.Log("[ok] Toolchain contract is present for " .. target .. ".")
end

return Toolchain
