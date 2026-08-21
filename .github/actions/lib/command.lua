local command = {}
function command.quote(value) return "'" .. tostring(value):gsub("'", "'\\''") .. "'" end
function command.run(root, program)
  io.stdout:write("+ ", program, "\n"); io.stdout:flush()
  local ok, reason, code = os.execute("cd " .. command.quote(root) .. " && " .. program)
  if ok then return true end
  return false, string.format("command failed (%s %s): %s", reason or "exit", code or 1, program)
end
return command
