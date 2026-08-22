package.path = "./?.lua;" .. package.path
local releases = require("releases.validate")

assert(releases.valid_id("2026.08.1-regular"))
assert(releases.valid_id("2026.09.1-regular"))
assert(releases.valid_id("2026.12.12-security"))
assert(not releases.valid_id("v2026.08.1-regular"))
assert(not releases.valid_id("2026.13.1-hotfix"))
assert(releases.validate(".", "2026.08.1-regular"))

local authored = {}
local records = assert(io.popen(
  "find releases/records -maxdepth 1 -type f -name '*.md' -printf '%f\\n'"
))
for filename in records:lines() do
  authored[#authored + 1] = filename:gsub("%.md$", "")
end
assert(records:close())
assert(releases.validate_workflow_choices(".", authored))

local ok, message = releases.validate_workflow_choices(".", { "2026.08.2-hotfix" })
assert(not ok and message:match("missing authored release choice"))

assert(releases.validate_sequence("2026.08.1-regular", ""))
assert(releases.validate_sequence("2026.08.2-hotfix", "v2026.08.1-regular"))
assert(releases.validate_sequence("2026.09.1-regular", "v2026.08.9-hotfix"))
assert(not releases.validate_sequence("2026.08.1-regular", "v2026.08.1-regular"))
assert(not releases.validate_sequence("2026.07.1-regular", "v2026.08.1-regular"))
