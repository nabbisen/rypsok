# Changelog

All notable changes to rypsok are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). Release tags carry no `v` prefix.

Until release 1.0.0 the external contract (tool names, parameters,
response fields, outcomes, error kinds) may change between minor
versions; every such change is announced here. From 1.0.0 the contract is
stable as the [stability page](docs/src/reference/stability.md) defines
it: what it lists as frozen changes only with a major version, additions
arrive in minor versions, and every change of a default value is
announced here with both values.

A change to a tool's definition (its name, description, input schema, output
schema or annotations, or the server instructions) is announced on a line of
its own under `### Tool definitions`, first under the version, because a host
that pins definitions needs to be told of each one (RFC 021).

## [Unreleased]

## [0.10.0] — 2026-11-11 — The rmcp security fix; call maxima; tool-definition changes announced

RFC 021 and RFC 022. A security fix of the HTTP transport, an upgrade of the
protocol library with the changes it shows on the wire, the call maxima shown
by `check`, and the rule that every change of a tool definition is announced
on a line of its own. No tool definition changed in this release.

### Security

- The protocol library `rmcp` is upgraded from 3.5.0 to 3.9.0 (RUSTSEC-2026-0461,
  RUSTSEC-2026-0457, RUSTSEC-2026-0462; RFC 022). In HTTP mode, every release
  from 0.4.0 to 0.9.0 let a client that reaches the endpoint grow the process's
  memory without bound, by sending `tools/call` requests with distinct tool
  names, until it failed: any local process when no callers are configured, or
  any authenticated caller. Unauthenticated clients were refused first when
  callers are configured, and stdio was not affected. 0.10.0 uses rmcp 3.9.0,
  which no longer keeps the names (RUSTSEC-2026-0461 was fixed in 3.6.0).
  Measured on release builds: 4,000 requests with distinct 8 KB names took
  0.9.0 (the published musl binary) from 11 to 44 MiB, and 0.10.0 (measured on
  a glibc build of the release tree) from 11 to 14 MiB, the same as 4,000
  requests with one repeated name; 20,000 distinct names leave 0.10.0 at the
  same level. If you cannot upgrade at once, configure callers or use stdio.
  RUSTSEC-2026-0457 and RUSTSEC-2026-0462 concern client code that rypsok does
  not use.

### Changed

**Contract changes** (announced as REQ-MAIN-006 requires; they are why this is
a minor release). They come with rmcp 3.9.0 and show on the wire; no tool
definition is among them:

- Over HTTP, a 2026-07-28 `tools/call` for a tool rypsok does not serve is
  answered with HTTP 200 and the JSON-RPC error `-32602` `unknown tool`, as the
  protocol library now answers. It was HTTP 400 with the same body.
- A 2026-07-28 `tools/list` answer, over stdio and over HTTP, now carries
  `"cacheScope": "private"` and `"ttlMs": 0` (the library's default cache
  hints: do not share or keep the list; `server/discover` already carried
  `cacheScope`). The `tools` array is unchanged.
- Over HTTP, a request id written as a float (`7.0`) is read as that integer
  and answered; it was taken for a notification and answered 202 with no body.
- Over HTTP, a request id of `null` is refused with 415; it was answered 202.
- Over HTTP, `Accept` is read by its media types and quality values: a type
  with `q=0` no longer counts as accepted (406), and the types are matched
  without regard to case.
- Over HTTP, a POST of the handshake-based revisions that is not `initialize`
  and has no session is answered 400 `Session ID is required`; it was 422.
- Over stdio, a first message that is neither `initialize` nor
  `server/discover` and lacks `_meta` is answered with an error and the server
  stays open; it ended the process with exit code 1.
- Over stdio, a message with both `result` and `error` is answered `-32600`.
- Over stdio, a line longer than 16 MiB (LIM-O3, a bound of the library that
  0.9.0 did not have) is dropped without an answer and the server reads on.

Other changes:

- A `web_search` call lasts at most LIM-S7 and a `web_fetch` call at most
  LIM-F5, measured from its start. Both bounds already held in 0.9.0, by the way
  each pipeline is built; they are now also enforced by one outer bound, with
  the existing deadline error (`unavailable`, reason `deadline`), and pinned by
  tests, so that they stay true if a pipeline changes (RFC 021, D1). Nothing
  changes for a call that ends in time, and no new kind, reason or notice is
  added.

### Added

- `rypsok check` prints each tool's maximum call time on a line of its own,
  `rypsok: tool call maxima in ms: web_search=2800 web_fetch=11000;
  engine_status reads memory only`, with the effective values (RFC 021, D2).
  The text of `check` is for people and may change in a minor version.
- `rypsok-core` gains the constant `limits::defaults::STDIO_MAX_LINE_BYTES`
  (LIM-O3, 16 MiB), the documented value of the stdio line bound above.

### Fixed

- `SIGTERM` is heard before the client has sent its first message. In 0.9.0 the
  server waited for that message before it listened for the signal, so a server
  that had been started and not yet used kept running until its input closed;
  it now exits with code 0 within LIM-O1. An end of input after a
  `server/discover` and nothing else is still a clean end (exit code 0), and
  an end of input before any message is still an error exit, as in 0.9.0.

### Internal

