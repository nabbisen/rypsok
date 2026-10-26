//! Tests of the fetch module: the extraction corpus (REQ-FETCH-002,
//! REQ-PAGE-002, REQ-TEST-006) and the pipeline against `wiremock`
//! (REQ-FETCH-001 to -011, REQ-PAGE-003 to -008, REQ-SEC-012). Every test
//! names the requirement it verifies.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::egress::policy::parse_rule;
use crate::model::ErrorKind;

#[derive(Debug, serde::Deserialize)]
struct Expected {
    title: String,
    must_contain: Vec<String>,
    must_not_contain: Vec<String>,
    notices: Vec<String>,
    #[serde(default)]
    media_type: Option<String>,
}

fn corpus_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/pages")
}

/// The pure stages plus cleaning, as the pipeline applies them.
fn render_and_clean(bytes: &[u8], media_type: &str) -> (String, String, Vec<&'static str>) {
    let media = if media_type.starts_with("text/plain") || media_type.starts_with("text/markdown") {
        Media::Text
    } else {
        Media::Html
    };
    let document = render_document(
        bytes,
        media,
        charset::charset_parameter(media_type),
        &Limits::default(),
    );
    let mut notices = Vec::new();
    let title = boundary::clean_text(&document.title, 150).text;
    let cleaned = boundary::clean_content(&document.content, 0, 1_000_000).unwrap();
    if cleaned.removed > 0 {
        notices.push("characters_removed");
    }
    if document.partial {
        notices.push("extraction_partial");
    }
    (title, cleaned.text, notices)
}

/// REQ-FETCH-002, REQ-PAGE-002 and REQ-CTX-005: every fixture of the
/// corpus yields its expected title, its main text in order, none of its
/// chrome, hidden or script text, and the expected notices.
#[test]
fn req_fetch_002_corpus_main_text_present_and_chrome_absent() {
    let mut dirs: Vec<_> = std::fs::read_dir(corpus_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    dirs.sort();
    assert_eq!(dirs.len(), 15, "the corpus of RFC 012");
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let expected: Expected =
            toml::from_str(&std::fs::read_to_string(dir.join("expected.toml")).unwrap()).unwrap();
        let (bytes, media_type) = if dir.join("page.txt").exists() {
            (std::fs::read(dir.join("page.txt")).unwrap(), "text/plain")
        } else {
            (std::fs::read(dir.join("page.html")).unwrap(), "text/html")
        };
        let media_type = expected.media_type.as_deref().unwrap_or(media_type);
        let (title, content, notices) = render_and_clean(&bytes, media_type);
        assert_eq!(title, expected.title, "{name}: title");
        let mut cursor = 0;
        for phrase in &expected.must_contain {
            let found = content[cursor..].find(phrase.as_str());
            assert!(
                found.is_some(),
                "{name}: {phrase:?} missing or out of order in:\n{content}"
            );
            cursor += found.unwrap() + phrase.len();
        }
        for phrase in &expected.must_not_contain {
            assert!(
                !content.contains(phrase.as_str()),
                "{name}: {phrase:?} present in:\n{content}"
            );
        }
        let mut notices: Vec<String> = notices.iter().map(|n| n.to_string()).collect();
        notices.sort();
        let mut expected_notices = expected.notices.clone();
        expected_notices.sort();
        assert_eq!(notices, expected_notices, "{name}: notices");
    }
}

/// REQ-TEST-006 and THR-12: the hostile fixtures (deep nesting, node
/// bomb) are rendered on a thread with a small stack; the call ends, the
/// thread lives.
#[test]
fn req_test_006_hostile_documents_end_the_call_not_the_process() {
    for fixture in ["11-deep-nesting", "12-node-bomb"] {
        let bytes = std::fs::read(corpus_dir().join(fixture).join("page.html")).unwrap();
        let handle = std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(move || render_and_clean(&bytes, "text/html"))
            .unwrap();
        let (_, content, notices) = handle.join().unwrap();
        assert!(notices.contains(&"extraction_partial"), "{fixture}");
        assert!(!content.is_empty());
    }
}

// --- the pipeline against a loopback server ---------------------------------

/// A lookup that answers loopback for every name, so that a name can
/// stand for the test server; the policy still judges the address.
struct Loopback;

impl crate::egress::resolver::Lookup for Loopback {
    fn lookup(
        &self,
        _name: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<Vec<std::net::IpAddr>>> + Send>,
    > {
        Box::pin(async { Ok(vec![std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)]) })
    }
}

