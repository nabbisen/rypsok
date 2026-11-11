# RFC 021 — Enforced call maxima, shown by `check`; every tool-definition change announced

**Status.** Accepted (2026-11-11) by the owner, on two requests of the mimlys
team, rypsok's first consumer (letter of 2026-11-11). The owner asked that
both be checked against the project's principles first; this RFC is the
result.
**Tracks.** The next minor release. Requirements: REQ-AGG-003, REQ-PAGE-008,
REQ-MAIN-006, REQ-OPS-001.
**Handoffs.** [`021a`](../handoffs/021a-call-maxima-and-definition-changes/README.md).

## Problem

1. **A host wants each tool's maximum call time, from `rypsok check`.**
   - **Today.** `check` prints the deadlines (`deadline_ms=2500`,
     `deadline_ms=10000`), not the maxima of a call: LIM-S7 for a search
     (deadline + 300 ms) and LIM-F5 for a fetch (deadline + 1000 ms).
   - **What verification found.** **LIM-S7 is promised but not
     enforced.** REQ-AGG-003 says "a `web_search` call shall never last
     longer than LIM-S7". In the code, `Limits::search_call_max_ms` is
     called by no production path, and no test checks a search call
     against it. The verification ledger lists this as not asserted.
     LIM-F5 bounds the parse step of a fetch, not the whole call.
   - **The consequence.** Printing those numbers today would publish, for
     a program to build its timeouts on, bounds that nothing guarantees.
2. **A host that pins tool definitions needs every change of one
   announced.**
   - **The case.** mimlys pins each tool's definition per task, so any
     change, however small, unpins the tool for running tasks.
   - **What happened in 0.9.0.** The changelog named all three
     definition changes, but scattered them, two of them inside a line
     about error messages.
   - **What the pin does not catch.** The fixture test pins the full
     definitions, so a change cannot happen unnoticed by us, but it does
     not check the changelog.

## Design

**D1. The maxima are hard bounds of the whole call.**
- **The bounds.** A `web_search` call lasts at most LIM-S7 and a
  `web_fetch` call at most LIM-F5. Each is measured from the moment the
  core receives the call (`Rypsok::search_as`, `Rypsok::fetch_as`) to its
  answer.
- **How they are enforced.** By one small function that bounds a future
  by a deadline, used by both tools.
- **On expiry.** A call ends with the error it already has for running
  out of time: `unavailable` with the reason `deadline`, and the message
  each tool uses for its deadline. **No new error kind, reason or
  notice:** the vocabulary stays as hosts pass it on. Dropping the
  future cancels the engine tasks within LIM-O2.
- **The pipeline is unchanged.** The bound is the outer guarantee, and
  the pipeline's own deadlines still decide the normal answer.

**D2. `check` prints the maxima.** One line of its own, in the form of the
rate-limits line:

    rypsok: tool call maxima in ms: web_search=2800 web_fetch=11000; engine_status reads memory only

- **The values** are the effective ones for the configuration checked.
- **`engine_status`** has no maximum, because it answers from memory,
  so the line says that and invents no number.
- **For a program.** The configuration page says that `check`'s text is
  for people and may change in a minor version (the stability page). For
  a program, the stable source is its own configuration plus the two
  fixed margins in the limits table.

**D3. Every tool-definition change is announced on its own line.**
- **What counts.** A change to a tool's name, description, input schema,
  output schema or annotations, or to the server instructions. The server
  version is excluded, since it changes with every release.
- **Where it goes.** A line under `### Tool definitions` in the changelog
  section of the release. The line names the tool and the field and says
  what changed.
- **How it is enforced.** A job of the release checks compares the
  pinned definitions at the release commit with those at the latest
  release tag. It fails when they differ and the section has no such
  heading. It runs once per release (RFC 020, D-23).

## Compatibility

- **D1.** It changes nothing in an answer that ends in time. A call that
  would have run past its maximum now ends at it, with the existing
  error.
- **D2.** It adds a line to `check`'s output, announced in the changelog.
- **D3.** A process rule and one more release-check job.

## Tests

- **D1.** A unit test of the bounding function with a future that never
  ends. Paused-time tests that a search and a fetch end by their maxima,
  with the existing error. These close the open row of REQ-AGG-003 in the
  verification ledger.
- **D2.** `check` output tests.
- **D3.** Gate tests of the new job.

## Alternatives considered

- **Printing the numbers now, as the request was first answered.**
  Rejected: an unenforced bound must not be published as a maximum.
- **A machine-readable form of `check`, or the maxima in the tool
  definitions.** Not now: either is an addition to the contract, and it
  is the owner's decision.
- **A rule without enforcement.** Rejected: a rule that rests on memory
  is forgotten exactly once, which is once too often for a host that pins
  definitions.

## Amendment (2026-11-11, after implementation in 021a)

The Problem section says that LIM-S7 was "promised but not enforced".
**That is true of the code, not of the behavior.**
- **The code.** Nothing named LIM-S7 bounded a call, and no test asserted
  it.
- **The behavior.** After the fan-out, the search pipeline does not await,
  and its fan-out ends at LIM-S4 − LIM-S6, before LIM-S7. A fetch's inner
  bounds end at or before LIM-F5. Both maxima therefore already held by
  construction.
- **What 021a added.** The outer bound (`deadline::within`) that keeps
  them true if the pipelines change, and the tests that pin them.
- **What it changes for hosts.** Nothing that a host could have observed.
