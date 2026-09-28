#!/usr/bin/env lua
-- Emits the GitHub Actions package matrix from distribution/builds.json.
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/distribution/scripts/hosted%-matrix%.lua$") or "."
package.path = root .. "/?.lua;" .. package.path
local settings = require("distribution.lib.settings")

local function quote(value)
  return '"' .. value:gsub('\\', '\\\\'):gsub('"', '\\"') .. '"'
end
local include = {}
for _, item in pairs(settings.read(root).targets) do
  if item.enabled and item.hosted then
    local hosted = item.hosted
    local fields = {
      target = item.name, os = hosted.runner, lua = hosted.lua, qt_version = hosted.qtVersion,
      qt_host = hosted.qtHost, qt_arch = hosted.qtArch, msvc_arch = hosted.msvcArchitecture,
    }
    local encoded = {}
    for key, value in pairs(fields) do if value then encoded[#encoded + 1] = quote(key) .. ":" .. quote(value) end end
    table.sort(encoded); include[#include + 1] = "{" .. table.concat(encoded, ",") .. "}"
  end
end
table.sort(include)
io.write('{"include":[', table.concat(include, ","), "]}\n")
