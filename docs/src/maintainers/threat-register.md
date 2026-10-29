# Threat Register

**Status:** Baseline 1.0 (2026-09-30), revised 2026-10-27 (RFC 018; read with the [threat model](./threat-model.md)). A living document: it is reviewed at
every milestone and whenever a data flow, an external integration or an
authentication mechanism changes (project rule "Release Deliverables").

Boundaries B1 to B6 are defined in requirements §3. Every threat names its
controls as requirements and the tests that verify them.

| ID | Threat | Boundary | Impact | Controls | Verified by |
|---|---|---|---|---|---|
| THR-01 | Indirect prompt injection: a page or snippet carries text meant to steer the model | B3, B4 | Unsafe agent actions | REQ-SEC-001, REQ-SEC-002, REQ-SEC-003, REQ-CTX-002, REQ-CTX-005, REQ-SEC-015 | REQ-TEST-007 |
| THR-02 | Boundary forgery: external text imitates the envelope, a field, or a closing marker | B4 | Injection appears as rypsok's own statement | REQ-SEC-002, REQ-CTX-003, REQ-SEC-016 | REQ-TEST-007 |
| THR-03 | Hidden and invisible content: `display:none`, comments, zero-width and tag characters, bidirectional controls | B3 | Instructions invisible to a reviewer | REQ-CTX-005, REQ-SEC-015 | REQ-TEST-007 |
| THR-04 | Injection through the error path: provider or page text in an error, status or log | B2, B3 | Injection bypasses the boundary | REQ-SEC-016, REQ-STATUS-002, REQ-ERR-004 | REQ-TEST-007 |
| THR-05 | SSRF: a fetch reaches loopback, private, link-local, metadata or other internal destinations | B3 | Internal network access | REQ-SEC-005, REQ-SEC-006, REQ-SEC-009 | REQ-TEST-005 |
| THR-06 | Check-to-connect gap: the address validated is not the address connected to (DNS rebinding, address literals bypassing the resolver, environment proxies) | B3 | SSRF despite validation | REQ-SEC-007 | REQ-TEST-005 |
| THR-07 | Redirect abuse: a public URL redirects to an internal destination or to a site the caller did not approve | B3 | SSRF; bypass of a host's own limits | REQ-SEC-008, REQ-FETCH-006 | REQ-TEST-005 |
| THR-08 | Credential leakage on redirect: an engine's key header sent to another origin | B2 | Key theft | REQ-SEC-017 | REQ-TEST-015 |
| THR-09 | Exfiltration through outbound parameters: a steered agent encodes secrets in a query or URL | B1, B3 | Disclosure to a third party | REQ-PRIV-002, REQ-SEC-006 (operator lists), REQ-SEC-021 (strict mode), host expectation 4 | REQ-TEST-008, REQ-TEST-005 |
| THR-10 | Accidental secret in a query or URL | B1, B2 | Disclosure to a provider | REQ-PRIV-002 | REQ-TEST-008 |
| THR-11 | Resource bomb: oversized, compressed, deeply nested or slow responses | B2, B3 | Memory or CPU exhaustion, process death | REQ-PAGE-003, REQ-PAGE-006, REQ-PAGE-008, REQ-SEC-010, REQ-SEC-011, REQ-SEC-013, REQ-SEC-014 | REQ-TEST-006 |
| THR-12 | Process death through parsing: panic, stack overflow, abort on panic | B3 | Loss of service, and of the host if embedded | REQ-SEC-014, REQ-AGG-007, REQ-PAGE-006 | REQ-TEST-006 |
| THR-13 | Denial of wallet and quota exhaustion: a looping agent burns a paid quota or triggers a ban | B1, B2 | Cost; loss of an engine | REQ-MCP-010, REQ-CMP-002, REQ-RES-004 | REQ-TEST-016, REQ-TEST-017 |
| THR-14 | Target-site abuse: rypsok used to flood or scan third-party sites | B3 | Harm to others; blocking of the operator | REQ-SEC-012, REQ-MCP-010, REQ-SEC-009 | REQ-TEST-006, REQ-TEST-016 |
| THR-15 | Provider rate limiting, CAPTCHA and format drift | B2 | Reduced coverage | REQ-ENG-006, REQ-ENG-009, REQ-RES-002, REQ-RES-004, REQ-TEST-003 | REQ-TEST-002, REQ-TEST-003 |
| THR-16 | Secret leakage through logs or debug output | B5 | Credential exposure | REQ-PRIV-004, REQ-SEC-020, REQ-CFG-004 | REQ-TEST-008 |
| THR-17 | Cache contamination: private or forbidden data served to another caller | B4 | Disclosure; breach of provider terms | REQ-CACHE-002, REQ-CACHE-003, REQ-CACHE-004, REQ-CMP-007 | REQ-TEST-010 |
| THR-18 | Inbound DNS rebinding or CSRF against a local HTTP listener | B6 | A web page drives rypsok | REQ-SEC-018 | REQ-TEST-014 |
| THR-19 | Unauthorized network caller | B6 | Resource consumption; data egress on someone else's behalf | REQ-SEC-018, REQ-SEC-019 | REQ-TEST-014 |
| THR-20 | Provider impersonation or downgrade | B2 | False results | REQ-SEC-017 (HTTPS to configured hosts) | REQ-TEST-015 |
| THR-21 | Protocol corruption: a stray line on stdout | B1 | Loss of service | REQ-MCP-002 | REQ-TEST-009 |
| THR-22 | Supply chain: a vulnerable, license-incompatible or compromised dependency, including a CI action referenced by a moving tag | — | Any of the above | REQ-SEC-020, REQ-TEST-012 | REQ-TEST-012 |
| THR-23 | Non-determinism used to hide behavior or defeat tests | — | Untestable output | REQ-ARCH-004 | REQ-TEST-011 |
| THR-24 | Misleading source metadata: homograph or confusable domains in a returned URL | B4 | The caller trusts the wrong site | REQ-RANK-003 (hosts returned in ASCII form) | REQ-TEST-007 |
| THR-25 | A scraping driver runs against a provider's terms, or imitates a browser to get through | B2 | The operator's exposure; the project's honesty | REQ-ENG-013, REQ-ENG-014, REQ-CMP-004 | REQ-TEST-018 |
| THR-26 | A slow or stream-flooding HTTP client exhausts the listener: a body sent a byte at a time, or one HTTP/2 connection opening many streams | B6 | Loss of service for other callers | REQ-SEC-018, REQ-SEC-022 | REQ-TEST-014 |
| THR-27 | Session exhaustion by legacy handshakes: one client opens handshakes until memory is gone | B6 | Loss of service for other callers | REQ-SEC-018, REQ-SEC-024 | REQ-TEST-014 |

