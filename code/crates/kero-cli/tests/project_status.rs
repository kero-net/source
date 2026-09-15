use std::process::Command;
#[test]
fn status_is_machine_readable() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kero-core/tests/fixtures/knowledge/phase4");
    let output = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args(["--json", "status"])
        .arg(fixture)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema"], "kero/cli-result/v1alpha1");
}
