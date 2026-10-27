# RFC 018 — Threat model revision and hardening

**Status.** Accepted (2026-10-27) — implementer may start at once.
**Tracks.** Milestone M7 (`ROADMAP.md`, accepted 2026-10-27); requirements
REQ-SEC-018, REQ-SEC-020 (amended), REQ-SEC-022 and REQ-SEC-023 (new),
REQ-SEC-013, REQ-SEC-016, REQ-PRIV-004, REQ-TEST-013, REQ-DEP-003;
external design §1, §8, §10, §15, §16 (I5 amended); the threat register
as a whole.
**Touches.** `docs/src/maintainers/threat-model.md` (new),
`docs/src/maintainers/threat-register.md` (revised), the limits table
(LIM-H4, LIM-H5), `rypsok-core::config` (`[service.http] max_streams`,
`request_timeout_s`), `rypsok::mcp::http` (the request time bound, the
stream bound), `fuzz/` (new), `.github/workflows/` (pinned actions, the
fuzz job), `docs/src/reference/deployment.md`, the requirements.
**Handoff.** [`../handoffs/018-threat-model-revision/README.md`](../handoffs/018-threat-model-revision/README.md)

## Summary

The register of 2026-09-30 was written before the system existed. This
RFC walks the delivered system flow by flow, writes the threat model as a
page of its own (assets, attackers, flows, boundaries), corrects and
extends the register so that every threat names a control and a test,
and does the hardening the walk calls for: a time bound and a stream
bound on the listener, fuzz targets on every parser that reads external
bytes, CI actions pinned to commit hashes, an explicit rule for provider
text in logs, and hardened deployment profiles. A security review of the
result is recorded as a release-readiness report in the package.

## Design

### The walk, and what it found

Every flow of external design §1 was read against the code at 0.6.0:
B1 (caller → rypsok), B2 (rypsok → engines), B3 (rypsok → sites), B4
(rypsok → caller), B5 (operator → rypsok), B6 (network → listener).
The findings, in the order of severity:

