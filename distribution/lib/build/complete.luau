-- distribution/lib/build/complete.luau
--!strict

local Task = require("@lune/task")
local Runtime = require("../Runtime")
local Settings = require("../configuration/settings")
local Artifact = require("../verification/artifact")
local Contracts = require("../verification/contracts")
local Publication = require("./publication")
local Target = require("./target")

type PreparationResult = {
	target: Settings.Target,
	prepared: Target.Prepared?,
	error: string?,
}

local function run(root: string, line: string, scope: string)
	local passed, message = Runtime.Run(root, line, true, scope)
	assert(passed, message)
end

local function executeJob(name: string, log: string, total: number, callback: () -> ())
	Runtime.Job(name, "run", "log: .heap/cache/logs/" .. log .. ".log", total)
	local succeeded, failure = pcall(callback)
	if not succeeded then
		Runtime.Job(name, "fail", "log: .heap/cache/logs/" .. log .. ".log")
		error(tostring(failure), 0)
	end
	Runtime.Job(name, "done")
end

local function enabledTargets(configuration: Settings.Configuration): { Settings.Target }
	local targets: { Settings.Target } = {}
	for _, target in pairs(configuration.targets) do
		if target.enabled then
			table.insert(targets, target)
		end
	end
	table.sort(targets, function(left, right)
		return left.name < right.name
	end)
	return targets
end

local function hostTarget(): string
	local architecture = Runtime.Architecture()
	if Runtime.IsWsl() or Runtime.OperatingSystem() == "windows" then
		return if architecture == "aarch64" then "windows-arm64" else "windows-x64"
	end
	if Runtime.OperatingSystem() == "linux" then
		return if architecture == "aarch64" then "linux-arm64" else "linux-x64"
	end
	return "macos-unavailable"
end

local function sourceChecks(root: string)
	local job = "source validation"
	if Runtime.Environment("KERO_SOURCE_CHECKED") == "1" then
		Runtime.Step(job, "Checking", "Windows-native source checks completed before WSL handoff")
		Runtime.StepComplete(job)
		return
	end
	local environment = if Runtime.IsWsl() then "KERO_CARGO=cargo.exe ACTIONLINT_BIN=actionlint.exe " else ""
	Runtime.Step(job, "Checking", "source and workflow contracts")
	run(root, environment .. "lune run .github/scripts/source-checks.luau local", "source-validation")
	Runtime.StepComplete(job)
end

local function portableChecks(root: string)
	local job = "portable workflow validation"
	Runtime.Step(job, "Checking", "distribution contracts")
	Contracts.Verify(root)
	Runtime.StepComplete(job)
	Runtime.Step(job, "Checking", "GitHub workflow replay completed")
	assert(Runtime.Environment("KERO_WORKFLOW_REPLAY") == "1", "run Kero: Validate Locally to execute required GitHub workflows before building")
	Runtime.StepComplete(job)
end

local function writeDependencyRecord(root: string)
	Runtime.WriteDirectory(root .. "/.heap/cache")
	Runtime.WriteFile(root .. "/.heap/cache/README.md", table.concat({
		"# KERO build cache",
		"",
		"Downloads are keyed by their manifest filenames and verified checksums.",
		"WSL extracts Qt kits and host tools into the ext4 workspace cache.",
		"KERO_CLEAN=1 removes both caches; the next run recreates them.",
		"",
	}, "\n"))
end

local function dependenciesReady(target: Settings.Target, publisher: Publication.Publisher): boolean
	for _, dependency in target.builder.dependsOn do
		local state = publisher.states[dependency]
		if state ~= "prepared" and state ~= "building" and state ~= "complete" then
			return false
		end
	end
	return true
end

local function dependencyFailed(target: Settings.Target, publisher: Publication.Publisher): boolean
	for _, dependency in target.builder.dependsOn do
		local state = publisher.states[dependency]
		if state == "failed" or state == "blocked" then
			return true
		end
	end
	return false
end

local function discardWorkingState(root: string)
	local job = "working-state cleanup"
	Runtime.Step(job, "Removing", "disposable compiler, CMake, and staging output")
	Runtime.RemoveDirectory(root .. "/.heap/cache/build")
	local wslRoot = Runtime.Environment("KERO_WSL_WORK_ROOT")
	if wslRoot then Runtime.RemoveDirectory(wslRoot) end
	Runtime.StepComplete(job)
	Runtime.Log("[ok] Removed disposable build state; keyed dependency caches remain.")
end

