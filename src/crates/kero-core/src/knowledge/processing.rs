use crate::host::native_input;
use crate::host::native_repository::{RUNTIME_DIRECTORY, RepositoryBoundary};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessedFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessedArtifact {
    pub format_version: u32,
    pub input_id: String,
    pub files: Vec<ProcessedFile>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DetachedSignature {
    pub format_version: u32,
    pub algorithm: String,
    pub artifact_sha256: String,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("processing.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("processing.input: {0}")]
    Input(String),
    #[error("processing.artifact: {0}")]
    Artifact(String),
    #[error("trust.key: {0}")]
    Key(String),
    #[error("trust.signature: {0}")]
    Signature(String),
}

pub fn build(boundary: &RepositoryBoundary, id: &str) -> Result<PathBuf, ProcessingError> {
    let artifact = reconstruct(boundary, id)?;
    let path = artifact_path(boundary, id);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, canonical_json(&artifact)?)?;
    Ok(path)
}

pub fn verify(boundary: &RepositoryBoundary, id: &str) -> Result<PathBuf, ProcessingError> {
    let path = artifact_path(boundary, id);
    let expected = canonical_json(&reconstruct(boundary, id)?)?;
    if fs::read(&path)? != expected {
        return Err(ProcessingError::Artifact(format!(
            "artifact does not reconstruct: {}",
            path.display()
        )));
    }
    Ok(path)
}

pub fn sign(
    artifact: &Path,
    key: &Path,
    output: Option<&Path>,
) -> Result<PathBuf, ProcessingError> {
    let seed = fs::read_to_string(key)?;
    let seed = seed.trim();
    if seed.len() != 64
        || !seed
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(ProcessingError::Key(
            "key must contain exactly 32 lower-case hexadecimal bytes".into(),
        ));
    }
    let seed = hex::decode(seed)
        .map_err(|_| ProcessingError::Key("key must be lower-case hexadecimal".into()))?;
    let seed: [u8; 32] = seed
        .try_into()
        .map_err(|_| ProcessingError::Key("key must contain exactly 32 bytes".into()))?;
    let signing = SigningKey::from_bytes(&seed);
    let bytes = fs::read(artifact)?;
    let signature = signing.sign(&bytes);
    let record = DetachedSignature {
        format_version: FORMAT_VERSION,
        algorithm: "ed25519".into(),
        artifact_sha256: sha256(&bytes),
        public_key: hex::encode(signing.verifying_key().to_bytes()),
        signature: hex::encode(signature.to_bytes()),
    };
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| artifact.with_extension("sig.json"));
    fs::write(&output, canonical_json(&record)?)?;
    Ok(output)
}

pub fn verify_signature(
    artifact: &Path,
    signature: Option<&Path>,
) -> Result<PathBuf, ProcessingError> {
    let path = signature
        .map(Path::to_path_buf)
        .unwrap_or_else(|| artifact.with_extension("sig.json"));
    let record: DetachedSignature = serde_json::from_slice(&fs::read(&path)?)
        .map_err(|error| ProcessingError::Signature(error.to_string()))?;
    if record.format_version != FORMAT_VERSION || record.algorithm != "ed25519" {
        return Err(ProcessingError::Signature(
            "unsupported signature format".into(),
        ));
    }
    let bytes = fs::read(artifact)?;
    if record.artifact_sha256 != sha256(&bytes) {
        return Err(ProcessingError::Signature(
            "artifact digest does not match signature".into(),
        ));
    }
    let public: [u8; 32] = hex::decode(record.public_key)
        .map_err(|_| ProcessingError::Signature("invalid public key".into()))?
        .try_into()
        .map_err(|_| ProcessingError::Signature("invalid public key".into()))?;
    let signature: [u8; 64] = hex::decode(record.signature)
        .map_err(|_| ProcessingError::Signature("invalid signature".into()))?
        .try_into()
        .map_err(|_| ProcessingError::Signature("invalid signature".into()))?;
    VerifyingKey::from_bytes(&public)
        .map_err(|_| ProcessingError::Signature("invalid public key".into()))?
        .verify(&bytes, &Signature::from_bytes(&signature))
        .map_err(|_| ProcessingError::Signature("signature verification failed".into()))?;
    Ok(path)
}

fn reconstruct(
    boundary: &RepositoryBoundary,
    id: &str,
) -> Result<ProcessedArtifact, ProcessingError> {
    let root = native_input::content_root(boundary, id)
        .map_err(|error| ProcessingError::Input(error.to_string()))?;
    let mut files = Vec::new();
    collect(&root, Path::new(""), &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(ProcessedArtifact {
        format_version: FORMAT_VERSION,
        input_id: id.into(),
        files,
    })
}

fn collect(
    root: &Path,
    relative: &Path,
    files: &mut Vec<ProcessedFile>,
) -> Result<(), ProcessingError> {
    let mut entries = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(ProcessingError::Artifact(format!(
                "symbolic link in snapshot: {}",
                entry.path().display()
            )));
        }
        let path = relative.join(entry.file_name());
        if kind.is_dir() {
            collect(&entry.path(), &path, files)?;
        } else if kind.is_file() {
            let bytes = fs::read(entry.path())?;
            files.push(ProcessedFile {
                path: path.to_string_lossy().replace('\\', "/"),
                sha256: sha256(&bytes),
                bytes: bytes.len() as u64,
            });
        } else {
            return Err(ProcessingError::Artifact(format!(
                "unsupported snapshot entry: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn artifact_path(boundary: &RepositoryBoundary, id: &str) -> PathBuf {
    boundary
        .directory
        .join(RUNTIME_DIRECTORY)
        .join("processed")
        .join(format!("{id}.json"))
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, ProcessingError> {
    serde_json::to_vec_pretty(value).map_err(|error| ProcessingError::Artifact(error.to_string()))
}
