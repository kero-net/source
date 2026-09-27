#!/usr/bin/env lua
-- Dispatches an explicitly configured contributor-owned VM adapter.
local adapter, target, stage = assert(arg[1], "adapter required"), assert(arg[2], "target required"), assert(arg[3], "stage required")
assert(stage == "test", "adapters support only runtime testing")
package.path = "./?.lua;" .. package.path
local settings = require("distribution.lib.settings")
assert(settings.adapter(".", adapter), "adapter is not configured in distribution.local.kst; copy distribution.local.kst.example")
if adapter == "hyperv" then
  assert(os.execute("lua distribution/scripts/hyperv.lua " .. target) == true, "Hyper-V adapter failed")
elseif adapter == "qemu" then
  error("QEMU adapter is configured but guest launch is coming soon")
else error("unknown adapter: " .. adapter) end
