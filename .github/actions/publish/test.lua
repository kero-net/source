package.path = "./?.lua;./.github/actions/?.lua;" .. package.path

local policy = require("publish.policy")

local function read(path)
  local file = assert(io.open(path, "r"))
  local value = file:read("*a")
  file:close()
  return value
end

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
assert(not policy.publication_matches(
  publication, "canary", "2026.08.2-hotfix", "0123456789012345678901234567890123456789"
))
assert(policy.release_identity_matches(publication, "canary", "2026.08.1-regular"))
assert(not policy.release_identity_matches(publication, "stable", "2026.08.1-regular"))

local workflow = read(".github/workflows/release.yml")
assert(workflow:match("actions/create%-github%-app%-token@bcd2ba49218906704ab6c1aa796996da409d3eb1"))
assert(workflow:match("client%-id: %${{ vars%.KERO_RELEASE_APP_CLIENT_ID }}"))
assert(workflow:match("private%-key: %${{ secrets%.KERO_RELEASE_APP_PRIVATE_KEY }}"))
assert(workflow:match("owner: kero%-net"))
assert(workflow:match("repositories: kero"))
assert(not workflow:match("PERSONAL_"))
assert(not workflow:match("RELEASE_SIGNING_"))

local publisher = read(".github/actions/publish/main.lua")
assert(publisher:match("gh release delete"))
assert(not publisher:match("tar %-czf"))
assert(not publisher:match('command%.quote%(archive%)'))

local missing = io.popen(
  "env -u KERO_RELEASE_TOKEN -u GH_TOKEN lua5.4 .github/actions/publish/main.lua canary "
    .. "2026.08.1-regular 0123456789012345678901234567890123456789 . 2>&1"
)
local missing_output = missing:read("*a")
local missing_ok = missing:close()
assert(not missing_ok)
assert(missing_output:match("KERO_RELEASE_TOKEN is required"))