- Every change of a tool definition (or of the server instructions) is announced
  under `### Tool definitions`, and the release checks compare the pinned
  definitions with the last release tag (RFC 021, D3). The server instructions
  are now pinned in `crates/rypsok/fixtures/schemas/instructions.txt` beside
  `tools_list.json`.
- Dependencies updated for the cycle: the whole lock file (48 packages, one
  crate added, `utf_types`, under `utf8_iter`), the pinned actions of the
  workflows to the latest release of each, and the base image digest and the
  release compiler (1.99.1) re-read and unchanged. One update a user could
  notice: the Unicode data behind host-name handling (`icu_*` 2.4, through
  `url` and `idna`) is newer. 13,007 code points assigned since the old data
  are now accepted in an internationalized host name, where `web_fetch` refused
  the whole URL as invalid; no name accepted before changes its result.
- The threat register has THR-28 (an unbounded cache in the protocol library),
  and the limits table has LIM-O3.

## [0.9.0] — 2026-11-08 — The audit (milestone M8)

RFC 020: the project audit and its fixes. The [audit report](docs/src/maintainers/audit.md)
lists every finding, what was done, the requirement it is held to and the test
that pins it. This is a minor version before 1.0.0, so it carries announced
changes of the external contract and of the public items of `rypsok-core`;
they are under Changed and Removed.

### Changed

**Contract changes** (announced as REQ-MAIN-006 requires; they are why this is
a minor release):

- A redirect target is checked against robots.txt on its own host too, not
  only on another host (RFC 020, D-1): a call whose redirect target robots.txt
  disallows now answers `denied`/`robots_txt` where it returned the page. The
  verdict of the origin is cached, so the check costs no request.
- The built-in refusal table follows the IANA special-purpose address
  registries, read 2026-11-03 (RFC 020, D-2). These blocks are now refused:
  `2001:db8::/32` and `3fff::/20` (documentation), `5f00::/16` (segment
  routing), `100:0:0:1::/64` (dummy prefix), and the deprecated `::/96`
  (IPv4-compatible) and `fec0::/10` (site-local). A fetch of an address in
  them is `denied` (`egress`) unless an `allow` rule admits it, as for every
  built-in refusal.
- The parse time of a page is linear in its nesting depth, and a page deeper
  than 256 levels is read in part where it was read in full (RFC 020, D-3):
  the text after the deep part is not returned, and the answer carries the
  `extraction_partial` notice. A page nested 100,000 levels deep parses in
  under a millisecond (before: about 17 s for 80,000, with the two parse
  permits held and unrelated fetches waiting). LIM-F11 is a fixed limit (it was
  listed as configurable and has no key).
- A fetch says whether retrying can help (RFC 020, D-7): an HTTP error status
  ends "retry later" for 408, 425, 429 and 5xx, and says the site refused the
  request or has no such page, and that retrying the same URL will not help,
  for any other 4xx; `denied` with the reason `scheme` has two messages, one
  for a scheme that is not allowed and one for a failed retry over `https`;
  `detail.retry_after_ms` is set for a 408, 425, 429 or 5xx answer that carries
  `Retry-After` in seconds (an integer, at most 3,600; a date is ignored). The
  field, the kinds, the reasons and the 200-character bound do not change;
  `detail.status` is unchanged. The documentation of the `unavailable` kind no
  longer says that the caller may retry: whether retrying helps depends on
  `detail.reason`.
- Search results are merged on a stricter key (RFC 020, D-8): an explicit port
  that is not the default of the URL's scheme, a non-empty fragment and the
  order of repeated parameters are part of the identity, so `http://x:443/p` is
  no longer `https://x/p`, `/app#/inbox` is no longer `/app#/settings` and
  `?a=1&a=2` is no longer `?a=2&a=1`. `http` and `https` and the `www.` fold are
  unchanged. When `http` and `https` observations merge, the result shows the
  `https` URL even if the `http` one ranked first. Strict mode remembers
  returned URLs without their fragment, so a returned URL may be fetched with
  any fragment.
