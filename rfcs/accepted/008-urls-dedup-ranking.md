# RFC 008 — URL comparison keys, deduplication, rank fusion

**Status.** Accepted (2026-09-30) — implementer may start after RFC 007.
**Tracks.** Milestone M1; requirements REQ-RANK-001 to -007, REQ-ARCH-004,
REQ-CTX-001; external design §3.1, §6; threat THR-24.
**Touches.** `rypsok-core::search::{normalize, dedup, rank}`.
**Handoff.** [`../handoffs/008-urls-dedup-ranking/README.md`](../handoffs/008-urls-dedup-ranking/README.md)

## Summary

Two forms of a URL: the returned URL, changed as little as possible, and an
internal comparison key, normalized aggressively and never returned. Results
with the same key merge into one with a list of observations. The order
comes from reciprocal rank fusion in integer arithmetic with explicit
tie-breakers.

## Design

### Returned URL (REQ-RANK-003)

Starting from the driver's URL (already unwrapped from any provider redirect
wrapper by the driver): parse with the `url` crate; refuse the result if the
scheme is not `http` or `https` or the URL has credentials (the result is
dropped, `results_dropped` with reason `url_invalid`) or exceeds LIM-F9
(reason `url_too_long`);
remove the query parameters of the tracking list; write the host in its
ASCII (IDNA) form; keep everything else, including case, fragment, trailing
slash and port.

Tracking list (a constant, reviewable): `utm_source`, `utm_medium`,
`utm_campaign`, `utm_term`, `utm_content`, `utm_id`, `fbclid`, `gclid`,
`dclid`, `msclkid`, `mc_cid`, `mc_eid`, `igshid`, `yclid`, `_hsenc`,
`_hsmi`, `ref_src`.

### Comparison key (REQ-RANK-002)

From the returned URL: scheme lower-cased and `http` mapped to `https`;
host lower-cased, ASCII form, a leading `www.` removed; default port
removed; path with percent-encoding normalized (uppercase hex, unreserved
characters decoded) and a trailing slash removed except for the root; query
parameters sorted by name with the tracking list already removed; fragment
removed. The key is a `String` used only for equality. It is never returned
and never fetched.

### Deduplication and merge (REQ-RANK-004)

Results are grouped by key in the order of the engine selection. A merged
result keeps the returned URL, title, snippet and date of the observation
with the best rank; ties break by engine preference. `engines` is the list
of engine identifiers in selection order, present in the response only when
the operator enabled `search.engine_attribution` (RFC 004) or when any contributing
engine has `attribution_required` (REQ-CTX-001, REQ-CMP-003).

### Rank fusion (REQ-RANK-005)

Reciprocal rank fusion with `k = 60`: each observation contributes
`SCALE / (k + rank)` where `rank` starts at 1 and `SCALE = 1_000_000`, in
integer arithmetic. Contributions are summed per result in the order of the
engine selection. A result seen by two engines outranks a result seen by
one at the same rank, which is the cross-engine agreement REQ-RANK-005 asks
for.

### Total order (REQ-RANK-006)

Sort by score descending, then best rank ascending, then the preference of
the engine that gave the best rank ascending, then comparison key ascending.
Then cut to `max_results` (REQ-RANK-007).

### Tests

Unit tests for every normalization rule with a table of inputs and expected
keys, including IDNA hosts, percent-encoding variants, tracking parameters,
`www.` and default ports; a property test that the key is idempotent and
that two URLs with the same key produce the same returned host; fusion tests
with hand-computed scores; the determinism test of RFC 007.

## Alternatives considered

- **Returning the comparison key.** Rejected: it may not resolve to the same
  resource (startup review C-07).
- **Score-based fusion using provider scores.** Rejected: provider scores
  are not comparable and mostly absent.
- **Floating-point fusion.** Rejected: arrival-order sums can differ in the
  last digit (R-08).

## Amendment to the baseline

External design §3.1 gains the `results_dropped` reason `url_invalid`
(a result whose URL has no `http` or `https` scheme or carries
credentials). Applied with this RFC.

## Open questions

- Whether `m.` mobile hosts should be folded like `www.`. Decided here: no,
  until a corpus shows the need.
