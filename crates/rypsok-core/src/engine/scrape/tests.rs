//! Tests of result extraction (RFC 015; REQ-TEST-018) on the hand-written
//! DuckDuckGo fixtures. Every test names the requirement it verifies.

use super::*;
use crate::boundary::Provenance;
use crate::engine::drivers::duckduckgo::{DuckDuckGo, SHAPE};
use crate::html::dom::parse;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/fixtures/scrape/duckduckgo/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// REQ-ENG-015 and REQ-ENG-001: results come in page order with the
/// redirect wrapper unwrapped, a non-web link dropped, a missing snippet
/// empty, and `max_results` respected.
#[test]
fn req_test_018_results_in_order_with_wrappers_unwrapped() {
    let dom = parse(&fixture("results.html"), 50_000);
    let Page::Results(results) = read(&dom, &SHAPE, 10, &DuckDuckGo::unwrap_link) else {
        panic!("results expected");
    };
    assert_eq!(results.len(), 3, "{results:?}");
    assert_eq!(results[0].title, "Rust Programming Language");
    assert_eq!(results[0].url, "https://www.rust-lang.org/");
    assert_eq!(
        results[0].snippet,
        "A language empowering everyone to build reliable and efficient software."
    );
    assert_eq!(results[1].url, "https://doc.rust-lang.org/book/");
    assert_eq!(
        results[2].url,
        "https://en.wikipedia.org/wiki/Rust_(programming_language)"
    );
    assert_eq!(results[2].snippet, "");
    assert!(results.iter().all(|r| r.date.is_none()));
    let Page::Results(two) = read(&dom, &SHAPE, 2, &DuckDuckGo::unwrap_link) else {
        panic!("results expected");
    };
    assert_eq!(two.len(), 2);
}

/// REQ-ENG-015: the three other page states are told apart.
#[test]
fn req_test_018_no_results_challenge_and_drift_are_told_apart() {
    let no = parse(&fixture("no-results.html"), 50_000);
    assert_eq!(
        read(&no, &SHAPE, 10, &DuckDuckGo::unwrap_link),
        Page::NoResults
    );
    let challenge = parse(&fixture("challenge.html"), 50_000);
    assert_eq!(
        read(&challenge, &SHAPE, 10, &DuckDuckGo::unwrap_link),
        Page::Challenge
    );
    let drifted = parse(&fixture("drifted.html"), 50_000);
    assert_eq!(
        read(&drifted, &SHAPE, 10, &DuckDuckGo::unwrap_link),
        Page::Drifted
    );
    let empty = parse("", 50_000);
    assert_eq!(
        read(&empty, &SHAPE, 10, &DuckDuckGo::unwrap_link),
        Page::Drifted
    );
}

/// REQ-ENG-015: a page above the node bound ends parsing with what was
/// found or with drift, never a panic.
#[test]
fn req_test_018_the_node_bound_ends_parsing_without_a_panic() {
    let padding: String = "<span>x</span>".repeat(2_000);
    let after = fixture("results.html")
        .replace("<div id=\"links\"", &format!("{padding}<div id=\"links\""));
    let dom = parse(&after, 300);
    assert!(dom.truncated());
    assert_eq!(
        read(&dom, &SHAPE, 10, &DuckDuckGo::unwrap_link),
        Page::Drifted
    );
    let before = fixture("results.html").replace(
        "<div id=\"bottom_navigation\">",
        &format!("{padding}<div id=\"bottom_navigation\">"),
    );
    let dom = parse(&before, 300);
    assert!(dom.truncated());
    assert!(
        matches!(read(&dom, &SHAPE, 10, &DuckDuckGo::unwrap_link), Page::Results(r) if !r.is_empty())
    );
}

/// REQ-ENG-001: the redirect wrapper yields its target; anything that is
/// not a web link is dropped.
#[test]
fn req_eng_001_wrapper_unwrapping() {
    assert_eq!(
        DuckDuckGo::unwrap_link(
            "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.org%2Fa%3Fb%3Dc&rut=1"
        ),
        Some("https://example.org/a?b=c".to_owned())
    );
    assert_eq!(
        DuckDuckGo::unwrap_link("https://duckduckgo.com/l/?uddg=ftp%3A%2F%2Fexample.org%2F"),
        None
    );
    assert_eq!(
        DuckDuckGo::unwrap_link("https://duckduckgo.com/l/?rut=1"),
        None
    );
    assert_eq!(
        DuckDuckGo::unwrap_link("https://example.org/page"),
        Some("https://example.org/page".to_owned())
    );
    assert_eq!(DuckDuckGo::unwrap_link("javascript:void(0)"), None);
    assert_eq!(DuckDuckGo::unwrap_link("/html/?q=next"), None);
}

/// The selector matches by tag, class token and attribute, each only
/// when given.
#[test]
fn req_eng_015_selectors_match_by_tag_class_and_attribute() {
    let dom = parse(
        "<div class=\"a result b\"><a class=\"result__a\" href=\"/x\">t</a></div><form action=\"/html/anomaly\"></form>",
        1000,
    );
    let div = dom.first("div").unwrap();
    assert!(Selector::class("div", "result").matches(&dom, div));
    assert!(!Selector::class("div", "resul").matches(&dom, div));
    assert!(!Selector::class("p", "result").matches(&dom, div));
    let form = dom.first("form").unwrap();
    assert!(
        Selector {
            tag: Some("form"),
            class: None,
            attr: Some(("action", "/html/anomaly"))
        }
        .matches(&dom, form)
    );
    assert_eq!(text_of(&dom, div), "t");
}

/// REQ-SEC-003 and REQ-ENG-012 (RFC 019 D3): a scraping driver's results
/// are marked decoded: the DOM already decoded `&lt;T&gt;` into `<T>`, so
/// the pipeline must not run the markup cleaner over it.
#[test]
fn req_sec_003_scraped_results_are_marked_decoded() {
    let page = r#"<html><body><div class="result"><a class="result__a" href="https://doc.rust-lang.org/std/vec/struct.Vec.html">std::vec::Vec&lt;T&gt; - Rust</a><a class="result__snippet" href="https://doc.rust-lang.org/">Use Vec&lt;T&gt; here.</a></div></body></html>"#;
    let dom = parse(page, 50_000);
    let Page::Results(results) = read(&dom, &SHAPE, 10, &DuckDuckGo::unwrap_link) else {
        panic!("results expected");
    };
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "std::vec::Vec<T> - Rust");
    assert_eq!(results[0].snippet, "Use Vec<T> here.");
    assert_eq!(results[0].provenance(), Provenance::Decoded);
    // A result a driver builds from provider text stays raw by default.
    assert_eq!(
        RawResult::new("t", "https://example.org/", "s").provenance(),
        Provenance::Raw
    );
}