local function executeTargets(
	root: string,
	targets: { Settings.Target },
	configuration: Settings.Configuration,
	publisher: Publication.Publisher
)
	local results: { PreparationResult } = {}
	local prepared: { [string]: Target.Prepared } = {}
	local preparing = 0
	local terminalStates = 0
	local native = hostTarget()
	local nativeChecked = publisher.states[native] == "complete"
	for _, target in targets do
		if publisher.states[target.name] == "complete" then terminalStates += 1 end
	end

	local function beginPreparation(target: Settings.Target)
		Publication.SetState(publisher, target.name, "preparing")
		Runtime.Job("prepare " .. target.name, "run", "log: .heap/cache/logs/" .. target.name .. "-prepare.log", Target.PreparationSteps(target, configuration))
		preparing += 1
		Task.spawn(function()
			local succeeded, value = pcall(Target.Prepare, root, target.name, configuration)
			if succeeded then
				table.insert(results, { target = target, prepared = value :: Target.Prepared })
			else
				table.insert(results, { target = target, error = tostring(value) })
			end
			preparing -= 1
		end)
	end

	local function consumeResults()
		while #results > 0 do
			local result = assert(table.remove(results, 1))
			if result.error then
				Publication.SetState(publisher, result.target.name, "failed", result.error)
				Runtime.Job("prepare " .. result.target.name, "fail", "log: .heap/cache/logs/" .. result.target.name .. "-prepare.log")
				Runtime.Job("build and publish " .. result.target.name, "skip", "preparation failed")
				terminalStates += 1
			else
				prepared[result.target.name] = assert(result.prepared)
				Publication.SetState(publisher, result.target.name, "prepared")
				Runtime.Job("prepare " .. result.target.name, "done")
			end
		end
	end

	while terminalStates < #targets do
		consumeResults()
		local changed = false
		for _, target in targets do
			if publisher.states[target.name] == "pending" then
				if dependencyFailed(target, publisher) then
					Publication.SetState(publisher, target.name, "blocked", "preparation dependency failed")
					Runtime.Job("prepare " .. target.name, "fail", "dependency failed")
					Runtime.Job("build and publish " .. target.name, "skip", "dependency failed")
					terminalStates += 1
					changed = true
				elseif dependenciesReady(target, publisher) then
					beginPreparation(target)
					changed = true
				end
			end
		end

		local buildTarget: Settings.Target? = nil
		for _, target in targets do
			if publisher.states[target.name] == "prepared" then
				buildTarget = target
				break
			end
		end
		if buildTarget then
			local target = buildTarget
			Publication.SetState(publisher, target.name, "building")
			local buildSteps = if target.builder.kind == "wsl-linux" then 3 else 11
			Runtime.Job("build and publish " .. target.name, "run", "log: .heap/cache/logs/" .. target.name .. "-build.log", buildSteps)
			local succeeded, message = pcall(function()
				local job = "build and publish " .. target.name
				if target.builder.kind == "wsl-linux" then
					Runtime.Step(job, "Building", target.name)
					Target.Build(assert(prepared[target.name]))
					Runtime.StepComplete(job)
				else
					Target.Build(assert(prepared[target.name]))
				end

				Runtime.Step(job, "Checking", target.name .. " artifact")
				Artifact.Verify(root, target.name, false)
				Runtime.StepComplete(job)

				Runtime.Step(job, "Publishing", target.finalArtifact)
				Publication.Publish(publisher, target)
				Runtime.StepComplete(job)

				if target.name == native then
					Runtime.Status("Checking", target.name .. " runtime payload")
					Artifact.Verify(root, target.name, true)
					nativeChecked = true
				end
			end)
			if not succeeded then
				Publication.SetState(publisher, target.name, "failed", tostring(message))
				Runtime.Job("build and publish " .. target.name, "fail", "log: .heap/cache/logs/" .. target.name .. "-build.log")
			else
				Runtime.Job("build and publish " .. target.name, "done", "published")
			end
			terminalStates += 1
			changed = true
		end

		if not changed and preparing > 0 then
			Task.wait(0.05)
		elseif not changed then
			break
		end
	end

	consumeResults()
	if not nativeChecked then
		Publication.FailRun(publisher, "native runtime evidence is unavailable for " .. native)
	end
end

local Complete = {}

