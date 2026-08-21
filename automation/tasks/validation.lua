local command = require("lib.command")
local json = require("lib.json")
local validation = {}
function validation.run(context)
  for _, file in ipairs({
    "automation/config/automation.json", "automation/config/publication.json",
    "automation/config/repository-validation.json", "automation/schemas/automation.schema.json",
    "automation/schemas/publication.schema.json", "automation/schemas/repository-validation.schema.json",
  }) do
    local ok, message = json.validate_file(context.root .. "/" .. file); if not ok then return false, message end
  end
  for _, value in ipairs({
    "git diff --check", "find automation -name '*.lua' -print0 | xargs -0 -n1 luac -p",
    "groff -z -mandoc docs/man/kero.1",
    "CARGO_TARGET_DIR=\"$PWD/../.heap/source/cargo/target\" cargo fmt --manifest-path code/Cargo.toml --all --check",
    "CARGO_TARGET_DIR=\"$PWD/../.heap/source/cargo/target\" cargo clippy --manifest-path code/Cargo.toml --all-targets --all-features -- -D warnings",
    "CARGO_TARGET_DIR=\"$PWD/../.heap/source/cargo/target\" cargo test --manifest-path code/Cargo.toml --all-targets --locked",
  }) do
    local ok, message = command.run(context.root, value); if not ok then return false, message end
  end
  return true
end
return validation
