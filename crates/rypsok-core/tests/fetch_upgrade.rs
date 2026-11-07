//! REQ-FETCH-010 (RFC 020, TST-5): an `http` URL is retried over `https`
//! when plain HTTP is not allowed, and the fetch states the `https` URL it
//! read.
//!
//! The site is a TLS server on loopback with a certificate made here, at
//! run time. The fetch client trusts the system's roots; the test adds its
//! own through the standard `SSL_CERT_FILE` variable, which the system root
//! loader reads on Linux. A process cannot set its own environment without
//! `unsafe` (forbidden in this workspace), so the test runs its body in a
//! child process of itself that is started with the variable set: no seam
//! in the product, no root store of its own in the client.

#![cfg(target_os = "linux")]

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use rypsok_core::CancellationToken;
use rypsok_core::config::SecretDetection;
use rypsok_core::egress::policy::{EgressPolicy, parse_rule};
use rypsok_core::egress::{AgentClientSettings, ReturnedUrls};
use rypsok_core::fetch::{self, FetchService};
use rypsok_core::limits::Limits;
use rypsok_core::model::{CallerId, FetchRequest, Outcome};
use rypsok_core::telemetry::Counters;

const PAGE: &str = "<html><head><title>Over TLS</title></head><body><main><p>The page was served over https and it is long enough to be the main part of the document for the extractor.</p></main></body></html>";

/// The parent: makes a certificate, starts the child with it trusted and
/// reads its verdict.
#[test]
fn req_fetch_010_an_http_url_is_fetched_over_https_when_the_site_speaks_it() {
    let dir = std::env::temp_dir().join(format!("rypsok-upgrade-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    std::fs::write(dir.join("ca.pem"), certified.cert.pem()).unwrap();
    std::fs::write(dir.join("cert.der"), certified.cert.der().as_ref()).unwrap();
    std::fs::write(dir.join("key.der"), certified.signing_key.serialize_der()).unwrap();

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "upgrade_child", "--ignored", "--nocapture"])
        .env("SSL_CERT_FILE", dir.join("ca.pem"))
        .env("SSL_CERT_DIR", &dir)
        .env("RYPSOK_UPGRADE_DIR", &dir)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        output.status.success() && text.contains("1 passed"),
        "the child failed:\n{text}"
    );
}

/// The child: the real test. Ignored, so that it runs only when the parent
/// asks for it with the certificate trusted.
#[test]
#[ignore = "run by the parent test with a trusted certificate"]
fn upgrade_child() {
    let dir = std::path::PathBuf::from(
        std::env::var("RYPSOK_UPGRADE_DIR").expect("started by the parent test"),
    );
    let cert =
        rustls::pki_types::CertificateDer::from(std::fs::read(dir.join("cert.der")).unwrap());
    let key =
        rustls::pki_types::PrivateKeyDer::try_from(std::fs::read(dir.join("key.der")).unwrap())
            .unwrap();
    rypsok_core::egress::ensure_crypto_provider();
    let config = Arc::new(
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap(),
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let config = Arc::clone(&config);
            std::thread::spawn(move || serve(stream, config));
        }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        // Plain HTTP is not allowed: the URL is retried over https.
        let policy = Arc::new(EgressPolicy::new(
            vec![parse_rule("127.0.0.1/32").unwrap()],
            Vec::new(),
            false,
            vec![80, 443, port],
        ));
        let limits = Limits::default();
        let settings = AgentClientSettings {
            user_agent: "rypsok/test (+https://example.org)".into(),
            connect_timeout: Duration::from_secs(2),
            timeout: Duration::from_secs(5),
            allow_http: false,
        };
        let service = FetchService::new(&settings, Arc::clone(&policy), &limits).unwrap();
        let counters = Counters::new();
        let returned = ReturnedUrls::new(10);
        let caller = CallerId::local();
        for robots_txt in [true, false] {
            let pipeline = fetch::Pipeline {
                limits: &limits,
                policy: &policy,
                service: &service,
                returned: &returned,
                strict_mode: false,
                robots_txt,
                follow_cross_host_redirects: false,
                secret_detection: SecretDetection::Refuse,
                counters: &counters,
                caller: &caller,
            };
            let response = fetch::run(
                &pipeline,
                FetchRequest::new(format!("http://127.0.0.1:{port}/page")),
                CancellationToken::new(),
            )
            .await
            .unwrap_or_else(|e| panic!("robots_txt {robots_txt}: {e:?}"));
            assert_eq!(response.outcome, Outcome::Ok);
            assert_eq!(
                response.url,
                format!("https://127.0.0.1:{port}/page"),
                "the answer states the https URL it read (robots_txt {robots_txt})"
            );
            assert_eq!(response.title, "Over TLS");
            assert!(response.content.contains("served over https"));
        }
    });
}

/// One connection of the TLS site: `/robots.txt` is 404, anything else is
/// the page.
fn serve(stream: std::net::TcpStream, config: Arc<rustls::ServerConfig>) {
    let Ok(connection) = rustls::ServerConnection::new(config) else {
        return;
    };
    let mut tls = rustls::StreamOwned::new(connection, stream);
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match tls.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&buffer[..n]),
        }
    }
    let first = String::from_utf8_lossy(&request);
    let answer = if first.starts_with("GET /robots.txt") {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
    } else {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{PAGE}",
            PAGE.len()
        )
    };
    let _ = tls.write_all(answer.as_bytes());
    let _ = tls.flush();
    tls.conn.send_close_notify();
    let _ = tls.flush();
}
