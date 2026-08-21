use kero_core::boundary::bubblewrap::{self, BubblewrapError, CommandSpec};
use std::time::Duration;

fn spec() -> CommandSpec {
    CommandSpec {
        argv: vec!["/bin/echo".into(), "fixture".into()],
        working_directory: "/".into(),
        environment: Vec::new(),
        timeout: Duration::from_secs(1),
        output_limit: 64,
    }
}

#[test]
fn absent_bubblewrap_is_a_capability_failure() {
    assert!(matches!(
        bubblewrap::run("/definitely-not-kero-bwrap", &spec()),
        Err(BubblewrapError::Unavailable)
    ));
}

#[test]
fn credential_environment_and_non_absolute_workdir_are_rejected() {
    let mut unsafe_environment = spec();
    unsafe_environment
        .environment
        .push(("API_TOKEN".into(), "secret".into()));
    assert!(matches!(
        bubblewrap::run("/definitely-not-kero-bwrap", &unsafe_environment),
        Err(BubblewrapError::Invalid)
    ));
    let mut unsafe_directory = spec();
    unsafe_directory.working_directory = "relative".into();
    assert!(matches!(
        bubblewrap::run("/definitely-not-kero-bwrap", &unsafe_directory),
        Err(BubblewrapError::Invalid)
    ));
}
