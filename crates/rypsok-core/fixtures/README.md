# Fixtures

Recorded provider responses, one directory per engine (`wikipedia/`,
`brave/`), named by case: `search-ok.json`, `search-empty.json`,
`error-429.json`, `error-blocked.html`, `error-malformed.json` (RFC 002).

Rules:

- No credentials, no personal data, no session identifiers. A fixture that
  carries either is rejected in review. The pull request says how each file
  was recorded and what was removed.
- `hostile/` holds the hostile corpus for content-boundary and resource tests.
  Compressed bombs are generated at test time and never checked in.
