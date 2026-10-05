--!strict

-- Analyzer-only declaration for the Lune serialization module.
export type Format = "json" | "jsonc" | "toml" | "yaml"

local Serde = {} :: {
  encode: (format: Format, value: never, pretty: boolean?) -> string,
  decode: <T>(format: Format, encoded: string | buffer) -> T,
  hash: (algorithm: "sha256", message: string | buffer) -> string,
}

return Serde
