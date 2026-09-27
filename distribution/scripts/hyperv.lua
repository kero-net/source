#!/usr/bin/env lua
-- Runs one package stage through PowerShell Direct in an explicitly configured Hyper-V guest.
local target = assert(arg[1], "usage: lua distribution/scripts/hyperv.lua TARGET")
assert(target == "windows-arm64" or target == "windows-x64", "Hyper-V adapter supports Windows targets only")
package.path = "./?.lua;" .. package.path
local settings = require("distribution.lib.settings")
local adapter = settings.adapter(".", "hyperv") or {}
local guest = adapter.guest
local source = adapter.source
if not guest or not source then
  io.stdout:write("[skip] ", target, " has no Hyper-V guest/source mapping in distribution.local.kst.\n")
  os.exit(2)
end
local command = "powershell.exe -NoProfile -NonInteractive -Command \"$vm=Get-VM -Name '" .. guest:gsub("'", "''")
  .. "' -ErrorAction Stop; if($vm.State -ne 'Running'){throw 'Hyper-V guest is not running'}; "
  .. "Invoke-Command -VMName '" .. guest:gsub("'", "''") .. "' -ScriptBlock { param($p,$t); Set-Location $p; lua distribution/scripts/local-run.lua build $t } -ArgumentList '"
  .. source:gsub("'", "''") .. "','" .. target .. "'\""
assert(os.execute(command) == true, "Hyper-V guest build failed")
