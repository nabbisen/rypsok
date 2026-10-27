# Threat model

**Status:** written 2026-10-27 against the delivered system (release 0.6.0;
RFC 018). The [threat register](./threat-register.md) is the table of
record: one row per threat with its controls and the tests that verify
them. This page is how to read it: what is protected, from whom, along
which flows.

## Assets

| Asset | Why it matters |
|---|---|
| The host's network | A fetch that reached an internal address would turn rypsok into a proxy into the operator's network |
| The operator's keys and tokens | A leaked engine key costs money and a ban; a leaked caller token lets anyone search and fetch as that caller |
| Provider quotas and goodwill | A looping agent can burn a paid quota or get the operator blocked |
| The caller's queries and the pages it reads | Queries say what a person is doing; rypsok holds them only for the call, or for LIM-K2 when the operator enabled the cache |
| The availability of the process | One process serves one host, or several callers; its death or exhaustion stops them all |
| The honesty of every response | A caller acts on what rypsok says; a field that lies, or external text that looks like rypsok's own word, steers an agent |

## Attackers and what they can do

| Attacker | Capability | Cannot |
|---|---|---|
| A steered model (through its host) | Call any tool with any argument, as often as the host lets it; encode text in a query or URL | Change a limit, a policy or a destination rule (Principle P4) |
| A hostile page or snippet | Put anything in the bytes rypsok reads: instructions, invisible text, oversized or malformed documents, redirects, slow responses | Reach the caller outside the external-data fields; run scripts (none are run) |
| A provider | Block, challenge, rate-limit, drift its format, answer with error text | Reach a log above `debug`; be sent a key for another origin |
| A network peer (HTTP mode) | Connect, send bytes, open streams, hold connections | Pass the gate without a token; drive rypsok from a browser page |
| A dependency or a CI action | Run code in the build or in the process | Enter unnoticed: advisories, licenses and sources are checked; actions are pinned |
| The operator | Everything; the operator is trusted | Be protected from their own misconfiguration beyond validation that names the key |

## Flows and their controls

| Flow | Boundary | What crosses | Controls | Threats |
|---|---|---|---|---|
| F1 Tool call in | B1 | Arguments, bounded by the schema and the parser; the body by LIM-H2 over HTTP | Validation per parameter; secret check before any request; per-caller rate limits | THR-09, THR-10, THR-13 |
| F2 Query to an engine | B2 | The query, the engine's parameters, rypsok's User-Agent, the key in the driver's header | HTTPS to the configured host only; the key never to another origin; the admission record's rate | THR-08, THR-13, THR-14, THR-20, THR-25 |
| F3 Engine answer | B2 | Raw results or an error body | Bounded read; classified errors; provider text only at `debug` and bounded; the content boundary before the caller | THR-04, THR-11, THR-15 |
| F4 Fetch out | B3 | A URL the caller chose, rypsok's own headers | The egress policy at validation and at every resolution and connect; the scheme and port rules; robots.txt; strict mode | THR-05, THR-06, THR-07, THR-09, THR-14 |
| F5 Page in | B3 | Bytes of any size and shape; redirects | Byte bound while streaming; node and depth bounds; iterative parsing off the runtime; redirects judged and reported; media type checked | THR-01, THR-03, THR-11, THR-12 |
| F6 Response out | B4 | One structured object | External data only in named fields; fixed vocabularies for everything rypsok says; no provider text | THR-01, THR-02, THR-04, THR-24 |
| F7 Operator in | B5 | Configuration, secrets by reference, the command line | Validation that names the key; secrets never inline; `Secret` redacts; logs never carry a secret, a query or a path | THR-16 |
| F8 Network in (HTTP) | B6 | Connections, headers, bodies, streams | `Host` and `Origin` checks; bearer tokens in constant time; a path, a body bound, a connection cap, an idle timeout, a request time bound, a stream bound | THR-18, THR-19, THR-26 |

Flows that do not exist, and are therefore not threats: rypsok never
calls a language model; never runs a script from a page; never writes to
disk; never reads the host's environment beyond the variables it is told
to read.

## Verified in the revision of 2026-10-27

The walk of RFC 018 read every flow against the code of 0.6.0. These
controls were confirmed and left unchanged:

- Decompression is bounded on the decoded side, for engines
  (`egress::read_body`) and for fetches (the same reader under LIM-F3);
  only gzip is negotiated.
- Every address a name resolves to is judged, and judged again at each
  resolution; address literals are judged like names; the connection
  pool reuses a connection to the judged address only.
- Keys and tokens live in `Secret<T>`, whose `Debug` prints
  `[redacted]`; `check` prints the name of a variable or file, never a
  value; the proxy URL is never logged.
- Standard output carries protocol messages only; the gate answers 401,
  403, 404 and 413 before any JSON is parsed.
- A driver's panic ends the engine's task, not the process; a parse
  panic ends the call.
- Cache keys hold the query text in memory for LIM-K2, and the
  configuration page says so beside the switch.

What the walk changed is recorded in the register's review log and in
RFC 018.

## Reading the register

Each row names a boundary from the table above, an impact, the
requirements that control it and the test requirement that verifies it.
A threat with no test is a gap; a control with no threat is a question.
The register is reviewed at every milestone and whenever a flow, an
integration or an authentication mechanism changes.
