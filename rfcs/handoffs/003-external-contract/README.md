# Handoff — RFC 003: External contract

**RFC.** [`../../accepted/003-external-contract.md`](../../accepted/003-external-contract.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-MCP-001 to
-011, REQ-SEARCH-001 to -006, REQ-STATUS-001 to -003, REQ-CTX-002, -003,
REQ-ERR-001 to -005, REQ-SEC-016), `docs/src/maintainers/external-design.md`
§2, §3, §4, §7, `ROADMAP.md` M1.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 001 and RFC 002 delivered (M0 closed).
**Milestone.** M1.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `rypsok-core::model`: the types of RFC 003 with `serde` derives, closed enumerations, `deny_unknown_fields` on requests | Golden tests: the JSON examples of external design §3 deserialize and re-serialize byte-identically; every enumeration serializes to the snake_case names of §3 and §4 |
| 2 | Request validation in the core (`LIM-S1`, `LIM-S2`, configured languages, serviceable categories) before any other step | Tests named after REQ-SEARCH-006 and REQ-ERR-002; each refusal carries the `invalid_request` message template of RFC 003 |
| 3 | `decide(n, a, f, r)` and the `web_fetch` mapping as pure functions | Exhaustive test against external design §7 (every row, both parts) |
| 4 | Error message templates and `ToolError` construction | A test that no template contains a placeholder for request, provider or page content; every message ≤ LIM-I4 characters |
| 5 | `rypsok::mcp::tools`: the normative tool texts and JSON Schemas as constants | Protocol test: the texts served by `tools/list` equal the constants; total definition size ≤ LIM-I1; each schema validates the §3 examples (REQ-MCP-008) |
| 6 | `rypsok::mcp::stdio`: the `rmcp` adapter mapping `server/discover`, `tools/list`, `tools/call`, `notifications/cancelled`, `structuredContent` on or off by configuration, stdout carrying MCP messages only | Protocol tests of RFC 002 layer 4: a spawned binary answers the four messages; stderr is the only place a log line appears; EOF on stdin ends the process within LIM-O1 (REQ-MCP-001 to -007, -011) |
| 7 | `Rypsok::status()` returning the `StatusResponse` shape with the registry state (engines come from RFC 006; until then the response lists none) | REQ-STATUS-001 test with an empty registry |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | `model` types, enumerations, golden tests, `decide` | — |
| 2 | Validation, message templates, `ToolError` | 1 |
| 3 | Tool constants, schemas, schema tests | 1 |
| 4 | Stdio adapter and protocol tests; `serve` starts the adapter | 2, 3, RFC 004 PR 2 |

## 3. Conventions that the reviewer will check

- No `String` in an envelope field that rypsok asserts (REQ-CTX-002); every
  vocabulary is an enum.
- `rmcp` appears only in `crates/rypsok` (dependency rule of RFC 001).
- The normative texts in the constants are the texts of RFC 003 word for
  word; a change to them is a change to the RFC first.
- Tests live in `tests.rs` beside each module; names carry the requirement
  identifier.

## 4. Review request package

Hand back `.git-exclude/reviews/003-external-contract/README.md` with links
to this handoff and RFC 003, the PR list, the protocol test log, the size of
the tool definitions in characters, and any deviation as a question.
