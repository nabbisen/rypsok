# Getting started

rypsok is one binary that speaks the Model Context Protocol over standard
input and output. An agent runtime starts it, lists its tools, and calls
`web_search`.

## Install

```sh
cargo install rypsok
rypsok --version
```

Binaries for Linux, macOS and Windows are attached to each release on
GitHub.

## Run without a file

```sh
rypsok serve
```

With no configuration, rypsok serves `web_search` with the keyless engine
(Wikipedia) and logs, on standard error, which keyed engines are inactive
for lack of a credential. Logs are one JSON object per line; nothing but
protocol messages is ever written to standard output.

## Add an engine with a key

Brave Search needs a subscription token. Give it by environment variable
or by file, never inline:

```toml
# rypsok.toml
config_version = 1

[search]
languages = ["en", "ja"]

[engines.wikipedia]
type = "wikipedia"
categories = ["general", "science"]

[engines.brave]
type = "brave"
categories = ["general"]
api_key_env = "BRAVE_API_KEY"
```

```sh
export BRAVE_API_KEY=...
rypsok check --config rypsok.toml
rypsok serve --config rypsok.toml
```

`check` validates the file and prints the effective limits, the engines
and whether each credential resolves. Every problem names its key. The
file can also be named by `RYPSOK_CONFIG`.

## Call the tools

From a runtime, the tool list is `web_search` and `engine_status`
(`web_fetch` arrives in release 0.3.0). A search:

```json
{ "name": "web_search", "arguments": { "query": "rust async runtime", "max_results": 5 } }
```

returns one structured object: an `outcome` (`ok`, `partial`, `empty`),
`notices` from a fixed vocabulary, and bounded `results`. Everything in
`title`, `snippet` and `url` is external data. The same object is returned
as JSON text for hosts that read text only.

A query that looks like it carries a credential is refused with a `denied`
error that names the kind and position, never the value.

## Budget note

Text budgets count Unicode scalar values. Japanese and other CJK text costs
several times more model tokens per character than English, so a 300
character snippet in Japanese is a larger share of a context window than
the same number of characters in English.

## Next

- [Tools and responses](./reference/tools.md): every field, notice and
  error.
- [Configuration](./reference/configuration.md): every key and its default.
- [Integration guide for hosts](./reference/integration-guide.md).