- Joiners keep the orthography they carry (RFC 020, D-10): the zero width
  non-joiner in Arabic-script and Indic text (Persian `می‌خواهم`) and the zero
  width joiner in Indic conjuncts and between emoji (`👨‍👩‍👧`) are no longer
  removed, and no longer counted in `characters_removed`. A joiner anywhere
  else, a second one in a row, and one at the start or the end of a text are
  removed and counted as before, as is every other invisible character; a flag
  built with tag characters (Scotland's) still comes back as the black flag,
  with the tags counted.
- The secret check recognizes more formats and names them in `detail.kind`
  (new values `npm_token`, `gitlab_token`, `huggingface_token`, `sendgrid_key`,
  `telegram_bot_token`, `twilio_key`, `aws_secret_key`, `password_assignment`,
  `basic_auth_header`, `azure_account_key`): npm, GitLab, Hugging Face,
  SendGrid, Telegram bot and Twilio keys, an AWS secret access key named by its
  context, `password=`, `passwd=` and `secret=` with a value that has a digit or
  a symbol, `Authorization: Basic …`, Azure `AccountKey=`, `Bearer%20…`, a
  lower-case AWS key id, a key with a zero width character inside and a GitHub
  token wrapped by a space. A request that carried one of these used to pass;
  with `secret_detection = "refuse"` it is now `denied`. An AWS key id
  beginning `ASIA` or `A3T` needs a digit in its sixteen-character tail;
  one beginning `AKIA` does not. The Slack, OpenAI, Stripe, JWT and Bearer
  shapes are stricter (the real format, not any run of letters), and ordinary
  text no longer trips the check: `sk-` or `xoxb-` followed by hyphenated words,
  `bearer` and a phrase, a 20-letter word after `ASIA`, `host.example:8080@mirror`,
  `ssh://git:@host/…`, three long dotted words starting `eyJ`, `sk_test_` and a
  plain word, and `password=` followed by an ordinary word all pass.
- `engine_status`: an entry whose `state` is `disabled` has a new optional
  field `reason`, `disabled` (the operator switched the engine off) or
  `no_credential` (it needs a key and none could be read). It is absent for
  every other state; the output schema gains the field.
- The URL a search result carries keeps the engine's query exactly, except
  that tracking parameters are removed (the scheme and the host are written in
  their canonical form and a default port is dropped, as before). Before, a
  query was rewritten whenever there was one: `?a=1&b` became `?a=1&b=`, `?a=1;b=2` became `?a=1%3Bb%3D2`,
  `?q=a%20b` became `?q=a+b`, `?a=%7e` became `?a=%7E` and `?x=%zz` became
  `?x=%25zz`. The query is rebuilt only when a tracking parameter was removed,
  and the other parameters keep their text (RFC 020, DAT-8).
- A search result whose URL has a host with an empty label (such as
  `https://.example/`) is dropped and counted as `url_invalid`, as other invalid
  URLs are, instead of being shown (RFC 020, D-24). A result's host must pass
  the check `web_fetch` applies, so the search never shows a URL that the fetch
  refuses on syntax; the egress policy is still decided at fetch time.
- Two configuration ceilings are lowered; the defaults do not change (RFC 020,
  D-20 and D-21): `fetch.max_bytes` is now 65536 to 10485760 (was up to
  20971520) and `service.http.max_sessions` is now 1 to 2048 (was up to 65536).
  Measured on the delivered binary: four fetches of a 20 MiB page took 265 to
  282 MiB, above the 256 MiB of the documented systemd unit (the kernel killed
  the process in 3 of 3 runs), and 65536 sessions took 3.7 GiB at about 60 KiB
  each. A configuration that sets a value above the new ceiling is refused with
  the usual "must be between" message.
- The header block of an HTTP request (request line and headers on HTTP/1, the
  header list on HTTP/2) is bounded at 16 KiB, a new fixed limit LIM-H7. A
  larger block is answered 431 (HTTP/2: the connection is closed); hyper's
  default for HTTP/1 was about 400 KB.
- A connection to the HTTP listener that never sends a byte, or that stops
  before the protocol is known (a single `P`, part of the HTTP/2 preface, or a
  preface sent a byte at a time), is closed after `idle_timeout_s` counted from
  accept, and frees its connection slot (LIM-H3 now holds for it too; RFC 020,
  D-4). Before, `max_connections` such connections kept every legitimate caller
  out for as long as they stayed open. The close is counted as the new refusal
  reason `idle_connection` in the `counters` event (`refusals`, under `http`),
  and not logged.

Other changes:

- Configuration messages say what to do (RFC 020, DAT-11, DAT-12, DAT-14). A
  value of the wrong kind is reported as `<key path>: expected <what>, found
  <what> (line N, column M)` in the words of the file, for example
  `search.max_results: expected an integer from 1 to 20, found text (line 3,
  column 15)`, where it was `invalid type: string "five", expected u8 (at byte
  42)`; a syntax error, a duplicate and an unknown key have a key path and a
  line and column too. A text that was found is never repeated. Where a range
  depends on another key the message names that key and its value
  (`search.engine_timeout_ms: must be between 100 and 300 (search.deadline_ms
  500 minus search.reserve_ms 200)`), and a deny or allow rule that ends in a
  dot says so. The wording changed; the rules did not: a file that was valid is
  valid.
- A message never repeats a value from the file (RFC 020): an unknown word for
  a setting is reported as `found an unknown word` after the list of the
  allowed words, and the name in `api_key_env` or `token_env` is shown in
  "environment variable X is unset or empty" only when it looks like a variable
  name (upper-case letters, digits and underscores). A secret pasted into those
  keys by mistake used to come back in the message, in `check` and in the log of
  `serve`.
- `rypsok check` prints more (RFC 020, DAT-2, D-6, D-20, D-21): a line for every
  section it said nothing of (`service.http`, `fetch`, `egress`, `privacy`,
  `logging`, `cache`, `scraping`, `resilience`, `limits`); the worst-case memory
  of the settings, for the build that runs it (`13 MiB + fetches × max_bytes +
  parses × (5.2 × max_bytes + 11 MiB) + max_sessions × 60 KiB` while
  `legacy_revisions` is on, on musl; 14 MiB and `10.6 × max_bytes + 21 MiB` on a
  glibc build; both fitted on the delivered four fetches and two parses, and
  above every measured peak; a platform that was not measured prints the figure
  with a note and is not compared with the unit); and warnings, each one line
  `rypsok: warning: <key>: <what happens>; <the safer setting>`, for an allow
  rule wider than /16 or /32, `secret_detection = "off"`, a bind off loopback
  (plain HTTP), `log_queries`, `robots_txt = false`, `allow_http`,
  `follow_cross_host_redirects`, a scraping engine, a cache with no storable
  engine and a worst case above 256 MiB (122 MiB for the defaults on musl, 197 on
  glibc). The exit code does not change. There is no warning for
  `legacy_revisions`. The format of the lines is not part of the contract.
- The stability page says what the code holds about `rypsok-core` (RFC 020,
  D-11): the frozen surface is the facade (`Rypsok`) and the types of the `model`
  module; the other public items of the crate are public for the `rypsok` crate
  and are outside the promise until 1.0.0 decides what else is frozen. The page
  lists the types that are `#[non_exhaustive]` and the types that are
  deliberately exhaustive (`Outcome`, `ErrorKind`, `CallError`, `EngineState`,
  `TimeRange` and the validated identifiers), and a test keeps the lists equal
  to the source. It used to say that all public types are `#[non_exhaustive]`,
  which 16 of the surface's types are.
- The book is published from the release tag and says on its first page which
  release it describes (RFC 020, D-17): a push to `main` publishes nothing, a
  tag `X.Y.Z` publishes, and a manual run publishes the ref it was started from
  and says so; a failed publish is red. The owner allows the tag in the
  `github-pages` environment (`releasing.md`).
- Documents: the deployment page adds `MemorySwapMax=0` and `--memory-swap
  256m` (without them a host with swap slows down instead of restarting the
  process), says which certificates the fetches trust (the system store, and on
  Linux `SSL_CERT_FILE` and `SSL_CERT_DIR`; a start without any root fails at
  once) and the measured memory of the delivered defaults (98 to 107 MiB on the
  musl build, 171 to 187 MiB on a glibc build); the Wikipedia admission record
  states the burst wait; the basis of LIM-S4 names its measurements. Every audit
  decision in the book is written `RFC 020, D-n`, and a bare `D-nn` is said to
  be a decision of the startup review.
- The container image is made from the release binary instead of being built
  from source with another compiler: the `Dockerfile` compiles nothing and
  copies the binary of the release onto the pinned distroless base (RFC 020,
  D-12). `docker build` needs the musl binary under `dist/linux-<arch>/`.
- The release binaries are built with a named compiler (1.99.1) instead of the
  day's `stable`, and the Windows executable is linked with `/Brepro` (no
  timestamp in the header), so that two builds of one commit give the same
  binary. The release notes name the compiler. The binaries are stripped of
  symbols (`strip = "symbols"`; RFC 020, D-22): the Linux x86_64 binary shrinks
  from 18,997,040 to 13,692,376 bytes (28 %). Unwinding stays, and there is no
  LTO. Nothing a binary does changes.