| # | Finding | Consequence |
|---|---|---|
| W1 | **The listener has no bound on the time a request may take before its handler runs, nor on the streams one HTTP/2 connection may open.** A client that sends a body one byte at a time holds one of LIM-H1 connections for as long as it likes; one HTTP/2 connection may open hyper's default of 200 streams. The header read timeout (LIM-H3) covers only the headers. | THR-26 (new); LIM-H4, LIM-H5; REQ-SEC-022 |
| W2 | **Provider error text reaches the log at `debug`.** `debug_provider_text` writes at most 200 characters of a provider's error body; invariant I5 says "no provider text in a log" without a level. Either the code or the invariant is wrong. | I5 amended to state the `debug` exception and its bound; THR-04 unchanged |
| W3 | **CI actions are referenced by moving tags** (`@v4`, `@master`, `@cargo-deny`). A compromised tag runs in the project's CI with the tokens it has. | THR-22 extended; REQ-SEC-020 amended; a test that every action is pinned |
| W4 | **No parser that reads external bytes is fuzzed.** The scheduled job's fuzz step is a placeholder since M0; property tests cover the cleaner only. | REQ-SEC-023; `fuzz/` with one target per parser |
| W5 | **The register carries two THR-20 rows** (the M5 scraping row collided with the baseline's impersonation row), and its review log stops at the baseline. | Renumbered; the log catches up |
| W6 | **The deployment profiles are permissive**: the systemd unit lacks the usual confinement, the `docker run` line publishes a port and nothing else. | Hardened profiles on the deployment page |
| W7 | Verified with no change: decompression is bounded on the decoded side for engines and fetches (REQ-SEC-013); every address is judged at every resolution and address literals are judged like names (REQ-SEC-007); the pool reuses connections to the judged address only; tokens and keys are `Secret` and print `[redacted]`; `check` prints names, never values; the proxy URL is never logged; stdout carries protocol messages only; the gate answers before any JSON is parsed; drivers' panics are isolated; cache keys hold the query text in memory for LIM-K2 and the page says so. | Recorded in the threat model page as verified controls |

### The threat model page

`docs/src/maintainers/threat-model.md`: the assets (the host's network,
the operator's keys and tokens, the provider quotas, the caller's
queries, the process's availability, the honesty of every response);
the attackers (a steered model, a hostile page or snippet, a provider
that drifts or blocks, a network peer in HTTP mode, a dependency); the
flows F1 to F8 with their boundary, what crosses, and the controls on it;
and the rule for reading the register. The register stays the table of
record; the page is how to read it.

### The register, revised

- THR-20 (scraping) becomes THR-25; THR-20 is the impersonation row again.
- THR-26 "A slow or stream-flooding HTTP client exhausts the listener"
  (B6): controls REQ-SEC-018, REQ-SEC-022; verified by REQ-TEST-014.
- THR-22 reads "a vulnerable, license-incompatible or compromised
  dependency, including a CI action"; controls REQ-SEC-020.
- Residual risks gain: queries retained in memory while the cache is on
  (operator's choice, documented); 200 characters of a provider's error
  body at `debug` (the operator who turns `debug` on reads logs as data).
- The review log gains one row per milestone M1 to M7 naming what each
  added.

### Hardening

**H1. Request time and stream bounds (W1).** Two keys under
`[service.http]`:

| Key | Default | Range | Limit |
|---|---|---|---|
| `request_timeout_s` | 30 | 3–600, and at least `fetch.deadline_ms` + 2000 ms | LIM-H5 |
| `max_streams` | 16 | 1–256 | LIM-H4 |

The gate wraps the SDK's service in a timeout of LIM-H5 for the whole
request: body, dispatch and handler. On expiry the answer is `408` with
an empty body and the log event `http_refused` with reason
`request_timeout`; the handler's own deadline is shorter by construction
(the validation rule), so a timed-out request is a slow client, not a
slow tool. hyper's HTTP/2 builder gets `max_concurrent_streams(LIM-H4)`.

**H2. Provider text in logs (W2).** I5 reads: "No provider text appears
in an envelope, error, status or log at `info` or above; at `debug` a
driver may log at most 200 characters of a provider's error body." The
bound is a constant beside `debug_provider_text` and a test holds it.

**H3. Pinned actions (W3).** Every `uses:` in `.github/workflows/`
names a full commit hash with the version as a comment. A test in the
gates file asserts it for every workflow. The moving `@cargo-deny` tag
becomes the pinned installer with `tool: cargo-deny`.

**H4. Fuzz targets (W4).** `fuzz/` (cargo-fuzz, its own workspace) with
one target per parser of external bytes: `clean_text`, `clean_content`,
`html_document` (charset, DOM, extraction, Markdown through
`render_document`), `charset_decode`, `scrape_read` (the DuckDuckGo
shape), `returned_url`, `privacy_check`, `config_parse`. Each target
asserts the function returns (no panic, no overflow) and, where the
bound is the point, that the output stays within it. The scheduled job
builds them on nightly and runs each for a fixed time, report only
(RFC 002); the implementer runs each once locally before the package.

**H5. Deployment profiles (W6).** The systemd unit gains `ProtectHome`,
`PrivateTmp`, `PrivateDevices`, `RestrictAddressFamilies=AF_INET
AF_INET6`, `RestrictNamespaces`, `LockPersonality`,
`MemoryDenyWriteExecute`, `SystemCallFilter=@system-service`,
`SystemCallArchitectures=native`, `CapabilityBoundingSet=`,
`MemoryMax`, `TasksMax`, `UMask=077`; the `docker run` line gains
`--read-only`, `--cap-drop ALL`, `--security-opt no-new-privileges`,
`--memory`, `--pids-limit`. Documentation only; the image is unchanged.

### Requirements (amendment to the baseline)

- **REQ-SEC-022 Request time and stream bounds — M7 · T.** In HTTP mode
  a request shall be ended with `408` after LIM-H5 whatever its state,
  and one HTTP/2 connection shall open at most LIM-H4 streams.
- **REQ-SEC-023 Fuzzing — M7 · D.** Every parser that reads external
  bytes shall have a fuzz target that runs on the schedule of
  REQ-TEST-003, report only.
- **REQ-SEC-020** gains: "CI actions are pinned to commit hashes."
- Limits: LIM-H4, LIM-H5.
- External design §16 I5 as in H2.

### Security review

The package records the review as a release-readiness report: each
finding with likelihood, impact, detection, mitigation and residual,
and a Go/No-Go for 0.7.0.

## Non-goals

No TLS in the binary (the proxy's); no OAuth (DR-010); no sandboxing in
the binary beyond what the profiles give; no change to the egress
policy's ranges; no new tool or response field.

## Alternatives considered

- **A body-read timeout only.** Rejected: a bound on the whole request
  is simpler to state ("a request takes at most LIM-H5") and covers the
  handler as a second fence behind the call deadlines.
- **Dependabot for actions instead of a test.** Complementary, not a
  substitute: the test fails the build when an unpinned reference
  appears; the bot proposes updates.
- **Fuzzing in the CI gate.** Rejected: fuzzing is open-ended; it
  reports on the schedule and its findings become tests.

## Open questions

- Whether `request_timeout_s` should be derived from the fetch deadline
  instead of configured. Decided here: configured with a validation rule;
  an operator reads one number and the rule says why.
