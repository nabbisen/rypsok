# Changelog

All notable changes to rypsok are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). Release tags carry no `v` prefix.

Until 1.0.0 the external contract (tool names, parameters, response fields,
outcomes, error kinds) may change between minor versions; every such change
is announced here.

## [Unreleased]

### Added

- Cargo workspace with the `rypsok-core` library and the `rypsok` binary
  (RFC 001).
- CI gates: format, lints, tests, documentation, dependency rule,
  requirement coverage, advisories and licenses, book build (RFC 002).
- Command line skeleton: `serve`, `check`, `--version`; the commands arrive
  with 0.1.0.
