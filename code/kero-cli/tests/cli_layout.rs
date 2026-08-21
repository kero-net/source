use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn project_init_cli_creates_only_the_kero_consumer_root() {
    let repository = tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args(["project", "init", "--root"])
        .arg(repository.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["schema"], "kero/initialized-layout/v1");
    let entries: Vec<_> = fs::read_dir(repository.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, [".kero"]);
}

#[test]
fn setup_init_configures_an_explicit_global_root_without_prompting() {
    let temporary = tempdir().unwrap();
    let global = temporary.path().join("global-kero");
    let config = temporary.path().join("config");
    let output = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args(["init", "--root"])
        .arg(&global)
        .env("KERO_CONFIG_HOME", &config)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["schema"], "kero/setup-initialized/v1");
    assert_eq!(result["global"]["root"], global.display().to_string());
    assert!(config.join("config.toml").is_file());
}

#[test]
fn global_init_persists_selected_location_and_discovery_identity() {
    let temporary = tempdir().unwrap();
    let global = temporary.path().join("shared/global-kero");
    let config = temporary.path().join("config");
    let initialized = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args(["global", "init", "--root"])
        .arg(&global)
        .env("KERO_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stdout)
    );
    assert!(global.join("kero.toml").is_file());
    assert!(global.join("knowledge").is_dir());
    assert!(config.join("config.toml").is_file());

    let discovered = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args(["global", "path"])
        .env("KERO_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(discovered.status.success());
    let result: Value = serde_json::from_slice(&discovered.stdout).unwrap();
    assert_eq!(result["identity"]["kind"], "global");
    assert_eq!(result["identity"]["id"], "kero.global");

    let repeated = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args(["global", "init"])
        .env("KERO_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(repeated.status.success());
    let result: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(
        result["global"]["root"],
        Value::String(global.display().to_string())
    );

    let knowledge = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args(["knowledge", "path", "global", "--plain"])
        .env("KERO_CONFIG_HOME", &config)
        .output()
        .unwrap();
    assert!(knowledge.status.success());
    assert_eq!(
        String::from_utf8(knowledge.stdout).unwrap().trim(),
        global.join("knowledge").display().to_string()
    );
}
