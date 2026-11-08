//! Tests of URL normalization. Every test names the requirement it verifies.

use proptest::prelude::*;

use super::*;

const MAX: u32 = 2048;

/// A key as text.
fn text(key: &ComparisonKey) -> &str {
    &key.0
}

/// REQ-RANK-003: the returned URL keeps everything but tracking
/// parameters and writes the host in ASCII form; invalid URLs are rejected.
#[test]
fn req_rank_003_returned_url_table() {
    let rows: &[(&str, Result<&str, UrlRejection>)] = &[
        ("https://example.org/page", Ok("https://example.org/page")),
        ("https://example.org/page/", Ok("https://example.org/page/")),
        ("https://Example.ORG/Page", Ok("https://example.org/Page")),
        (
            "https://example.org/page#section",
            Ok("https://example.org/page#section"),
        ),
        (
            "https://example.org:8443/page",
            Ok("https://example.org:8443/page"),
        ),
        (
            "https://example.org:443/page",
            Ok("https://example.org/page"),
        ),
        ("http://example.org/page", Ok("http://example.org/page")),
        (
            "https://example.org/page?utm_source=x",
            Ok("https://example.org/page"),
        ),
        (
            "https://example.org/page?utm_source=x&id=7",
            Ok("https://example.org/page?id=7"),
        ),
        (
            "https://example.org/page?id=7&UTM_CAMPAIGN=y",
            Ok("https://example.org/page?id=7"),
        ),
        (
            "https://example.org/page?fbclid=a&gclid=b&q=rust",
            Ok("https://example.org/page?q=rust"),
        ),
        (
            "https://example.org/page?a=1&b=2",
            Ok("https://example.org/page?a=1&b=2"),
        ),
        (
            "https://example.org/page?b=2&a=1",
            Ok("https://example.org/page?b=2&a=1"),
        ),
        (
            "https://例え.jp/ページ",
            Ok("https://xn--r8jz45g.jp/%E3%83%9A%E3%83%BC%E3%82%B8"),
        ),
        (
            "https://bücher.example/",
            Ok("https://xn--bcher-kva.example/"),
        ),
        ("https://example.org/a%2Fb", Ok("https://example.org/a%2Fb")),
        (
            "https://example.org/%7Euser",
            Ok("https://example.org/%7Euser"),
        ),
        ("https://example.org/a b", Ok("https://example.org/a%20b")),
        (
            "  https://example.org/page  ",
            Ok("https://example.org/page"),
        ),
        (
            "https://www.example.org/page",
            Ok("https://www.example.org/page"),
        ),
        // RFC 020 DAT-8: the query is the engine's, byte for byte, unless a
        // tracking parameter is removed; then the others keep their text.
        (
            "https://example.org/p?a=1&b",
            Ok("https://example.org/p?a=1&b"),
        ),
        (
            "https://example.org/p?a=1;b=2",
            Ok("https://example.org/p?a=1;b=2"),
        ),
        (
            "https://example.org/p?q=a%20b",
            Ok("https://example.org/p?q=a%20b"),
        ),
        (
            "https://example.org/p?a=%7e",
            Ok("https://example.org/p?a=%7e"),
        ),
        (
            "https://example.org/p?x=%zz",
            Ok("https://example.org/p?x=%zz"),
        ),
        (
            "https://example.org/p?a=1&&b=2",
            Ok("https://example.org/p?a=1&&b=2"),
        ),
        (
            "https://example.org/p?utm_source=x&a=1&b",
            Ok("https://example.org/p?a=1&b"),
        ),
        (
            "https://example.org/p?a=1;z&utm_medium=y",
            Ok("https://example.org/p?a=1;z"),
        ),
        (
            "https://example.org/p?a=1&utm_medium=y;z",
            Ok("https://example.org/p?a=1"),
        ),
        (
            "https://example.org/p?q=a%20b&fbclid=1&r=%7e",
            Ok("https://example.org/p?q=a%20b&r=%7e"),
        ),
        (
            "https://example.org/p?utm_source=x&gclid=y#top",
            Ok("https://example.org/p#top"),
        ),
        ("ftp://example.org/file", Err(UrlRejection::Invalid)),
        ("javascript:alert(1)", Err(UrlRejection::Invalid)),
        ("https://user:pw@example.org/", Err(UrlRejection::Invalid)),
        ("https://user@example.org/", Err(UrlRejection::Invalid)),
        ("not a url", Err(UrlRejection::Invalid)),
        ("/relative/path", Err(UrlRejection::Invalid)),
        ("https://", Err(UrlRejection::Invalid)),
    ];
    let mut wrong = Vec::new();
    for (input, expected) in rows {
        let got = returned_url(input, MAX);
        if got.as_ref().map(String::as_str).map_err(|e| *e) != *expected {
            wrong.push(format!("{input}: got {got:?}, expected {expected:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    let long = format!("https://example.org/{}", "x".repeat(2100));
    assert_eq!(returned_url(&long, MAX), Err(UrlRejection::TooLong));
    assert!(returned_url(&long, 4096).is_ok());
}

/// REQ-RANK-003 (RFC 020, D-24): a result's URL is shown only if its host
/// passes the check `web_fetch` applies (`Host::from_name` for a name, or it
/// is an address), so that the search never shows a URL the fetch refuses on
/// syntax. A host with an empty label is invalid; one trailing root dot is
/// not; underscores stay; address literals pass.
#[test]
fn req_rank_003_a_host_web_fetch_would_refuse_is_invalid() {
    let rows: &[(&str, Result<&str, UrlRejection>)] = &[
        ("https://.xtras__url/", Err(UrlRejection::Invalid)),
        ("https:/.xtras__url", Err(UrlRejection::Invalid)),
        ("https://a..b/", Err(UrlRejection::Invalid)),
        ("https://a../", Err(UrlRejection::Invalid)),
        ("https://.example/", Err(UrlRejection::Invalid)),
        ("https://a./", Ok("https://a./")),
        ("https://a_b.example/", Ok("https://a_b.example/")),
        ("https://127.0.0.1/", Ok("https://127.0.0.1/")),
        ("https://0x7f.1/", Ok("https://127.0.0.1/")),
        ("https://[::1]/p", Ok("https://[::1]/p")),
    ];
    let mut wrong = Vec::new();
    for (input, expected) in rows {
        let got = returned_url(input, MAX);
        if got.as_ref().map(String::as_str).map_err(|e| *e) != *expected {
            wrong.push(format!("{input}: got {got:?}, expected {expected:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    // One rule, not two: for a range of hosts the answer is what the fetch
    // path's own check gives.
    for host in [
        "example.org",
        "a.b.c",
        "a.",
        "a..",
        ".a",
        "a..b",
        "x--y.example",
        "_x.example",
        "1.2.3.4",
        "xn--r8jz45g.jp",
        "a.b.",
        "www.",
        "\u{4f8b}\u{3048}.jp",
    ] {
        let url = format!("https://{host}/");
        let fetchable = match Url::parse(&url)
            .ok()
            .and_then(|u| u.host().map(|h| h.to_owned()))
        {
            Some(url::Host::Domain(name)) => crate::egress::policy::Host::from_name(&name).is_ok(),
            Some(_) => true,
            None => false,
        };
        assert_eq!(returned_url(&url, MAX).is_ok(), fetchable, "{host}");
    }
}

/// REQ-RANK-002: equal keys for the forms of one resource, different keys
/// for different resources.
#[test]
fn req_rank_002_comparison_key_table() {
    let same: &[&[&str]] = &[
        &[
            "https://example.org/page",
            "http://example.org/page",
            "https://www.example.org/page",
            "https://EXAMPLE.org/page/",
            "https://example.org:443/page",
            "https://example.org/page#",
            "https://example.org/page?utm_source=x",
        ],
        &[
            "https://example.org/a?x=1&y=2",
            "https://example.org/a?y=2&x=1",
            "http://www.example.org/a/?y=2&x=1",
        ],
        &[
            "https://example.org/%7euser",
            "https://example.org/~user",
            "https://example.org/%7Euser",
        ],
        &["https://example.org/a%2fb", "https://example.org/a%2Fb"],
        &[
            "https://例え.jp/x",
            "https://xn--r8jz45g.jp/x",
            "https://XN--R8JZ45G.jp/x/",
        ],
        &[
            "https://example.org/",
            "https://example.org",
            "https://www.example.org/",
        ],
    ];
    for group in same {
        let keys: Vec<ComparisonKey> = group
            .iter()
            .map(|u| comparison_key(&returned_url(u, MAX).unwrap()))
            .collect();
        for key in &keys {
            assert_eq!(key, &keys[0], "{group:?}");
        }
    }
    let different: &[(&str, &str)] = &[
        ("https://example.org/Page", "https://example.org/page"),
        ("https://example.org/a", "https://example.org/b"),
        ("https://example.org/a?x=1", "https://example.org/a?x=2"),
        ("https://example.org:8443/a", "https://example.org/a"),
        ("https://m.example.org/a", "https://example.org/a"),
        ("https://example.org/a%2Fb", "https://example.org/a/b"),
    ];
    for (a, b) in different {
        assert_ne!(comparison_key(a), comparison_key(b), "{a} vs {b}");
    }
    assert_eq!(
        text(&comparison_key("https://www.Example.org:443/a/?b=2&a=1#f")),
        "https://example.org/a?a=1&b=2#f"
    );
}

/// REQ-RANK-002 (RFC 020 D-8): the twelve pairs of audit 020c DAT-9, equal
/// and different. `http` is `https` and `www.` is folded (unchanged); the
/// real port, a non-empty fragment and the order of repeated parameters are
/// part of the identity (new); the order of parameters of different names is
/// not.
#[test]
fn req_rank_002_the_twelve_pairs_of_the_audit() {
    let pairs: [(&str, &str, bool); 12] = [
        ("http://x.example/page", "https://x.example/page", true),
        ("https://x.example/", "https://www.x.example/", true),
        (
            "https://x.example/p?a=1&b=2",
            "https://x.example/p?b=2&a=1",
            true,
        ),
        (
            "https://x.example/p?a=1&a=2",
            "https://x.example/p?a=2&a=1",
            false,
        ),
        (
            "https://x.example/app#/inbox",
            "https://x.example/app#/settings",
            false,
        ),
        ("http://x.example:443/p", "https://x.example/p", false),
        ("https://x.example:80/p", "https://x.example/p", false),
        ("https://x.example/p/", "https://x.example/p", true),
        (
            "https://x.example/p?q=a+b",
            "https://x.example/p?q=a%20b",
            true,
        ),
        ("https://x.example/a%2Fb", "https://x.example/a/b", false),
        ("https://x.example/A", "https://x.example/a", false),
        (
            "https://x.example/p?q=a%2Bb",
            "https://x.example/p?q=a+b",
            false,
        ),
    ];
    let mut wrong = Vec::new();
    for (a, b, equal) in pairs {
        let (ka, kb) = (
            comparison_key(&returned_url(a, MAX).unwrap()),
            comparison_key(&returned_url(b, MAX).unwrap()),
        );
        if (ka == kb) != equal {
            wrong.push(format!("{a} vs {b}: expected equal={equal}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // More around the same rules.
    let more: [(&str, &str, bool); 9] = [
        ("https://x.example/app#", "https://x.example/app", true),
        ("https://x.example/app#a", "https://x.example/app#a", true),
        ("https://x.example/app#a", "https://x.example/app#b", false),
        ("http://x.example:8080/p", "https://x.example:8080/p", true),
        ("http://x.example:80/p", "http://x.example/p", true),
        ("https://x.example:443/p", "https://x.example/p", true),
        ("http://x.example:80/p", "https://x.example:80/p", false),
        (
            "https://x.example/p?a=2&a=1&b=0",
            "https://x.example/p?b=0&a=2&a=1",
            true,
        ),
        (
            "https://x.example/p?a=2&b=0&a=1",
            "https://x.example/p?a=1&b=0&a=2",
            false,
        ),
    ];
    for (a, b, equal) in more {
        let (ka, kb) = (
            comparison_key(&returned_url(a, MAX).unwrap()),
            comparison_key(&returned_url(b, MAX).unwrap()),
        );
        if (ka == kb) != equal {
            wrong.push(format!("{a} vs {b}: expected equal={equal}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// REQ-RANK-002: the key's text, which pins what the survivors of the
/// mutation run (audit 020d) change: the root path keeps its slash, only a
/// trailing slash of a longer path goes, the default port is hidden and no
/// other is, the fragment and the repeated-parameter order are in the key.
#[test]
fn req_rank_002_the_key_text_pins_the_port_the_root_and_the_fragment() {
    for (url, key) in [
        ("https://x.example/", "https://x.example/"),
        ("https://x.example", "https://x.example/"),
        ("https://x.example/p/", "https://x.example/p"),
        ("https://x.example//", "https://x.example/"),
        ("https://x.example:443/p", "https://x.example/p"),
        ("https://x.example:80/p", "https://x.example:80/p"),
        ("http://x.example:443/p", "https://x.example:443/p"),
        ("http://x.example:80/p", "https://x.example/p"),
        ("https://x.example:8443/p", "https://x.example:8443/p"),
        ("https://x.example:81/p", "https://x.example:81/p"),
        ("https://x.example:444/p", "https://x.example:444/p"),
        ("https://x.example/p#top", "https://x.example/p#top"),
        (
            "https://x.example/p?b=1&a=2&a=1",
            "https://x.example/p?a=2&a=1&b=1",
        ),
    ] {
        assert_eq!(text(&comparison_key(url)), key, "{url}");
    }
}

/// REQ-CTX-006 and LIM-F9: a URL of exactly the bound is accepted, one
/// character more is `TooLong` (the mutant `>` to `>=` of audit 020d).
#[test]
fn req_ctx_006_the_url_bound_is_inclusive() {
    let base = "https://example.org/";
    let exact = format!("{base}{}", "a".repeat(30 - base.len()));
    assert_eq!(exact.chars().count(), 30);
    assert_eq!(returned_url(&exact, 30), Ok(exact.clone()));
    let over = format!("{exact}a");
    assert_eq!(returned_url(&over, 30), Err(UrlRejection::TooLong));
}

proptest! {
    /// REQ-ARCH-004 with RFC 008: normalization is idempotent and the key
    /// of a returned URL is stable; two URLs with one key share a host.
    #[test]
    fn req_arch_004_normalization_is_idempotent(
        host in "[a-z]{1,8}\\.(org|jp)",
        path in "(/[a-zA-Z0-9%7E~._-]{0,8}){0,3}/?",
        query in "(\\?(utm_source|q|id)=[a-z0-9]{0,4}(&(utm_medium|p)=[a-z]{0,3})?)?",
        scheme in "https?",
        www in proptest::bool::ANY,
    ) {
        prop_assume!(!host.starts_with("www."));
        let raw = format!("{scheme}://{}{host}{path}{query}", if www { "www." } else { "" });
        if let Ok(once) = returned_url(&raw, MAX) {
            let twice = returned_url(&once, MAX).unwrap();
            prop_assert_eq!(&twice, &once);
            prop_assert_eq!(comparison_key(&twice), comparison_key(&once));
            let key = comparison_key(&once);
            prop_assert!(text(&key).starts_with("https://"));
            prop_assert!(!text(&key).contains("utm_"));
            // The `www.` label is removed; a host that merely contains the
            // letters (`awww.org`) keeps them (CI found the broader assertion
            // on 2026-10-29).
            let key_host = Url::parse(text(&key)).unwrap().host_str().unwrap().to_owned();
            prop_assert!(!key_host.starts_with("www."));
            prop_assert!(Url::parse(&once).unwrap().host_str().unwrap().ends_with(&host));
        }
    }
}

/// REQ-RANK-003 (RFC 020, TST-7): a percent-escape is read wherever it is in
/// the path, not only near the start (a mutant that compared twice the
/// position with the length was missed): unreserved characters are decoded,
/// the rest keep their escape in upper case, and a truncated escape stays.
#[test]
fn req_rank_003_percent_escapes_are_read_anywhere_in_the_path() {
    assert_eq!(normalize_percent("/a%41"), "/aA");
    assert_eq!(
        normalize_percent("/abcdefghijklmnop%41%7e%2f%2F"),
        "/abcdefghijklmnopA~%2F%2F"
    );
    assert_eq!(
        normalize_percent("/a/b/c/d/e/f/g/h/i/j/%61%62"),
        "/a/b/c/d/e/f/g/h/i/j/ab"
    );
    assert_eq!(normalize_percent("/%41"), "/A", "an escape at the very end");
    assert_eq!(normalize_percent("/ab%4"), "/ab%4", "truncated");
    assert_eq!(normalize_percent("/ab%"), "/ab%");
    assert_eq!(
        normalize_percent("/é%41é"),
        "/éAé",
        "multi-byte characters around"
    );
}
