# rypsok — Roadmap

**Status:** Authorized by the owner on 2026-09-30; milestones M6 to M8
added on 2026-10-27 at the owner's direction and accepted the same day;
1.0.0 is unscheduled and the owner's explicit decision.
Milestones are defined by content and exit criteria, not by dates. Each
milestone becomes one minor release; release tags carry no `v` prefix.

The normative sources are the [requirements](docs/src/maintainers/requirements.md)
and the [external design](docs/src/maintainers/external-design.md). Every
requirement names its milestone.

| Milestone | Name | Delivers | Release |
|---|---|---|---|
| **M0** | Foundation | Baselined specifications, glossary, threat register, limits table; repository scaffolding; workspace layout with a core crate free of protocol dependencies; CI gates: format, lints, tests, dependency audit, license check, dependency rule | — |
| **M1** | Search kernel | Core library with a documented interface; engine abstraction and registry; Wikipedia and Brave drivers with admission records; concurrent aggregation with deadlines, cancellation and early completion; provider rate and concurrency limits; the egress component with the engine profile; URL comparison keys, deduplication, rank fusion; the content boundary; the privacy check; configuration with validation; error and outcome vocabulary; MCP over stdio; `web_search`; command line; structured logs; measurements of overhead and cold start | 0.1.0 |
| **M2** | Operational control | Health state; circuit breaker; `engine_status`; error counters; live driver checks | 0.2.0 |
| **M3** | Secure fetch | The agent-directed egress profile; strict mode; connect-time enforcement; redirect handling; robots.txt; fetch pipeline with byte, node and depth limits; charset handling; extraction; continuation; `web_fetch`; memory measurement | 0.3.0 |
| **M4** | Network transport | MCP Streamable HTTP; `Origin` and `Host` validation; loopback bind by default; authentication; per-caller limits; container image | 0.4.0 |
| **M5** | Optimization | Result cache under the cache policy; further engines through admission records; scraping drivers as an opt-in engine type (DR-006); adaptive selection prepared | 0.5.0 and later |
| **M6** | Users | Documentation a user can follow without the maintainers' part: tutorials (first search over stdio; adding a keyed engine; network mode behind a proxy with a token; the scraping switch and what it means); worked examples checked by CI (host configurations, `rypsok.toml` files validated by `check`, example calls and answers); a page on reading a response (outcomes, notices, errors) written for the person who writes the host's prompt; troubleshooting with `check` and `engine_status`; the README and the book's user part revised against what a new user asks first | 0.6.0 |
| **M7** | Security | The threat model revisited end to end against the delivered system: every data flow and trust boundary, the register extended or corrected, each threat tied to a control and a test; hardening that the revision calls for (candidates: fuzz targets wired into the scheduled job, decompression and body bounds re-verified, the HTTP gate under slow and malformed clients, resolver and connect-time judgment over time, secret handling and logging re-audited, the container and systemd profiles tightened); a security review recorded as a release-readiness report | 0.7.0 |
| **M8** | Audit | A careful audit of the project as built: the data models (the model, configuration and driver types against the external design), the networking (clients, resolver, timeouts, pools, proxies, redirects, every bound), the test adequacy per requirement, the measurements of REQ-PERF, the dependency tree, dead code and drift from the RFCs; findings as an audit report with a fix list, the fixes as handoffs | 0.8.0 |
| **1.0.0** | Stable contract | Not scheduled. The owner declares 1.0.0 by an explicit decision once actual use cases have been achieved with rypsok, not at the end of a milestone; RFC 016 prepared the contract (the stability page, growth-safe types, a semver gate), and breaking changes need a major version from then on | 1.0.0 |

## Exit criteria

A milestone closes only when:

1. every requirement assigned to it passes its verification method;
2. the threat register has been reviewed for the data flows the milestone
   introduced;
3. the documentation matches the delivered behavior;
4. the security audit required by the project rules is recorded;
5. the review result carries a Go.

## Order of RFCs

RFC numbers are assigned when each file is created (RFC 000).

| Order | Topic | Milestone |
|---|---|---|
| 1 | Workspace layout and crate boundaries | M0 |
| 2 | Test strategy and CI gates | M0 |
| 3 | External contract: tools, schemas, envelope, errors, tool profiles | M1 |
| 4 | Configuration schema and limits | M1 |
| 5 | Content boundary | M1 |
| 6 | Engine abstraction, registry, admission; Wikipedia and Brave | M1 |
| 7 | Aggregation: deadlines, concurrency, cancellation | M1 |
| 8 | URL comparison keys, deduplication, rank fusion | M1 |
| 9 | Query privacy and logging | M1 |
| 10 | Health, circuit breaking, provider limits, `engine_status` | M2 |
| 11 | Egress policy and SSRF defense | M3 |
| 12 | Fetch pipeline, extraction, robots.txt | M3 |
| 13 | Streamable HTTP and authentication | M4 |
| 14 | Result cache | M5 |
| 15 | Scraping drivers as an opt-in engine type | M5 |
| 16 | 1.0.0: the stable contract (prepared; declared after M8) | 1.0.0 |
| 17 | User documentation, tutorials and checked examples | M6 |
| 18 | Threat model revision and hardening | M7 |
| 19 | Project audit: data models, networking, tests, measurements | M8 |

## Release cycle

One minor release per milestone. Patch releases carry fixes only. A release
is cut from `main` after the exit criteria are met and the changelog is
updated.
