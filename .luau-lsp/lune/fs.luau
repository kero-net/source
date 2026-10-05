--!strict

-- Analyzer-only declaration for the Lune filesystem module.
local FileSystem = {} :: {
  readFile: (path: string) -> string,
  readDir: (path: string) -> { string },
  writeFile: (path: string, contents: string | buffer) -> (),
  writeDir: (path: string) -> (),
  removeFile: (path: string) -> (),
  removeDir: (path: string) -> (),
  isFile: (path: string) -> boolean,
  isDir: (path: string) -> boolean,
  move: (from: string, to: string, overwrite: boolean?) -> (),
  copy: (from: string, to: string, overwrite: boolean?) -> (),
}

return FileSystem
