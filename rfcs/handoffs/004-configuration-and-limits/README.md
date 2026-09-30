# Handoff — RFC 004: Configuration schema and limits

**RFC.** [`../../accepted/004-configuration-and-limits.md`](../../accepted/004-configuration-and-limits.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-CFG-001 to
-006, REQ-OPS-001 to -004, REQ-MAIN-004), `docs/src/maintainers/external-design.md`
§13, `docs/src/maintainers/limits-and-defaults.md`, `ROADMAP.md` M1.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 001 and RFC 002 delivered. Independent of RFC 003.
**Milestone.** M1.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `rypsok-core::config`: the TOML schema of RFC 004 as typed structures with `deny_unknown_fields`, `config_version` check, defaults equal to `docs/src/maintainers/limits-and-defaults.md` | A test that an empty file yields the documented defaults; a test that every unknown key and every unknown table is refused with its path in the message (REQ-CFG-002) |
| 2 | Precedence `--config` → `RYPSOK_CONFIG` → defaults | Tests for each step and for a missing file named explicitly (exit 2) |
| 3 | Validation rules of RFC 004 (ranges, `LIM-S5 ≤ LIM-S4 − LIM-S6`, HTTP transport refused before M4, engine `type` known, secrets resolvable) | One test per rule; messages name the key and the accepted range |
| 4 | `rypsok-core::limits`: the `Limits` structure mirroring every `LIM-` identifier, and `tools/check-limits` comparing the constants with the limits document | The script passes; proven by a deliberate temporary breakage in the PR description |
| 5 | `Secret<String>` with redacted `Debug` and `Display`, resolution by `*_env` and `*_file` | A test that `format!("{:?}")` of a configuration with a secret contains neither the value nor the file content |
| 6 | `rypsok::cli` with `serve`, `check`, `--config`, `--version`, exit codes 0, 1, 2 | `rypsok check` on a file setting every key exits 0 and prints the effective limits; on an invalid file exits 2 with the messages of deliverable 3 |
| 7 | `docs/src/reference/configuration.md`: every key, its default, its range, its milestone | Built by `mdbook`; a test or script that every key of the schema appears in the page |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | Types, defaults, precedence, unknown-key refusal | — |
| 2 | Validation, `Secret`, `check` and `serve` command wiring (without the adapter) | 1 |
| 3 | `Limits`, `tools/check-limits`, the configuration reference page | 1 |

## 3. Conventions that the reviewer will check

- A limit exists in one place: the `Limits` constant, checked against the
  document; no literal number for a limit elsewhere in the code.
- Nothing is read from the environment except `RYPSOK_CONFIG`, the declared
  `*_env` secret variables and the proxy variables when `[egress].proxy` is
  set.
- `print_stdout` stays denied; `check` output goes through the CLI's writer.

## 4. Review request package

Hand back `.git-exclude/reviews/004-configuration-and-limits/README.md` with
links to this handoff and RFC 004, the PR list, the output of `rypsok check`
on a full configuration and on a deliberately invalid file, and any deviation as
a question.
