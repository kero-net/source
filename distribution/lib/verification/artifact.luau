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
	local release = root .. "/.heap/cache/build/releases/" .. target
	if target:match("^windows") then
		return release .. "/kero-installer.exe"
	end
	return release .. "/kero.AppImage"
end

--# Functions

local Artifact = {}

function Artifact.Verify(root: string, target: string, runtime: boolean)
	local artifact = artifactPath(root, target)
	Runtime.Status("Checking", target .. " package")
	readBytes(artifact)
	Runtime.Status("Checking", target .. " checksum")
	assert(Runtime.IsFile(artifact .. ".sha256"), "missing package checksum: " .. artifact)

	if target:match("^windows") then
		Runtime.Status("Checking", target .. " PE machine type")
		local expected = if target == "windows-arm64" then 0xAA64 else 0x8664
		assert(peMachine(artifact) == expected, artifact .. " has the wrong machine type")

		local stage = root .. "/.heap/cache/build/qt/" .. target .. "/stage"
		local listing, message = Runtime.Capture(root, "find " .. Runtime.Quote(stage) .. " -type f -iname '*.dll' -print")
		assert(listing, "cannot list staged DLLs: " .. tostring(message))

		for path in listing:gmatch("[^\r\n]+") do
			assert(peMachine(path) == expected, path .. " has the wrong machine type")
		end

		if runtime then
			local command = "timeout 30s " .. Runtime.Quote(artifact) .. " --verify-payload"
			if Runtime.IsWsl() then
				local script, scriptError = Runtime.Capture(root, "wslpath -m " .. Runtime.Quote(root .. "/distribution/toolchains/private/verify-windows-payload.ps1"))
				local installer, installerError = Runtime.Capture(root, "wslpath -m " .. Runtime.Quote(artifact))
				assert(script and installer, "cannot translate Windows verifier paths: " .. tostring(scriptError or installerError))
				command = "timeout 30s powershell.exe -NoProfile -NonInteractive -File " .. Runtime.Quote(script) .. " -Installer " .. Runtime.Quote(installer)
			end
			local passed, details = Runtime.Run(root, command, true, target .. "-build")
			assert(passed, "bootstrap payload verification failed: " .. details)
		end
	else
		Runtime.Status("Checking", target .. " ELF AppImage")
		local bytes = readBytes(artifact)
		assert(#bytes > 1024, "Linux AppImage is implausibly small")
		assert(bytes:sub(1, 4) == "\127ELF", "Linux package is not an ELF AppImage")
		local expected = if target == "linux-arm64" then 183 else 62
		assert(string.unpack("<I2", bytes, 19) == expected, artifact .. " has the wrong ELF machine type")
	end

	Runtime.Log("[ok] Verified " .. target .. " artifact.")
end

return Artifact
