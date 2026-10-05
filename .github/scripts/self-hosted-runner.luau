--!nonstrict

local runtime = require("../actions/lib/legacy-runtime")
local io = runtime.io
local process = require("@lune/process")
local Serde = require("@lune/serde")
local Toml = require("../../distribution/lib/configuration/toml")

local function contains(values, expected)
	for _, value in ipairs(values or {}) do
		if value == expected then
			return true
		end
	end
	return false
end

local function split(value)
	local values = {}
	for entry in (value or ""):gmatch("[^,]+") do
		table.insert(values, entry)
	end
	return values
end

local workflow = assert(process.env.KERO_SELF_HOSTED_WORKFLOW, "KERO_SELF_HOSTED_WORKFLOW is required")
local target = assert(process.env.KERO_SELF_HOSTED_TARGET, "KERO_SELF_HOSTED_TARGET is required")
local triggerActor = process.env.KERO_TRIGGER_ACTOR or ""
local pullRequestAuthors = split(process.env.KERO_PR_AUTHORS or "")

local policy = Toml.Read(".github/self-hosted-runners.toml")
local runners = policy.runner or {}
local selected = nil

for _, runner in ipairs(runners) do
	assert(type(runner.id) == "string" and runner.id ~= "", "self-hosted runner id is required")
	assert(type(runner.users) == "table" and #runner.users > 0, runner.id .. ": users are required")
	assert(type(runner.workflows) == "table" and #runner.workflows > 0, runner.id .. ": workflows are required")
	assert(type(runner.targets) == "table" and #runner.targets > 0, runner.id .. ": targets are required")
	assert(type(runner.labels) == "table" and #runner.labels > 0, runner.id .. ": labels are required")

	if contains(runner.workflows, workflow) and contains(runner.targets, target) then
		assert(selected == nil, "multiple self-hosted runner policies match " .. workflow .. "/" .. target)
		selected = runner
	end
end

local allowed = false
if selected then
	allowed = contains(selected.users, triggerActor)
	if not allowed then
		for _, author in ipairs(pullRequestAuthors) do
			if contains(selected.users, author) then
				allowed = true
				break
			end
		end
	end
end

local labels = if selected then selected.labels else {}
local runnerId = if selected then selected.id else ""

local output = process.env.GITHUB_OUTPUT
if output then
	local stream = assert(io.open(output, "a"))
	stream:write("allowed=", if allowed then "true" else "false", "\n")
	stream:write("runner_id=", runnerId, "\n")
	stream:write("labels=", Serde.encode("json", labels), "\n")
	stream:close()
end

print(
	"Self-hosted runner policy:",
	workflow .. "/" .. target,
	"trigger=" .. triggerActor,
	"pr-authors=" .. table.concat(pullRequestAuthors, ","),
	"runner=" .. runnerId,
	"allowed=" .. tostring(allowed)
)
