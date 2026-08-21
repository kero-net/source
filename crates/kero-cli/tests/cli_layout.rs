use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn init_cli_creates_only_the_scope_consumer_root() {
    let repository = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scope"))
        .args(["init", "--root"])
        .arg(repository.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["schema"], "scope/initialized-layout/v1");
    let entries: Vec<_> = fs::read_dir(repository.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, [".scope"]);
}

#[test]
fn global_init_persists_selected_location_and_discovery_identity() {
    let temporary = tempdir().unwrap();
    let global = temporary.path().join("shared/global-scope");
    let config = temporary.path().join("config");
    let initialized = Command::new(env!("CARGO_BIN_EXE_scope"))
        .args(["global", "init", "--root"])
        .arg(&global)
        .env("SCOPE_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stdout)
    );
    assert!(global.join("scope.toml").is_file());
    assert!(global.join("knowledge").is_dir());
    assert!(config.join("config.toml").is_file());

    let discovered = Command::new(env!("CARGO_BIN_EXE_scope"))
        .args(["global", "path"])
        .env("SCOPE_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(discovered.status.success());
    let result: Value = serde_json::from_slice(&discovered.stdout).unwrap();
    assert_eq!(result["identity"]["kind"], "global");
    assert_eq!(result["identity"]["id"], "scope.global");

    let repeated = Command::new(env!("CARGO_BIN_EXE_scope"))
        .args(["global", "init"])
        .env("SCOPE_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(repeated.status.success());
    let result: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(
        result["global"]["root"],
        Value::String(global.display().to_string())
    );

    let knowledge = Command::new(env!("CARGO_BIN_EXE_scope"))
        .args(["knowledge", "path", "global", "--plain"])
        .env("SCOPE_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(knowledge.status.success());
    assert_eq!(
        String::from_utf8(knowledge.stdout).unwrap().trim(),
        global.join("knowledge").display().to_string()
    );
}
