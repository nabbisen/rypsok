# Handoff 020y — A result is shown only with a host that `web_fetch` would accept

**Created 2026-11-08.** A fix handoff of the audit (RFC 020, D3), wave 4.
Owner decision **RFC 020, D-24** (2026-11-08, on N2 of the 020w review): a
returned URL whose host has an empty label is invalid. The owner's words:
"Sounds it should be treated as invalid. I want finally clean, safe and
secure, and robust and sophisticated design."

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md).
**Rules.** As 020w, including the per-change rule (`search/normalize.rs` reads
untrusted input; its fuzz targets are `returned_url`, `engine_json` and
`scrape_read`). Evidence goes to `.git-exclude/evidence/020y/`.
**Depends on.** 020x. **020v waits for this.**

## Why, and the design

- **Today.** `normalize::returned_url` accepts any host that the `url` crate
  parses. A provider's `https:/.xtras__url` becomes the result
  `https://.xtras__url/`. That is a URL no client can fetch, and `web_fetch`
  refuses it (`Host::from_name` rejects an empty label).
- **The clean form is one rule, not two.** **A result's URL is shown only if
  its host passes the same host check as `web_fetch`:**
  - a name accepted by `egress::policy::Host::from_name` (lower-cased, at
    most one trailing root dot, no empty label), or
  - an address literal.

  Then the search can never show a URL that the fetch tool refuses on syntax,
  and the rule lives in one function that both paths call.
- **What is not in this rule.** Egress *policy* (private addresses, operator
  rules) is not applied to results. Whether a host may be *fetched* is
  decided at fetch time, as today. This rule is about whether the URL is
  well formed.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | `normalize::returned_url`: after the scheme and userinfo checks, a domain host must pass `Host::from_name`; if not, `UrlRejection::Invalid`. Address hosts pass as today. The result is then dropped and counted with the other invalid URLs (`dropped_invalid`, the `results_dropped` notice with reason `url_invalid`), as an invalid URL is today. No new reason, no new notice | Table rows in `req_rank_003_returned_url_table`, or a new `req_rank_003_…` test: `https://.xtras__url/`, `https://a..b/`, `https://a./` (one root dot: accepted, as `from_name` accepts it) and `https://a../` (refused) each give what `from_name` gives. A pipeline test: a provider answer with such a URL among good ones yields the good ones and a `results_dropped` notice counting one `url_invalid` |
| 2 | **The unit test of 020w C5** (`req_eng_001_oddly_written_links_are_made_canonical_by_the_pipeline`) now expects the `.xtras__url` case to be **refused**, not canonicalized. **The `scrape_read` and `engine_json` fuzz targets** assert that a URL `returned_url` accepts has a host `Host::from_name` accepts. Run both 300 s from their corpus | the changed unit test; the two targets' runs in the evidence |
| 3 | **Requirement.** REQ-RANK-003 gets a second sentence, exact text: "A result whose URL is not an `http` or `https` URL with a host that `web_fetch` would accept (a name without an empty label, or an address) shall be dropped and counted in a notice. (RFC 020, D-24)". The tools page (`results_dropped`, `url_invalid`) says the same in one clause | `tools/check-requirements-coverage` |
| 4 | **Changelog** `[Unreleased]`, under Changed, as a contract change: "A search result whose URL has a host with an empty label (such as `https://.example/`) is dropped and counted as `url_invalid`, as other invalid URLs are, instead of being shown (RFC 020, D-24)" | — |

## Explicit non-change scope

- No egress policy applied to results.
- No change to `web_fetch`.
- No new notice or reason.
- Underscores in names stay accepted, as `web_fetch` accepts them.

## Required evidence and review request

1. The gates.
2. Failing first.
3. The per-change rule's runs: mutation of `search/normalize.rs`, and fuzz
   of the three targets for 120 s each, plus the two 300 s runs of item 2.
4. The review request at `.git-exclude/review-requests/020y-fetchable-result-hosts/README.md`.
