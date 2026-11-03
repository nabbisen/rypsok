# Stability and versioning

From release 1.0.0 rypsok's external contract is stable: what this page
lists as frozen changes only with a new major version. What it lists as
"may be added" can arrive in a minor version, announced in the changelog.
Read the rules below the table as carefully as the table: they say what
may change, so that nobody builds on a detail the project never promised.

## The contract

| Surface | Frozen | May be added in a minor version | Changing or removing it is a major version |
|---|---|---|---|
| Tools | The names `web_search`, `web_fetch`, `engine_status`; every parameter's name, type and meaning | A new optional parameter with a default; a new tool | A parameter's name, type or meaning; a tool's name; a parameter becoming required |
| Responses | The fields of each response; the outcomes `ok`, `partial`, `empty`; the error object (`kind`, `message`, `detail`; the *wording* of `message` may change in a minor version, so match on `kind` and `detail.reason`); the error kinds; the notice shape (`code`, `engine`, `reason`, `count`) | A new field; a new notice code; a new reason; a new `detail` key | A field's name, type or meaning; a code's meaning; an outcome; an error kind |
| Protocol | MCP over stdio and over Streamable HTTP; the revisions served (2026-07-28 and the handshake-based revisions the SDK serves); the path `/mcp`; the statuses and who answers them: the gate answers 401, 404, 408, 413 and 503, the pinned SDK answers 400, 403 and 406, hyper answers 431 for a header block above LIM-H7, and an SDK upgrade that changes one of them is a contract change; the per-request headers of the 2026-07-28 revision; nothing on stdout but protocol messages | A new revision | Dropping a served revision; the path; a status's meaning |
| Configuration | `config_version = 1`: every key's name, type and meaning; unknown keys refused; secrets by reference only | A new key with a default; a new driver type; a wider range | A key's name, type or meaning; a narrower range; `config_version = 2` |
| Command line | `serve --transport stdio\|http`, `check`, `--version`, `--config`, `RYPSOK_CONFIG`; exit codes 0 (success), 1 (configuration), 2 (usage); `serve` writes nothing to stdout | A new subcommand or flag | A subcommand's or flag's meaning; an exit code |
| `rypsok-core` | The facade (`Rypsok` and its three services, `search`, `fetch` and `status`) and the types of the `model` module, under Rust semantic versioning; the types marked `#[non_exhaustive]` (listed [below](#the-public-types-of-rypsok-core)) grow through new fields, variants and setters. The other public items of the crate are public so that the `rypsok` command-line crate can use them: they are **not** part of the promise until the maintainer decides, before 1.0.0, what else is frozen | New items; new fields, variants and setters on the types marked below | Anything `cargo semver-checks` reports as breaking in the facade or in the `model` types |

## What may change, and how you learn of it

- **Default values.** The value of a default (a row of the limits table,
  such as the search deadline or the result ceiling) may change in a
  minor version. The changelog names the limit and both values. The key
  that sets it does not change. Read a value from the
  [configuration page](./configuration.md), not from memory.
- **Logs.** Log events and their field names are not part of the
  contract. They may change in a minor version with a changelog entry.
  What is promised about logs is only this: nothing reaches stdout, and
  no secret, query text or URL path reaches a log.
- **The output of `check`.** The human-readable report of `rypsok check`
  may change in a minor version; its exit code does not.
- **Engines.** A new driver type is an addition. A driver type is removed
  only in a major version, announced one minor version ahead by a line
  in `check` and in the changelog, because a configuration naming it
  would be refused.
- **Deprecation.** A contract element is deprecated in a minor version
  (a changelog line, a note on its documentation page, a warning from
  `check` where it applies) and removed in the next major version.
- **Minimum Rust version.** 1.88 at 1.0.0. Raising it is a minor version
  with a changelog entry.
- **The protocol SDK.** `rmcp` is pinned to an exact version. An upgrade
  that changes nothing on this page is a patch version.

## The public types of `rypsok-core`

The frozen surface of `rypsok-core` is the facade and the `model` module.
Not every public type there is `#[non_exhaustive]`, and the page does not
say so: it lists which are and which are not. A test reads the source and
fails when either list differs from it, so a type added to `model` has to be
put on one of them.

### Types marked `#[non_exhaustive]`

These grow through new fields, variants and setters in a minor version.

```text
Category
DetailReason
DisabledReason
ErrorDetail
FetchRequest
FetchResponse
EngineStatus
Notice
NoticeCode
Reason
SearchRequest
SearchResponse
SearchResult
SecretKind
StatusResponse
ToolError
```

### Types that are deliberately exhaustive

A new variant or field of these is a compile error in the code of a host,
which is what "needs a major version" should feel like. Each has a reason.

```text
Outcome: the outcomes `ok`, `partial` and `empty` are frozen; a new one is a major version
ErrorKind: the error kinds are frozen; a new one is a major version
CallError: the two ways a call ends (an error object, or cancelled); a third is a major version
EngineState: the circuit states of `engine_status`; a new state is a major version
TimeRange: the periods `day`, `week`, `month` and `year` of the search parameter
Decision: the result of the outcome decision table of the external design, which is closed
ErrorEnvelope: the shape `{"error": …}` of an error response
Cleaned: a marker with no data, proof that text passed the content boundary
EngineId: a validated identifier with a private field, so there is nothing to add
CallerId: a validated identifier with a private field, so there is nothing to add
Language: a validated language tag with a private field, so there is nothing to add
Date: a validated date with a private field, so there is nothing to add
InvalidEngineId: the error of the validating constructor, which has no fields
InvalidLanguage: the error of the validating constructor, which has no fields
InvalidDate: the error of the validating constructor, which has no fields
Rypsok: the facade; its fields are private and it grows through methods
```

## How a breaking change is caught

A fixture test compares the whole `tools/list` result (every tool's
description, input schema and annotations) and every response schema with the
committed fixtures, so a change to the contract cannot pass unnoticed; from the first commit after 1.0.0 a CI job runs
`cargo semver-checks` against the latest published crates. Every
intentional change to this page is an RFC.