struct Harness {
    server: MockServer,
    policy: Arc<EgressPolicy>,
    service: FetchService,
    returned: ReturnedUrls,
    limits: Limits,
    counters: Counters,
    strict_mode: bool,
    robots_txt: bool,
    follow_cross_host_redirects: bool,
}

async fn harness() -> Harness {
    harness_with(Limits::default()).await
}

async fn harness_with(limits: Limits) -> Harness {
    let server = MockServer::start().await;
    let port = Url::parse(&server.uri()).unwrap().port().unwrap();
    // The production policy with the one rule a test needs (REQ-TEST-005).
    let policy = Arc::new(EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        true,
        vec![80, 443, port],
    ));
    let settings = AgentClientSettings {
        user_agent: "rypsok/test (+https://example.org)".into(),
        connect_timeout: Duration::from_millis(limits.fetch_connect_timeout_ms.into()),
        timeout: Duration::from_millis(limits.fetch_deadline_ms.into()),
        allow_http: true,
    };
    let service =
        FetchService::with_lookup(&settings, Arc::clone(&policy), &limits, Loopback).unwrap();
    Harness {
        server,
        policy,
        service,
        returned: ReturnedUrls::new(100),
        limits,
        counters: Counters::new(),
        strict_mode: false,
        robots_txt: true,
        follow_cross_host_redirects: false,
    }
}

impl Harness {
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.server.uri())
    }

    async fn fetch(&self, url: &str) -> Result<FetchResponse, ToolError> {
        self.fetch_req(FetchRequest {
            url: url.to_owned(),
            max_chars: None,
            start_char: None,
        })
        .await
    }

    async fn fetch_req(&self, req: FetchRequest) -> Result<FetchResponse, ToolError> {
        let pipeline = Pipeline {
            limits: &self.limits,
            policy: &self.policy,
            service: &self.service,
            returned: &self.returned,
            strict_mode: self.strict_mode,
            robots_txt: self.robots_txt,
            follow_cross_host_redirects: self.follow_cross_host_redirects,
            secret_detection: SecretDetection::Refuse,
            counters: &self.counters,
        };
        match run(&pipeline, req, CancellationToken::new()).await {
            Ok(response) => Ok(response),
            Err(CallError::Tool(error)) => Err(error),
            Err(CallError::Cancelled) => panic!("not cancelled"),
        }
    }
}

fn page(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.as_bytes().to_vec(), "text/html; charset=utf-8")
}

const ARTICLE: &str = "<html><head><title>Hello</title></head><body><nav>menu</nav><main><h1>Hello</h1><p>The main content of the page is here and it is long enough to be the candidate of the extractor, with a few more words to make sure of it.</p><p>Second paragraph.</p></main></body></html>";

/// REQ-FETCH-001, REQ-FETCH-004 and REQ-PAGE-001: a page is read through
/// the whole pipeline into the structured response; robots.txt absent
/// (404) allows it; the response states the URL fetched.
#[tokio::test]
async fn req_fetch_001_page_is_read_into_the_response() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/article"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    let response = h.fetch(&h.url("/article")).await.unwrap();
    assert_eq!(response.outcome, Outcome::Ok);
    assert_eq!(response.title, "Hello");
    assert!(response.content.starts_with("# Hello\n\nThe main content"));
    assert!(!response.content.contains("menu"));
    assert!(!response.truncated);
    assert_eq!(response.next_start_char, None);
    assert_eq!(response.url, h.url("/article"));
    assert!(response.notices.is_empty(), "{:?}", response.notices);
    assert_eq!(h.counters.calls_of("web_fetch", "ok"), 1);
}