## Residual risks, accepted

| Risk | Why accepted | Owner of the residual |
|---|---|---|
| A host that passes external text to its model as an instruction | rypsok cannot control the host; it marks and documents | Host (external design §1.2) |
| A deliberate exfiltration that encodes a secret so that no pattern matches | Pattern detection catches accidents, not adversaries | Host: approve what leaves |
| Results poisoned by search-engine optimization | Inherent to search | Caller |
| A caller times `web_fetch` to tell a name the policy refused from one that does not resolve (THR-05) | The two answers are identical (`unavailable`, `network`), but their latency is not equalised: a refused name answers as soon as the resolver has judged it. Equalising it would add a delay to every failure; the policy's purpose is met by the identical object | Operator |
| The session bound turns memory exhaustion into slot exhaustion (THR-27) | One caller that passes the gate can hold every slot for the 300 s idle expiry of the SDK. Per-caller session quotas and a shorter expiry are deferred to the audit (M8) | Operator |
| The SDK writes one ERROR log line per refused handshake, at a rate the caller sets (THR-27) | Log amplification by a caller that passes the gate; filtering that log target is an M8 candidate | Operator |
| Loss of health state at restart | State is not retained by design | Operator |
| Queries retained in memory for LIM-K2 while the cache is on | A cache keyed by query is retention of queries; the configuration page says so next to the switch (REQ-CACHE-007) | Operator |
| At most 200 characters of a provider's error body in a `debug` log line | A driver's author needs the provider's words to classify a new failure; `debug` is explicit and the bound is a constant (external design §16 I5) | Operator who enables `debug` |

## Review log

| Date | Trigger | Change |
|---|---|---|
| 2026-09-30 | Baseline | Register created from the draft threat lists, the startup findings and the consumer analysis |
| 2026-10-20 | M1 | Controls verified by the M1 tests; THR-16 reviewed against the `Secret` wrapper and the log filter |
| 2026-10-24 | M2 | THR-13 and THR-15 tied to the circuit breaker (REQ-RES-004) |
| 2026-10-27 | M3 | THR-05 to THR-07, THR-11, THR-12 and THR-14 reviewed against the fetch pipeline and the egress policy |
| 2026-10-27 | M4 | THR-18 and THR-19 added for the listener |
| 2026-10-27 | M5 | THR-17 reviewed against the cache; the scraping threat added (as THR-20, renumbered THR-25 on 2026-10-27) |
| 2026-10-27 | M7 | Revision against the delivered system (RFC 018): THR-22 extended to CI actions, THR-26 added, two residual risks recorded, the threat model page added |
| 2026-10-29 | RFC 019 | THR-27 added (session exhaustion by legacy handshakes) with REQ-SEC-024 and LIM-H6; two residual risks recorded |
