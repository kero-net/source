-- distribution/lib/configuration/settings.luau
--!strict

--# Modules

local ReadManifests = require("./read-manifests")
local ResolveTools = require("../tools/resolve")

--# Types

export type Builder = {
	kind: string,
	dependsOn: { string },
	distribution: string?,
	linuxdeployArchitecture: string?,
	msvcArchitecture: string?,
	installerPackage: string?,
	installComponents: { string },
	cross: boolean?,
	qtVersion: string,
	qtHost: string,
	qtArchitecture: string,
	qtDirectory: string,
	qtHostArchitecture: string?,
}

export type Target = {
	name: string,
	enabled: boolean,
	lifecycle: string,
	platform: string,
	architecture: string,
	artifact: string,
	finalArtifact: string,
	nativeHost: string,
	preset: string,
	rustTarget: string,
	builder: Builder,
	requiredEnvironment: { string },
	hosted: ReadManifests.HostedManifest?,
}

export type Verification = {
	checksum: string,
	gpgKeyEnvironment: string,
	gpgPrivateKeyEnvironment: string,
}

export type Configuration = {
	verification: Verification,
	tools: ResolveTools.Configuration,
	targets: { [string]: Target },
}

--# Helpers

local function requiredEnvironment(builder: ReadManifests.BuilderManifest): { string }
	if builder.kind == "windows-llvm" then
		return { "KERO_QT_PREFIX", "KERO_CXX_COMPILER" }
	end

	return { "KERO_QT_PREFIX" }
end

local function configureTarget(manifest: ReadManifests.TargetManifest): Target
	local distribution = manifest.distribution
	local qt = manifest.qt
	local builder = manifest.builder

	return {
		name = distribution.id,
		enabled = distribution.enabled,
		lifecycle = distribution.lifecycle,
		platform = distribution.platform,
		architecture = distribution.architecture,
		artifact = distribution.artifact,
		finalArtifact = distribution.finalArtifact,
		nativeHost = distribution.nativeHost,
		preset = distribution.preset,
		rustTarget = manifest.rust.target,
		builder = {
			kind = builder.kind,
			dependsOn = builder.dependsOn,
			distribution = builder.distribution,
			linuxdeployArchitecture = builder.linuxdeployArchitecture,
			msvcArchitecture = builder.msvcArchitecture,
			installerPackage = builder.installerPackage,
			installComponents = builder.installComponents,
			cross = builder.cross,
			qtVersion = qt.version,
			qtHost = qt.host,
			qtArchitecture = qt.architecture,
			qtDirectory = qt.directory,
			qtHostArchitecture = qt.hostArchitecture,
		},
		requiredEnvironment = requiredEnvironment(builder),
		hosted = manifest.hosted,
	}
end

--# Functions

local Settings = {}

function Settings.Read(root: string): Configuration
	local registry = ReadManifests.Read(root)
	local targets: { [string]: Target } = {}

	for identifier, manifest in pairs(registry.targets) do
		targets[identifier] = configureTarget(manifest)
	end

	return {
		verification = {
			checksum = registry.policy.verification.checksum,
			gpgKeyEnvironment = registry.policy.verification.gpgKeyEnvironment,
			gpgPrivateKeyEnvironment = registry.policy.verification.gpgPrivateKeyEnvironment,
		},
		tools = ResolveTools.Read(root),
		targets = targets,
	}
end

function Settings.Target(root: string, name: string): Target?
	return Settings.Read(root).targets[name]
end

return Settings
