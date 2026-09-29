-- distribution/lib/configuration/toml.luau
--!strict

--# Modules

local Serde = require("@lune/serde")
local Runtime = require("../Runtime")

--# Types

export type Scalar = boolean | number | string
export type Value = Scalar | Array | Table
export type Array = { Value }
export type Table = { [string]: Value }

--# Helpers

local function describe(path: string, key: string): string
	return path .. ": invalid TOML value at " .. key
end

local function normalize(value: unknown, path: string, key: string): Value
	local kind = typeof(value)

	if kind == "boolean" then
		return value :: boolean
	end

	if kind == "number" then
		return value :: number
	end

	if kind == "string" then
		return value :: string
	end

	assert(kind == "table", describe(path, key))

	local raw = value :: { [unknown]: unknown }
	local count = #raw
	local isArray = count > 0

	for entryKey, _ in pairs(raw) do
		if typeof(entryKey) == "number" then
			assert(entryKey >= 1 and entryKey <= count and entryKey % 1 == 0, describe(path, key))
		else
			isArray = false
			assert(typeof(entryKey) == "string", describe(path, key))
		end
	end

	if isArray then
		local entries: Array = {}
		for index = 1, count do
			entries[index] = normalize(raw[index], path, key .. "[" .. index .. "]")
		end
		return entries
	end

	local fields: Table = {}
	for entryKey, entryValue in pairs(raw) do
		assert(typeof(entryKey) == "string", describe(path, key))
		fields[entryKey] = normalize(entryValue, path, key .. "." .. entryKey)
	end
	return fields
end

--# Functions

local Toml = {}

function Toml.Read(path: string): Table
	-- Serde returns external data. Normalize it before it crosses this module's
	-- boundary so every consumer receives only the recursive TOML value model.
	local decoded: unknown = Serde.decode("toml", Runtime.ReadFile(path))
	local document = normalize(decoded, path, "root")
	assert(typeof(document) == "table", path .. ": TOML document must be a table")
	return document :: Table
end

return Toml
