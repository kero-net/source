//! Exact-destination HTTPS boundary backed by Curl.

use crate::canonical;
use crate::result::Capability;
use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use thiserror::Error;

pub const BOUNDARY: &str = "broker.https-service/v1";

#[derive(Clone, Debug)]
pub struct HttpsSpec {
    pub url: String,
    pub method: String,
    pub headers: Vec<String>,
    pub body: Vec<u8>,
    pub body_digest: String,
    pub hostname: String,
    pub address: IpAddr,
    pub timeout: Duration,
    pub response_limit: usize,
    pub ca_file: PathBuf,
}
#[derive(Debug, Error)]
pub enum HttpsError {
    #[error("https.request-invalid")]
    Invalid,
    #[error("https.capability-unavailable")]
    Unavailable,
    #[error("https.execution: {0}")]
    Execution(#[from] std::io::Error),
    #[error("https.nonzero")]
    Nonzero,
    #[error("https.response-overflow")]
    Overflow,
}

pub fn capability(binary: &Path) -> Capability {
    if binary.is_file() {
        Capability::Can
    } else {
        Capability::Cannot
    }
}

pub fn invoke(binary: impl Into<PathBuf>, spec: &HttpsSpec) -> Result<Vec<u8>, HttpsError> {
    validate(spec)?;
    let binary = binary.into();
    if capability(&binary) != Capability::Can {
        return Err(HttpsError::Unavailable);
    }
    let resolve = format!("{}:443:{}", spec.hostname, spec.address);
    let mut command = Command::new(binary);
    command.env_clear().args([
        "--fail-with-body",
        "--silent",
        "--show-error",
        "--noproxy",
        "*",
        "--proxy",
        "",
        "--max-redirs",
        "0",
        "--proto",
        "=https",
        "--connect-timeout",
        &spec.timeout.as_secs().to_string(),
        "--max-time",
        &spec.timeout.as_secs().to_string(),
        "--cacert",
        spec.ca_file.to_str().ok_or(HttpsError::Invalid)?,
        "--resolve",
        &resolve,
        "-X",
        &spec.method,
    ]);
    for header in &spec.headers {
        command.args(["-H", header]);
    }
    let mut child = command
        .arg("--data-binary")
        .arg("@-")
        .arg(&spec.url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or(HttpsError::Execution(std::io::Error::other(
            "curl stdin unavailable",
        )))?
        .write_all(&spec.body)?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or(HttpsError::Execution(std::io::Error::other(
            "curl stdout unavailable",
        )))?;
    let mut response = Vec::with_capacity(spec.response_limit.min(8192));
    let mut buffer = [0_u8; 8192];
    loop {
        let read = stdout.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if response.len().saturating_add(read) > spec.response_limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HttpsError::Overflow);
        }
        response.extend_from_slice(&buffer[..read]);
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(HttpsError::Nonzero);
    }
    Ok(response)
}

fn validate(spec: &HttpsSpec) -> Result<(), HttpsError> {
    if !spec.url.starts_with(&format!("https://{}", spec.hostname))
        || spec.url.contains('@')
        || spec.url.contains('#')
        || spec.hostname.is_empty()
        || spec.timeout.is_zero()
        || spec.response_limit == 0
        || !spec.ca_file.is_file()
        || canonical::sha256(&spec.body) != spec.body_digest
        || spec
            .headers
            .iter()
            .any(|header| header.contains('\r') || header.contains('\n'))
    {
        return Err(HttpsError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_non_https_userinfo_fragments_and_body_tamper() {
        let directory = tempdir().unwrap();
        let ca = directory.path().join("ca.pem");
        std::fs::write(&ca, "fixture").unwrap();
        let body = b"fixture".to_vec();
        let base = HttpsSpec {
            url: "https://service.example/v1".into(),
            method: "POST".into(),
            headers: vec!["content-type: text/plain".into()],
            body: body.clone(),
            body_digest: canonical::sha256(&body),
            hostname: "service.example".into(),
            address: "127.0.0.1".parse().unwrap(),
            timeout: Duration::from_secs(1),
            response_limit: 1024,
            ca_file: ca,
        };
        validate(&base).unwrap();
        for url in [
            "http://service.example/v1",
            "https://user@service.example/v1",
            "https://service.example/v1#fragment",
        ] {
            assert!(
                validate(&HttpsSpec {
                    url: url.into(),
                    ..base.clone()
                })
                .is_err(),
                "accepted {url}"
            );
        }
        assert!(
            validate(&HttpsSpec {
                body_digest: "sha256:wrong".into(),
                ..base
            })
            .is_err()
        );
    }
}
