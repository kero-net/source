local result = {}
function result.ok(value) return true, value end
function result.error(message) return false, message end
return result
