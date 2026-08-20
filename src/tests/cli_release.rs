use std::process::Command;

#[test]
fn supported_release_commands_are_listed_and_invalid_write_is_structured() {
    let binary = env!("CARGO_BIN_EXE_scope");
    let help = Command::new(binary).arg("--help").output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("workspace-write"));
    assert!(text.contains("verify-artifact"));

    let invalid = Command::new(binary)
        .args(["workspace-write", "--root", "/missing"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(
        String::from_utf8(invalid.stderr)
            .unwrap()
            .contains("required arguments")
    );
}
