local policy = {}

policy.target = "kero-net/kero"

function policy.tag(channel, version)
  if channel == "stable" then return "v" .. version end
  return "v" .. version .. "-" .. channel
end

function policy.publication_matches(value, channel, version, source_commit)
  if type(value) ~= "string" then return false end
  return value:match('channel%s*=%s*"([^"]+)"') == channel
    and value:match('version%s*=%s*"([^"]+)"') == version
    and value:match('source%s*=%s*"([^"]+)"') == source_commit
end

function policy.release_identity_matches(value, channel, version)
  if type(value) ~= "string" then return false end
  return value:match('channel%s*=%s*"([^"]+)"') == channel
    and value:match('version%s*=%s*"([^"]+)"') == version
end

return policy
