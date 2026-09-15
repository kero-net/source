use std::process::Command;
#[test]
fn add_and_remove_use_reviewable_project() {
    let temp = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_kero");
    assert!(
        Command::new(bin)
            .arg("init")
            .arg(temp.path())
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(temp.path().join("guide.md"), "# Guide\n").unwrap();
    assert!(
        Command::new(bin)
            .arg("add")
            .arg(temp.path().join("guide.md"))
            .args(["--project"])
            .arg(temp.path())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new(bin)
            .args(["remove", "guide", "--project"])
            .arg(temp.path())
            .status()
            .unwrap()
            .success()
    );
}
