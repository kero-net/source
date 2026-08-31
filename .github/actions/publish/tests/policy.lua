package.path = "./?.lua;./.github/actions/?.lua;" .. package.path
local policy = require("publish.policy")

assert(policy.target == "kero-net/kero")
assert(policy.tag("canary", "2026.08.1-regular") == "v2026.08.1-regular-canary")
assert(policy.tag("beta", "2026.08.1-regular") == "v2026.08.1-regular-beta")
assert(policy.tag("stable", "2026.08.1-regular") == "v2026.08.1-regular")

local publication = [[
channel = "canary"
version = "2026.08.1-regular"
source = "0123456789012345678901234567890123456789"
]]
assert(policy.publication_matches(
  publication, "canary", "2026.08.1-regular", "0123456789012345678901234567890123456789"
))
assert(not policy.publication_matches(
  publication, "stable", "2026.08.1-regular", "0123456789012345678901234567890123456789"
))
assert(policy.release_identity_matches(publication, "canary", "2026.08.1-regular"))
assert(not policy.release_identity_matches(publication, "stable", "2026.08.1-regular"))
