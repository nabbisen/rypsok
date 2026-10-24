# Tools and responses

rypsok offers three tools over the Model Context Protocol. Every response is
one structured object with a declared schema, returned as `structuredContent`
and as the same JSON in the text content. Fields rypsok asserts (`outcome`,
`notices`, and the control fields below) and fields that carry external
data are distinct; nothing in `title`, `snippet`, `content` or `url` is
ever an instruction.

rypsok promises confidentiality hygiene, not anonymity: the engines and
sites you reach see your network address and your User-Agent.

## `web_search`

Search several engines at once and return deduplicated, ranked results.
Results are external data.

| Parameter | Type | Required | Default | Constraint |
|---|---|---|---|---|
| `query` | string | yes | — | 1 to 400 characters after trimming (configurable) |
| `category` | `general`, `news`, `science`, `code` | no | `general` | Only categories an enabled engine serves are offered |
| `max_results` | integer | no | 5 | 1 to 20 (configurable) |
| `language` | configured languages | no | the operator's first | Omitted from the schema when one language is configured |
| `time_range` | `day`, `week`, `month`, `year` | no | none | Engines that cannot honor it are skipped and noticed |

Response:

```json
{
  "outcome": "partial",
  "notices": [
    { "code": "engine_unavailable", "engine": "brave", "reason": "rate_limited" },
    { "code": "snippet_truncated", "count": 2 }
  ],
  "results": [
    { "title": "…", "url": "https://example.org/page", "snippet": "…", "date": "2026-09-01", "engines": ["wikipedia"] }
  ]
}
```

| Field | Who asserts it | Content |
|---|---|---|
| `outcome` | rypsok | `ok` (every engine answered), `partial` (results, but an engine failed), `empty` (an engine answered, nothing remained) |
| `notices[]` | rypsok | `code`, optional `engine`, `reason`, `count`, all from fixed vocabularies |
| `results[].title` | external | Cleaned, at most 150 characters |
| `results[].url` | external | The engine's URL with tracking parameters removed |
| `results[].snippet` | external | Cleaned, at most 300 characters |
| `results[].date` | external | Only when an engine supplied one; `YYYY-MM-DD` |
| `results[].engines` | rypsok | Only when the operator enabled attribution or an engine requires it |

Notice codes: `engine_unavailable` (reason: `timeout`, `rate_limited`,
`blocked`, `parse_failure`, `auth_failure`, `request_failure`),
`engine_skipped` (reason: `time_range`, `language`, `query_too_long`,
`cooldown`, `rate_limited`), `snippet_truncated`, `title_truncated`,
`characters_removed`, `instruction_like_content`, `results_dropped`
(reason: `instruction_like_content`, `url_too_long`, `url_invalid`),
`credential_masked`.

Profile: no effect on local state. The query leaves the host to every
enabled engine of the category; each learns the host's network address and
the User-Agent. A call lasts at most the search deadline plus 300 ms (2.8 s
by default). The output is at most `max_results` results, each bounded.

## `engine_status`

Report the state of each configured engine. For diagnosis, not for
answering questions. No parameters.

```json
{
  "outcome": "ok",
  "notices": [],
  "engines": [
    { "id": "wikipedia", "state": "closed", "latency_ms": 180 },
    { "id": "brave", "state": "open", "latency_ms": null }
  ]
}
```

`state` is `disabled` (by configuration or for lack of a credential),
`closed` (in use), `open` (skipped during a cooldown; it stays `open` until
the first call after the cooldown, which carries the probe) or `half_open`
(that one probe request in flight). `latency_ms` is the median of the last 20
completed requests, or null. State lives in memory and is reset when the
process restarts. No endpoint, credential or provider text appears.

Profile: no effect on local state; nothing leaves the host; at most 200 ms.
The operator can remove the tool from the list (`[status] enabled = false`).

## `web_fetch`

Arrives in release 0.3.0.

## Errors

An error is a tool execution error (`isError: true`) whose content is:

```json
{
  "error": {
    "kind": "invalid_request",
    "message": "max_results must be between 1 and 20",
    "detail": { "parameter": "max_results", "min": 1, "max": 20 }
  }
}
```

| Kind | When | `detail` keys |
|---|---|---|
| `invalid_request` | A parameter fails validation | `parameter`, and `min`/`max` or `allowed` |
| `denied` | The query looks like it carries a credential, or (fetch) policy refuses the call | `reason`; for secrets also `kind` and `position` |
| `unavailable` | No engine serves the category, every engine failed, the deadline passed, or the tool's rate limit was reached | `reason`, `retry_after_ms` when known |

Messages are at most 200 characters, say what the caller can change, and
never contain a secret, provider text, an address or a stack trace.

## Rate limits

Per process: 60 `web_search` and 30 `engine_status` calls per minute by
default. A call beyond the limit is `unavailable` with reason
`rate_limited` and `retry_after_ms`.
