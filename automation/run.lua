#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local automation = script:match("^(.*)/run%.lua$") or "automation"
local root = automation:match("^(.*)/automation$") or "."
package.path = automation .. "/?.lua;" .. package.path
local tasks = {
  documentation = require("tasks.documentation"), localization = require("tasks.localization"),
  publication = require("tasks.publication"), release = require("tasks.release"),
  repository = require("tasks.repository"), validation = require("tasks.validation"),
}
local name = arg[1]
if not name or not tasks[name] then
  io.stderr:write("usage: lua automation/run.lua {documentation|localization|publication|release|repository|validation}\n")
  os.exit(2)
end
local ok, message = tasks[name].run({ root = root, automation = automation, args = arg })
if not ok then io.stderr:write((message or (name .. " failed")) .. "\n"); os.exit(1) end
