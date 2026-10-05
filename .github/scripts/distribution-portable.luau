-- .github/scripts/distribution-portable.luau
--!strict

local Contracts = require("../../distribution/lib/verification/contracts")
Contracts.Verify(".")