### Added

- Two settings for limits that had no key (RFC 020, D-9):
  `search.connect_timeout_ms` (LIM-S12, 200 to 5000, default 1000) and
  `service.shutdown_ms` (LIM-O1, 500 to 10000, default 3000). The defaults are
  the values the program used; `rypsok check` prints both. The limits table now
  calls "fixed" every row that no key sets.
- Every event of a tool call carries `request_id` and `caller` as top-level
  fields of its log line (RFC 009 and RFC 013 promised them; no line had them):
  a per-call identifier (the start time in milliseconds and a sequence number,
  in hexadecimal; unique within the process, different for each call, not
  returned to the caller and in no response) and the caller's id (`local` over
  stdio and on a listener without callers). They are on the engines' events, the
  refusals, `fetched` and the closing `call` event, at every log level, and
  calls that overlap do not mix their ids. `engine_status` logs a `call` event
  like the other tools, with its outcome and latency. The events of the MCP SDK
  and of the HTTP stack, and of the listener's gate, are outside a call and have
  neither field. `Rypsok::fetch_as` and `Rypsok::status_as` take the caller, as
  `search_as` does; `fetch` and `status` are as before and say `local`. The log
  format is not part of the contract.
- Linux aarch64: a static binary (`aarch64-unknown-linux-musl`) is built for
  each release and CI runs on GitHub's native arm runner (RFC 020, D-13). macOS
  x86_64 and Windows aarch64 are not built.
- The container images (Linux x86_64 and aarch64) are files of the release,
  `rypsok-X.Y.Z-image-amd64.tar.gz` and `-arm64.tar.gz`, and are listed in
  `SHA256SUMS` (RFC 020, D-12). There is still no registry.
- Provenance attestations (RFC 020, D-16): on a release tag the `checksums` job
  attests the four archives and the two images, so that `gh attestation verify
  <file> --repo nabbisen/rypsok` shows that the release workflow built a file.
  Only that job can sign, and it runs no build code.

### Removed

