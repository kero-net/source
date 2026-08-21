use kero_core::boundary::git_push::{self, GitPushError, PushSpec};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir;

fn spec(worktree: PathBuf) -> PushSpec {
    PushSpec {
        worktree,
        remote: "origin".into(),
        refspec: "refs/heads/main".into(),
        timeout: Duration::from_secs(1),
    }
}

#[test]
fn unavailable_git_is_a_capability_result_not_a_push_attempt() {
    let directory = tempdir().unwrap();
    assert!(matches!(
        git_push::push("/definitely-not-kero-git", &spec(directory.path().into())),
        Err(GitPushError::Unavailable)
    ));
}

#[test]
fn unsafe_refspecs_and_missing_worktrees_fail_closed() {
    let directory = tempdir().unwrap();
    for refspec in ["+refs/heads/main", "refs/heads/*", "refs/heads/main:other"] {
        assert!(matches!(
            git_push::push(
                "/definitely-not-kero-git",
                &PushSpec {
                    refspec: refspec.into(),
                    ..spec(directory.path().into())
                },
            ),
            Err(GitPushError::Invalid)
        ));
    }
    assert!(matches!(
        git_push::push(
            "/definitely-not-kero-git",
            &spec(directory.path().join("missing"))
        ),
        Err(GitPushError::Invalid)
    ));
}
