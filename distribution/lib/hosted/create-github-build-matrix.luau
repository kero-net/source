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

	for _, target in pairs(Settings.Read(root).targets) do
		if target.enabled and target.hosted then
			local hosted = target.hosted
			local fields: { string } = {
				assert(field("target", target.name)),
				assert(field("os", hosted.runner)),
				assert(field("lua", hosted.lua)),
				assert(field("qt_version", target.builder.qtVersion)),
				assert(field("qt_host", target.builder.qtHost)),
				assert(field("qt_arch", target.builder.qtArchitecture)),
			}

			local msvcArchitecture = field("msvc_arch", target.builder.msvcArchitecture)
			if msvcArchitecture then
				table.insert(fields, msvcArchitecture)
			end

			table.sort(fields)
			table.insert(include, "{" .. table.concat(fields, ",") .. "}")
		end
	end

	table.sort(include)
	return '{"include":[' .. table.concat(include, ",") .. "]}"
end

return GitHubBuildMatrix
