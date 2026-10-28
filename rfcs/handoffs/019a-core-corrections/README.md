# Handoff 019a — Core corrections: egress rule names, unreachable robots.txt, decoded text, the compare

**Task title.** Fix four defects of `rypsok-core` and one of the gate found
by the first independent review (RFC 019 D1 to D4).
**RFC.** [`../../accepted/019-first-independent-review.md`](../../accepted/019-first-independent-review.md)
(the design; this handoff never overrides it).
**Applicable requirements.** REQ-SEC-006, REQ-SEC-007 (D1); REQ-CMP-005,
REQ-FETCH-011 (D2); REQ-SEC-003, REQ-CTX-005, REQ-ENG-012 (D3);
REQ-SEC-018 (D4). Threats THR-05, THR-07, THR-01.
**Source findings.** `.git-exclude/reviewed/arch-001-prepare/README.md` §3
items 3, 4, 5 and S3; `.git-exclude/review-requests/dev-001-prepare/README.md`
S1, S2.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** Nothing. **Purpose.** A caller's and an operator's
statements come true: a denied name is denied however it is written, an
unreachable site is reported as unreachable, a title is returned as the
page wrote it, and the token compare has no aliasing.

## Background

The review reproduced each item against the 0.7.0 binary; the original
architect re-verified them in the code (`policy.rs` `judge_name`,
`robots.rs` `read`, `fetch.rs` title cleaning, `http.rs`
`constant_time_eq`). Nothing is disputed.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | `egress/policy.rs`: a `canonical_name(&str) -> String` (lower-case, trailing dot removed; empty stays empty) applied at the entry of `judge_url` (for `Host::Name`), `judge_name`, `judge_address`, and in `resolver::admitted`; `HostPattern::new` canonicalizes the rule's name the same way; `from_texts` refuses a rule that is empty after canonicalization | `req_sec_006_rules_match_the_canonical_name`: deny `blocked.invalid`; `https://blocked.invalid./`, `https://BLOCKED.INVALID./`, `https://blocked.invalid/` all `Denial::HostName`; a `*.ads.invalid` rule against `x.ads.invalid.`; a proptest that `judge_name(n) == judge_name(canonical_name(n))` over `[A-Za-z0-9.-]{1,40}` | configuration page, `[egress]` rules: "names are compared lower-cased and without a trailing dot" |
| 2 | `fetch/robots.rs`: a `Verdict::Unreachable` for a transport failure (`send` error) and for an invalid redirect target that the policy refuses at the robots stage? No: a policy refusal stays `DisallowAll` (it is a refusal). Only the `send().await` error and a timeout become `Unreachable`; `Robots::check` returns a `Decision::Unreachable` that is **not stored** in the cache; `fetch.rs` maps it to `ToolError::network()` before the page request | `req_cmp_005_unreachable_robots_is_unavailable_not_denied`: with `wiremock` stopped (a closed loopback port in `base`), `web_fetch` is `unavailable`/`network`; the robots cache has no entry afterwards; a 5xx robots.txt is still `denied`/`robots_txt` and cached | tools page (`web_fetch` errors) and troubleshooting: "an unreachable site is `unavailable` with reason `network`; `robots_txt` means the file said so or answered 5xx" |
| 3 | `boundary.rs`: `pub fn clean_decoded_text(text: &str, max_chars: u32) -> Cleaned` (NFC, invisible removal, whitespace collapse, bound; no markup step, no entity decoding); `fetch.rs` uses it for the title; `engine.rs`: `RawResult` gains a private provenance field with `RawResult::new` (raw) and `RawResult::decoded(self) -> Self`; the search pipeline's `make_result` call site chooses the cleaner by it; `engine::scrape` marks its results decoded | `req_sec_003_decoded_titles_keep_their_angle_brackets`: a page `<title>std::vec::Vec&lt;T&gt; - Rust</title>` through `render_document` + the fetch pipeline returns `title` `std::vec::Vec<T> - Rust`; a scraped result with `Vec<T>` in its title keeps it; a Wikipedia snippet with `<span class="searchmatch">` still loses the markup; the corpus and the hostile corpus unchanged | RFC 005's steps table in `docs/src/maintainers/external-design.md` §9 (one sentence on provenance) |
| 4 | `mcp/http.rs` `constant_time_eq`: `let mut difference = u8::from(a.len() != b.len());` | `req_sec_018_token_comparison_has_no_length_aliasing`: `(t, t + 256 zero bytes)` is unequal; the existing structure test unchanged | — |

## Explicit non-change scope

The built-in refused ranges and names; the robots.txt 5xx rule; the
cleaner's markup rules for raw provider text (an unterminated `<x` still
consumes the rest, as a tokenizer does at end of input); the
`characters_removed` semantics; the `Cleaned` type; every public
signature except the additions named above.

## Prohibited shortcuts

No re-encoding of decoded text into entities to reuse `clean_text`; no
`unwrap` on the robots cache lock; no change to the fixture corpus to
make a test pass; no `#[allow]` to silence a new warning.

## Boundaries

`rypsok-core` only for items 1 to 3; `rypsok::mcp::http` for item 4. No
protocol dependency enters the core (`tools/check-core-dependencies`).

## Compatibility and security constraints

Item 2 changes an error kind for one situation toward its documented
meaning; say so in the `CHANGELOG.md` `Fixed` line. Items 1 and 4 only
tighten. Item 3 must not let markup from a raw provider snippet through:
the Wikipedia fixture test proves it.

## Known risks

Item 3 touches the one function every result passes through; run the
hostile corpus (`req_test_007`) and the fetch corpus before and after.

## Required evidence and review request

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace`, the three `tools/` gates, `cargo
doc`, `mdbook build docs`; the new tests named above green; the fuzz
targets `clean_text` and `returned_url` run once for 60 s on nightly.
Review request at `.git-exclude/review-requests/019a-core-corrections/README.md`
with the fields of the operating instructions §7; deviations as
questions, never redesigns.
