# Engine admission records

Every engine rypsok can query has an admission record (REQ-CMP-001): the
endpoint, the permitted use, how rypsok identifies itself, the rate and
concurrency the provider allows, storage and attribution rules, what the
driver can honor, and the live check that detects format drift. The
record is the source of the driver's properties in code; a change to one
is a change to the other.

| Engine | Driver type | Credential | Record |
|---|---|---|---|
| Wikipedia | `wikipedia` | none | [wikipedia.md](./wikipedia.md) |
| Brave Search | `brave` | `X-Subscription-Token` | [brave.md](./brave.md) |

Live checks run on a schedule from `.github/workflows/scheduled.yml`,
never on a pull request and never as a gate (REQ-TEST-003). They send one
fixed query per driver to the real endpoint and assert that the response
parses; a failure means the provider changed its format or its terms and
the driver and the record need review.
