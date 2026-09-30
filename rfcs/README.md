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

## Proposed

_None yet._

## Accepted

| ID | Title | Milestone | Handoff |
|----|-------|-----------|---------|
| 001 | [Workspace layout and crate boundaries](./accepted/001-workspace-layout.md) | M0 | [handoffs/001-workspace-layout](./handoffs/001-workspace-layout/README.md) |
| 002 | [Test strategy and CI gates](./accepted/002-test-strategy.md) | M0 | [handoffs/002-test-strategy](./handoffs/002-test-strategy/README.md) |

## Implemented

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | Project start (pre-release) |

## Archive

_None yet._
