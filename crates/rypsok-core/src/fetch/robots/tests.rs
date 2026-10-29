//! Tests of the robots.txt cache (REQ-CMP-005). Every test names the
//! requirement it verifies. The server is `wiremock`, admitted by an
//! allow rule of the test policy.

use std::sync::Arc;
use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::egress::policy::parse_rule;
use crate::egress::resolver::JudgingResolver;
use crate::egress::{AgentClientSettings, agent_client};

fn policy() -> Arc<EgressPolicy> {
    Arc::new(EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        true,
        vec![80, 443, 65535],
    ))
}

async fn harness() -> (MockServer, reqwest::Client, Arc<EgressPolicy>) {
    let server = MockServer::start().await;
    let policy = policy();
    let settings = AgentClientSettings {
        user_agent: "rypsok/test".into(),
        connect_timeout: Duration::from_secs(2),
        timeout: Duration::from_secs(5),
        allow_http: true,
    };
    let client = agent_client(
        &settings,
        Arc::new(JudgingResolver::new(Arc::clone(&policy))),
    )
    .unwrap();
    (server, client, policy)
}

fn target(policy: &EgressPolicy, server: &MockServer, path: &str) -> Target {
    let mut url = Url::parse(&server.uri()).unwrap();
    url.set_path(path);
    let allowed = EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        true,
        vec![url.port().unwrap()],
    );
    let _ = policy;
    allowed.judge_url(&url).unwrap()
}

fn limits() -> ReadLimits {
    ReadLimits {
        max_bytes: 512 * 1024,
        max_redirects: 5,
    }
}

/// REQ-CMP-005: rules for the product token and for `*` are honored with
/// longest-match semantics; the read is cached per origin so the second
/// check makes no request.
#[tokio::test]
async fn req_cmp_005_rules_are_honored_and_cached() {
    let (server, client, policy) = harness().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "User-agent: *\nDisallow: /private/\n\nUser-agent: rypsok\nDisallow: /secret/\nAllow: /secret/ok\n",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let robots = Robots::new(Duration::from_secs(86_400), 1000);
    let port_policy = EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        true,
        vec![Url::parse(&server.uri()).unwrap().port().unwrap()],
    );
    for (p, expected) in [
        ("/public", Decision::Allowed),
        ("/private/x", Decision::Allowed),
        ("/secret/x", Decision::Disallowed),
        ("/secret/ok/y", Decision::Allowed),
    ] {
        let t = target(&policy, &server, p);
        assert_eq!(
            robots.check(&client, &port_policy, &t, limits()).await,
            expected,
            "{p}"
        );
    }
    assert_eq!(robots.len(), 1);
}

/// REQ-CMP-005 and RFC 9309: a 404 allows everything; a 500 disallows
/// everything and is kept for one hour, not a day; a non-text body allows.
#[tokio::test]
async fn req_cmp_005_status_table() {
    for (status, body_type, expected, expected_ttl_s) in [
        (404, "text/plain", Decision::Allowed, 86_400),
        (500, "text/plain", Decision::Disallowed, 3_600),
        (200, "application/octet-stream", Decision::Allowed, 86_400),
        (200, "text/plain", Decision::Disallowed, 86_400),
    ] {
        let (server, client, policy) = harness().await;
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_raw(b"User-agent: *\nDisallow: /\n".to_vec(), body_type),
            )
            .mount(&server)
            .await;
        let robots = Robots::new(Duration::from_secs(86_400), 1000);
        let port_policy = EgressPolicy::new(
            vec![parse_rule("127.0.0.1/32").unwrap()],
            Vec::new(),
            true,
            vec![Url::parse(&server.uri()).unwrap().port().unwrap()],
        );
        let t = target(&policy, &server, "/page");
        assert_eq!(
            robots.check(&client, &port_policy, &t, limits()).await,
            expected,
            "status {status} {body_type}"
        );
        let entry = robots
            .entries
            .lock()
            .unwrap()
            .map
            .values()
            .next()
            .cloned()
            .unwrap();
        assert_eq!(
            entry.ttl,
            Duration::from_secs(expected_ttl_s),
            "status {status}"
        );
    }
}

/// LIM-F15: the cache evicts its oldest origin at capacity.
#[tokio::test]
async fn req_cmp_005_cache_is_bounded() {
    let robots = Robots::new(Duration::from_secs(86_400), 2);
    robots.store("https://a.example".into(), Verdict::AllowAll);
    robots.store("https://b.example".into(), Verdict::AllowAll);
    robots.store("https://c.example".into(), Verdict::AllowAll);
    assert_eq!(robots.len(), 2);
    assert!(robots.fresh("https://a.example").is_none());
    assert!(robots.fresh("https://c.example").is_some());
    assert_eq!(
        origin_of(&Url::parse("https://x.example:8443/a/b?c").unwrap()),
        "https://x.example:8443"
    );
    assert_eq!(
        origin_of(&Url::parse("https://x.example/").unwrap()),
        "https://x.example"
    );
}

/// REQ-CMP-005 and RFC 9309: a robots.txt redirect is followed, across
/// origins too, each hop under the policy; the rules of the final document
/// apply to the first origin; a hop the policy refuses makes the file
/// unreachable.
#[tokio::test]
async fn req_cmp_005_robots_redirects_are_followed_under_the_policy() {
    let (first, client, policy) = harness().await;
    let second = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/robots.txt", second.uri()).as_str()),
        )
        .mount(&first)
        .await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /private\n"),
        )
        .mount(&second)
        .await;
    let both = EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        true,
        vec![first.address().port(), second.address().port()],
    );
    let robots = Robots::new(Duration::from_secs(86_400), 1000);
    assert_eq!(
        robots
            .check(
                &client,
                &both,
                &target(&policy, &first, "/private/x"),
                limits()
            )
            .await,
        Decision::Disallowed
    );
    assert_eq!(
        robots
            .check(
                &client,
                &both,
                &target(&policy, &first, "/public"),
                limits()
            )
            .await,
        Decision::Allowed
    );
    let third = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "http://10.0.0.1/robots.txt"),
        )
        .mount(&third)
        .await;
    let robots = Robots::new(Duration::from_secs(86_400), 1000);
    let third_policy = EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        true,
        vec![third.address().port()],
    );
    assert_eq!(
        robots
            .check(
                &client,
                &third_policy,
                &target(&policy, &third, "/anything"),
                limits()
            )
            .await,
        Decision::Disallowed
    );
}

/// REQ-CMP-005 and REQ-SEC-006 (RFC 019 D1): the cache key names the host
/// in its canonical form, so `x.example` and `x.example.` share one entry.
#[test]
fn req_cmp_005_cache_key_uses_the_canonical_host_name() {
    for (a, b) in [
        ("https://x.example/", "https://x.example./"),
        ("https://X.Example:8443/p", "https://x.example.:8443/q"),
    ] {
        assert_eq!(
            origin_of(&Url::parse(a).unwrap()),
            origin_of(&Url::parse(b).unwrap()),
            "{a} and {b}"
        );
    }
    assert_eq!(
        origin_of(&Url::parse("https://x.example./").unwrap()),
        "https://x.example"
    );
    assert_eq!(
        origin_of(&Url::parse("https://[::1]:8443/").unwrap()),
        "https://[::1]:8443"
    );
}
