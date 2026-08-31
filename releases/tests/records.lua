package.path = "./?.lua;" .. package.path
local releases = require("releases.validate")

assert(releases.validate(".", "2026.08.1-regular"))
local ok, message = releases.validate(".", "2026.08.2-hotfix")
assert(not ok and message:match("missing authored release record"))
