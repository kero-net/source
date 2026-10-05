-- distribution/lib/configuration/read-manifests.luau
--!strict

--# Modules

local Runtime = require("../Runtime")
local Toml = require("./toml")

--# Types

export type VerificationPolicy = {
	checksum: string,
	gpgKeyEnvironment: string,
	gpgPrivateKeyEnvironment: string,
}

export type PolicyManifest = {
	verification: VerificationPolicy,
}

export type DistributionManifest = {
	id: string,
	enabled: boolean,
	lifecycle: string,
	platform: string,
	architecture: string,
	artifact: string,
	finalArtifact: string,
	nativeHost: string,
	preset: string,
}

export type RustManifest = {
	target: string,
}

export type QtManifest = {
	version: string,
	host: string,
	architecture: string,
	directory: string,
	hostArchitecture: string?,
}

export type BuilderManifest = {
	kind: string,
	dependsOn: { string },
	distribution: string?,
	linuxdeployArchitecture: string?,
	msvcArchitecture: string?,
	installerPackage: string?,
	installComponents: { string },
	cross: boolean?,
}

export type HostedManifest = {
	runner: string,
}

export type TargetManifest = {
	distribution: DistributionManifest,
	rust: RustManifest,
	qt: QtManifest,
	builder: BuilderManifest,
	dependencies: { [string]: string },
	hosted: HostedManifest?,
}

export type Registry = {
	policy: PolicyManifest,
	targets: { [string]: TargetManifest },
}

export type ToolRegistry = { [string]: Toml.Table }

--# Helpers

local function buildManifestPaths(root: string): { string }
	local paths: { string } = {}
	local builds = Runtime.Path(root, "distribution/builds")

	for _, directory in Runtime.ReadDirectory(builds) do
		local path = builds .. "/" .. directory .. "/build.toml"
		if Runtime.IsFile(path) then
			table.insert(paths, path)
		end
	end

	table.sort(paths)
	return paths
end

local function toolManifestPaths(root: string): { string }
	local paths: { string } = {}
	local tools = Runtime.Path(root, "distribution/tools")

	for _, name in Runtime.ReadDirectory(tools) do
		if name:sub(-5) == ".toml" then
			table.insert(paths, tools .. "/" .. name)
		end
	end

	table.sort(paths)
	return paths
end

local function tableField(document: Toml.Table, key: string, path: string): Toml.Table
	local value = assert(document[key], path .. " requires [" .. key .. "]")
	assert(typeof(value) == "table", path .. "." .. key .. " must be a table")
	return value :: Toml.Table
end

local function stringField(document: Toml.Table, key: string, path: string): string
	local value = assert(document[key], path .. " requires " .. key)
	assert(typeof(value) == "string", path .. "." .. key .. " must be a string")
	return value
end

local function optionalStringField(document: Toml.Table, key: string, path: string): string?
	local value = document[key]
	if value == nil then
		return nil
	end

	assert(typeof(value) == "string", path .. "." .. key .. " must be a string")
	return value
end

local function booleanField(document: Toml.Table, key: string, path: string): boolean
	local value = assert(document[key], path .. " requires " .. key)
	assert(typeof(value) == "boolean", path .. "." .. key .. " must be a boolean")
	return value
end

local function optionalBooleanField(document: Toml.Table, key: string, path: string): boolean?
	local value = document[key]
	if value == nil then
		return nil
	end

	assert(typeof(value) == "boolean", path .. "." .. key .. " must be a boolean")
	return value
end

local function optionalStringArray(document: Toml.Table, key: string, path: string): { string }
	local value = document[key]
	if value == nil then
		return {}
	end

	assert(typeof(value) == "table", path .. "." .. key .. " must be an array")
	local result: { string } = {}
	for _, item in value :: Toml.Array do
		assert(typeof(item) == "string", path .. "." .. key .. " entries must be strings")
		table.insert(result, item)
	end
	return result
end

local function stringTable(document: Toml.Table, path: string): { [string]: string }
	local result: { [string]: string } = {}

	for key, value in pairs(document) do
		assert(typeof(value) == "string", path .. "." .. key .. " must be a string")
		result[key] = value
	end

	return result
