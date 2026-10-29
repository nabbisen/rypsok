# RFC 016 — 1.0.0: the stable contract

**Status.** Implemented (0.6.0); the 1.0.0 declaration it prepares is the owner's decision after real use; accepted 2026-10-27.
review decisions.
**Tracks.** The 1.0.0 row of `ROADMAP.md`; requirements REQ-MAIN-006
(contract versioning), REQ-ARCH-007 (the core interface, unstable until
1.0.0), REQ-MAIN-003 (`config_version`), REQ-CTX-002 (the canonical
response), REQ-OPS-001 (command line), REQ-MCP-006 (served revisions);
decision D-35.
**Touches.** The book (`reference/stability.md`, new; the introduction,
the integration guide), `README.md`, `CHANGELOG.md`, the requirements
(REQ-ARCH-007 amended), `rypsok-core` (`#[non_exhaustive]` and
constructors on the public types), the CI (a semver gate), the release
1.0.0.
**Handoff.** [`../handoffs/016-stable-contract/README.md`](../handoffs/016-stable-contract/README.md)

## Summary

1.0.0 changes no behavior. It says, in one page that cannot be misread,
what rypsok promises to keep, what it may add, and what it may change
with notice; it makes the Rust types able to grow without breaking a
host; and it adds a gate that catches a breaking change before it is
published. From 1.0.0 a breaking change of anything on that page needs a
major version (REQ-MAIN-006).

## Design

### What is frozen at 1.0.0 (the contract)

| Surface | Frozen | May be added in a minor version | Changing or removing needs a major version |
|---|---|---|---|
| Tools | The names `web_search`, `web_fetch`, `engine_status`; every parameter's name, type and meaning; the defaults' *meaning* (the value of a limit may change, see below) | A new optional parameter with a default; a new tool | A parameter's name, type or meaning; a tool's name; a required parameter |
| Responses | The fields of each response; the outcomes `ok`, `partial`, `empty`; the error object (`kind`, `message`, `detail`); the error kinds; the notice shape (`code`, `engine`, `reason`, `count`) | A new field (never required to be read); a new notice code; a new reason; a new detail key | A field's name, type or meaning; the meaning of a code; an outcome |
| Protocol | MCP over stdio and Streamable HTTP; the revisions served (2026-07-28 and the handshake-based revisions the SDK serves); `/mcp`; the HTTP status semantics of the gate (401, 403, 404, 413); the required headers; stdout purity | A new revision | Dropping a served revision; the path; a status's meaning |
| Configuration | `config_version = 1`: every key's name, type, range form and meaning; unknown keys refused; secrets by reference | A new key with a default; a new driver type; a wider range | A key's name, type or meaning; a narrower range; `config_version = 2` |
| Command line | `serve --transport stdio|http`, `check`, `--version`, `--config`, `RYPSOK_CONFIG`; exit codes 0, 1, 2; `check` prints to stdout and `serve` never does | A new subcommand or flag | A subcommand's or flag's meaning; an exit code |
| Core crate | The documented public interface of `rypsok-core` under Rust semantic versioning | New items; new fields and variants on `#[non_exhaustive]` types; new setters | Anything `cargo semver-checks` reports as breaking |

**Limits and defaults.** The value of a default (a limit of the limits
table) may change in a minor version with a changelog entry that names
the limit and both values; the key that configures it may not. A host
that depends on a value reads it from the configuration page, not from
memory.

**Logs** are not part of the contract beyond REQ-MCP-002 (nothing on
stdout) and REQ-PRIV-004 (no secret, query or URL path). Event and field
names may change in a minor version with a changelog entry. The
stability page says so, in those words, so that nobody builds an alert
on an event name and calls its rename a breach.

**Engines** come and go by their admission records (REQ-MAIN-004). Adding
a driver type is additive. Removing one (a provider withdrew its terms)
is announced one minor version ahead with a deprecation line in `check`,
and the removal is a major version, because a configuration naming it
would be refused.

### Growth without breakage (REQ-ARCH-007)

The public types of `rypsok-core` that a host reads or a driver author
constructs are marked `#[non_exhaustive]` and gain constructors, so that
a field or a variant can be added in a minor version:

- responses and their parts (`SearchResponse`, `FetchResponse`,
  `StatusResponse`, `SearchResult`, `Notice`, `ToolError`, `ErrorDetail`,
  `EngineStatus`): read by hosts, never constructed by them;
- the enums `Outcome`, `ErrorKind`, `DetailReason`, `NoticeCode`,
  `Reason`, `EngineState`, `Category`, `TimeRange`, `CallError`,
  `EngineErrorClass`: a host matches with a wildcard arm;
