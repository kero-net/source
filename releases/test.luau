--!nonstrict
local runtime = require("../.github/actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local releases = require("./validate")

assert(releases.valid_id("2026.09.1-regular"))
assert(releases.valid_id("2026.09.12-hotfix"))
assert(releases.valid_id("2026.12.2-security"))
assert(not releases.valid_id("v2026.09.1-regular"))
assert(not releases.valid_id("2026.13.1-hotfix"))
assert(not releases.valid_id("2026.09.0-regular"))
assert(releases.validate(".", "2026.08.1-regular"))
assert(releases.validate_sequence("2026.08.1-regular", ""))
assert(not releases.validate_sequence("2026.08.1-regular", "v2026.08.1-regular"))
