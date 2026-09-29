-- .github/scripts/distribution-portable.luau
--!strict

local Compatibility = require("../../distribution/lib/Compatibility")
Compatibility.Install()

local Complete = require("../../distribution/lib/build/complete")
Complete.Portable(".")
