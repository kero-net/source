-- .github/scripts/distribution-signing-key.luau
--!strict

local Compatibility = require("../../distribution/lib/Compatibility")
Compatibility.Install()

local SigningKey = require("../../distribution/lib/verification/signing-key")
SigningKey.Verify(".")
