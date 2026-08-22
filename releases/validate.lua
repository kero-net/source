local releases = {}

function releases.parse_id(value)
  if type(value) ~= "string" then return nil end
  local year, month, sequence, kind = value:match("^(%d%d%d%d)%.(%d%d)%.([1-9]%d*)%-(%a+)$")
  if not year then return nil end
  month = tonumber(month)
  sequence = tonumber(sequence)
  if month < 1 or month > 12 then return nil end
  if kind ~= "regular" and kind ~= "hotfix" and kind ~= "security" then return nil end
  return {
    year = tonumber(year),
    month = month,
    sequence = sequence,
    kind = kind,
    period = tonumber(year .. string.format("%02d", month)),
  }
end

function releases.valid_id(value)
  return releases.parse_id(value) ~= nil
end

function releases.validate(root, version)
  if not releases.valid_id(version) then
    return false, "invalid release ID: " .. tostring(version)
  end
  local file = io.open(root .. "/releases/records/" .. version .. ".md", "r")
  if not file then return false, "missing authored release record: " .. version end
  file:close()
  return true
end

local function read(path)
  local file = io.open(path, "r")
  if not file then return nil end
  local value = file:read("*a")
  file:close()
  return value
end

function releases.validate_workflow_choices(root, record_ids)
  local workflow = read(root .. "/.github/workflows/release.yml")
  if not workflow then return false, "missing release workflow" end

  local version = workflow:match("\n      version:\n(.-)\n\npermissions:")
  if not version or not version:match("\n        type: choice\n") then
    return false, "release workflow version input must use type: choice"
  end

  local options = version:match("\n        options:\n(.*)")
  if not options then return false, "release workflow version input must define options" end

  local actual = {}
  local count = 0
  for choice in options:gmatch("          %- ([^\r\n]+)") do
    if actual[choice] then return false, "duplicate release workflow choice: " .. choice end
    actual[choice] = true
    count = count + 1
  end
  if count == 0 then return false, "release workflow version choices must not be empty" end

  local expected = {}
  for _, release_id in ipairs(record_ids) do expected[release_id] = true end
  for release_id in pairs(expected) do
    if not actual[release_id] then
      return false, "release workflow is missing authored release choice: " .. release_id
    end
  end
  for release_id in pairs(actual) do
    if not expected[release_id] then
      return false, "release workflow has unauthored release choice: " .. release_id
    end
  end
  if count ~= #record_ids then return false, "release workflow choices do not match authored releases" end
  return true
end

function releases.validate_sequence(version, tags)
  local requested = releases.parse_id(version)
  if not requested then return false, "invalid release ID: " .. tostring(version) end

  local latest_period = 0
  local latest_sequence = 0
  for tag in (tags or ""):gmatch("[^\r\n]+") do
    local existing = releases.parse_id(tag:gsub("^v", ""))
    if existing and (existing.period > latest_period
        or (existing.period == latest_period and existing.sequence > latest_sequence)) then
      latest_period = existing.period
      latest_sequence = existing.sequence
    end
  end

  if requested.period < latest_period then
    return false, string.format(
      "release period %04d.%02d precedes the current release period %04d.%02d",
      requested.year, requested.month, math.floor(latest_period / 100), latest_period % 100
    )
  end

  local expected = requested.period == latest_period and (latest_sequence + 1) or 1
  if requested.sequence ~= expected then
    return false, string.format(
      "release sequence must be %d for %04d.%02d; got %d",
      expected, requested.year, requested.month, requested.sequence
    )
  end
  return true
end

return releases
