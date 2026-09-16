use std::process::Command;
#[test]
fn init_is_minimal_and_refuses_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("README.md"), "# suggested").unwrap();
    let bin = env!("CARGO_BIN_EXE_kero");
    let first = Command::new(bin)
        .args(["--json", "init"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(first.status.success());
    let project = std::fs::read_to_string(temp.path().join(".kero/project.toml")).unwrap();
    assert!(!project.contains("mounts"));
    assert!(
        !Command::new(bin)
            .arg("init")
            .arg(temp.path())
            .status()
            .unwrap()
            .success()
    );
}
