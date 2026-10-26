# Extraction corpus

Hand-written pages for REQ-FETCH-002 and REQ-PAGE-002 (RFC 012). No
third-party content. Each directory holds `page.html` (or `page.txt`) and
`expected.toml` with the title, phrases that must appear in the rendered
Markdown in order (`must_contain`), phrases that must be absent
(`must_not_contain`), the expected notice codes, and optionally the media
type the fixture is served as.

| # | Fixture | Exercises |
|---|---|---|
| 01 | blog-article | `article` with `nav`, `header`, `footer`, sidebar; links, emphasis, lists |
| 02 | news-no-semantics | `div` soup; scoring picks the article; ads, promo, social by class |
| 03 | docs-page | `main`, headings, a code block with indentation, nested lists, a table |
| 04 | wiki-page | `role=main`, infobox table, references list, `role=navigation` and `contentinfo` |
| 05 | hidden-injection | `display:none`, `hidden`, `aria-hidden`, a comment, zero-width and bidi characters, a fake envelope (THR-01, THR-03) |
| 06 | shift-jis | `Shift_JIS` declared in `meta` only |
| 07 | euc-jp-undeclared | no declaration; detection |
| 08 | xhtml | `application/xhtml+xml`, self-closing tags |
| 09 | plain-text | `text/plain`, no extraction |
| 10 | malformed | unclosed and misnested tags, stray closers; the parser recovers |
| 11 | deep-nesting | 10 000 nested `div`; the depth bound reports `extraction_partial` |
| 12 | node-bomb | more than LIM-F10 nodes; the parse stops and reports `extraction_partial` |
| 13 | noscript-template | content inside `noscript` and `template` absent |
| 14 | meta-refresh | not followed; the page's own text returned |
| 15 | tiny-candidate | a one-line `article`; fallback to `body` |
