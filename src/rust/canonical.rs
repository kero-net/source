use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CanonicalError {
    #[error("JCS serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub fn canonicalize<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalError> {
    Ok(serde_jcs::to_vec(value)?)
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

pub fn digest<T: Serialize>(value: &T) -> Result<String, CanonicalError> {
    Ok(sha256(&canonicalize(value)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn canonicalizes_equivalent_json_identically() {
        let left: Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let right: Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();

        assert_eq!(canonicalize(&left).unwrap(), canonicalize(&right).unwrap());
        assert_eq!(digest(&left).unwrap(), digest(&right).unwrap());
    }

    #[test]
    fn canonicalization_preserves_values() {
        let value: Value =
            serde_json::from_str(r#"{"message":"arbitrary text","number":1,"flag":true}"#).unwrap();
        let canonical = canonicalize(&value).unwrap();
        let decoded: Value = serde_json::from_slice(&canonical).unwrap();

        assert_eq!(decoded, value);
    }
}
