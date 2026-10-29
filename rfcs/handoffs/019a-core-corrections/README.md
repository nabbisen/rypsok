# Handoff 019a — Core corrections: canonical host names, unreachable sites, decoded text, the compare

**Revision 2 (2026-10-29)**, after the independent review (`arch-002-review-019`,
M1, M2, M4, R4) and the RFC 019 amendment of the same day. Where this
revision differs from revision 1, this one holds.

**Task title.** Fix four defects of `rypsok-core` and one of the gate
found by the first independent review (RFC 019 D1 to D4 as amended).
**RFC.** [`../../accepted/019-first-independent-review.md`](../../accepted/019-first-independent-review.md),
including its amendment of 2026-10-29.
**Applicable requirements.** REQ-SEC-006, REQ-SEC-007, REQ-SEC-012 (D1);
REQ-CMP-005, REQ-FETCH-006, REQ-FETCH-011, REQ-ERR-004 (D2); REQ-SEC-003,
REQ-CTX-005, REQ-ENG-012, REQ-SEC-023 (D3); REQ-SEC-018 (D4). Threats
THR-05, THR-06, THR-07, THR-01.
**Source findings.** `arch-001-prepare` §3 items 3, 4, 5, S3; `dev-001-prepare`
S1, S2; `arch-002-review-019` M1, M2, M4, R4.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** Nothing. **May start at once:** items 3 and 4. **Items 1
and 2:** start with this revision. **Purpose.** A denied name is denied
however it is written and a malformed name is refused; a site the fetch
cannot reach is reported the same way whatever the reason, so that the
answer discloses nothing about internal names; a title is returned as
the page wrote it; the token compare has no aliasing.

## Background

