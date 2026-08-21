//! Exact-destination HTTPS boundary backed by Curl on Linux.

use crate::canonical;
use crate::result::Capability;
use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
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
    #[error("https.timeout")]
    Timeout,
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
        "-q",
        "--fail-with-body",
        "--silent",
        "--show-error",
        "--noproxy",
        "*",
        "--no-netrc",
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
    let stdin = child
        .stdin
        .take()
        .ok_or(HttpsError::Execution(std::io::Error::other(
            "curl stdin unavailable",
        )))?;
    if let Err(error) = (&stdin).write_all(&spec.body)
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        return Err(HttpsError::Execution(error));
    }
    drop(stdin);
    let mut stdout = child
        .stdout
        .take()
        .ok_or(HttpsError::Execution(std::io::Error::other(
            "curl stdout unavailable",
        )))?;
    let response_limit = spec.response_limit;
    let reader = std::thread::spawn(move || -> Result<(Vec<u8>, bool), std::io::Error> {
        let mut response = Vec::with_capacity(response_limit.min(8192));
        let mut buffer = [0_u8; 8192];
        let mut overflow = false;
        loop {
            let read = stdout.read(&mut buffer)?;
            if read == 0 {
                return Ok((response, overflow));
            }
            let available = response_limit.saturating_sub(response.len());
            response.extend_from_slice(&buffer[..read.min(available)]);
            overflow |= read > available;
        }
    });
    let deadline = Instant::now() + spec.timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            let (response, overflow) = reader.join().map_err(|_| {
                HttpsError::Execution(std::io::Error::other("curl output reader panicked"))
            })??;
            if overflow {
                return Err(HttpsError::Overflow);
            }
            return if status.success() {
                Ok(response)
            } else {
                Err(HttpsError::Nonzero)
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(HttpsError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn validate(spec: &HttpsSpec) -> Result<(), HttpsError> {
    let authority = spec
        .url
        .strip_prefix("https://")
        .and_then(|value| value.split(['/', '?', '#']).next());
    if authority != Some(spec.hostname.as_str())
        || spec.url.contains('@')
        || spec.url.contains('#')
        || spec.url.contains('?')
        || spec.hostname.is_empty()
        || spec.timeout.is_zero()
        || spec.response_limit == 0
        || !spec.ca_file.is_file()
        || canonical::sha256(&spec.body) != spec.body_digest
        || spec.headers.iter().any(|header| {
            header.contains('\r')
                || header.contains('\n')
                || header.to_ascii_lowercase().starts_with("proxy-")
        })
    {
        return Err(HttpsError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};
    use std::thread;
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
            "https://service.example.attacker/v1",
            "https://service.example:8443/v1",
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

    #[test]
    fn invokes_a_locally_trusted_exact_tls_destination_when_tools_are_available() {
        let curl = Path::new("/usr/bin/curl");
        let openssl = Path::new("/usr/bin/openssl");
        if !curl.is_file() || !openssl.is_file() {
            return;
        }
        let directory = tempdir().unwrap();
        let certificate = directory.path().join("certificate.pem");
        let key = directory.path().join("key.pem");
        let generated = Command::new(openssl)
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                key.to_str().unwrap(),
                "-out",
                certificate.to_str().unwrap(),
                "-subj",
                "/CN=service.example",
                "-addext",
                "subjectAltName=DNS:service.example",
                "-days",
                "1",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(generated.success());
        let mut server = Command::new(openssl)
            .args([
                "s_server",
                "-accept",
                "127.0.0.1:443",
                "-cert",
                certificate.to_str().unwrap(),
                "-key",
                key.to_str().unwrap(),
                "-www",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let ready = (0..50).any(|_| {
            if TcpStream::connect("127.0.0.1:443").is_ok() {
                true
            } else {
                thread::sleep(Duration::from_millis(10));
                false
            }
        });
        if !ready {
            let _ = server.kill();
            let _ = server.wait();
            return;
        }
        let body = Vec::new();
        let response = invoke(
            curl,
            &HttpsSpec {
                url: "https://service.example/".into(),
                method: "GET".into(),
                headers: vec![],
                body_digest: canonical::sha256(&body),
                body,
                hostname: "service.example".into(),
                address: "127.0.0.1".parse().unwrap(),
                timeout: Duration::from_secs(3),
                response_limit: 64 * 1024,
                ca_file: certificate,
            },
        )
        .unwrap();
        let _ = server.kill();
        let _ = server.wait();
        assert!(String::from_utf8_lossy(&response).contains("s_server"));
    }

    #[test]
    #[cfg(unix)]
    fn nonzero_curl_exit_is_reported_as_a_boundary_failure() {
        let directory = tempdir().unwrap();
        let fake_curl = directory.path().join("curl-fail");
        std::fs::write(&fake_curl, "#!/bin/sh\nexit 22\n").unwrap();
        std::fs::set_permissions(&fake_curl, std::fs::Permissions::from_mode(0o700)).unwrap();
        let ca = directory.path().join("ca.pem");
        std::fs::write(&ca, "fixture").unwrap();
        let body = Vec::new();
        let spec = HttpsSpec {
            url: "https://service.example/".into(),
            method: "GET".into(),
            headers: vec![],
            body_digest: canonical::sha256(&body),
            body,
            hostname: "service.example".into(),
            address: "127.0.0.1".parse().unwrap(),
            timeout: Duration::from_secs(1),
            response_limit: 1024,
            ca_file: ca,
        };
        assert!(matches!(invoke(fake_curl, &spec), Err(HttpsError::Nonzero)));
    }

    #[test]
    #[cfg(unix)]
    fn broker_enforces_timeout_when_curl_ignores_its_flags() {
        let directory = tempdir().unwrap();
        let fake_curl = directory.path().join("curl-sleep");
        std::fs::write(&fake_curl, "#!/bin/sh\nsleep 2\n").unwrap();
        std::fs::set_permissions(&fake_curl, std::fs::Permissions::from_mode(0o700)).unwrap();
        let ca = directory.path().join("ca.pem");
        std::fs::write(&ca, "fixture").unwrap();
        let body = Vec::new();
        let spec = HttpsSpec {
            url: "https://service.example/".into(),
            method: "GET".into(),
            headers: vec![],
            body_digest: canonical::sha256(&body),
            body,
            hostname: "service.example".into(),
            address: "127.0.0.1".parse().unwrap(),
            timeout: Duration::from_millis(20),
            response_limit: 1024,
            ca_file: ca,
        };
        assert!(matches!(invoke(fake_curl, &spec), Err(HttpsError::Timeout)));
    }
}