- requests (`SearchRequest`, `FetchRequest`): constructed with `new` and
  public fields set afterwards, as today;
- the driver interface (`EngineProperties`, `EngineRequest`,
  `EngineResponse`, `RawResult`, `EngineError`, `DriverContext`):
  `EngineProperties::new(max_query_chars, requests_per_minute,
  max_in_flight)` with conservative defaults and `with_*` setters;
  `RawResult::new(title, url, snippet)` and `with_date`; `EngineError::new`
  as today;
- the configuration tables (`Config` and every `*Config`, `EngineTable`):
  parsed from text, read by hosts.

Within the crate, literal construction stays as it is. The `Driver` trait
is stable; a new method gets a default body.

### The semver gate

The CI gains a job that runs `cargo semver-checks check-release` for both
crates against the latest published version, failing the pull request
on a breaking change unless the workspace version's major part was
bumped. It runs from the first commit after 1.0.0; before that it would
only report what the changelog already announces.

### The stability page and the words around it

`docs/src/reference/stability.md` (in the integrators' part of the book)
holds the table above, the limits rule, the logs rule, the engines rule,
the deprecation rule (deprecated in a minor version with a changelog
line and a `check` warning; removed in the next major), the minimum
Rust version rule (1.88 at 1.0.0; raising it is a minor version with a
changelog entry), and the SDK rule (`rmcp` pinned exactly; an upgrade
that changes nothing on this page is a patch). The README gains a
"Stability" paragraph of three sentences; the changelog's preamble
drops "Until 1.0.0 …" for the page's rule; the introduction and the
integration guide drop "unstable until 1.0.0"; REQ-ARCH-007 is amended
to say "stable from 1.0.0 under the policy of RFC 016".

### The contract audit before freezing

Before the page is written as fact, the implementer walks every row of
the tools page, the configuration page and the command line against the
code and the schema fixtures and lists every inconsistency, every name
that could be misread and every field whose meaning is not written. Each
finding is a question in the review package, not a silent change: a
fix before 1.0.0 is the last cheap moment, and the owner decides. Known
candidates: `service.structured_content` (a switch whose `false` changes
the response shape), `legacy_revisions` default `true` (kept, and
written), the `from_cache` notice appearing only with the cache on
(written), `engine_status`'s constant `outcome`.

### Release 1.0.0

The same steps as 0.5.0: version 1.0.0 in the workspace manifest,
`[1.0.0] — <date> — Stable contract` in the changelog with the page's
summary, tag `1.0.0`, both crates published, the GitHub release, the
image job. `tools/CURRENT_MILESTONE` stays `M5`: 1.0.0 adds no
requirement with a test; the semver gate is the new check.

## Amendment to the baseline

Applied with this RFC:

- Requirements: REQ-ARCH-007 reads "… and shall be marked unstable until
  release 1.0.0, stable from 1.0.0 under the policy of RFC 016".
- `ROADMAP.md`: the 1.0.0 row names RFC 016.

## Alternatives considered

- **Freezing by fixture only.** The schema fixtures already catch a
  change; they do not say which changes are allowed. The page does.
- **A `rypsok-core` 1.0 with sealed driver types.** Rejected: driver
  authors outside the repository are a stated goal (REQ-MAIN-004); the
  types stay constructible through constructors.
- **Logs as part of the contract.** Rejected: operators deserve the
  truth that event names may change, written once, over a promise the
  project would break the first time a field is renamed.
- **Dropping the handshake-based revisions at 1.0.0.** Rejected: hosts
  still speak them; the switch exists for operators who do not want
  them.

## Open questions

- Whether 1.0.0 should wait for a second scraping driver or the
  operator's-upstream engine. Decided here: no. The contract is about
  tools, responses, configuration and the command line; engines are
  additive by design.

## Amendment to RFC 016 (2026-10-29, review B9)

The list of enums to mark `#[non_exhaustive]` above is narrowed as the
implementation decided (package 016, A12): only the vocabularies the
stability table allows to grow are non-exhaustive (`NoticeCode`,
`Reason`, `DetailReason`, `SecretKind`, `Category`, `EngineErrorClass`,
`LanguageSupport`, `TimeRangeSupport`, `UserAgentFormat`); `Outcome`,
`ErrorKind`, `CallError`, `EngineState` and `TimeRange` stay exhaustive
so that a new variant is a compile error, which is what "needs a major
version" should feel like. Also: `check` writes its report to standard
error, not standard output, as the configuration page and REQ-MCP-002
say; the sentence above that says "stdout" was wrong (review B10).
