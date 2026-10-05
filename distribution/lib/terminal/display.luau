-- distribution/lib/terminal/display.luau
--!strict

local process = require("@lune/process")
local stdio = require("@lune/stdio")
local Progress = require("./progress")

-- Terminal colors are presentation only; logs and redirected output stay plain.
local colors = {
    reset = "\27[0m", dim = "\27[2m", blue = "\27[34m",
    cyan = "\27[36m", green = "\27[32m", yellow = "\27[33m",
    red = "\27[31m", magenta = "\27[35m",
}

local function colorEnabled(): boolean
    return process.env.NO_COLOR == nil and process.env.TERM ~= "dumb"
        and (process.env.KERO_INTERACTIVE == "1" or process.env.KERO_STYLE_PREVIEW == "1")
end

local function paint(value: string, color: string): string
    if not colorEnabled() then return value end
    return color .. value .. colors.reset
end

local function labelColor(action: string): string
    local key = action:lower()
    if key == "failed" or key == "error" or key == "fatal" then return colors.red end
    if key == "warn" or key == "warning" or key == "skipped" or key == "skip" then return colors.yellow end
    if key == "completed" or key == "ok" or key == "success" then return colors.green end
    if key == "reading" or key == "listing" or key == "inspecting" then return colors.dim end
    if key == "running" or key == "starting" then return colors.cyan end
    return colors.blue
end

local function statusLine(action: string, message: string): string
    return "    " .. paint("[" .. action .. "]", labelColor(action)) .. " " .. message
end

local function nestedLine(line: string): string
    local marker = line:sub(1, 2)
    if marker == "::" then return "        " .. paint("::", colors.blue) .. line:sub(3) end
    marker = line:sub(1, 1)
    if marker == "+" then return "        " .. paint("+", colors.green) .. line:sub(2) end
    if marker == "-" then return "        " .. paint("-", colors.yellow) .. line:sub(2) end
    return "        " .. line
end

export type JobState = "wait" | "run" | "done" | "fail" | "skip"

type Job = {
    name: string,
    state: JobState,
    completed: number,
    total: number,
    active: string?,
}

export type Controller = {
    interactive: boolean,
    jobs: { Job },
    index: { [string]: number },
    current: string?,
    status: string,
    columns: number,
    progressVisible: boolean,
    outputPartial: boolean,
    childLineStart: boolean,
    closed: boolean,
}

local function terminalColumns(): number
    if process.os ~= "windows" then
        local result = process.exec("/bin/sh", { "-c", "stty size < /dev/tty 2>/dev/null" })
        local columns = tonumber(result.stdout:match("%d+%s+(%d+)"))
        if result.ok and columns and columns >= 40 then
            return columns
        end
    end
    return tonumber(process.env.COLUMNS or "") or 120
end

local function terminalJob(state: JobState): boolean
    return state == "done" or state == "fail" or state == "skip"
end

local function completedJobs(controller: Controller): number
    local completed = 0
    for _, job in controller.jobs do
        if terminalJob(job.state) then
            completed += 1
        end
    end
    return completed
end

local function currentJob(controller: Controller): Job?
    if controller.current then
        local index = controller.index[controller.current]
        if index and controller.jobs[index].state == "run" then
            return controller.jobs[index]
        end
    end
    for _, job in controller.jobs do
        if job.state == "run" then
            controller.current = job.name
            return job
        end
    end
    controller.current = nil
    return nil
end

local function trim(text: string, width: number): string
    if #text <= width then
        return text
    end
    if width <= 3 then
        return text:sub(1, math.max(0, width))
    end
    return text:sub(1, width - 3) .. "..."
end

local function progressText(controller: Controller): string
    local completed = completedJobs(controller)
    local total = #controller.jobs
    local active, remaining = Progress.Segments(completed, total, 12)
    local job = currentJob(controller)
    local status = if controller.status ~= "" then controller.status else (if job then job.name else "")
    local line = "Building [" .. active .. remaining .. "] " .. Progress.Count(completed, total)
    if status ~= "" then
        line ..= ": " .. status
    end
    return paint(trim(line, controller.columns - 1), colors.magenta)
end

local function clearProgress(controller: Controller)
    if controller.interactive and controller.progressVisible then
        stdio.write("\r\27[2K")
        controller.progressVisible = false
    end
end

local function drawProgress(controller: Controller)
    if controller.interactive and not controller.closed and not controller.outputPartial then
        stdio.write("\r\27[2K" .. progressText(controller))
        controller.progressVisible = true
    end
end

local function setStatus(controller: Controller, action: string, message: string)
    controller.status = action .. " " .. message
    if controller.interactive then
        clearProgress(controller)
        if controller.outputPartial then
            stdio.write("\n")
            controller.outputPartial = false
            controller.childLineStart = true
        end
        stdio.write(statusLine(action, message) .. "\n")
        drawProgress(controller)
    else
        stdio.write(statusLine(action, message) .. "\n")
    end
end

local Display = {}

