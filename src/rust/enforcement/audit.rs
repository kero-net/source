use super::model::{Attempt, Lifecycle};
use crate::canonical;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuditEvent {
    pub schema: String,
    pub attempt: Attempt,
    pub lifecycle: Lifecycle,
    pub previous: String,
}

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("audit.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("audit.malformed")]
    Malformed,
    #[error(transparent)]
    Canonical(#[from] canonical::CanonicalError),
}

#[derive(Clone, Debug)]
pub struct AuditLog {
    path: PathBuf,
}

impl AuditLog {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn append(&self, attempt: Attempt, lifecycle: Lifecycle) -> Result<String, AuditError> {
        // Verify the complete chain before admitting the next durable event;
        // a valid final line alone cannot prove earlier history was intact.
        let previous = self.verify()?;
        let event = AuditEvent {
            schema: "scope/audit-event/v1".into(),
            attempt,
            lifecycle,
            previous,
        };
        let bytes = canonical::canonicalize(&event)?;
        let digest = canonical::sha256(&bytes);
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&self.path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(digest)
    }

    pub fn head(&self) -> Result<String, AuditError> {
        let Some(last) = fs::read_to_string(&self.path)
            .ok()
            .and_then(|data| data.lines().last().map(str::to_owned))
        else {
            return Ok("sha256:genesis".into());
        };
        let event: AuditEvent = serde_json::from_str(&last).map_err(|_| AuditError::Malformed)?;
        let canonical = canonical::canonicalize(&event)?;
        if canonical != last.as_bytes() {
            return Err(AuditError::Malformed);
        }
        Ok(canonical::sha256(&canonical))
    }

    pub fn verify(&self) -> Result<String, AuditError> {
        let data = match fs::read_to_string(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok("sha256:genesis".into());
            }
            Err(error) => return Err(error.into()),
        };
        let mut previous = "sha256:genesis".to_owned();
        for line in data.lines() {
            let event: AuditEvent =
                serde_json::from_str(line).map_err(|_| AuditError::Malformed)?;
            let bytes = canonical::canonicalize(&event)?;
            if bytes != line.as_bytes() || event.previous != previous {
                return Err(AuditError::Malformed);
            }
            previous = canonical::sha256(&bytes);
        }
        Ok(previous)
    }
}
