# Tutorial 3: reading a page

**What you will have at the end.** The main content of a web page as
Markdown, read in two parts, and a clear picture of what rypsok refuses
to read and why.

**What you need.** Tutorial 1 done.

## 1. Set the budgets

`web_fetch` returns at most `max_chars` characters of a page's main
content. The defaults are 3000 per call and a ceiling of 20000; this file
raises the default:

```toml
{{#include ../../../examples/configs/reading-a-page.toml}}
```

## 2. Read a page

```json
{{#include ../../../examples/calls/web-fetch-ok.json}}
```

- `url` is the page that was actually read, after a redirect on the same
  host or an upgrade from `http` to `https`.
- `title` and `content` are the page's main content as Markdown:
  navigation, headers, footers, sidebars, scripts and anything a browser
  would not render are left out. Links keep their absolute targets.
- `truncated: false`: the whole content fit in `max_chars`.

## 3. Read a long page in parts

Ask for 500 characters:

```json
{{#include ../../../examples/calls/web-fetch-continuation.json}}
```

`truncated: true` and `next_start_char: 500` say where to continue. The
next call passes that number as `start_char`:

```json
{{#include ../../../examples/calls/web-fetch-continuation-2.json}}
```

Each call fetches the page again, so a page that changed between the
calls may not continue seamlessly. Budgets count characters, not tokens:
Japanese text costs several times more tokens per character than English.

## 4. What a page can answer with

A page that moved to another host is not followed; rypsok reports where
it went and lets the caller decide:

```json
{{#include ../../../examples/calls/web-fetch-redirected.json}}
```

A private or internal address is refused before any request leaves the
machine:

```json
{{#include ../../../examples/calls/web-fetch-denied.json}}
```

Other answers you will meet: `denied` with reason `robots_txt` (the site
asks not to be read), `unavailable` with `http_status` and the status
(the page answered 4xx or 5xx; its body is never returned),
`unsupported_content` (not HTML, XHTML, plain text or Markdown), and
`too_large` (above the byte limit).

## 5. Strict mode

An operator who wants `web_fetch` to read only what `web_search` found
turns on strict mode and lists the hosts that may be read anyway:

```toml
{{#include ../../../examples/configs/strict-fetch.toml}}
```

Any other URL is `denied` with reason `strict_mode`, before any request.

Next: [Tutorial 4: network mode](./04-network-mode.md).
