//! Tests of URL normalization. Every test names the requirement it verifies.

use proptest::prelude::*;

use super::*;

const MAX: u32 = 2048;

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
        ("ftp://example.org/file", Err(UrlRejection::Invalid)),
        ("javascript:alert(1)", Err(UrlRejection::Invalid)),
        ("https://user:pw@example.org/", Err(UrlRejection::Invalid)),
        ("https://user@example.org/", Err(UrlRejection::Invalid)),
        ("not a url", Err(UrlRejection::Invalid)),
        ("/relative/path", Err(UrlRejection::Invalid)),
        ("https://", Err(UrlRejection::Invalid)),
    ];
    for (input, expected) in rows {
        assert_eq!(
            returned_url(input, MAX)
                .as_ref()
                .map(String::as_str)
                .map_err(|e| *e),
            *expected,
            "{input}"
        );
    }
    let long = format!("https://example.org/{}", "x".repeat(2100));
    assert_eq!(returned_url(&long, MAX), Err(UrlRejection::TooLong));
    assert!(returned_url(&long, 4096).is_ok());
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
            "https://example.org/page#top",
            "https://example.org/page?utm_source=x",
        ],
        &[
            "https://example.org/a?x=1&y=2",
            "https://example.org/a?y=2&x=1",
            "http://www.example.org/a/?y=2&x=1#frag",
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
        comparison_key("https://www.Example.org:443/a/?b=2&a=1#f").as_str(),
        "https://example.org/a?a=1&b=2"
    );
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
        let raw = format!("{scheme}://{}{host}{path}{query}", if www { "www." } else { "" });
        if let Ok(once) = returned_url(&raw, MAX) {
            let twice = returned_url(&once, MAX).unwrap();
            prop_assert_eq!(&twice, &once);
            prop_assert_eq!(comparison_key(&twice), comparison_key(&once));
            let key = comparison_key(&once);
            prop_assert!(key.as_str().starts_with("https://"));
            prop_assert!(!key.as_str().contains("utm_"));
            prop_assert!(!key.as_str().contains("www."));
            prop_assert!(Url::parse(&once).unwrap().host_str().unwrap().ends_with(&host));
        }
    }
}
