use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn built_cli_captures_lists_and_removes_knowledge_snapshots() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let runtime = build_wasm_core(workspace);
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    let source = directory.path().join("source");
    let home = directory.path().join("home");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(&source).unwrap();
    git(&repository, ["init"]);
    std::fs::create_dir(source.join("guides")).unwrap();
    std::fs::write(source.join("guides").join("start.md"), "KERO smoke input\n").unwrap();

    run(
        &runtime,
        &home,
        ["init".into(), repository.display().to_string()],
    );
    let added = run(
        &runtime,
        &home,
        [
            "knowledge".into(),
            "add".into(),
            source.display().to_string(),
            "--repository".into(),
            repository.display().to_string(),
        ],
    );
    let id = added["result"]["id"].as_str().unwrap().to_owned();
    assert_eq!(id.len(), 64);
    assert!(
        repository
            .join(".kero/data/input")
            .join(&id)
            .join("content/guides/start.md")
            .is_file()
    );

    let listed = run(
        &runtime,
        &home,
        [
            "knowledge".into(),
            "list".into(),
            "--repository".into(),
            repository.display().to_string(),
        ],
    );
    assert_eq!(listed["result"].as_array().unwrap().len(), 1);
    assert_eq!(listed["result"][0]["id"], id);

    let processed = run(
        &runtime,
        &home,
        [
            "process".into(),
            "build".into(),
            id.clone(),
            "--repository".into(),
            repository.display().to_string(),
        ],
    );
    let artifact = PathBuf::from(processed["result"]["path"].as_str().unwrap());
    assert!(artifact.is_file());
    run(
        &runtime,
        &home,
        [
            "process".into(),
            "verify".into(),
            id.clone(),
            "--repository".into(),
            repository.display().to_string(),
        ],
    );
    let key = directory.path().join("signing.key");
    std::fs::write(
        &key,
        "0707070707070707070707070707070707070707070707070707070707070707\n",
    )
    .unwrap();
    let signature = run(
        &runtime,
        &home,
        [
            "trust".into(),
            "sign".into(),
            artifact.display().to_string(),
            "--key".into(),
            key.display().to_string(),
        ],
    );
    assert!(PathBuf::from(signature["result"]["path"].as_str().unwrap()).is_file());
    run(
        &runtime,
        &home,
        [
            "trust".into(),
            "verify".into(),
            artifact.display().to_string(),
        ],
    );

    run(
        &runtime,
        &home,
        [
            "knowledge".into(),
            "remove".into(),
            id.clone(),
            "--repository".into(),
            repository.display().to_string(),
        ],
    );
    assert!(!repository.join(".kero/data/input").join(id).exists());
}

fn build_wasm_core(workspace: &Path) -> PathBuf {
    let manifest = workspace.join("Cargo.toml");
    let status = Command::new("cargo")
        .args(["build", "--manifest-path"])
        .arg(manifest)
        .args(["-p", "kero-core", "--target", "wasm32-wasip1"])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "could not build the WASM core for smoke testing"
    );
    let runtime = workspace.join("target/wasm32-wasip1/debug/kero_core.wasm");
    assert!(runtime.is_file(), "WASM core artifact was not produced");
    runtime
}

fn run<const N: usize>(runtime: &Path, home: &Path, args: [String; N]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_kero-host"))
        .args(["--service-dispatch", "--json", "--runtime"])
        .arg(runtime)
        .args(args)
        .env("KERO_HOME", home)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = serde_json::from_slice::<Value>(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    value
}

fn git<const N: usize>(directory: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
