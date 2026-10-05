-- .github/scripts/distribution-signing-key.luau
--!strict

local SigningKey = require("../../distribution/lib/verification/signing-key")
SigningKey.Verify(".")
