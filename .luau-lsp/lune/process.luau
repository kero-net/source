--!strict

-- Analyzer-only declaration for the Lune process module.
export type Execution = {
  ok: boolean,
  code: number,
  stdout: string,
  stderr: string,
}

export type StandardStream = "default" | "inherit" | "forward" | "none"

export type ExecutionOptions = {
  cwd: string?,
  env: { [string]: string }?,
  shell: boolean | string?,
  stdio: StandardStream | {
    stdin: string | buffer?,
    stdout: StandardStream?,
    stderr: StandardStream?,
  }?,
}

local Process = {} :: {
  os: "linux" | "macos" | "windows",
  arch: "x86_64" | "aarch64",
  cwd: string,
  args: { string },
  env: { [string]: string? },
  exec: (program: string, arguments: { string }?, options: ExecutionOptions?) -> Execution,
  exit: (code: number?) -> never,
}

return Process
