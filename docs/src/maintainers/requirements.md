# rypsok — Requirements Specification

**Status:** Baseline 1.0 (2026-09-30). Supersedes the draft of 2026-09-28.
**Normative.** Precedence: owner constraints and project rules, then this
document, then the [external design](./external-design.md), then the
architecture overview (informative), then the studies (informative).

## 0. How to read this document

Every requirement has an identifier, a milestone in which it is verified, and
a verification method:

| Code | Verification |
|---|---|
| T | Automated test in CI |
| I | Inspection of code, configuration or documents by the reviewer |
| A | Analysis, such as a threat-model review or a measurement report |
| D | Demonstration against a live client or provider |

Milestones M0 to M5 are defined in `ROADMAP.md`; `All` marks a requirement
that holds from M1 on. Numbers are never written here; they are cited as
`LIM-` identifiers of [Limits and Defaults](./limits-and-defaults.md).
Threats are cited as `THR-` identifiers of the
[Threat Register](./threat-register.md). Decisions `D-nn` and findings
(`C-`, `M-`, `T-`, `A-`, `R-`, `X-`) refer to the startup review.

Normative words: **shall** (binding), **should** (binding unless a recorded
reason exists), **may** (permitted). A requirement written with "should"
names the condition under which it may be waived.

Identifiers of the draft are kept. Requirements that the draft stated twice
are merged; the surviving identifier is listed in
[Appendix A](#appendix-a--disposition-of-the-draft-requirements).

---

## 1. Purpose and scope

rypsok is a headless meta-search service for AI agents. It accepts a search
or fetch request from a caller, queries several external sources
concurrently, normalizes, deduplicates and ranks what returns, keeps every
external text recognizable as data, bounds every result, and returns a
compact, structured response. It runs as an independent service; the owner's
agent runtime mimlys is one of its clients (D-01).

In scope: the three tools `web_search`, `web_fetch` and `engine_status`; the
MCP interface over stdio and Streamable HTTP; a documented Rust interface of
the core; engines behind a common abstraction; the content boundary; the
egress policy; query privacy; configuration and the operator interface;
observability; testing.

Out of scope, as non-goals: a human-facing user interface of any kind;
templates and themes; browser sessions, cookies and human authentication;
favicons; OpenSearch; CAPTCHA solving or forwarding; JavaScript execution
when fetching pages; following `meta refresh`; pagination of search results;
proxy or User-Agent rotation; an HTTP API outside MCP (D-30); any dependence
on an LLM inside rypsok.

---

## 2. Principles

| # | Principle |
|---|---|
| P1 | **Headless and agent-native.** Consumers are agents, their hosts, and operators. MCP is a first-class interface; the core does not depend on it |
| P2 | **External data is never an instruction.** Titles, snippets, page content and URLs stay recognizable as data all the way to the caller |
| P3 | **Bounded everything.** Deadlines, size limits, concurrency limits and rate limits govern every operation |
| P4 | **Safe by default; only the operator relaxes.** A caller can never weaken a control |
| P5 | **Fail soft across engines, fail closed on policy.** One failing engine reduces coverage; a policy failure refuses the call |
| P6 | **Compliance is architecture.** Terms of a provider, robots.txt and the amount of returned content are decided per engine and per tool before code exists |
| P7 | **Interfaces that cannot be misread.** The clarity rules of §2.1 apply to every tool, parameter, outcome and configuration key |
| P8 | **Minimal retention.** Nothing retrieved becomes durable unless a policy says so |
| P9 | **Deterministic core.** For the same engine responses, the same output |

### 2.1 Interface clarity rules

| # | Rule |
|---|---|
| CLR-1 | A tool does what was asked, or it refuses and says why. It never does something else silently |
| CLR-2 | No silent fallback: an invalid value is an error that lists the valid ones |
| CLR-3 | No silent reduction: a value above a ceiling is an error that states the ceiling |
| CLR-4 | Every call ends in exactly one outcome from a small, fixed vocabulary |
| CLR-5 | Every error says what the caller can change |
| CLR-6 | One name for each thing, one meaning for each name |
| CLR-7 | A name says what is counted: `max_chars`, `start_char`, `deadline_ms` |
| CLR-8 | Few tools, few parameters, flat schemas, enumerations instead of free text |
| CLR-9 | Defaults are safe, stated in the schema, and the same everywhere |
| CLR-10 | External text is marked in the structure, not inside a string |
| CLR-11 | What rypsok changes in external text, it reports through a notice |
| CLR-12 | Clarity is measured: refused calls are counted per tool and reason |

---

## 3. Actors and trust boundaries

| Actor | Trust |
|---|---|
| Caller (a model through its host, or a runtime) | Untrusted input. Its arguments may be malformed, adversarial, or steered by injected content |
| Host | Trusted to run rypsok; not trusted to sanitize what it passes |
| Operator | Trusted to the extent of the configuration; cannot disable mandatory controls through a caller's parameters |
| Provider | Untrusted content; trusted only for the endpoint the operator configured |
| Target site | Untrusted content and behavior |

Boundaries, cited as `B1` to `B6` throughout:

| ID | Boundary | Crossed by |
|---|---|---|
| B1 | Caller → rypsok | Tool arguments |
| B2 | rypsok → engine endpoints | Queries and engine responses |
| B3 | rypsok → target sites | Fetch requests, robots.txt reads, responses, redirects |
| B4 | rypsok → caller | Every response |
| B5 | Operator → rypsok | Configuration, secrets, command line, environment |
| B6 | Network → rypsok listener (HTTP mode only) | Inbound connections |

---

## 4. Architecture

### REQ-ARCH-001 Layer separation — All · I
The implementation shall keep the protocol adapters, the application
services, the search and fetch pipelines, the engine drivers, the egress
policy, the content boundary and the configuration in separate modules with
explicit interfaces. The canonical decomposition is §5 of the external design.

### REQ-ARCH-002 Provider independence — All · I
No provider-specific request, parsing or error logic shall exist outside the
driver of that engine.

### REQ-ARCH-003 Headless runtime — All · I
No search or fetch shall require a browser, a rendering engine, or a
human-facing user interface.

### REQ-ARCH-004 Deterministic pipeline — All · T
For an identical set of engine responses and an identical configuration, the
normalization, deduplication, ranking, content-boundary and formatting stages
shall produce identical output, byte for byte. Accumulation of scores shall
happen in a fixed order, and the final order shall be a total order with
explicit tie-breakers. (A-07, R-08)

### REQ-ARCH-005 Core without protocol dependency — M0 · T
The crate that holds the models, the pipelines, the drivers and the policies
shall not depend on any MCP SDK or transport library. CI shall fail when such
a dependency appears. (K-02, R-16)

### REQ-ARCH-006 No LLM inside rypsok — All · I
No stage of rypsok shall call a language model. Normalization, ranking,
policy and sanitization are rule-based. (AO §15.4 promoted)

### REQ-ARCH-007 Documented core interface — M1 · I
The Rust interface of the core crate shall be documented and shall be marked
unstable until release 1.0.0. (D-35)

### REQ-ARCH-008 Pinned protocol SDK — M1 · I
The MCP SDK shall be pinned to an exact version and updated only through a
reviewed change that names the protocol revisions it adds or drops. (R-16)

---

## 5. Protocol requirements

### REQ-MCP-001 MCP support — M1 · T
rypsok shall expose its tools through the Model Context Protocol.

### REQ-MCP-002 stdio transport — M1 · T
rypsok shall serve MCP over stdio. It shall write nothing to standard output
except valid MCP messages; all diagnostics go to standard error. When standard input reaches end of
file it shall start shutdown at once and exit within LIM-O1. (R-06)

### REQ-MCP-003 Streamable HTTP transport — M4 · T
rypsok shall serve MCP over the Streamable HTTP transport of the supported
protocol revisions. The HTTP+SSE transport of revision 2024-11-05 shall not
be offered. (C-01, D-12)

### REQ-MCP-004 Machine-readable requests — M1 · T
Every tool argument shall be validated against the tool's input schema
before any work starts. A failure shall be reported as a tool execution
error of kind `invalid_request` (§17).

### REQ-MCP-005 Minimal tool surface — M1 · I
The tool set shall be exactly `web_search`, `web_fetch` and
`engine_status`. No tool shall be added to reproduce human-oriented search
features. (D-24)

### REQ-MCP-006 Supported protocol revisions — M1 · T
rypsok shall implement the current MCP revision at the time of each release
and shall keep serving the handshake-based revisions that the SDK supports in
parallel, until the operator disables them. The supported revisions shall be
listed in the release notes. (D-12, M-08)

### REQ-MCP-007 Cancellation — M1 · T
Within LIM-O2 after a cancellation, a call shall have started no new
outbound request and shall have aborted the ones in flight, and rypsok shall
send no further message for it. On stdio the signal is
`notifications/cancelled`; on Streamable HTTP it is the closed response
stream. (REQ-AGG-006, M-08)

### REQ-MCP-008 Tool definitions under budget — M1 · T
The names, descriptions and input schemas of all tools, together with the
server instructions, shall not exceed LIM-I1. Each tool shall have at most LIM-I2 parameters, and every
input schema shall be a flat object of primitive types and enumerations
(LIM-I3). Tool descriptions and the server instructions are part of the
external contract and change only through review. (R-15, X-06)

### REQ-MCP-009 Tool annotations — M1 · T
Every tool shall declare the MCP annotations `readOnlyHint: true`,
`destructiveHint: false`, `idempotentHint: true`; `openWorldHint` is `true`
for `web_search` and `web_fetch` and `false` for `engine_status`.
The list of tools shall be returned in a deterministic order and shall not
vary per connection; it may vary with configuration. (X-02)

### REQ-MCP-010 Invocation rate limit — M1 · T
rypsok shall accept at most LIM-C6, LIM-C7 and LIM-C8 calls per tool per
minute per process and shall refuse further calls with an error of kind
`unavailable` that states when the caller may retry. (M-04)

### REQ-MCP-011 Structured results — M1 · T
Every tool shall declare an output schema, shall return its response as
structured content that conforms to it, and shall return the same response
serialized as text in the rendering the operator configured, JSON by
default. The operator may switch the structured content off for hosts that
would feed both forms to a model. (D-13)

---

## 6. Tools

### 6.1 `web_search`

### REQ-SEARCH-001 Query parameter — M1 · T
`web_search` shall accept a required `query` of at least one and at most
LIM-S1 characters after trimming. A query outside that range shall be
refused with `invalid_request`.

### REQ-SEARCH-002 Category parameter — M1 · T
`web_search` shall accept an optional `category` from the vocabulary
`general` (default), `news`, `science`, `code`. The input schema shall
enumerate only categories that at least one enabled engine serves. Any other
value shall be refused with `invalid_request` naming the valid values.
(C-10, D-15)

### REQ-SEARCH-003 Result count — M1 · T
`web_search` shall accept an optional integer `max_results` with default
LIM-S2 and the range of LIM-S2. A value outside the range shall be refused
with `invalid_request` stating the range. (CLR-3)

### REQ-SEARCH-004 Language parameter — M1 · T
`web_search` shall accept an optional `language`, enumerated from the
languages the operator configured, with the operator's default. It selects
the language of the results where an engine supports the selection, and it
Engines that cannot honor the selection shall be skipped for that call, and
a notice shall name them. (D-14)

### REQ-SEARCH-005 Time range parameter — M1 · T
`web_search` shall accept an optional `time_range` from `day`, `week`,
`month`, `year`. Engines that cannot honor it shall be skipped for that call,
and a notice shall name them. (D-14)

### REQ-SEARCH-008 Region and safe search — M1 · T
The operator shall configure a default region and a safe-search level
(`off`, `moderate`, `strict`; default `moderate`). Both are passed to every
engine that supports them and are not exposed as parameters. (D-14)

### REQ-SEARCH-006 Input validation before work — M1 · T
No engine request shall start before every parameter has passed validation
and the privacy check (§13). (former REQ-SEARCH-004)

### REQ-SEARCH-007 Bounded output — M1 · T
A response shall contain at most `max_results` results, each bounded by
LIM-S9 and LIM-S10, and shall be complete in itself: no continuation and no
pagination. (former REQ-SEARCH-005; D-14)

### 6.2 `web_fetch`

### REQ-FETCH-001 URL retrieval — M3 · T
`web_fetch` shall retrieve the resource at a required `url` of at most
LIM-F9 characters, subject to the egress policy (§12.2), the fetch policy
(§14) and the resource limits of §11.

### REQ-FETCH-002 Main content extraction — M3 · T
`web_fetch` shall return the main content of an HTML document. Against the
extraction fixture corpus defined in the fetch RFC, every fixture's expected
main text shall be present and its listed navigation, header, footer, script
and style text absent.

### REQ-FETCH-003 Size parameter — M3 · T
`web_fetch` shall accept an optional integer `max_chars` with default LIM-F1
and ceiling LIM-F2, the ceiling set by the operator. A value above the
ceiling shall be refused with `invalid_request` stating the ceiling. Output
longer than `max_chars` shall be cut at a character boundary and marked
`truncated`. (M-02, CLR-3)

### REQ-FETCH-004 Rendering — M3 · T
The content shall be returned as Markdown derived from the document
structure, or as plain text where the document has none, inside the
structured response of REQ-MCP-011.

### REQ-FETCH-005 Continuation — M3 · T
`web_fetch` shall accept an optional integer `start_char` (default 0) and
shall return `next_start_char` when output was truncated. A `start_char` at
or beyond the end of the extracted content shall be refused with
`invalid_request` stating the content length. The page is fetched again; no
state is kept between calls, and the tool description shall say
that the page may have changed between two calls. (D-17)

### REQ-FETCH-006 Redirects — M3 · T
`web_fetch` shall follow at most LIM-F7 redirects whose target is on the same
host as the current request. A redirect to another host shall not be
followed; when the target passes the agent-directed egress policy, the call
shall end with the error kind `redirected`, whose detail gives the target
URL; when it does not, the call shall end with `denied` and the target shall
not be disclosed. The operator may configure following of cross-host
redirects that pass the egress policy; the response then states the URL
fetched. (D-34, X-03)

### REQ-FETCH-007 No credentials — M3 · T
`web_fetch` shall send no cookies, no authorization header and no
credentials taken from the URL. A URL that carries credentials shall be
refused with `denied`. (A-03, M-07)

### REQ-FETCH-008 Eligible content — M3 · T
`web_fetch` shall extract HTML, XHTML and plain text. Any other media type
shall end the call with the error kind `unsupported_content` naming the
type. PDF extraction is deferred (§24).

### REQ-FETCH-011 Response status — M3 · T
Only a response with a 2xx status shall be extracted. A 4xx or 5xx status
shall end the call with `unavailable` and the reason `http_status`, giving
the status code and never the body. 3xx follows REQ-FETCH-006.

### REQ-FETCH-009 No script execution — M3 · I
`web_fetch` shall not execute scripts and shall not follow `meta refresh`.
The tool description shall say so. (M-13)

### REQ-FETCH-010 Scheme policy — M3 · T
Only `https` URLs shall be fetched by default. An `http` URL shall be
retried as `https`, and the response shall state the URL fetched; if the
retry fails, the call shall end with `denied`. The operator may allow plain
`http`. (D-18)

### 6.3 `engine_status`

### REQ-STATUS-001 Health information — M2 · T
`engine_status` shall report, for every configured engine, its identifier,
its state (`disabled`, `closed`, `open`, `half_open`), and the median
latency of its last LIM-S11 completed requests, or null when there are none.
Nothing else. (D-26)

### REQ-STATUS-002 No secrets, no provider text — M2 · T
The response shall contain no endpoint, no credential, and no text that
originates from a provider. (REQ-ERR-004, R-03)

### REQ-STATUS-003 Operator control — M2 · T
The operator shall be able to disable `engine_status`; the tool then does
not appear in the tool list. (D-26)

---

## 7. Engines

### REQ-ENG-001 Engine abstraction — M1 · I
Every engine shall implement a common abstraction that exposes a stable
identifier, its categories, an asynchronous search operation, and its own
error classification.

### REQ-ENG-002 Concurrent use — M1 · T
Drivers shall be usable concurrently by the aggregator.

### REQ-ENG-003 Enable and disable by configuration — M1 · T
Operators shall enable and disable engines in configuration, without code
changes.

### REQ-ENG-004 Registry — M1 · T
A registry shall resolve configured engine identifiers to driver instances
and shall reject an identifier with no driver at startup. (merges
REQ-CFG-002)

### REQ-ENG-005 Driver-owned parsing — M1 · T
Each driver shall own the parsing of its provider's responses, so that a
change of the provider's format affects that driver only.

### REQ-ENG-006 Error classification — M1 · T
Every driver shall classify a failed request as exactly one of: `timeout`,
`rate_limited`, `blocked` (CAPTCHA or anti-bot), `parse_failure`,
`auth_failure`, `request_failure`. No provider text shall leave the driver
with the classification.

### REQ-ENG-007 Category-based selection — M1 · T
A search shall query only the enabled engines that serve the requested
category, at most LIM-S8 of them, in a deterministic order of preference set
by configuration.

### REQ-ENG-008 Official interfaces first — All · I
Every engine shipped enabled by default shall use an official or explicitly
permitted interface of its provider. Drivers that scrape a human-facing
interface are a later, opt-in engine type (DR-006). (D-06; merges
REQ-RES-001)

### REQ-ENG-009 Anti-bot responses — M1 · T
A CAPTCHA or anti-bot response shall be classified `blocked` and shall never
be forwarded to the caller. (merges REQ-RES-003)

### REQ-ENG-010 Provider obligations as engine properties — M1 · T
Each engine shall carry the properties recorded at its admission (§14): its
rate limit and concurrency cap (LIM-C9, LIM-C2), its maximum query length,
which `language` and `time_range` values it honors, whether its results may
be cached, whether attribution is mandatory, and the User-Agent format it
requires. An engine whose maximum query length is exceeded shall be skipped
for that call with a notice. The aggregator, the cache and the formatter shall obey them.
(M-03)

### REQ-ENG-011 Result count per engine — M1 · T
Each engine shall be asked for LIM-S3 results, or fewer where the provider
allows fewer. (A-04)

### REQ-ENG-012 Snippet delivery — M1 · T
Drivers shall deliver titles and snippets as raw text; the pipeline, not the
driver, decodes entities, strips markup and bounds the length. (M-18)

---

## 8. Aggregation

### REQ-AGG-001 Concurrent requests — M1 · T
Engine requests of one search shall run concurrently, within LIM-C1 and
LIM-C2.

### REQ-AGG-002 Per-engine timeout — M1 · T
Every engine request shall be cancelled after LIM-S5.

### REQ-AGG-003 Overall deadline — M1 · T
A search shall return within LIM-S4 measured from the start of the call, and
a `web_search` call shall never last longer than LIM-S7. LIM-S5 plus LIM-S6
shall not exceed LIM-S4; configuration that violates this shall be refused
at startup. (C-14, X-04)

### REQ-AGG-004 Partial results — M1 · T
When some engines fail or time out and at least one answered, the search
shall return the results obtained with outcome `partial` and one notice per
failed engine. (merges REQ-ERR-002, REQ-RES-006)

### REQ-AGG-005 Bounded concurrency — M1 · T
Outbound requests in flight shall never exceed LIM-C1 for the process and
LIM-C2 per engine. (merges REQ-SEC-012)

### REQ-AGG-006 Cancellation of useless work — M1 · T
Engine work that can no longer contribute after the deadline or after a
cancellation shall be aborted, not detached. (R-05)

### REQ-AGG-007 Failure isolation — M1 · T
A panic, a malformed response or a network failure in one engine's driver
shall neither end the process nor discard the results of other engines.
(merges REQ-ERR-005; REQ-SEC-014 applies)

### REQ-AGG-008 No retry within a call — M1 · T
A failed engine request shall not be retried within the same search. Retry
after cooldown is the circuit breaker's task (§15). (M-15)

### REQ-AGG-009 Early completion — M1 · T
A search shall complete as soon as every queried engine has answered or
failed, without waiting for the deadline.

---

## 9. Normalization, deduplication and ranking

### REQ-RANK-001 Canonical result model — M1 · T
Internally a result shall hold the returned URL, the comparison key, a
title, a snippet, an optional date, and a list of observations, each an
engine identifier with the rank the engine gave. (C-06)

### REQ-RANK-002 Comparison key — M1 · T
The comparison key shall be derived from the returned URL by conservative,
test-driven rules that never change resource identity: scheme and host
lower-cased, default port removed, fragment removed, known tracking
parameters removed, percent-encoding normalized. The comparison key shall
never be returned to the caller and never be fetched. (C-07)

### REQ-RANK-003 Returned URL — M1 · T
The URL returned to the caller shall be the engine's URL with known tracking
parameters removed, provider redirect wrappers unwrapped by the driver, and
the host written in its ASCII (IDNA) form, and nothing else changed.

### REQ-RANK-004 Cross-engine deduplication — M1 · T
Results with the same comparison key shall be merged into one result whose
observations are the union. The title and snippet of the merged result shall
come from the observation with the best rank; ties break by engine
preference order.

### REQ-RANK-005 Rank fusion — M1 · T
The final order shall be computed from the observations by a rank-based
formula that rewards agreement between engines, defined in an RFC, without
floating-point accumulation in arrival order. (REQ-RANK-004 of the draft)

### REQ-RANK-006 Stable order — M1 · T
The final order shall be a total order: score, then best rank, then engine
preference, then comparison key. (REQ-RANK-005 of the draft)

### REQ-RANK-007 Limit enforcement — M1 · T
After ranking, the list shall be cut to `max_results`. (REQ-RANK-006 of the
draft)

---

## 10. Output and context

### REQ-CTX-001 Minimal result — M1 · T
A returned result shall contain `title`, `url`, `snippet`, and `date` when
an engine supplied one. Engine attribution (`engines`) shall be present only
when the operator enabled it or when an engine's admission record requires
attribution. No score shall be returned. (C-09, D-16)

### REQ-CTX-002 Canonical structured response — M1 · T
The response of every tool shall be one structured object with a declared
schema. Fields that rypsok asserts (`outcome`, `notices`, and the control
fields the external design lists for each tool) and fields that carry
external data shall be distinct fields, and the external design shall state
the class of every field. (X-01, X-05)

### REQ-CTX-003 Text renderings — M1 · T
The text form of a response shall be the JSON serialization of the
structured response by default. A compact context rendering may be offered
as an operator setting once its encoding rules are specified in an RFC; it
shall encode every external string for its format. (D-13, M-14)

### REQ-CTX-004 Snippet and title cleaning — M1 · T
Titles and snippets shall have markup removed, entities decoded, whitespace
collapsed, invisible and control characters removed, and length bounded by
LIM-S9 and LIM-S10, in that order. (M-18)

### REQ-CTX-005 Noise removal in fetched content — M3 · T
Fetched content shall exclude scripts, styles, navigation, and content that a
browser would not render: elements hidden by markup or style, comments,
`noscript` and `template` contents. (R-03)

### REQ-CTX-006 Size bounds — M1 · T
Every string in a response shall be bounded: snippets and titles by
LIM-S9 and LIM-S10, page content by `max_chars`, notices and error messages
by LIM-I4, URLs by LIM-F9. A result whose URL exceeds LIM-F9 shall be
dropped and counted in a notice.

### REQ-CTX-007 Truncation signaling — M1 · T
Whenever rypsok cut external text to a bound, the response shall say so:
`truncated` on fetched content, a notice on a shortened snippet or title.
(CLR-11)

### REQ-CTX-008 No human presentation — All · I
No rendering shall be optimized for visual display.

### REQ-CTX-009 Tool profile — M1 · I
The external design shall state for each tool: its effect on local state,
the data that leaves the host and to whom, what the other side learns, the
maximum duration, and the maximum output. (X-02)

### REQ-CTX-010 Budget unit — M1 · T
All text budgets are counted in Unicode scalar values (LIM-U1). The
documentation shall state that Japanese text costs several times more tokens
per character than English text. (M-12)

---

## 11. Page retrieval and extraction

### REQ-PAGE-001 Pipeline order — M3 · T
A fetch shall proceed in this order: parameter validation → privacy check of
the URL → egress policy → robots.txt policy → request → byte limit while
streaming → media type check → charset decoding → parsing under node and
depth limits → main-content extraction → cleaning (REQ-CTX-005, invisible
characters) → truncation to `max_chars` → structured response. (C-19)

### REQ-PAGE-002 Main-content preference — M3 · T
Where a fixture of the corpus contains both an article and site chrome, the
article shall be returned first and the chrome shall be absent
(REQ-FETCH-002).

### REQ-PAGE-003 Response byte limit — M3 · T
At most LIM-F3 bytes after decompression shall be read; a longer response
ends the call with `too_large`. The limit shall be enforced while streaming,
never by trusting `Content-Length`. (R-01, merges REQ-SEC-010 for fetches)

### REQ-PAGE-004 Streaming safety — M3 · T
No response body shall be buffered beyond LIM-F3.

### REQ-PAGE-005 Media types — M3 · T
Only the media types of REQ-FETCH-008 shall be parsed; the type is taken
from the response header, with sniffing limited to distinguishing HTML from
text.

### REQ-PAGE-006 Malformed content — M3 · T
Malformed HTML shall yield partial content with the notice
`extraction_partial`, or, when nothing can be extracted, the error
`unsupported_content` with the reason `extraction_failed`; never a process
failure. Parsing shall stop at LIM-F10 nodes or LIM-F11
levels of nesting. (R-04)

### REQ-PAGE-007 Charset handling — M3 · T
Documents shall be decoded according to the declared charset, with detection
for legacy encodings when none is declared, and errors replaced rather than
fatal. (M-12)

### REQ-PAGE-008 Computation off the I/O path — M3 · T
Parsing and extraction shall run outside the network event loop, at most
LIM-C5 at a time, and shall not extend a `web_fetch` call beyond LIM-F5.
(R-04, X-04)

---

## 12. Security

### 12.1 Content boundary (B4)

### REQ-SEC-001 External data is data — M1 · T
Every string that originates outside rypsok shall reach the caller only
inside the external-data part of the structured response (REQ-CTX-002),
never in the envelope, in an error message, or in a log line. The only
exceptions are the `location` of a `redirected` error and the `media_type`
of an `unsupported_content` error, which are external identifiers, bounded
by LIM-F9, syntactically validated, and documented as external data. (C-02)

### REQ-SEC-002 Unforgeable separation — M1 · T
No external string shall be able to alter the structure of a response or
imitate a field that rypsok asserts. In the JSON rendering this follows from
the encoding; every other rendering shall prove it by test with hostile
content. (R-03)

### REQ-SEC-003 Instruction-like content — M1 · T
rypsok shall not rewrite visible external text. It may detect instruction-
like patterns and shall then add a notice; the operator may configure that
results with such patterns are dropped, in which case a notice states the
count. (A-05, D-11)

### REQ-SEC-004 No effect of content on rypsok — M1 · T
External content shall never select an engine, change a limit, alter
configuration, or trigger a request other than the one the caller made.

### REQ-SEC-015 Invisible characters — M1 · T
Zero-width characters, Unicode tag characters, bidirectional controls and
other control characters shall be removed from external text, and a notice
shall report the removal. (R-03, CLR-11)

### REQ-SEC-016 Fixed error vocabulary — M1 · T
Error kinds, notice codes and engine error classes shall come from fixed
vocabularies. No text from a provider or a page shall appear in them.
(REQ-STATUS-002, R-03)

### 12.2 Egress policy (B2, B3)

### REQ-SEC-005 Egress policy component — M1 · T
All outbound requests shall pass one egress component that applies one of
two profiles: *engine* for configured endpoints (M1, REQ-SEC-017) and
*agent-directed* for fetches and robots.txt reads (M3, REQ-SEC-006).
(C-05, D-21)

### REQ-SEC-006 Agent-directed destinations — M3 · T
The agent-directed profile shall allow only globally routable unicast
addresses, judged on every address that will be connected to, after
canonicalizing IPv4-mapped IPv6 addresses. Loopback, private, link-local,
unique-local, multicast, broadcast, unspecified, carrier-grade NAT,
benchmarking, documentation, reserved and NAT64 ranges, and cloud metadata
addresses shall be refused. The operator may add deny rules and explicit
allow rules; no caller parameter can change the profile. (M-07)

### REQ-SEC-021 Strict fetch mode — M3 · T
The operator may enable a strict mode in which `web_fetch` accepts only
URLs that rypsok itself returned earlier in the same process, and URLs of
the operator's allow list. Any other URL shall be refused with `denied` and
the reason `strict_mode`. (D-09, R-02)

### REQ-SEC-007 Connect-time enforcement — M3 · T
The address that passed validation shall be the address connected to, for
every hop, including hosts given as address literals. Environment proxy
variables shall be ignored. A configured proxy shall not be used for
agent-directed requests unless it enforces the same profile. (R-01)

### REQ-SEC-008 Redirect validation — M3 · T
Every redirect target shall pass the same profile as the original request,
in addition to REQ-FETCH-006.

### REQ-SEC-009 Scheme, port, method — M3 · T
Agent-directed requests shall use `https` (or `http` when the operator
allowed it), the ports of LIM-F8, the GET method only, and no request header
beyond those rypsok sets itself. (M-07)

### REQ-SEC-017 Engine profile — M1 · T
Engine requests shall go only to the configured host of the engine, over
HTTPS, and shall follow no cross-origin redirect; a custom credential header
shall never be sent to another origin; environment proxy variables shall be
ignored. A private address or plain HTTP is
accepted only when the operator configured that endpoint explicitly. (R-01)

### 12.3 Resources

### REQ-SEC-010 Response size limits — M1 · T
Engine responses and fetched responses shall be bounded while streaming
(LIM-F3 for fetches; a driver-declared bound for engines).

### REQ-SEC-011 Time limits — M1 · T
Every outbound request shall have a connect timeout (LIM-S12 for engines,
LIM-F6 for fetches), a total timeout (LIM-S5 for engines, LIM-F4 for
fetches), and a read timeout equal to the time remaining of the total.

### REQ-SEC-012 Fetch and parsing concurrency — M3 · T
Fetches in flight shall never exceed LIM-C3 for the process and LIM-C4 per
host, and parsing tasks shall never exceed LIM-C5; further work waits or,
when the deadline would be missed, ends with `unavailable`.

### REQ-SEC-013 Decompression safety — M1 · T
Byte limits shall apply to decompressed bytes, counted as they are produced.

### REQ-SEC-014 Process stability — All · T
No external content, provider failure or malformed response shall end the
process. Release builds shall unwind on panic; drivers and extraction shall
run in tasks whose panics are contained; recursion shall be bounded by
LIM-F11. (R-05)

### 12.4 Inbound (B6)

### REQ-SEC-018 Listener protections — M4 · T
In HTTP mode rypsok shall validate the `Origin` and `Host` headers, shall
bind to loopback unless the operator configures otherwise, and shall require
authentication for non-loopback binds. (M-16)

### REQ-SEC-019 Per-caller limits — M4 · T
In HTTP mode the rate limits of REQ-MCP-010 shall apply per authenticated
caller.

### 12.5 Engineering

### REQ-SEC-020 Supply chain and hardening — M0 · T
CI shall run a dependency vulnerability audit and a license check; project
code shall forbid `unsafe`; types that hold secrets shall redact them in
debug output. (M-21)

---

## 13. Query privacy

### REQ-PRIV-001 Queries are sensitive — All · I
Design and documentation shall treat every query and every fetched URL as
potentially confidential.

### REQ-PRIV-002 Secrets in queries and URLs — M1 · T
Before any outbound request, the query of a search and the URL of a fetch
shall be checked for high-confidence credential formats. On detection the
call shall be refused with `denied`, naming the kind and position of the
credential and never its value. The operator may select `warn`, `mask` (with
a notice) or `off` (LIM-P1). Personal data is not detected. (D-10, R-09)

### REQ-PRIV-003 Non-retention — M1 · T
rypsok shall keep no query, URL or content beyond the call, except in a
cache the operator enabled (§16) and only for data the cache policy admits.

### REQ-PRIV-004 No sensitive data in logs — M1 · T
Query text, URL paths and query strings, content, credentials and
authorization headers shall never appear in logs at the default level. The host name of a fetch shall be logged only after it passed the egress
policy; a refused destination is logged as its reason class only. A debug
mode that logs queries shall be an explicit operator choice and shall warn
at start (LIM-P3). (D-19)

### REQ-PRIV-005 Disclosure statement — M1 · I
The documentation and each tool's profile shall state what leaves the host:
the query to every enabled engine of the category, the URL to the target
site, and the host's network address to each of them. rypsok promises
confidentiality hygiene, not anonymity. (A-16)

---

## 14. Compliance

### REQ-CMP-001 Admission record — M1 · I
No driver shall be merged before an admission record exists for its
provider: the endpoint, the permitted use, rate and concurrency limits,
attribution, storage rules, and the required User-Agent format. The record
lives with the driver's RFC. (M-03)

### REQ-CMP-002 Provider limits honored — M1 · T
Requests to an engine shall never exceed the rate and concurrency in its
admission record (LIM-C9, LIM-C2), and `Retry-After` shall be honored
(LIM-B4).

### REQ-CMP-003 Attribution — M1 · T
Where the admission record requires attribution, the result shall carry the
engine identifier (REQ-CTX-001) and the documentation shall name the
provider.

### REQ-CMP-004 Identifying User-Agent — M1 · T
All requests shall carry one stable User-Agent of the form
`rypsok/<version> (+<project URL>; <operator contact>)`. No rotation, no
imitation of a browser. A per-engine format is used only where the admission
record requires it. (C-16, D-07)

### REQ-CMP-005 robots.txt — M3 · T
Before an agent-directed fetch, rypsok shall read and honor the site's
robots.txt for its own product token and for `*`, caching it for at most
LIM-F12 and reading at most LIM-F13. A disallowed URL ends the call with
`denied` and the reason `robots_txt`. An unreachable robots.txt (server
error) counts as disallow, as RFC 9309 requires. The operator may disable
the check. (M-01, D-08)

### REQ-CMP-006 Content minimization — M3 · I
`web_fetch` returns the main content of one page, bounded by `max_chars`
and LIM-F2, and never retains it. The documentation shall state this as the
project's answer to copyright and redistribution concerns. (M-02)

### REQ-CMP-007 Cache eligibility per engine — M5 · T
Results of an engine whose admission record forbids storage shall never be
cached. (REQ-ENG-010)

---

## 15. Resilience

### REQ-RES-002 Rate-limit detection — M1 · T
HTTP 429, and the provider-specific rate-limit signals listed in the
engine's admission record, shall be classified `rate_limited`.

### REQ-RES-004 Circuit breaker — M2 · T
An engine shall enter cooldown after LIM-B1 consecutive failures of class
`timeout`, `rate_limited`, `blocked` or `parse_failure`, for LIM-B2 growing
by LIM-B3, and shall be skipped meanwhile.

### REQ-RES-005 Recovery — M2 · T
After a cooldown, one real request (LIM-B5) shall probe the engine; success
closes the circuit, failure reopens it with a longer cooldown. rypsok shall
send no synthetic probe request. (M-20)

### REQ-RES-007 User-Agent — All · I
See REQ-CMP-004. (rewritten)

### REQ-RES-008 Proxy — M3 · T
One outbound proxy may be configured for engine requests. It shall be taken
from configuration only, never from environment variables. (R-01)

### REQ-RES-009 State is ephemeral — M2 · I
Health and circuit state live in memory and are lost at restart. The
documentation shall say so. (A-12)

---

## 16. Cache (M5)

### REQ-CACHE-001 Optional — M5 · T
The cache shall be off by default (LIM-K1) and shall not be required for
correctness.

### REQ-CACHE-002 Public search results only — M5 · T
Only `web_search` results shall be cacheable. Fetched content shall never be
cached. (C-17)

### REQ-CACHE-003 Eligibility — M5 · T
Entries shall respect REQ-CMP-007 and shall carry the engine set,
configuration version and policy with which they were produced.

### REQ-CACHE-004 Isolation — M5 · T
In HTTP mode cache entries shall be keyed by the authenticated caller; in
stdio mode the process is the isolation unit. (M-17)

### REQ-CACHE-005 Expiration — M5 · T
Every entry shall expire after LIM-K2; the cache holds at most LIM-K3
entries.

### REQ-CACHE-006 Failure isolation — M5 · T
A cache failure shall degrade performance only.

### REQ-CACHE-007 No query retention beyond the cache — M5 · A
A cache keyed by query is retention of queries; enabling it shall be
documented as such, and no semantic or persistent cache shall exist without
its own privacy RFC. (R-13)

---

## 17. Errors and outcomes

### REQ-ERR-001 One outcome per call — M1 · T
Every call shall end with exactly one outcome: `ok`, `partial`, `empty`, or
an error of one kind. The decision table is §7 of the external design.
(C-15, CLR-4)

### REQ-ERR-002 Error kinds — M1 · T
Error kinds shall be exactly: `invalid_request`, `denied`, `unavailable`,
`redirected`, `unsupported_content`, `too_large`. Each error shall carry a
message of at most LIM-I4 characters that says what the caller can change,
and a `detail` object with fixed keys. (CLR-5)

### REQ-ERR-003 Errors as tool execution errors — M1 · T
Errors shall be returned as MCP tool execution errors (`isError: true`) with
the structured error object, never as protocol errors, except for an unknown
tool or a malformed protocol message.

### REQ-ERR-004 Safe errors — M1 · T
No error shall contain a secret, a credential, an internal address, a stack
trace, or provider text.

### REQ-ERR-005 Empty is not failure — M1 · T
`empty` shall be returned only when at least one engine answered and no
result remained; when no engine answered, the error is `unavailable`.

---

## 18. Observability

### REQ-OBS-001 Health state — M2 · T
The process shall hold a health state per engine (§15).

### REQ-OBS-002 Latency — M2 · T
Latency of every engine request and of every call shall be measured.

### REQ-OBS-003 Errors by class — M2 · T
Engine errors shall be counted by class (REQ-ENG-006) and call errors by
kind (REQ-ERR-002).

### REQ-OBS-004 Circuit visibility — M2 · T
Circuit transitions shall be logged and visible through `engine_status`.

### REQ-OBS-005 Structured logs — M1 · T
Logs shall be structured lines on standard error with a request identifier,
engine, host name, timing, counts, outcome and policy decision, under
REQ-PRIV-004.

### REQ-OBS-006 No content logging — M1 · T
No response body or page content shall be logged at any level.

### REQ-OBS-007 Refused calls counted — M1 · T
Refused calls shall be counted per tool and per reason. The documentation
shall name a rising count as a defect of the interface. (X-07, CLR-12)

### REQ-OBS-008 Exposure — M2 · I
Metrics leave the process through the log lines of REQ-OBS-005 and through
`engine_status`; in HTTP mode an OpenTelemetry exporter may be added by RFC.
(M-25)

---

## 19. Performance

### REQ-PERF-001 Asynchronous I/O — All · I
All network I/O shall be asynchronous.

### REQ-PERF-002 Deadline first — All · I
No stage of the search pipeline shall wait for an engine beyond LIM-S4; the
aggregator's code is inspected for this at every change.

### REQ-PERF-003 Memory ceiling — M3 · A
The memory needed by one process shall be derivable from the limits table:
LIM-C3 × LIM-F3 for bodies, LIM-C5 × parse cost for trees, plus a fixed base.
A measurement report shall confirm it in M3.

### REQ-PERF-004 Overhead — M1 · A
The time rypsok adds beyond the slowest engine shall be measured and
reported; the reserve LIM-S6 shall be set from the measurement.

### REQ-PERF-005 Cold start — M1 · A
The latency of the first call of a process shall be measured before LIM-S4
is fixed. (R-10)

---

## 20. Data lifecycle

### REQ-LIFE-001 Intake — M1 · T
Validation and privacy check precede every outbound request.

### REQ-LIFE-002 Disposal — M1 · T
Bodies, queries, URLs and intermediate results shall be released when the
call ends.

### REQ-LIFE-003 Retention classes — M1 · I
The external design shall list every class of data with its retention:
request lifetime, process lifetime, cache lifetime, or durable
(configuration and secrets only).

---

## 21. Testing

### REQ-TEST-001 Specification-derived tests — All · I
Every requirement marked `T` shall have at least one test that names it.

### REQ-TEST-002 Driver fixtures — M1 · T
Every driver shall have fixture tests for a successful response, an empty
response, and each error class.

### REQ-TEST-003 Live checks — M2 · D
Each driver shall have a scheduled live check, outside the CI gate, within
the provider's terms, that detects format drift. (R-12)

### REQ-TEST-004 Deadline and partial-failure tests — M1 · T
Tests shall verify LIM-S4, LIM-S5, early completion, cancellation, and that
one failing engine never removes the results of another.

### REQ-TEST-005 Egress tests — M3 · T
Tests shall cover every refused range of REQ-SEC-006 in IPv4 and IPv6,
address literals in decimal, octal and hexadecimal notation, IPv4-mapped
IPv6, DNS answers that change between resolution and connection, redirects
to refused destinations, credentials in URLs, forbidden ports and methods.
The policy under test shall be the production policy; tests reach a local
server only through an explicit allow rule injected in code. (R-11)

### REQ-TEST-006 Resource tests — M3 · T
Tests shall verify byte limits under compression, node and depth limits,
concurrency limits, and that a hostile document ends the call and not the
process.

### REQ-TEST-007 Content boundary tests — M1 · T
Tests shall feed titles, snippets and pages that contain closing markers,
Markdown, hidden elements, invisible characters, instruction-like text and
non-ASCII host names, and shall verify the structure of the response, the
notices and the ASCII form of returned hosts. They shall also feed provider
text through the error, status and log paths and verify that none of it
appears there.

### REQ-TEST-008 Privacy tests — M1 · T
Tests shall verify each mode of LIM-P1 and that no query appears in logs at
the default level.

### REQ-TEST-009 Protocol tests — M1 · T
Tests shall verify the tool list, the schemas and their budget (LIM-I1 to
LIM-I3), the annotations, cancellation, stdout purity, and shutdown on end
of file, against every supported revision.

### REQ-TEST-010 Cache tests — M5 · T
Tests shall verify eligibility, isolation and expiration.

### REQ-TEST-011 Determinism tests — M1 · T
Tests shall verify REQ-ARCH-004 with shuffled arrival orders.

### REQ-TEST-013 Fuzz and property tests — M1 · T
URL handling, cleaning and, from M3, extraction shall have fuzz or property
tests that run in CI with a fixed time budget. (M-21)

### REQ-TEST-014 Listener tests — M4 · T
Tests shall verify `Origin` and `Host` validation, the loopback default,
authentication, and per-caller limits.

### REQ-TEST-015 Engine profile tests — M1 · T
Tests shall verify that a credential header is never sent to another origin,
that engine requests use HTTPS, and that environment proxy variables are
ignored.

### REQ-TEST-016 Limit tests — M1 · T
Tests shall verify the invocation limits of REQ-MCP-010 and the provider
limits of REQ-CMP-002.

### REQ-TEST-017 Circuit tests — M2 · T
Tests shall verify opening, cooldown growth, probing and closing of the
circuit, and `Retry-After` handling.

### REQ-TEST-012 CI gate — M0 · T
Formatting, lints (including one that forbids printing to standard output in
project code), tests, the dependency audit, the license check and the
dependency rule of REQ-ARCH-005 shall run on every change.

---

## 22. Maintainability and versioning

### REQ-MAIN-001 Isolated drivers — All · I
A change of one driver shall touch no other driver and no shared stage.

### REQ-MAIN-002 Tools depend on the core only — All · I
Protocol adapters shall depend on the application services, never on a
driver.

### REQ-MAIN-003 Versioned configuration — M1 · T
The configuration file shall carry a schema version; an unknown version
shall be refused at startup.

### REQ-MAIN-004 Safe extension — All · I
Adding an engine shall require a driver, its admission record and its
configuration entry, and nothing else.

### REQ-MAIN-005 Testable without the network — All · T
Every stage shall be testable with fixtures and a local server.

### REQ-MAIN-006 Contract versioning — M1 · I
Crates follow semantic versioning. A change of a tool's name, a parameter, a
response field, an outcome or an error kind is a breaking change of the
external contract and shall be announced in the changelog before 1.0.0 and
carried by a major version after it. (M-22)

---

## 23. Platforms, deployment and operation

### REQ-RUST-001 Rust — All · I
rypsok is implemented in Rust, edition 2024, minimum version 1.88.
(D-23)

### REQ-RUST-002 Platforms — M1 · T
Releases shall build and pass tests on Linux, macOS and Windows, on x86_64
and aarch64. (D-23)

### REQ-RUST-003 Typed boundaries — All · I
Every external structure shall be an explicit type validated at
deserialization.

### REQ-RUST-004 No UI dependencies — All · I
No dependency whose purpose is human-facing rendering.

### REQ-CFG-001 Configuration file — M1 · T
All operational settings shall come from one TOML file, located by
`--config` or `RYPSOK_CONFIG`, with documented defaults for every key.
(D-22)

### REQ-CFG-003 Sensible defaults — M1 · T
Every default shall be the value of the limits table; a process started with
no configuration file shall run with keyless engines only.

### REQ-CFG-004 Secrets by reference — M1 · T
Credentials shall be given as the name of an environment variable or the
path of a file; an inline secret shall be refused at startup.

### REQ-CFG-005 Validation at startup — M1 · T
Invalid or unsafe configuration shall stop the process at startup with an
error that names the key; there is no hot reload in the first releases.

### REQ-OPS-001 Command line — M1 · T
The binary shall offer `serve` (with `--transport stdio|http`), `check`
(validate configuration and exit), and `--version`; usage errors exit with
code 2, configuration errors with 1, success with 0. (M-09)

### REQ-OPS-002 Signals and shutdown — M1 · T
`SIGTERM` and end of file on stdin shall start a graceful shutdown that
aborts in-flight calls and exits within LIM-O1.

### REQ-OPS-003 Zero-configuration run — M1 · D
`rypsok serve` with no configuration shall serve `web_search` with the
keyless engines and state in its log which engines are inactive for lack of
a key. (A-13)

### REQ-DEP-001 Local mode — M1 · D
rypsok shall run as a child process of an MCP host over stdio.

### REQ-DEP-002 Network mode — M4 · D
rypsok shall run as a service reachable over Streamable HTTP with
authentication.

### REQ-DEP-003 Deployment egress control — M3 · I
The documentation shall describe how to restrict rypsok's outbound network
at the operating-system level in addition to the egress policy.

### REQ-DEP-004 Container limits — M1 · I
rypsok shall run under operating-system memory, CPU, file-descriptor and
network limits and shall document its needs.

### REQ-DEP-005 Distribution — M1 · D
Releases shall be published on crates.io and as binaries on the release
page, and from M4 also as a container image; tags carry no `v` prefix.
(D-32)

---

## 24. Deferred requirements

| ID | Deferred capability | Condition |
|---|---|---|
| DR-001 | Adaptive engine selection from measured health | After M5, own RFC |
| DR-002 | Ranking that uses observed provider quality | After M5, own RFC |
| DR-003 | Persistent local cache | Only with the privacy RFC of REQ-CACHE-007 |
| DR-004 | Vector similarity search | Only with the privacy RFC of REQ-CACHE-007 |
| DR-005 | Further engines: developer and scientific sources | Each through its admission record |
| DR-006 | Scraping drivers for human-facing interfaces, as SearXNG offers | Own engine type, off by default, enabled by the operator at the operator's risk, own RFC (D-06) |
| DR-007 | PDF extraction | Own RFC |
| DR-008 | Compact context rendering | Own RFC with encoding rules (REQ-CTX-003) |
| DR-009 | Caller-supplied deadline that shortens a call | Own RFC (X-04) |
| DR-010 | Multi-tenant HTTP service | Own RFC (D-29) |
| DR-011 | OpenTelemetry export | Own RFC (REQ-OBS-008) |

---

## 25. Acceptance per milestone

| Milestone | Accepted when |
|---|---|
| M0 | REQ-ARCH-005, REQ-SEC-020, REQ-TEST-012 hold; specifications, glossary, threat register and limits table are baselined |
| M1 | Every requirement marked M1 passes its verification; an MCP host invokes `web_search` over stdio and receives a structured, bounded response with the content boundary and privacy check in force |
| M2 | Health, circuit breaking, provider limits, `engine_status` and metrics verified |
| M3 | `web_fetch` with the full egress policy, robots.txt, extraction and resource tests verified |
| M4 | Streamable HTTP with listener protections and authentication verified |
| M5 | Cache and further engines verified |

---

## Appendix A — Disposition of the draft requirements

Every identifier of the draft of 2026-09-28 is accounted for.

| Draft identifier | Disposition |
|---|---|
| REQ-SEARCH-004, REQ-SEARCH-005 | Renumbered to REQ-SEARCH-006, REQ-SEARCH-007; the freed numbers hold the new `language` and `time_range` parameters |
| REQ-RANK-004, -005, -006 | Renumbered to REQ-RANK-005, -006, -007; REQ-RANK-004 now holds deduplication |
| REQ-RES-001 | Merged into REQ-ENG-008 |
| REQ-RES-003 | Merged into REQ-ENG-009 |
| REQ-RES-006 | Merged into REQ-AGG-004 |
| REQ-RES-007 | Rewritten to point to REQ-CMP-004 |
| REQ-CFG-002 | Merged into REQ-ENG-004 |
| REQ-ERR-002, REQ-ERR-005 | Merged into REQ-AGG-004, REQ-AGG-007; the numbers now hold error kinds and empty semantics |
| REQ-ERR-003 | Renumbered to REQ-ERR-005 |
| REQ-SEC-012 | Points to REQ-AGG-005 |
| REQ-PERF-002 to REQ-PERF-006 | Replaced by REQ-PERF-002 to REQ-PERF-005 with measurable content |
| REQ-LIFE-001, REQ-LIFE-002, REQ-LIFE-003, REQ-LIFE-004, REQ-LIFE-005, REQ-LIFE-006, REQ-LIFE-007, REQ-LIFE-008, REQ-LIFE-009 | Condensed into REQ-LIFE-001 to REQ-LIFE-003; the stage order lives in REQ-PAGE-001 and the external design |
| REQ-TEST-001 to REQ-TEST-012 | Rewritten as REQ-TEST-001 to REQ-TEST-012 with new content |
| REQ-OBS-005, REQ-OBS-006 | Kept, now M1 |
| REQ-DEP-003 (configuration injection) | Merged into REQ-CFG-004; the number now holds deployment egress control |
| REQ-DEP-004, REQ-DEP-005 | Renumbered to REQ-DEP-003 and REQ-DEP-004; REQ-DEP-005 now holds distribution |
| REQ-RUST-002 (async model) | Merged into REQ-PERF-001; the number now holds platforms |
| DR-001 to DR-005 | Kept; DR-006 to DR-011 added |
| REQ-STATUS-002 (operational metrics) | Topic moved to REQ-OBS-002; the number now holds "no secrets, no provider text" |
| REQ-CTX-004 (compact link representation) | Topic deferred as DR-008; the number now holds snippet and title cleaning |
| REQ-PRIV-005 (sensitive request classification) | Dropped: no fetch carries credentials (REQ-FETCH-007); the number now holds the disclosure statement |
| REQ-CACHE-003 (private data exclusion) | Topic in REQ-CACHE-002; the number now holds eligibility |
| All other identifiers | Kept, with rewritten and verifiable statements |

New groups: REQ-CMP (compliance), REQ-OPS (operator interface).
New identifiers: REQ-ARCH-005 to -008, REQ-MCP-006 to -011, REQ-SEARCH-004, -005 and -008, REQ-FETCH-005 to -011, REQ-STATUS-003, REQ-ENG-010 to -012,
REQ-AGG-008 and -009, REQ-CTX-009 and -010, REQ-PAGE-007 and -008,
REQ-SEC-015 to -021, REQ-CACHE-007, REQ-TEST-013 to -017, REQ-OBS-007 and -008, REQ-MAIN-006,
REQ-RES-009.
