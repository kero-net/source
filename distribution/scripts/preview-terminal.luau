--!strict

-- Prints the terminal vocabulary without preparing a heap or running a build.
local process = require("@lune/process")
local Display = require("../lib/terminal/display")

assert(#process.args == 0, "preview-terminal accepts no arguments")
process.env.KERO_STYLE_PREVIEW = "1"
Display.Preview()