The review reproduced each item against the 0.7.0 binary and the
original architect re-verified them in the code. The independent
architect then found that normalizing *all* trailing dots would match
malformed names, that distinguishing a refused name from an unreachable
one in the caller's answer is an oracle for internal names, and that a
new cleaner needs a fuzz target. Those three corrections are in items
1 to 3 below.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | `egress/policy.rs`: `Host::name(raw: &str) -> Result<Host, Denial>` is the **one** constructor of a `Host::Name`: lower-case, at most one trailing root dot removed, then refused with `Denial::HostName` if empty or if any label is empty (`a..`, `a..b`, `.a`). Used by `judge_url` (domain hosts), `judge_name`, `judge_address`, `resolver::admitted`, `fetch.rs` strict mode (today `Host::Name(name.to_ascii_lowercase())`, line 308), the per-host fetch permit key (`host_key`, LIM-C4: `x.example` and `x.example.` share one permit), `robots::origin_of` (the robots cache key) and the same-host redirect comparison. `HostPattern::new` canonicalizes a rule's name the same way; `from_texts` refuses a rule that is malformed after canonicalization with the key named | `req_sec_006_rules_match_the_canonical_name`: deny `blocked.invalid`; `https://blocked.invalid./` and `https://BLOCKED.INVALID./` refused as `HostName`; `https://blocked.invalid../` and `https://blocked.invalid.../` refused as `HostName` (malformed, not matched); a `*.ads.invalid` rule against `x.ads.invalid.`; property: for `host in "[a-z0-9]{1,8}(\\.[a-z0-9]{1,8}){1,3}"`, `judge_name(host) == judge_name(host.to_uppercase()) == judge_name(format!("{host}."))`; for any name with an empty label the judgment is `Err(HostName)`; a test that `host_key`, `origin_of` and strict mode agree on `x.example` and `x.example.` | configuration page, `[egress]` rules: "names are compared lower-cased and without the root dot; a name with an empty label is refused" |
| 2 | **One classification of every `send()` error in the fetch path.** A function `fetch::classify_send_error(&reqwest::Error) -> SendFailure { Unreachable, Refused(Denial) }` walks the error's source chain for `resolver::Refused`; the page request, each redirect hop and the robots.txt read call it. The caller's answer is the **same** for both: `ToolError::network()` with the message "the site could not be reached"; neither is cached (the robots cache stores no verdict for either; today's `DisallowAll` on a `send()` error goes). `Refused` additionally logs `warn` `refused` with `denial = <name>` and **no host** (today a `debug` `address_refused`). A robots.txt that answers 5xx stays `DisallowAll` → `denied`/`robots_txt`, cached one hour (RFC 9309 §2.3.1.4). Literal addresses are judged on the URL before any request and stay `denied`/`egress` | `req_cmp_005_unreachable_robots_is_unavailable_not_denied` (a closed loopback port: `unavailable`/`network`; no robots cache entry; a 5xx robots.txt still `denied`/`robots_txt` and cached); `req_sec_007_a_refused_name_and_an_unreachable_name_answer_alike`: with a scripted `Lookup`, `private.invalid` → `10.0.0.1` and `missing.invalid` → no address give **byte-identical** error objects (kind, reason, message, detail), with `robots_txt` on and off, on the first and on the second call; the refused case logged `refused` once (capture the subscriber); a literal `http://10.0.0.1/` stays `denied`/`egress` | troubleshooting "Reading pages": the `denied`/`egress` row says *literal addresses and ports*; a new row: "a name that cannot be reached, for whatever reason, is `unavailable`/`network`; the operator's log says whether the policy refused it"; the tools page's `unavailable` row the same; external design §8 "Destination" row gains the sentence |
| 3 | `boundary.rs`: rename `clean_text` → `clean_markup_text` (first doc line: "for raw provider text: markup is stripped, entities decoded; use `clean_decoded_text` for text a parser already decoded"); add `pub fn clean_decoded_text(text, max_chars) -> Cleaned` (NFC, invisible removal, whitespace collapse, bound; no markup step, no entity decoding; first doc line says so); `fetch.rs` uses it for the title; `engine.rs`: `RawResult` gains a private provenance field, `RawResult::new` = raw, `RawResult::decoded(self) -> Self`; `boundary::make_result` or its caller in `search/pipeline.rs` chooses the cleaner by it; `engine::scrape` marks its results decoded; `fuzz/fuzz_targets/clean_decoded_text.rs` asserting the bound, no leading or trailing space and **full** idempotence (`twice == once`, `removed == 0`), listed in `fuzz/Cargo.toml` | `req_sec_003_decoded_titles_keep_their_angle_brackets`: `<title>std::vec::Vec&lt;T&gt; - Rust</title>` through the fetch pipeline → `title` `std::vec::Vec<T> - Rust`; a scraped result with `Vec<T>` keeps it; a result stored in and served from the cache keeps its provenance (search twice with the cache on); the Wikipedia fixture snippet with `<span class="searchmatch">` still loses the markup; the hostile corpus (`req_test_007`) and the fetch corpus unchanged; the fuzz target run 60 s on nightly with no finding | external design §9 (one sentence on provenance and the two cleaners); `CHANGELOG` line in 019d (a renamed public function of `rypsok-core`) |
| 4 | `mcp/http.rs` `constant_time_eq`: `let mut difference = u8::from(a.len() != b.len());` | `req_sec_018_token_comparison_has_no_length_aliasing`: `(t, t + 256 zero bytes)` unequal; the structure test unchanged | — |

## Explicit non-change scope

The built-in refused ranges and names (M8 owns S2); the robots.txt 5xx
rule; the cleaner's markup rules for raw provider text (an unterminated
`<x` still consumes the rest, as a tokenizer does at end of input); the
`characters_removed` semantics; the `Cleaned` type; the error message
vocabulary beyond the one sentence named in item 2.

## Prohibited shortcuts

No re-encoding of decoded text into entities to reuse the markup cleaner;
no second normalization function beside `Host::name`; no `unwrap` on a
lock; no fixture edited to make a test pass; no `#[allow]` to silence a
new warning; no log line that carries a host name in item 2.

## Boundaries

`rypsok-core` for items 1 to 3 (plus one fuzz target); `rypsok::mcp::http`
for item 4. No protocol dependency enters the core.

## Compatibility and security constraints

Item 2 changes the caller's answer for a refused *name* from today's
accidental mixture (`denied`/`robots_txt` with robots on, `unavailable`/
`network` with it off) to one answer; say so in the changelog line as a
correction toward the documented meaning of both kinds, and say that the
operator's log now names the refusal. Item 3 renames a public function
of `rypsok-core` (pre-1.0: a changelog line). Items 1 and 4 only tighten.
Item 3 must not let markup from a raw provider snippet through.

## Known risks

Item 2 touches the only place the egress policy's decision becomes an
answer; the byte-identity test is the proof, and the reviewer will read
the classification function. Item 3 touches the function every result
passes through; run both corpora before and after.

## Required evidence and review request

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace`, the three `tools/` gates, `cargo
doc`, `mdbook build docs`; the new tests named above green; the fuzz
targets `clean_text` (renamed with the function), `clean_decoded_text`
and `returned_url` run once for 60 s each on nightly with their
execution counts reported. Review request at
`.git-exclude/review-requests/019a-core-corrections/README.md` with the
fields of the operating instructions §7; deviations as questions, never
redesigns.
