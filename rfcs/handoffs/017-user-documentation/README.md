# Handoff — RFC 017: User documentation, tutorials and checked examples

**RFC.** [`../../done/017-user-documentation.md`](../../done/017-user-documentation.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-DOC-001
to -004, REQ-OPS-001, REQ-MCP-011, REQ-CTX-002),
`docs/src/maintainers/external-design.md` §1.2, §13.1, `ROADMAP.md` M6.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** Release 0.5.0.
**Milestone.** M6. Deliverable 7 closes the milestone (release 0.6.0 on
the owner's authorization).

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `examples/configs/` (first-search, keyed-engine, reading-a-page, network-mode, scraping, minimal, cache, strict-fetch), `examples/hosts/` (stdio, http), `examples/calls/` (discover; web_search ok, partial, empty, denied; web_fetch ok, continuation, redirected, denied; engine_status), `examples/README.md` | `tests/examples.rs`: every config validates and `check` exits 0; every host entry has the shape; every call's arguments parse and its response round-trips through the response type |
| 2 | `tests/examples.rs` end-to-end: the first tutorial's two lines through the binary over stdio against a `wiremock` Wikipedia mock | The answer's outcome and fields match the sample; the test is named `req_doc_002_…` |
| 3 | The user part of the book: getting started (short), tutorials 1 to 5, each including its files from `examples/` | `mdbook build` green with `create-missing = false`; every `{{#include}}` resolves |
| 4 | "Reading a response" | The four tables of the RFC; the prompt paragraph |
| 5 | "Troubleshooting" | The symptom table of the RFC, each row with cause and fix |
| 6 | README quick start and the introduction revised; `docs.yml` workflow (build always, deploy when Pages is enabled) | README under 200 lines; the workflow green on push |
| 7 | M6 closing: requirements §22 gains REQ-DOC-001 to -004; `tools/CURRENT_MILESTONE = M6`; coverage gate green; `CHANGELOG.md` Unreleased for 0.6.0; the review request package | CI green with the M6 gate |

## 2. Conventions that the reviewer will check

- A tutorial shows a file by `{{#include ../../../examples/…}}`, never by
  a copy; a command shown is one the test runs or one `check` validates.
- Sample answers are real answers: produced by running the binary against
  the mock, then edited only to shorten snippets, with the edit noted in
  `examples/README.md`.
- Pages speak to a person: what you will have, what you need, steps,
  what you should see, what to do if not. No project vocabulary
  (milestones, RFC numbers) in the user part.
- Nothing in the maintainers' part changes except the four requirements.

## 3. Tests

`req_doc_002_example_configurations_validate`,
`req_doc_002_example_host_entries_have_the_shape`,
`req_doc_002_example_calls_match_the_contract`,
`req_doc_002_first_tutorial_runs_end_to_end`, in
`crates/rypsok/tests/examples.rs`.

## 4. Review request package

`.git-exclude/review-requests/017-user-documentation/README.md`: the
gate results, the list of pages and files, the sample answers' provenance
(the command that produced each), open points for the owner (Pages).
