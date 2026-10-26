//! Tests of main-content extraction (REQ-FETCH-002, REQ-PAGE-002,
//! REQ-CTX-005). Every test names the requirement it verifies. The corpus
//! test in `fetch/tests.rs` covers whole pages; these cover the rules.

use super::*;
use crate::fetch::dom::parse;

fn content_text(html: &str) -> (String, Extraction) {
    let dom = parse(html, 100_000);
    let extraction = extract(&dom, 256);
    let (text, _) = text_of(&dom, extraction.root, &extraction.excluded, 256);
    (
        text.split_whitespace().collect::<Vec<_>>().join(" "),
        extraction,
    )
}

/// REQ-CTX-005: scripts, styles, templates, noscript, comments and
/// elements hidden by attribute or inline style are excluded with their
/// subtrees.
#[test]
fn req_ctx_005_non_rendered_content_is_excluded() {
    let (text, _) = content_text(
        "<html><head><title>T</title><style>p{color:red}</style></head><body><main>\
         <p>visible</p><script>alert('js')</script><template><p>tpl</p></template>\
         <noscript><p>noscript</p></noscript><!-- comment -->\
         <p hidden>hidden-attr</p><p aria-hidden=\"true\">aria</p>\
         <div style=\"display: none\"><p>display-none</p></div>\
         <span style=\"color:red; visibility:hidden\">vis-hidden</span>\
         <p style=\"display:block\">shown</p></main></body></html>",
    );
    assert_eq!(text, "visible shown");
}

/// REQ-FETCH-002 and REQ-PAGE-002: `main` wins over `article` and over
/// scoring; chrome outside the candidate is absent; chrome inside the
/// candidate is excluded too; ancestors of the candidate are kept.
#[test]
fn req_page_002_candidate_order_and_chrome() {
    let (text, extraction) = content_text(
        "<html><body><header class=\"site-header\">Site</header><nav>Home About</nav>\
         <div id=\"wrapper\"><main><h1>Title</h1><p>Body text of the article that is long enough to count as content for the page under test, with more words here.</p>\
         <nav class=\"toc\">toc links</nav><aside>aside</aside><p>More body.</p></main></div>\
         <footer>Footer</footer><div class=\"comments\">comment 1</div></body></html>",
    );
    assert!(text.starts_with("Title Body text"));
    assert!(text.ends_with("More body."));
    for absent in [
        "Site",
        "Home About",
        "toc links",
        "aside",
        "Footer",
        "comment 1",
    ] {
        assert!(!text.contains(absent), "{absent:?} in {text:?}");
    }
    assert_eq!(
        extraction.title, "Title",
        "no <title>: the first h1 of the candidate"
    );
}

/// REQ-FETCH-002: without semantic elements the best-scored container is
/// chosen: the one whose paragraphs carry the most non-link text, not the
/// link-dense sidebar.
#[test]
fn req_fetch_002_scoring_picks_the_prose_container() {
    let sidebar_links = (0..30)
        .map(|i| format!("<p><a href=\"https://x.example/{i}\">link number {i} here</a></p>"))
        .collect::<String>();
    let html = format!(
        "<html><head><title>Scored</title></head><body><div class=\"wrap\">\
         <div class=\"col-left\">{sidebar_links}</div>\
         <div class=\"col-main\"><p>First paragraph of the real article text which is clearly prose and not navigation.</p>\
         <p>Second paragraph continues the article with even more prose for the scorer to see.</p>\
         <p>Third paragraph ends it, and together these exceed the tiny-candidate threshold easily.</p></div></div></body></html>"
    );
    let (text, extraction) = content_text(&html);
    assert!(text.starts_with("First paragraph"));
    assert!(!text.contains("link number"));
    assert_eq!(extraction.title, "Scored");
}

/// REQ-FETCH-002: a candidate under 200 characters yields to the body when
/// the body has more text (a page whose `article` is a teaser).
#[test]
fn req_fetch_002_tiny_candidate_falls_back_to_body() {
    let (text, _) = content_text(
        "<html><body><article><p>Teaser only.</p></article>\
         <div><p>The real text lives outside the article element on this page, which happens on sites that misuse the element for a blurb. It goes on for a while so that it clearly exceeds the threshold used by the extractor.</p></div></body></html>",
    );
    assert!(text.contains("Teaser only."));
    assert!(text.contains("The real text lives outside"));
}

/// LIM-F11: a subtree deeper than the bound is skipped and reported.
#[test]
fn req_page_006_depth_bound_is_reported() {
    let mut html = String::from("<html><body><main><p>top</p>");
    for _ in 0..300 {
        html.push_str("<div>");
    }
    html.push_str("<p>deep text</p>");
    for _ in 0..300 {
        html.push_str("</div>");
    }
    html.push_str("</main></body></html>");
    let dom = parse(&html, 100_000);
    let extraction = extract(&dom, 256);
    assert!(extraction.depth_truncated);
    let (text, truncated) = text_of(&dom, extraction.root, &extraction.excluded, 256);
    assert!(truncated);
    assert!(text.contains("top"));
    assert!(!text.contains("deep text"));
    let shallow = extract(&dom, 1000);
    assert!(!shallow.depth_truncated);
}

/// REQ-CTX-005: the chrome token list matches whole tokens and dashed
/// prefixes, not substrings of unrelated words.
#[test]
fn req_ctx_005_chrome_tokens_match_tokens_not_substrings() {
    let (text, _) = content_text(
        "<html><body><main><p>The main paragraph with sufficient length to be the candidate of this document for the test.</p>\
         <div class=\"advertisement-slot\">ad text</div><div class=\"ad-banner\">banner ad</div>\
         <div class=\"headerless\">kept one</div><div class=\"nav-bar\">nav text</div>\
         <div class=\"navigator\">kept two</div><div id=\"share_buttons\">share</div></main></body></html>",
    );
    assert!(text.contains("kept one"));
    assert!(text.contains("kept two"));
    for absent in ["ad text", "banner ad", "nav text", "share"] {
        assert!(!text.contains(absent), "{absent:?} in {text:?}");
    }
}