- From `rypsok-core`'s public items (RFC 020, TST-10; none is in the frozen
  surface of the stability page): the type `UserAgentFormat` and the field
  `EngineProperties::user_agent`; the six builders
  `EngineProperties::with_max_query_words`, `with_languages`,
  `with_attribution_required`, `with_user_agent`, `with_max_response_bytes` and
  `with_scraping`; `Rypsok::returned_urls`; the fields `fetch_ports`,
  `http_max_connections`, `http_max_request_bytes`, `http_idle_timeout_s`,
  `http_max_streams`, `http_request_timeout_s` and `http_max_sessions` of
  `Limits` (the listener and the fetch service read the configuration, which is
  the one source of those numbers). `Limits::worst_case_memory` takes the number
  of sessions as its first argument, and `EngineLimiter::acquire` takes a
  `tokio::time::Instant`.
- `SearchResult` and `SearchResponse` no longer implement `Deserialize`: a
  result is built only by the content boundary, and reading one from JSON would
  have been a second way. Nothing in the tree needed to read them back; the
  tests that did compare the serialized form instead, or read the documentation
  examples into a private mirror of the type.

### Fixed

- A panic is logged as one JSON line without its message (`event` `panic`,
  `level` `ERROR`, `location`, `thread`); before, the standard hook wrote its
  location and message as plain text to standard error, outside the JSON log
  and its privacy rules. A panic inside the extraction of a page still ends that
  call only.
- A panic inside a critical section no longer makes every later call panic:
  the counters, health table, limiters, robots.txt cache, returned URL memory
  and the per-caller limiters recover a poisoned lock instead of failing on it.
  The result cache keeps disabling itself on a poisoned lock.
- The client that fetches pages probes a pooled connection every 60 s
  (`tcp_keepalive`), as RFC 011 says; it only dropped an idle one after 30 s.
- The listener no longer spins and floods the log when `accept` fails (for
  instance when the descriptor limit is below `max_connections`): it waits 100
  ms between attempts and logs the error at most once per 10 s. The deployment
  page advises `LimitNOFILE` of at least `max_connections` + 64.
- The `Bearer` scheme of the `Authorization` header is matched without regard to
  case (RFC 9110), followed by one or more spaces.
- `web_fetch` errors no longer mislead. A URL whose scheme is not `http` or
  `https` (`ftp://…`) is no longer told that "the http URL was retried over
  https". A fetch that ran out of time says "the page was not retrieved before
  the deadline" and not that "no engine answered". The unknown `category` error
  lists only the categories the process serves (it listed all four). The
  URL-too-long message gives the number of characters allowed, not the name of
  the limit. The `web_fetch` description in the tool definition reads "starting
  at `start_char`", as the design does, and the `extraction_partial` notice is
  documented as what it is: the page was larger than rypsok reads, in nodes or
  nesting, and was read in part.
- `rypsok check` accepted configurations that `serve --transport http` refuses:
  a caller whose token variable is unset, whose token file cannot be read or
  whose token is shorter than 32 bytes. `check` reads the callers' tokens as
  `serve` does, fails with the same message and exits 1 (RFC 020, DAT-1).
- A DuckDuckGo search with no results answers with no results, instead of
  counting as a parse failure that could open the engine's circuit (RFC 020,
  NET-18): the provider's no-results page marks itself with `span.no-results`,
  and the driver looked for a `div`. Three searches with no results took the
  engine offline for 60 s and told callers it had failed.
- A client that cannot be built says why, once: `rypsok: configuration: fetch:
  the HTTP client could not be built: no root certificates could be loaded (the
  system store, or SSL_CERT_FILE / SSL_CERT_DIR when set; ...)`. It used to say
  "could not be built" twice and only `builder error`. The reason of an engine
  that is inactive for the same cause changes the same way.

### Internal

- Checks run per change and before every release, not on a timer (RFC 020,
  D-23). `release-checks.yml` (reproducibility of the release builds of every
  target, every one of the 14 fuzz targets for 300 s, mutation testing of the six
  security modules, the live checks and the advisory check) replaces the weekly
  `scheduled.yml`; the live checks and the advisories are also
  `external-checks.yml`, run on demand. The release build is one local action,
  `.github/actions/build-release`, which alone names the release compiler. New
  requirement REQ-TEST-019. `docs/src/maintainers/checks.md` says when each check
  runs and what red means.
- Fuzz targets on parsers of untrusted input: five more (`judge_url`,
  `parse_rule`, `robots_txt`, `engine_json`, `headers`, 14 in all), every one
  with seed inputs in `fuzz/corpus/<target>/seed-*`. Some items of `rypsok-core`
  were made public (hidden from the documentation) and the egress rule types
  gained `Display` for them; none is part of the stable surface.
- The verification debt on the security paths. Tests for what the audit found
  unverified: a successful `http` to `https` upgrade against a TLS server with a
  certificate made at test time, what a fetch sends (method, headers, no cookie,
  no credentials, no proxy), the engine client's timeout, hostile provider text
  in errors and logs, every error constructor's message, and a gate that every
  request is sent on a client `egress.rs` built; a test for 48 of the 54
  mutants that `cargo-mutants` found alive on the security paths and
  `.cargo/mutants.toml` with the six that cannot change behaviour;
  `docs/src/maintainers/verification-ledger.md` lists the requirements whose
  tests are still partial. A property test that could generate `0.0.0.0` as a
  host name and fail was corrected. `rcgen` is a dev-dependency of
  `rypsok-core`.
