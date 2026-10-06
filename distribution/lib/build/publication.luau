-- distribution/lib/build/publication.luau
--!strict

--# Modules

local Runtime = require("../Runtime")
local Settings = require("../configuration/settings")

--# Types

export type State = "pending" | "preparing" | "prepared" | "building" | "complete" | "failed" | "blocked"

export type Publisher = {
	root: string,
	revision: string,
	targets: { Settings.Target },
	states: { [string]: State },
	digests: { [string]: string },
	details: { [string]: string },
	runFailures: { string },
	finalized: boolean,
}

--# Helpers

local function writeIndexes(publisher: Publisher)
	local checksums: { string } = {}
	local completed: { string } = {}
	local unfinished: { string } = {}

	for _, target in publisher.targets do
		local digest = publisher.digests[target.name]
		if digest then
			table.insert(checksums, digest .. "  distributions/" .. target.finalArtifact)
			table.insert(completed, "- `artifacts/distributions/" .. target.finalArtifact .. "`")
		else
			local detail = publisher.details[target.name]
			local suffix = if detail then " — " .. detail else ""
			table.insert(unfinished, "- `" .. target.name .. "`: " .. publisher.states[target.name] .. suffix)
		end
	end

	table.sort(checksums)
	table.sort(completed)
	table.sort(unfinished)
	local complete = publisher.finalized and #completed == #publisher.targets and #publisher.runFailures == 0
	local lines = {
		"# KERO local build",
		"",
		"Status: **" .. (if complete then "complete" else "incomplete") .. "**",
		"",
		"Unsigned locally validated artifacts for revision `" .. publisher.revision .. "`.",
		"",
		"## Completed artifacts",
		"",
	}

	if #completed == 0 then
		table.insert(lines, "- None yet.")
	else
		for _, line in completed do
			table.insert(lines, line)
		end
	end

	if #unfinished > 0 then
		table.insert(lines, "")
		table.insert(lines, "## Remaining targets")
		table.insert(lines, "")
		for _, line in unfinished do
			table.insert(lines, line)
		end
	end

	if #publisher.runFailures > 0 then
		table.insert(lines, "")
		table.insert(lines, "## Run failures")
		table.insert(lines, "")
		for _, failure in publisher.runFailures do
			table.insert(lines, "- " .. failure)
		end
	end

	Runtime.AtomicWrite(publisher.root .. "/.heap/release/artifacts/SHA256SUMS", table.concat(checksums, "\n") .. (if #checksums > 0 then "\n" else ""))
	Runtime.AtomicWrite(publisher.root .. "/.heap/release/RELEASE-MESSAGE.md", table.concat(lines, "\n") .. "\n")
end

--# Functions

local Publication = {}

function Publication.Create(root: string, targets: { Settings.Target }, resume: boolean?): Publisher
	local revision, message = Runtime.Capture(root, "git rev-parse HEAD")
	assert(revision, "cannot determine source revision: " .. tostring(message))
	local states: { [string]: State } = {}
	local digests: { [string]: string } = {}
	for _, target in targets do
		states[target.name] = "pending"
	end
	local index = root .. "/.heap/release/artifacts/SHA256SUMS"
	if resume and Runtime.IsFile(index) then
		local recorded: { [string]: string } = {}
		for line in Runtime.ReadFile(index):gmatch("[^\r\n]+") do
			local digest, filename = line:match("^([0-9a-fA-F]+)  distributions/(.+)$")
			if digest and filename then recorded[filename] = digest:lower() end
		end
		for _, target in targets do
			local expected = recorded[target.finalArtifact]
			local artifact = root .. "/.heap/release/artifacts/distributions/" .. target.finalArtifact
			if expected and #expected == 64 and Runtime.IsFile(artifact) then
				local output = Runtime.Capture(root, "sha256sum " .. Runtime.Quote(artifact))
				local actual = if output then output:match("^([0-9a-fA-F]+)") else nil
				if actual and actual:lower() == expected then
					states[target.name] = "complete"
					digests[target.name] = expected
				end
			end
		end
	end

	Runtime.WriteDirectory(root .. "/.heap/release")
	Runtime.WriteDirectory(root .. "/.heap/release/artifacts")
	Runtime.WriteDirectory(root .. "/.heap/release/artifacts/distributions")
	local publisher: Publisher = {
		root = root,
		revision = revision,
		targets = targets,
		states = states,
		digests = digests,
		details = {},
		runFailures = {},
		finalized = false,
	}
	writeIndexes(publisher)
	return publisher
end

function Publication.SetState(publisher: Publisher, target: string, state: State, detail: string?)
	publisher.states[target] = state
	if detail then
		publisher.details[target] = detail:match("^[^\r\n]+") or detail
	end
	writeIndexes(publisher)
end

function Publication.Publish(publisher: Publisher, target: Settings.Target)
	local source = publisher.root .. "/.heap/cache/build/releases/" .. target.name .. "/" .. target.artifact
	local destination = publisher.root .. "/.heap/release/artifacts/distributions/" .. target.finalArtifact
	local temporary = destination .. ".partial"
	Runtime.Copy(source, temporary)
	Runtime.Move(temporary, destination)

	local checksum, message = Runtime.Capture(publisher.root, "sha256sum " .. Runtime.Quote(destination))
	assert(checksum, "cannot checksum " .. target.finalArtifact .. ": " .. tostring(message))
	local digest = assert(checksum:match("^([0-9a-fA-F]+)"), "invalid checksum output for " .. target.finalArtifact):lower()
	publisher.digests[target.name] = digest
	publisher.states[target.name] = "complete"
	publisher.details[target.name] = nil
	writeIndexes(publisher)
end

function Publication.IsComplete(publisher: Publisher): boolean
	for _, target in publisher.targets do
		if publisher.states[target.name] ~= "complete" then
			return false
		end
	end
	return #publisher.runFailures == 0
end

function Publication.FailRun(publisher: Publisher, message: string)
	table.insert(publisher.runFailures, message:match("^[^\r\n]+") or message)
	writeIndexes(publisher)
end

function Publication.Finalize(publisher: Publisher)
	publisher.finalized = true
	writeIndexes(publisher)
end

return Publication
