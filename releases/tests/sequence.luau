--!nonstrict
local runtime = require("../../.github/actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local releases = require("../validate")

assert(releases.validate_sequence("2026.08.1-regular", ""))
assert(releases.validate_sequence("2026.08.2-hotfix", "v2026.08.1-regular"))
assert(releases.validate_sequence("2026.09.1-regular", "v2026.08.9-hotfix"))
assert(not releases.validate_sequence("2026.08.1-regular", "v2026.08.1-regular"))
assert(not releases.validate_sequence("2026.07.1-regular", "v2026.08.1-regular"))
