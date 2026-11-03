# Handoff 020n — Text that reaches the user is the text that was sent

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3), wave 2.
Owner decisions **D-8** and **D-10** (authorized 2026-11-03,
`arch-003-audit/DECISIONS-REVIEWED.md`). Findings DAT-8 (Medium, TST-4),
DAT-9, DAT-15, DAT-16 (Low). The owner's principle: no silent alteration of
visible text; what rypsok changes it reports.

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), RFC 005 (content boundary), RFC 008 (URLs, deduplication, ranking), RFC 009 (privacy).
**Applicable requirements.** REQ-RANK-002, REQ-RANK-003, REQ-SEC-003, REQ-CTX-*, REQ-PRIV-002.
**Rules.** As 020h; evidence to `.git-exclude/evidence/020n/`. Fuzz `clean_content` and `clean_decoded_text` and `returned_url` and `privacy_check` for 60 s each on the result and report.
**Depends on.** Nothing.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | **DAT-8, TST-4. The returned URL is the engine's, byte for byte, except tracking parameters.** In `normalize.rs` `strip_tracking`, rebuild the query **only when a tracking parameter was removed**; otherwise leave the query exactly as given. When one was removed, keep the others' original text: split the query on `&`, drop the segments whose name (the text before `=`, percent-decoded, lower-cased) is a tracking parameter, rejoin the remaining segments with `&` unchanged. `?a=1&b`, `?a=1;b=2`, `?q=a%20b`, `?a=%7e`, `?x=%zz`, `?a=1&&b=2` stay as they are | The case table of `req_rank_003_returned_url_table` gets these six inputs (each unchanged), plus: `?utm_source=x&a=1&b` → `?a=1&b`; `?a=1&utm_medium=y;z` keeps `z` text; a URL whose only parameter is tracking loses the `?`. Failing first | `external-design.md` field description says "byte for byte except"; changelog Fixed |
| 2 | **D-8. The comparison key.** Keep `http`≡`https` and the `www.` fold (unchanged). Change: (a) **keep an explicit non-default port** (the `url` crate already hides the default of the URL's own scheme: after the scheme fold, `http://x:443/p` is not `https://x/p` and `https://x:80/p` is not `https://x/p`); (b) **keep a non-empty fragment** (an empty fragment is dropped); (c) **sort the query parameters by name only, stably**, so the relative order of repeated parameters (`a=1&a=2` vs `a=2&a=1`) stays part of the identity | The twelve pairs of audit 020c DAT-9 as a table (equal / different): `http`≡`https`; `www.` fold; `?a=1&b=2`≡`?b=2&a=1`; `?a=1&a=2`≠`?a=2&a=1`; `#/inbox`≠`#/settings`; `:443` on http≠https; `:80` on https≠https; `/p/`≡`/p`; `?q=a+b`≡`?q=a%20b`; `/a%2Fb`≠`/a/b`; `/A`≠`/a`; `?q=a%2Bb`≠`?q=a+b`. The mutants of audit 020d that survived (`p != 80 && p != 443`, `path.len() > 1`, `>` vs `>=` at LIM-F9) get tests | `external-design.md`/RFC 008 amendment (appended) |
| 3 | **D-8 display.** When observations merge under one key, show the best-ranked observation's URL **unless** a merged observation has an `https` URL and the best-ranked has `http`: then show the best-ranked `https` one. Still an engine's own URL, byte for byte. Deterministic (the determinism tests `req_arch_004_*`, `req_test_011_*` still pass) | A test: two engines return `http://x/p` (rank 1) and `https://x/p` (rank 3): the result shows the `https` URL, rank by fusion unchanged | changelog Changed |
| 4 | **D-10. Zero-width joiners keep orthography and close the hidden channel.** In `boundary.rs` `is_removed`/the removal step: keep U+200C (ZWNJ) only between two characters of Arabic or an Indic script (code point ranges: Arabic 0600–06FF, 0750–077F, 08A0–08FF, FB50–FDFF, FE70–FEFF; Indic 0900–0DFF including Sinhala); keep U+200D (ZWJ) only in an Indic conjunct (after a virama, between Indic characters) or between two emoji (Extended_Pictographic, FE0F and the skin-tone modifiers count as emoji neighbours; ranges documented with the Unicode version); **never more than one joiner in a row; none at the start or end of the text.** Everything else in `Cf` (tag characters, bidi marks, soft hyphen, BOM, all other joiners) stays removed and counted in `characters_removed` as now. No new dependency (a small private range table). The notice is unchanged | Corpus test: Persian `می‌خواهم` unchanged (removed = 0); a Hindi conjunct with ZWJ unchanged; `👨‍👩‍👧` and `🏳️‍🌈` unchanged; ZWJ between Latin letters removed; two ZWNJ in a row → one kept; ZWNJ at the start removed; the Scotland flag (tag characters) → black flag with `removed` = 6; idempotence (cleaning twice = once, the property test of REQ-TEST-013) holds; the fuzz targets run | RFC 005 amendment (appended); `reading-a-response.md` / the page that lists what the boundary removes: what is kept and why; changelog Changed |
| 5 | **DAT-15. The secret check: fewer false refusals, the missed formats.** The 9 refused-but-ordinary strings of the audit (`css class sk-folding-cube-spinner-child`, `sk-hynix-semiconductor-roadmap-2026`, `bearer authentication-and-authorization-flow`, `ASIAPACIFICTRADEZONE`, `ssh://git:@github.com/…`, `https://example.org:8080@mirror…`, three dotted long words starting `eyJ`, `sk_test_configuration…`, `xoxb-feature-branch-naming-convention`) must pass; the credential-shaped ones the audit listed as passed must be refused: npm (`npm_` + 36), GitLab (`glpat-` + 20), Hugging Face (`hf_` + 34), SendGrid (`SG.` + 22 `.` 43), Telegram bot, Twilio `SK` + 32 hex, AWS secret key in context (`aws_secret_access_key = …`), `password=` / `passwd=` / `secret=` with a value of at least 8 characters that is not an ordinary word, `Authorization: Basic …`, Azure `AccountKey=`, `Bearer%20…`, an AWS key with a zero-width space inside, a lowercase AWS key, a GitHub token split by a space. **The corpus is the specification**: rebuild it (the original is lost); keys in it are made up and obviously fake; the rules are yours to design and the architect reviews them. A tail must look like a credential (a single unbroken run of the key alphabet, with the length of the real format), not like hyphenated words. No per-pattern override; the global `secret_detection` modes stay | The corpus as a committed table test (both lists), the five surviving mutants of `privacy::scan` (overlap rule: add a test with two overlapping findings, e.g. `Bearer ghp_…`), failing first | `reference/privacy` page / RFC 009 amendment (appended): the formats; changelog Changed |
| 6 | **Changelog.** Fixed (URL bytes, false refusals); Changed (key identity, display, joiners, new formats) | — | `CHANGELOG.md` |

## Explicit non-change scope

The cleaner's markup step and the instruction-like patterns; tracking-parameter
list; RFC 008's rank fusion; the secret check's modes; Unicode normalization.

## Prohibited shortcuts

A regex copied from a public list without the corpus test. Keeping joiners
"between letters" (that reopens the hidden channel). A key change that is
not covered by the twelve-pair table.

## Required evidence and review request

The gates; failing-first output; the four fuzz runs; the review request at
`.git-exclude/review-requests/020n-text-fidelity/README.md` with a
"Questions" section, listing every rule of item 5 with the corpus lines it
decides.
