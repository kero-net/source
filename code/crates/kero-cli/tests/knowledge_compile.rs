use std::process::Command;

#[test]
fn compiles_and_inspects_phase4_fixture_deterministically() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kero-core/tests/fixtures/knowledge/phase4");
    let temp = tempfile::tempdir().unwrap();
    let one = temp.path().join("one.json");
    let two = temp.path().join("two.json");
    for output in [&one, &two] {
        assert!(
            Command::new(env!("CARGO_BIN_EXE_kero"))
                .args(["knowledge", "compile"])
                .arg(&fixture)
                .args(["--output"])
                .arg(output)
                .status()
                .unwrap()
                .success()
        );
    }
    assert_eq!(std::fs::read(&one).unwrap(), std::fs::read(&two).unwrap());
    assert!(
        Command::new(env!("CARGO_BIN_EXE_kero"))
            .args(["--json", "knowledge", "inspect"])
            .arg(&one)
            .output()
            .unwrap()
            .status
            .success()
    );
}
