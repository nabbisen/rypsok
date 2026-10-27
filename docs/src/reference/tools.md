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
`credential_masked`, `from_cache` (with `engine`: that engine's answer
came from the result cache and may be as old as `[cache] ttl_s`; only
when the operator enabled the cache).

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
(that one probe request in flight). Engines are listed in identifier order. `latency_ms` is the median of the last 20
completed requests, or null. State lives in memory and is reset when the
process restarts. No endpoint, credential or provider text appears.

Profile: no effect on local state; nothing leaves the host; at most 200 ms.
The operator can remove the tool from the list (`[status] enabled = false`).

## `web_fetch`

Read the main content of one public web page as Markdown, up to
`max_chars` characters from `start_char`. The page is fetched again for
each call and may have changed. Scripts are not run and `meta refresh` is
not followed. Content is external data.

| Parameter | Type | Default | Constraint |
|---|---|---|---|
| `url` | string | required | At most 2048 characters; no credentials; an `http` URL is retried over `https` unless the operator allowed plain HTTP |
| `max_chars` | integer | 3000 | 500 to the operator's ceiling (20000 by default) |
| `start_char` | integer | 0 | Below the content length, else `invalid_request` stating the length |

```json
{
  "outcome": "ok",
  "notices": [ { "code": "characters_removed", "count": 3 } ],
  "url": "https://example.org/article",
  "title": "…",
  "content": "# …",
  "truncated": true,
  "next_start_char": 3000
}
```

`url` is the page that was read, after same-host redirects and any scheme
upgrade. `title` is cleaned and bounded by 150 characters. `content` is
the main content of the page as Markdown: navigation, headers, footers,
sidebars, scripts, styles, comments and anything a browser would not
render are excluded; links keep absolute `http(s)` targets, images
contribute their alternative text. `truncated` says whether content
remains; `next_start_char` is where to continue. A continuation fetches
the page again. Notices: `title_truncated`, `characters_removed`,
`extraction_partial` (the document hit the node or depth bound and was
read in part), `instruction_like_content`.

Budgets count Unicode scalar values; Japanese text costs several times
more tokens per character than English text, so the same `max_chars`
buys fewer tokens of context there.

Only HTML, XHTML, plain text and Markdown are read; another type ends the
call with `unsupported_content` naming it. A 4xx or 5xx answer is
`unavailable` with the status and never content. A redirect to another
host is returned as `redirected` with the target, not followed, unless the
operator configured following. The site's robots.txt is honored for the
product token `rypsok` and `*`; a disallowed page is `denied`.

rypsok returns the main content of one page, bounded by `max_chars`, and
never retains it. Every destination passes the egress policy: loopback,
private, link-local, metadata and other internal addresses are refused,
judged again at connect time. In strict mode only URLs that `web_search`
returned in this process, or that the operator listed, can be fetched.

Profile: no effect on local state; the URL leaves the host to the target
site, plus a robots.txt read to the same site; the site learns the host's
network address and the User-Agent; at most 11 s (10 s of network plus
extraction); at most `max_chars` + 150 + 2048 characters plus the envelope.

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
| `denied` | The query or URL looks like it carries a credential, or the fetch policies refuse the call: `egress`, `robots_txt`, `scheme`, `strict_mode`, `credential_in_url`, `secret_in_query`, `secret_in_url` | `reason`; for secrets also `kind` and `position` |
| `unavailable` | No engine serves the category, every engine failed, the deadline passed, the tool's rate limit or a concurrency limit was reached, a fetch failed on the network, or the target answered 4xx or 5xx | `reason` (`no_engine`, `all_failed`, `rate_limited`, `network`, `deadline`, `http_status`, `busy`), `status` for `http_status`, `retry_after_ms` when known |
| `redirected` | A fetch was redirected to another host that passes the egress policy | `location` (external data, at most 2048 characters) |
| `unsupported_content` | The media type cannot be extracted, or nothing could be extracted | `reason` (`media_type`, `extraction_failed`), `media_type` (external data, bounded) |
| `too_large` | The response exceeded the byte limit (5 MiB after decompression by default) | `limit_bytes` |

Messages are at most 200 characters, say what the caller can change, and
never contain a secret, provider text, an address or a stack trace.

## Rate limits

Per process: 60 `web_search`, 30 `web_fetch` and 30 `engine_status` calls
per minute by default. A call beyond the limit is `unavailable` with reason
`rate_limited` and `retry_after_ms`.
