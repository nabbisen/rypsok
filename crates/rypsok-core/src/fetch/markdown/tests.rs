//! Tests of the Markdown renderer (REQ-FETCH-004). Every test names the
//! requirement it verifies.

use std::collections::HashSet;

use super::*;
use crate::html::dom::parse;

fn md(html: &str) -> String {
    let dom = parse(html, 100_000);
    let body = dom.first("body").unwrap();
    render(
        &dom,
        body,
        &HashSet::new(),
        Options {
            max_depth: 256,
            max_url_chars: 2048,
        },
    )
    .markdown
}

/// REQ-FETCH-004: headings, paragraphs, emphasis, code and links render
/// to their Markdown forms; paragraphs are blank-line separated.
#[test]
fn req_fetch_004_headings_paragraphs_inline_and_links() {
    let out = md(
        "<body><h1>Title</h1><p>Plain <b>bold</b> and <em>em</em> with <code>x = 1</code>.</p>\
         <h2>Sub</h2><p>See <a href=\"https://example.org/a\">the page</a> or <a href=\"/rel\">relative</a>.</p></body>",
    );
    assert_eq!(
        out,
        "# Title\n\nPlain **bold** and *em* with `x = 1`.\n\n## Sub\n\nSee [the page](https://example.org/a) or relative."
    );
}

/// REQ-FETCH-004: lists nest with two-space indentation; ordered lists
/// count from `start`; blockquotes prefix lines; `pre` becomes a fence
/// whose content is kept verbatim.
#[test]
fn req_fetch_004_lists_quotes_and_code_blocks() {
    let out = md(
        "<body><ul><li>one</li><li>two<ul><li>nested</li></ul></li></ul>\
         <ol start=\"3\"><li>three</li><li>four</li></ol>\
         <blockquote><p>quoted</p></blockquote>\
         <pre><code>fn main() {\n    println!(\"hi\");\n}</code></pre></body>",
    );
    assert_eq!(
        out,
        "- one\n- two\n  - nested\n\n3. three\n4. four\n\n> quoted\n\n```\nfn main() {\n    println!(\"hi\");\n}\n```"
    );
}

/// REQ-FETCH-004: a uniform table becomes a pipe table with a header
/// separator; a ragged one becomes lines; images contribute their
/// alternative text only; `br` breaks a line; `hr` is a rule.
#[test]
fn req_fetch_004_tables_images_breaks_and_rules() {
    let out = md(
        "<body><table><tr><th>Name</th><th>Value</th></tr><tr><td>a</td><td>1 | 2</td></tr></table>\
         <table><tr><td>x</td></tr><tr><td>y</td><td>z</td></tr></table>\
         <p><img src=\"https://example.org/i.png\" alt=\"A picture\"> after<br>next line</p><hr><p>end</p></body>",
    );
    assert_eq!(
        out,
        "| Name | Value |\n| --- | --- |\n| a | 1 \\| 2 |\n\nx\ny · z\n\nA picture after  \nnext line\n\n---\n\nend"
    );
}

/// REQ-SEC-002 and REQ-FETCH-004: characters that would start Markdown
/// syntax inside text are escaped, so external text cannot forge
/// structure; whitespace runs collapse like a browser's rendering.
#[test]
fn req_sec_002_text_is_escaped_and_whitespace_collapsed() {
    let out = md(
        "<body><p>  Stars *not emphasis*   and   [not a link](x) and `not code`\n\n  here  </p><p># not a heading</p></body>",
    );
    assert_eq!(
        out,
        "Stars \\*not emphasis\\* and \\[not a link\\](x) and \\`not code\\` here\n\n\\# not a heading"
    );
}

/// LIM-F9: a link whose target is longer than the URL bound, or not an
/// absolute http(s) URL, is rendered as its text.
#[test]
fn req_ctx_006_links_are_bounded() {
    let long = format!("https://example.org/{}", "a".repeat(3000));
    let out = md(&format!(
        "<body><p><a href=\"{long}\">long</a> <a href=\"javascript:alert(1)\">js</a> <a href=\"https://example.org/ok\"></a></p></body>"
    ));
    assert_eq!(out, "long js <https://example.org/ok>");
}
