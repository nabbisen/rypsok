# rypsok

[![CI](https://github.com/nabbisen/rypsok/actions/workflows/ci.yml/badge.svg)](https://github.com/nabbisen/rypsok/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/rypsok.svg)](https://crates.io/crates/rypsok)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Headless meta-search for AI agents.** One tool call, several engines, one
clean answer, nothing leaked.

## Overview

rypsok is a small service that an AI agent runtime calls over the Model
Context Protocol (MCP). It queries several search engines at once, merges
and ranks their results, cleans every piece of external text before it
reaches the agent, and refuses to send anything that looks like a secret.
It is written in Rust and runs as one process with one configuration file.

Tools: `web_search`, `web_fetch` and `engine_status`.

## Why and when

- **Your agent needs the web, not a browser.** rypsok returns bounded,
  structured results sized for a context window, never a page dump.
- **You care where queries go.** Engines are admitted one by one with their
  terms on record; queries carrying credentials are refused; logs never hold
  a query or a URL path.
- **You want one dependency, not a stack.** One binary, stdio transport,
  no database, no browser engine.

rypsok is not a crawler, not a scraper by default, and not an answer engine:
it returns what the engines returned, cleaned and ranked, and says so.

## Quick start

```sh
cargo install rypsok
rypsok serve            # MCP over stdio, Wikipedia only, no file needed
```

Add an MCP server entry to your agent runtime, for example:

```json
{ "mcpServers": { "rypsok": { "command": "rypsok", "args": ["serve"] } } }
```

To add Brave Search, put a key in the environment and a file next to it:

```toml
# rypsok.toml
config_version = 1

[engines.brave]
type = "brave"
categories = ["general"]
api_key_env = "BRAVE_API_KEY"
```

```sh
rypsok check --config rypsok.toml   # validates and prints the effective settings
rypsok serve --config rypsok.toml
```

See [Getting started](docs/src/getting-started.md), the
[tools reference](docs/src/reference/tools.md) and the
[configuration reference](docs/src/reference/configuration.md).

## Design notes

- **Two crates.** `rypsok-core` holds every model, pipeline, driver and
  policy and depends on no protocol SDK; `rypsok` adds the MCP adapters and
  the command line. CI fails when a protocol crate reaches the core.
- **One structured response.** Every tool returns one outcome (`ok`,
  `partial`, `empty`) with notices from a fixed vocabulary; nothing is
  altered or reduced silently.
- **A content boundary.** External text is cleaned, bounded and stripped of
  invisible characters in one place; it is never rewritten.
- **Specification first.** Requirements, external design, limits and the
  threat register are baselined in `docs/src/maintainers/`; every test
  names the requirement it verifies.
- **A stable contract.** From 1.0.0 the tools, responses, configuration
  and command line change only with a major version; additions arrive in
  minor versions and are announced. The
  [stability page](docs/src/reference/stability.md) says exactly what is
  frozen and what may change.

## More detail

- [Documentation](docs/src/SUMMARY.md) (mdBook): getting started,
  tools and responses, configuration, integration guide.
- [Requirements](docs/src/maintainers/requirements.md) and
  [external design](docs/src/maintainers/external-design.md).
- [Roadmap](ROADMAP.md) and [RFCs](rfcs/README.md).
- [Changelog](CHANGELOG.md).

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
