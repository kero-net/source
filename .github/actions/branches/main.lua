#!/usr/bin/env lua
local branch = arg[1] or os.getenv("GITHUB_REF_NAME") or ""
local kind = arg[2] or "any"

local source = branch == "source"
local channel = branch == "canary" or branch == "beta" or branch == "stable"

local valid = (kind == "any" and (source or channel))
  or (kind == "source" and source)
  or (kind == "channel" and channel)

if not valid then
  io.stderr:write("unsupported KERO " .. kind .. " branch: " .. tostring(branch) .. "\n")
  os.exit(1)
end
