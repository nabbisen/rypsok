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

Tools: `web_search` (0.1.0), `engine_status` (0.2.0), `web_fetch` (0.3.0).

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

The first release, 0.1.0, delivers `web_search` over stdio. Until then the
binary builds and reports what is missing:

```sh
cargo install rypsok
rypsok --version
rypsok serve        # "not implemented until milestone M1"
```

The configuration format and a host integration guide arrive with 0.1.0 in
[`docs/`](docs/src/SUMMARY.md).

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

## More detail

- [Documentation](docs/src/SUMMARY.md) (mdBook): getting started,
  tools and responses, configuration, integration guide.
- [Requirements](docs/src/maintainers/requirements.md) and
  [external design](docs/src/maintainers/external-design.md).
- [Roadmap](ROADMAP.md) and [RFCs](rfcs/README.md).
- [Changelog](CHANGELOG.md).

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
