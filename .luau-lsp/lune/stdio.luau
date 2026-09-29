--!strict

-- Analyzer-only declaration for Lune's direct terminal output module.
local Stdio = {} :: {
	write: (text: string) -> (),
	ewrite: (text: string) -> (),
	readLine: () -> string,
}

return Stdio
