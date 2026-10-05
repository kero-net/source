-- .github/scripts/distribution-package.luau
--!strict

local process = require("@lune/process")
local Package = require("../../distribution/lib/build/package")
Package.Run(".", assert(process.env.KERO_BUILD_TARGET))
