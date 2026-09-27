local target, runtime = assert(arg[1], "usage: lua distribution/tests/run.lua TARGET [--runtime]"), arg[2] == "--runtime"
assert(target == "windows-arm64" or target == "windows-x64" or target == "linux-x64" or target == "macos-arm64", "unknown target")

assert(loadfile("distribution/tests/contract.lua"))()
local suite = target:match("^windows") and "distribution/tests/windows.lua" or "distribution/tests/unix.lua"
local chunk = assert(loadfile(suite))
local previous = arg
arg = { [0] = suite, target, runtime and "--runtime" or "--static" }
chunk()
arg = previous
io.stdout:write("[ok] Distribution tests passed for ", target, "\n")