function Display.Create(names: { string }): Controller
    local controller: Controller = {
        interactive = process.env.KERO_INTERACTIVE == "1",
        jobs = {},
        index = {},
        current = nil,
        status = "",
        columns = terminalColumns(),
        progressVisible = false,
        outputPartial = false,
        childLineStart = true,
        closed = false,
    }
    for index, name in names do
        table.insert(controller.jobs, { name = name, state = "wait", completed = 0, total = 0 })
        controller.index[name] = index
    end
    drawProgress(controller)
    return controller
end

function Display.Update(controller: Controller, name: string, state: JobState, detail: string?, total: number?)
    local index = assert(controller.index[name], "unknown display job: " .. name)
    local job = controller.jobs[index]
    job.state = state
    if total then
        assert(total > 0, "job progress requires at least one action")
        job.total = total
        job.completed = 0
    end
    if state == "run" then
        controller.current = name
    elseif terminalJob(state) and controller.current == name then
        controller.current = nil
    end
    if state == "fail" then
        setStatus(controller, "Failed", name .. (if detail then ": " .. detail else ""))
    elseif state == "skip" then
        setStatus(controller, "Skipped", name .. (if detail then ": " .. detail else ""))
    elseif state == "done" then
        setStatus(controller, "Completed", name)
    elseif state == "run" then
        setStatus(controller, "Starting", name)
    else
        drawProgress(controller)
    end
end

function Display.Step(controller: Controller, name: string, action: string, message: string)
    local index = assert(controller.index[name], "unknown display job: " .. name)
    local job = controller.jobs[index]
    assert(job.state == "run", "cannot start an action for an inactive job: " .. name)
    controller.current = name
    job.active = message
    setStatus(controller, action, message)
end

function Display.StepComplete(controller: Controller, name: string)
    local index = assert(controller.index[name], "unknown display job: " .. name)
    local job = controller.jobs[index]
    if job.total > 0 then
        job.completed = math.min(job.completed + 1, job.total)
    end
    local completed = job.active
    job.active = nil
    if completed then
        setStatus(controller, "Completed", completed)
    else
        drawProgress(controller)
    end
end

function Display.Status(controller: Controller?, action: string, message: string, _dim: boolean?)
    if controller then
        setStatus(controller, action, message)
    else
        stdio.write(statusLine(action, message) .. "\n")
    end
end

function Display.Preview()
    stdio.write(paint("KERO terminal style preview", colors.magenta) .. "\n")
    stdio.write(paint("Building [====>       ] 5/13: preparing linux-x86_64", colors.magenta) .. "\n")
    stdio.write(statusLine("Starting", "source validation") .. "\n")
    stdio.write(statusLine("Reading", "distribution/policy.toml") .. "\n")
    stdio.write(statusLine("Listing", "distribution/builds") .. "\n")
    stdio.write(statusLine("Inspecting", "toolchain availability") .. "\n")
    stdio.write(statusLine("Running", "source-checks: lune run .github/scripts/source-checks.luau") .. "\n")
    stdio.write(nestedLine(":: repository checks") .. "\n")
    stdio.write(nestedLine("+ lune run .github/scripts/source-checks.luau local") .. "\n")
    stdio.write(nestedLine("- pages: pandoc is unavailable") .. "\n")
    stdio.write(statusLine("Completed", "source validation") .. "\n")
    stdio.write(statusLine("Skipped", "pages: pandoc is unavailable") .. "\n")
    stdio.write(statusLine("Warning", "optional prerequisite is missing") .. "\n")
    stdio.write(statusLine("Failed", "linux-x86_64: command exited with code 1") .. "\n")
    stdio.write(statusLine("Error", "see .heap/logs/validate-locally.log") .. "\n")
    stdio.write("\n" .. paint("Legend:", colors.blue) .. " 0 = progress; 4 = runner actions; 8 = scopes, commands, and child output.\n")
    stdio.write("        :: scope    + command run    - omitted optional work\n")
end

function Display.Output(controller: Controller?, chunk: string)
    if chunk == "" then
        return
    end
    if not controller then
        stdio.write(chunk)
        return
    end
    clearProgress(controller)
    local offset = 1
    while offset <= #chunk do
        local newline = chunk:find("\n", offset, true)
        local finish = newline or #chunk
        local part = chunk:sub(offset, finish)
        if controller.childLineStart then
            stdio.write(nestedLine(part))
        else
            stdio.write(part)
        end
        controller.childLineStart = newline ~= nil
        offset = finish + 1
    end
    controller.outputPartial = not (chunk:sub(-1) == "\n" or chunk:sub(-1) == "\r")
    drawProgress(controller)
end

function Display.Confirm(controller: Controller?, message: string): boolean
    if controller then
        clearProgress(controller)
    end
    stdio.write(message .. " [Y/n] ")
    local answer = stdio.readLine():lower()
    if controller then
        controller.outputPartial = false
        drawProgress(controller)
    end
    return answer == "" or answer == "y" or answer == "yes"
end

function Display.Message(controller: Controller?, message: string)
    if controller then
        clearProgress(controller)
        if controller.outputPartial then
            stdio.write("\n")
            controller.childLineStart = true
        end
        controller.closed = true
    end
    stdio.write(message .. "\n")
end

return Display
