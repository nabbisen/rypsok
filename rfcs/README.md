# rypsok RFCs

Index of all RFCs, grouped by lifecycle state. The lifecycle, folder layout,
naming, and numbering rules are defined in
[RFC 000 — RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md).

## Conventions in this project

- The **5-folder variant** is used:

  | Folder | State |
  |---|---|
  | `draft/` | Draft (optional) |
  | `proposed/` | Proposed — open for review |
  | `accepted/` | Accepted — review complete; implementer may start |
  | `done/` | Implemented |
  | `archive/` | Withdrawn or Superseded |

- A state folder is created when the first RFC enters that state.
- Companion implementation handoffs live under `handoffs/NNN-slug/` and
  inherit their state from the matching RFC.
- Release tags carry no `v` prefix, so an implemented RFC reads
  `**Status.** Implemented (0.1.0)`.
- The architect writes and accepts RFCs under the owner's authorization of
  `ROADMAP.md`; the owner may object to any RFC before its handoff is
  taken up.
- An RFC may amend the baseline specifications in `docs/src/maintainers/`
  under a section "Amendment to the baseline", or an earlier accepted RFC
  under "Amendment to RFC NNN"; the amendment is applied to the document
  in the same commit.
- Handoff order within M1 follows the dependency lines in each handoff:
  003 and 004 first (independent), then 005, 006, 008, 009 (independent of
  each other), then 007, whose wiring PR joins them and whose measurements
  feed the M1 review.

## Proposed

_None yet._

## Accepted

| ID | Title | Milestone | Handoff |
|----|-------|-----------|---------|
| 019 | [Corrections from the first independent review](./accepted/019-first-independent-review.md) | 0.7.1 | [019a](./handoffs/019a-core-corrections/README.md), [019b](./handoffs/019b-listener-and-http-docs/README.md), [019c](./handoffs/019c-release-ci-records/README.md) |

Review records are not in the repository: the owner keeps review requests and reviewed packages under a private directory, which earlier handoffs name as `.git-exclude/reviews/`, today `.git-exclude/review-requests/` and `.git-exclude/reviewed/`.

## Implemented

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | Project start (pre-release) |
| 001 | [Workspace layout and crate boundaries](./done/001-workspace-layout.md) | 0.1.0 |
| 002 | [Test strategy and CI gates](./done/002-test-strategy.md) | 0.1.0 |
| 003 | [External contract](./done/003-external-contract.md) | 0.1.0 |
| 004 | [Configuration schema and limits](./done/004-configuration-and-limits.md) | 0.1.0 |
| 005 | [Content boundary](./done/005-content-boundary.md) | 0.1.0 |
| 006 | [Engine abstraction, registry, admission; Wikipedia and Brave](./done/006-engines-and-admission.md) | 0.1.0 |
| 007 | [Aggregation](./done/007-aggregation.md) | 0.1.0 |
| 008 | [URL comparison keys, deduplication, rank fusion](./done/008-urls-dedup-ranking.md) | 0.1.0 |
| 009 | [Query privacy and logging](./done/009-privacy-and-logging.md) | 0.1.0 |
| 010 | [Health, circuit breaking, provider limits, `engine_status`](./done/010-health-and-circuit.md) | 0.2.0 |
| 011 | [Egress policy and SSRF defense](./done/011-egress-policy.md) | 0.3.0 |
| 012 | [Fetch pipeline: `web_fetch`, extraction, robots.txt](./done/012-fetch-pipeline.md) | 0.3.0 |
| 013 | [Streamable HTTP, listener protections, authentication, per-caller limits](./done/013-streamable-http.md) | 0.4.0 |
| 014 | [Result cache](./done/014-result-cache.md) | 0.5.0 |
| 015 | [Scraping drivers as an opt-in engine type](./done/015-scraping-drivers.md) | 0.5.0 |
| 016 | [1.0.0: the stable contract](./done/016-stable-contract.md) | 0.6.0 |
| 017 | [User documentation, tutorials and checked examples](./done/017-user-documentation.md) | 0.6.0 |
| 018 | [Threat model revision and hardening](./done/018-threat-model-revision.md) | 0.7.0 |

## Archive

_None yet._
