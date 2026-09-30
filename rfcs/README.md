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
  under a section "Amendment to the baseline"; the amendment is applied to
  the document in the same commit.
- Handoff order within M1 follows the dependency lines in each handoff:
  003 and 004 first (independent), then 005, 006, 008, 009 (independent of
  each other), then 007, whose wiring PR joins them and whose measurements
  feed the M1 review.

## Proposed

_None yet._

## Accepted

| ID | Title | Milestone | Handoff |
|----|-------|-----------|---------|
| 001 | [Workspace layout and crate boundaries](./accepted/001-workspace-layout.md) | M0 | [handoffs/001-workspace-layout](./handoffs/001-workspace-layout/README.md) |
| 002 | [Test strategy and CI gates](./accepted/002-test-strategy.md) | M0 | [handoffs/002-test-strategy](./handoffs/002-test-strategy/README.md) |
| 003 | [External contract](./accepted/003-external-contract.md) | M1 | [handoffs/003-external-contract](./handoffs/003-external-contract/README.md) |
| 004 | [Configuration schema and limits](./accepted/004-configuration-and-limits.md) | M1 | [handoffs/004-configuration-and-limits](./handoffs/004-configuration-and-limits/README.md) |
| 005 | [Content boundary](./accepted/005-content-boundary.md) | M1 | [handoffs/005-content-boundary](./handoffs/005-content-boundary/README.md) |
| 006 | [Engine abstraction, registry, admission; Wikipedia and Brave](./accepted/006-engines-and-admission.md) | M1 | [handoffs/006-engines-and-admission](./handoffs/006-engines-and-admission/README.md) |
| 007 | [Aggregation](./accepted/007-aggregation.md) | M1 | [handoffs/007-aggregation](./handoffs/007-aggregation/README.md) |
| 008 | [URL comparison keys, deduplication, rank fusion](./accepted/008-urls-dedup-ranking.md) | M1 | [handoffs/008-urls-dedup-ranking](./handoffs/008-urls-dedup-ranking/README.md) |
| 009 | [Query privacy and logging](./accepted/009-privacy-and-logging.md) | M1 | [handoffs/009-privacy-and-logging](./handoffs/009-privacy-and-logging/README.md) |

## Implemented

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | Project start (pre-release) |

## Archive

_None yet._
