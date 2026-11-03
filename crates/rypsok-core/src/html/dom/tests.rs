//! Tests of the bounded tree (REQ-PAGE-006, REQ-TEST-006). Every test
//! names the requirement it verifies.

use super::*;

/// The LIM-F11 value, for tests that are not about depth.
const DEPTH: u32 = crate::limits::defaults::HTML_MAX_DEPTH;

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
        DEPTH,
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
    let dom = parse(&html, 100, DEPTH);
    assert!(dom.truncated());
    assert!(
        dom.len() < 100 + FEED_CHUNK_BYTES / 4,
        "{} nodes: the bound plus at most one chunk",
        dom.len()
    );
    assert!(text_of(&dom, DOCUMENT).starts_with("paragraph 0paragraph 1"));
    assert!(!text_of(&dom, DOCUMENT).contains("paragraph 4999"));
    let full = parse(&html, 100_000, DEPTH);
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
            // The depth bound is lifted here: the test is about the
            // traversals of a tree that is deeper than any real bound.
            let dom = parse(&html, 100_000, 100_000);
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
        DEPTH,
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

/// `n` nested `<div>`s inside `<html><body>`, with `inner` in the deepest
/// one; the deepest div is at depth `n + 2` (document = 0, html = 1).
fn nested(n: usize, inner: &str) -> String {
    let mut html = String::from("<html><body>");
    for _ in 0..n {
        html.push_str("<div>");
    }
    html.push_str(inner);
    for _ in 0..n {
        html.push_str("</div>");
    }
    html.push_str("</body></html>");
    html
}

/// REQ-PAGE-006, REQ-SEC-011 and LIM-F11 (RFC 020 D-3, NET-5): a document
/// nested to exactly the bound is parsed in full; one level deeper is cut.
#[test]
fn req_page_006_a_document_at_the_depth_bound_parses_in_full() {
    let bound = 256;
    let at = parse(&nested(bound - 2, "deepest"), 100_000, bound as u32);
    assert!(!at.depth_exceeded(), "depth {bound} is allowed");
    assert!(!at.truncated());
    let deepest = at.elements().iter().map(|&(_, d)| d).max().unwrap();
    assert_eq!(deepest, bound, "the deepest element is at the bound");
    assert_eq!(text_of(&at, DOCUMENT), "deepest");

    let over = parse(&nested(bound - 1, "deepest"), 100_000, bound as u32);
    assert!(over.depth_exceeded(), "depth {} is one too deep", bound + 1);
    assert!(!over.truncated(), "the node bound was not reached");
}

/// REQ-PAGE-006 and LIM-F11: a million nested elements stop the feed loop;
/// the tree holds the bound plus at most one chunk of elements. The node
/// count is asserted, not the time.
#[test]
fn req_page_006_a_million_nested_divs_stop_at_the_bound_plus_one_chunk() {
    let html = nested(1_000_000, "x");
    let dom = parse(&html, 100_000, DEPTH);
    assert!(dom.depth_exceeded());
    // `<div>` is 5 bytes: one chunk holds FEED_CHUNK_BYTES / 5 of them, and
    // the document, html, head and body come on top.
    let limit = DEPTH as usize + FEED_CHUNK_BYTES / 5 + 8;
    assert!(
        dom.len() <= limit,
        "{} nodes; the bound plus one chunk is {limit}",
        dom.len()
    );
}

/// REQ-PAGE-006: the text before the deep spot is in the tree, the text
/// after it is not.
#[test]
fn req_page_006_text_before_the_deep_spot_is_kept() {
    let mut html = String::from("<html><body><p>before the deep part</p>");
    html.push_str(&nested(5_000, "inside"));
    html.push_str("<p>after the deep part</p>");
    let dom = parse(&html, 100_000, DEPTH);
    assert!(dom.depth_exceeded());
    let text = text_of(&dom, DOCUMENT);
    assert!(text.starts_with("before the deep part"), "{text:?}");
    assert!(!text.contains("after the deep part"));
}

