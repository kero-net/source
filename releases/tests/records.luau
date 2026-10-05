--!nonstrict
local runtime = require("../../.github/actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local releases = require("../validate")

assert(releases.validate(".", "2026.08.1-regular"))
local ok, message = releases.validate(".", "2026.08.2-hotfix")
assert(not ok and message:match("missing authored release record"))
