-- .github/scripts/distribution-matrix.luau
--!strict

local Matrix = require("../../distribution/lib/hosted/create-github-build-matrix")
print(Matrix.Create("."))