/// REQ-FETCH-003 and REQ-FETCH-005: the window and the continuation; a
/// start beyond the end is refused stating the length; `max_chars` above
/// the ceiling names the ceiling.
#[tokio::test]
async fn req_fetch_005_window_and_continuation() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/a"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    let first = h
        .fetch_req(FetchRequest {
            url: h.url("/a"),
            max_chars: Some(500),
            start_char: None,
        })
        .await
        .unwrap();
    assert!(
        !first.truncated,
        "the article is shorter than 500 characters"
    );
    let total = first.content.chars().count() as u32;
    let beyond = h
        .fetch_req(FetchRequest {
            url: h.url("/a"),
            max_chars: Some(500),
            start_char: Some(total),
        })
        .await
        .unwrap_err();
    assert_eq!(beyond.kind, ErrorKind::InvalidRequest);
    assert_eq!(beyond.detail.parameter.as_deref(), Some("start_char"));
    assert_eq!(beyond.detail.max, Some(total - 1));
    assert!(beyond.message.contains(&total.to_string()));
    let too_many = h
        .fetch_req(FetchRequest {
            url: h.url("/a"),
            max_chars: Some(h.limits.fetch_max_chars_ceiling + 1),
            start_char: None,
        })
        .await
        .unwrap_err();
    assert_eq!(too_many.detail.max, Some(h.limits.fetch_max_chars_ceiling));
    // A continuation from the middle picks up exactly where the first
    // window of that size ends.
    let small = h
        .fetch_req(FetchRequest {
            url: h.url("/a"),
            max_chars: Some(500),
            start_char: Some(20),
        })
        .await
        .unwrap();
    let whole: String = first.content.chars().skip(20).collect();
    assert_eq!(small.content, whole);
}

/// REQ-FETCH-006, REQ-SEC-008 and D-34: same-host redirects are followed
/// up to the limit and the response states the final URL; a cross-host
/// target that passes the policy is `redirected` with its location; one
/// that fails the policy is `denied` without disclosure; the operator may
/// follow cross-host redirects.
#[tokio::test]
async fn req_fetch_006_redirect_table() {
    let mut h = harness().await;
    Mock::given(method("GET"))
        .and(path("/start"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/next"))
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/next"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    let response = h.fetch(&h.url("/start")).await.unwrap();
    assert_eq!(response.url, h.url("/next"));

    Mock::given(method("GET"))
        .and(path("/elsewhere"))
        .respond_with(
            ResponseTemplate::new(301).insert_header("location", "https://example.org/moved"),
        )
        .mount(&h.server)
        .await;
    let redirected = h.fetch(&h.url("/elsewhere")).await.unwrap_err();
    assert_eq!(redirected.kind, ErrorKind::Redirected);
    assert_eq!(
        redirected.detail.location.as_deref(),
        Some("https://example.org/moved")
    );

    Mock::given(method("GET"))
        .and(path("/internal"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "http://169.254.169.254/latest"),
        )
        .mount(&h.server)
        .await;
    let denied = h.fetch(&h.url("/internal")).await.unwrap_err();
    assert_eq!(denied.kind, ErrorKind::Denied);
    assert_eq!(denied.detail.reason, Some(DetailReason::Egress));
    assert_eq!(denied.detail.location, None, "the target is not disclosed");
    assert!(!denied.message.contains("169.254"));

    Mock::given(method("GET"))
        .and(path("/loop"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/loop"))
        .mount(&h.server)
        .await;
    let looped = h.fetch(&h.url("/loop")).await.unwrap_err();
    assert_eq!(
        looped.detail.reason,
        Some(DetailReason::Network),
        "more than LIM-F7 hops"
    );

    // The operator switch: a cross-host target (a name that resolves to the
    // same loopback server) is followed and the response says so.
    h.follow_cross_host_redirects = true;
    let other_host = h.url("/next").replace("127.0.0.1", "mirror.example");
    Mock::given(method("GET"))
        .and(path("/cross"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", other_host.as_str()))
        .mount(&h.server)
        .await;
    let followed = h.fetch(&h.url("/cross")).await.unwrap();
    assert_eq!(
        followed.url, other_host,
        "the response states the URL fetched"
    );
}

/// REQ-FETCH-011: a 4xx or 5xx ends the call with `unavailable` and the
/// status; the body is never returned.
#[tokio::test]
async fn req_fetch_011_error_statuses_are_never_content() {
    let h = harness().await;
    for status in [404u16, 500] {
        Mock::given(method("GET"))
            .and(path(format!("/s{status}")))
            .respond_with(ResponseTemplate::new(status).set_body_string("SECRET-BODY-TEXT"))
            .mount(&h.server)
            .await;
        let error = h.fetch(&h.url(&format!("/s{status}"))).await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Unavailable);
        assert_eq!(error.detail.reason, Some(DetailReason::HttpStatus));
        assert_eq!(error.detail.status, Some(status));
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("SECRET-BODY-TEXT")
        );
    }
}

/// REQ-FETCH-008 and REQ-PAGE-005: an inextractable type names its
/// essence, bounded; a missing type is sniffed; text and markdown are
/// returned as they are.
#[tokio::test]
async fn req_fetch_008_media_types() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"%PDF-1.4".to_vec(), "application/pdf; name=x"),
        )
        .mount(&h.server)
        .await;
    let error = h.fetch(&h.url("/pdf")).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::UnsupportedContent);
    assert_eq!(error.detail.reason, Some(DetailReason::MediaType));
    assert_eq!(error.detail.media_type.as_deref(), Some("application/pdf"));

    Mock::given(method("GET"))
        .and(path("/untyped"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(ARTICLE.as_bytes().to_vec()))
        .mount(&h.server)
        .await;
    let sniffed = h.fetch(&h.url("/untyped")).await.unwrap();
    assert_eq!(sniffed.title, "Hello");

    Mock::given(method("GET"))
        .and(path("/plain"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"just <b>text</b>\n\nhere".to_vec(), "text/plain"),
        )
        .mount(&h.server)
        .await;
    let text = h.fetch(&h.url("/plain")).await.unwrap();
    assert_eq!(text.content, "just <b>text</b>\n\nhere");
    assert_eq!(text.title, "");
}

/// REQ-PAGE-003, REQ-PAGE-004 and REQ-SEC-013: a body larger than LIM-F3
/// after decompression ends the call with `too_large` at the bound, whether
/// declared or streamed.
#[tokio::test]
async fn req_page_003_byte_limit_while_streaming() {
    let h = harness_with(Limits {
        fetch_max_bytes: 65_536,
        ..Limits::default()
    })
    .await;
    let big = vec![b'a'; 70_000];
    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(big, "text/plain"))
        .mount(&h.server)
        .await;
    let error = h.fetch(&h.url("/big")).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::TooLarge);
    assert_eq!(error.detail.limit_bytes, Some(65_536));

    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&vec![b'b'; 400_000]).unwrap();
    let gz = encoder.finish().unwrap();
    assert!(gz.len() < 65_536, "compressed far below the bound");
    Mock::given(method("GET"))
        .and(path("/gz"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_raw(gz, "text/plain"),
        )
        .mount(&h.server)
        .await;
    let error = h.fetch(&h.url("/gz")).await.unwrap_err();
    assert_eq!(
        error.kind,
        ErrorKind::TooLarge,
        "the bound applies after decompression"
    );
}

