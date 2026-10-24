# Handoff — RFC 011: Egress policy and SSRF defense

**RFC.** [`../../accepted/011-egress-policy.md`](../../accepted/011-egress-policy.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-SEC-005 to
-009, REQ-SEC-021, REQ-FETCH-007, REQ-FETCH-010, REQ-RES-008, REQ-TEST-005,
REQ-DEP-003), `docs/src/maintainers/external-design.md` §8, §16,
`docs/src/maintainers/limits-and-defaults.md` LIM-F6 to LIM-F9, LIM-F14,
`docs/src/maintainers/threat-register.md` THR-05 to THR-07, THR-09, THR-14,
`ROADMAP.md` M3.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** M2 closed (release 0.2.0).
**Milestone.** M3. RFC 012 depends on deliverables 1 to 3.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `egress::policy`: `EgressPolicy`, `Rule`, `Denial`, `Target`, `judge_url`, `judge_address`, the refused-range table as a constant | The range table test (every range, inside and outside, both families); literal notations; credentials; ports; host-name refusals; the deny → allow → built-in order; the property test |
| 2 | `egress::resolver::JudgingResolver` implementing reqwest's `Resolve`, and `egress::agent_client` with the settings of RFC 011 | Rebinding and mixed-answer tests; a test that the client has no redirect policy, no proxy, no cookie store |
| 3 | `egress::strict::ReturnedUrls` bounded to LIM-F14, recorded by the search pipeline after the boundary | Eviction test at the bound; a protocol test that a URL returned by `web_search` passes strict mode and another is `denied`, `strict_mode` |
| 4 | Configuration validation of `egress.allow`, `egress.deny` entries with the index in the message; `LIM-F14` and `LIM-F15` in `Limits`, `[limits]` keys `strict_mode_urls` and `robots_cache_entries`, the configuration page, `check-limits` | `rypsok check` names `egress.allow[n]` on a bad entry; the page test of RFC 002 stays green |
| 5 | Refusal logging: `refused` with `reason` and `denial`, host only after admission | stderr capture test: a refused loopback fetch logs no URL and no address |
| 6 | Documentation: the egress section of `configuration.md` (proxy applies to engines only; allow rules are the operator's responsibility), "Restricting egress at the operating-system level" in the integration guide (REQ-DEP-003) | Built by `mdbook`; the systemd and container recipes present |
| 7 | Threat register review of THR-05, THR-06, THR-07, THR-09, THR-14 with a note in the review package | Package section |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | Policy, range table, tests | — |
| 2 | Judging resolver, agent client, tests | 1 |
| 3 | Strict-mode set, configuration validation, limits rows | 1 |
| 4 | Logging, documentation, threat review | 2, 3 |

## 3. Conventions that the reviewer will check

- No address is ever connected to without passing `judge_address` in the
  same request: no `resolve_to_addrs`, no second client, no `reqwest`
  redirect following anywhere in the agent-directed path.
- The refused-range table is one constant with one comment per range naming
  its RFC; tests iterate the table rather than repeating it.
- No URL, address or `Location` value reaches a log field or an error
  message; `Denial` names are the only vocabulary.
- Test servers are reached through an `[egress] allow` rule in test
  configuration, never through a `cfg(test)` bypass in the policy.
- `ipnet` is the only new dependency of this RFC; licenses pass
  `cargo deny`.

## 4. Review request package

Hand back `.git-exclude/reviews/011-egress-policy/README.md` with links to
this handoff and RFC 011, the PR list, the range-table test log, the
rebinding test, the threat review, and any deviation as a question.
