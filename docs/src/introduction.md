# rypsok

rypsok is a headless meta-search service for AI agents. It searches several
engines at once, deduplicates and ranks the results, reads the main content
of web pages, and returns compact, structured responses in which external
text is always recognizable as data.

It speaks the Model Context Protocol over stdio and over Streamable HTTP, and
its core is a Rust library that any host can use.

This book has three parts:

- **New users:** install and run rypsok with an MCP host.
- **Integrators:** the tools, their responses, the configuration, and what a
  host must do on its side.
- **Maintainers:** the requirements, the external design, the limits, the
  threat register and the glossary. These are the source of truth for every
  design decision and every test.

The maintainers' part is baselined; the other parts are written as the
milestones deliver their features (see `ROADMAP.md`).
