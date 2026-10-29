# RFC 019 — Corrections from the first independent review

**Status.** Implemented (0.8.0); accepted 2026-10-29.
**Tracks.** Release 0.8.0 (the corrections release after M7; it was 0.7.1 until the owner's decision of 2026-10-29, see Amendment 2; the audit M8 follows as 0.9.0);
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

## Amendment to RFC 019 (2026-10-29, after the independent review, revision 2)

The independent architect's review of this RFC and its handoffs
(`.git-exclude/reviewed/arch-002-review-019/README.md`) was conditionally
approved with six mandatory corrections and four recommendations; the
owner decided the three questions it raised on 2026-10-29. Every point is
applied here and in the handoffs; the dev team implements the amended
handoffs, not the text above where the two differ.

- **D1 revised (M2).** The canonical form of a host name is lower case
  with at most one root dot removed. A name with an empty label after
  that (`a..`, `a..b`, `.a`) is refused as `HostName`; malformed names
  are never normalized into a match. One constructor builds the
  canonical `Host::Name` for every user: the judgments, the resolver,
  strict mode, the per-host fetch permit (LIM-C4), the robots.txt cache
  key and the same-host redirect comparison. The property: for every
  well-formed name, case and an added root dot do not change the
  judgment; every name with an empty label is refused.
- **D2 revised (M1).** For a host name, a connect-time policy refusal and
  an unreachable site give the caller the same answer: `unavailable`,
  reason `network`, the same message ("the site could not be reached"),
  and neither is cached. Otherwise the error kind is an oracle for
  internal names (a name that resolves to a private address versus one
  that does not resolve), the reconnaissance the policy exists to deny,
  as REQ-FETCH-006 already withholds a refused redirect target. One
  classification of every `send()` error in the fetch path (the page
  request, each redirect hop, the robots.txt read) makes this uniform.
  The operator still learns of a refusal: a `warn` `refused` event with
  the denial's name and no host, by walking the error's source chain for
  the resolver's refusal, for logging only. Literal addresses are judged
  on the URL and stay `denied`/`egress`, which discloses nothing. The
  troubleshooting page's sentence that a name resolving to a private
  address is `denied`/`egress` is wrong today and changes.
- **D3 revised (M4, R4).** `clean_text` becomes `clean_markup_text`, with
  a first-line sentence saying it assumes raw provider text (markup and
  entities), and `clean_decoded_text` says it assumes text a parser
  already decoded; forgetting `RawResult::decoded()` errs on the safe
  side. `clean_decoded_text` gets a fuzz target with the full idempotence
  property (it has no entity residual). Provenance is a field of
  `RawResult`, so it survives the result cache, which stores
  `EngineResponse`. The thinking-aloud passage in D3 above ("? No:") is
  withdrawn; this paragraph is the record.
- **D5 specified (M3; owner's decision D-A, option 1).** The wrapper
  counts in `create_session` and decrements in `close_session` when the
  id was present; the SDK calls `close_session` when a session worker
  exits, expiry included, so no sweep. The SDK answers a failed
  `create_session` with 500 and our error's text in the body; the gate
  therefore runs the SDK call inside a `tokio::task_local!` cell that the
  wrapper sets when it refuses (`create_session` is awaited inline in the
  request's task), and the gate, seeing the cell set, discards the SDK's
  response and answers **503** with an empty body and `Retry-After: 30`
  (a fixed, documented hint). The wrapper's error text is a fixed
  non-sensitive sentence, because the SDK would put it in a 500 body if
  the cell were ever unset; a test pins the 503 on the pinned SDK and a
  second test pins the fallback (cell unset → 500 with the fixed text) so
  that an SDK change fails loudly. The gate logs `http_refused` with
  reason `session_limit` and the address family. Residual risk recorded
  with THR-27: the bound turns memory exhaustion into slot exhaustion
  (one caller can hold every slot for the 300 s idle expiry); per-caller
  session quotas and a shorter expiry are deferred.
- **D6 revised (R1, R2).** The gate logs an inner **403** as
  `http_refused` with reason `sdk_forbidden` and counts it: a foreign
  `Host` or `Origin` is the signature of THR-18 and must leave a trace;
  400 and 406 are client mistakes and are counted but not logged per
  request. The stability page says who owns each status: the gate
  answers 401, 404, 408, 413 and 503; the pinned SDK answers 400, 403 and
  406, and an SDK upgrade that changes them is a contract change. Tests
  pin all four SDK answers (403 `Host`, 403 `Origin`, 400 header, 406
  `Accept`). The pages tell users to send both `Accept` media types
  because the protocol requires it: Streamable HTTP, revision
  2026-07-28, "Sending Messages" 2: "The client MUST include an `Accept`
  header listing both `application/json` and `text/event-stream`."
- **D8 revised (M5; owner's decision D-C, option 2).** The release
  workflow never creates a release. The build jobs always upload workflow
  artifacts; an attach job uploads to the release of the tag only if that
  release exists, refuses to overwrite an existing asset, and otherwise
  writes a visible notice in the run summary; a `workflow_dispatch` input
  (the tag, validated against the tag pattern and passed through an
  environment variable, never interpolated) re-runs the attach after the
  owner publishes. Only the attach job gets `contents: write`. A
  build-only dispatch on `main` must run green and be recorded before
  0.7.1. `musl-tools` is installed for the musl target. The book names
  the attached targets (Linux x86_64, macOS arm64, Windows x86_64) and
  says that `SHA256SUMS` proves integrity, not origin; attestation is an
  M8 candidate.
- **D10 extended (M6; owner's decision D-B, option 3).** A fourth
  handoff, 019d "release preparation", carries the changelog section
  (including the two cleaner changes of 2026-10-27 and 2026-10-28 that
  0.6.0 and 0.7.0 omitted, B16, and the statuses), the version bump and
  `CURRENT_MILESTONE`; the dev team implements it last, the independent
  architect reviews it, the original architect tags and publishes on the
  owner's authorization after the reviewer's Go. The RFC amendments the
  reply owed (B4, B5, B9, B11, B14) are the original architect's and land
  before 019d.
- **Sequencing (R3).** 019a item 4 lands before 019b item 1; 019b item 3
  and 019c item 2 touch the gate tests and are given to one developer or
  landed in that order. What may start at once: 019a items 3 and 4; 019b
  item 2; 019c items 2, 3 and 4. The rest starts with this amendment.

## Amendment 2 to RFC 019 (2026-10-29, after the re-review, revision 3)

By the architect (the independent architect now holds the role; the
original architect's amendment above stands except where this one says
otherwise). The re-review (`arch-002-review-019/RE-REVIEW.md`) found six
mechanisms of the first amendment that do not work as written. The
handoffs are at revision 3 and win over the text above.

- **D5 (N1, N2). The count and the error.** The wrapper owns the count:
  a set of live session ids and a pending-reservation count under one
  lock, so the count is `live + pending` and cannot drift or underflow;
  the SDK closes one session from two places (its `DELETE` handler and
  the worker's exit) and a `has_session` check before delegating would
  miscount concurrent closes. The wrapper's error type is its own, with
  `LimitReached` (Display `session limit reached`) and a transparent
  `Delegate`, because the SDK's error type carries no text. The SDK's
  per-refusal ERROR log line is a recorded residual of THR-27.
- **D6 (N3). Counting that is emitted.** The gate counts every refusal it
  makes or sees under `refusals[("http", reason)]`, and counting runs the
  same once-a-minute flush check as a tool call; without it a flood of
  refused requests with no tool call would never be logged. This
  supersedes "counted" in D6 revised.
- **D8 (N4). The re-run is made from the tag.** The `tag` and
  `build_only` inputs are withdrawn. The attach job runs only when the
  workflow runs on a tag ref and fails unless the workspace version equals
  the tag; the owner re-runs the workflow from the tag after publishing; a
  run from a branch builds artifacts only and is the dry run. A dispatch
  that builds `main` and attaches it under a tag's name is therefore not
  possible. The decision D-C (attach only to an existing release, never
  overwrite, write permission on the attach job only) stands. The image
  job runs on tag refs only. A maintainers page documents the procedure.
- **D2 (N5, N7). One answer, the existing one.** The uniform answer is the
  existing `ToolError::network()` with its existing message; the first
  amendment's new sentence is withdrawn. The two answers are identical,
  their latency is not equalised: a residual, recorded with THR-05.
- **D3 (N6). Rename, in a minor release.** `clean_text` becomes
  `clean_markup_text` as D3 revised says; with the release number settled
  as 0.8.0 (below) a rename is a permitted, announced change before 1.0.0,
  and no deprecated alias is kept. The re-review's alternative (an alias
  to keep a patch release compatible) is withdrawn with the patch.
- **D1 (rules).** `parse_rule` already refuses a rule with an empty label;
  it is unchanged and pinned by a test. A rule is written without the root
  dot. The first amendment's canonicalization of rules is withdrawn.
- **D2 (review of the implementation).** An `http` URL upgraded to
  `https` whose first https attempt cannot connect answers `denied`/
  `scheme` (REQ-FETCH-010) at the robots.txt stage as at the page request,
  so the answer does not depend on the `robots_txt` setting. `Host::name(raw)`
  of the handoff is `Host::from_name(raw)`: a function that builds a host and
  a method that reads one should not share a name.
- **D10 (N9).** 019d removes the changelog's "as they land" sentence and
  uses the preparation date.

### Release number: 0.8.0 (owner's decision, 2026-10-29)

The first amendment assumed a patch release ("do it as a patch release" in
Goals; "release 0.7.1" in Acceptance). That assumption was wrong, and the
architect's reading that kept it was wrong with it:

- Semantic Versioning 2.0.0 asks for a patch only for backward-compatible
  bug fixes and for a minor when functionality is added; 0.7.1 adds a
  configuration key (`max_sessions`), a status (503), a public function and
  release binaries.
- The CHANGELOG preamble says that before 1.0.0 the external contract
  (tool names, parameters, response fields, outcomes, error kinds) may
  change *between minor versions* and that every such change is announced.
  D2 changes an error kind (an unreachable site: `denied`/`robots_txt`
  becomes `unavailable`/`network`) and what a refused name answers.
- ROADMAP: "Patch releases carry fixes only."

Therefore the corrections release is **0.8.0**, and the audit (M8) is
**0.9.0**; the ROADMAP is amended accordingly and the audit's RFC takes the
number 020. Wherever the text above says 0.7.1 or a patch release, read
0.8.0 and a minor release; the contract changes (D2's error kinds, D3's
rename, the 503, the new key) are announced under `Changed` in the
changelog (019d). `tools/CURRENT_MILESTONE` stays `M7`: the requirements are
tagged by milestone and this release adds none beyond REQ-SEC-024, which
carries M7. The plan is `.git-exclude/plans/0.8.0-release-plan.md`.