end

local function readTarget(path: string): TargetManifest
	local document = Toml.Read(path)
	local distribution = tableField(document, "distribution", path)
	local rust = tableField(document, "rust", path)
	local qt = tableField(document, "qt", path)
	local builder = tableField(document, "builder", path)
	local dependencies = tableField(document, "dependencies", path)
	local hostedValue = document.hosted
	local hosted: HostedManifest? = nil

	if hostedValue ~= nil then
		assert(typeof(hostedValue) == "table", path .. ".hosted must be a table")
		local hostedTable = hostedValue :: Toml.Table
		hosted = {
			runner = stringField(hostedTable, "runner", path .. ".hosted"),
		}
	end

	return {
		distribution = {
			id = stringField(distribution, "id", path .. ".distribution"),
			enabled = booleanField(distribution, "enabled", path .. ".distribution"),
			lifecycle = stringField(distribution, "lifecycle", path .. ".distribution"),
			platform = stringField(distribution, "platform", path .. ".distribution"),
			architecture = stringField(distribution, "architecture", path .. ".distribution"),
			artifact = stringField(distribution, "artifact", path .. ".distribution"),
			finalArtifact = stringField(distribution, "final-artifact", path .. ".distribution"),
			nativeHost = stringField(distribution, "native-host", path .. ".distribution"),
			preset = stringField(distribution, "preset", path .. ".distribution"),
		},
		rust = {
			target = stringField(rust, "target", path .. ".rust"),
		},
		qt = {
			version = stringField(qt, "version", path .. ".qt"),
			host = stringField(qt, "host", path .. ".qt"),
			architecture = stringField(qt, "architecture", path .. ".qt"),
			directory = stringField(qt, "directory", path .. ".qt"),
			hostArchitecture = optionalStringField(qt, "host-architecture", path .. ".qt"),
		},
		builder = {
			kind = stringField(builder, "kind", path .. ".builder"),
			dependsOn = optionalStringArray(builder, "depends-on", path .. ".builder"),
			distribution = optionalStringField(builder, "distribution", path .. ".builder"),
			linuxdeployArchitecture = optionalStringField(builder, "linuxdeploy-architecture", path .. ".builder"),
			msvcArchitecture = optionalStringField(builder, "msvc-architecture", path .. ".builder"),
			installerPackage = optionalStringField(builder, "installer-package", path .. ".builder"),
			installComponents = optionalStringArray(builder, "install-components", path .. ".builder"),
			cross = optionalBooleanField(builder, "cross", path .. ".builder"),
		},
		dependencies = stringTable(dependencies, path .. ".dependencies"),
		hosted = hosted,
	}
end

--# Functions

local ReadManifests = {}

function ReadManifests.Read(root: string): Registry
	local policyPath = Runtime.Path(root, "distribution/policy.toml")
	local policy = tableField(Toml.Read(policyPath), "verification", policyPath)
	local targets: { [string]: TargetManifest } = {}

	for _, path in buildManifestPaths(root) do
		local manifest = readTarget(path)
		local identifier = manifest.distribution.id
		assert(targets[identifier] == nil, "duplicate distribution id: " .. identifier)
		targets[identifier] = manifest
	end

	return {
		policy = {
			verification = {
				checksum = stringField(policy, "checksum", policyPath .. ".verification"),
				gpgKeyEnvironment = stringField(policy, "gpg-key-environment", policyPath .. ".verification"),
				gpgPrivateKeyEnvironment = stringField(policy, "gpg-private-key-environment", policyPath .. ".verification"),
			},
		},
		targets = targets,
	}
end

function ReadManifests.Tools(root: string): ToolRegistry
	local tools: ToolRegistry = {}

	for _, path in toolManifestPaths(root) do
		local manifest = Toml.Read(path)
		local definition = tableField(manifest, "tool", path)
		local name = stringField(definition, "name", path .. ".tool")
		assert(tools[name] == nil, "duplicate tool manifest: " .. name)
		tools[name] = manifest
	end

	return tools
end

return ReadManifests
