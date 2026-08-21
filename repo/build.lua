local command = require("lib.command")
local publication = {}
function publication.run(context)
  local channel = context.args[2] or "canary"
  if channel ~= "canary" and channel ~= "beta" and channel ~= "stable" then return false, "publication channel must be canary, beta, or stable" end
  local destination = "../.heap/source/staging/publication/" .. channel
  return command.run(context.root, "mkdir -p " .. destination .. " && cp -R code docs " .. destination .. "/")
end
return publication
