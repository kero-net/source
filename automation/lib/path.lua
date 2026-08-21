local path = {}
function path.join(...) return table.concat({ ... }, "/"):gsub("/+", "/") end
function path.is_safe_relative(value)
  return value ~= "" and value:sub(1, 1) ~= "/" and not value:match("^%.%./") and not value:match("/%.%./")
end
return path
