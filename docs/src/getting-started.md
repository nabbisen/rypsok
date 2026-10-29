# Getting started

rypsok is one binary. An agent runtime starts it, or connects to it over
HTTP, and calls three tools: `web_search`, `web_fetch` and `engine_status`.

## Install

```sh
cargo install rypsok
rypsok --version
```

Binaries for Linux x86_64, macOS arm64 and Windows x86_64 are attached to
each release; `SHA256SUMS` proves a download is intact, not where it came
from. A container image is built for each release.

## Run

```sh
rypsok serve
```

With no configuration file, rypsok serves `web_search` and `web_fetch`
with the keyless engine, Wikipedia. Logs go to standard error, one JSON
object per line; standard output carries protocol messages only. Stop it
with Ctrl-C or by closing its input.

## Check a configuration

The smallest file is the same as no file:

```toml
{{#include ../../examples/configs/minimal.toml}}
```

```sh
rypsok check --config rypsok.toml
```

`check` says whether the file is valid, prints the effective settings and
each engine with the reason it is inactive, if it is, and exits with 0 for
a valid file, 1 for an invalid one. The file can also be named by the
environment variable `RYPSOK_CONFIG`.

## Where next

- [Tutorial 1: your first search](./tutorials/01-first-search.md) takes
  ten minutes and needs nothing but the binary.
- [Reading a response](./reading-a-response.md) is for the person who
  writes the prompt of the agent that will call rypsok.
- [Troubleshooting](./troubleshooting.md) when something does not work.