function Complete.Run(root: string)
	assert(Runtime.Environment("KERO_HEAP_PREPARED") == "1", "run distribution/actions/kero-build.sh instead of invoking the Luau controller directly")
	Runtime.WriteDirectory(root .. "/.heap")
	Runtime.WriteDirectory(root .. "/.heap/cache/logs")
	Runtime.StartRunLog(root .. "/.heap/cache/logs/validate-locally.log")
	local configuration = Settings.Read(root)
	local targets = enabledTargets(configuration)
	local jobs = { "source validation", "portable workflow validation", "toolchain preflight", "host-tool preparation", "WASM build" }
	for _, target in targets do
		table.insert(jobs, "prepare " .. target.name)
	end
	for _, target in targets do
		table.insert(jobs, "build and publish " .. target.name)
	end
	table.insert(jobs, "final validation")
	table.insert(jobs, "working-state cleanup")
	Runtime.StartDisplay(jobs)
	writeDependencyRecord(root)
	for _, target in targets do
		for _, dependency in target.builder.dependsOn do
			Runtime.Job("prepare " .. target.name, "wait", "depends on prepare " .. dependency)
		end
	end

	executeJob("source validation", "source-validation", 1, function()
		sourceChecks(root)
	end)
	executeJob("portable workflow validation", "portable-workflow", 2, function()
		portableChecks(root)
	end)
	executeJob("toolchain preflight", "toolchain-preflight", 2, function()
		Target.Preflight(root, configuration)
	end)
	executeJob("host-tool preparation", "host-tool-preparation", 1, function()
		Runtime.Step("host-tool preparation", "Installing", "shared aqtinstall")
		Target.PrepareHostTools(root, configuration)
		Runtime.StepComplete("host-tool preparation")
	end)
	local wslRoot = Runtime.Environment("KERO_WSL_WORK_ROOT")
	if wslRoot then
		local snapshot = wslRoot .. "/build/source"
		Runtime.Status("Copying", "source snapshot to Linux ext4")
		Runtime.RemoveDirectory(snapshot)
		run(root, "mkdir -p " .. Runtime.Quote(snapshot)
			.. " && git ls-files --cached --others --exclude-standard -z"
			.. " | tar --null --ignore-failed-read -T - -cf -"
			.. " | tar -xf - -C " .. Runtime.Quote(snapshot), "source-snapshot")
	end
	executeJob("WASM build", "wasm-build", 1, function()
		Runtime.Step("WASM build", "Compiling", "shared kero-core")
		local wslRoot = Runtime.Environment("KERO_WSL_WORK_ROOT")
		local cargoTarget = if wslRoot then wslRoot .. "/build/cargo/wasm" else root .. "/.heap/cache/build/cargo/wasm"
		local cargo = "CARGO_TARGET_DIR=" .. Runtime.Quote(cargoTarget) .. " cargo"
		local destination = root .. "/.heap/cache/build/cargo/wasm/wasm32-wasip1/release"
		local manifest = if wslRoot then wslRoot .. "/build/source/src/Cargo.toml" else root .. "/src/Cargo.toml"
		run(root, "rustup target add wasm32-wasip1 && " .. cargo .. " build --manifest-path " .. Runtime.Quote(manifest) .. " --release -p kero-core --target wasm32-wasip1"
			.. " && mkdir -p " .. Runtime.Quote(destination) .. " && cp -f "
			.. Runtime.Quote(cargoTarget .. "/wasm32-wasip1/release/kero_core.wasm") .. " " .. Runtime.Quote(destination .. "/kero_core.wasm"), "wasm-build")
		Runtime.StepComplete("WASM build")
	end)

	local publisher = Publication.Create(root, targets, Runtime.Environment("KERO_RESUME") == "1")
	for _, target in targets do
		if publisher.states[target.name] == "complete" then
			local valid = pcall(Artifact.Verify, root, target.name, target.name == hostTarget())
			if valid then
				Runtime.Job("prepare " .. target.name, "done", "verified existing artifact")
				Runtime.Job("build and publish " .. target.name, "done", "verified existing artifact")
			else
				publisher.digests[target.name] = nil
				Publication.SetState(publisher, target.name, "pending", "prior artifact needs rebuilding")
			end
		end
	end
	executeTargets(root, targets, configuration, publisher)
	Runtime.Job("final validation", "run", nil, 1)
	Runtime.Step("final validation", "Checking", "enabled distribution results")
	local complete = Publication.IsComplete(publisher)
	Runtime.StepComplete("final validation")
	if complete then
		Runtime.Job("final validation", "done")
		executeJob("working-state cleanup", "working-state-cleanup", 1, function()
			discardWorkingState(root)
		end)
		Publication.Finalize(publisher)
		Runtime.Finish("[ok] Validation produced every enabled artifact under .heap/release/artifacts/.")
		return
	end

	Runtime.Job("final validation", "fail", "one or more targets failed")
	Runtime.Job("working-state cleanup", "skip", "preserving diagnostic state")
	Publication.FailRun(publisher, "one or more enabled targets did not complete")
	Publication.Finalize(publisher)
	error("one or more enabled distribution targets failed", 0)
end

return Complete
