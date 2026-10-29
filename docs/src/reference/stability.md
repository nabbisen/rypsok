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
| Responses | The fields of each response; the outcomes `ok`, `partial`, `empty`; the error object (`kind`, `message`, `detail`); the error kinds; the notice shape (`code`, `engine`, `reason`, `count`) | A new field; a new notice code; a new reason; a new `detail` key | A field's name, type or meaning; a code's meaning; an outcome; an error kind |
| Protocol | MCP over stdio and over Streamable HTTP; the revisions served (2026-07-28 and the handshake-based revisions the SDK serves); the path `/mcp`; the statuses and who answers them: the gate answers 401, 404, 408, 413 and 503, the pinned SDK answers 400, 403 and 406, and an SDK upgrade that changes one of them is a contract change; the per-request headers of the 2026-07-28 revision; nothing on stdout but protocol messages | A new revision | Dropping a served revision; the path; a status's meaning |
| Configuration | `config_version = 1`: every key's name, type and meaning; unknown keys refused; secrets by reference only | A new key with a default; a new driver type; a wider range | A key's name, type or meaning; a narrower range; `config_version = 2` |
| Command line | `serve --transport stdio\|http`, `check`, `--version`, `--config`, `RYPSOK_CONFIG`; exit codes 0 (success), 1 (configuration), 2 (usage); `serve` writes nothing to stdout | A new subcommand or flag | A subcommand's or flag's meaning; an exit code |
| `rypsok-core` | The documented public interface, under Rust semantic versioning; the public types are `#[non_exhaustive]` and grow through new fields, variants and setters | New items, fields, variants and setters | Anything `cargo semver-checks` reports as breaking |

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

## How a breaking change is caught

A schema fixture test compares every tool definition and every response
schema with the committed fixtures, so a change to the contract cannot
pass unnoticed; from the first commit after 1.0.0 a CI job runs
`cargo semver-checks` against the latest published crates. Every
intentional change to this page is an RFC.
