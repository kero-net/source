use super::state::{BrokerState, StateError};
use std::fs::OpenOptions;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NonceError {
    #[error(transparent)]
    State(#[from] StateError),
    #[error("nonce.invalid")]
    Invalid,
    #[error("nonce.replay")]
    Replay,
    #[error("nonce.io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct NonceStore {
    state: BrokerState,
}

impl NonceStore {
    pub fn new(state: BrokerState) -> Self {
        Self { state }
    }

    /// Admission is durable and irrevocable. Callers must retain a consumed
    /// nonce after any later failure, including an uncertain side effect.
    pub fn admit(&self, nonce: &str, generation: u64) -> Result<(), NonceError> {
        self.state.require_active(generation)?;
        if nonce.len() != 64 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(NonceError::Invalid);
        }
        let path = self.state.nonce_dir().join(nonce);
        match OpenOptions::new().create_new(true).write(true).open(path) {
            Ok(file) => {
                file.sync_all()?;
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(NonceError::Replay)
            }
            Err(error) => Err(error.into()),
        }
    }
}
