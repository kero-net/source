use kero_core::boundary::https_service::{self, HttpsError, HttpsSpec};
use kero_core::canonical;
use std::time::Duration;
use tempfile::tempdir;

fn spec(ca_file: std::path::PathBuf) -> HttpsSpec {
    let body = b"fixture request".to_vec();
    HttpsSpec {
        url: "https://service.example/v1".into(),
        method: "POST".into(),
        headers: vec!["content-type: text/plain".into()],
        body_digest: canonical::sha256(&body),
        body,
        hostname: "service.example".into(),
        address: "127.0.0.1".parse().unwrap(),
        timeout: Duration::from_secs(1),
        response_limit: 1024,
        ca_file,
    }
}

#[test]
fn unavailable_curl_is_a_capability_failure() {
    let directory = tempdir().unwrap();
    let ca = directory.path().join("ca.pem");
    std::fs::write(&ca, "fixture ca").unwrap();
    assert!(matches!(
        https_service::invoke("/definitely-not-kero-curl", &spec(ca)),
        Err(HttpsError::Unavailable)
    ));
}

#[test]
fn proxy_redirect_and_ambiguous_url_inputs_fail_closed() {
    let directory = tempdir().unwrap();
    let ca = directory.path().join("ca.pem");
    std::fs::write(&ca, "fixture ca").unwrap();
    for url in [
        "https://service.example/v1?redirect=1",
        "https://service.example/v1#fragment",
        "https://user@service.example/v1",
    ] {
        assert!(matches!(
            https_service::invoke(
                "/definitely-not-kero-curl",
                &HttpsSpec {
                    url: url.into(),
                    ..spec(ca.clone())
                },
            ),
            Err(HttpsError::Invalid)
        ));
    }
    assert!(matches!(
        https_service::invoke(
            "/definitely-not-kero-curl",
            &HttpsSpec {
                headers: vec!["Proxy-Authorization: secret".into()],
                ..spec(ca)
            },
        ),
        Err(HttpsError::Invalid)
    ));
}
