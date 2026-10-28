# RFC 019 — Corrections from the first independent review

**Status.** Accepted (2026-10-29) — implementer may start at once.
**Tracks.** Release 0.7.1 (a correction release between M7 and M8);
requirements REQ-SEC-006 and REQ-SEC-007 (egress rules), REQ-CMP-005 and
REQ-FETCH-011 (robots.txt and unreachable sites), REQ-SEC-003 and
REQ-CTX-005 (no silent rewriting of visible text), REQ-SEC-018 and
REQ-SEC-024 (new: session bound), REQ-DEP-005 (binaries on the release
page), REQ-TEST-012 (gates), REQ-DOC-001 and REQ-DOC-002; threat register
THR-05, THR-26, THR-27 (new); RFC 000 (lifecycle).
**Source.** The first independent review of the delivered system,
`.git-exclude/reviewed/arch-001-prepare/README.md` (the new architect,
2026-10-28) and `.git-exclude/review-requests/dev-001-prepare/README.md`
(the dev team, 2026-10-28). Every item below names its finding there.
**Touches.** `rypsok-core::egress::policy`, `rypsok-core::fetch::robots`,
`rypsok-core::boundary` and `engine` (text provenance), `rypsok::mcp::http`
(constant-time compare, session bound), the release and CI workflows,
the book's user pages and the stability page, the RFC index.
**Handoffs.** [`019a`](../handoffs/019a-core-corrections/README.md) (core
defects), [`019b`](../handoffs/019b-listener-and-http-docs/README.md)
(listener and HTTP documentation), [`019c`](../handoffs/019c-release-ci-records/README.md)
(release binaries, gates, records).

## Problem and motivation

The first review by someone who did not write the code found, and the
original architect re-verified, five defects of consequence and a set of
documentation and process faults. Each is small; together they are the
difference between a system that matches its specification and one that
says so. They are fixed before the audit milestone M8 so that the audit
starts from a true baseline.

## Goals

Fix every item the review placed in its top five and every item of its
list that is a defect of behavior, of a user-facing document, or of a
gate; do it as a patch release; change no contract surface except where
the surface was wrong about the delivered behavior.

## Non-goals

The baseline drift the review lists (B1 to B16 except where named
below), the IPv6 range questions (S2), the lock-poisoning pattern (S9),
floating tool versions and workflow permissions (C2, C3), the platform
matrix (C6): these are M8's audit scope and are not in this RFC. No new
tool, parameter or engine.

## Design

### D1. Host names in operator rules are canonical before they match (§3.3, S1)

A name is canonicalized once at the entry of `judge_url`, `judge_name`
and `judge_address`, and in `resolver::admitted`: lower-cased, a trailing
dot removed, and the result used for both the operator's rules and the
built-in refusals. A rule written with a trailing dot is canonicalized
the same way at configuration time. Test: `deny = ["blocked.invalid"]`
refuses `https://blocked.invalid./`, `https://BLOCKED.INVALID./` and the
resolver's name alike; a property test asserts that the judgment of a
name equals the judgment of its canonical form.

### D2. An unreachable robots.txt is `unavailable`, not `denied` (§3.4)

The robots stage distinguishes three ends of a read:

| robots.txt | Verdict | The caller sees | Cached |
|---|---|---|---|
| 2xx, parsed | per the file | as today | LIM-F12 |
| 4xx | allow all | as today | LIM-F12 |
| 5xx | disallow all (RFC 9309 §2.3.1.4) | `denied`, `robots_txt` | one hour, as today |
| transport failure (no address, refused, timeout) | **unreachable** | `unavailable`, reason `network` | **not cached** |

The message of the `network` case says the site could not be reached;
nothing about robots.txt. A fetch whose robots.txt read fails on
transport would fail on the page read too, so no request is saved by
caching the failure, and a transient failure is never stated as a rule.

### D3. Text that is already decoded does not pass the markup step (§3.5)

The boundary gains `clean_decoded_text(text, max)`: steps 2 to 5 of
RFC 005 (entity decoding is harmless on decoded text and is kept for
uniformity? No: it is skipped; a decoded `&lt;` is the four characters
`&lt;`). So: NFC, invisible characters, whitespace, bound. Where text
comes from:

| Source | Cleaner | Why |
|---|---|---|
| A fetched page's title and content (the DOM) | `clean_decoded_text` / `clean_content` | The parser decoded entities and consumed markup; what remains is text |
| An engine answer from a JSON interface (Wikipedia, Brave) | `clean_text` | Providers put markup and entities in snippets |
| A scraping driver's results (the DOM) | `clean_decoded_text` | As the page |

`RawResult` states its provenance: `RawResult::new` marks text raw;
`RawResult::decoded()` marks it decoded, and the scraping reader uses it.
The pipeline chooses the cleaner by that mark. A `<T>` in a fetched title
or a scraped snippet survives. An unterminated `<x` in raw provider text
still consumes the rest, as a browser's tokenizer does at end of input;
the review's note is answered, not changed.

