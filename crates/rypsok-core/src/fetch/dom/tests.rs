//! Tests of the bounded tree (REQ-PAGE-006, REQ-TEST-006). Every test
//! names the requirement it verifies.

use super::*;

fn text_of(dom: &Dom, h: Handle) -> String {
    let mut out = String::new();
    let mut stack = vec![h];
    while let Some(n) = stack.pop() {
        if let Some(t) = dom.text(n) {
            out.push_str(t);
        }
        for &c in dom.children(n).iter().rev() {
            stack.push(c);
        }
    }
    out
}

/// REQ-PAGE-006: a document parses into a tree whose text runs are merged
/// and whose elements carry their attributes; template contents are kept
/// apart from the tree.
#[test]
fn req_page_006_tree_holds_elements_text_and_attributes() {
    let dom = parse(
        "<!doctype html><html><body><p id=\"a\" class=\"x y\">Hello, <b>world</b>!</p><template><p>never</p></template><!-- c --></body></html>",
        1000,
    );
    assert!(!dom.truncated());
    let p = dom.first("p").unwrap();
    assert_eq!(dom.attr(p, "id"), Some("a"));
    assert_eq!(dom.attr(p, "class"), Some("x y"));
    assert_eq!(text_of(&dom, p), "Hello, world!");
    let template = dom.first("template").unwrap();
    assert!(
        dom.children(template).is_empty(),
        "template contents are not children"
    );
    assert_eq!(text_of(&dom, DOCUMENT), "Hello, world!");
    assert_eq!(dom.depth(p), 3, "document > html > body > p");
}

/// REQ-PAGE-006 and LIM-F10: once the node bound is reached the parser is
/// fed no further chunk; the tree is marked truncated and holds at most
/// the bound plus one chunk; what was built before the bound is intact.
#[test]
fn req_page_006_parse_stops_at_the_node_bound() {
    let mut html = String::from("<html><body>");
    for i in 0..5000 {
        html.push_str(&format!("<p>paragraph {i}</p>"));
    }
    html.push_str("</body></html>");
    let dom = parse(&html, 100);
    assert!(dom.truncated());
    assert!(
        dom.len() < 100 + FEED_CHUNK_BYTES / 4,
        "{} nodes: the bound plus at most one chunk",
        dom.len()
    );
    assert!(text_of(&dom, DOCUMENT).starts_with("paragraph 0paragraph 1"));
    assert!(!text_of(&dom, DOCUMENT).contains("paragraph 4999"));
    let full = parse(&html, 100_000);
    assert!(!full.truncated());
    assert!(text_of(&full, DOCUMENT).ends_with("paragraph 4999"));
}

/// REQ-TEST-006 and THR-12: deep nesting is parsed without recursion; the
/// test runs on a thread with a small stack.
#[test]
fn req_test_006_deep_nesting_parses_on_a_small_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let mut html = String::from("<html><body>");
            for _ in 0..20_000 {
                html.push_str("<div>");
            }
            html.push_str("deep");
            for _ in 0..20_000 {
                html.push_str("</div>");
            }
            let dom = parse(&html, 100_000);
            (dom.len(), dom.elements().len(), dom.truncated())
        })
        .unwrap();
    let (len, elements, truncated) = handle.join().unwrap();
    assert!(elements > 1000, "{elements} elements");
    assert!(len > 1000);
    assert!(!truncated);
}

/// REQ-PAGE-006: malformed markup (unclosed tags, stray closers,
/// misnested inline elements) parses into a usable tree; the parser's
/// adoption agency reparents without recursion.
#[test]
fn req_page_006_malformed_markup_still_parses() {
    let dom = parse(
        "<html><body><div><p>one<b>two<i>three</b>four</i></div></span><p>five",
        1000,
    );
    assert!(!dom.truncated());
    let text = text_of(&dom, DOCUMENT);
    assert!(text.contains("one"));
    assert!(text.contains("three"));
    assert!(text.contains("five"));
    assert!(dom.first("div").is_some());
    assert!(
        dom.first("span").is_none(),
        "a stray closer creates nothing"
    );
}