/// REQ-CMP-005: a robots.txt that disallows the path ends the call with
/// `denied` / `robots_txt`; the operator may disable the check.
#[tokio::test]
async fn req_cmp_005_robots_txt_is_honored_unless_disabled() {
    let mut h = harness().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /private\n"),
        )
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/private/page"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    let error = h.fetch(&h.url("/private/page")).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Denied);
    assert_eq!(error.detail.reason, Some(DetailReason::RobotsTxt));
    assert_eq!(h.counters.refusals_of("web_fetch", "robots_txt"), 1);
    h.robots_txt = false;
    assert!(h.fetch(&h.url("/private/page")).await.is_ok());
}

/// REQ-SEC-021: in strict mode only URLs returned earlier (by comparison
/// key) or allowed by rule are fetched.
#[tokio::test]
async fn req_sec_021_strict_mode_admits_returned_urls_only() {
    let mut h = harness().await;
    h.strict_mode = true;
    Mock::given(method("GET"))
        .and(path("/known"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    let named = |p: &str| h.url(p).replace("127.0.0.1", "mirror.example");
    let error = h.fetch(&named("/known")).await.unwrap_err();
    assert_eq!(error.detail.reason, Some(DetailReason::StrictMode));
    // Recorded as the search pipeline records it: the returned form's key.
    let returned = normalize::returned_url(&named("/known?utm_source=x"), 2048).unwrap();
    h.returned.record(&comparison_key(&returned));
    let known = h.fetch(&named("/known")).await;
    assert!(known.is_ok(), "same resource by key: {known:?}");
    assert_eq!(
        h.fetch(&named("/unknown")).await.unwrap_err().detail.reason,
        Some(DetailReason::StrictMode)
    );
    assert!(
        h.fetch(&h.url("/known")).await.is_ok(),
        "an address the operator allowed by rule needs no prior result"
    );
}

/// REQ-FETCH-007, REQ-FETCH-010 and REQ-PRIV-002: credentials in the URL,
/// a scheme that is not http(s), a secret in the URL and an internal
/// destination are refused before any request.
#[tokio::test]
async fn req_fetch_007_refusals_before_any_request() {
    let h = harness().await;
    Mock::given(method("GET"))
        .respond_with(page(ARTICLE))
        .expect(0)
        .mount(&h.server)
        .await;
    let cases = [
        // The privacy scan runs before the policy (REQ-PAGE-001): userinfo
        // with a password is a credential finding first.
        (
            format!("http://user:pw@127.0.0.1:{}/x", h.server.address().port()),
            DetailReason::SecretInUrl,
        ),
        (
            format!("http://user@127.0.0.1:{}/x", h.server.address().port()),
            DetailReason::CredentialInUrl,
        ),
        ("ftp://example.org/x".to_owned(), DetailReason::Scheme),
        (
            "https://example.org/?token=AKIAIOSFODNN7EXAMPLE".to_owned(),
            DetailReason::SecretInUrl,
        ),
        ("https://10.0.0.1/x".to_owned(), DetailReason::Egress),
        ("https://localhost/x".to_owned(), DetailReason::Egress),
    ];
    for (url, reason) in cases {
        let error = h.fetch(&url).await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Denied, "{url}");
        assert_eq!(error.detail.reason, Some(reason), "{url}");
    }
    let bad = h.fetch("not a url").await.unwrap_err();
    assert_eq!(bad.kind, ErrorKind::InvalidRequest);
    assert_eq!(bad.detail.parameter.as_deref(), Some("url"));
}

/// REQ-SEC-012: fetches beyond LIM-C3 wait for a permit; one that cannot
/// have it before the deadline ends with `busy`; the per-host permit
/// serializes requests to one host.
#[tokio::test]
async fn req_sec_012_concurrency_limits() {
    let h = harness_with(Limits {
        fetches_in_flight: 1,
        fetch_deadline_ms: 500,
        ..Limits::default()
    })
    .await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    // Another call holds the only permit past this call's deadline.
    let held = Arc::clone(&h.service.global).acquire_owned().await.unwrap();
    let started = std::time::Instant::now();
    let error = h.fetch(&h.url("/slow")).await.unwrap_err();
    assert_eq!(error.detail.reason, Some(DetailReason::Busy));
    assert!(started.elapsed() < Duration::from_millis(900));
    drop(held);
    assert!(h.fetch(&h.url("/slow")).await.is_ok(), "the permit is back");

    let h = harness_with(Limits {
        fetches_in_flight: 4,
        ..Limits::default()
    })
    .await;
    Mock::given(method("GET"))
        .and(path("/host"))
        .respond_with(page(ARTICLE).set_delay(Duration::from_millis(300)))
        .mount(&h.server)
        .await;
    let url = h.url("/host");
    let started = std::time::Instant::now();
    let (a, b) = tokio::join!(h.fetch(&url), h.fetch(&url));
    assert!(a.is_ok() && b.is_ok());
    assert!(
        started.elapsed() >= Duration::from_millis(600),
        "one fetch per host at a time (LIM-C4): {:?}",
        started.elapsed()
    );
}

/// REQ-SEC-011: a server slower than LIM-F4 ends the call with
/// `unavailable` / `deadline`.
#[tokio::test]
async fn req_sec_011_slow_server_hits_the_deadline() {
    let h = harness_with(Limits {
        fetch_deadline_ms: 1000,
        ..Limits::default()
    })
    .await;
    Mock::given(method("GET"))
        .and(path("/drip"))
        .respond_with(page(ARTICLE).set_delay(Duration::from_secs(3)))
        .mount(&h.server)
        .await;
    let started = std::time::Instant::now();
    let error = h.fetch(&h.url("/drip")).await.unwrap_err();
    assert_eq!(error.detail.reason, Some(DetailReason::Deadline));
    assert!(started.elapsed() < Duration::from_millis(2500));
}

/// REQ-MCP-007 and REQ-LIFE-002: a cancelled fetch ends as cancelled and
/// is counted so.
#[tokio::test]
async fn req_mcp_007_cancelled_fetch_returns_nothing() {
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/hang"))
        .respond_with(page(ARTICLE).set_delay(Duration::from_secs(5)))
        .mount(&h.server)
        .await;
    let pipeline = Pipeline {
        limits: &h.limits,
        policy: &h.policy,
        service: &h.service,
        returned: &h.returned,
        strict_mode: false,
        robots_txt: false,
        follow_cross_host_redirects: false,
        secret_detection: SecretDetection::Refuse,
        counters: &h.counters,
    };
    let cancel = CancellationToken::new();
    let soon = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        soon.cancel();
    });
    let req = FetchRequest {
        url: h.url("/hang"),
        max_chars: None,
        start_char: None,
    };
    let started = std::time::Instant::now();
    assert_eq!(
        run(&pipeline, req, cancel).await.unwrap_err(),
        CallError::Cancelled
    );
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(h.counters.calls_of("web_fetch", "cancelled"), 1);
}

