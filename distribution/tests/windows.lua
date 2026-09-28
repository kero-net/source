local target = assert(arg[1], "target is required")
local stage = assert(os.getenv("KERO_PACKAGE_STAGE_DIR"), "KERO_PACKAGE_STAGE_DIR is required")
local artifact = assert(os.getenv("KERO_PACKAGE_ARTIFACT"), "KERO_PACKAGE_ARTIFACT is required")

local function bytes(path)
  local file = assert(io.open(path, "rb"), "missing " .. path)
  local value = file:read("*a")
  file:close()
  return value
end

local function machine(path)
  local value = bytes(path)
  assert(value:sub(1, 2) == "MZ", path .. " is not a PE executable")
  local offset = string.unpack("<I4", value, 0x3d)
  assert(value:sub(offset + 1, offset + 4) == "PE\0\0", path .. " has no PE signature")
  return string.unpack("<I2", value, offset + 5)
end

local expected = target == "windows-arm64" and 0xaa64 or 0x8664
assert(machine(artifact) == expected, artifact .. " has the wrong machine type")
local listing = assert(io.popen('dir /b /s "' .. stage:gsub('"', '""') .. '\\*.dll"')):read("*a")
for path in listing:gmatch("[^\r\n]+") do
  assert(machine(path) == expected, path .. " has the wrong machine type")
end

if arg[2] == "--runtime" then
  local bootstrap = artifact
  local bootstrapCommand = "powershell.exe -NoProfile -NonInteractive -Command \"$p=Start-Process -FilePath '"
    .. bootstrap:gsub("'", "''")
    .. "' -ArgumentList '--verify-payload' -PassThru; if(-not $p.WaitForExit(15000)){Stop-Process -Id $p.Id -Force; exit 1}; exit $p.ExitCode\""
  assert(os.execute(bootstrapCommand) == true, "bootstrap payload verification failed")
  local installer = stage .. "\\bin\\kero.exe"
  local command = "powershell.exe -NoProfile -NonInteractive -Command \"$p=Start-Process -FilePath '"
    .. installer:gsub("'", "''")
    .. "' -PassThru; Start-Sleep -Seconds 3; if($p.HasExited){exit 1}; Stop-Process -Id $p.Id -Force -ErrorAction Stop; exit 0\""
  assert(os.execute(command) == true, "installer role did not remain running")
end
