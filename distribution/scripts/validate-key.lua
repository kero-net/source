#!/usr/bin/env lua
-- Validates an optional local or CI GPG signing key without importing secrets.
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/distribution/scripts/validate%-key%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path
local command = require("lib.command")
local settings = require("distribution.lib.settings")

local environment = settings.read(root).verification.gpg_key_environment
local fingerprint = os.getenv(environment)
if not fingerprint or fingerprint == "" then
  io.stdout:write("[skip] No ", environment, " configured; release artifacts remain unsigned.\n")
  os.exit(0)
end
fingerprint = fingerprint:gsub("^0x", ""):upper()
if not fingerprint:match("^[0-9A-F]+$") then
  io.stderr:write(environment, " must be a hexadecimal OpenPGP fingerprint.\n")
  os.exit(1)
end
local output, message = command.capture(root, "gpg --batch --with-colons --list-secret-keys " .. command.quote(fingerprint))
if not output then io.stderr:write("Cannot inspect configured signing key: ", message, "\n"); os.exit(1) end
local discovered = output:match("\nfpr:::::::::([0-9A-F]+):") or output:match("^fpr:::::::::([0-9A-F]+):")
if discovered ~= fingerprint then
  io.stderr:write("Configured ", environment, " does not match an available secret signing key.\n")
  os.exit(1)
end
io.stdout:write("[ok] GPG signing key ", fingerprint, " is available.\n")
