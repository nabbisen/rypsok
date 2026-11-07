# Examples

Every file here is included by a tutorial of the book and checked by CI
(`crates/rypsok/tests/examples.rs`, RFC 017): the configurations validate
and pass `rypsok check`, the host entries have the shape a runtime reads,
and every call's request matches the tool contract while its response
round-trips through rypsok's own response types. The first tutorial's
two lines (`calls/first-search.jsonl`) run end to end in CI against a
mock provider.

| File | Shown in | What it is |
|---|---|---|
| `configs/minimal.toml` | Getting started | The smallest file, equal to no file |
| `configs/first-search.toml` | Tutorial 1 | One keyless engine |
| `configs/keyed-engine.toml` | Tutorial 2 | Wikipedia and Brave, the key by environment variable |
| `configs/reading-a-page.toml` | Tutorial 3 | `web_fetch` budgets and the redirect rule |
| `configs/network-mode.toml` | Tutorial 4 | Streamable HTTP with two callers behind a proxy |
| `configs/scraping.toml` | Tutorial 5 | The scraping switch and the DuckDuckGo driver |
| `configs/cache.toml` | Configuration | The result cache on |
| `configs/strict-fetch.toml` | Tutorial 3 | Strict mode with an allow list |
| `hosts/stdio.json` | Tutorial 1 | A runtime that starts rypsok as a child process |
| `hosts/http.json` | Tutorial 4 | A runtime that connects to a listener with a token |
| `calls/first-search.jsonl` | Tutorial 1 | The two JSON-RPC lines piped into `rypsok serve` |
| `calls/*.json` | Tutorials 1, 3; Reading a response | One request and the answer rypsok gave, as `{"request", "response"}` |
| `pages/rust-ownership.html` | Tutorial 3 | The page the fetched answers were made from (a loopback server serves it as `/rust-ownership`) |

## Warnings that `rypsok check` gives for them

`rypsok check` warns about risky settings (RFC 020, D-6). The shipped
configurations are written to give none, except these two, which provoke
exactly the warning named (a test reads this list):

- `configs/network-mode.toml`: `service.http.bind` (the listener speaks
  plain HTTP; the example puts a TLS-terminating proxy in front of it)
- `configs/scraping.toml`: `engines.ddg` (a scraping engine, by design the
  point of the example)

`configs/network-mode.toml` also names two callers: `check` reads their
tokens as `serve` does, so it exits 1 until `RYPSOK_TOKEN_MIMLYS` is set (32
bytes or more) and `/run/secrets/rypsok-ops` exists.

## Where the answers come from

Each `calls/*.json` answer was produced on 2026-10-27 by piping the
request into `rypsok serve` (release 0.5.0 plus the M6 work) with the
Wikipedia engine pointed at a loopback server that served the Wikipedia
fixture of `crates/rypsok-core/fixtures/wikipedia/search-ok.json` and a
page. Two edits were made afterwards and nothing else: the loopback host
in URLs was replaced by `https://en.wikipedia.org/` for search results
and `https://example.org/` for the fetched page, and the `_note` key was
added where a file points to its continuation. The `web-search-partial`
answer used a second Wikipedia engine pointed at a port where nothing
listens; `web-search-empty` used the `search-empty.json` fixture;
`web-fetch-denied` asked for a private address, which the egress policy
refuses before any request.

**The four answers made from the page** (`web-fetch-ok`,
`web-fetch-continuation`, `web-fetch-continuation-2` and
`web-fetch-redirected`) can be made again. The page they were made from
was written by hand and lost; `pages/rust-ownership.html` is the page
written back from their content (RFC 020, TST-13), and
`tools/regenerate-example-answers` runs the four requests against it:

```sh
cargo build -p rypsok
tools/regenerate-example-answers            # prints what differs, exit 1 if anything does
tools/regenerate-example-answers --write    # replaces the files
```

The script serves the page and a redirect to another host from a loopback
address, starts `rypsok serve` with `fetch.follow_cross_host_redirects =
false`, sends `server/discover` and the four requests, and puts
`https://example.org` where the loopback address was. On 2026-11-07, with
release 0.8.0 plus the unreleased changes, all four answers came out
**identical** to the committed files. When a release changes an answer,
run it with `--show`, read the difference, and commit the new files with
the change.
