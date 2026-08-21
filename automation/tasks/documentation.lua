local command = require("lib.command")
local documentation = {}
function documentation.run(context)
  return command.run(context.root, "mkdir -p ../.heap/source/preview/documentation && cp -R docs/. ../.heap/source/preview/documentation/")
end
return documentation
