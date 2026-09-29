-- distribution/lib/verification/contracts.luau
--!strict

--# Modules

local ReadManifests = require("../configuration/read-manifests")

--# Functions

local Contracts = {}

function Contracts.Verify(root: string)
	local registry = ReadManifests.Read(root)
	local tools = ReadManifests.Tools(root)

	assert(registry.policy.verification.checksum == "sha256", "distribution/policy.toml requires sha256 verification")
	assert(tools.aqtinstall and tools.linuxdeploy and tools["appimage-runtime"], "required tool manifests are missing")

	local enabled = 0
	for identifier, target in pairs(registry.targets) do
		assert(target.distribution.platform ~= "", "build manifest platform is missing: " .. identifier)
		assert(target.distribution.architecture ~= "", "build manifest architecture is missing: " .. identifier)
		assert(target.rust.target ~= "", "build manifest Rust target is missing: " .. identifier)
		assert(target.qt.version ~= "", "build manifest Qt version is missing: " .. identifier)
		assert(target.builder.kind ~= "", "build manifest builder is missing: " .. identifier)
		for _, dependency in target.builder.dependsOn do
			assert(registry.targets[dependency] ~= nil, identifier .. " has unknown preparation dependency " .. dependency)
			assert(dependency ~= identifier, identifier .. " cannot depend on itself")
		end
		assert(target.distribution.finalArtifact ~= "", "build manifest final artifact is missing: " .. identifier)

		if target.distribution.enabled then
			enabled += 1
		end
	end

	assert(enabled == 4, "exactly four distributions are enabled")
	assert(registry.targets["macos-arm64"] == nil, "macOS is not currently a distribution definition")
end

return Contracts