- Tests that depended on elapsed time run with paused time and exact assertions;
  the crate builds without a warning with no engine feature and with each one
  alone (CI checks it); `cfg(test)` helpers moved into test modules; the page
  behind four example answers is in `examples/pages/`.
- `tools/measure-memory --defaults` measures the delivered configuration
  (default limits, the largest page `fetch.max_bytes` admits, four hosts) and
  prints the worst case of `rypsok check` beside it. `EstimateBasis` and
  `Limits::worst_case_memory_for` select the constants of a build.

## [0.8.0] — 2026-10-29 — Corrections from the first independent review

RFC 019: the fixes of handoffs 019a to 019c, found by the first
independent review of 0.7.0. This is a minor version before 1.0.0, so it
carries announced changes of the external contract and of the public items
of `rypsok-core`; they are under Changed.

### Added

- `[service.http] max_sessions` (LIM-H6, default 256, 1 to 65536): the
  sessions of the handshake-based protocol revisions are bounded, and a
  handshake beyond the bound is answered 503 with `Retry-After: 30` and
  logged, instead of holding memory without limit (REQ-SEC-024, THR-27).
  The stateless revision opens no session and is unaffected.
- Every request the HTTP listener refuses, by its own gate or by the SDK
  behind it, is counted under `http` with its reason and appears in the
  once-a-minute counters line, also when no tool is called (REQ-OBS-007).
- Binaries are attached to a release: Linux x86_64 (static), macOS arm64
  and Windows x86_64 archives and `SHA256SUMS`, by the `release` workflow
  from a release the owner has published, never replacing an asset
  (REQ-DEP-005). The maintainers' book has a page on the procedure.
- The fuzz target `clean_decoded_text`.
- `tools/check-limits` and `cargo deny` with `bans` run in CI; a test
  keeps every page of the book in its summary; a test runs the headers of
  the documented `curl` against the binary.

### Changed

- RFCs 001 to 018 moved to `rfcs/done/` with the release each shipped in,
  as RFC 000 prescribes; no design changed.
- A site `web_fetch` cannot reach is `unavailable` / `network`, where an
  unreachable robots.txt was answered `denied` / `robots_txt`; a name the
  egress policy refuses (a private address behind a name) is answered
  exactly like a name that does not resolve, so that the answer does not
  tell a caller which internal names exist. The operator's log names the
  refusal (`refused`, with the denial, without the host). A robots.txt
  that answers 5xx is still `denied` / `robots_txt`. An `http` URL whose
  `https` attempt fails is `denied` / `scheme` whether `robots_txt` is on
  or off. These are changes of error kinds and reasons (the announcement
  the preamble requires).
- A handshake over HTTP beyond `max_sessions` is answered 503 with
  `Retry-After`. The statuses of the listener are documented by owner: the
  gate answers 401, 404, 408, 413 and 503; the SDK behind it answers 400,
  403 and 406.
- Public items of `rypsok-core`:
  - renamed: `boundary::clean_text` is `boundary::clean_markup_text` (no
    alias is kept); the new `boundary::clean_decoded_text` serves text that
    a parser already decoded (page titles and scraped results);
  - added: `egress::Host::from_name` (the one constructor of a host name:
    lower-case, one root dot dropped, empty labels refused),
    `fetch::SendFailure`, `fetch::robots::Decision::Unreachable` (a new
    variant of an exhaustive enum), `boundary::Provenance`,
    `boundary::make_decoded_result`, `engine::RawResult::{decoded,
    provenance}`, `Rypsok::{shared_counters, with_counters}`,
    `Counters::{refused_http, with_flush_interval}`. `Rypsok` now keeps
    its counters in a shared handle (a private field).
- `<` followed by a non-ASCII letter is kept as text (2026-10-27).
- Control characters are removed before the markup step (2026-10-28).

These two cleaner changes were made in 0.6.0 and 0.7.0 and were not listed.

### Fixed

- A trailing root dot (`blocked.invalid.`) bypassed an operator `deny`
  host rule: names are compared in one canonical form, and a name with an
  empty label is refused.
- An unreachable site was reported as a robots.txt refusal; a refused name
  answered differently from an unreachable one.
- A title or a scraped result lost `<T>` in `Vec<T>` as if it were markup.
- The bearer token comparison treated two tokens whose lengths differ by a
  multiple of 256 as the same length.
- The documented `curl` answered 406: it now lists both `Accept` media
  types.
- The DuckDuckGo record was missing from the book's summary.
- The book said binaries were attached to every release while none were.
- The tutorial and the troubleshooting page said different things about a
  corrected key; one sentence is used: the key is read at startup, and a
  corrected key takes effect at the next restart, the engine being asked
  meanwhile.
- The `IPAddressAllow` examples now name a placeholder for the engines'
  addresses.

## [0.7.0] — 2026-10-28 — Security (milestone M7)

### Added

- The threat model page and the register revised against the delivered
  system: THR-26 (slow or stream-flooding HTTP clients), THR-22 extended
  to CI actions, the scraping threat renumbered THR-25, two residual
  risks recorded, the review log caught up (RFC 018).
- `[service.http] request_timeout_s` (LIM-H5) and `max_streams` (LIM-H4):
  a request still running after the bound is answered 408; one HTTP/2
  connection opens at most `max_streams` streams.
