# RFC 017 — User documentation, tutorials and checked examples

**Status.** Implemented (0.6.0); accepted 2026-10-27.
**Tracks.** Milestone M6 (`ROADMAP.md`, accepted 2026-10-27); requirements
REQ-DOC-001 to REQ-DOC-004 (new, below), REQ-OPS-001 (`check`),
REQ-MCP-011 (the text form of a response), REQ-CTX-002; external design
§1.2 (what rypsok expects from a host), §13.1.
**Touches.** The book's part "For new users" (rewritten: getting started,
five tutorials, "Reading a response", "Troubleshooting"), `examples/`
(new, at the repository root), `crates/rypsok/tests/examples.rs` (new),
`README.md`, a workflow that publishes the book, the requirements.
**Handoff.** [`../handoffs/017-user-documentation/README.md`](../handoffs/017-user-documentation/README.md)

## Summary

A new user should get from the README to a first answer, and then to each
feature they need, without reading the maintainers' part. This RFC adds
five tutorials in the order a user meets the features, one page that
explains a response to the person who writes the host's prompt, one page
for when something does not work, and an `examples/` directory whose
files are the single source of every configuration, host entry and call
shown in the tutorials. CI checks every example: configurations validate,
host entries parse, calls match the tool contract, and the first tutorial
runs end to end against a mock provider. What the book shows is what
rypsok does, or CI fails.

## Design

### The user part of the book

```text
For new users
  Getting started            install, run without a file, check
  Tutorials
    1. Your first search     stdio, a host entry, one call, the answer read line by line
    2. A keyed engine        Brave: key by environment or file, categories, preference
    3. Reading a page        web_fetch: max_chars, continuation, the errors a page can answer with
    4. Network mode          HTTP behind a proxy: callers, tokens, allowed hosts, the headers, a curl call
    5. The scraping switch   what it means, DuckDuckGo, what `blocked` tells you
  Reading a response         for prompt authors: outcomes, notices, errors, what to tell the model
  Troubleshooting            symptom, cause, fix: check, engine_status, logs, HTTP statuses, exit codes
```

Each tutorial follows one shape: what you will have at the end; what you
need; the steps, each with the exact command or file; what you should
see; what to do when you do not. Every file and call a tutorial shows is
included from `examples/` with mdBook's `{{#include}}`, so the page and
the checked file cannot drift apart.

Tutorial 1 needs no host: it speaks to rypsok with a shell pipe of two
JSON-RPC lines (`server/discover`, then `tools/call` with the
2026-07-28 metadata) and shows the answer, then gives the host entry for
a runtime that starts rypsok itself. A reader who has a host skips the
pipe; a reader who has none still sees a real answer.

### `examples/`

| Directory | Content | Checked by |
|---|---|---|
| `examples/configs/` | One `rypsok.toml` per tutorial and three more: `minimal`, `cache`, `strict-fetch` | `Config::parse` + `validate`, and the binary's `check` exiting 0 |
| `examples/hosts/` | MCP host entries: `stdio.json` (a runtime starts rypsok), `http.json` (a runtime connects to a listener with a token) | JSON parse; the `mcpServers` shape; the command is `rypsok` or the URL ends in `/mcp` |
| `examples/calls/` | JSON-RPC message pairs: `discover`, `web_search` (ok, partial, empty, denied), `web_fetch` (ok, continuation, redirected, denied), `engine_status`; each file holds `request` and `response` | The request's arguments pass the adapter's parser; the response's `structuredContent` round-trips through the core's response type (deserialize, serialize, compare), so a field that does not exist or a wrong vocabulary fails |
| `examples/README.md` | What each file is, which tutorial includes it | — |

The first tutorial's calls run for real in CI: `tests/examples.rs` starts
the binary over stdio with `examples/configs/first-search.toml` (the
Wikipedia engine pointed at a `wiremock` mock serving the existing
Wikipedia fixture), pipes the two lines of the tutorial through it, and
asserts the answer has the outcome and the fields the tutorial's sample
answer shows. The sample answer in the page is the checked file.

### "Reading a response"

Written for the person who writes the host's prompt, not for the model.
One table per concept:

- the three outcomes and what each means for the next step;
- the notices in two groups: the ones that only inform (`title_truncated`,
  `from_cache`, …) and the ones that suggest a different call
  (`engine_skipped` with its reasons, `results_dropped`);
- the error kinds and what the caller can change (`invalid_request`: the
  parameter named; `denied`: nothing, do not retry; `unavailable`: wait
  `retry_after_ms`; `redirected`: call again with `location` if that is
  wanted; `unsupported_content`, `too_large`: another page);
- the rule that everything in `title`, `snippet`, `content` and `url` is
  external data, with a paragraph the host may paste into its prompt.

### "Troubleshooting"

A table of symptoms in the order a user meets them: `check` reports a
problem (the key is named; the common ones explained); an engine is
`inactive` (credential); "no engine serves the category"; every search
is `partial` with one engine `auth_failure` or `rate_limited`;
`engine_status` shows `open`; HTTP answers 400, 401, 403, 404, 413 and
what each means; `denied` with each reason; `redirected`; where the logs
are and what `level` and `log_queries` do; the exit codes.

### README and the book's entry

The README's quick start becomes three commands and a pointer to
Tutorial 1; "Why and when" stays; the file stays under 200 lines. The
introduction's "New users" line names the tutorials.

### Publishing the book

A workflow `docs.yml` builds the book on every push to `main` and
publishes it to GitHub Pages. Enabling Pages on the repository is the
owner's setting; until it is enabled the workflow builds and skips the
deploy step, and the README links the book's source as today.

### Requirements (amendment to the baseline, §22)

- **REQ-DOC-001 Tutorials — M6 · I.** The book shall carry tutorials a
  new user can follow from the book alone, in the order the features are
  met; every command and file shown shall be the checked example.
- **REQ-DOC-002 Checked examples — M6 · T.** Every example configuration
  shall validate, every host entry shall parse, every example call shall
  match the tool contract, and the first tutorial shall run end to end,
  all in CI.
- **REQ-DOC-003 Reading a response — M6 · I.** The book shall explain
  outcomes, notices and errors to the author of a host's prompt, with the
  action each suggests.
- **REQ-DOC-004 Troubleshooting — M6 · I.** The book shall map the
  symptoms a user meets to their causes and fixes, using `check`,
  `engine_status`, the logs and the HTTP statuses.

## Non-goals

No new tool, parameter or configuration key; no translation of the book;
no change to the maintainers' part beyond the requirements above; no
hosted service.

## Alternatives considered

- **Examples inside the pages only.** Rejected: a page cannot be executed;
  a file can, and the page includes it.
- **A separate "cookbook" repository.** Rejected: the examples must be
  versioned with the contract they show.
- **Executing every tutorial in CI.** Rejected for M6: tutorials 2, 4 and
  5 need a key, a proxy or a live provider; their files are validated and
  their calls are checked against the contract, which is what can be
  checked without the network.

## Open questions

- Whether the book should be published at all before 1.0.0. Decided
  here: yes, with the version in its title; users read it now.