/// REQ-PAGE-006: the node bound and the depth bound work together; the one
/// reached first stops the feed and only its flag is set.
#[test]
fn req_page_006_the_node_bound_and_the_depth_bound_together() {
    let wide: String = (0..5000).map(|i| format!("<p>p{i}</p>")).collect();
    let deep = nested(1_000, "d");
    let wide_first = format!("<html><body>{wide}{deep}</body></html>");
    let by_nodes = parse(&wide_first, 100, DEPTH);
    assert!(by_nodes.truncated());
    assert!(!by_nodes.depth_exceeded(), "never got as deep as the bound");
    let deep_first = format!("<html><body>{deep}{wide}</body></html>");
    let by_depth = parse(&deep_first, 100_000, DEPTH);
    assert!(by_depth.depth_exceeded());
    assert!(
        !by_depth.truncated(),
        "never got as many nodes as the bound"
    );
    let both = parse(&deep_first, 100, DEPTH);
    assert!(both.truncated() || both.depth_exceeded());
    assert!(both.len() < 100 + FEED_CHUNK_BYTES);
}

/// REQ-SEC-012 and LIM-F11 (RFC 020 D-3): a generous guard, not a
/// benchmark. `html5ever` scans its stack of open elements, so the time of a
/// parse grew with the square of the depth (7.8 s for 80,000 nested divs in a
/// release build); with the bound a page 100,000 deep is cut after a few
/// hundred elements. Fails if the parse takes more than 5 s in a debug
/// build, which the old behaviour does by a wide margin.
#[test]
fn req_sec_012_a_page_nested_100000_deep_parses_quickly() {
    let html = nested(100_000, "x");
    let started = std::time::Instant::now();
    let dom = parse(&html, 100_000, DEPTH);
    let took = started.elapsed();
    assert!(dom.depth_exceeded());
    assert!(
        took < std::time::Duration::from_secs(5),
        "the parse took {took:?}"
    );
}

/// Markup whose repair by the parser (the adoption agency, foster
/// parenting) moves whole subtrees, repeated.
const REPAIRING_UNITS: [(&str, &str); 7] = [
    ("b-div", "<b><div>"),
    ("a-div", "<a><div>"),
    ("b-p-closeb", "<b><p>x</b>"),
    ("nested-b-div-closeb", "<b><div></b>"),
    ("i-b-div-closei", "<i><b><div></i>"),
    ("table-foster", "<table><b><tr><td>"),
    ("font-center", "<font><center></font>"),
];

/// The one unit above that repairs without getting deeper: the tree stays
/// 4 levels deep however often it is repeated.
const STAYS_SHALLOW: &str = "b-p-closeb";

/// REQ-PAGE-006 (RFC 020 D-3): the depth the tree records is the depth it
/// has, also where the parser moves subtrees. A depth taken only when a node
/// is first attached says 4 for 150 repetitions of `<i><b><div></i>`, whose
/// tree is 303 deep.
#[test]
fn req_page_006_the_recorded_depth_is_the_true_depth_after_repairs() {
    for (name, unit) in REPAIRING_UNITS {
        let html = format!("<html><body>{}", unit.repeat(150));
        let dom = parse(&html, 1_000_000, u32::MAX);
        let true_max = dom.elements().iter().map(|&(_, d)| d).max().unwrap();
        assert_eq!(dom.levels[DOCUMENT] as usize, true_max, "{name}");
    }
}

/// REQ-SEC-012 and LIM-F11 (RFC 020 D-3): markup that nests through the
/// parser's repairs does not get past the bound either, and the parse of
/// 100,000 repetitions ends after a few hundred nodes.
#[test]
fn req_sec_012_repairing_markup_cannot_hide_its_depth() {
    for (name, unit) in REPAIRING_UNITS {
        let html = format!("<html><body>{}", unit.repeat(100_000));
        let started = std::time::Instant::now();
        let dom = parse(&html, 100_000, DEPTH);
        if name == STAYS_SHALLOW {
            assert!(!dom.depth_exceeded(), "{name} is not deep");
            continue;
        }
        assert!(dom.depth_exceeded(), "{name}");
        assert!(
            dom.len() < 10_000,
            "{name}: {} nodes; the feed went on after the bound",
            dom.len()
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "{name}: {:?}",
            started.elapsed()
        );
    }
}