### D4. The constant-time compare has no length aliasing (S3)

`difference` starts as `u8::from(a.len() != b.len())`. The structure test
is extended: equal content with lengths differing by 256 is unequal.

### D5. Sessions are bounded (§3.2; THR-27, LIM-H6, REQ-SEC-024)

`[service.http] max_sessions` (LIM-H6, default 256, range 1 to 65536).
The listener wraps the SDK's session manager in one that counts live
sessions and refuses to create one beyond the bound; the SDK answers the
`initialize` with its error for a failed session creation and no session
id; the gate logs `http_refused` with reason `too_many_sessions`. The
SDK's idle expiry (300 s) stays. The stateless revision is unaffected.
Threat register: THR-27 "session exhaustion by legacy handshakes" (B6),
controls REQ-SEC-018 and REQ-SEC-024, verified by REQ-TEST-014.

### D6. The HTTP contract as delivered, written once (U1, U2, U3, B3)

- Every documented request sends `Accept: application/json,
  text/event-stream`; the examples test runs the tutorial's `curl` shape
  against the binary in HTTP mode and must get 200.
- A foreign `Host` and a refused `Origin` are **403** (the SDK's answer);
  a missing protocol header is **400**; the stability page freezes 400,
  401, 403, 404, 408, 413 with their meanings, and the external design
  §2 gains the listener's contract (path, statuses, headers).
- Refusals the SDK answers (`Host`, `Origin`, missing headers) are not
  logged as `http_refused`; the troubleshooting page and RFC 013's
  logging section say which refusals the gate logs and which the SDK
  answers.

### D7. Documentation defects (U4, U5, U6, U8)

The DuckDuckGo record enters the book's summary; the sentence about
release binaries becomes true by D8; one sentence about a corrected key
is used everywhere ("the key is read at startup; a corrected key takes
effect at the next restart, and the engine keeps being asked meanwhile,
so no cooldown delays it"); the `IPAddressAllow` examples name a
placeholder and say to list the engines' addresses.

### D8. Binaries on the release page (U5, REQ-DEP-005)

The release workflow builds `rypsok` for Linux x86_64 (musl, static),
macOS arm64 and Windows x86_64, attaches the archives and a checksum file
to the GitHub release, and keeps the image job. The workflow gains
`contents: write` for that job only.

### D9. Gates (C1, C4)

`tools/check-limits` and `cargo deny check bans` run in the rules job;
the gate-listing test names them.

### D10. Records (P2, P4, P5)

RFCs 001 to 018 move to `done/` with `Implemented (<tag>)`, in the commit
that carries this RFC; the index lists them under Implemented; the
handoffs' stale `.git-exclude/reviews/` path is explained by one line in
the index (review records live outside the repository, under the owner's
`review-requests/` and `reviewed/`). Packages 017 and 018 are archived
under `reviewed/`.

## Compatibility

D2 changes an error a caller may see from `denied`/`robots_txt` to
`unavailable`/`network` for unreachable sites: a correction toward the
documented meaning of both kinds. D3 changes returned text only where it
was wrongly altered. D5 adds a key with a default. D6 corrects the
documented statuses to the delivered ones. Nothing else is visible.

## Security considerations

D1 closes a bypass of an operator's deny rule; D4 removes a latent
weakness; D5 closes a memory-exhaustion path open to any legacy client
that passes the gate. None weakens a control.

## Tests

Named by requirement: `req_sec_006_rules_match_the_canonical_name`,
`req_cmp_005_unreachable_robots_is_unavailable_not_denied`,
`req_sec_003_decoded_titles_keep_their_angle_brackets`,
`req_sec_018_token_comparison_has_no_length_aliasing`,
`req_sec_024_sessions_are_bounded`, `req_doc_002_documented_curl_gets_200`,
`req_test_012_ci_workflow_lists_every_gate` (extended).

## Acceptance

Every handoff's acceptance met; CI green including the new gates; the
examples test's HTTP case green; release 0.7.1 with a `Fixed` section
that names each item.

## Alternatives considered

- **Folding these into M8.** Rejected: an audit should start from a
  system whose known defects are fixed, and users are following the
  wrong `curl` today.
- **Dropping legacy revisions instead of bounding sessions.** Rejected
  by the owner's answer: the default stays for compatibility; the bound
  makes its cost finite.
- **Escaping decoded text back to entities before the cleaner.**
  Rejected: a second encoding to undo a first is the kind of trick that
  hides a bypass; naming the provenance is honest and cheap.

## Risks

D5 depends on the SDK's session manager interface; the handoff names the
trait and the wrapping, and the dev team reports if the SDK's error path
cannot carry the refusal.
