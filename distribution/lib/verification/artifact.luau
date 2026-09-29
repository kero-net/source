-- distribution/lib/verification/artifact.luau
--!strict

--# Modules

local Runtime = require("../Runtime")

--# Helpers

local function readBytes(path: string): string
	assert(Runtime.IsFile(path), "missing package artifact: " .. path)
	return Runtime.ReadFile(path)
end

local function peMachine(path: string): number
	local value = readBytes(path)
	assert(value:sub(1, 2) == "MZ", path .. " is not a PE executable")

	local offset = string.unpack("<I4", value, 0x3D)
	assert(value:sub(offset + 1, offset + 4) == "PE\0\0", path .. " has no PE signature")
	return string.unpack("<I2", value, offset + 5)
end

local function artifactPath(root: string, target: string): string
	local release = root .. "/.heap/build/releases/" .. target
	if target:match("^windows") then
		return release .. "/kero-installer.exe"
	end
	return release .. "/kero.AppImage"
end

--# Functions

local Artifact = {}

function Artifact.Verify(root: string, target: string, runtime: boolean)
	local artifact = artifactPath(root, target)
	readBytes(artifact)
	assert(Runtime.IsFile(artifact .. ".sha256"), "missing package checksum: " .. artifact)

	if target:match("^windows") then
		local expected = if target == "windows-arm64" then 0xAA64 else 0x8664
		assert(peMachine(artifact) == expected, artifact .. " has the wrong machine type")

		local stage = root .. "/.heap/build/qt/" .. target .. "/stage"
		local listing, message = Runtime.Capture(root, "find " .. Runtime.Quote(stage) .. " -type f -iname '*.dll' -print")
		assert(listing, "cannot list staged DLLs: " .. tostring(message))

		for path in listing:gmatch("[^\r\n]+") do
			assert(peMachine(path) == expected, path .. " has the wrong machine type")
		end

		if runtime then
			local command = "timeout 15s " .. Runtime.Quote(artifact) .. " --verify-payload"
			local passed, details = Runtime.Run(root, command, true, target .. "-build")
			assert(passed, "bootstrap payload verification failed: " .. details)
		end
	else
		local bytes = readBytes(artifact)
		assert(#bytes > 1024, "Linux AppImage is implausibly small")
		assert(bytes:sub(1, 4) == "\127ELF", "Linux package is not an ELF AppImage")
	end

	Runtime.Log("[ok] Verified " .. target .. " artifact.")
end

return Artifact
