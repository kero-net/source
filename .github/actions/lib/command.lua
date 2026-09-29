local command = {}

local is_windows = package.config:sub(1, 1) == "\\"

local function posix_shell(command_line)
  if not is_windows then return command_line end
  local executable = os.getenv("KERO_POSIX_SHELL") or "C:\\Program Files\\Git\\bin\\sh.exe"
  local handle = io.open(executable, "rb")
  if not handle then
    return nil, "Windows build helpers require Git for Windows sh.exe; set KERO_POSIX_SHELL to override it"
  end
  handle:close()
  return '""' .. executable .. '" -lc "' .. command_line:gsub('"', '\\"') .. '""'
end

local function windows_native(root, program, capture)
  local escaped_root = tostring(root):gsub('"', '""')
  local redirect = capture and " 2>&1" or ""
  return 'cmd.exe /d /s /c "cd /d \"' .. escaped_root .. '\" && set \"CARGO_TARGET_DIR=.heap/build/cargo/target\" && ' .. program .. redirect .. '"'
end

local function invocation(root, program, capture)
  local command_line = "cd " .. command.quote(root) .. " && " .. program
  if is_windows and program:match("^cargo ") then
    return windows_native(root, program, capture)
  end
  if not is_windows and (program:match("^cargo ") or program:match("^cargo%.exe ")) then
    command_line = "cd " .. command.quote(root) .. " && CARGO_TARGET_DIR=.heap/build/cargo/target " .. program
  end
  if capture then command_line = command_line .. " 2>&1" end
  return posix_shell(command_line)
end

function command.quote(value)
  return "'" .. tostring(value):gsub("'", "'\\''") .. "'"
end

local function succeeded(ok, _, code)
  return ok == true or ok == 0 or code == 0
end

function command.run(root, program, quiet)
  if not quiet then
    io.stdout:write("+ ", program, "\n")
    io.stdout:flush()
  end
  local invocation, shell_error = invocation(root, program, false)
  if not invocation then return false, shell_error end
  local ok, reason, code = os.execute(invocation)
  if succeeded(ok, reason, code) then return true end
  return false, string.format("command failed (%s %s): %s", reason or "exit", code or ok or 1, program)
end

function command.run_secret(root, program, label)
  local invocation, shell_error = invocation(root, program, false)
  if not invocation then return false, shell_error end
  local ok, reason, code = os.execute(invocation)
  if succeeded(ok, reason, code) then return true end
  return false, string.format("%s failed (%s %s)", label or "secret command", reason or "exit", code or ok or 1)
end

function command.capture(root, program)
  local invocation, shell_error = invocation(root, program, true)
  if not invocation then return nil, shell_error end
  local pipe, message = io.popen(invocation, "r")
  if not pipe then return nil, message end
  local output = pipe:read("*a")
  local ok, reason, code = pipe:close()
  if succeeded(ok, reason, code) then
    return (output:gsub("%s+$", ""))
  end
  return nil, string.format("command failed (%s %s): %s\n%s", reason or "exit", code or ok or 1, program, output)
end

return command
