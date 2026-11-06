# Handoff 020u — Fuzz targets for the parsers that had none

**Created 2026-11-07.** A fix handoff of the audit (RFC 020, D3), wave 3.
Finding TST-14 (Medium for the policy and robots.txt, Low for the rest):
parsers of untrusted input with no fuzz target. REQ-SEC-023 lists the eight
parsers that had one.

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), RFC 018 (fuzzing).
**Rules.** As 020q. Evidence to `.git-exclude/evidence/020u/`. Each new target
runs 120 s locally before hand-in (state runs, crashes); a crash found is
**not fixed here**: it becomes a test and a finding in the review request, and
the architect decides.
**Depends on.** Nothing.

## Change scope

| # | New target (`fuzz/fuzz_targets/`) | Input and invariant |
|---|---|---|
| 1 | `judge_url` | any text as a URL through `EgressPolicy::judge_url` of a default policy: no panic; an admitted URL is `https` (or `http` with `allow_http`), has no userinfo, a port in the policy's set, and a host that is a name or an address the table admits after canonicalization (re-judge the canonical form: same verdict) |
| 2 | `parse_rule` | any text as an `egress.allow`/`deny` rule: no panic; an accepted rule round-trips (its display parses to the same rule) |
| 3 | `robots_txt` | any bytes as a robots.txt body with any status (the status logic and the matcher): no panic; the decision for a path is the same twice; a body over LIM-F13 is judged on its prefix only |
| 4 | `engine_json` | any bytes as a Wikipedia response and as a Brave response, through each driver's response parser: no panic; every result has an `http`/`https` URL and passes `returned_url`; at most the requested count |
| 5 | `headers` | any bytes as `Retry-After` (both the engine and the fetch reader) and as a `Content-Type` charset parameter: no panic; a parsed `Retry-After` is within its bound |

The functions these targets need may be private today. Make each reachable
from the fuzz crate as a `pub` item marked `#[doc(hidden)]` with a doc line
"public for the fuzz targets; not part of the stable surface" (the stability
page already puts everything outside `model` and the facade outside the
promise, 020m). Name each in the review request. No behaviour change.

| # | Also | |
|---|---|---|
| 6 | Seed corpora: `fuzz/corpus/<target>/` with a handful of real-shaped inputs each (the existing fixtures where they fit; **no token-shaped strings**) | |
| 7 | `scheduled.yml`: the job runs every target (it lists them with `cargo fuzz list`; the timeout of 60 minutes must still fit: 14 targets × 300 s = 70 min; **reduce to 240 s each** and set the timeout to `targets × 4 + 15`; the 020o gate test that ties the timeout to the target count is updated) | |
| 8 | `requirements.md` REQ-SEC-023: the list of parsers gains the five; the external design's fuzzing line likewise; changelog under Added (internal) | |

## Explicit non-change scope

No fix of a crash found (report it); no change of a parser's behaviour.

## Required evidence and review request

The gates; the 120 s local runs per target; the review request at
`.git-exclude/review-requests/020u-fuzz-targets/README.md`.
