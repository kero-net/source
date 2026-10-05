-- distribution/lib/verification/signing-key.luau
--!strict

--# Modules

local Runtime = require("../Runtime")
local Settings = require("../configuration/settings")

--# Functions

local SigningKey = {}

function SigningKey.Verify(root: string): boolean
	local environment = Settings.Read(root).verification.gpgKeyEnvironment
	local configured = Runtime.Environment(environment)

	if configured == nil or configured == "" then
		Runtime.Log("[skip] No " .. environment .. " configured; release artifacts remain unsigned.")
		return false
	end

	local fingerprint = configured:gsub("^0x", ""):upper()
	assert(fingerprint:match("^[0-9A-F]+$") ~= nil, environment .. " must be a hexadecimal OpenPGP fingerprint.")

	local output, message = Runtime.Capture(root, "gpg --batch --with-colons --list-secret-keys " .. Runtime.Quote(fingerprint))
	assert(output, "Cannot inspect configured signing key: " .. tostring(message))

	local discovered = output:match("\nfpr:::::::::([0-9A-F]+):") or output:match("^fpr:::::::::([0-9A-F]+):")
	assert(discovered == fingerprint, "Configured " .. environment .. " does not match an available secret signing key.")

	Runtime.Log("[ok] GPG signing key " .. fingerprint .. " is available.")
	return true
end

return SigningKey