/// REQ-PAGE-006 and REQ-TEST-006: a node bomb served over the network
/// yields partial content with the notice; the process lives.
#[tokio::test]
async fn req_page_006_node_bomb_over_the_network_is_partial() {
    let h = harness_with(Limits {
        html_max_nodes: 5000,
        ..Limits::default()
    })
    .await;
    let bytes = std::fs::read(corpus_dir().join("12-node-bomb").join("page.html")).unwrap();
    Mock::given(method("GET"))
        .and(path("/bomb"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(bytes, "text/html"))
        .mount(&h.server)
        .await;
    let response = h.fetch(&h.url("/bomb")).await.unwrap();
    assert!(
        response
            .notices
            .iter()
            .any(|n| n.code == NoticeCode::ExtractionPartial)
    );
    assert!(
        response
            .content
            .contains("First paragraph of the node bomb")
    );
}

/// REQ-FETCH-003: `max_chars` defaults to LIM-F1, is bounded by the
/// configured ceiling, and cuts at a character boundary with `truncated`.
#[tokio::test]
async fn req_fetch_003_max_chars_default_ceiling_and_cut() {
    let h = harness_with(Limits {
        fetch_max_chars: 600,
        fetch_max_chars_ceiling: 700,
        ..Limits::default()
    })
    .await;
    let long = format!(
        "<html><head><title>Long</title></head><body><main><p>{}</p></main></body></html>",
        "日本語の文章です。".repeat(200)
    );
    Mock::given(method("GET"))
        .and(path("/long"))
        .respond_with(page(&long))
        .mount(&h.server)
        .await;
    let default = h.fetch(&h.url("/long")).await.unwrap();
    assert_eq!(default.content.chars().count(), 600, "LIM-F1 as configured");
    assert!(default.truncated);
    assert_eq!(default.next_start_char, Some(600));
    assert!(default.content.is_char_boundary(default.content.len()));
    let error = h
        .fetch_req(FetchRequest {
            url: h.url("/long"),
            max_chars: Some(701),
            start_char: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidRequest);
    assert_eq!(error.detail.max, Some(700));
    let error = h
        .fetch_req(FetchRequest {
            url: h.url("/long"),
            max_chars: Some(10),
            start_char: None,
        })
        .await
        .unwrap_err();
    assert_eq!(error.detail.min, Some(500));
}

/// REQ-FETCH-010 and D-18: an `http` URL is retried over `https`; when
/// the retry fails at the connection the call is `denied` / `scheme`;
/// with `allow_http` the plain URL is fetched.
#[tokio::test]
async fn req_fetch_010_http_is_retried_over_https() {
    let server = MockServer::start().await;
    let port = Url::parse(&server.uri()).unwrap().port().unwrap();
    Mock::given(method("GET"))
        .and(path("/p"))
        .respond_with(page(ARTICLE))
        .mount(&server)
        .await;
    let strict_https = Arc::new(EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        false,
        vec![443, port],
    ));
    let limits = Limits::default();
    let settings = AgentClientSettings {
        user_agent: "rypsok/test".into(),
        connect_timeout: Duration::from_secs(2),
        timeout: Duration::from_secs(5),
        allow_http: false,
    };
    let service = FetchService::new(&settings, Arc::clone(&strict_https), &limits).unwrap();
    let counters = Counters::new();
    let returned = ReturnedUrls::new(10);
    let pipeline = Pipeline {
        limits: &limits,
        policy: &strict_https,
        service: &service,
        returned: &returned,
        strict_mode: false,
        robots_txt: false,
        follow_cross_host_redirects: false,
        secret_detection: SecretDetection::Refuse,
        counters: &counters,
    };
    let req = FetchRequest {
        url: format!("http://127.0.0.1:{port}/p"),
        max_chars: None,
        start_char: None,
    };
    let error = match run(&pipeline, req, CancellationToken::new()).await {
        Err(CallError::Tool(error)) => error,
        other => panic!("{other:?}"),
    };
    assert_eq!(error.kind, ErrorKind::Denied);
    assert_eq!(
        error.detail.reason,
        Some(DetailReason::Scheme),
        "the plain server cannot speak https, so the retry fails"
    );
    let h = harness().await;
    Mock::given(method("GET"))
        .and(path("/p"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    assert!(
        h.fetch(&h.url("/p")).await.is_ok(),
        "allow_http fetches the plain URL"
    );
}

/// REQ-PAGE-001: the stages run in the documented order: the privacy check
/// before the policy, strict mode before the policy, robots.txt before the
/// page request, the policy before any request.
#[tokio::test]
async fn req_page_001_stages_run_in_order() {
    let mut h = harness().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /\n"))
        .mount(&h.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(page(ARTICLE))
        .expect(0)
        .mount(&h.server)
        .await;
    // Privacy before policy: a secret in an internal URL is secret_in_url.
    let e = h
        .fetch("https://10.0.0.1/?k=AKIAIOSFODNN7EXAMPLE")
        .await
        .unwrap_err();
    assert_eq!(e.detail.reason, Some(DetailReason::SecretInUrl));
    // Strict mode before policy: an internal URL is strict_mode first.
    h.strict_mode = true;
    let e = h.fetch("https://10.0.0.1/x").await.unwrap_err();
    assert_eq!(e.detail.reason, Some(DetailReason::StrictMode));
    h.strict_mode = false;
    // robots.txt before the page: the page is never requested.
    let e = h.fetch(&h.url("/page")).await.unwrap_err();
    assert_eq!(e.detail.reason, Some(DetailReason::RobotsTxt));
}

/// REQ-PAGE-004 and REQ-SEC-013: a body is never buffered beyond LIM-F3:
/// a compressed response whose declared length is small still ends at the
/// decompressed bound.
#[tokio::test]
async fn req_page_004_no_body_is_buffered_beyond_the_bound() {
    let h = harness_with(Limits {
        fetch_max_bytes: 32_768,
        ..Limits::default()
    })
    .await;
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&vec![b'z'; 2_000_000]).unwrap();
    let gz = encoder.finish().unwrap();
    assert!(gz.len() < 32_768);
    Mock::given(method("GET"))
        .and(path("/zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_raw(gz, "text/plain"),
        )
        .mount(&h.server)
        .await;
    let error = h.fetch(&h.url("/zip")).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::TooLarge);
    assert_eq!(error.detail.limit_bytes, Some(32_768));
}

/// REQ-PAGE-008 and LIM-C5: parsing runs under the parsing permit; a call
/// that cannot have it before the deadline ends with `busy`.
#[tokio::test]
async fn req_page_008_parsing_permit_bounds_parsing() {
    let h = harness_with(Limits {
        parsing_in_flight: 1,
        fetch_deadline_ms: 500,
        ..Limits::default()
    })
    .await;
    Mock::given(method("GET"))
        .and(path("/p"))
        .respond_with(page(ARTICLE))
        .mount(&h.server)
        .await;
    let held = Arc::clone(&h.service.parsing)
        .acquire_owned()
        .await
        .unwrap();
    let error = h.fetch(&h.url("/p")).await.unwrap_err();
    assert_eq!(error.detail.reason, Some(DetailReason::Busy));
    drop(held);
    assert!(h.fetch(&h.url("/p")).await.is_ok());
}

/// REQ-SEC-008: a redirect target passes the same policy as the first
/// request; a refused target is `denied` and never disclosed.
#[tokio::test]
async fn req_sec_008_redirect_targets_pass_the_policy() {
    let h = harness().await;
    for target in [
        "http://127.0.0.2/x",
        "http://[::1]/x",
        "http://10.1.2.3/x",
        "http://localhost/x",
        "https://user:pw@example.org/x",
    ] {
        let name = format!("/to-{}", target.len());
        Mock::given(method("GET"))
            .and(path(name.clone()))
            .respond_with(ResponseTemplate::new(302).insert_header("location", target))
            .mount(&h.server)
            .await;
        let error = h.fetch(&h.url(&name)).await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Denied, "{target}");
        assert_eq!(error.detail.location, None, "{target}");
        let rendered = serde_json::to_string(&error).unwrap();
        assert!(
            !rendered.contains("10.1.2.3") && !rendered.contains("::1"),
            "{target}"
        );
    }
}
