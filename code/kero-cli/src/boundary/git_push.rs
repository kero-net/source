//! Exact-refspec Git push boundary with ambient helpers disabled on Linux.

use crate::result::Capability;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

pub const BOUNDARY: &str = "broker.git-push/v1";

#[derive(Clone, Debug)]
pub struct PushSpec {
    pub worktree: PathBuf,
    pub remote: String,
    pub refspec: String,
    pub timeout: Duration,
}
#[derive(Debug, Error)]
pub enum GitPushError {
    #[error("git.push-invalid")]
    Invalid,
    #[error("git.capability-unavailable")]
    Unavailable,
    #[error("git.execution: {0}")]
    Execution(#[from] std::io::Error),
    #[error("git.nonzero")]
    Nonzero,
    #[error("git.timeout")]
    Timeout,
}

pub fn capability(binary: &Path) -> Capability {
    if binary.is_file() {
        Capability::Can
    } else {
        Capability::Cannot
    }
}

pub fn push(binary: impl Into<PathBuf>, spec: &PushSpec) -> Result<(), GitPushError> {
    validate(spec)?;
    let binary = binary.into();
    if capability(&binary) != Capability::Can {
        return Err(GitPushError::Unavailable);
    }
    let mut child = Command::new(binary)
        .current_dir(&spec.worktree)
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "/bin/false")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "credential.helper=",
            "push",
            &spec.remote,
            &spec.refspec,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + spec.timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(GitPushError::Nonzero)
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(GitPushError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn validate(spec: &PushSpec) -> Result<(), GitPushError> {
    if !spec.worktree.is_dir()
        || spec.timeout.is_zero()
        || spec.remote.is_empty()
        || !spec.refspec.starts_with("refs/")
        || spec.refspec.contains('*')
        || spec.refspec.contains(':')
        || spec.refspec.ends_with("/delete")
    {
        return Err(GitPushError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn rejects_force_deletion_and_wildcard_refspecs() {
        let directory = tempdir().unwrap();
        let base = PushSpec {
            worktree: directory.path().into(),
            remote: "https://example.invalid/repo.git".into(),
            refspec: "refs/heads/main".into(),
            timeout: Duration::from_secs(1),
        };
        validate(&base).unwrap();
        for refspec in [
            "+refs/heads/main",
            "refs/heads/*",
            "refs/heads/main:refs/heads/main",
            "refs/heads/delete",
        ] {
            assert!(
                validate(&PushSpec {
                    refspec: refspec.into(),
                    ..base.clone()
                })
                .is_err(),
                "accepted {refspec}"
            );
        }
    }

    #[test]
    fn pushes_an_exact_refspec_to_a_local_broker_owned_remote() {
        let directory = tempdir().unwrap();
        let worktree = directory.path().join("worktree");
        let remote = directory.path().join("remote.git");
        let git = Path::new("/usr/bin/git");
        if !git.is_file() {
            return;
        }
        for arguments in [
            vec!["init", "--initial-branch=main", worktree.to_str().unwrap()],
            vec!["init", "--bare", remote.to_str().unwrap()],
        ] {
            assert!(
                Command::new(git)
                    .args(arguments)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        std::fs::write(worktree.join("README"), "fixture\n").unwrap();
        for arguments in [
            vec![
                "-C",
                worktree.to_str().unwrap(),
                "config",
                "user.name",
                "KERO Test",
            ],
            vec![
                "-C",
                worktree.to_str().unwrap(),
                "config",
                "user.email",
                "kero@example.invalid",
            ],
            vec!["-C", worktree.to_str().unwrap(), "add", "README"],
            vec!["-C", worktree.to_str().unwrap(), "commit", "-m", "fixture"],
            vec![
                "-C",
                worktree.to_str().unwrap(),
                "remote",
                "add",
                "origin",
                remote.to_str().unwrap(),
            ],
        ] {
            assert!(
                Command::new(git)
                    .args(arguments)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        push(
            git,
            &PushSpec {
                worktree: worktree.clone(),
                remote: "origin".into(),
                refspec: "refs/heads/main".into(),
                timeout: Duration::from_secs(5),
            },
        )
        .unwrap();
        assert!(
            Command::new(git)
                .args([
                    "--git-dir",
                    remote.to_str().unwrap(),
                    "show-ref",
                    "--verify",
                    "refs/heads/main"
                ])
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    #[cfg(unix)]
    fn kills_a_nonresponsive_git_process_at_the_bound_timeout() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir().unwrap();
        let fake_git = directory.path().join("git-sleep");
        std::fs::write(&fake_git, "#!/bin/sh\n/bin/sleep 2\n").unwrap();
        std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o700)).unwrap();
        let spec = PushSpec {
            worktree: directory.path().into(),
            remote: "origin".into(),
            refspec: "refs/heads/main".into(),
            timeout: Duration::from_millis(100),
        };
        let result = push(fake_git, &spec);
        assert!(matches!(result, Err(GitPushError::Timeout)), "{result:?}");
    }
}
