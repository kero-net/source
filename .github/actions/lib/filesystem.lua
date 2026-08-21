local filesystem = {}
function filesystem.exists(path) local f = io.open(path, "rb"); if f then f:close(); return true end; return false end
function filesystem.read(path)
  local f, e = io.open(path, "rb"); if not f then return nil, e end
  local value = f:read("*a"); f:close(); return value
end
function filesystem.write(path, value)
  local f, e = io.open(path, "wb"); if not f then return false, e end
  f:write(value); f:close(); return true
end
return filesystem
