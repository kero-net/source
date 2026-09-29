-- .github/scripts/distribution-matrix.luau
--!strict

local Compatibility = require("../../distribution/lib/Compatibility")
Compatibility.Install()

local Matrix = require("../../distribution/lib/hosted/create-github-build-matrix")
print(Matrix.Create("."))
