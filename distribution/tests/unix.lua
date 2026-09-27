local target = assert(arg[1], "target is required")
local artifact = assert(os.getenv("KERO_PACKAGE_ARTIFACT"), "KERO_PACKAGE_ARTIFACT is required")
local stage = assert(os.getenv("KERO_PACKAGE_STAGE_DIR"), "KERO_PACKAGE_STAGE_DIR is required")
if artifact:find("%*") then
  local directory, pattern = assert(artifact:match("^(.*)/([^/]+)$"))
  local pipe = assert(io.popen("find " .. string.format("'%s'", directory:gsub("'", "'\\''"))
    .. " -maxdepth 1 -type f -name " .. string.format("'%s'", pattern:gsub("'", "'\\''")) .. " -print -quit"))
  artifact = pipe:read("*l") or ""
  pipe:close()
end
assert(artifact ~= "", "package artifact was not found")
local checksum = artifact .. ".sha256"
local file = assert(io.open(artifact, "rb"), "missing package artifact: " .. artifact)
file:close()
local digest = assert(io.open(checksum, "rb"), "missing package checksum: " .. checksum)
digest:close()
if target == "linux-x64" then
  assert(os.execute("test -s " .. string.format("'%s'", artifact:gsub("'", "'\\''"))) == true, "Linux package is empty")
end
if arg[2] == "--runtime" then
  local executable = target == "linux-x64" and (stage .. "/usr/bin/kero-install")
    or (stage .. "/kero-install.app/Contents/MacOS/kero-install")
  local quoted = string.format("'%s'", executable:gsub("'", "'\\''"))
  local smoke = "QT_QPA_PLATFORM=offscreen " .. quoted
    .. " >/dev/null 2>&1 & pid=$!; sleep 3; kill -0 $pid 2>/dev/null || exit 1; kill $pid; wait $pid || true"
  assert(os.execute("sh -c " .. string.format("'%s'", smoke:gsub("'", "'\\''"))) == true, "installer role did not remain running")
end