- `fuzz/`: eight fuzz targets on every parser that reads external bytes,
  run by the scheduled workflow, report only.
- Every CI action pinned to a commit hash, with a gate test.
- Hardened systemd and container profiles on the deployment page.

### Changed

- Invariant I5 states the one exception it always had: a driver may log
  at most 200 characters of a provider's error body, at `debug` only.

## [0.6.0] — 2026-10-27 — Users (milestone M6)

### Added

- The book's part for new users: five tutorials (a first search by hand
  and with a runtime, a keyed engine, reading a page in parts, network
  mode behind a proxy, the scraping switch), "Reading a response" for
  the author of a host's prompt, and "Troubleshooting" (RFC 017).
- `examples/`: configurations, host entries and real call answers that
  the tutorials include and CI checks; the first tutorial runs end to
  end in CI.
- A workflow that builds the book on every push and publishes it to
  GitHub Pages once Pages is enabled.

- The stability page: what is frozen from 1.0.0, what may be added in a
  minor version, and what may change with notice (defaults, logs, the
  output of `check`) (RFC 016).
- A CI job running `cargo semver-checks` on both crates from the first
  commit after 1.0.0.
- `rypsok-core`: constructors for the driver interface
  (`EngineProperties::new` and `with_*`, `RawResult::new`,
  `EngineRequest::new`, `EngineResponse::new`, `DriverContext::new`),
  `FetchRequest::new`, `ConfigError::new`.

### Changed

- `rypsok-core`: the requests, responses, driver types, configuration
  tables and the growable vocabularies (`NoticeCode`, `Reason`,
  `DetailReason`, `SecretKind`, `Category`, `EngineErrorClass`,
  `LanguageSupport`, `TimeRangeSupport`, `UserAgentFormat`) are
  `#[non_exhaustive]`: build them through their constructors and match
  them with a wildcard arm. The frozen vocabularies (`Outcome`,
  `ErrorKind`, `CallError`, `EngineState`, `TimeRange`) stay exhaustive.
- A `max_results` outside 1 to 255 is refused with the operator's
  ceiling in the message, as a value within that range already was.
- `serve --help` no longer mentions a milestone.

## [0.5.0] — 2026-10-27 — Optimization (milestone M5)

### Added

- `[cache]`: an in-memory result cache, off by default; one entry per
  engine answer per caller, only for engines whose admission record
  allows storage; a hit is reported by the notice `from_cache`; bounded
  by `ttl_s`, `max_entries` and `max_entry_bytes` (LIM-K1 to LIM-K4,
  RFC 014). `web_fetch` is never cached.
- `CallerId` is a core type; `Rypsok::search_as` searches on behalf of a
  caller (the caller of the HTTP gate, or `local`).
- Scraping drivers as an opt-in engine type behind `[scraping] enabled`
  (off by default; a scraping table is refused without it; never in the
  default set); the standard User-Agent and no browser imitation; a
  challenge is `blocked`; bounded like `web_fetch` and parsed under the
  shared parsing permits (LIM-G1, RFC 015). First driver: `duckduckgo`
  (the HTML endpoint), with its admission record.
- `rypsok_core::html`: the bounded DOM and charset decoding, shared by
  the fetch pipeline and the scraping drivers (moved out of `fetch`).

### Changed

- `legacy_revisions = false` now refuses the handshake-based revisions:
  `initialize` gets the JSON-RPC error `-32022` and `server/discover`
  lists the 2026-07-28 revision alone (RFC 013 amendment).
- A bind on an unspecified address (`0.0.0.0`, `::`) requires
  `allowed_hosts`; the bind host is no longer admitted as a `Host`.
- HTTP/2 connections get a keep-alive ping after `idle_timeout_s` and are
  closed when it goes unanswered (LIM-H3 on both HTTP versions).

### Documentation

- The headers a host must send over HTTP (`MCP-Protocol-Version`,
  `Mcp-Method`, `Mcp-Name`) with a complete example.

## [0.4.0] — 2026-10-27 — Network transport (milestone M4)

### Added

- `serve --transport http`: MCP over Streamable HTTP on `/mcp`, loopback
  by default; the 2026-07-28 revision served statelessly, the
  handshake-based revisions 2024-11-05 to 2025-11-25 with sessions in
  memory until `legacy_revisions = false` (RFC 013).
- `Host` and `Origin` validation; a bind off loopback requires callers.
- Static bearer tokens per caller, read from the environment or a file,
  compared in constant time; rate limits per caller.
- Bounds on connections, request bodies and idle time (LIM-H1 to LIM-H3).
- A `Dockerfile` (static binary, distroless, non-root) and the deployment
  page.

### Supported protocol revisions

2026-07-28 (current, stateless) and the handshake-based revisions the SDK
serves: 2025-11-25, 2025-06-18, 2025-03-26, 2024-11-05.

## [0.3.0] — 2026-10-27 — Secure fetch (milestone M3)

### Added

- `web_fetch`: the main content of one public page as Markdown, bounded by
  `max_chars`, continuable by `start_char`; HTML, XHTML, plain text and
  Markdown; charset from the header, a `meta` declaration or detection;
  parsing bounded in nodes and depth with `extraction_partial` when a
  bound is hit; the error kinds `redirected`, `unsupported_content`,
  `too_large` and the fetch reasons of `denied` and `unavailable` (RFC 012).
