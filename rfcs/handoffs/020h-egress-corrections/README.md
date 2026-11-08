# Handoff 020h — Egress corrections: redirects and robots.txt, the address table

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3). Owner
decisions **D-1** and **D-2** of `arch-003-audit/DECISIONS-REVIEWED.md`
(authorized 2026-11-03). Findings NET-3 (S4), NET-4 (S2), TST-3.

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md),
RFC 011 (egress policy) and RFC 012 (fetch pipeline).
**Applicable requirements.** REQ-CMP-005, REQ-SEC-006, REQ-SEC-008.
**Rules.** English; one-line commit digests, optional body of at most 400
characters, no `Co-authored-by`; commit your own work, **push only after the
architect approves the review request**; no tag, no release. Copy the
scripts and raw results your evidence rests on to
`.git-exclude/evidence/020h/` before you hand in. Use relative paths with `cd`.
**Depends on.** Nothing.

## Change scope

Tests first, run on the unmodified tree and shown failing, then the fix.

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | **D-1. Every redirect hop is checked against robots.txt, same host included.** In `fetch.rs` `network()` the re-check after a hop is guarded by `!same_host`; remove the guard so the target of every hop passes `robots.check` after the policy check and the hop count. The verdict of the same origin is cached, so a same-host hop costs no request (assert it). The first request has already gone to the redirecting URL (allowed); only the target is refused | With a loopback server (the audit's `srv.py` shape, written afresh): `/private` disallowed by robots.txt is `denied`/`robots_txt` directly **and** as the target of `/start-same` (302); the server saw exactly one `robots.txt` request; a same-host hop to an allowed path still returns the page; a chain A→B→C where C is disallowed is refused at C; the cross-host cases (follow on, follow off → `redirected`) are unchanged; with `robots_txt = false` the redirect to `/private` returns the page | Append an amendment to RFC 012 (redirect table: every hop is checked; the old text said same-host hops were not); `requirements.md` REQ-CMP-005 says "every URL fetched, including each redirect target"; `external-design.md` and the `robots_txt` row of `reading-a-response.md` mention a redirect target; `threat-register.md` if a row speaks of redirects; changelog under Changed ("a call whose redirect target robots.txt disallows now answers `denied`/`robots_txt` where it returned the page") |
| 2 | **D-2. The built-in refusal table follows the IANA special-purpose registries.** Read the two registries once each (the IPv4 and the IPv6 Special-Purpose Address Registry pages at iana.org, their CSV downloads; record the URL and the date you read them in the file header below; no other site). Add every block whose "Globally Reachable" is False to `REFUSED_RANGES` that is missing, at least `2001:db8::/32`, `3fff::/20`, `5f00::/16`, `100:0:0:1::/64`, and the deprecated forms the registry does not list, `::/96` and `fec0::/10`. Fix the comment of `2001::/23` (it does **not** contain `2001:db8::/32`; name what it does contain). A block the registry marks globally reachable may stay refused (stricter); say which in a comment. IPv4-mapped (`::ffff:0:0/96`) stays judged as IPv4 after canonicalization | The table's own list as **independent test data**: a committed file `crates/rypsok-core/src/egress/policy/special_purpose.csv` (block, name, globally-reachable, registry date) read by the test, not derived from `REFUSED_RANGES`. For every non-global block the test asserts that the first address, the last address and an inner address are refused; for the extra deprecated blocks likewise; `REFUSED_RANGES.len()` is no longer asserted against itself. The audit's addresses (`::7f00:1`, `fec0::1`, `2001:db8::1`, `3fff::1`, `5f00::1`) are named cases. Public addresses (as today) still pass. **Do not send a packet to any of them** (reachability is `ip route get`, never a probe) | `threat-register.md` THR-05/SSRF row names the registry and the date; changelog under Changed ("these blocks are now refused") |

## Explicit non-change scope

`allow` rules still override the built-in refusals (operator's choice, as
today); the resolver, the name rules, strict mode, the cache; the robots.txt
reader's own redirect handling; no new configuration key.

## Prohibited shortcuts

A range test whose expected values come from `REFUSED_RANGES`. A robots
re-check that issues a new request for a cached origin. Adding a block "for
completeness" that the registry does not support without saying so in the
comment.

## Required evidence and review request

The gates (fmt, clippy `-D warnings`, test, doc, coverage, limits,
core-dependencies, `cargo deny`, `mdbook`); the failing-first output; the
server log lines proving one `robots.txt` read; the registry dates; the review
request at `.git-exclude/review-requests/020h-egress-corrections/README.md`
with a "Questions" section.
