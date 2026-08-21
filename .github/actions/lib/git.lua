local command = require("lib.command")
local git = {}
function git.verify(root) return command.run(root, "git diff --check") end
return git