- The agent-directed egress profile: a policy that refuses loopback,
  private, link-local, metadata and other internal destinations, judged
  on the URL before any request and on every address at connect time
  through the client's resolver; operator `allow` and `deny` rules;
  address literals in every notation; names that never name a public site
  (RFC 011).
- Strict fetch mode: only URLs `web_search` returned in this process, or
  the operator's allow list, can be fetched.
- robots.txt honored per RFC 9309 for the product token `rypsok` and `*`,
  cached per origin for a day (one hour for a failed read), bounded.
- Manual redirects: same-host redirects followed up to 5 hops, every
  target re-judged by the policy; a cross-host target returned as
  `redirected` unless the operator allows following.
- Concurrency limits for fetches: per process, per host (one at a time)
  and for parsing, which runs off the I/O path.
- An extraction corpus of 15 hand-written pages, resource tests for byte,
  node and depth bombs, and a memory measurement script.

### Changed

- A `<` followed by any letter starts a tag for the title and snippet
  cleaner, so that cleaning stays idempotent under NFC.

## [0.2.0] — 2026-10-24 — Health and circuit breaking (milestone M2)

### Added

- A circuit per engine, driven by real requests and the clock: `open` after
  3 consecutive failures of class `timeout`, `rate_limited`, `blocked` or
  `parse_failure`, cooldown 60 s doubling up to 900 s, `Retry-After`
  honored within that cap; one real request probes the engine after the
  cooldown; `auth_failure` and `request_failure` never open it (RFC 010).
- `engine_status` reports the circuit state and the median latency of the
  last 20 completed requests per engine; state lives in memory and is
  reset when the process restarts.
- Circuit transitions are logged as `circuit_opened`, `circuit_half_open`
  and `circuit_closed` events with the engine, the cooldown and the
  failure count.

### Changed

- A cancelled probe, or a probe that ends with an uncounted failure,
  reopens the circuit with the same cooldown instead of counting as a
  verdict.

## [0.1.0] — 2026-10-20 — Search kernel (milestone M1)

### Added

- `web_search` over MCP stdio (revision 2026-07-28 with `server/discover`,
  and the handshake-based revisions 2024-11-05 to 2025-11-25): one
  structured response with a declared output schema, the same JSON as text,
  outcomes `ok`, `partial`, `empty`, and tool execution errors of kinds
  `invalid_request`, `denied`, `unavailable` (RFC 003).
- `engine_status` reporting `disabled`, `closed`, `open`, `half_open` per
  configured engine; the operator can remove the tool (RFC 003).
- Engines: Wikipedia (keyless) and Brave Search (keyed; web and news
  endpoints), each with an admission record, fixtures and a scheduled live
  check (RFC 006). A `base_url` key lets the Wikipedia driver query a
  private mirror.
- Concurrent aggregation under one deadline with per-engine timeouts, early
  completion, cancellation through `notifications/cancelled`, panic
  isolation, and no retries (RFC 007).
- URL normalization with tracking parameters removed, deduplication by
  comparison key, reciprocal rank fusion in integer arithmetic (RFC 008).
- The content boundary: markup and entities, NFC, removal of invisible and
  control characters, bounded titles and snippets, notices for every change,
  instruction-like content noticed or dropped (RFC 005).
- The secret check on queries with modes `refuse` (default), `mask`, `warn`,
  `off`; structured JSON logs on standard error that never carry a query,
  a URL path or content; in-process counters (RFC 009).
- Configuration in one TOML file with every default from the limits table,
  validation that names the key, secrets by environment variable or file,
  `rypsok check` (RFC 004).
- Per-tool invocation rate limits; per-engine rate and in-flight limits
  honoring `Retry-After`.
- Workspace, CI gates, requirement coverage gate, limits consistency gate
  (RFC 001, RFC 002).
- Log events of dependencies are capped at `warn` whatever the configured
  level, so that no library can log a query or a URL at `debug`.

### Limits (provisional, to be confirmed by measurement)

Search deadline 2500 ms, engine timeout 2000 ms, reserve 200 ms, up to 8
engines per request, 10 results per engine, snippets 300 and titles 150
characters, query 400 characters, 60 `web_search` calls per minute.

[Unreleased]: https://github.com/nabbisen/rypsok/compare/0.10.0...HEAD
[0.10.0]: https://github.com/nabbisen/rypsok/compare/0.9.0...0.10.0
[0.9.0]: https://github.com/nabbisen/rypsok/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/nabbisen/rypsok/releases/tag/0.8.0
[0.7.0]: https://github.com/nabbisen/rypsok/releases/tag/0.7.0
[0.6.0]: https://github.com/nabbisen/rypsok/releases/tag/0.6.0
[0.5.0]: https://github.com/nabbisen/rypsok/releases/tag/0.5.0
[0.4.0]: https://github.com/nabbisen/rypsok/releases/tag/0.4.0
[0.3.0]: https://github.com/nabbisen/rypsok/releases/tag/0.3.0
[0.2.0]: https://github.com/nabbisen/rypsok/releases/tag/0.2.0
[0.1.0]: https://github.com/nabbisen/rypsok/releases/tag/0.1.0
