use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("state.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("state.malformed")]
    Malformed,
    #[error("state.disabled")]
    Disabled,
    #[error("state.generation-mismatch")]
    Generation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StateFile {
    generation: u64,
    disabled: bool,
}

/// Owns the durable private state for one broker boundary.
#[derive(Clone, Debug)]
pub struct BrokerState {
    root: PathBuf,
}

impl BrokerState {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StateError> {
        let root = root.into();
        fs::create_dir_all(root.join("nonces"))?;
        private(&root)?;
        private(&root.join("nonces"))?;
        let state = Self { root };
        if !state.state_path().exists() {
            state.write(StateFile {
                generation: 0,
                disabled: false,
            })?;
        }
        Ok(state)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn nonce_dir(&self) -> PathBuf {
        self.root.join("nonces")
    }
    pub fn audit_path(&self) -> PathBuf {
        self.root.join("audit.jsonl")
    }
    pub fn checkpoint_path(&self) -> PathBuf {
        self.root.join("checkpoint.json")
    }

    pub fn generation(&self) -> Result<u64, StateError> {
        Ok(self.read()?.generation)
    }

    pub fn require_active(&self, expected_generation: u64) -> Result<(), StateError> {
        let state = self.read()?;
        if state.disabled {
            return Err(StateError::Disabled);
        }
        if state.generation != expected_generation {
            return Err(StateError::Generation);
        }
        Ok(())
    }

    /// Invalidates every artifact bound to the prior generation.
    pub fn disable(&self) -> Result<u64, StateError> {
        let mut state = self.read()?;
        state.disabled = true;
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or(StateError::Generation)?;
        self.write(state.clone())?;
        Ok(state.generation)
    }

    pub(crate) fn enable_after_recovery(&self, expected_generation: u64) -> Result<(), StateError> {
        let mut state = self.read()?;
        if state.generation != expected_generation {
            return Err(StateError::Generation);
        }
        state.disabled = false;
        self.write(state)
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }
    fn read(&self) -> Result<StateFile, StateError> {
        serde_json::from_slice(&fs::read(self.state_path())?).map_err(|_| StateError::Malformed)
    }
    fn write(&self, state: StateFile) -> Result<(), StateError> {
        let destination = self.state_path();
        let temporary = self.root.join(format!(".state-{}", std::process::id()));
        let bytes = serde_json::to_vec(&state).map_err(|_| StateError::Malformed)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        private(&temporary)?;
        fs::rename(&temporary, &destination)?;
        OpenOptions::new().read(true).open(&self.root)?.sync_all()?;
        Ok(())
    }
}

fn private(path: &Path) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
