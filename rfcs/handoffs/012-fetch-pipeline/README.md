# Handoff — RFC 012: Fetch pipeline, extraction, robots.txt

**RFC.** [`../../done/012-fetch-pipeline.md`](../../done/012-fetch-pipeline.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-FETCH-001
to -011, REQ-PAGE-001 to -008, REQ-CTX-005, REQ-CMP-005, REQ-CMP-006,
REQ-SEC-012, REQ-PERF-003, REQ-TEST-006), `docs/src/maintainers/external-design.md`
§3.2, §4, §9, §14, `docs/src/maintainers/limits-and-defaults.md` LIM-F1 to
LIM-F15, LIM-C3 to LIM-C5, LIM-C7, `docs/src/maintainers/threat-register.md`
THR-01, THR-03, THR-07, THR-11, THR-12, THR-14, `ROADMAP.md` M3.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 011 PR 1 to 3 (policy, agent client, strict mode).
**Milestone.** M3. Deliverable 10 closes the milestone.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `fetch::run` with validation, URL privacy check, strict mode, policy, the scheme rule, concurrency permits (LIM-C3, LIM-C4, LIM-C5) and the deadline LIM-F5 | Tests per refusal and per permit path with `wiremock`; `busy` when LIM-C3 + 1 fetches run |
| 2 | `fetch::robots`: cache per origin (LIM-F12, LIM-F15), bounded read (LIM-F13), `robotstxt` parsing, the 2xx/3xx/4xx/5xx table | Tests per row; a disallowed URL is `denied`, `robots_txt`; a 5xx robots.txt disallows for one hour |
| 3 | Request, manual redirects, status handling | Tests for every row of the redirect table, including the undisclosed refused target; a 4xx body never returned |
| 4 | Media type, sniffing, streaming byte bound, charset decoding | `too_large` under gzip at exactly LIM-F3; `unsupported_content` with a bounded `media_type`; fixtures 06 to 09 decode correctly |
| 5 | `BoundedSink`, iterative extraction with the depth bound, the chrome list, the candidate scoring, the title | Fixtures 01 to 05, 10 to 15 pass their `expected.toml`; fixtures 11 and 12 end the call and leave the process alive |
| 6 | Markdown renderer and `boundary::clean_content` with the window | Rendering tests per rule; property tests: output within `max_chars`, idempotent cleaning, `next_start_char` continues without overlap or gap |
| 7 | `web_fetch` tool definition, adapter wiring, per-tool limiter LIM-C7 | Protocol tests: the end-to-end fetch of fixture 01 through stdio; the error kinds `denied`, `redirected`, `unavailable`/`http_status`, `unsupported_content`, `too_large`; tool definitions under LIM-I1 |
| 8 | The fixture corpus `fixtures/pages/01` to `15` with `expected.toml` and a README | Hand-written, no third-party content; each fixture names what it exercises |
| 9 | `tools/measure-memory` and the memory report (REQ-PERF-003) | Numbers in the review package; a proposal for LIM-F10 if the tree cost exceeds 64 MiB |
| 10 | M3 closing: `tools/CURRENT_MILESTONE = M3`; coverage gate green; documentation of RFC 012 (tools page, configuration page, integration guide); `CHANGELOG.md` for 0.3.0; threat register review of THR-01, THR-03, THR-11, THR-12, THR-14; the security audit of the project rules recorded in the package | CI green with the M3 gate |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | Pipeline skeleton: validation, privacy, strict mode, policy, concurrency, request, redirects, status, bytes; `wiremock` tests | RFC 011 PR 1 to 3 |
| 2 | robots.txt cache and parsing | 1 |
| 3 | Charset, `BoundedSink`, extraction, Markdown, `clean_content`, the corpus | — (pure functions; may proceed in parallel with 1) |
| 4 | Wiring, `web_fetch` tool, protocol tests | 1, 2, 3 |
| 5 | Memory measurement, documentation, milestone closing | 4 |

## 3. Conventions that the reviewer will check

- The order of REQ-PAGE-001 is the order of the code; no I/O before the
  policy, no parsing before the byte bound.
- No recursion in parsing, extraction or rendering; every traversal is an
  explicit stack. A test with fixture 11 runs under a 1 MiB thread stack.
- Bodies are read through `egress::read_body`; `Content-Length` is never
  trusted; a 4xx or 5xx body is never parsed.
- `spawn_blocking` is the only place parsing runs, under the LIM-C5 permit;
  nothing in `fetch` holds a lock across an await (`await_holding_lock`
  stays denied).
- The URL is `Sensitive` end to end; `host` is the only URL part that may
  reach a log, and only after the policy admitted it.
- New dependencies: `html5ever` (with `markup5ever`), `encoding_rs`,
  `chardetng`, `robotstxt`; nothing else without a question in the package.
  Licenses pass `cargo deny`.
- The tool description carries the three sentences of RFC 012 verbatim.

## 4. Review request package

Hand back `.git-exclude/reviews/012-fetch-pipeline/README.md` with links to
this handoff and RFC 012, the PR list, the corpus results, the resource
test log, the memory report, the threat review, the security audit, and any
deviation as a question.
